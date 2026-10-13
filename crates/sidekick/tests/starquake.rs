//! The real Starquake tape loads and its game starts. Needs the player's tape in `assets/`
//! (`Starquake.tap.zip` from World of Spectrum); skipped without it, as in CI.

use sidekick_core::{Fact, Kempston, Key, Machine, Model, Read, Script, Session};
use sidekick_def::{tapes_in, Package, Situation};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[test]
#[allow(clippy::print_stderr, reason = "a skipped test says why")]
fn starquake_loads_and_its_game_runs() {
    let zip = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/Starquake.tap.zip");
    if !zip.exists() {
        eprintln!("skipped: no {} (the player's own tape)", zip.display());
        return;
    }
    let tapes = tapes_in(&zip).expect("the zip holds a tape");
    assert_eq!(
        tapes[0].sha1, "65450d6f33692c2c2868c0b497037f2cfd0ef3bd",
        "the supported dump"
    );

    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
    machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
    for _ in 0..1500 {
        machine.run_frame().expect("a frame");
    }
    // The loading screen waits for a key, as on a Spectrum.
    let colors: BTreeSet<u8> = machine.picture().pixels().iter().copied().collect();
    assert!(colors.len() > 2, "the loading screen is up: {colors:?}");
    machine.key(Key::Space, true);
    for _ in 0..5 {
        machine.run_frame().expect("a frame");
    }
    machine.key(Key::Space, false);
    let in_game = (0..500).any(|_| {
        machine.run_frame().expect("a frame");
        machine.registers().get_pc() >= 0x4000
    });
    assert!(in_game, "after a key, the game's own code runs");
}

/// Runs frames until `done` holds, at most `most`; returns whether it did.
fn until(session: &mut Session, most: usize, done: impl Fn(&Session) -> bool) -> bool {
    (0..most).any(|_| {
        session.run_frame().expect("a frame");
        done(session)
    })
}

/// Presses a key for a few frames.
fn press(session: &mut Session, key: Key) {
    session.machine_mut().key(key, true);
    for _ in 0..4 {
        session.run_frame().expect("a frame");
    }
    session.machine_mut().key(key, false);
}

#[test]
#[allow(clippy::print_stderr, reason = "a skipped test says why")]
fn starquake_s_states_and_values_read_what_the_game_shows() {
    let zip = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/Starquake.tap.zip");
    if !zip.exists() {
        eprintln!("skipped: no {} (the player's own tape)", zip.display());
        return;
    }
    let tapes = tapes_in(&zip).expect("the zip holds a tape");
    let package =
        Package::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/starquake"))
            .expect("Starquake's definition");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
    machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
    let mut s = Session::new(machine, Some(package.definition));
    let number = |s: &Session, name: &str| s.reading()[name].number();
    let state = |s: &Session| s.state().map(str::to_owned);

    until(&mut s, 400, |_| false);
    press(&mut s, Key::Space);
    assert!(
        until(&mut s, 1500, |s| s.state() == Some("title")),
        "the title menu, once its tune has played"
    );

    // 0 starts a game; the menu takes keys between its frames, so press until it does.
    let started = (0..10).any(|_| {
        press(&mut s, Key::N0);
        until(&mut s, 200, |s| s.state() == Some("play"))
    });
    assert!(started, "play: {:?}", state(&s));
    assert_eq!(number(&s, "lives"), Some(4));
    assert_eq!(number(&s, "room"), Some(8), "a game starts in room 8");
    assert_eq!(number(&s, "platforms"), Some(50));
    assert_eq!(number(&s, "laser"), Some(127));

    press(&mut s, Key::Space);
    assert!(
        until(&mut s, 50, |s| s.state() == Some("paused")),
        "SPACE pauses"
    );
    press(&mut s, Key::P);
    assert!(
        until(&mut s, 50, |s| s.state() == Some("play")),
        "a move goes on"
    );

    assert!(
        until(&mut s, 3000, |s| number(s, "lives") == Some(3)),
        "standing still, energy runs down and a life goes"
    );
    assert!(
        until(&mut s, 200, |s| number(s, "energy")
            .is_some_and(|e| e > 100)),
        "and energy refills"
    );
}

/// Starquake in play from its real tape, with its definition; `None` without the tape.
fn in_play() -> Option<Session> {
    in_play_from(|_| {})
}

/// Starquake in play, the session set up by `prepare` before the tape runs.
#[allow(clippy::print_stderr, reason = "a skipped test says why")]
fn in_play_from(prepare: impl FnOnce(&mut Session)) -> Option<Session> {
    let zip = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/Starquake.tap.zip");
    if !zip.exists() {
        eprintln!("skipped: no {} (the player's own tape)", zip.display());
        return None;
    }
    let tapes = tapes_in(&zip).expect("the zip holds a tape");
    let package =
        Package::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/starquake"))
            .expect("Starquake's definition");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
    machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
    let mut s = Session::new(machine, Some(package.definition));
    prepare(&mut s);
    until(&mut s, 400, |_| false);
    press(&mut s, Key::Space);
    assert!(until(&mut s, 1500, |s| s.state() == Some("title")));
    let started = (0..10).any(|_| {
        press(&mut s, Key::N0);
        until(&mut s, 200, |s| s.state() == Some("play"))
    });
    assert!(started, "play");
    Some(s)
}

#[test]
fn starquake_s_training_switches_work_on_the_game() {
    let Some(mut s) = in_play() else { return };
    let number = |s: &Session, name: &str| s.reading()[name].number();

    s.switch("full-energy", true).expect("full energy goes on");
    until(&mut s, 3000, |_| false);
    assert_eq!(number(&s, "lives"), Some(4), "the timer took no life");
    assert_eq!(number(&s, "energy"), Some(127));

    s.switch("full-platforms", true)
        .expect("full platforms goes on");
    until(&mut s, 2, |_| false);
    assert_eq!(number(&s, "platforms"), Some(127));
    press(&mut s, Key::A);
    until(&mut s, 4, |_| false);
    assert_eq!(
        number(&s, "platforms"),
        Some(127),
        "a platform built, still full"
    );

    // The switch's patches, as the definition gives them: where, what each writes, what it
    // expects.
    let patches = s
        .definition()
        .and_then(|d| d.cheats.iter().find(|c| c.id == "no-harm"))
        .map(|c| c.patch.clone())
        .expect("no harm patches the game");
    let bytes_at = |s: &Session, patch: &sidekick_def::Patch| -> Vec<u8> {
        let at = patch.addr.expect("an address").0;
        (at..)
            .take(patch.bytes.0.len())
            .map(|a| s.machine().peek(a))
            .collect()
    };
    s.switch("no-harm", true)
        .expect("its patches find the game's bytes in play");
    for patch in &patches {
        assert_eq!(bytes_at(&s, patch), patch.bytes.0, "on");
    }
    s.switch("no-harm", false).expect("and come off");
    for patch in &patches {
        assert_eq!(
            bytes_at(&s, patch),
            patch.expect.0,
            "the game's own bytes back"
        );
    }
}

/// Starquake in play from its real tape, BLOB on the Kempston joystick; `None` without the
/// tape.
#[allow(clippy::print_stderr, reason = "a skipped test says why")]
fn in_play_on_the_joystick() -> Option<Session> {
    on_the_joystick(900)
}

/// Starquake in play on the Kempston joystick, `settle` frames after play began; `None` without
/// the tape.
#[allow(clippy::print_stderr, reason = "a skipped test says why")]
fn on_the_joystick(settle: usize) -> Option<Session> {
    let zip = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/Starquake.tap.zip");
    if !zip.exists() {
        eprintln!("skipped: no {} (the player's own tape)", zip.display());
        return None;
    }
    let tapes = tapes_in(&zip).expect("the zip holds a tape");
    let package =
        Package::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/starquake"))
            .expect("Starquake's definition");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
    machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
    let mut s = Session::new(machine, Some(package.definition));
    until(&mut s, 400, |_| false);
    press(&mut s, Key::Space);
    assert!(until(&mut s, 1500, |s| s.state() == Some("title")));
    press(&mut s, Key::N1);
    let started = (0..10).any(|_| {
        press(&mut s, Key::N0);
        until(&mut s, 200, |s| s.state() == Some("play"))
    });
    assert!(started, "play");
    until(&mut s, settle, |_| false);
    Some(s)
}

/// Lives lost walking BLOB right and left, jumping now and then, energy kept full: the walk
/// meets the start room's deadly scenery.
fn lives_lost_walking(s: &mut Session, frames: u32) -> i64 {
    let lives = |s: &Session| s.reading()["lives"].number().expect("lives");
    let mut lost = 0;
    let mut before = lives(s);
    for frame in 0..frames {
        s.write("energy", 127).expect("energy is writable");
        let phase = frame % 600;
        let machine = s.machine_mut();
        machine.kempston(Kempston::Right, phase < 300);
        machine.kempston(Kempston::Left, phase >= 300);
        machine.kempston(Kempston::Up, frame % 97 < 10);
        s.run_frame().expect("a frame");
        let now = lives(s);
        lost += (before - now).max(0);
        before = now;
    }
    lost
}

#[test]
fn no_harm_keeps_blob_s_lives_against_deadly_scenery() {
    let Some(mut s) = in_play_on_the_joystick() else {
        return;
    };
    let mut without = s.clone();
    assert!(
        lives_lost_walking(&mut without, 1200) > 0,
        "the walk does meet harm"
    );
    s.switch("no-harm", true).expect("it goes on");
    assert_eq!(lives_lost_walking(&mut s, 1200), 0, "with the switch, none");
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Gives the session Starquake's script, at a guidance level.
fn with_script(s: &mut Session, level: u8) {
    let source = std::fs::read_to_string(repo().join("games/starquake/scripts/starquake.rhai"))
        .expect("its script");
    s.set_script(Script::compile(&source).map_err(|e| e.to_string()));
    s.set_level(level);
}

/// Enters a room as a crafted situation does: at the sync point, the values written, then the
/// definition's room entry called, returning to the sync point.
fn enter_room(s: &mut Session, values: &[(&str, i64)]) {
    let definition = s.definition().expect("its definition");
    let sync = definition
        .run
        .as_ref()
        .and_then(|r| r.sync)
        .expect("a sync")
        .0;
    let enter = definition
        .situations
        .as_ref()
        .and_then(|st| st.enter)
        .expect("a room entry")
        .0;
    assert!(
        (0..200).any(|_| s.run_to_sync().expect("a frame")),
        "the sync point"
    );
    for &(name, number) in values {
        s.write(name, number).expect("writable");
    }
    s.machine_mut().call(enter, sync);
}

fn field<'a>(fact: &'a Fact, key: &str) -> &'a Fact {
    match fact {
        Fact::Map(map) => &map[key],
        other => panic!("{key} of {other:?}"),
    }
}

fn list(fact: &Fact) -> &[Fact] {
    match fact {
        Fact::List(items) => items,
        other => panic!("a list: {other:?}"),
    }
}

/// Enters `room` and runs until it's drawn and play goes on.
fn walk_into(s: &mut Session, room: i64) {
    enter_room(s, &[("room", room)]);
    // Entering plays a transition; the room is drawn once play comes back.
    assert!(until(s, 50, |s| s.state() != Some("play")));
    assert!(until(s, 200, |s| s.state() == Some("play")));
    s.run_frame().expect("a frame");
    assert_eq!(s.script_off(), None);
}

#[test]
fn starquake_s_rooms_decoded_from_its_data_have_the_cells_the_screen_shows() {
    // Given the script from the start, as the program does: its on_load runs once the tape
    // has loaded, and the room data must be in place by then.
    let Some(mut s) = in_play_from(|s| with_script(s, 0)) else {
        return;
    };
    for room in [8, 9, 20, 24, 36, 41, 176, 300, 362, 511] {
        walk_into(&mut s, room);
        let results = s.script_results();
        assert_eq!(results["room"], Fact::Number(room));
        let Read::Bytes(area) = &s.reading()["play_area"] else {
            panic!("the play area's attributes");
        };
        // A cell blocks when its attribute lacks BRIGHT.
        let screen: Vec<Fact> = area
            .chunks(32)
            .map(|row| {
                let bits = row.iter().fold(0i64, |bits, &attribute| {
                    bits << 1 | i64::from(attribute & 0x40 == 0)
                });
                Fact::Number(bits)
            })
            .collect();
        assert_eq!(
            results["blocks"],
            Fact::List(screen),
            "room {room}: decoded, and as the game draws it"
        );
    }
}

/// The map cell of `room` at level 6, every room's.
fn cell_of(s: &Session, room: usize) -> Fact {
    let cells = list(field(&s.script_results()["map"], "cells")).to_vec();
    assert_eq!(cells.len(), 512, "level 6: every room");
    cells[room].clone()
}

fn walls_of(cell: &Fact) -> &str {
    match field(cell, "walls") {
        Fact::Text(walls) => walls,
        other => panic!("{other:?}"),
    }
}

/// How many wall and door cells are inside a map cell.
fn inside_of(cell: &Fact) -> (usize, usize) {
    match cell {
        Fact::Map(map) => map.get("inside").map_or((0, 0), |inside| {
            (
                list(field(inside, "walls")).len(),
                list(field(inside, "doors")).len(),
            )
        }),
        other => panic!("{other:?}"),
    }
}

#[test]
fn starquake_s_map_shows_the_walls_inside_its_rooms_and_its_doors() {
    let Some(mut s) = in_play_from(|s| with_script(s, 6)) else {
        return;
    };
    s.run_frame().expect("a frame");
    assert_eq!(s.script_off(), None);
    let start = cell_of(&s, 8);
    assert_eq!(
        walls_of(&start),
        "N",
        "the sky above; open left, right and down"
    );
    assert_eq!(inside_of(&start), (0, 0), "the start room is one part");
    // A block of rock splits room 24; its left edge is shut, ledges filling it all the way
    // down. A rock pillar splits room 36: one part leads out south, the other east.
    for room in [24, 36] {
        let (walls, doors) = inside_of(&cell_of(&s, room));
        assert!(walls > 0, "room {room} in two parts, a wall between");
        assert_eq!(doors, 0);
    }
    assert!(walls_of(&cell_of(&s, 24)).contains('W'));
    assert_eq!(walls_of(&cell_of(&s, 36)), "NW");
    // A pair of wall passages joins rooms 41 and 42.
    assert!(!walls_of(&cell_of(&s, 41)).contains('E'));
    assert!(!walls_of(&cell_of(&s, 42)).contains('W'));
    // The core's room is entered from its left only.
    assert_eq!(walls_of(&cell_of(&s, 199)), "NES");
    assert!(!walls_of(&cell_of(&s, 198)).contains('E'));
    // Room 176's security door, numbered at its place.
    let (walls, doors) = inside_of(&cell_of(&s, 176));
    assert!(walls > 0 && doors > 0, "a door in its wall");
    let markers = list(field(&s.script_results()["map"], "markers")).to_vec();
    let doors: Vec<&Fact> = markers
        .iter()
        .filter(|m| field(m, "kind") == &Fact::Text("door".to_owned()))
        .collect();
    assert_eq!(doors.len(), 8, "eight security doors, each numbered");
    assert!(doors
        .iter()
        .all(|d| matches!(field(d, "at"), Fact::List(_))));
}

#[test]
fn a_security_door_s_cards_are_the_ones_the_script_works_out() {
    let Some(mut s) = in_play_on_the_joystick() else {
        return;
    };
    with_script(&mut s, 6);
    let text = std::fs::read_to_string(repo().join("games/starquake/tests/door.toml"))
        .expect("the door situation");
    let situation = Situation::from_toml(&text).expect("it parses");
    let values: Vec<(&str, i64)> = situation
        .values
        .iter()
        .map(|(name, &number)| (name.as_str(), number))
        .collect();
    enter_room(&mut s, &values);
    s.machine_mut().kempston(Kempston::Left, true);
    let met = until(&mut s, 200, |s| {
        !s.kept().get("doors-met").is_none_or(Vec::is_empty)
    });
    s.machine_mut().kempston(Kempston::Left, false);
    assert!(met, "walking left meets the door");
    let kept = s.kept()["doors-met"][0].clone();
    let room = situation.values["room"];
    assert!(kept.contains(&format!(":{room}:")), "{kept}");

    s.run_frame().expect("a frame");
    // Level 6 lists every door, by room: the situation's is the first.
    let doors = list(&s.script_results()["doors"]);
    assert_eq!(doors.len(), 8, "eight security doors");
    let worked_out: Vec<String> = list(field(&doors[0], "cards"))
        .iter()
        .map(|card| match field(card, "picture") {
            Fact::Number(n) => n.to_string(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(
        kept.ends_with(&format!(":{}", worked_out.join("-"))),
        "met {kept}, worked out {worked_out:?}"
    );
}

/// The rooms on the map, by number.
fn rooms_on_the_map(s: &Session) -> Vec<i64> {
    let map = &s.script_results()["map"];
    let mut rooms: Vec<i64> = list(field(map, "cells"))
        .iter()
        .map(|cell| match (field(cell, "x"), field(cell, "y")) {
            (Fact::Number(x), Fact::Number(y)) => y * 16 + x,
            other => panic!("{other:?}"),
        })
        .collect();
    rooms.sort_unstable();
    rooms
}

#[test]
fn the_rooms_walked_are_the_game_s_own() {
    let Some(mut s) = in_play_from(|s| with_script(s, 2)) else {
        return;
    };
    s.run_frame().expect("a frame");
    assert_eq!(rooms_on_the_map(&s), [8], "the start room");
    enter_room(&mut s, &[("room", 9)]);
    assert!(until(&mut s, 50, |s| s.state() != Some("play")));
    assert!(until(&mut s, 200, |s| s.state() == Some("play")));
    s.run_frame().expect("a frame");
    assert_eq!(rooms_on_the_map(&s), [8, 9], "and the room entered");
}

/// The screen bytes of the status area above the play area: its six character rows' pixels.
fn status_area(s: &Session) -> Vec<u8> {
    (0x4000u16..0x4800)
        .filter(|a| (a >> 5) & 7 < 6)
        .map(|a| s.machine().peek(a))
        .collect()
}

/// Holds `input` on the joystick for a few frames, then lets go, `times` times.
fn tap_joystick(s: &mut Session, input: Kempston, times: usize) {
    for _ in 0..times {
        s.machine_mut().kempston(input, true);
        until(s, 8, |_| false);
        s.machine_mut().kempston(input, false);
        until(s, 30, |_| false);
    }
}

/// With `switch` on from the start of play, the game's own bar for `value` stays as drawn once
/// the switch went on while `act` runs; without it, the bar is drawn shorter.
fn bar_stays_full(switch: &str, value: &str, act: impl Fn(&mut Session)) {
    // Soon after play begins, before anything has taken from the bars.
    let Some(base) = on_the_joystick(50) else {
        return;
    };
    let mut off = base.clone();
    let mut on = base;
    on.switch(switch, true)
        .expect("its patches find the game's bytes");
    until(&mut on, 5, |_| false);
    until(&mut off, 5, |_| false);
    let first = status_area(&on);
    act(&mut off);
    act(&mut on);
    // Where the two games' status areas differ is the bar the switch keeps.
    let (with, without) = (status_area(&on), status_area(&off));
    let bar: Vec<usize> = (0..with.len()).filter(|&i| with[i] != without[i]).collect();
    assert!(!bar.is_empty(), "{switch}: without it, the bar goes down");
    assert!(
        bar.iter().all(|&i| with[i] == first[i]),
        "{switch}: with it, the bar is as drawn when it went on"
    );
    assert_eq!(
        on.reading()[value].number(),
        Some(127),
        "{switch}: and {value} is full"
    );
}

#[test]
fn full_laser_keeps_its_bar_full_on_screen_while_firing() {
    bar_stays_full("full-laser", "laser", |s| {
        tap_joystick(s, Kempston::Fire, 4);
    });
}

#[test]
fn full_platforms_keeps_its_bar_full_on_screen_while_building() {
    bar_stays_full("full-platforms", "platforms", |s| {
        tap_joystick(s, Kempston::Down, 2);
    });
}

#[test]
fn full_energy_keeps_its_bar_full_on_screen_as_time_goes() {
    // Long enough for the game's timer to take from energy a few times.
    bar_stays_full("full-energy", "energy", |s| {
        until(s, 800, |_| false);
    });
}

#[test]
fn full_platforms_draws_its_bar_full_as_it_goes_on() {
    // Platforms start at 50 of 127: switched on, the bar is drawn longer at once.
    let Some(base) = on_the_joystick(50) else {
        return;
    };
    let mut off = base.clone();
    let mut on = base;
    on.switch("full-platforms", true)
        .expect("its patches find the game's bytes");
    until(&mut off, 5, |_| false);
    until(&mut on, 5, |_| false);
    let (with, without) = (status_area(&on), status_area(&off));
    let bar: Vec<usize> = (0..with.len()).filter(|&i| with[i] != without[i]).collect();
    assert!(!bar.is_empty(), "the bar is drawn again");
    let ink = |area: &[u8]| bar.iter().map(|&i| area[i].count_ones()).sum::<u32>();
    assert!(ink(&with) > ink(&without), "and longer");
    assert_eq!(on.reading()["platforms"].number(), Some(127));
}
