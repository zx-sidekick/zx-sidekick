//! Checks of ZX Sidekick against the player's own copy of Manic Miner
//! (#142). Each needs the tape in the folder given; `entry` and the ROM's
//! half of `font` need a Spectrum 48K ROM there too. They are the contract
//! the machine keeps with the game, and none runs in CI: the tape and the
//! ROM are never committed.
//!
//! `manicminer-check <entry|keys|facts|font|all> <assets-dir>`

use std::path::Path;

use manicminer::facts::{ENTRY_PC, ENTRY_SP, FONT, PAUSE_KEYS, at, is_supported_tape, routine};
use manicminer::{JOY_FIRE, JOY_RIGHT, Machine};
use zx_spectrum::Key;

/// The names the tape goes by in the assets folder.
const TAPES: [&str; 2] = ["manicminer.tap", "manic.tap"];

/// Reads `name` from `dir`, or stops with why not.
fn read(dir: &Path, name: &str) -> Vec<u8> {
    std::fs::read(dir.join(name))
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.join(name).display()))
}

/// The supported tape in `dir`, by either of its names.
fn tape(dir: &Path) -> Vec<u8> {
    for name in TAPES {
        if let Ok(bytes) = std::fs::read(dir.join(name)) {
            assert!(
                is_supported_tape(&bytes),
                "{name} is not the Bug-Byte tape this version supports"
            );
            return bytes;
        }
    }
    panic!("no {} in {}", TAPES.join(" or "), dir.display());
}

/// The game as the program starts it, with our font.
fn machine(dir: &Path) -> Machine {
    manicminer::start(&tape(dir), &manicminer::font::OURS).expect("the tape loads")
}

fn key(name: &str) -> Key {
    Key::by_name(name).expect("a key name")
}

/// Runs `frames` frames, returning every watched address reached, in order.
fn run(m: &mut Machine, frames: u64) -> Vec<u16> {
    let mut hits = Vec::new();
    for _ in 0..frames {
        hits.extend(m.run_frame());
    }
    hits
}

/// From the start through the title to a game in the first cavern, ENTER
/// held until the main loop runs.
fn into_play(dir: &Path) -> Machine {
    let mut m = machine(dir);
    m.watch = vec![routine::MAIN_LOOP];
    run(&mut m, 100);
    m.zx.set_key(key("enter"), true);
    for _ in 0..600 {
        if m.run_frame().contains(&routine::MAIN_LOOP) {
            m.zx.release_all_keys();
            run(&mut m, 40);
            return m;
        }
    }
    panic!("no game started from the title with ENTER held");
}

/// Presses, or lets go, one of Willy's controls.
type Press = fn(&mut Machine, bool);

/// Willy's cell in the attribute buffer.
fn cell(m: &Machine) -> u16 {
    m.zx.read16(at::WILLY_CELL)
}

/// The real ROM's loader, fed the tape block by block at LD-BYTES, runs the
/// BASIC loader through to its `RANDOMIZE USR`, which must arrive where the
/// program starts the game, with the stack recorded.
fn entry_check(dir: &Path) -> bool {
    const LD_BYTES: u16 = 0x0556;
    /// Where LD-BYTES sends its own return, pushed before it loads.
    const SA_LD_RET: u16 = 0x053F;
    let tap = tape(dir);
    let mut blocks = Vec::new();
    let mut i = 0;
    while i + 2 <= tap.len() {
        let len = usize::from(tap[i]) | usize::from(tap[i + 1]) << 8;
        blocks.push(tap[i + 2..i + 2 + len].to_vec());
        i += 2 + len;
    }
    let mut m = Machine::blank(0, 0).with_rom(&read(dir, "48.rom"));
    let z = &mut m.zx;
    // Dismiss the copyright message, then LOAD "" and ENTER.
    let typing: [(u64, &[&str]); 10] = [
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
        if z.pc() == LD_BYTES {
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
                // Every block is in: the loader's BASIC goes on to its USR.
                let arrived = z.run_until_any(&[ENTRY_PC], 500);
                let (pc, sp) = (z.pc(), z.sp());
                let ok = arrived && (pc, sp) == (ENTRY_PC, ENTRY_SP);
                println!(
                    "entry: the loader goes to {pc:04x} with the stack at {sp:04x}{}",
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

/// The keyboard and the Kempston joystick each move Willy and make him
/// jump; a pause key is kept from the game, which goes on running its loop;
/// and Start on a pad starts a game from the title.
fn keys_check(dir: &Path) -> bool {
    let mut ok = true;
    let base = into_play(dir);
    if base.zx.mem[usize::from(at::KEMPSTON)] != 1 {
        println!("keys: the game did not find the Kempston joystick");
        ok = false;
    }
    // Each way of moving right, and of jumping.
    let ways: [(&str, Press, Press); 2] = [
        (
            "keyboard",
            |m, down| m.zx.set_key(key("p"), down),
            |m, down| m.zx.set_key(key("space"), down),
        ),
        (
            "Kempston",
            |m, down| m.zx.kempston = if down { JOY_RIGHT } else { 0 },
            |m, down| m.zx.kempston = if down { JOY_FIRE } else { 0 },
        ),
    ];
    for (name, right, jump) in ways {
        let mut m = base.clone();
        let before = cell(&m);
        right(&mut m, true);
        run(&mut m, 60);
        right(&mut m, false);
        let moved = cell(&m) != before;
        jump(&mut m, true);
        let mut jumped = false;
        for _ in 0..8 {
            m.run_frame();
            jumped |= m.zx.mem[usize::from(at::AIRBORNE)] == 1;
        }
        jump(&mut m, false);
        println!(
            "keys: {name}: Willy {} and {}",
            if moved { "moved" } else { "did NOT move" },
            if jumped { "jumped" } else { "did NOT jump" }
        );
        ok &= moved && jumped;
    }
    // A pause key: reported, kept from the game, and the loop goes on.
    let mut m = base.clone();
    m.watch = vec![routine::MAIN_LOOP];
    let (row, bits) = PAUSE_KEYS;
    m.zx.keys[row] &= !(bits & 0x01);
    let mut reported = false;
    let mut looping = 0;
    for _ in 0..20 {
        looping += m.run_frame().len();
        reported |= m.rules.pause_pressed;
    }
    println!(
        "keys: the pause key was {} and the game {}",
        if reported { "reported" } else { "NOT reported" },
        if looping > 3 {
            "went on playing"
        } else {
            "stopped in its own pause"
        }
    );
    ok &= reported && looping > 3;
    // Start on a pad, and nothing else, starts a game.
    let mut m = machine(dir);
    m.watch = vec![routine::NEW_GAME];
    run(&mut m, 100);
    m.rules.start = true;
    let started = run(&mut m, 600).contains(&routine::NEW_GAME);
    println!(
        "keys: Start {} a game from the title",
        if started { "started" } else { "did NOT start" }
    );
    ok && started
}

/// A game played by nobody arrives at the game's routines in order: the
/// title, a new game, the main loop, a life lost, and game over back to the
/// title; and CAPS SHIFT with SPACE quits a game to the title.
fn facts_check(dir: &Path) -> bool {
    let order = [
        routine::TITLE,
        routine::NEW_GAME,
        routine::MAIN_LOOP,
        routine::LOSE_LIFE,
        routine::GAME_OVER,
        routine::TITLE,
    ];
    let mut m = machine(dir);
    m.watch = order.to_vec();
    let mut seen = Vec::new();
    let mut enter = true;
    for frame in 0..40_000u64 {
        m.zx.release_all_keys();
        if enter && (100..300).contains(&frame) {
            m.zx.set_key(key("enter"), true);
        }
        for hit in m.run_frame() {
            if seen.last() != Some(&hit) {
                seen.push(hit);
            }
        }
        if seen.contains(&routine::GAME_OVER) && seen.last() == Some(&routine::TITLE) {
            break;
        }
        enter &= !seen.contains(&routine::MAIN_LOOP);
    }
    let mut i = 0;
    for hit in &seen {
        if i < order.len() && *hit == order[i] {
            i += 1;
        }
    }
    let in_order = i == order.len();
    println!(
        "facts: title, new game, play, a life lost and game over {}",
        if in_order {
            "came in order"
        } else {
            "did NOT all come in order"
        }
    );
    // Quit: CAPS SHIFT and SPACE in play go back to the title.
    let mut m = into_play(dir);
    m.watch = vec![routine::TITLE];
    m.zx.set_key(key("caps"), true);
    m.zx.set_key(key("space"), true);
    let quit = run(&mut m, 20).contains(&routine::TITLE);
    println!(
        "facts: CAPS SHIFT and SPACE {} to the title",
        if quit { "went back" } else { "did NOT go back" }
    );
    in_order && quit
}

/// The cavern's name, as the game prints it on the screen, is drawn from the
/// bytes where the ROM's character set would be: ours with no ROM, and the
/// ROM's own when the player's ROM gives them.
fn font_check(dir: &Path) -> bool {
    let mut fonts = vec![("ours", manicminer::font::OURS)];
    if let Ok(rom) = std::fs::read(dir.join("48.rom")) {
        match manicminer::font::from_rom(&rom) {
            Some(font) => fonts.push(("the ROM's", font)),
            None => println!("font: 48.rom is not the ROM this version knows; its half skipped"),
        }
    }
    let mut ok = true;
    for (name, font) in fonts {
        let mut m = into_play(dir);
        let at = usize::from(FONT);
        m.zx.mem[at..at + 768].copy_from_slice(&font);
        // The name is printed as a cavern starts: start this one again.
        let mut m2 = manicminer::start(&tape(dir), &font).expect("the tape loads");
        m2.zx.mem[usize::from(at::CAVERN)] = m.zx.mem[usize::from(at::CAVERN)];
        let m = {
            m2.watch = vec![routine::MAIN_LOOP];
            run(&mut m2, 100);
            m2.zx.set_key(key("enter"), true);
            let mut started = false;
            for _ in 0..600 {
                if m2.run_frame().contains(&routine::MAIN_LOOP) {
                    started = true;
                    break;
                }
            }
            assert!(started, "a game starts");
            m2
        };
        // The name's row on the screen: character row 16, from the working
        // buffer's 32 letters.
        let mut right = 0;
        for col in 0..32usize {
            let letter = usize::from(m.zx.mem[usize::from(at::CAVERN_NAME) + col]);
            let glyph = &font[(letter - 32) * 8..(letter - 32) * 8 + 8];
            let drawn: Vec<u8> = (0..8)
                .map(|line| {
                    let y = 128 + line;
                    m.zx.mem[0x4000 + zx_core::screen::line_offset(y) + col]
                })
                .collect();
            if drawn == glyph {
                right += 1;
            }
        }
        println!("font: with {name}, {right} of 32 letters of the cavern's name drawn from it");
        ok &= right == 32;
    }
    ok
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "manicminer-check <entry|keys|facts|font|all> <assets-dir>";
    let (Some(command), Some(dir)) = (args.first(), args.get(1)) else {
        eprintln!("usage: {usage}");
        std::process::exit(2);
    };
    let dir = Path::new(dir);
    let ok = match command.as_str() {
        "entry" => entry_check(dir),
        "keys" => keys_check(dir),
        "facts" => facts_check(dir),
        "font" => font_check(dir),
        "all" => {
            let results = [
                entry_check(dir),
                keys_check(dir),
                facts_check(dir),
                font_check(dir),
            ];
            results.iter().all(|&r| r)
        }
        _ => {
            eprintln!("usage: {usage}");
            std::process::exit(2);
        }
    };
    std::process::exit(i32::from(!ok));
}
