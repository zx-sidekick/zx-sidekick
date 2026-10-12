//! A high-score table read from and written into memory, and the moments a program acts on.

use sidekick_core::{Machine, Milestone, Model, ScoreEntry, Session};
use sidekick_def::Definition;

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[values.mode]
addr = 0x8100
type = "u8"

[[states]]
name = "title"
role = "title"
when = { value = "mode", equals = 1 }
[[states]]
name = "play"
role = "play"
when = { value = "mode", equals = 2 }

[high_scores]
title = "HEROES"
addr = 0x8000
count = 3
stride = 9
name = { offset = 0, len = 3, type = "text", charset = { first = 1, chars = " ABCDEFGHIJKLMNOPQRSTUVWXYZ" } }
score = { offset = 3, len = 6, type = "digits" }
"#;

fn session() -> Session {
    let definition = Definition::from_toml(DEFINITION).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    Session::new(machine, Some(definition))
}

fn mode(s: &mut Session, mode: u8) {
    s.machine_mut().write(0x8100, mode);
    s.run_frame().expect("a frame");
}

fn entry(name: &str, score: &str) -> ScoreEntry {
    ScoreEntry {
        name: name.to_owned(),
        score: score.to_owned(),
    }
}

#[test]
fn the_table_is_written_and_read_in_its_fields() {
    let mut s = session();
    s.set_scores(&[
        entry("ANN", "12500"),
        entry("BO", "900"),
        entry("CAT", "50"),
    ]);
    let bytes: Vec<u8> = (0x8000..0x8009).map(|a| s.machine().peek(a)).collect();
    assert_eq!(
        bytes,
        [2, 15, 15, b'0', b'1', b'2', b'5', b'0', b'0'],
        "its own charset"
    );
    assert_eq!(
        s.machine().peek(0x800B),
        1,
        "a short name padded with its space"
    );
    assert_eq!(
        s.scores().expect("a table"),
        [
            entry("ANN", "12500"),
            entry("BO", "900"),
            entry("CAT", "50")
        ]
    );
    s.set_scores(&[entry("aaa", "1"), entry("DAN", "9999999")]);
    assert_eq!(
        s.scores().expect("a table")[..2],
        [entry("ANN", "12500"), entry("BO", "900")],
        "an entry with a name outside the charset, or a score too long, is left whole"
    );
}

#[test]
fn the_first_title_and_each_game_s_end_are_milestones() {
    let mut s = session();
    mode(&mut s, 0);
    assert!(s.take_milestones().is_empty());
    mode(&mut s, 1);
    assert_eq!(s.take_milestones(), [Milestone::FirstTitle]);
    mode(&mut s, 1);
    mode(&mut s, 2);
    mode(&mut s, 0);
    assert!(
        s.take_milestones().is_empty(),
        "a frame with no state isn't the title"
    );
    mode(&mut s, 1);
    assert_eq!(s.take_milestones(), [Milestone::GameEnded]);
    mode(&mut s, 1);
    mode(&mut s, 0);
    mode(&mut s, 1);
    assert!(s.take_milestones().is_empty(), "no game since");
}

/// The same game with its states named otherwise: their meanings come from their roles.
const RENAMED: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[values.mode]
addr = 0x8100
type = "u8"

[values.energy]
addr = 0x8101
type = "u8"
writable = true

[[states]]
name = "menu"
role = "title"
when = { value = "mode", equals = 1 }
[[states]]
name = "running"
role = "play"
when = { value = "mode", equals = 2 }
[[states]]
name = "play"
when = { value = "mode", equals = 3 }

[[cheats]]
id = "full"
label = "Full"
set = { value = "energy", to = 9 }
"#;

#[test]
fn a_state_s_meaning_comes_from_its_role_not_its_name() {
    let definition = Definition::from_toml(RENAMED).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let mut s = Session::new(machine, Some(definition));
    mode(&mut s, 1);
    assert_eq!(s.take_milestones(), [Milestone::FirstTitle], "role title");
    s.switch("full", true).expect("it goes on");
    mode(&mut s, 2);
    mode(&mut s, 2);
    assert_eq!(s.machine().peek(0x8101), 9, "role play: the switch applies");
    // Values are read before a frame's state is decided: the first frame in a new state
    // still counts as the one before.
    mode(&mut s, 3);
    s.machine_mut().write(0x8101, 0);
    mode(&mut s, 3);
    assert_eq!(
        s.machine().peek(0x8101),
        0,
        "a state merely named play isn't play: no switch applies"
    );
    mode(&mut s, 1);
    assert_eq!(
        s.take_milestones(),
        [Milestone::GameEnded],
        "back to the title ends it"
    );
}
