//! A definition's hints: which one shows, at which levels, from values, states and the script.

use sidekick_core::{Machine, Model, Script, Session};
use sidekick_def::Definition;

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

/// Free RAM, above the ROM editor's workspace.
const FREE: u16 = 0x8000;

const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[values.energy]
addr = 0x8000
type = "u8"
[values.mode]
addr = 0x8001
type = "u8"

[[states]]
name = "door"
when = { value = "mode", equals = 1 }
[[states]]
name = "play"
when = { value = "mode", equals = 2 }

[[guidance.levels]]
name = "Off"
description = "None."
[[guidance.levels]]
name = "One"
description = "One."
[[guidance.levels]]
name = "Two"
description = "Two."
[[guidance.levels]]
name = "Three"
description = "Three."

[script]
file = "stand-in.rhai"

[[hints]]
text = "Energy low"
priority = 2
state = "play"
when = { value = "energy", at_most = 30 }

[[hints]]
text = "Energy very low"
priority = 1
from_level = 2
when = { value = "energy", at_most = 10 }

[[hints]]
text = "At the door"
priority = 2
to_level = 2
state = ["door"]

[[hints]]
text = "Lost"
priority = 3
script = "lost"
"#;

fn session(script: &str) -> Session {
    let definition = Definition::from_toml(DEFINITION).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let mut s = Session::new(machine, Some(definition));
    s.set_script(Script::compile(script).map_err(|e| e.to_string()));
    s
}

/// A frame with these values; the script, if any, loads in the first.
fn frame(s: &mut Session, energy: u8, mode: u8) {
    s.machine_mut().write(FREE, energy);
    s.machine_mut().write(FREE + 1, mode);
    s.run_frame().expect("a frame");
}

const NO_SCRIPT: &str = "fn on_frame() { #{} }";

#[test]
fn the_lowest_priority_that_holds_shows_and_the_first_listed_of_equals() {
    let mut s = session(NO_SCRIPT);
    frame(&mut s, 127, 2);
    assert_eq!(s.hint(1), None, "none holds");
    frame(&mut s, 20, 2);
    assert_eq!(s.hint(1), Some("Energy low"));
    frame(&mut s, 5, 2);
    assert_eq!(s.hint(1), Some("Energy low"), "the very low one is from 2");
    assert_eq!(s.hint(2), Some("Energy very low"), "priority 1 first");
    frame(&mut s, 20, 1);
    assert_eq!(s.hint(1), Some("At the door"), "in the door state only");
    frame(&mut s, 20, 2);
    assert_eq!(
        s.hint(1),
        Some("Energy low"),
        "priority 2 both: the first listed"
    );
}

#[test]
fn a_hint_shows_from_its_level_to_its_to_level_and_never_at_off() {
    let mut s = session(NO_SCRIPT);
    frame(&mut s, 127, 1);
    assert_eq!(s.hint(0), None, "level 0 is no guidance");
    assert_eq!(s.hint(1), Some("At the door"));
    assert_eq!(s.hint(2), Some("At the door"));
    assert_eq!(s.hint(3), None, "past its to_level");
}

#[test]
fn a_frame_matching_no_state_keeps_the_state_last_matched() {
    let mut s = session(NO_SCRIPT);
    frame(&mut s, 127, 1);
    assert_eq!(s.hint(1), Some("At the door"));
    frame(&mut s, 127, 0);
    assert_eq!(s.state(), None);
    assert_eq!(
        s.hint(1),
        Some("At the door"),
        "a frame between doesn't blink it"
    );
    frame(&mut s, 127, 2);
    assert_eq!(s.hint(1), None, "another state");
}

#[test]
fn a_script_condition_holds_while_the_script_names_it() {
    let mut s = session(
        r#"fn on_frame(mem) { #{ hints: if mem.value("energy") == 99 { ["lost"] } else { [] } } }"#,
    );
    frame(&mut s, 99, 2);
    frame(&mut s, 99, 2);
    assert_eq!(s.hint(1), Some("Lost"));
    frame(&mut s, 98, 2);
    assert_eq!(s.hint(1), None);
}

#[test]
fn a_stopped_script_names_nothing() {
    let mut s = session(r#"fn on_frame() { let x = 1 / 0; #{ hints: ["lost"] } }"#);
    frame(&mut s, 99, 2);
    frame(&mut s, 99, 2);
    assert!(s.script_off().is_some());
    assert_eq!(s.hint(1), None);
}
