//! The real Starquake tape loads and its game starts. Needs the player's tape in `assets/`
//! (`Starquake.tap.zip` from World of Spectrum); skipped without it, as in CI.

use sidekick_core::{Key, Machine, Model};
use sidekick_def::tapes_in;
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
