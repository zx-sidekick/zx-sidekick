//! `sidekick check` and `sidekick convert`, run as the player or an author would.

use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

/// The SHA-1 of the three bytes "abc".
const ABC_SHA1: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";

const COMPLETE: &str = r#"schema = 1
[game]
id = "example"
title = "Example"
machine = "48k"
[[media]]
name = "Example"
sha1 = "a9993e364706816aba3e25717850c26c9cd0d89d"
[values.lives]
addr = 0x8F21
type = "u8"
writable = true
[[cheats]]
id = "endless-lives"
label = "Endless lives"
hold = "lives"
"#;

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

fn sidekick(args: &[&str]) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_sidekick"))
        .args(args)
        .output()
        .expect("sidekick runs");
    Run {
        status: output.status.code().expect("an exit status"),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn path(p: &Path) -> &str {
    p.to_str().expect("a UTF-8 path")
}

#[test]
fn a_complete_definition_checks_clean() {
    let dir = TempDir::new().unwrap();
    let def = dir.path().join("example.toml");
    fs::write(&def, COMPLETE).unwrap();
    let run = sidekick(&["check", path(&def)]);
    assert_eq!(run.status, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn starquake_s_definition_checks_clean() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/starquake");
    let run = sidekick(&["check", path(&folder)]);
    assert_eq!(run.status, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout, "");
}

#[test]
fn a_tape_is_checked_against_the_definition_s_dumps() {
    let dir = TempDir::new().unwrap();
    let def = dir.path().join("example.toml");
    fs::write(&def, COMPLETE).unwrap();
    let ours = dir.path().join("anything.tap");
    fs::write(&ours, b"abc").unwrap();
    let theirs = dir.path().join("other.tap");
    fs::write(&theirs, b"something else").unwrap();

    let run = sidekick(&["check", path(&def), path(&ours)]);
    assert_eq!(run.status, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("one of its dumps"), "{}", run.stdout);

    let run = sidekick(&["check", path(&def), path(&theirs)]);
    assert_eq!(run.status, 1);
    assert!(
        run.stdout.contains("isn't one of its dumps (SHA-1 "),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains(ABC_SHA1), "names the tape's own SHA-1");
}

#[test]
fn convert_goes_to_json_and_back() {
    let dir = TempDir::new().unwrap();
    let toml = dir.path().join("example.toml");
    fs::write(&toml, COMPLETE).unwrap();
    let json = dir.path().join("example.json");
    let run = sidekick(&["convert", path(&toml), "-o", path(&json)]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(fs::read_to_string(&json).unwrap().contains("\"0x8F21\""));

    let run = sidekick(&["convert", path(&json)]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    let back = dir.path().join("back.toml");
    fs::write(&back, &run.stdout).unwrap();
    let run = sidekick(&["check", path(&back)]);
    assert_eq!(
        run.status, 0,
        "the round trip still checks clean: {}",
        run.stdout
    );
}

#[test]
fn something_unreadable_exits_2_with_its_reason_on_standard_error() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.toml");
    let run = sidekick(&["check", path(&missing)]);
    assert_eq!(run.status, 2);
    assert!(run.stderr.contains("missing.toml"), "{}", run.stderr);
    assert_eq!(run.stdout, "");

    let broken = dir.path().join("broken.toml");
    fs::write(&broken, "schema = 1\n[game]\nsnyc = 1\n").unwrap();
    let run = sidekick(&["convert", path(&broken)]);
    assert_eq!(run.status, 2);
    assert!(run.stderr.contains("snyc"), "{}", run.stderr);
}

#[test]
fn check_compiles_the_script_and_finds_its_action_functions() {
    let dir = TempDir::new().unwrap();
    let game = dir.path().join("game");
    fs::create_dir_all(game.join("scripts")).unwrap();
    let definition = format!(
        "{COMPLETE}[[actions]]\nid = \"route\"\nscript = \"next_route\"\n[script]\nfile = \"scripts/game.rhai\"\n"
    );
    fs::write(game.join("game.toml"), definition).unwrap();

    fs::write(
        game.join("scripts/game.rhai"),
        "fn on_frame(mem) {\n  #{ x: \n}\n",
    )
    .unwrap();
    let broken = sidekick(&["check", path(&game)]);
    assert_eq!(broken.status, 1, "{}{}", broken.stdout, broken.stderr);
    assert!(
        broken
            .stdout
            .starts_with("script: scripts/game.rhai: line 3: "),
        "{}",
        broken.stdout
    );

    fs::write(game.join("scripts/game.rhai"), "fn on_frame(mem) { #{} }\n").unwrap();
    let missing = sidekick(&["check", path(&game)]);
    assert_eq!(
        missing.stdout,
        "actions[0].script: the script has no function next_route\n"
    );

    fs::write(
        game.join("scripts/game.rhai"),
        "fn on_frame(mem) { #{} }\nfn next_route(mem) { #{} }\n",
    )
    .unwrap();
    let clean = sidekick(&["check", path(&game)]);
    assert_eq!((clean.status, clean.stdout.as_str()), (0, ""));

    fs::remove_file(game.join("scripts/game.rhai")).unwrap();
    let gone = sidekick(&["check", path(&game)]);
    assert!(
        gone.stdout.starts_with("script: scripts/game.rhai: "),
        "{}",
        gone.stdout
    );
}

#[test]
fn a_warning_is_said_but_doesn_t_fail_the_check() {
    let dir = TempDir::new().unwrap();
    let def = dir.path().join("example.toml");
    fs::write(
        &def,
        format!("{COMPLETE}\n[[states]]\nname = \"play\"\npc = 0x6000\n"),
    )
    .unwrap();
    let run = sidekick(&["check", path(&def)]);
    assert_eq!(run.status, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.starts_with("states[0].role: warning: "),
        "{}",
        run.stdout
    );
}

#[test]
fn import_pok_writes_switches_a_definition_takes() {
    let dir = TempDir::new().unwrap();
    let pok = dir.path().join("game.pok");
    fs::write(&pok, "NInfinite lives\nZ 8 36643 0 53\nY\n").unwrap();
    let run = sidekick(&["import-pok", path(&pok)]);
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(
        run.stdout.contains("id = \"infinite-lives\""),
        "{}",
        run.stdout
    );

    // The definition with them, less its own switch, checks clean.
    let without = COMPLETE[..COMPLETE.find("[[cheats]]").unwrap()].to_owned();
    let def = dir.path().join("game.toml");
    fs::write(&def, format!("{without}\n{}", run.stdout)).unwrap();
    let check = sidekick(&["check", path(&def)]);
    assert_eq!((check.status, check.stdout.as_str()), (0, ""));

    fs::write(&pok, "Z 8 1 2 3\n").unwrap();
    let broken = sidekick(&["import-pok", path(&pok)]);
    assert_eq!(broken.status, 2);
    assert!(broken.stderr.contains("line 1"), "{}", broken.stderr);
}

#[test]
fn check_refuses_a_hint_wider_than_the_panel_s_line() {
    let dir = TempDir::new().unwrap();
    let def = dir.path().join("example.toml");
    let levels = "[[guidance.levels]]\nname = \"Off\"\ndescription = \"None.\"\n[[guidance.levels]]\nname = \"On\"\ndescription = \"On.\"\n";
    let hint = |text: &str| {
        format!("{COMPLETE}{levels}[[hints]]\ntext = \"{text}\"\npriority = 1\nwhen = {{ value = \"lives\", at_most = 1 }}\n")
    };
    fs::write(&def, hint("Lives low: go carefully")).unwrap();
    let fits = sidekick(&["check", path(&def)]);
    assert_eq!((fits.status, fits.stdout.as_str()), (0, ""));
    fs::write(
        &def,
        hint("Lives low: go carefully, keep away from everything that moves, and rest"),
    )
    .unwrap();
    let wide = sidekick(&["check", path(&def)]);
    assert_eq!(wide.status, 1, "{}", wide.stdout);
    assert!(
        wide.stdout
            .starts_with("hints[0].text: too wide for the panel's line"),
        "{}",
        wide.stdout
    );
}
