//! Checks of ZX Sidekick against the player's own copy of Manic Miner
//! (#142). Each needs the tape in the folder given; `entry`, and `font`'s
//! comparison with the ROM, need a Spectrum 48K ROM there too. They are the contract
//! the machine keeps with the game, and none runs in CI: the tape and the
//! ROM are never committed.
//!
//! `manicminer-check <entry|keys|facts|font|training|all> <assets-dir>`

use std::path::Path;

use manicminer::facts::{
    ENTRY_PC, ENTRY_SP, FONT, PAUSE_KEYS, ROM_SHA1, at, is_supported_tape, routine,
};
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

/// The game as the program starts it.
fn machine(dir: &Path) -> Machine {
    manicminer::start(&tape(dir)).expect("the tape loads")
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
    // The demo: the main loop runs in it too, told from a game by DEMO.
    let mut m = machine(dir);
    m.watch = vec![routine::MAIN_LOOP];
    let demo =
        (0..4_000).any(|_| !m.run_frame().is_empty() && m.zx.mem[usize::from(at::DEMO)] != 0);
    let game = into_play(dir).zx.mem[usize::from(at::DEMO)] == 0;
    println!(
        "facts: the demo byte is {} in the demo and {} in a game",
        if demo { "set" } else { "NOT set" },
        if game { "clear" } else { "NOT clear" }
    );
    in_order && quit && demo && game && guide_check(dir)
}

/// Pressing `keys` for a frame's worth of play, as a player would.
fn hold(m: &mut Machine, keys: &[&str], frames: u64) -> Vec<u16> {
    m.zx.release_all_keys();
    for k in keys {
        m.zx.set_key(key(k), true);
    }
    run(m, frames)
}

/// What guidance reads (#153), against the game in every cavern: the
/// cavern's own definition on the tape; its cells as the game lays them;
/// each patrol kept by its guardian through play; Willy's cell where his
/// height says; an item taken leaving one fewer; and the air's passes
/// counting down one a pass of the main loop.
fn guide_check(dir: &Path) -> bool {
    use manicminer::guide::{self, Patrol, Tile};
    use manicminer::play::Training;
    // Nothing kills, so play goes on in the cavern it is about.
    let safe = Training {
        lives: true,
        air: true,
        falls: true,
        guardians: true,
        nasties: true,
    };
    let mut ok = true;
    let (mut defined, mut laid, mut kept, mut patrols, mut willy) = (0, 0, 0, 0, 0);
    let mut taken = None;
    for cavern in 0..20u8 {
        let Some(mut m) = in_cavern(dir, cavern, safe) else {
            println!("facts: cavern {cavern} was NOT reached");
            ok = false;
            continue;
        };
        let c = guide::read(&m.zx.mem[..]);
        // The definition on the tape: the second half of the cavern's 1K is
        // what the game copies to where the facts are read.
        let def = usize::from(at::CAVERNS) + 1024 * usize::from(cavern);
        let copy = |a: u16, len: usize| {
            let off = usize::from(a - at::CAVERN_COPY);
            m.zx.mem[usize::from(a)..usize::from(a) + len]
                == m.zx.mem[def + 512 + off..def + 512 + off + len]
        };
        let items_where = (0..c.items.len() as u16).all(|i| copy(at::ITEMS + 5 * i + 1, 2));
        if copy(at::TILES, 72)
            && copy(at::CONVEYOR, 4)
            && copy(at::PORTAL_CELL, 2)
            && items_where
            && c.number == cavern
        {
            defined += 1;
        }
        // The empty cavern's cells are its own layout, and the conveyor's
        // cells are conveyor.
        let cells_ok = m.zx.mem[usize::from(at::EMPTY_CELLS)..usize::from(at::EMPTY_CELLS) + 512]
            == m.zx.mem[def..def + 512];
        let conveyor_ok = c.conveyor.is_none_or(|v| {
            (0..v.length).all(|k| {
                c.tile(usize::from(v.cell.row), usize::from(v.cell.col + k)) == Tile::Conveyor
            })
        });
        if cells_ok && conveyor_ok {
            laid += 1;
        }
        // Play: each guardian inside its patrol, Willy's cell at his height.
        let mut inside = vec![true; c.patrols.len()];
        let mut willy_ok = true;
        for frame in 0..1000u64 {
            let keys: &[&str] = match (frame / 100) % 4 {
                0 => &["p"],
                1 => &["o", "space"],
                2 => &["p", "space"],
                _ => &["o"],
            };
            hold(&mut m, keys, 1);
            let mem = &m.zx.mem;
            let now = guide::read(&mem[..]);
            let mut positions = Vec::new();
            for g in 0..4u16 {
                let r = usize::from(at::HORIZONTAL + 7 * g);
                if mem[r] == 0xFF {
                    break;
                }
                if mem[r] != 0 {
                    positions.push(mem[r + 1] & 31);
                }
            }
            for g in 0..4u16 {
                let r = usize::from(at::VERTICAL + 7 * g);
                if positions.len() >= c.patrols.len() || mem[r] == 0xFF {
                    break;
                }
                positions.push(mem[r + 2] / 8);
            }
            for ((p, at), inside) in c.patrols.iter().zip(&positions).zip(&mut inside) {
                let (Patrol::Across { from, to, .. } | Patrol::Down { from, to, .. }) = *p;
                *inside &= (from..=to).contains(at);
            }
            // Willy's height is kept doubled: a cell is 16 of it.
            let height = mem[usize::from(at::WILLY_Y)] / 16;
            willy_ok &= now.willy.is_some_and(|w| w.row == height);
            if taken.is_none() && now.items_left() + 1 == c.items_left() {
                taken = Some(cavern);
            }
        }
        kept += inside.iter().filter(|&&k| k).count();
        patrols += inside.len();
        willy += usize::from(willy_ok);
    }
    println!("facts: {defined} of 20 caverns read as the tape defines them");
    println!(
        "facts: {laid} of 20 caverns' cells as the game lays them, the conveyor on its own tiles"
    );
    println!("facts: {kept} of {patrols} guardians kept their patrols through play");
    println!("facts: Willy's cell was at his height in {willy} of 20 caverns");
    println!(
        "facts: {}",
        taken.map_or("NO item was taken in play".to_string(), |c| format!(
            "an item taken in cavern {c} left one fewer"
        ))
    );
    ok &= defined == 20 && laid == 20 && kept == patrols && patrols > 0 && willy == 20;
    ok &= taken.is_some();
    // The portal: shut with items left, open once none is, staged by
    // clearing the items' attributes as taking them does.
    let mut m = into_play(dir);
    let shut = !guide::read(&m.zx.mem[..]).portal_open;
    let mut a = usize::from(at::ITEMS);
    while m.zx.mem[a] != 0xFF {
        m.zx.mem[a] = 0;
        a += 5;
    }
    run(&mut m, 20);
    let open = guide::read(&m.zx.mem[..]).portal_open;
    println!(
        "facts: the portal was {} with items left and {} once none was",
        if shut { "shut" } else { "NOT shut" },
        if open { "open" } else { "NOT open" }
    );
    ok &= shut && open;
    // The air: one pass fewer each pass of the main loop, until it is out.
    let mut m = into_play(dir);
    let before = guide::read(&m.zx.mem[..]).air_passes;
    let passes = run(&mut m, 1000)
        .iter()
        .filter(|&&h| h == routine::MAIN_LOOP)
        .count() as u32;
    let after = guide::read(&m.zx.mem[..]).air_passes;
    let air_ok = before - after == passes;
    println!(
        "facts: the air's passes went {before} to {after} over {passes} passes of the main loop{}",
        if air_ok { "" } else { "  NOT one a pass" }
    );
    ok && air_ok
}

/// The cavern's name, as the game prints it on the screen, is drawn letter
/// for letter from the character set placed where the ROM has it; and those
/// bytes are the ROM's own, when the ROM is in the folder to compare with.
fn font_check(dir: &Path) -> bool {
    let set = &manicminer::font::CHARACTER_SET;
    let mut ok = true;
    match std::fs::read(dir.join("48.rom")) {
        Ok(rom) if zx_core::sha1::sha1_hex(&rom) == ROM_SHA1 => {
            let at = usize::from(FONT);
            let same = rom[at..at + 768] == set[..];
            println!(
                "font: the character set {} the 48K ROM's",
                if same { "is" } else { "is NOT" }
            );
            ok &= same;
        }
        Ok(_) => println!("font: 48.rom is not the ROM this version knows; not compared"),
        Err(_) => println!("font: no 48.rom to compare the character set with"),
    }
    let m = into_play(dir);
    // The name's row on the screen: character row 16, from the working
    // buffer's 32 letters.
    let mut right = 0;
    for col in 0..32usize {
        let letter = usize::from(m.zx.mem[usize::from(at::CAVERN_NAME) + col]);
        let glyph = &set[(letter - 32) * 8..(letter - 32) * 8 + 8];
        let drawn: Vec<u8> = (0..8)
            .map(|line| m.zx.mem[0x4000 + zx_core::screen::line_offset(128 + line) + col])
            .collect();
        if drawn == glyph {
            right += 1;
        }
    }
    println!("font: {right} of 32 letters of the cavern's name drawn from the character set");
    ok && right == 32
}

/// How Willy was killed: the instruction that sent the game to its kill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Death {
    Fall,
    Guardian,
    Nasty,
}

/// Runs `frames` frames with `keys` held, and says how Willy was first
/// killed, if he was.
fn death(m: &mut Machine, frames: u64, keys: &[&str]) -> Option<Death> {
    use manicminer::facts::steer;
    for k in keys {
        m.zx.set_key(key(k), true);
    }
    let mut prev = 0u16;
    let mut found = None;
    for _ in 0..frames {
        m.run_frame_observing(|z| {
            let pc = z.pc();
            if found.is_none() && (pc == 0x8D05 || pc == 0x8D06) {
                found = if prev == steer::FALL_KILL {
                    Some(Death::Fall)
                } else if steer::NASTY_KILLS.contains(&prev) {
                    Some(Death::Nasty)
                } else if steer::GUARDIAN_DRAWS.iter().any(|&g| prev == g + 3) {
                    Some(Death::Guardian)
                } else {
                    None
                };
            }
            prev = pc;
        });
        if found.is_some() {
            break;
        }
    }
    m.zx.release_all_keys();
    found
}

/// A game in play in `cavern`, reached by typing the game's own cheat
/// ([`manicminer::play::Play::go_to`]), with `training` in force.
fn in_cavern(dir: &Path, cavern: u8, training: manicminer::play::Training) -> Option<Machine> {
    let mut m = into_play(dir);
    m.rules.training = training;
    m.rules.go_to = Some(cavern);
    m.watch = vec![routine::MAIN_LOOP];
    for _ in 0..400 {
        m.run_frame();
        if m.rules.go_to.is_none() && m.zx.mem[usize::from(at::CAVERN)] == cavern {
            run(&mut m, 20);
            return Some(m);
        }
    }
    None
}

/// Training mode (#148): going to any cavern by the game's own cheat; each
/// switch keeps Willy from the death it is about, which the same play
/// without it meets; air and lives stay where they are; and with every
/// switch off the machine changes nothing the game would not.
fn training_check(dir: &Path) -> bool {
    use manicminer::play::Training;
    let mut ok = true;
    // Every cavern by the cheat.
    let reached = (0..20u8)
        .filter(|&c| in_cavern(dir, c, Training::default()).is_some())
        .count();
    println!("training: went to {reached} of 20 caverns by the game's own cheat");
    ok &= reached == 20;
    // Each switch against the death it is about, in play that meets it.
    let cases: [(&str, Training, u8, &[&str], Death); 5] = [
        (
            "safe falls",
            Training {
                falls: true,
                ..Training::default()
            },
            5,
            &["p"],
            Death::Fall,
        ),
        (
            "no harm from nasties",
            Training {
                nasties: true,
                ..Training::default()
            },
            9,
            &["p", "space"],
            Death::Nasty,
        ),
        (
            "no harm from guardians",
            Training {
                guardians: true,
                ..Training::default()
            },
            0,
            &["p", "space"],
            Death::Guardian,
        ),
        (
            "no harm from Eugene",
            Training {
                guardians: true,
                ..Training::default()
            },
            4,
            &[],
            Death::Guardian,
        ),
        (
            "no harm from the Kong Beast",
            Training {
                guardians: true,
                ..Training::default()
            },
            7,
            &[],
            Death::Guardian,
        ),
    ];
    for (name, on, cavern, keys, meant) in cases {
        let off =
            in_cavern(dir, cavern, Training::default()).map(|mut m| death(&mut m, 1500, keys));
        let with = in_cavern(dir, cavern, on).map(|mut m| death(&mut m, 1500, keys));
        let good = off == Some(Some(meant)) && with.is_some_and(|d| d != Some(meant));
        println!(
            "training: {name}: without it {:?}, with it {:?}{}",
            off.flatten(),
            with.flatten(),
            if good { "" } else { "  NOT AS PROMISED" }
        );
        ok &= good;
    }
    // Endless lives: a death takes none.
    for (lives, name) in [(false, "off"), (true, "on")] {
        let Some(mut m) = in_cavern(
            dir,
            9,
            Training {
                lives,
                ..Training::default()
            },
        ) else {
            ok = false;
            continue;
        };
        let before = m.zx.mem[usize::from(at::LIVES)];
        m.watch = vec![routine::LOSE_LIFE, routine::MAIN_LOOP];
        m.zx.set_key(key("p"), true);
        m.zx.set_key(key("space"), true);
        let mut died = false;
        for _ in 0..600 {
            if m.run_frame().contains(&routine::LOSE_LIFE) {
                died = true;
                m.zx.release_all_keys();
                run(&mut m, 200);
                break;
            }
        }
        let after = m.zx.mem[usize::from(at::LIVES)];
        let good = died && (after == before) == lives;
        println!(
            "training: endless lives {name}: {before} lives, {after} after a death{}",
            if good { "" } else { "  NOT AS PROMISED" }
        );
        ok &= good;
    }
    // Air stays full: a minute of play takes none.
    for (air, name) in [(false, "off"), (true, "on")] {
        let mut m = into_play(dir);
        m.rules.training = Training {
            air,
            ..Training::default()
        };
        let before = m.zx.mem[usize::from(at::AIR)];
        run(&mut m, 3000);
        let after = m.zx.mem[usize::from(at::AIR)];
        let good = (after == before) == air;
        println!(
            "training: air stays full {name}: {before} then {after}{}",
            if good { "" } else { "  NOT AS PROMISED" }
        );
        ok &= good;
        // The end-of-cavern bonus still counts the air down, switch or not:
        // the cavern is left from the top of the main loop, where the bonus
        // is entered.
        let mut m = into_play(dir);
        m.rules.training = Training {
            air,
            ..Training::default()
        };
        while m.zx.pc() != routine::MAIN_LOOP {
            m.zx.step();
        }
        m.zx.set_pc(routine::BONUS);
        m.watch = vec![routine::NEXT_CAVERN];
        // The lowest the air went: the next cavern's own fills the buffer.
        let before = m.zx.mem[usize::from(at::AIR)];
        let mut after = before;
        let next = (0..600).any(|_| {
            !m.run_frame_observing(|z| after = after.min(z.mem[usize::from(at::AIR)]))
                .is_empty()
        });
        let good = next && after < before;
        println!(
            "training: the bonus with air stays full {name}: {before} counted down to {after}{}",
            if good { "" } else { "  NOT AS PROMISED" }
        );
        ok &= good;
    }
    // Every switch off: the whole of memory as a machine with no rules at
    // all leaves it, after play that meets a death.
    let tap = tape(dir);
    let mut plain =
        sidekick::Machine::<()>::from_tape(&tap, ENTRY_PC, ENTRY_SP).expect("the tape loads");
    let font_at = usize::from(FONT);
    plain.zx.mem[font_at..font_at + 768].copy_from_slice(&manicminer::font::CHARACTER_SET);
    let mut ours = machine(dir);
    for frame in 0..2000u64 {
        for z in [&mut plain.zx, &mut ours.zx] {
            z.release_all_keys();
            if (100..300).contains(&frame) {
                z.set_key(key("enter"), true);
            }
            if frame > 700 {
                z.set_key(key("p"), true);
                z.set_key(key("space"), true);
            }
        }
        plain.run_frame();
        ours.run_frame();
    }
    let same = plain.zx.mem[..] == ours.zx.mem[..] && plain.zx.pc() == ours.zx.pc();
    println!(
        "training: with every switch off, memory {} a machine with no rules",
        if same {
            "is the same as"
        } else {
            "is NOT the same as"
        }
    );
    ok && same
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "manicminer-check <entry|keys|facts|font|training|all> <assets-dir>";
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
        "training" => training_check(dir),
        "all" => {
            let results = [
                entry_check(dir),
                keys_check(dir),
                facts_check(dir),
                font_check(dir),
                training_check(dir),
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
