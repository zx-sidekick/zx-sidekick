//! Training switches while a game runs, and the record of what a game was played with.

use sidekick_core::{Machine, Model, Session, SwitchError};
use sidekick_def::Definition;

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

/// Free RAM, above the ROM editor's workspace, where these tests put their bytes.
const FREE: u16 = 0x8000;

const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[values.lives]
addr = 0x8000
type = "u8"
writable = true
max = 9
[values.energy]
addr = 0x8001
type = "u8"
writable = true
max = 200
[values.mode]
addr = 0x8002
type = "u8"

[[states]]
name = "title"
role = "title"
when = { value = "mode", equals = 1 }
[[states]]
name = "play"
role = "play"
when = { value = "mode", equals = 2 }
[[states]]
name = "game_over"
role = "game_over"
when = { value = "mode", equals = 3 }

[[cheats]]
id = "full-energy"
label = "Full energy"
set = { value = "energy", to = "max" }

[[cheats]]
id = "three-lives"
label = "Three lives"
set = { value = "lives", to = 3 }
marks_score = false

[[cheats]]
id = "endless-lives"
label = "Endless lives"
hold = "lives"

[[cheats]]
id = "no-harm"
label = "No harm"
patch = [
  { addr = 0x8010, bytes = "00 00 00", expect = "CD 40 75" },
  { addr = 0x8020, bytes = "C9", expect = "F5" },
]
"#;

fn session() -> Session {
    let definition = Definition::from_toml(DEFINITION).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    Session::new(machine, Some(definition))
}

fn poke(s: &mut Session, addr: u16, bytes: &[u8]) {
    for (at, &byte) in (addr..).zip(bytes) {
        s.machine_mut().write(at, byte);
    }
}

fn frame(s: &mut Session) {
    s.run_frame().expect("a frame");
}

/// Puts the game in play.
fn play(s: &mut Session) {
    poke(s, FREE + 2, &[2]);
    frame(s);
}

#[test]
fn set_writes_its_number_or_the_value_s_max_each_frame() {
    let mut s = session();
    play(&mut s);
    poke(&mut s, FREE, &[7, 50]);
    s.switch("full-energy", true).expect("on");
    s.switch("three-lives", true).expect("on");
    frame(&mut s);
    assert_eq!(
        [s.machine().peek(FREE), s.machine().peek(FREE + 1)],
        [3, 200]
    );
    assert_eq!(s.reading()["energy"].number(), Some(200), "the reading too");
    poke(&mut s, FREE + 1, &[10]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE + 1), 200, "again each frame");
    s.switch("full-energy", false).expect("off");
    poke(&mut s, FREE + 1, &[10]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE + 1), 10, "off: left alone");
}

#[test]
fn set_and_hold_act_only_in_play() {
    let mut s = session();
    poke(&mut s, FREE, &[0, 50, 1]);
    frame(&mut s);
    s.switch("full-energy", true).expect("on");
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE + 1), 50, "on the title: left alone");
    play(&mut s);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE + 1), 200, "in play");
    poke(&mut s, FREE + 1, &[10]);
    poke(&mut s, FREE + 2, &[0]);
    frame(&mut s);
    assert_eq!(
        s.machine().peek(FREE + 1),
        200,
        "through a frame that matched no state"
    );
}

#[test]
fn hold_puts_a_value_back_when_it_goes_down() {
    let mut s = session();
    play(&mut s);
    poke(&mut s, FREE, &[4]);
    frame(&mut s);
    s.switch("endless-lives", true).expect("on");
    poke(&mut s, FREE, &[3]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE), 4, "put back");
    poke(&mut s, FREE, &[6]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE), 6, "going up is fine");
    poke(&mut s, FREE, &[5]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE), 6);
}

#[test]
fn patches_go_on_and_come_off_together() {
    let mut s = session();
    poke(&mut s, FREE + 0x10, &[0xCD, 0x40, 0x75]);
    poke(&mut s, FREE + 0x20, &[0xF5]);
    s.switch("no-harm", true).expect("on");
    assert!(s.switched_on("no-harm"));
    assert_eq!(s.machine().peek(FREE + 0x10), 0x00);
    assert_eq!(s.machine().peek(FREE + 0x20), 0xC9);
    s.switch("no-harm", false).expect("off");
    assert_eq!(s.machine().peek(FREE + 0x10), 0xCD);
    assert_eq!(s.machine().peek(FREE + 0x20), 0xF5);
}

#[test]
fn a_patch_that_doesn_t_match_refuses_the_whole_switch() {
    let mut s = session();
    poke(&mut s, FREE + 0x10, &[0xCD, 0x40, 0x75]);
    poke(&mut s, FREE + 0x20, &[0xF6]);
    assert_eq!(
        s.switch("no-harm", true),
        Err(SwitchError::Mismatch { addr: FREE + 0x20 })
    );
    assert!(!s.switched_on("no-harm"));
    assert_eq!(
        s.machine().peek(FREE + 0x10),
        0xCD,
        "not even its first patch"
    );
    assert_eq!(s.switch("no-such", true), Err(SwitchError::Unknown));
    assert_eq!(
        s.can_switch_on("no-harm"),
        Err(SwitchError::Mismatch { addr: FREE + 0x20 }),
        "asking says the same, and changes nothing"
    );
    poke(&mut s, FREE + 0x20, &[0xF5]);
    assert_eq!(s.can_switch_on("no-harm"), Ok(()));
    assert!(!s.switched_on("no-harm"));
}

#[test]
fn the_record_keeps_the_switches_and_level_of_the_game_being_played() {
    let mut s = session();
    let mode = |s: &mut Session, mode: u8| {
        poke(s, FREE + 2, &[mode]);
        frame(s);
    };
    mode(&mut s, 1);
    mode(&mut s, 2);
    assert!(s.record().switches.is_empty());
    s.switch("full-energy", true).expect("on");
    s.switch("three-lives", true).expect("on");
    s.set_level(3);
    s.set_level(1);
    s.switch("full-energy", false).expect("off");
    assert_eq!(s.record().level, 3, "the highest");
    assert_eq!(
        s.record().switches.iter().collect::<Vec<_>>(),
        ["full-energy"],
        "used, though off now; three lives doesn't mark the score"
    );

    mode(&mut s, 0);
    mode(&mut s, 2);
    assert_eq!(
        s.record().level,
        3,
        "a frame with no state isn't a new game"
    );

    mode(&mut s, 3);
    mode(&mut s, 1);
    s.switch("endless-lives", true).expect("on");
    mode(&mut s, 2);
    assert_eq!(s.record().level, 1, "a new game: the level now");
    assert_eq!(
        s.record().switches.iter().collect::<Vec<_>>(),
        ["endless-lives"]
    );
}

#[test]
fn a_definition_reloaded_keeps_the_switches_it_still_has() {
    let mut s = session();
    poke(&mut s, FREE + 0x10, &[0xCD, 0x40, 0x75]);
    poke(&mut s, FREE + 0x20, &[0xF5]);
    play(&mut s);
    s.switch("no-harm", true).expect("its bytes are there");
    s.switch("full-energy", true).expect("a set");
    assert_eq!(s.machine().peek(FREE + 0x20), 0xC9);

    // Edited: no harm now patches only its first place, with other bytes; full energy is gone.
    let edited = DEFINITION
        .replace(
            r#"  { addr = 0x8010, bytes = "00 00 00", expect = "CD 40 75" },
  { addr = 0x8020, bytes = "C9", expect = "F5" },"#,
            r#"  { addr = 0x8010, bytes = "11 11 11", expect = "CD 40 75" },"#,
        )
        .replace(
            "[[cheats]]\nid = \"full-energy\"\nlabel = \"Full energy\"\nset = { value = \"energy\", to = \"max\" }\n",
            "",
        );
    assert_ne!(edited, DEFINITION);
    let left = s.reload(Definition::from_toml(&edited).expect("it parses"));
    assert_eq!(left, ["full-energy"], "gone from the definition");
    assert!(s.switched_on("no-harm"));
    assert_eq!(
        (0..3)
            .map(|k| s.machine().peek(FREE + 0x10 + k))
            .collect::<Vec<_>>(),
        [0x11; 3],
        "its new bytes"
    );
    assert_eq!(
        s.machine().peek(FREE + 0x20),
        0xF5,
        "the place it left, put back"
    );
}

#[test]
fn a_state_s_set_is_written_while_the_game_is_in_it() {
    let text = DEFINITION.replace(
        "[[states]]\nname = \"title\"\nrole = \"title\"\nwhen = { value = \"mode\", equals = 1 }\n",
        "[[states]]\nname = \"title\"\nrole = \"title\"\nwhen = { value = \"mode\", equals = 1 }\nset = { value = \"lives\", to = 7 }\n",
    );
    assert_ne!(text, DEFINITION);
    let definition = Definition::from_toml(&text).expect("it parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let mut s = Session::new(machine, Some(definition));
    poke(&mut s, FREE + 2, &[1]);
    frame(&mut s);
    poke(&mut s, FREE, &[0]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE), 7, "on the title");
    poke(&mut s, FREE + 2, &[2]);
    frame(&mut s);
    poke(&mut s, FREE, &[3]);
    frame(&mut s);
    assert_eq!(s.machine().peek(FREE), 3, "in play: left alone");
}

#[test]
fn frames_that_match_no_state_are_counted_from_the_last_that_did() {
    let mut s = session();
    poke(&mut s, FREE + 2, &[1]);
    frame(&mut s);
    assert_eq!(s.frames_unmatched(), 0, "on the title");
    poke(&mut s, FREE + 2, &[0]);
    for _ in 0..3 {
        frame(&mut s);
    }
    assert_eq!(s.frames_unmatched(), 3);
    assert_eq!(s.progress().last_known.as_deref(), Some("title"));
    poke(&mut s, FREE + 2, &[1]);
    frame(&mut s);
    assert_eq!(s.frames_unmatched(), 0, "the title again");
}
