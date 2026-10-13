//! Format 1, as `docs/definition-format.md` and the committed definitions write it.

use sidekick_def::{schema, Border, Definition};
use std::path::Path;

fn repo(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The TOML blocks of the format reference, in order.
fn reference_examples() -> Vec<String> {
    let doc = repo("docs/definition-format.md");
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in doc.lines() {
        match (&mut current, line) {
            (None, "```toml") => current = Some(String::new()),
            (Some(_), "```") => blocks.extend(current.take()),
            (Some(block), line) => {
                block.push_str(line);
                block.push('\n');
            }
            (None, _) => {}
        }
    }
    blocks
}

/// What a fragment needs around it to be a whole definition.
const HEADER: &str = r#"
schema = 1
[game]
id = "example"
title = "Example"
machine = "128k"
[[media]]
name = "Example"
"#;

#[test]
fn every_example_in_the_reference_is_a_definition_s_shape() {
    let examples = reference_examples();
    assert!(examples.len() > 10, "found {} examples", examples.len());
    for example in &examples {
        let text = if example.contains("schema =") {
            example.clone()
        } else {
            format!("{HEADER}{example}")
        };
        if let Err(e) = Definition::from_toml(&text) {
            panic!("this example doesn't parse:\n{example}\n{e}");
        }
    }
}

#[test]
fn the_reference_s_first_definition_is_complete() {
    let first = &reference_examples()[0];
    let def = Definition::from_toml(first).expect("it parses");
    assert_eq!(def.check(), [], "no problems");
}

#[test]
fn starquake_s_draft_parses_and_lists_what_is_left_to_find() {
    let def = Definition::from_toml(&repo("games/starquake/game.toml")).expect("the draft parses");
    let problems: Vec<String> = def.check().iter().map(ToString::to_string).collect();
    assert!(
        !problems.iter().any(|p| p.starts_with("media")),
        "the dump's SHA-1 is recorded"
    );
    assert!(
        problems.iter().any(|p| p == "high_scores.addr: missing"),
        "{problems:#?}"
    );
    assert!(
        !problems.iter().any(|p| p.contains("no value named")
            || p.contains("no state named")
            || p.contains("isn't")),
        "only what's still to find, no broken references: {problems:#?}"
    );
}

#[test]
fn toml_and_json_say_the_same() {
    let def = Definition::from_toml(&repo("games/starquake/game.toml")).expect("it parses");
    let from_json = Definition::from_json(&def.to_json()).expect("its JSON parses");
    assert_eq!(from_json, def);
    let from_toml = Definition::from_toml(&def.to_toml()).expect("its TOML parses");
    assert_eq!(from_toml, def);
}

#[test]
fn json_takes_addresses_as_numbers_or_hex_strings() {
    let json = |sync: &str| {
        format!(
            r#"{{ "schema": 1, "game": {{ "id": "g", "title": "G", "machine": "48k" }},
                "media": [{{ "name": "G" }}], "run": {{ "sync": {sync} }} }}"#
        )
    };
    let hex = Definition::from_json(&json("\"0x6A00\"")).expect("a hex string");
    let number = Definition::from_json(&json("27136")).expect("a number");
    assert_eq!(hex, number);
    assert!(hex.to_json().contains("\"0x6A00\""), "written back as hex");
    assert!(
        Definition::from_json(&json("70000")).is_err(),
        "past 0xFFFF"
    );
}

#[test]
fn an_unknown_key_is_an_error_naming_it() {
    let text = format!("{HEADER}[run]\nsnyc = 0x6A00\n");
    let error = Definition::from_toml(&text)
        .expect_err("a typo")
        .to_string();
    assert!(error.contains("snyc"), "{error}");
}

#[test]
fn broken_references_are_reported_with_their_paths() {
    let text = format!(
        r#"{HEADER}
[values.lives]
addr = 0x8000
type = "u8"

[[cheats]]
id = "endless"
label = "Endless lives"
hold = "lifes"

[[cheats]]
id = "full"
label = "Full lives"
set = {{ value = "lives", to = "max" }}

[pad.default]
west = "jump"
start = {{ keys = ["START"] }}

[pad.menu]
south = "fire"
select = "fire"
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def.check().iter().map(ToString::to_string).collect();
    for expected in [
        "cheats[0].hold: no value named \"lifes\"",
        "cheats[1].set: \"lives\" isn't writable",
        "cheats[1].set: \"lives\" has no max to fill it to",
        "pad.default.west: \"jump\" isn't a direction, fire, menu, none or an action",
        "pad.default.start: \"START\" isn't a Spectrum key",
        "pad.menu: no state named \"menu\"",
        "pad.menu.select: select always opens the guidance menu",
    ] {
        assert!(
            problems.iter().any(|p| p == expected),
            "{expected} in {problems:#?}"
        );
    }
}

#[test]
fn a_joystick_of_the_game_s_own_keys_names_them_all() {
    let problems = |run: &str| -> Vec<String> {
        let text = format!("{HEADER}\n[run]\n{run}\n");
        let def = Definition::from_toml(&text).expect("it parses");
        def.check()
            .iter()
            .map(ToString::to_string)
            .filter(|p| p.starts_with("run."))
            .collect()
    };
    assert_eq!(
        problems(r#"joystick = "keys""#),
        ["run.keys: joystick = \"keys\" needs the game's keys for each direction"]
    );
    assert_eq!(
        problems(
            r#"joystick = "keys"
keys = { up = "Q", down = "A", left = "O", right = "P", fire = "FIRE" }"#
        ),
        ["run.keys: \"FIRE\" isn't a Spectrum key"]
    );
    assert_eq!(
        problems(r#"keys = { up = "Q", down = "A", left = "O", right = "P", fire = "M" }"#),
        ["run.keys: only with joystick = \"keys\""]
    );
}

#[test]
fn the_border_is_shown_unless_a_definition_leaves_it_out() {
    let border = |run: &str| {
        Definition::from_toml(&format!("{HEADER}\n[run]\n{run}\n"))
            .map(|d| d.run.unwrap_or_default().border)
    };
    assert_eq!(border("").expect("it parses"), Border::Full);
    assert_eq!(
        border(r#"border = "none""#).expect("it parses"),
        Border::None
    );
    assert!(border(r#"border = "some""#).is_err());
}

#[test]
fn a_switch_s_call_needs_a_sync_point() {
    let text = format!(
        r#"{HEADER}
[values.energy]
addr = 0x9000
type = "u8"
writable = true
[[cheats]]
id = "full"
label = "Full"
set = {{ value = "energy", to = 9 }}
call = 0xD425
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def.check().iter().map(ToString::to_string).collect();
    assert!(
        problems.contains(
            &"cheats[0].call: a routine is called at the sync point, and there's none (run.sync)"
                .to_owned()
        ),
        "{problems:?}"
    );
    let synced =
        Definition::from_toml(&format!("{text}[run]\nsync = 0x8000\n")).expect("it parses");
    assert!(
        !synced
            .check()
            .iter()
            .any(|p| p.to_string().contains(".call")),
        "with one"
    );
}

#[test]
fn a_tap_lasts_at_least_a_frame_and_a_sequence_s_keys_are_keys() {
    let text = format!(
        r#"{HEADER}
[pad.default]
north = {{ tap = ["P"], frames = 0 }}
east = {{ keys = ["H"], frames = 3 }}
south = {{ sequence = ["1", "NOPE"], frames = 5 }}
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let mut problems: Vec<String> = def
        .check()
        .iter()
        .map(ToString::to_string)
        .filter(|p| p.starts_with("pad."))
        .collect();
    problems.sort();
    assert_eq!(
        problems,
        [
            "pad.default.east: frames is only for a tap or a sequence",
            "pad.default.north: a tap lasts at least one frame",
            "pad.default.south: \"NOPE\" isn't a Spectrum key",
        ]
    );
}

#[test]
fn an_action_s_host_key_is_one_the_spectrum_lacks() {
    let problems = |key: &str| -> Vec<String> {
        let text = format!(
            "{HEADER}\n[[actions]]\nid = \"jump\"\nkeys = [\"SPACE\"]\nhost_key = \"{key}\"\n"
        );
        let def = Definition::from_toml(&text).expect("it parses");
        def.check()
            .iter()
            .map(ToString::to_string)
            .filter(|p| p.starts_with("actions"))
            .collect()
    };
    assert!(problems("Tab").is_empty());
    assert!(problems("PageDown").is_empty());
    assert_eq!(
        problems("F9"),
        ["actions[0].host_key: F9 is the program's own"]
    );
    let q = problems("Q");
    assert_eq!(q.len(), 1);
    assert!(
        q[0].starts_with(
            "actions[0].host_key: \"Q\" isn't one of the keys the Spectrum lacks: Tab, F1,"
        ),
        "{q:?}"
    );
}

#[test]
fn a_notice_is_the_paused_state_s_and_shows_parts_a_pad_has() {
    let text = format!(
        r#"{HEADER}
[[states]]
name = "paused"
role = "paused"
pc = 0x8000
notice = {{ text = "Fire to go on.", hints = [{{ pad = "select", does = "go on" }}, {{ does = "wait" }}] }}

[[states]]
name = "play"
role = "play"
pc = 0x8001
notice = {{ text = "Playing." }}

[[states]]
name = "title"
role = "title"
pc = 0x8002
notice = {{ heading = "Ignore the list", text = "Play with the arrow keys." }}
set = [{{ value = "controls", to = 1 }}, {{ value = "shown", to = 2 }}]

[values.controls]
addr = 0x9000
type = "u8"
writable = true
[values.shown]
addr = 0x9001
type = "u8"
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def
        .check()
        .iter()
        .map(ToString::to_string)
        .filter(|p| p.starts_with("states"))
        .collect();
    assert_eq!(
        problems,
        [
            "states[0].notice.hints[0].pad: \"select\" isn't a part a notice shows: one of dpad, stick, west, south, east, north, start",
            "states[0].notice.hints[1]: needs keys, pad or both",
            "states[1].notice: only the states with role = \"paused\" or \"title\" have a notice",
            "states[2].set: \"shown\" isn't writable",
        ]
    );
}

#[test]
fn a_high_score_table_names_in_text_and_scores_in_numbers() {
    let text = format!(
        r#"{HEADER}
[high_scores]
addr = 0x8000
count = 8
stride = 9
name = {{ offset = 0, len = 3, type = "u8" }}
score = {{ offset = 3, len = 6, type = "text" }}
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def
        .check()
        .iter()
        .map(ToString::to_string)
        .filter(|p| p.starts_with("high_scores"))
        .collect();
    assert_eq!(
        problems,
        [
            "high_scores.name: a name is text",
            "high_scores.score: a score is digits, bcd, u8 or u16",
        ]
    );
}

#[test]
fn the_committed_schema_is_the_generated_one() {
    assert!(
        repo("schema/definition.schema.json") == schema(),
        "schema/definition.schema.json is stale: run `cargo run -p sidekick-def --example schema > schema/definition.schema.json`"
    );
}

#[test]
fn pictures_in_cells_want_whole_cells() {
    let text = format!(
        "{HEADER}[values.pictures]\ntype = \"sprites\"\naddr = 0x8000\ncount = 1\nwidth = 16\nheight = 12\nlayout = \"cells\"\n"
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def.check().iter().map(ToString::to_string).collect();
    assert!(
        problems.contains(&"values.pictures.height: a multiple of 8, for cells".to_owned()),
        "{problems:?}"
    );
    let rows = text.replace("layout = \"cells\"\n", "");
    let def = Definition::from_toml(&rows).expect("it parses");
    assert!(
        !def.check().iter().any(|p| p.path.ends_with("height")),
        "rows may be any height"
    );
}

#[test]
fn meanings_come_from_roles_menus_and_markers_and_the_old_names_are_pointed_out() {
    let text = format!(
        r#"{HEADER}
[values.mode]
addr = 0x8000
type = "u8"

[[states]]
name = "play"
when = {{ value = "mode", equals = 1 }}
[[states]]
name = "running"
role = "play"
when = {{ value = "mode", equals = 2 }}
[[states]]
name = "racing"
role = "play"
when = {{ value = "mode", equals = 3 }}

[[actions]]
id = "end-game"
tap = ["A"]
[[actions]]
id = "quit"
tap = ["Q"]
menu_in = ["running", "nowhere"]

[guidance]
levels = [{{ name = "Off", description = "None." }}, {{ name = "Map", description = "A map." }}]

[[panel]]
kind = "map-grid"
from_level = 1
source = "value:mode"
[[panel]]
kind = "text"
from_level = 1
source = "value:mode"
markers = {{ you = "player" }}
[[panel]]
kind = "map-grid"
from_level = 1
source = "value:mode"
markers = {{ item = "picture" }}
places = {{ item = "top-right", door = "top-left" }}
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def.check().iter().map(|p| p.path.clone()).collect();
    for expected in [
        "states[0].role",
        "states[2].role",
        "actions[0].menu",
        "actions[1].menu_in",
        "panel[0].markers",
        "panel[1].markers",
        "panel[2].pictures",
        "panel[2].places.door",
    ] {
        assert!(
            problems.iter().any(|p| p == expected),
            "{expected} in {problems:#?}"
        );
    }
    assert_eq!(
        problems
            .iter()
            .filter(|p| *p == "actions[1].menu_in")
            .count(),
        2,
        "no menu, and a state it doesn't have"
    );
    assert!(
        !problems.iter().any(|p| p == "panel[2].places.item"),
        "a kind it draws may have a place"
    );
}
