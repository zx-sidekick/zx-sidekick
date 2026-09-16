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
//!   a game, and checks that the joystick reaches it through the machine:
//!   every direction and fire move the picture where Blob is; that the pause
//!   key is reported and kept from the game, which goes on playing, also
//!   after the pause key was redefined; that Start or fire alone starts a
//!   game from the title screen and goes past the intro text; and that the
//!   control facts read as recorded.
//! - `facts`: checks the entry points the guidance panel follows: the menu
//!   runs first, then play, and holding A S D F G from the top of the play
//!   loop, as End this game does, reaches the game-over screens and comes
//!   back round to the menu, in every control method; and that the
//!   teleporter table holds fifteen codes in fifteen rooms and, with a ROM,
//!   that walking into each booth prints its code; and that in play the
//!   room stays a room and every room walked into is marked visited; and
//!   that every room marked as holding a missing core piece gets a wanted
//!   piece placed in it when the game enters it; and that the pieces and
//!   the core's holes are drawn from the graphics table as the core column
//!   reads it.
//! - `map [walks]`: has the game draw every room and reads the map from
//!   them, then walks Blob at random through play from many rooms and checks
//!   that he never leaves a room through an edge the map shows closed, and
//!   never gets from one part of a room to another across a wall it shows
//!   inside.
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

/// Runs `m` on for 30 frames with the joystick `held` for the first three;
/// returns the play area after frames 1, 2, 10, 15 and 29.
fn after(m: &Machine, held: u8) -> Vec<Vec<u8>> {
    let mut m = m.clone();
    let mut shots = vec![];
    for frame in 0..30u64 {
        m.zx.release_all_keys();
        m.joystick = if frame < 3 { held } else { 0 };
        m.run_frame();
        if matches!(frame, 1 | 2 | 10 | 15 | 29) {
            shots.push(play_area(&m));
        }
    }
    shots
}

/// Holds `key` on `m` for three frames, then lets go for 30: whether the
/// machine reported the pause key pressed, once, and whether the game read
/// its keys at the start of its reader in every frame, which a paused game
/// does not.
fn pause_with(m: &Machine, key: Key) -> (bool, bool) {
    let mut m = m.clone();
    m.watch = vec![sidekick::starquake::PLAY_INPUT];
    let (mut pressed, mut playing) = (0, true);
    for frame in 0..33u64 {
        m.zx.release_all_keys();
        m.zx.set_key(key, frame < 3);
        playing &= !m.run_frame().is_empty();
        pressed += usize::from(m.pause_pressed);
    }
    (pressed == 1, playing)
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
        let none = after(&m, 0);
        let with = after(&m, JOY_RIGHT);
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
/// method chosen and a game started: the machine must take the key the game
/// pauses with, N in methods 2 to 5 and Space in the Kempston method
/// whatever was defined, and keep it from the game; the other key must not
/// count as the pause key.
fn pause_after_redefining(dir: &Path) -> bool {
    let mut ok = true;
    let key = |n: &str| Key::by_name(n).expect("a key name");
    for method in 1..=5u8 {
        let mut m = machine(dir);
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
        let (pause, other) = if method == 1 {
            ("space", "n")
        } else {
            ("n", "space")
        };
        let good = m.zx.mem[usize::from(PAUSE_KEY)] == b'N'
            && m.zx.mem[usize::from(CONTROL_METHOD)] == method
            && pause_with(&m, key(pause)) == (true, true)
            && !pause_with(&m, key(other)).0;
        println!(
            "  pause redefined as N, method {method}: {pause} is the pause key and {other} is not {}",
            if good { "ok" } else { "FAILED" }
        );
        ok &= good;
    }
    ok
}

/// For every control method: the control facts read as recorded, each
/// joystick direction and fire change the picture where Blob is within a
/// few frames of being pressed, and Space, the pause key the tape ships, is
/// taken by the machine while the game plays on; and Start or fire alone
/// gets from the title screen into play.
fn keys_check(dir: &Path) -> bool {
    let mut ok = starts_from_the_controller(dir) & pause_after_redefining(dir);
    for method in 1..=5u8 {
        let m = into_play(dir, method);
        let facts = m.zx.mem[usize::from(CONTROL_METHOD)] == method
            && &m.zx.mem[usize::from(KEY_TABLES)..usize::from(KEY_TABLES) + 20]
                == b"5867012345OPAQMQWERT"
            && m.zx.mem[usize::from(PAUSE_KEY)] == b'*';
        let none = after(&m, 0);
        let moves = |bit: u8| {
            let with = after(&m, bit);
            (0..3).any(|i| with[i] != none[i])
        };
        let results = [
            ("facts", facts),
            ("left", moves(JOY_LEFT)),
            ("right", moves(JOY_RIGHT)),
            ("down", moves(JOY_DOWN)),
            ("up", moves(JOY_UP)),
            ("fire", moves(JOY_FIRE)),
            (
                "pause",
                pause_with(&m, Key::by_name("space").expect("a key")) == (true, true),
            ),
        ];
        let line: Vec<String> = results
            .iter()
            .map(|(name, good)| format!("{name} {}", if *good { "ok" } else { "FAILED" }))
            .collect();
        println!("  method {method}: {}", line.join(", "));
        ok &= results.iter().all(|(_, good)| *good);
    }
    println!(
        "keys: the joystick and the pause key work in {}",
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

/// Starts a game and, for every room marked as holding a missing core piece,
/// has the game enter that room and checks it placed a pickup whose item is
/// one the core still wants.
fn pieces_check(dir: &Path) -> bool {
    use sidekick::starquake::{CORE_ROOM, at, items_and_core, missing_piece_rooms, routine};
    let mut base = machine(dir);
    base.watch = vec![routine::MAIN_LOOP];
    let mut script = Script(0xBEEF);
    for frame in 0..600 {
        script.apply(&mut base, frame.min(399));
        if base.run_frame().contains(&routine::MAIN_LOOP) {
            break;
        }
    }
    let (items, core) = items_and_core(&base.zx.mem[..]);
    let marked = missing_piece_rooms(&core, &items);
    let rooms: Vec<u16> = (0..512).filter(|&r| marked.contains(r)).collect();
    let open_holes = core.iter().filter(|&&slot| slot & 0x80 != 0).count();
    let (mut placed, mut in_part) = (0, 0);
    for &room in &rooms {
        let mut m = base.clone();
        m.zx.write16(at::ROOM, room);
        m.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
        if room == CORE_ROOM || !m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000) {
            continue;
        }
        // Once placed, a piece's spot lies in a part of its room, where a
        // level 5 route ends (#50).
        let map_room = sidekick::starquake::read_room(&mut base.clone(), room);
        let spots_in_parts = items_and_core(&m.zx.mem[..])
            .0
            .iter()
            .filter(|i| {
                i.room() == room
                    && core
                        .iter()
                        .any(|&s| s & 0x80 != 0 && s & 0x7F == i.graphic())
            })
            .all(|i| i.row() != 0 && map_room.open.at(i.row(), i.column()) != 0);
        if spots_in_parts {
            in_part += 1;
        } else {
            println!("  room {room}: a wanted piece's spot is in no part of the room");
        }
        let z = &m.zx;
        let end = z.read16(at::MARKERS_END).max(at::MARKERS);
        // A pickup's marker kind is 0x14 plus its item's number.
        let wanted = (at::MARKERS..end).step_by(3).any(|a| {
            let kind = z.mem[usize::from(a) + 2];
            let Some(k) = kind.checked_sub(0x14).map(usize::from) else {
                return false;
            };
            let item = items_and_core(&z.mem[..]).0.get(k).copied();
            item.is_some_and(|i| {
                i.room() == room
                    && core
                        .iter()
                        .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == i.graphic())
            })
        });
        if wanted {
            placed += 1;
        } else {
            println!("  room {room} is marked, but no wanted piece was placed in it");
        }
    }
    let good = !rooms.is_empty() && placed == rooms.len() && in_part == rooms.len();
    println!(
        "  the missing pieces: {} rooms marked for {open_holes} open holes, a wanted piece placed in {placed} of them, its spot in a part of the room in {in_part} {}",
        rooms.len(),
        if good { "ok" } else { "FAILED" }
    );
    good
}

/// The 32 bytes of the 2 × 2 cells at character (`row`, `col`) of `z`'s
/// screen, top left, top right, bottom left, bottom right.
fn cells(z: &zx_spectrum::Zx, row: u8, col: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (k, (dr, dc)) in [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().enumerate() {
        for y in 0..8 {
            let line = zx_core::screen::line_offset(usize::from(row + dr) * 8 + y);
            out[k * 8 + y] = z.mem[0x4000 + line + usize::from(col + dc)];
        }
    }
    out
}

/// The graphics the core column draws (#7): entering every room marked as
/// holding a missing piece, the game lays the table's bytes for the item's
/// graphic on the screen at the item's cell; and walking into the core, with
/// three holes filled, draws each hole with the graphic `hole` gives, red
/// while open and white once filled.
fn graphics_check(dir: &Path) -> bool {
    use sidekick::starquake::{
        CORE_ROOM, at, graphic, hole, items_and_core, missing_piece_rooms, routine,
    };
    let mut base = machine(dir);
    base.watch = vec![routine::MAIN_LOOP];
    let mut script = Script(0xBEEF);
    for frame in 0..600 {
        script.apply(&mut base, frame.min(399));
        if base.run_frame().contains(&routine::MAIN_LOOP) {
            break;
        }
    }
    base.watch.clear();
    let (items, core) = items_and_core(&base.zx.mem[..]);
    let marked = missing_piece_rooms(&core, &items);
    let (mut drawn, mut rooms) = (0, 0);
    for room in (0..512u16).filter(|&r| marked.contains(r) && r != CORE_ROOM) {
        rooms += 1;
        let mut m = base.clone();
        m.zx.write16(at::ROOM, room);
        m.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
        // Each draw: its cell, the cells before it, and where it returns.
        let mut pending: Vec<(u8, u8, [u8; 32], u16)> = vec![];
        let mut done: Vec<(u8, u8, [u8; 32])> = vec![];
        m.call_observing(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000, |z| {
            if z.pc() == routine::DRAW_GRAPHIC {
                pending.push((z.b(), z.c(), cells(z, z.b(), z.c()), z.read16(z.sp())));
            }
            if let Some(i) = pending.iter().position(|p| p.3 == z.pc()) {
                let (row, col, before, _) = pending.remove(i);
                let after = cells(z, row, col);
                let mut xor = [0u8; 32];
                for k in 0..32 {
                    xor[k] = before[k] ^ after[k];
                }
                done.push((row, col, xor));
            }
        });
        let (placed, _) = items_and_core(&m.zx.mem[..]);
        let good = placed.iter().filter(|i| i.room() == room).any(|i| {
            done.iter().any(|d| {
                (d.0, d.1) == (i.row(), i.column()) && d.2 == graphic(&m.zx.mem[..], i.graphic())
            })
        });
        drawn += usize::from(good);
        if !good {
            println!("  room {room}: the piece was not drawn as the table has it");
        }
    }
    let pieces_ok = rooms > 0 && drawn == rooms;
    println!(
        "  the pieces' graphics: drawn from the table in {drawn} of {rooms} marked rooms {}",
        if pieces_ok { "ok" } else { "FAILED" }
    );

    // The core: three holes filled, then walk in from the room to its left.
    let mut m = base.clone();
    let slots = usize::from(at::CORE_SLOTS);
    for i in [0usize, 1, 4] {
        m.zx.mem[slots + i] = i as u8;
    }
    let bytes: Vec<u8> = m.zx.mem[slots..slots + 9].to_vec();
    m.zx.write16(at::ROOM, CORE_ROOM - 1);
    m.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
    let entered = m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000);
    m.zx.t = 0;
    m.zx.set_interrupts(true);
    m.zx.mem[usize::from(at::ENTITIES) + 5] = 0xE8;
    let mut draws: Vec<(u8, u8, u8, u16)> = vec![];
    for frame in 0..200 {
        m.zx.release_all_keys();
        m.zx.kempston = if frame < 100 { 0x01 } else { 0 };
        m.run_frame_observing(|z| {
            if z.pc() == routine::DRAW_GRAPHIC {
                draws.push((
                    z.b(),
                    z.c(),
                    z.a(),
                    u16::from(z.h()) << 8 | u16::from(z.l()),
                ));
            }
        });
    }
    let holes_ok = entered
        && (0..9).all(|i| {
            let (g, open) = hole(i, bytes[i]);
            let (row, col) = (12 + 2 * (i / 3) as u8, 13 + 2 * (i % 3) as u8);
            draws
                .iter()
                .find(|d| (d.0, d.1) == (row, col))
                .is_some_and(|d| {
                    d.3 == at::GRAPHICS + u16::from(g) * 32 && d.2 == if open { 2 } else { 7 }
                })
        });
    println!(
        "  the core's holes: each drawn with its graphic, red while open and white once filled {}",
        if holes_ok { "ok" } else { "FAILED" }
    );
    pieces_ok && holes_ok
}

/// Plays 6,000 frames under random joystick input and checks the two
/// addresses the map follows (#5): the room stays a room, and every room
/// walked into is marked visited in the game's own set within 50 frames,
/// which is the time entering a room takes to draw.
fn visited_check(dir: &Path) -> bool {
    use sidekick::starquake::{at, routine};
    let mut m = machine(dir);
    m.watch = vec![routine::MENU, routine::MAIN_LOOP, routine::GAME_OVER];
    let mut script = Script(0xBEEF);
    let (mut playing, mut last_room, mut visits, mut late, mut slowest, mut bad) =
        (false, None, 0, 0, 0, 0);
    let mut waiting: Option<(u16, u64)> = None;
    // The reason each room was entered with, as the game enters it.
    let mut reasons: Vec<u8> = vec![];
    for frame in 0..6000u64 {
        script.apply(&mut m, frame);
        let hits = m.run_frame_observing(|z| {
            if z.pc() == routine::ENTER_ROOM {
                reasons.push(z.mem[usize::from(at::ENTRY_REASON)]);
            }
        });
        for hit in hits {
            playing = hit == routine::MAIN_LOOP;
            if !playing {
                last_room = None;
                waiting = None;
            }
        }
        if !playing {
            continue;
        }
        let z = &m.zx;
        let room = z.read16(at::ROOM);
        if room >= 512 {
            bad += 1;
            continue;
        }
        let unvisited = |room: u16| {
            let byte = z.mem[usize::from(at::UNVISITED_ROOMS + (room >> 3))];
            byte & (0x80 >> (room & 7)) != 0
        };
        if let Some((r, since)) = waiting {
            if !unvisited(r) {
                slowest = slowest.max(frame - since);
                waiting = None;
            } else if frame > since + 50 {
                late += 1;
                waiting = None;
            }
        }
        if last_room != Some(room) {
            if last_room.is_some() {
                visits += 1;
                waiting = Some((room, frame));
            }
            last_room = Some(room);
        }
    }
    // Walking about, every room is entered with the reason for walking in.
    let entered = reasons.len();
    bad += reasons
        .iter()
        .filter(|&&why| why != sidekick::starquake::entry::WALKED)
        .count();
    let good = visits > 0 && late == 0 && bad == 0;
    println!(
        "  the rooms visited: {visits} rooms walked into, each marked within {slowest} frames; {entered} rooms entered, all as walked; {late} late, {bad} out of place {}",
        if good { "ok" } else { "FAILED" }
    );
    good
}

/// The teleporter table holds fifteen codes of five capital letters in
/// fifteen different rooms, and, with a ROM for the stepping, walking into
/// each booth has the game print the code the table gives for its room:
/// the moment a code counts as seen (#4).
fn teleporters_check(dir: &Path) -> bool {
    use sidekick::rom::PRINT_A_2;
    use sidekick::starquake::{BOOTH_MARKER, at, routine, teleporter_code};
    let m = machine(dir);
    let entries: Vec<(u16, [u8; 5])> = (0..at::TELEPORTER_COUNT)
        .map(|i| {
            let entry = usize::from(at::TELEPORTER_NAMES) + i * 7;
            let code: [u8; 5] = m.zx.mem[entry..entry + 5].try_into().expect("five bytes");
            (m.zx.read16(entry as u16 + 5), code)
        })
        .collect();
    let rooms: std::collections::HashSet<u16> = entries.iter().map(|e| e.0).collect();
    let mut ok = rooms.len() == at::TELEPORTER_COUNT
        && entries
            .iter()
            .all(|(room, code)| *room < 512 && code.iter().all(u8::is_ascii_uppercase));
    println!(
        "  the teleporter table: {} codes in {} rooms {}",
        entries.len(),
        rooms.len(),
        if ok { "ok" } else { "FAILED" }
    );
    let rom = dir.join("48.rom");
    if !rom.exists() {
        println!("  no 48.rom: the booths were NOT walked into");
        return ok;
    }
    // Into play first, as a player would, on a machine with the real ROM,
    // which handles the interrupts while the booth is stepped through.
    let mut base = m.with_rom(&read(dir, "48.rom"));
    base.watch = vec![routine::MAIN_LOOP];
    let mut script = Script(0xBEEF);
    for frame in 0..600 {
        script.apply(&mut base, frame.min(399));
        if base.run_frame().contains(&routine::MAIN_LOOP) {
            break;
        }
    }
    base.zx.release_all_keys();
    let mut printed_ok = 0;
    for &(room, code) in &entries {
        let mut m = base.clone();
        m.zx.write16(at::ROOM, room);
        m.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
        if !m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000) {
            println!("  room {room} did not finish entering: FAILED");
            ok = false;
            continue;
        }
        let z = &mut m.zx;
        let end = z.read16(at::MARKERS_END).max(at::MARKERS);
        let booth = (at::MARKERS..end)
            .step_by(3)
            .find(|&a| z.mem[usize::from(a) + 2] == BOOTH_MARKER);
        let Some(marker) = booth else {
            println!("  room {room} has no booth: FAILED");
            ok = false;
            continue;
        };
        // Stand Blob on the booth, carry on as the play loop does, on a
        // fresh frame with interrupts on, and read what is printed from the
        // moment the booth starts.
        z.mem[usize::from(at::ENTITIES) + 5] = z.mem[usize::from(marker)];
        z.mem[usize::from(at::ENTITIES) + 6] = z.mem[usize::from(marker) + 1];
        z.t = 0;
        z.set_interrupts(true);
        z.kempston = 0x01;
        let (mut in_booth, mut printed) = (false, Vec::new());
        for _ in 0..400 {
            if !z.run_until_any(&[routine::TELEPORT_BOOTH, PRINT_A_2], 1) {
                continue;
            }
            if z.pc() == routine::TELEPORT_BOOTH {
                in_booth = true;
                z.kempston = 0;
            } else if in_booth {
                printed.push(z.a());
                if printed.len() > 80 {
                    break;
                }
            }
        }
        let good = in_booth
            && printed.windows(5).any(|w| w == code)
            && teleporter_code(&z.mem[..], room) == Some(code);
        printed_ok += usize::from(good);
        ok &= good;
        // From the first booth, type the next booth's code: the game should
        // move Blob there and record the room as entered by teleport.
        if room == entries[0].0 && good {
            let (to, next) = entries[1];
            // Let the booth finish printing and wait for a key.
            for _ in 0..60 {
                m.zx.release_all_keys();
                m.run_frame();
            }
            let letter = |c: u8| {
                Key::by_name(&(c as char).to_ascii_lowercase().to_string()).expect("a letter")
            };
            for &c in &next {
                for frame in 0..15 {
                    m.zx.release_all_keys();
                    m.zx.set_key(letter(c), frame < 5);
                    m.run_frame();
                }
            }
            // The room number changes as the booth takes the code; the game
            // enters the room, with its reason set, when the booth is done.
            let (mut arrived, mut reason) = (false, 0xFF);
            for _ in 0..300 {
                m.zx.release_all_keys();
                let mut entering = None;
                m.run_frame_observing(|z| {
                    if z.pc() == routine::ENTER_ROOM && entering.is_none() {
                        entering = Some((z.read16(at::ROOM), z.mem[usize::from(at::ENTRY_REASON)]));
                    }
                });
                if let Some((room, why)) = entering {
                    (arrived, reason) = (room == to, why);
                    break;
                }
            }
            let teleported = arrived && reason == sidekick::starquake::entry::TELEPORTED;
            println!(
                "  typing another booth's code: arrived in room {to} {}, entry reason {reason} {}",
                if arrived { "yes" } else { "no" },
                if teleported { "ok" } else { "FAILED" }
            );
            ok &= teleported;
        }
    }
    println!(
        "  walking into each booth prints its code: {printed_ok} of {} {}",
        entries.len(),
        if printed_ok == entries.len() {
            "ok"
        } else {
            "FAILED"
        }
    );
    ok
}

/// The entry points the panel follows: the menu first and then play, and
/// End this game's keys ending a game in every control method.
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
        let ended = ends_the_game(&m);
        let line = match ended {
            Some((over, menu)) => format!("game over after {over} frames, menu after {menu} ok"),
            None => "FAILED".to_string(),
        };
        ok &= ended.is_some();
        println!("  method {method}: End this game {line}");
    }
    ok &= teleporters_check(dir);
    ok &= visited_check(dir);
    ok &= pieces_check(dir);
    ok &= graphics_check(dir);
    println!(
        "facts: the panel's entry points {}",
        if ok { "hold" } else { "do NOT hold" }
    );
    ok
}

/// Reads the map by having the game draw every room, then walks Blob at
/// random from `walks` rooms, and fails if he ever leaves a room through an
/// edge the map shows closed or crosses a wall it shows inside a room.
fn map_check(dir: &Path, walks: usize) -> bool {
    use sidekick::starquake::{CORE_ROOM, all_rooms, at, routine};
    let mut base = machine(dir);
    let rooms = all_rooms(&base);
    let openings = sidekick::map::openings(&rooms, CORE_ROOM);
    let parts: Vec<_> = rooms.iter().map(|r| r.open.clone()).collect();
    let graph = sidekick::map::Graph::new(&rooms, CORE_ROOM);
    // Into play, as a player would.
    base.watch = vec![routine::MAIN_LOOP];
    let mut script = Script(0xBEEF);
    for frame in 0..600 {
        script.apply(&mut base, frame.min(399));
        if base.run_frame().contains(&routine::MAIN_LOOP) {
            break;
        }
    }
    base.watch = vec![routine::MODAL, routine::DEATH, routine::MAIN_LOOP];
    let mut rng = Script(0x3A9);
    let mut next = |n: u64| {
        rng.0 ^= rng.0 << 13;
        rng.0 ^= rng.0 >> 7;
        rng.0 ^= rng.0 << 17;
        rng.0 % n
    };
    let (mut crossings, mut positions, mut failures) = (0u64, 0u64, 0u64);
    // Level 5's graph against the same walks (#10): every crossing between
    // two places Blob stood in must be a way.
    let (mut checked, mut missing) = (0u64, 0u64);
    for walk in 0..walks {
        let mut m = base.clone();
        // Start each walk in a different room, entered as walking in.
        let start = next(512) as u16;
        if start != sidekick::starquake::CORE_ROOM {
            let z = &mut m.zx;
            z.write16(at::ROOM, start);
            z.mem[usize::from(at::ENTRY_REASON)] = 0;
            if !m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000) {
                continue;
            }
            let z = &mut m.zx;
            z.t = 0;
            z.set_interrupts(true);
        }
        let mut part = 0;
        let mut last_place: Option<sidekick::map::Place> = None;
        let mut pending: Option<(sidekick::map::Place, u16)> = None;
        for frame in 0..1500 {
            if frame % 25 == 0 {
                m.zx.release_all_keys();
                m.zx.kempston = next(16) as u8;
            }
            let from = m.zx.read16(at::ROOM);
            let hits = m.run_frame();
            if hits
                .iter()
                .any(|&h| h == routine::MODAL || h == routine::DEATH)
            {
                // A door, booth or pyramid screen, or a lost life: this walk's
                // view of where Blob is starts again.
                part = 0;
                last_place = None;
                pending = None;
                continue;
            }
            let z = &m.zx;
            let room = z.read16(at::ROOM);
            if room != from {
                part = 0;
                // Only a step to a room beside: a teleport lands anywhere.
                let beside = [1, 0xFFFF, 16, 0xFFF0].contains(&room.wrapping_sub(from));
                pending = last_place
                    .filter(|p| beside && p.0 != CORE_ROOM && room != CORE_ROOM)
                    .map(|p| (p, room));
                last_place = None;
                let o = openings[usize::from(from) % 512];
                let edge = match room.wrapping_sub(from) {
                    1 => Some(("right", o.right)),
                    0xFFFF => Some(("left", o.left)),
                    16 => Some(("bottom", o.down)),
                    0xFFF0 => Some(("top", o.up)),
                    _ => None,
                };
                if let Some((name, open)) = edge {
                    crossings += 1;
                    if !open {
                        failures += 1;
                        println!(
                            "map: walk {walk}: left room {from} through its {name} edge, shown closed"
                        );
                    }
                }
            }
            let (x, y) = (
                z.mem[usize::from(at::ENTITIES) + 5],
                z.mem[usize::from(at::ENTITIES) + 6],
            );
            if x & 7 != 0 || room >= 512 {
                continue;
            }
            let place = graph.place(room, x, y);
            if place.1 != 0 {
                if let Some((was, to)) = pending.take()
                    && to == room
                {
                    checked += 1;
                    if !graph.ways(was).contains(&place) {
                        missing += 1;
                        println!(
                            "map: walk {walk}: from room {} part {} into room {room} part {}, which the graph has no way for",
                            was.0, was.1, place.1
                        );
                    }
                }
                last_place = Some(place);
            }
            let here = parts[usize::from(room)].at((0xBF - y) >> 3, x >> 3);
            if here == 0 {
                continue;
            }
            positions += 1;
            if part == 0 {
                part = here;
            } else if here != part {
                failures += 1;
                println!(
                    "map: walk {walk}: crossed a wall inside room {room} at ({x:#04x}, {y:#04x})"
                );
                part = here;
            }
        }
    }
    let open: usize = openings
        .iter()
        .map(|o| {
            [o.left, o.right, o.up, o.down]
                .into_iter()
                .filter(|&e| e)
                .count()
        })
        .sum();
    let divided = openings
        .iter()
        .filter(|o| o.divides.cells.iter().any(|&bits| bits != 0))
        .count();
    println!(
        "map: {open} of 2048 edges open, {divided} rooms divided inside; {crossings} crossings and {positions} positions walked, {failures} against the map"
    );
    // The graph's reach from where play starts.
    let start = {
        let z = &base.zx;
        graph.place(
            z.read16(at::ROOM),
            z.mem[usize::from(at::ENTITIES) + 5],
            z.mem[usize::from(at::ENTITIES) + 6],
        )
    };
    let mut seen = std::collections::BTreeSet::from([start]);
    let mut todo = vec![start];
    while let Some(p) = todo.pop() {
        for &to in graph.ways(p) {
            if seen.insert(to) {
                todo.push(to);
            }
        }
    }
    let reach = seen
        .iter()
        .map(|p| p.0)
        .collect::<std::collections::BTreeSet<u16>>()
        .len();
    let (places, ways) = graph
        .places()
        .fold((0, 0), |(n, w), p| (n + 1, w + graph.ways(p).len()));
    println!(
        "map: level 5's graph: {places} places, {ways} ways; from the start it reaches {reach} rooms without a teleport; {checked} crossings checked against it, {missing} with no way"
    );
    let passages_ok = passages_check(&base, &rooms, &openings);
    failures == 0 && crossings > 0 && passages_ok && missing == 0 && checked > 0
}

/// Every wall passage walked into from each side Blob can stand beside it,
/// on copies of `base` in play: he must reach the room on that side, and the
/// map must join the two rooms that way, and join no two rooms a walk does
/// not (#10).
fn passages_check(
    base: &Machine,
    rooms: &[sidekick::map::Room],
    openings: &[sidekick::map::Openings],
) -> bool {
    use sidekick::map::COLS;
    use sidekick::starquake::{at, routine};
    let (mut walked, mut failures) = (0, 0);
    let mut joined = std::collections::BTreeSet::new();
    for (i, r) in rooms.iter().enumerate() {
        let Some((row, col)) = r.passage else {
            continue;
        };
        let room = i as u16;
        // Blob's top-left column beside the tile, the Kempston input that
        // walks him into it, and the room that way.
        for (beside, input, to) in [
            (i16::from(col) - 2, 1u8, room + 1),
            (i16::from(col) + 4, 2, room.wrapping_sub(1)),
        ] {
            let Some((c, r_top)) =
                (0..31)
                    .contains(&beside)
                    .then_some(beside as u8)
                    .and_then(|c| {
                        (row.saturating_sub(1)..=row + 1)
                            .find(|&rr| r.shut.at(rr, c) != 0)
                            .map(|rr| (c, rr))
                    })
            else {
                continue;
            };
            let mut m = base.clone();
            m.zx.write16(at::ROOM, room);
            m.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
            if !m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000) {
                continue;
            }
            m.zx.t = 0;
            m.zx.set_interrupts(true);
            m.zx.mem[usize::from(at::ENTITIES) + 5] = c * 8;
            m.zx.mem[usize::from(at::ENTITIES) + 6] = 143 - 8 * (r_top - 6);
            m.watch = vec![routine::MODAL, routine::DEATH];
            let mut reached = None;
            for _ in 0..80 {
                m.zx.release_all_keys();
                m.zx.kempston = input;
                if !m.run_frame().is_empty() {
                    break;
                }
                let now = m.zx.read16(at::ROOM);
                if now != room {
                    reached = Some(now);
                    break;
                }
            }
            walked += 1;
            let shown = if input == 1 {
                room % COLS != COLS - 1 && openings[i].right
            } else {
                !room.is_multiple_of(COLS) && openings[i].left
            };
            if reached == Some(to) && shown {
                joined.insert((room.min(to), room.max(to)));
            } else {
                failures += 1;
                println!(
                    "map: walking into room {room}'s passage toward {to} reached {reached:?}; the map joins them: {shown}"
                );
            }
        }
    }
    // No join the walks did not make.
    for (i, o) in openings.iter().enumerate() {
        let room = i as u16;
        // Open on the right only through a passage, not a gap in the wall.
        if o.right && !rooms[i].openings.right && !joined.contains(&(room, room + 1)) {
            failures += 1;
            println!(
                "map: rooms {room} and {} are joined by passages no walk went through",
                room + 1
            );
        }
    }
    println!(
        "map: {walked} walks into wall passages, {} pairs of rooms joined by them, {failures} against the map",
        joined.len()
    );
    failures == 0 && walked > 0
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
        Some("map") => {
            let walks = args.get(2).and_then(|f| f.parse().ok()).unwrap_or(120);
            std::process::exit(i32::from(!map_check(&dir, walks)));
        }
        Some("shot") => {
            let frames = args.get(2).and_then(|f| f.parse().ok()).unwrap_or(600);
            shots(
                &dir,
                frames,
                &PathBuf::from(args.get(3).map_or("shots", String::as_str)),
            );
        }
        _ => {
            eprintln!(
                "usage: sk-check rom|entry|keys|facts|map|shot <assets-dir> [frames] [out-dir]"
            );
            std::process::exit(2);
        }
    }
}
