//! Local checks against the player's own copy of the game.
//!
//! Usage: `sk-check <command> <assets-dir>`, where the folder holds
//! `starquake.tap` and, for `rom`, `48.rom`. Nothing here runs in CI, which
//! has neither.
//!
//! - `rom [frames]`: runs a real-ROM machine through the menu and into play,
//!   and at every call the game makes to one of the three ROM routines
//!   ZX Sidekick answers, compares the real routine with the answer from the
//!   same state.
//! - `entry`: boots a real ROM, types `LOAD ""`, feeds it the tape block by
//!   block, and checks that the game starts where `sidekick::starquake` says.
//! - `keys`: chooses each control method on the title screen in turn, starts
//!   a game, and checks that the joystick and Start reach it through the
//!   machine: every direction and fire move the picture where Blob is, Start
//!   freezes it and a direction then resumes it; that Start or fire alone
//!   starts a game from the title screen and goes past the intro text; that
//!   Start still pauses in every method after the pause key was redefined;
//!   and that the control facts read as recorded.
//! - `facts`: checks the entry points the guidance panel follows: the menu
//!   runs first, then play, and holding A S D F G from the top of the play
//!   loop, as End this game does, reaches the game-over screens and comes
//!   back round to the menu, in every control method and from a paused
//!   game.
//! - `shot <frames> [out-dir]`: runs the ROM-free machine and writes a PNG of
//!   the screen every so often, to look at.

use std::path::{Path, PathBuf};

use sidekick::Machine;
use sidekick::machine::{JOY_DOWN, JOY_FIRE, JOY_LEFT, JOY_RIGHT, JOY_UP};
use sidekick::starquake::{CONTROL_METHOD, ENTRY_PC, ENTRY_SP, KEY_TABLES, PAUSE_KEY};
use zx_spectrum::Key;

fn read(dir: &Path, name: &str) -> Vec<u8> {
    std::fs::read(dir.join(name)).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {e}", dir.join(name).display());
        std::process::exit(2);
    })
}

fn machine(dir: &Path) -> Machine {
    Machine::from_tape(&read(dir, "starquake.tap"), ENTRY_PC, ENTRY_SP).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    })
}

/// The same input for every run: Kempston chosen on the menu, a game started,
/// then the joystick moved at random, a new direction every ten frames.
struct Script(u64);

impl Script {
    fn apply(&mut self, m: &mut Machine, frame: u64) {
        let z = &mut m.zx;
        z.release_all_keys();
        let key = |n| Key::by_name(n).expect("a key name");
        match frame {
            100..=104 => z.set_key(key("1"), true),
            150..=154 => z.set_key(key("0"), true),
            400.. => {
                if frame.is_multiple_of(10) {
                    self.0 ^= self.0 << 13;
                    self.0 ^= self.0 >> 7;
                    self.0 ^= self.0 << 17;
                }
                let dirs = ["joy_left", "joy_right", "joy_up", "joy_down", "joy_fire"];
                z.set_key(key(dirs[(self.0 % 5) as usize]), true);
            }
            _ => {}
        }
    }
}

/// Addresses the ROM changes that the ROM-free machine does not keep, and
/// that Starquake never reads: the keyboard state the interrupt maintains,
/// the print routine's own bookkeeping, and the stack below its pointer,
/// where routines leave what they pushed.
fn ignored(addr: usize, sp: u16) -> bool {
    matches!(addr, 0x5C00..=0x5C0A | 0x5C3B)
        || (0x5C00..usize::from(sp)).contains(&addr) && addr >= 0x5CB6
}

/// Runs a real-ROM machine through the menu and into play. At every call it
/// makes to one of the three ROM routines ZX Sidekick answers, two copies are
/// taken: one runs the ROM routine to its return, the other gets ZX
/// Sidekick's answer. Everything the game can see must then agree: memory,
/// the stack pointer and the return address, and for the multiply and the
/// interrupt the registers too. How long each took is compared as well, and
/// reported, since only the time is modelled.
fn rom_check(dir: &Path, frames: u64) -> bool {
    use sidekick::rom::{HL_HL_X_DE, MASK_INT, PRINT_A_2};
    let rom = read(dir, "48.rom");
    let mut real = machine(dir).with_rom(&rom);
    real.zx.traps = vec![MASK_INT];
    let mut script = Script(0xBEEF);
    // Per routine: calls checked, and the T-states each way.
    let mut stats = [(0u64, 0u64, 0u64); 4];
    let mut failures = 0;
    let mut skipped = 0;
    let mut recent: std::collections::VecDeque<(u8, u16, u16)> = std::collections::VecDeque::new();
    for frame in 0..frames {
        script.apply(&mut real, frame);
        let end = real.zx.frame + 1;
        while real.zx.frame < end {
            if real.zx.t >= zx_spectrum::FRAME_T {
                real.zx.t -= zx_spectrum::FRAME_T;
                real.zx.frame += 1;
                continue;
            }
            // The processor takes the interrupt and runs the routine's first
            // instruction in one step, so an interrupt is caught by keeping
            // the state from before a step that might take one, and noticing
            // afterwards that it did.
            let mut entry = None;
            if real.zx.t < zx_spectrum::INT_LEN && real.zx.iff1() {
                let before = real.clone();
                real.zx.step();
                if real.zx.fetched_from() != Some(MASK_INT) {
                    continue;
                }
                entry = Some(before);
            } else if ![PRINT_A_2, HL_HL_X_DE].contains(&real.zx.pc()) {
                real.zx.step();
                continue;
            }
            let which = match (&entry, real.zx.pc()) {
                (Some(_), _) => 0,
                (None, PRINT_A_2) => 1,
                _ => 2,
            };
            // A glyph drawn, as against a control code or its operand.
            let glyph = which == 1 && real.zx.a() >= 0x20 && {
                let z = &real.zx;
                z.read16(z.read16(0x5C51)) == 0x09F4
            };
            // Where the routine returns to, and the stack once it has.
            let (sp, ret) = match &entry {
                // An interrupt taken on a HALT returns past it.
                Some(before) => (
                    before.zx.sp().wrapping_sub(2),
                    before.zx.pc().wrapping_add(u16::from(before.zx.halted())),
                ),
                None => (real.zx.sp(), real.zx.read16(real.zx.sp())),
            };
            // The ROM's way: step until the routine has returned.
            let mut by_rom = real.clone();
            let mut t_rom = match &entry {
                Some(before) => u64::from(real.zx.t - before.zx.t),
                None => 0,
            };
            // Frames go on while it runs, as they would on the machine, so an
            // interrupt can land inside the routine; such a call is not
            // compared, since the answer is not interrupted.
            let mut steps = 0;
            let mut interrupted = false;
            while !(by_rom.zx.pc() == ret && by_rom.zx.sp() == sp.wrapping_add(2)) {
                if by_rom.zx.t >= zx_spectrum::FRAME_T {
                    by_rom.zx.t -= zx_spectrum::FRAME_T;
                    by_rom.zx.frame += 1;
                    t_rom += u64::from(zx_spectrum::FRAME_T);
                }
                let before = by_rom.zx.t;
                by_rom.zx.step();
                t_rom += u64::from(by_rom.zx.t - before);
                if by_rom.zx.fetched_from() == Some(MASK_INT) {
                    // The machine carries on from inside the interrupt.
                    interrupted = true;
                    break;
                }
                steps += 1;
                if steps > 1_000_000 {
                    println!(
                        "frame {frame}: routine {which} from pc {:04x} never returned to {ret:04x} (sp {:04x}); now pc {:04x} sp {:04x}",
                        real.zx.pc(),
                        sp.wrapping_add(2),
                        by_rom.zx.pc(),
                        by_rom.zx.sp()
                    );
                    return false;
                }
            }
            if interrupted {
                skipped += 1;
                real = by_rom;
                continue;
            }
            // ZX Sidekick's way: for the interrupt, from before it was taken,
            // taking it as the processor does (the return address pushed,
            // interrupts off, 13 T-states and an opcode fetch) and answering.
            let mut by_answer = match &entry {
                Some(before) => {
                    let mut m = before.clone();
                    m.zx.push(ret);
                    m.zx.set_pc(MASK_INT);
                    m.zx.set_interrupts(false);
                    m.zx.spend(13, 1);
                    m
                }
                None => real.clone(),
            };
            if which == 1 {
                let out = |z: &zx_spectrum::Zx| z.read16(z.read16(0x5C51));
                recent.push_back((real.zx.a(), out(&real.zx), real.zx.read16(0x5C0E)));
                if recent.len() > 12 {
                    recent.pop_front();
                }
            }
            let temps = |z: &zx_spectrum::Zx| (z.mem[0x5C8F], z.mem[0x5C90], z.mem[0x5C91]);
            let temps_before = temps(&real.zx);
            let before = entry.as_ref().map_or(by_answer.zx.t, |e| e.zx.t);
            assert!(sidekick::rom::answer(&mut by_answer.zx));
            let t_answer = u64::from(by_answer.zx.t - before);
            let (a, b) = (&by_answer.zx, &by_rom.zx);
            let low = a.sp().min(b.sp());
            let mem: Vec<usize> = (0x4000..0x10000)
                .filter(|&i| a.mem[i] != b.mem[i] && !ignored(i, low))
                .collect();
            let regs = |z: &zx_spectrum::Zx| {
                (
                    z.a(),
                    z.f(),
                    z.bc(),
                    z.de(),
                    z.hl(),
                    z.ix(),
                    z.iy(),
                    z.iff1(),
                )
            };
            let registers_agree = which == 1 || regs(a) == regs(b);
            if (a.pc(), a.sp()) != (b.pc(), b.sp()) || !mem.is_empty() || !registers_agree {
                failures += 1;
                if failures <= 5 {
                    println!(
                        "frame {frame}, routine {:04x} (a={:02x}): pc {:04x}/{:04x} sp {:04x}/{:04x}, memory {:04x?}, registers {:02x?} / {:02x?}",
                        [MASK_INT, PRINT_A_2, HL_HL_X_DE][which],
                        real.zx.a(),
                        a.pc(),
                        b.pc(),
                        a.sp(),
                        b.sp(),
                        &mem[..mem.len().min(8)],
                        regs(a),
                        regs(b)
                    );
                    println!(
                        "  recent (byte, channel output before, TVDATA before): {recent:02x?}"
                    );
                    println!(
                        "  ATTR_T, MASK_T, P_FLAG before {temps_before:02x?}, answered {:02x?}, ROM {:02x?}",
                        temps(a),
                        temps(b)
                    );
                }
            }
            let s = &mut stats[if which == 1 && !glyph { 3 } else { which }];
            *s = (s.0 + 1, s.1 + t_rom, s.2 + t_answer);
            // Carry on as the real machine did.
            real = by_rom;
        }
    }
    for (name, (n, t_rom, t_answer)) in [
        "interrupt",
        "print a character",
        "multiply",
        "print a control code",
    ]
    .iter()
    .zip(stats)
    {
        let mean = |t: u64| t.checked_div(n).unwrap_or(0);
        println!(
            "  {name}: {n} calls, mean {} T-states in the ROM, {} answered",
            mean(t_rom),
            mean(t_answer)
        );
    }
    let total: u64 = stats.iter().map(|s| s.0).sum();
    println!(
        "rom: {}/{total} calls match the real ROM ({skipped} more not compared: an interrupt landed inside)",
        total - failures
    );
    failures == 0 && total > 0
}

/// Boots a Spectrum with the real ROM, types `LOAD ""` on the keyboard, and
/// gives the ROM's tape loader (LD-BYTES, at 0x0556) the tape's blocks in
/// turn. The last block covers all of RAM, the stack included, so the
/// loader's closing `RET` goes wherever the block says: the game's start.
fn entry_check(dir: &Path) -> bool {
    const LD_BYTES: u16 = 0x0556;
    /// Where LD-BYTES sends its own return, pushed before it loads.
    const SA_LD_RET: u16 = 0x053F;
    let tap = read(dir, "starquake.tap");
    let mut blocks = Vec::new();
    let mut i = 0;
    while i + 2 <= tap.len() {
        let len = usize::from(tap[i]) | usize::from(tap[i + 1]) << 8;
        blocks.push(tap[i + 2..i + 2 + len].to_vec());
        i += 2 + len;
    }
    let mut m = Machine::blank(0, 0).with_rom(&read(dir, "48.rom"));
    let z = &mut m.zx;
    let key = |n| Key::by_name(n).expect("a key name");
    // Dismiss the copyright message, then LOAD "" and ENTER; later a key to
    // go past the loader's PAUSE.
    let typing: [(u64, &[&str]); 12] = [
        (150, &["enter"]),
        (160, &[]),
        (200, &["j"]),
        (210, &[]),
        (230, &["symbol", "p"]),
        (240, &[]),
        (260, &["symbol", "p"]),
        (270, &[]),
        (290, &["enter"]),
        (300, &[]),
        (500, &["space"]),
        (510, &[]),
    ];
    let mut typed = 0;
    let mut next = 0;
    while z.frame < 3000 {
        while typed < typing.len() && typing[typed].0 <= z.frame {
            z.release_all_keys();
            for k in typing[typed].1 {
                z.set_key(key(k), true);
            }
            typed += 1;
        }
        if z.pc() == LD_BYTES && next < blocks.len() {
            let block = &blocks[next];
            next += 1;
            let (len, dest) = (z.de() as usize, z.ix());
            z.set_sp(z.sp().wrapping_sub(2));
            z.write16(z.sp(), SA_LD_RET);
            let n = len.min(block.len() - 2);
            if block[0] == z.a() {
                for k in 0..n {
                    let at = dest.wrapping_add(k as u16);
                    if at >= 0x4000 {
                        z.mem[at as usize] = block[1 + k];
                    }
                }
            }
            z.set_ix(dest.wrapping_add(n as u16));
            z.set_de(0);
            z.set_f(z.f() | zx_spectrum::CF);
            let pc = z.pop();
            z.set_pc(pc);
            if next == blocks.len() {
                let (pc, sp) = (z.pc(), z.sp());
                let ok = (pc, sp) == (ENTRY_PC, ENTRY_SP);
                println!(
                    "entry: the loader returns to {pc:04x} with the stack at {sp:04x}{}",
                    if ok {
                        ", as recorded"
                    } else {
                        "; NOT as recorded"
                    }
                );
                return ok;
            }
        }
        let _ = z.run_until_any(&[LD_BYTES], 1);
    }
    println!(
        "entry: the tape never finished loading ({next} of {} blocks)",
        blocks.len()
    );
    false
}

/// Chooses control method `method` on the title screen, starts a game, and
/// plays past the intro text; then walks Blob left, off the ship and into
/// the next room, with the joystick alone once play begins, and lets the
/// room settle. If the joystick did not reach the game, Blob is still on
/// the ship, where a shot goes nowhere, and the checks say so.
fn into_play(dir: &Path, method: u8) -> Machine {
    let mut m = machine(dir);
    let key = |n: &str| Key::by_name(n).expect("a key name");
    for frame in 0..540u64 {
        m.zx.release_all_keys();
        m.joystick = 0;
        match frame {
            50..=54 => m.zx.set_key(key(&method.to_string()), true),
            100..=104 => m.zx.set_key(key("0"), true),
            // Any key takes the game past its intro text.
            330..=334 => m.zx.set_key(key("enter"), true),
            450..=490 => m.joystick = JOY_LEFT,
            _ => {}
        }
        m.run_frame();
    }
    m
}

/// The screen bytes below the panel: the play area's bitmap and attributes.
fn play_area(m: &Machine) -> Vec<u8> {
    let z = &m.zx;
    let mut out = Vec::with_capacity(0x1B00);
    for addr in 0..0x1800usize {
        let y = ((addr >> 8) & 7) | ((addr >> 2) & 0x38) | ((addr >> 5) & 0xC0);
        if y >= 48 {
            out.push(z.mem[0x4000 + addr]);
        }
    }
    out.extend_from_slice(&z.mem[0x5800 + 6 * 32..0x5B00]);
    out
}

/// Runs `m` on for 30 frames with the joystick `held` for the first three
/// and Start held throughout if `start`; returns the play area after frames
/// 1, 2, 10, 15 and 29.
fn after(m: &Machine, held: u8, start: bool) -> Vec<Vec<u8>> {
    let mut m = m.clone();
    let mut shots = vec![];
    for frame in 0..30u64 {
        m.zx.release_all_keys();
        m.joystick = if frame < 3 { held } else { 0 };
        m.start = start;
        m.run_frame();
        if matches!(frame, 1 | 2 | 10 | 15 | 29) {
            shots.push(play_area(&m));
        }
    }
    shots
}

/// Pauses `m` with Start for three frames, lets it sit, then pushes the
/// joystick right for three frames: the picture must be still while paused
/// and move again after. The game resumes on any direction or fire, not on
/// its pause key.
fn pauses_and_resumes(m: &Machine) -> bool {
    let mut m = m.clone();
    let mut shots = vec![];
    for frame in 0..60u64 {
        m.zx.release_all_keys();
        m.start = frame < 3;
        m.joystick = if (30..33).contains(&frame) {
            JOY_RIGHT
        } else {
            0
        };
        m.run_frame();
        if matches!(frame, 10 | 25 | 31 | 40) {
            shots.push(play_area(&m));
        }
    }
    shots[0] == shots[1] && shots[1] != shots[2] && shots[2] != shots[3]
}

/// From the title screen, Start held for a few frames starts a game, fire
/// held for a few more goes past the intro text, and the joystick then
/// moves Blob: a controller alone gets into play. Once with nothing chosen,
/// and once after choosing a method with the keyboard, since the title
/// screen reads keys differently before and after its first key.
fn starts_from_the_controller(dir: &Path) -> bool {
    let mut ok = true;
    for chosen in [None, Some("1")] {
        let mut m = machine(dir);
        for frame in 0..420u64 {
            m.zx.release_all_keys();
            if let Some(digit) = chosen
                && (20..=24).contains(&frame)
            {
                m.zx.set_key(Key::by_name(digit).expect("a key name"), true);
            }
            m.start = (60..=67).contains(&frame);
            m.joystick = if (330..=337).contains(&frame) {
                JOY_FIRE
            } else {
                0
            };
            m.run_frame();
        }
        let none = after(&m, 0, false);
        let with = after(&m, JOY_RIGHT, false);
        let reached = (0..3).any(|i| with[i] != none[i]);
        println!(
            "  from the title screen {}: Start and fire alone reach play {}",
            chosen.map_or("with nothing chosen".to_string(), |d| format!(
                "after choosing {d}"
            )),
            if reached { "ok" } else { "FAILED" }
        );
        ok &= reached;
    }
    ok
}

/// With the pause key redefined as N on the define-keys screen, then each
/// method chosen and a game started: Start through the machine must still
/// freeze the picture. The game pauses with Space in the Kempston method
/// whatever was defined, and with the defined key in the others; Start
/// presses whichever that is.
fn pauses_after_redefining(dir: &Path) -> bool {
    let mut ok = true;
    for method in 1..=5u8 {
        let mut m = machine(dir);
        let key = |n: &str| Key::by_name(n).expect("a key name");
        for frame in 0..1100u64 {
            m.zx.release_all_keys();
            let defined = ["z", "x", "c", "v", "b", "n"];
            match frame {
                50..=54 => m.zx.set_key(key("6"), true),
                100..=304 if (frame - 100) % 40 < 5 => {
                    m.zx.set_key(key(defined[((frame - 100) / 40) as usize]), true);
                }
                500..=506 => m.zx.set_key(key(&method.to_string()), true),
                600..=606 => m.zx.set_key(key("0"), true),
                850..=856 => m.zx.set_key(key("enter"), true),
                _ => {}
            }
            m.run_frame();
        }
        let none = after(&m, 0, false);
        let held = after(&m, 0, true);
        let good = m.zx.mem[usize::from(PAUSE_KEY)] == b'N'
            && m.zx.mem[usize::from(CONTROL_METHOD)] == method
            && held[3] == held[4]
            && none[3] != none[4];
        println!(
            "  pause redefined as N, method {method}: Start pauses {}",
            if good { "ok" } else { "FAILED" }
        );
        ok &= good;
    }
    ok
}

/// For every control method: the control facts read as recorded, each
/// joystick direction and fire change the picture where Blob is within a
/// few frames of being pressed, Start freezes it and a direction resumes
/// it; and Start or fire alone gets from the title screen into play.
fn keys_check(dir: &Path) -> bool {
    let mut ok = starts_from_the_controller(dir) & pauses_after_redefining(dir);
    for method in 1..=5u8 {
        let m = into_play(dir, method);
        let facts = m.zx.mem[usize::from(CONTROL_METHOD)] == method
            && &m.zx.mem[usize::from(KEY_TABLES)..usize::from(KEY_TABLES) + 20]
                == b"5867012345OPAQMQWERT"
            && m.zx.mem[usize::from(PAUSE_KEY)] == b'*';
        let none = after(&m, 0, false);
        let moves = |bit: u8| {
            let with = after(&m, bit, false);
            (0..3).any(|i| with[i] != none[i])
        };
        let results = [
            ("facts", facts),
            ("left", moves(JOY_LEFT)),
            ("right", moves(JOY_RIGHT)),
            ("down", moves(JOY_DOWN)),
            ("up", moves(JOY_UP)),
            ("fire", moves(JOY_FIRE)),
            ("start", {
                let held = after(&m, 0, true);
                held[3] == held[4] && none[3] != none[4]
            }),
            ("resume", pauses_and_resumes(&m)),
        ];
        let line: Vec<String> = results
            .iter()
            .map(|(name, good)| format!("{name} {}", if *good { "ok" } else { "FAILED" }))
            .collect();
        println!("  method {method}: {}", line.join(", "));
        ok &= results.iter().all(|(_, good)| *good);
    }
    println!(
        "keys: the joystick and Start reach the game in {}",
        if ok {
            "every control method"
        } else {
            "NOT every control method"
        }
    );
    ok
}

/// The game's routines the panel follows, by name, for the report.
fn routine_name(addr: u16) -> &'static str {
    use sidekick::starquake::routine;
    match addr {
        routine::MENU => "menu",
        routine::MAIN_LOOP => "play",
        routine::GAME_OVER => "game over",
        _ => "?",
    }
}

/// From `m` in play, holds End this game's keys as the app does and follows
/// the program, pressing `0` now and then for the screens that wait, until
/// it is back at the menu. Returns the frames at which the game-over screens
/// and the menu arrived.
fn ends_the_game(m: &Machine) -> Option<(u64, u64)> {
    use sidekick::starquake::{end_game_hold, routine};
    let mut m = m.clone();
    m.watch = vec![routine::MENU, routine::GAME_OVER];
    m.hold = Some(end_game_hold());
    let mut over = None;
    for frame in 0..4000u64 {
        m.zx.release_all_keys();
        m.joystick = 0;
        m.start = false;
        if over.is_some() && frame % 50 < 5 {
            m.zx.set_key(Key::by_name("0").expect("a key"), true);
        }
        for hit in m.run_frame() {
            match hit {
                routine::GAME_OVER if over.is_none() => {
                    over = Some(frame);
                    m.hold = None;
                }
                routine::MENU if over.is_some() => return over.map(|o| (o, frame)),
                _ => {}
            }
        }
    }
    None
}

/// The entry points the panel follows: the menu first and then play, and
/// End this game's keys ending a game in every control method, in play and
/// while paused.
fn facts_check(dir: &Path) -> bool {
    use sidekick::starquake::routine;
    let mut ok = true;
    let mut m = machine(dir);
    m.watch = vec![routine::MENU, routine::MAIN_LOOP, routine::GAME_OVER];
    let mut order: Vec<u16> = vec![];
    let key = |n: &str| Key::by_name(n).expect("a key name");
    for frame in 0..540u64 {
        m.zx.release_all_keys();
        match frame {
            50..=54 => m.zx.set_key(key("1"), true),
            100..=104 => m.zx.set_key(key("0"), true),
            330..=334 => m.zx.set_key(key("enter"), true),
            _ => {}
        }
        for hit in m.run_frame() {
            if order.last() != Some(&hit) {
                order.push(hit);
            }
        }
    }
    let names: Vec<&str> = order.iter().map(|&a| routine_name(a)).collect();
    let good = order == [routine::MENU, routine::MAIN_LOOP];
    println!(
        "  from the start: {} {}",
        names.join(", "),
        if good {
            "ok"
        } else {
            "FAILED, expected menu, play"
        }
    );
    ok &= good;
    for method in 1..=5u8 {
        let m = into_play(dir, method);
        let mut paused = m.clone();
        for frame in 0..20u64 {
            paused.zx.release_all_keys();
            paused.start = frame < 3;
            paused.run_frame();
        }
        let mut line = vec![];
        for (what, from, meant) in [("in play", &m, false), ("paused", &paused, true)] {
            let ended = ends_the_game(from);
            let good = from.paused == meant && ended.is_some();
            line.push(match ended {
                Some((over, menu)) if good => {
                    format!("{what}: game over after {over} frames, menu after {menu} ok")
                }
                _ => format!("{what}: FAILED (paused {}, ended {ended:?})", from.paused),
            });
            ok &= good;
        }
        println!("  method {method}: End this game {}", line.join("; "));
    }
    println!(
        "facts: the panel's entry points {}",
        if ok { "hold" } else { "do NOT hold" }
    );
    ok
}

fn shots(dir: &Path, frames: u64, out: &Path) {
    let mut m = machine(dir);
    let mut script = Script(0xBEEF);
    std::fs::create_dir_all(out).expect("output folder");
    for frame in 0..frames {
        script.apply(&mut m, frame);
        m.run_frame();
        if frame % (frames / 8).max(1) == 0 || frame + 1 == frames {
            let z = &m.zx;
            let pixels: Vec<u32> = (0..256 * 192)
                .map(|p| {
                    let (x, y) = (p % 256, p / 256);
                    let byte = z.mem[0x4000 + zx_core::screen::line_offset(y) + x / 8];
                    let attr = z.mem[0x5800 + (y / 8) * 32 + x / 8];
                    let ink = byte & (0x80 >> (x % 8)) != 0;
                    let colour = if ink { attr & 7 } else { (attr >> 3) & 7 };
                    let bright = if attr & 0x40 != 0 { 8 } else { 0 };
                    zx_core::screen::PALETTE[colour as usize + bright]
                })
                .collect();
            let path = out.join(format!("frame{frame:05}.png"));
            std::fs::write(&path, zx_core::png::encode(&pixels, 256, 192)).expect("write");
            println!("{}", path.display());
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = PathBuf::from(args.get(1).map_or("assets", String::as_str));
    match args.first().map(String::as_str) {
        Some("rom") => {
            let frames = args.get(2).and_then(|f| f.parse().ok()).unwrap_or(3000);
            std::process::exit(i32::from(!rom_check(&dir, frames)));
        }
        Some("entry") => std::process::exit(i32::from(!entry_check(&dir))),
        Some("keys") => std::process::exit(i32::from(!keys_check(&dir))),
        Some("facts") => std::process::exit(i32::from(!facts_check(&dir))),
        Some("shot") => {
            let frames = args.get(2).and_then(|f| f.parse().ok()).unwrap_or(600);
            shots(
                &dir,
                frames,
                &PathBuf::from(args.get(3).map_or("shots", String::as_str)),
            );
        }
        _ => {
            eprintln!("usage: sk-check rom|entry|keys|facts|shot <assets-dir> [frames] [out-dir]");
            std::process::exit(2);
        }
    }
}
