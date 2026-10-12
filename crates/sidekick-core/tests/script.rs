//! A definition's script run by the session: once loaded, each frame, and from actions.

use sidekick_core::{Fact, Machine, Model, Script, Session};
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

[values.lives]
addr = 0x8000
type = "u8"
writable = true
[values.seen]
addr = 0x8001
type = "u8"

# Matches every frame: the ROM's interrupt runs in each.
[[states]]
name = "play"
role = "play"
pc = 0x0038
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

fn frame(s: &mut Session) {
    s.run_frame().expect("a frame");
}

#[test]
fn the_script_loads_once_then_runs_each_frame() {
    let mut s = session(
        r#"
        fn on_load(mem, loaded, prev, kept, level) { #{ base: mem.peek(0x8000) } }
        fn on_frame(mem, loaded, prev, kept, level) {
            #{
                lives: mem.value("lives"),
                since: mem.value("lives") - loaded.base,
                state: mem.state,
            }
        }
        "#,
    );
    s.machine_mut().write(FREE, 3);
    frame(&mut s);
    assert!(s.script_results().is_empty(), "the first frame loads it");
    s.machine_mut().write(FREE, 5);
    frame(&mut s);
    let results = s.script_results();
    assert_eq!(results["lives"], Fact::Number(5));
    assert_eq!(results["since"], Fact::Number(2));
    assert_eq!(results["state"], Fact::Text("play".to_owned()));
    assert_eq!(s.script_off(), None);
}

#[test]
fn a_failing_script_is_turned_off_and_says_why() {
    let mut s = session(r"fn on_frame(mem, loaded, prev, kept, level) { let x = 1 / 0; #{} }");
    frame(&mut s);
    frame(&mut s);
    assert!(s
        .script_off()
        .is_some_and(|why| why.starts_with("on_frame: ")));
    frame(&mut s);

    let mut broken = session("fn on_frame(");
    assert!(broken
        .script_off()
        .is_some_and(|why| why.starts_with("line 1: ")));
    frame(&mut broken);
    assert!(broken.script_results().is_empty());
}

#[test]
fn an_action_writes_only_writable_values() {
    let mut s = session(
        r#"
        fn more(mem, loaded, prev, kept, level) {
            mem.write("lives", mem.value("lives") + 1);
            #{ added: true }
        }
        fn peek_seen(mem, loaded, prev, kept, level) { mem.write("seen", 1); #{} }
        "#,
    );
    s.machine_mut().write(FREE, 4);
    frame(&mut s);
    s.script_action("more").expect("the action runs");
    assert_eq!(s.machine().peek(FREE), 5);
    assert_eq!(s.script_results()["added"], Fact::Flag(true));
    assert_eq!(
        s.script_action("peek_seen"),
        Err("peek_seen: writing seen: the value isn't writable".to_owned())
    );
    assert!(s.script_action("no_such").is_err());
    assert_eq!(
        s.script_off(),
        None,
        "a missing function doesn't stop the script"
    );
}
