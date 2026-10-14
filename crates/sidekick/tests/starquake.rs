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
    assert!(
        machine.waits_on_loading_screen(),
        "its loader waits for a key"
    );
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
    assert!(
        !machine.waits_on_loading_screen(),
        "the loading screen's notice goes with the key"
    );
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
    // Long enough to meet harm whatever the game's seed, which the title's timing decides.
    let mut without = s.clone();
    assert!(
        lives_lost_walking(&mut without, 3600) > 0,
        "the walk does meet harm"
    );
    s.switch("no-harm", true).expect("it goes on");
    assert_eq!(lives_lost_walking(&mut s, 3600), 0, "with the switch, none");
}

/// One of Starquake's crafted situations, from `games/starquake/tests/`.
fn situation(name: &str) -> Situation {
    let text = std::fs::read_to_string(repo().join(format!("games/starquake/tests/{name}.toml")))
        .expect("the situation");
    Situation::from_toml(&text).expect("it parses")
}

/// Enters a situation's room, with its values, and runs until play goes on there.
fn enter_situation(s: &mut Session, situation: &Situation) {
    let values: Vec<(&str, i64)> = situation
        .values
        .iter()
        .map(|(name, &number)| (name.as_str(), number))
        .collect();
    enter_room(s, &values);
    assert!(until(s, 50, |s| s.state() != Some("play")));
    assert!(until(s, 200, |s| s.state() == Some("play")));
}

/// Lives lost, and the most energy lost, while BLOB stands still for `frames` frames.
fn harm_standing_still(s: &mut Session, frames: usize) -> (i64, i64) {
    let number = |s: &Session, name: &str| s.reading()[name].number().expect(name);
    let (lives, energy) = (number(s, "lives"), number(s, "energy"));
    let (mut lost, mut most_drained) = (0, 0);
    let mut before = lives;
    for _ in 0..frames {
        s.run_frame().expect("a frame");
        let now = number(s, "lives");
        lost += (before - now).max(0);
        before = now;
        most_drained = most_drained.max(energy - number(s, "energy"));
    }
    (lost, most_drained)
}

/// In a crafted situation with No harm among its switches, standing still costs BLOB lives
/// without its switches; with them he keeps his lives, and his energy only runs down with time.
fn no_harm_keeps_lives_and_energy_in(name: &str) {
    let Some(base) = on_the_joystick(50) else {
        return;
    };
    let situation = situation(name);
    assert!(situation.cheats.iter().any(|id| id == "no-harm"));
    // Long enough for the creature to reach BLOB whatever the game's seed, which the title's
    // timing decides.
    let frames = 3000;

    let mut without = base.clone();
    enter_situation(&mut without, &situation);
    let (lost, _) = harm_standing_still(&mut without, frames);
    assert!(lost > 0, "{name}: without No harm, a life goes");

    let mut with = base;
    for id in &situation.cheats {
        with.switch(id, true).expect("the switch goes on");
    }
    enter_situation(&mut with, &situation);
    let (lost, drained) = harm_standing_still(&mut with, frames);
    assert_eq!(lost, 0, "{name}: with No harm, no life goes");
    // Time alone takes 4 energy every 120 frames (docs/re/starquake.md).
    let by_time = 4 * (i64::try_from(frames).expect("a few frames") / 120 + 1);
    assert!(
        drained <= by_time,
        "{name}: with No harm, energy runs down with time only: {drained} > {by_time}"
    );
}

#[test]
fn no_harm_keeps_blob_s_lives_and_energy_in_a_zapper() {
    no_harm_keeps_lives_and_energy_in("zapper");
}

#[test]
fn no_harm_keeps_blob_s_lives_and_energy_against_a_creature_s_touch() {
    no_harm_keeps_lives_and_energy_in("creature");
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

#[test]
fn walking_into_the_core_s_room_marks_it_visited() {
    let Some(mut s) = on_the_joystick(50) else {
        return;
    };
    let visited = |s: &Session| {
        let at = s.definition().expect("its definition").values["visited"]
            .addr
            .expect("an address")
            .0;
        s.machine().peek(at + 199 / 8) & (0x80 >> (199 % 8)) == 0
    };
    assert!(!visited(&s), "a new game hasn't been there");
    // At floor level near the right of room 198, walking right into the core's room.
    enter_room(&mut s, &[("room", 198), ("blob_x", 224), ("blob_y", 39)]);
    s.machine_mut().kempston(Kempston::Right, true);
    assert!(
        until(&mut s, 300, visited),
        "the core's room is marked visited"
    );
}

#[test]
fn the_pad_s_start_pauses_the_game_in_play() {
    let Some(mut s) = in_play() else { return };
    until(&mut s, 50, |_| false);
    let definition = s.definition().expect("its definition");
    let Some(sidekick_def::PadTarget::Table(start)) = definition.pad["default"].start.clone()
    else {
        panic!("Start taps keys in play");
    };
    // The keys it taps, by the names a definition uses.
    let keys: Vec<Key> = start
        .tap
        .iter()
        .map(|name| match name.as_str() {
            "SPACE" => Key::Space,
            "P" => Key::P,
            other => panic!("a key this test doesn't name: {other}"),
        })
        .collect();
    for &key in &keys {
        s.machine_mut().key(key, true);
    }
    for _ in 0..start.frames.unwrap_or(1) {
        s.run_frame().expect("a frame");
    }
    for &key in &keys {
        s.machine_mut().key(key, false);
    }
    assert!(
        until(&mut s, 50, |s| s.state() == Some("paused")),
        "Start's tap pauses the game"
    );
}

#[test]
fn a_game_started_from_the_keyboard_plays_on_the_joystick() {
    // Started with 0 alone, which picks the keyboard on the original's title.
    let Some(mut s) = in_play() else { return };
    until(&mut s, 50, |_| false);
    let x = |s: &Session| s.reading()["blob_x"].number();
    let before = x(&s);
    s.machine_mut().kempston(Kempston::Right, true);
    until(&mut s, 40, |_| false);
    s.machine_mut().kempston(Kempston::Right, false);
    assert!(
        x(&s) > before,
        "the joystick moves BLOB: the title chose it"
    );
}

/// A route on the map: its label, its exit, and its steps' cells.
type RouteShown = (Fact, Option<Fact>, Vec<(i64, i64)>);

/// The map's routes, and its ring markers' rooms.
fn routes_of(s: &Session) -> (Vec<RouteShown>, Vec<i64>) {
    assert_eq!(s.script_off(), None, "the script within its limits");
    let map = &s.script_results()["map"];
    let number = |fact: &Fact| match fact {
        Fact::Number(n) => *n,
        other => panic!("{other:?}"),
    };
    let routes = list(field(map, "routes"))
        .iter()
        .map(|route| {
            let Fact::Map(fields) = route else {
                panic!("{route:?}")
            };
            let steps = list(&fields["steps"])
                .iter()
                .map(|step| (number(&list(step)[0]), number(&list(step)[1])))
                .collect();
            (fields["label"].clone(), fields.get("exit").cloned(), steps)
        })
        .collect();
    let rings = list(field(map, "markers"))
        .iter()
        .filter(|m| field(m, "kind") == &Fact::Text("target".to_owned()))
        .map(|m| number(field(m, "y")) * 16 + number(field(m, "x")))
        .collect();
    (routes, rings)
}

#[test]
fn level_5_routes_from_blob_s_place_to_the_nearest_pieces() {
    let Some(mut s) = in_play_from(|s| with_script(s, 5)) else {
        return;
    };
    s.run_frame().expect("a frame");
    let (routes, _) = routes_of(&s);
    assert_eq!(routes.len(), 1, "a piece, and nothing carried for the core");
    let (label, exit, steps) = &routes[0];
    assert_eq!(label, &Fact::Text("Item 1/3".to_owned()));
    assert!(exit.is_some(), "out of the start room");
    assert_eq!(steps[0], (8, 0), "from the start room");
    assert!(steps.len() > 1);

    s.script_action("next_route").expect("Tab");
    s.run_frame().expect("a frame");
    let (routes, _) = routes_of(&s);
    assert_eq!(routes[0].0, Fact::Text("Item 2/3".to_owned()));
}

#[test]
fn carrying_a_piece_the_core_needs_routes_into_the_core_s_room_from_its_left() {
    let Some(mut s) = on_the_joystick(50) else {
        return;
    };
    with_script(&mut s, 5);
    // At floor level near the right of room 198, carrying a piece the core needs.
    enter_room(&mut s, &[("room", 198), ("blob_x", 224), ("blob_y", 39)]);
    assert!(until(&mut s, 50, |s| s.state() != Some("play")));
    assert!(until(&mut s, 200, |s| s.state() == Some("play")));
    let definition = s.definition().expect("its definition");
    let slots = definition.values["core_slots"].addr.expect("an address").0;
    let carried = definition.values["carried"].addr.expect("an address").0;
    let piece = (0..9)
        .map(|k| s.machine().peek(slots + k))
        .find(|slot| slot & 0x80 != 0)
        .expect("a piece missing")
        & 0x7F;
    s.machine_mut().write(carried, piece);
    s.machine_mut().write(carried + 1, 3);
    s.run_frame().expect("a frame");
    let (routes, rings) = routes_of(&s);
    let core = routes
        .iter()
        .find(|r| r.0 == Fact::Text("Core".to_owned()))
        .expect("a route to the core");
    assert_eq!(core.2, [(6, 12), (7, 12)], "one step right, into room 199");
    assert_eq!(core.1, Some(Fact::Text("E".to_owned())));
    assert!(rings.contains(&199), "the core's end ringed");
}

#[test]
fn blob_s_place_is_found_wherever_he_walks() {
    let Some(mut s) = on_the_joystick(50) else {
        return;
    };
    with_script(&mut s, 5);
    s.switch("no-harm", true).expect("it goes on");
    s.switch("full-energy", true).expect("it goes on");
    let mut rooms = BTreeSet::new();
    for frame in 0..1500 {
        let machine = s.machine_mut();
        let phase = frame % 500;
        machine.kempston(Kempston::Right, phase < 250);
        machine.kempston(Kempston::Left, phase >= 250);
        machine.kempston(Kempston::Up, frame % 120 < 30);
        s.run_frame().expect("a frame");
        if s.state() == Some("play") && frame > 0 {
            rooms.insert(s.reading()["room"].number());
            let (routes, _) = routes_of(&s);
            assert!(
                !routes.is_empty(),
                "frame {frame}: no route from BLOB's place"
            );
        }
    }
    assert!(rooms.len() > 1, "walked through rooms: {rooms:?}");
}

/// The character rows of the screen, below the border, where two pictures `width` pixels wide
/// differ.
fn rows_changed(before: &[u8], after: &[u8], width: usize) -> BTreeSet<usize> {
    let top = (before.len() / width - 192) / 2;
    before
        .chunks_exact(width)
        .zip(after.chunks_exact(width))
        .enumerate()
        .filter(|(y, (a, b))| a != b && (top..top + 192).contains(y))
        .map(|(y, _)| (y - top) / 8)
        .collect()
}

#[test]
fn the_code_card_sits_just_under_the_line_the_booth_types_its_code_on() {
    let Some(mut s) = on_the_joystick(50) else {
        return;
    };
    let situation = situation("booth");
    for id in &situation.cheats {
        s.switch(id, true).expect("the switch goes on");
    }
    enter_situation(&mut s, &situation);
    s.machine_mut().kempston(Kempston::Right, true);
    let entered = until(&mut s, 200, |s| s.state() == Some("booth"));
    s.machine_mut().kempston(Kempston::Right, false);
    assert!(entered, "walking right goes into the booth");
    until(&mut s, 20, |_| false);

    let width = s.machine().picture().width();
    let before = s.machine().picture().pixels().to_vec();
    press(&mut s, Key::Q);
    until(&mut s, 20, |_| false);
    assert_eq!(s.state(), Some("booth"), "it waits for the next letter");
    let changed = rows_changed(&before, s.machine().picture().pixels(), width);
    let [line] = changed.iter().copied().collect::<Vec<_>>()[..] else {
        panic!("a letter typed changes one row: {changed:?}");
    };

    let card_row = s
        .definition()
        .and_then(|d| d.text_entry.as_ref())
        .and_then(|t| t.card_row)
        .expect("the definition places the card");
    assert_eq!(
        usize::try_from(card_row).expect("a row"),
        line + 1,
        "the card's top is the row just under the code line, row {line}"
    );
}

/// A Spectrum key by the name a definition gives it: a letter, a digit or `SPACE`.
fn key_named(name: &str) -> Key {
    let keys = [
        ("A", Key::A),
        ("B", Key::B),
        ("C", Key::C),
        ("D", Key::D),
        ("E", Key::E),
        ("F", Key::F),
        ("G", Key::G),
        ("H", Key::H),
        ("I", Key::I),
        ("J", Key::J),
        ("K", Key::K),
        ("L", Key::L),
        ("M", Key::M),
        ("N", Key::N),
        ("O", Key::O),
        ("P", Key::P),
        ("Q", Key::Q),
        ("R", Key::R),
        ("S", Key::S),
        ("T", Key::T),
        ("U", Key::U),
        ("V", Key::V),
        ("W", Key::W),
        ("X", Key::X),
        ("Y", Key::Y),
        ("Z", Key::Z),
        ("SPACE", Key::Space),
    ];
    let Some(&(_, key)) = keys.iter().find(|(n, _)| *n == name) else {
        panic!("a key this test doesn't name: {name}");
    };
    key
}

#[test]
fn a_s_d_f_g_held_together_end_the_game() {
    let Some(mut s) = in_play() else { return };
    until(&mut s, 50, |_| false);
    let end = s
        .definition()
        .and_then(|d| d.actions.iter().find(|a| a.id == "end-game"))
        .cloned()
        .expect("the end-game action");
    let keys: Vec<Key> = end.tap.iter().map(|name| key_named(name)).collect();
    for &key in &keys {
        s.machine_mut().key(key, true);
    }
    let ended = until(&mut s, end.frames.map_or(1, |f| f as usize), |s| {
        s.state() == Some("game_over")
    });
    assert!(ended, "the game is over: {:?}", s.state());
    assert_eq!(
        s.reading()["lives"].number(),
        Some(4),
        "with lives still left"
    );
}

/// The address of a value in the session's definition.
fn addr(s: &Session, name: &str) -> u16 {
    s.definition().expect("its definition").values[name]
        .addr
        .expect("an address")
        .0
}

#[test]
fn a_new_game_s_core_wants_five_pieces_at_random_and_a_piece_delivered_fills_its_slot() {
    let Some(mut s) = on_the_joystick(50) else {
        return;
    };
    s.switch("no-harm", true).expect("it goes on");
    let slots = addr(&s, "core_slots");
    let slot = |s: &Session, k: u8| s.machine().peek(slots + u16::from(k));
    assert_eq!(s.reading()["pieces_missing"].number(), Some(9));
    assert!(
        (0..9).all(|k| slot(&s, k) & 0x80 != 0),
        "every piece missing"
    );
    let own = (0..9).filter(|&k| slot(&s, k) & 0x7F == k).count();
    assert_eq!(own, 4, "four slots want their own number's picture");
    let random: Vec<u8> = (0..9)
        .map(|k| slot(&s, k) & 0x7F)
        .filter(|&picture| picture > 8)
        .collect();
    assert_eq!(random.len(), 5);
    assert!(
        random
            .iter()
            .all(|p| (9..=14).contains(p) || (26..=34).contains(p)),
        "five among the cards, 14 and the pieces from 26: {random:?}"
    );

    // Carrying the piece the first random slot wants, walking right into the core's room.
    let wanted = (0..9)
        .find(|&k| slot(&s, k) & 0x7F > 8)
        .expect("a random slot");
    let piece = slot(&s, wanted) & 0x7F;
    let carried = addr(&s, "carried");
    enter_room(&mut s, &[("room", 198), ("blob_x", 224), ("blob_y", 39)]);
    s.machine_mut().write(carried, piece);
    s.machine_mut().write(carried + 1, 3);
    s.machine_mut().kempston(Kempston::Right, true);
    assert!(
        until(&mut s, 600, |s| s.reading()["pieces_missing"].number()
            == Some(8)),
        "the piece is delivered"
    );
    s.machine_mut().kempston(Kempston::Right, false);
    assert_eq!(
        slot(&s, wanted),
        wanted,
        "its slot, no longer missing, holds its own number"
    );
    assert_eq!(
        (s.machine().peek(carried), s.machine().peek(carried + 1)),
        (0, 0),
        "and BLOB no longer carries it"
    );
}

/// Enters `room`, and puts BLOB on the first of its interactions of `kind`, at the sync point;
/// returns false if the room has none.
fn onto(s: &mut Session, room: i64, kind: i64) -> bool {
    walk_into(s, room);
    let Read::Table(entries) = &s.reading()["interactions"] else {
        panic!("the room's interactions");
    };
    let Some((x, height)) = entries
        .iter()
        .find(|e| e["kind"].number() == Some(kind))
        .map(|e| (e["x"].number(), e["height"].number()))
    else {
        return false;
    };
    assert!(
        (0..200).any(|_| s.run_to_sync().expect("a frame")),
        "the sync point"
    );
    s.write("blob_x", x.expect("x")).expect("writable");
    s.write("blob_y", height.expect("height"))
        .expect("writable");
    true
}

/// The teleport table's records: each one's code and room.
fn teleports(s: &Session) -> Vec<(String, i64)> {
    let Read::Table(records) = &s.reading()["teleports"] else {
        panic!("the teleport table");
    };
    records
        .iter()
        .map(|r| {
            let Read::Text(code) = &r["code"] else {
                panic!("a code")
            };
            (code.clone(), r["room"].number().expect("a room"))
        })
        .collect()
}

#[test]
fn a_booth_shows_its_own_code_and_another_s_code_sends_blob_there() {
    let Some(mut s) = on_the_joystick(50) else {
        return;
    };
    s.switch("no-harm", true).expect("it goes on");
    let records = teleports(&s);
    let (own, here) = records[0].clone();
    let (code, there) = records[1].clone();
    // A booth's place is where BLOB, moving, starts it.
    assert!(onto(&mut s, here, 13), "room {here} has a booth");
    s.machine_mut().kempston(Kempston::Right, true);
    assert!(
        until(&mut s, 50, |s| s.state() == Some("booth")),
        "the booth asks for a code"
    );
    s.machine_mut().kempston(Kempston::Right, false);
    assert_eq!(s.reading()["booth_code"], Read::Text(own), "its own code");

    for letter in code.chars() {
        let key = key_named(&letter.to_string());
        assert!(until(&mut s, 50, |s| s.state() == Some("booth")));
        s.machine_mut().key(key, true);
        until(&mut s, 2, |_| false);
        s.machine_mut().key(key, false);
        until(&mut s, 2, |_| false);
    }
    assert!(
        until(&mut s, 200, |s| s.reading()["room"].number() == Some(there)),
        "{code} goes to room {there}"
    );
}

#[test]
fn the_key_unlocks_a_locked_door_s_post_for_the_rest_of_the_game() {
    let Some(base) = on_the_joystick(50) else {
        return;
    };
    let locked = |s: &Session| -> Vec<(i64, bool)> {
        let Read::Table(doors) = &s.reading()["locked_doors"] else {
            panic!("the locked doors");
        };
        doors
            .iter()
            .map(|d| {
                let room = d["room_low"].number().expect("a room")
                    + 256 * d["room_high"].number().expect("its ninth bit");
                (room, d["locked"] == Read::Flag(true))
            })
            .collect()
    };
    let doors = locked(&base);
    assert!(doors.iter().all(|&(_, l)| l), "a new game locks them all");
    let room = doors[0].0;

    let at_the_post = |carrying: u8| {
        let mut s = base.clone();
        s.switch("no-harm", true).expect("it goes on");
        assert!(onto(&mut s, room, 11), "room {room} has a post");
        let carried = addr(&s, "carried");
        s.machine_mut().write(carried, carrying);
        s.machine_mut().write(carried + 1, 3);
        until(&mut s, 10, |_| false);
        s
    };
    let without = at_the_post(15);
    assert!(locked(&without)[0].1, "still locked without the key");
    let mut with = at_the_post(16);
    assert!(!locked(&with)[0].1, "unlocked by the key");
    walk_into(&mut with, room + 1);
    walk_into(&mut with, room);
    assert!(!locked(&with)[0].1, "and it stays unlocked");
}

#[test]
fn a_carried_15_answers_every_card_at_a_security_door() {
    let Some(base) = in_play_on_the_joystick() else {
        return;
    };
    let situation = situation("door");
    let values: Vec<(&str, i64)> = situation
        .values
        .iter()
        .map(|(name, &number)| (name.as_str(), number))
        .collect();
    // Walks left into the door carrying `picture` (nothing for 0), and returns where BLOB is
    // once it's done.
    let through = |picture: u8| {
        let mut s = base.clone();
        s.switch("no-harm", true).expect("it goes on");
        enter_room(&mut s, &values);
        let carried = addr(&s, "carried");
        s.machine_mut().write(carried, picture);
        s.machine_mut()
            .write(carried + 1, if picture == 0 { 0 } else { 3 });
        s.machine_mut().kempston(Kempston::Left, true);
        assert!(until(&mut s, 200, |s| s.state() == Some("door")));
        s.machine_mut().kempston(Kempston::Left, false);
        assert!(until(&mut s, 600, |s| s.state() == Some("play")));
        until(&mut s, 100, |_| false);
        s.reading()["blob_x"].number().expect("x")
    };
    let stopped = through(0);
    assert_eq!(
        through(15),
        stopped - 48,
        "the door moves BLOB 48 pixels on"
    );
}
