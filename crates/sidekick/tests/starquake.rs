//! The real Starquake tape loads and its game starts. Needs the player's tape in `assets/`
//! (`Starquake.tap.zip` from World of Spectrum); skipped without it, as in CI.

use sidekick_core::{Key, Machine, Model, Session};
use sidekick_def::{tapes_in, Package};
use std::collections::BTreeSet;
use std::path::Path;

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
#[allow(clippy::print_stderr, reason = "a skipped test says why")]
fn in_play() -> Option<Session> {
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

    s.switch("no-harm", true)
        .expect("its patch finds the game's bytes in play");
    assert_eq!(s.machine().peek(0xA56A), 0x00);
    s.switch("no-harm", false).expect("and comes off");
    assert_eq!(s.machine().peek(0xA56A), 0xC4, "the game's own bytes back");
}
