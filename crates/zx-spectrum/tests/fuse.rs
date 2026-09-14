//! Checks the processor ZX Sidekick runs on (`rustzx-z80`, inside our bus)
//! against an independent description of what a Z80 does: the test corpus
//! written for the Fuse emulator, which states for 1335 cases what the
//! registers, memory and T-state count should be afterwards, and when each
//! address went on the bus.
//!
//! The corpus is not in this repository, for the same reason the game and the
//! ROM are not. See `assets/README.md` for where to get it; without it this
//! test says so and passes, and the count it prints makes a vacuous run
//! obvious.

use std::path::PathBuf;

use rustzx_z80::RegName16;
use zx_core::Snapshot;
use zx_spectrum::Zx;

/// Everything the corpus states about the processor at one moment.
#[derive(Clone, PartialEq, Eq)]
struct State {
    /// AF BC DE HL AF' BC' DE' HL' IX IY SP PC, in that order.
    regs: [u16; 12],
    i: u8,
    r: u8,
    iff1: bool,
    iff2: bool,
    im: u8,
    halted: bool,
    /// T-states: to run for in `tests.in`, reached in `tests.expected`.
    t: u32,
}

const NAMES: [&str; 12] = [
    "AF", "BC", "DE", "HL", "AF'", "BC'", "DE'", "HL'", "IX", "IY", "SP", "PC",
];

struct Case {
    name: String,
    state: State,
    /// Memory to place before the test, and after it in the expected file.
    mem: Vec<(u16, Vec<u8>)>,
    /// When the processor put an address on the memory bus, and which: the
    /// corpus's `MC` events. Only in the expected file.
    ///
    /// The reads and writes are deliberately left out. What they carry is
    /// already settled by comparing memory and registers, and the corpus
    /// records them as the Fuse interpreter happens to do them — a JR that
    /// is not taken contends for its displacement without ever reading it,
    /// for instance. The contention events are the processor's, and they are
    /// the ones that decide what the ULA charges.
    events: Vec<(u32, String, u16)>,
}

/// What the Fuse test harness returns for a port read: the port's high byte.
/// The real ULA is the machine's, not the processor's.
fn port_in(port: u16) -> u8 {
    (port >> 8) as u8
}

fn hex16(s: &str) -> u16 {
    u16::from_str_radix(s, 16).unwrap_or_else(|_| panic!("not a hex word: {s:?}"))
}

/// Reads the `<address> <byte>... -1` lines that end a test, stopping at the
/// terminator `end` (a lone `-1` in `tests.in`, a blank line in the expected).
fn read_mem<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    blank_ends: bool,
) -> Vec<(u16, Vec<u8>)> {
    let mut mem = Vec::new();
    for line in lines {
        let line = line.trim();
        if line == "-1" || (blank_ends && line.is_empty()) {
            break;
        }
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let at = hex16(parts.next().expect("memory line address"));
        let bytes = parts
            .take_while(|p| *p != "-1")
            .map(|p| u8::from_str_radix(p, 16).expect("memory byte"))
            .collect();
        mem.push((at, bytes));
    }
    mem
}

fn read_state<'a>(lines: &mut impl Iterator<Item = &'a str>, regs_line: &str) -> State {
    let regs: Vec<u16> = regs_line.split_whitespace().map(hex16).collect();
    assert_eq!(regs.len(), 12, "expected 12 registers, got {regs_line:?}");
    let rest = lines.next().expect("state line");
    let f: Vec<&str> = rest.split_whitespace().collect();
    assert_eq!(f.len(), 7, "expected 7 state fields, got {rest:?}");
    State {
        regs: regs.try_into().expect("12 registers"),
        i: u8::from_str_radix(f[0], 16).expect("I"),
        r: u8::from_str_radix(f[1], 16).expect("R"),
        iff1: f[2] != "0",
        iff2: f[3] != "0",
        im: f[4].parse().expect("IM"),
        halted: f[5] != "0",
        t: f[6].parse().expect("tstates"),
    }
}

fn parse_in(text: &str) -> Vec<Case> {
    let mut lines = text.lines().peekable();
    let mut out = Vec::new();
    loop {
        while lines.peek().is_some_and(|l| l.trim().is_empty()) {
            lines.next();
        }
        let Some(name) = lines.next() else { break };
        let regs_line = lines.next().expect("register line").to_string();
        let state = read_state(&mut lines, &regs_line);
        out.push(Case {
            name: name.trim().to_string(),
            state,
            mem: read_mem(&mut lines, false),
            events: Vec::new(),
        });
    }
    out
}

fn parse_expected(text: &str) -> Vec<Case> {
    let mut lines = text.lines().peekable();
    let mut out = Vec::new();
    loop {
        while lines.peek().is_some_and(|l| l.trim().is_empty()) {
            lines.next();
        }
        let Some(name) = lines.next() else { break };
        // Then the bus events, which we do not model: they are the ULA's
        // contention pattern, cycle by cycle, and this interpreter accounts
        // for an instruction all at once. An event line is recognised by its
        // second field naming an event type. It has to be that and not "the
        // field is letters", because a hex word can be all letters too:
        // `0000 ffff ...` is a register line whose second word is not hex to
        // look at.
        let mut events = Vec::new();
        let regs_line = loop {
            let line = lines.next().expect("register line");
            let f: Vec<&str> = line.split_whitespace().collect();
            let is_event = f
                .get(1)
                .is_some_and(|s| matches!(*s, "MR" | "MW" | "MC" | "PR" | "PW" | "PC" | "PB"));
            if !is_event {
                break line.to_string();
            }
            if f[1] == "MC" {
                events.push((
                    f[0].parse().expect("event time"),
                    f[1].to_string(),
                    hex16(f[2]),
                ));
            }
        };
        let state = read_state(&mut lines, &regs_line);
        out.push(Case {
            name: name.trim().to_string(),
            state,
            mem: read_mem(&mut lines, true),
            events,
        });
    }
    out
}

fn blank_machine() -> Zx {
    let snap = Snapshot {
        a: 0,
        f: 0,
        b: 0,
        c: 0,
        d: 0,
        e: 0,
        h: 0,
        l: 0,
        a_: 0,
        f_: 0,
        b_: 0,
        c_: 0,
        d_: 0,
        e_: 0,
        h_: 0,
        l_: 0,
        ix: 0,
        iy: 0,
        sp: 0,
        pc: 0,
        i: 0,
        r: 0,
        iff1: false,
        iff2: false,
        im: 0,
        border: 0,
        ram: vec![0; 0xC000],
    };
    let mut z = Zx::new(&snap, None);
    z.port_in = Some(port_in);
    z.interrupts = false;
    z.low_writable = true;
    z
}

fn load(z: &mut Zx, case: &Case) {
    let s = &case.state;
    let [af, bc, de, hl, af_, bc_, de_, hl_, ix, iy, sp, pc] = s.regs;
    let cpu = z.cpu();
    let r = &mut cpu.regs;
    r.set_af(af_);
    r.set_bc(bc_);
    r.set_de(de_);
    r.set_hl(hl_);
    r.swap_af_alt();
    r.exx();
    r.set_af(af);
    r.set_bc(bc);
    r.set_de(de);
    r.set_hl(hl);
    r.set_ix(ix);
    r.set_iy(iy);
    r.set_sp(sp);
    r.set_pc(pc);
    r.set_i(s.i);
    r.set_r(s.r);
    r.set_iff1(s.iff1);
    r.set_iff2(s.iff2);
    cpu.set_im(s.im);
    for (at, bytes) in &case.mem {
        for (i, &b) in bytes.iter().enumerate() {
            z.mem[(*at as usize + i) & 0xFFFF] = b;
        }
    }
}

fn actual(z: &mut Zx) -> State {
    let t = z.t;
    let cpu = z.cpu();
    let halted = cpu.is_halted();
    let im = u8::from(cpu.get_im());
    let r = &mut cpu.regs;
    let main = [
        r.get_reg_16(RegName16::AF),
        r.get_bc(),
        r.get_de(),
        r.get_hl(),
    ];
    r.swap_af_alt();
    r.exx();
    let alt = [
        r.get_reg_16(RegName16::AF),
        r.get_bc(),
        r.get_de(),
        r.get_hl(),
    ];
    r.swap_af_alt();
    r.exx();
    State {
        regs: [
            main[0],
            main[1],
            main[2],
            main[3],
            alt[0],
            alt[1],
            alt[2],
            alt[3],
            r.get_ix(),
            r.get_iy(),
            r.get_sp(),
            r.get_pc(),
        ],
        i: r.get_i(),
        r: r.get_r(),
        iff1: r.get_iff1(),
        iff2: r.get_iff2(),
        im,
        halted,
        t,
    }
}

/// What differs between the corpus and us, in words.
fn differences(want: &State, got: &State) -> Vec<String> {
    let mut out = Vec::new();
    for (i, name) in NAMES.iter().enumerate() {
        if want.regs[i] != got.regs[i] {
            out.push(format!(
                "{name} want {:04x} got {:04x}",
                want.regs[i], got.regs[i]
            ));
        }
    }
    let mut byte = |name: &str, w: u8, g: u8| {
        if w != g {
            out.push(format!("{name} want {w:02x} got {g:02x}"));
        }
    };
    byte("I", want.i, got.i);
    byte("R", want.r, got.r);
    byte("IM", want.im, got.im);
    for (name, w, g) in [
        ("IFF1", want.iff1, got.iff1),
        ("IFF2", want.iff2, got.iff2),
        ("halted", want.halted, got.halted),
    ] {
        if w != g {
            out.push(format!("{name} want {w} got {g}"));
        }
    }
    if want.t != got.t {
        out.push(format!("T-states want {} got {}", want.t, got.t));
    }
    out
}

/// Loads the corpus, or says why it cannot and leaves the test to pass.
fn corpus() -> Option<(Vec<Case>, Vec<Case>)> {
    let dir = std::env::var_os("FUSE_TESTS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets"),
        PathBuf::from,
    );
    let (input, expected) = (dir.join("tests.in"), dir.join("tests.expected"));
    let (Ok(input), Ok(expected)) = (
        std::fs::read_to_string(&input),
        std::fs::read_to_string(&expected),
    ) else {
        println!(
            "skipped: no Z80 test corpus in {}; see assets/README.md for where to get \
             tests.in and tests.expected",
            dir.display()
        );
        return None;
    };
    let cases = parse_in(&input);
    let expect = parse_expected(&expected);
    assert!(!cases.is_empty(), "the corpus parsed to no tests");
    assert_eq!(
        cases.len(),
        expect.len(),
        "the two corpus files disagree on how many tests there are"
    );
    for (a, b) in cases.iter().zip(&expect) {
        assert_eq!(a.name, b.name, "the corpus files are out of step");
    }
    Some((cases, expect))
}

/// Cases where `rustzx-z80` and the corpus disagree only about bits 3 and 5
/// of F (the undocumented copies of the result, `F3` and `F5`) after `SCF`,
/// `CCF` and `BIT n,(HL)`. What real chips put there was worked out after the
/// corpus was written: for `SCF` and `CCF` it depends on whether the previous
/// instruction changed the flags (the "Q" register), and for `BIT n,(HL)` on
/// an internal register ("MEMPTR"). `rustzx-z80` follows that later model.
/// Any other difference fails.
const UNDOCUMENTED_FLAG_CASES: [&str; 6] = ["37_1", "3f", "cb4e", "cb5e", "cb6e", "cb76"];

#[test]
fn matches_the_z80_test_corpus() {
    let Some((cases, expect)) = corpus() else {
        return;
    };

    let mut failures: Vec<String> = Vec::new();
    let mut bus_failures: Vec<String> = Vec::new();
    let mut flag_only = 0;
    for (case, want) in cases.iter().zip(&expect) {
        let mut z = blank_machine();
        load(&mut z, case);
        z.events = Some(Vec::new());
        // The corpus says how long to run for, and lets the last instruction
        // finish.
        let until = case.state.t;
        let mut before = z.clone();
        while z.t < until {
            z.step();
        }

        let mut diffs = differences(&want.state, &actual(&mut z));

        // The expected memory is only what changed, so compare all of it:
        // that catches a write we should not have made as well as one we
        // should have.
        for (at, bytes) in &want.mem {
            for (i, &b) in bytes.iter().enumerate() {
                before.mem[(*at as usize + i) & 0xFFFF] = b;
            }
        }
        let wrong: Vec<usize> = (0..0x10000)
            .filter(|&a| before.mem[a] != z.mem[a])
            .collect();
        for &a in wrong.iter().take(4) {
            diffs.push(format!(
                "[{a:04x}] want {:02x} got {:02x}",
                before.mem[a], z.mem[a]
            ));
        }
        if wrong.len() > 4 {
            diffs.push(format!("and {} more bytes of memory", wrong.len() - 4));
        }
        let only_flags_3_and_5 = UNDOCUMENTED_FLAG_CASES.contains(&case.name.as_str())
            && diffs.len() == 1
            && want.state.regs[0] & !0x28 == actual(&mut z).regs[0] & !0x28;
        if !diffs.is_empty() && only_flags_3_and_5 {
            flag_only += 1;
        } else if !diffs.is_empty() {
            failures.push(format!("{}: {}", case.name, diffs.join(", ")));
        }

        let got: Vec<(u32, String, u16)> = z
            .events
            .take()
            .unwrap_or_default()
            .into_iter()
            .map(|(t, at)| (t, "MC".to_string(), at))
            .collect();
        if got != want.events {
            let show = |v: &[(u32, String, u16)]| {
                v.iter()
                    .map(|(t, k, a)| format!("{t} {k} {a:04x}"))
                    .collect::<Vec<_>>()
                    .join(" | ")
            };
            bus_failures.push(format!(
                "{}:\n    want {}\n    got  {}",
                case.name,
                show(&want.events),
                show(&got)
            ));
        }
    }

    let passed = cases.len() - failures.len();
    println!(
        "Z80 corpus: {}/{} cases match exactly, {flag_only} more differ only in the undocumented bits 3 and 5 of F",
        passed - flag_only,
        cases.len()
    );
    println!(
        "Z80 bus activity: {}/{} cases match",
        cases.len() - bus_failures.len(),
        cases.len()
    );
    for f in failures.iter().take(40) {
        println!("  {f}");
    }
    for f in bus_failures.iter().take(15) {
        println!("  {f}");
    }
    assert!(
        failures.is_empty(),
        "{} of {} Z80 conformance cases differ",
        failures.len(),
        cases.len()
    );
}
