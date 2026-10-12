//! Format 1, as `docs/definition-format.md` and the committed definitions write it.

use sidekick_def::{schema, Definition};
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
    for expected in [
        "values.room.addr: missing",
        "values.items.count: missing",
        "situations.enter: missing",
    ] {
        assert!(
            problems.iter().any(|p| p == expected),
            "{expected} in {problems:#?}"
        );
    }
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
fn a_tap_lasts_at_least_a_frame() {
    let text = format!(
        r#"{HEADER}
[pad.default]
north = {{ tap = ["P"], frames = 0 }}
east = {{ keys = ["H"], frames = 3 }}
"#
    );
    let def = Definition::from_toml(&text).expect("it parses");
    let problems: Vec<String> = def
        .check()
        .iter()
        .map(ToString::to_string)
        .filter(|p| p.starts_with("pad."))
        .collect();
    assert_eq!(
        problems,
        [
            "pad.default.east: frames is only for a tap",
            "pad.default.north: a tap lasts at least one frame",
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
