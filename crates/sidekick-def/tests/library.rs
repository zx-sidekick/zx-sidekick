//! Definitions found on disk in their three forms, and tapes matched to them by SHA-1.

use sidekick_def::{Identified, Library, LoadProblem, Package, TapeFormat};
use std::{fs, io::Write, path::Path};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

/// The SHA-1 of the three bytes "abc".
const ABC_SHA1: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";

fn definition(id: &str, title: &str, sha1: &str) -> String {
    format!(
        r#"schema = 1
[game]
id = "{id}"
title = "{title}"
machine = "48k"
[[media]]
name = "{title}"
sha1 = "{sha1}"
"#
    )
}

fn zip(path: &Path, files: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(fs::File::create(path).expect("a zip file"));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in files {
        zip.start_file(*name, options).expect("an entry");
        zip.write_all(bytes).expect("its bytes");
    }
    zip.finish().expect("the zip");
}

#[test]
fn a_definition_loads_from_a_file_a_folder_or_a_zip() {
    let dir = TempDir::new().expect("a folder");
    let base = dir.path();
    fs::write(
        base.join("toml-game.toml"),
        definition("toml-game", "TOML", ABC_SHA1),
    )
    .unwrap();
    let json = sidekick_def::Definition::from_toml(&definition("json-game", "JSON", ABC_SHA1))
        .unwrap()
        .to_json();
    fs::write(base.join("json-game.json"), json).unwrap();
    fs::create_dir_all(base.join("folder-game/scripts")).unwrap();
    fs::write(
        base.join("folder-game/game.toml"),
        definition("folder-game", "Folder", ABC_SHA1),
    )
    .unwrap();
    fs::write(base.join("folder-game/scripts/s.rhai"), "// script").unwrap();
    zip(
        &base.join("zip-game.zip"),
        &[
            (
                "game.toml",
                definition("zip-game", "Zip", ABC_SHA1).as_bytes(),
            ),
            ("scripts/s.rhai", b"// zipped script"),
        ],
    );

    let (library, problems) = Library::search(&[base.to_owned()]);
    assert!(problems.is_empty(), "{problems:?}");
    let ids: Vec<&str> = library
        .games()
        .map(|p| p.definition.game.id.as_str())
        .collect();
    assert_eq!(ids, ["folder-game", "json-game", "toml-game", "zip-game"]);

    let folder = library.get("folder-game").unwrap();
    assert_eq!(folder.read("scripts/s.rhai").unwrap(), b"// script");
    let zipped = library.get("zip-game").unwrap();
    assert_eq!(zipped.read("scripts/s.rhai").unwrap(), b"// zipped script");
    assert!(
        library
            .get("toml-game")
            .unwrap()
            .read("scripts/s.rhai")
            .is_err(),
        "a lone file has no others"
    );
}

#[test]
fn a_package_s_files_stay_inside_it() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("game")).unwrap();
    fs::write(
        dir.path().join("game/game.toml"),
        definition("g", "G", ABC_SHA1),
    )
    .unwrap();
    fs::write(dir.path().join("secret.txt"), "outside").unwrap();
    let package = Package::load(&dir.path().join("game")).unwrap();
    for escape in [
        "../secret.txt",
        "/etc/hosts",
        "./game.toml/../../secret.txt",
    ] {
        let error = package.read(escape).expect_err(escape);
        assert!(
            matches!(error.problem, LoadProblem::NoSuchFile(_)),
            "{escape}: {error}"
        );
    }
}

#[test]
fn a_later_folder_replaces_a_game_and_a_broken_file_is_reported() {
    let shipped = TempDir::new().unwrap();
    let players = TempDir::new().unwrap();
    fs::write(
        shipped.path().join("g.toml"),
        definition("g", "Shipped", ABC_SHA1),
    )
    .unwrap();
    fs::write(
        players.path().join("g.toml"),
        definition("g", "Player's", ABC_SHA1),
    )
    .unwrap();
    fs::write(players.path().join("broken.toml"), "schema = 1\n[game]\n").unwrap();
    fs::write(players.path().join("README.md"), "not a definition").unwrap();

    let (library, problems) = Library::search(&[
        shipped.path().to_owned(),
        players.path().to_owned(),
        shipped.path().join("does-not-exist"),
    ]);
    assert_eq!(library.get("g").unwrap().definition.game.title, "Player's");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].path.ends_with("broken.toml"));
}

#[test]
fn a_tape_is_matched_by_its_sha1_whatever_it_is_called() {
    let games = TempDir::new().unwrap();
    fs::write(games.path().join("g.toml"), definition("g", "G", ABC_SHA1)).unwrap();
    let (library, _) = Library::search(&[games.path().to_owned()]);

    let downloads = TempDir::new().unwrap();
    let tape = downloads.path().join("Something Else.TAP");
    fs::write(&tape, b"abc").unwrap();
    match library.identify(&tape).unwrap() {
        Identified::Game {
            package,
            media,
            tape,
        } => {
            assert_eq!(package.definition.game.id, "g");
            assert_eq!(media, 0);
            assert_eq!(tape.sha1, ABC_SHA1);
            assert_eq!(tape.format, TapeFormat::Tap);
        }
        Identified::Unknown { .. } => panic!("the tape matches"),
    }

    let zipped = downloads.path().join("download.zip");
    zip(&zipped, &[("readme.txt", b"hello"), ("GAME/G.TZX", b"abc")]);
    match library.identify(&zipped).unwrap() {
        Identified::Game { tape, .. } => {
            assert_eq!(tape.name, "GAME/G.TZX");
            assert_eq!(tape.format, TapeFormat::Tzx);
        }
        Identified::Unknown { .. } => panic!("the zipped tape matches"),
    }
}

#[test]
fn an_unknown_tape_is_handed_back_with_its_sha1() {
    let (library, _) = Library::search(&[]);
    let dir = TempDir::new().unwrap();
    let tape = dir.path().join("other.tap");
    fs::write(&tape, b"other bytes").unwrap();
    match library.identify(&tape).unwrap() {
        Identified::Unknown { tapes } => {
            assert_eq!(tapes.len(), 1);
            assert_eq!(tapes[0].bytes, b"other bytes");
            assert_eq!(tapes[0].sha1.len(), 40);
        }
        Identified::Game { .. } => panic!("nothing defines it"),
    }

    let empty = dir.path().join("no-tapes.zip");
    zip(&empty, &[("readme.txt", b"hello")]);
    assert!(library.identify(&empty).is_err(), "a zip with no tape");
    let not_zip = dir.path().join("broken.zip");
    fs::write(&not_zip, b"not a zip").unwrap();
    assert!(matches!(
        library.identify(&not_zip).unwrap_err().problem,
        LoadProblem::Zip(_)
    ));
}
