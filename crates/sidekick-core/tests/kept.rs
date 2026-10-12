//! Kept lists: entries added from a value the game shows in a state, or from the script.

use sidekick_core::{Machine, Model, Script, Session};
use sidekick_def::Definition;
use std::collections::{BTreeMap, BTreeSet};

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

[values.code]
addr = 0x8000
type = "text"
len = 5
[values.mode]
addr = 0x8005
type = "u8"

[[states]]
name = "booth"
when = { value = "mode", equals = 1 }

[persist.codes]
label = "codes"
file = "codes.txt"
add = { value = "code", in_state = "booth" }
most = 2

[persist.seen]
label = "places seen"
file = "seen.txt"
add = "script"
"#;

fn session() -> Session {
    let definition = Definition::from_toml(DEFINITION).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    Session::new(machine, Some(definition))
}

fn show(s: &mut Session, code: [u8; 5], mode: u8) {
    for (at, &byte) in (FREE..).zip(code.iter().chain(&[mode])) {
        s.machine_mut().write(at, byte);
    }
    s.run_frame().expect("a frame");
}

fn names(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|&n| n.to_owned()).collect()
}

#[test]
fn a_value_shown_in_its_state_is_kept_once() {
    let mut s = session();
    show(&mut s, *b"TOPAZ", 1);
    assert_eq!(s.kept()["codes"], ["TOPAZ"]);
    assert_eq!(s.take_kept_changes(), names(&["codes"]));
    show(&mut s, *b"TOPAZ", 1);
    assert!(s.take_kept_changes().is_empty(), "no repeats");
    show(&mut s, *b"LUMEN", 0);
    assert_eq!(s.kept()["codes"], ["TOPAZ"], "only in its state");
    show(&mut s, *b"     ", 1);
    assert_eq!(s.kept()["codes"], ["TOPAZ"], "never an empty one");
    show(&mut s, *b"LUMEN", 1);
    assert_eq!(s.kept()["codes"], ["TOPAZ", "LUMEN"], "in the order found");
    show(&mut s, *b"ORBIT", 1);
    assert_eq!(
        s.kept()["codes"],
        ["LUMEN", "ORBIT"],
        "at its most, the oldest goes"
    );
}

#[test]
fn lists_given_back_are_kept_on_and_can_be_forgotten() {
    let mut s = session();
    s.set_kept(BTreeMap::from([(
        "codes".to_owned(),
        vec!["NIVEN".to_owned()],
    )]));
    show(&mut s, *b"NIVEN", 1);
    assert!(s.take_kept_changes().is_empty(), "already kept");
    s.forget_kept();
    assert!(s.kept()["codes"].is_empty());
    assert_eq!(s.take_kept_changes(), names(&["codes"]));
}

#[test]
fn a_script_adds_to_its_lists_and_sees_them() {
    let mut s = session();
    s.set_script(
        Script::compile(
            r#"
            fn on_frame(mem, loaded, prev, kept) {
                #{ count: kept.seen.len(), kept: #{ seen: ["CAVE", "LAKE"], nowhere: ["X"] } }
            }
            "#,
        )
        .map_err(|e| e.to_string()),
    );
    s.set_kept(BTreeMap::from([("seen".to_owned(), Vec::new())]));
    // The script loads when the game's state is first known.
    show(&mut s, *b"     ", 1);
    show(&mut s, *b"     ", 1);
    assert_eq!(s.kept()["seen"], ["CAVE", "LAKE"]);
    assert!(
        !s.kept().contains_key("nowhere"),
        "only the definition's lists"
    );
    assert!(
        !s.script_results().contains_key("kept"),
        "kept isn't a widget's result"
    );
    show(&mut s, *b"     ", 1);
    assert_eq!(
        s.script_results()["count"],
        sidekick_core::Fact::Number(2),
        "the script sees its lists"
    );
}
