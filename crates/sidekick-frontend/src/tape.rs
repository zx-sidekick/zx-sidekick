//! Finding the player's copy of the game: where to look, which names count,
//! reading a tape out of the zip an archive serves it in, and keeping it.
//!
//! Nothing here knows what the tape holds. Which names it goes by and
//! whether a file is the one this version supports are the [`Game`]'s,
//! whose `accept` checks its SHA-1, so the tests can run on made-up files.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::Game;

/// A tape that passed the check, and the file it came from.
pub struct Tape {
    pub bytes: Vec<u8>,
    pub from: PathBuf,
}

/// Why a file was not taken.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// It could not be read, or is not a zip it claims to be.
    Unreadable(String),
    /// It was read, and it is not the tape: the game's name, and which
    /// release it needs.
    NotTheTape {
        game: &'static str,
        release: &'static str,
    },
    /// A zip with no tape in it at all.
    NoTapeInZip,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Refused::Unreadable(why) => f.write_str(why),
            Refused::NotTheTape { game, release } => {
                write!(
                    f,
                    "that is not the {game} tape this version needs ({release})"
                )
            }
            Refused::NoTapeInZip => f.write_str("that zip has no .tap file in it"),
        }
    }
}

/// Where to look, in order, when the player has not said.
///
/// Beside the executable comes first, because that is what somebody who has
/// just unpacked a release archive will have done. The data dir is where a
/// located tape is kept. The working directory comes last, so a development
/// checkout still finds the tape in `assets/`.
pub fn folders(game: &Game) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.to_path_buf());
        out.push(dir.join("assets"));
    }
    if let Some(dir) = data_dir() {
        out.push(dir.join(game.program));
    }
    out.push(PathBuf::from("."));
    out.push(PathBuf::from("assets"));
    out
}

/// This program's folder in the data dir, where the located tape and the
/// high scores are kept.
pub fn app_dir(game: &Game) -> Option<PathBuf> {
    data_dir().map(|d| d.join(game.program))
}

/// Where this system keeps application data a user installed themselves.
///
/// On Linux and the other unices it is the XDG Base Directory Specification:
/// `$XDG_DATA_HOME`, falling back to `~/.local/share`.
#[cfg(all(unix, not(target_os = "macos")))]
fn data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(xdg));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
}

/// Where this system keeps application data a user installed themselves.
#[cfg(target_os = "macos")]
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
}

/// Where this system keeps application data a user installed themselves.
#[cfg(windows)]
fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

/// The first file in `folders` that goes by one of the game's names and
/// holds a tape its `accept` takes.
///
/// Each folder is listed once and its names compared without regard to
/// case, since asking a case-sensitive file system for `starquake.tap`
/// would miss World of Spectrum's `STARQUAK.TAP`. A candidate that fails is passed over, so a
/// wrong file with a right name cannot hide a good one after it.
pub fn find(game: &Game, folders: &[PathBuf]) -> Option<Tape> {
    for folder in folders {
        let Ok(entries) = fs::read_dir(folder) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        // Listing order is the file system's; sorting keeps the result the
        // same everywhere when a folder holds two spellings of one name.
        files.sort();
        for name in game.names {
            for file in &files {
                let matches = file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(name));
                if matches && let Ok(tape) = load(game, file) {
                    return Some(tape);
                }
            }
        }
    }
    None
}

/// The most a file, or a tape inside a zip, is read of before it is
/// refused: far more than any tape.
const MOST: u64 = 16 * 1024 * 1024;

/// Reads the tape in `path`: the file itself, or, for a zip, the first
/// `.tap` inside it that the game's `accept` takes, whatever that is called.
///
/// # Errors
///
/// If the file cannot be read, or holds no tape the game's `accept` takes.
pub fn load(game: &Game, path: &Path) -> Result<Tape, Refused> {
    let accept = game.accept;
    let not_the_tape = Refused::NotTheTape {
        game: game.name,
        release: game.release,
    };
    // A tape is tens of kilobytes; a file far larger is never one, and
    // reading a video dropped by mistake would freeze the window.
    let size = fs::metadata(path)
        .map_err(|e| Refused::Unreadable(format!("cannot read {}: {e}", path.display())))?
        .len();
    if size > MOST {
        return Err(not_the_tape);
    }
    let bytes = fs::read(path)
        .map_err(|e| Refused::Unreadable(format!("cannot read {}: {e}", path.display())))?;
    let from = path.to_path_buf();
    if !is_zip(path) {
        return if accept(&bytes) {
            Ok(Tape { bytes, from })
        } else {
            Err(not_the_tape)
        };
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| {
        Refused::Unreadable(format!(
            "{} is not a zip that can be read: {e}",
            path.display()
        ))
    })?;
    let mut any_tape = false;
    for i in 0..zip.len() {
        let Ok(mut entry) = zip.by_index(i) else {
            continue;
        };
        let is_tap = Path::new(entry.name())
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("tap"));
        if !is_tap || !entry.is_file() || entry.size() > MOST {
            continue;
        }
        any_tape = true;
        let mut inner = Vec::new();
        if entry.read_to_end(&mut inner).is_ok() && accept(&inner) {
            return Ok(Tape { bytes: inner, from });
        }
    }
    Err(if any_tape {
        not_the_tape
    } else {
        Refused::NoTapeInZip
    })
}

fn is_zip(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

/// Saves a located tape where [`folders`] will find it next time, and
/// returns where that is.
///
/// # Errors
///
/// If this system has no data dir, or it cannot be written.
pub fn keep(game: &Game, bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = data_dir()
        .ok_or("this system has no folder for application data")?
        .join(game.program);
    keep_in(game, &dir, bytes)
}

fn keep_in(game: &Game, dir: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(game.kept);
    fs::write(&path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// The tape in a file named on the command line: a tape, or a zip holding
/// one.
///
/// # Errors
///
/// If the file cannot be read or is not a supported copy of the game.
pub fn read(game: &Game, path: &Path) -> Result<Vec<u8>, String> {
    load(game, path)
        .map(|tape| tape.bytes)
        .map_err(|why| format!("{}: {why}", path.display()))
}

/// What to tell a player when [`find`] came back empty, with no window to
/// ask in: where it looked, and what it looked for.
pub fn not_found_message(game: &Game, folders: &[PathBuf]) -> String {
    let mut msg = format!(
        "error: no copy of {} found.\n\n\
         ZX Sidekick contains no part of the original game: it runs your own copy.\n\
         Put the tape, or the zip it was downloaded in, next to the program, or name\n\
         it on the command line:\n\n    \
         {} path/to/{}\n\nLooked in:\n",
        game.name, game.program, game.kept
    );
    for f in folders {
        msg.push_str(&format!("  {}\n", f.display()));
    }
    msg.push_str(&format!("\nfor any of: {}\n", game.names.join(", ")));
    msg.push_str("\nSee README.md for where to find one.");
    msg
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    /// A made-up "tape": the check is the caller's, so any bytes will do.
    const GOOD: &[u8] = b"the tape";
    const BAD: &[u8] = b"some other tape";

    /// A game whose tape is [`GOOD`], going by Starquake's names.
    const GAME: Game = Game {
        name: "Starquake",
        release: "the original Bubble Bus release",
        program: "zx-sidekick-starquake",
        names: &[
            "starquake.tap",
            "starquak.tap",
            "starquake.tap.zip",
            "starquak.tap.zip",
        ],
        kept: "starquake.tap",
        accept: |bytes| bytes == GOOD,
        ..crate::tests::GAME
    };
    const NOT_THE_TAPE: Refused = Refused::NotTheTape {
        game: "Starquake",
        release: "the original Bubble Bus release",
    };

    /// A fresh, empty folder of the test's own.
    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("starquake-tape-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_file_far_larger_than_a_tape_is_refused_unread() {
        let dir = folder("large");
        let path = dir.join("film.tap");
        // Sparse: 17 MB long without writing them.
        fs::File::create(&path).unwrap().set_len(MOST + 1).unwrap();
        assert_eq!(load(&GAME, &path).err(), Some(NOT_THE_TAPE));
        let _ = fs::remove_dir_all(&dir);
    }

    fn zip_with(path: &Path, entries: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in entries {
            z.start_file(*name, options).unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn every_name_in_any_case() {
        for name in [
            "starquake.tap",
            "STARQUAK.TAP",
            "Starquake.Tap",
            "sTaRqUaK.tAp",
        ] {
            let dir = folder(&format!("case-{name}"));
            fs::write(dir.join(name), GOOD).unwrap();
            let tape = find(&GAME, std::slice::from_ref(&dir))
                .unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tape.bytes, GOOD);
        }
    }

    #[test]
    fn zips_in_any_case() {
        for name in ["starquake.tap.zip", "Starquake.tap.zip", "STARQUAK.TAP.ZIP"] {
            let dir = folder(&format!("zip-{name}"));
            zip_with(&dir.join(name), &[("STARQUAK.TAP", GOOD)]);
            let tape = find(&GAME, std::slice::from_ref(&dir))
                .unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tape.bytes, GOOD);
            assert_eq!(tape.from, dir.join(name));
        }
    }

    #[test]
    fn other_names_are_not_looked_at() {
        let dir = folder("other-names");
        fs::write(dir.join("game.tap"), GOOD).unwrap();
        fs::write(dir.join("starquake.z80"), GOOD).unwrap();
        assert!(find(&GAME, &[dir]).is_none());
    }

    #[test]
    fn a_wrong_file_does_not_hide_a_good_one() {
        let dir = folder("wrong-first");
        fs::write(dir.join("starquake.tap"), BAD).unwrap();
        fs::write(dir.join("STARQUAK.TAP"), GOOD).unwrap();
        assert_eq!(
            find(&GAME, std::slice::from_ref(&dir)).unwrap().from,
            dir.join("STARQUAK.TAP")
        );
    }

    #[test]
    fn tapes_before_zips_and_folders_in_order() {
        let first = folder("order-first");
        let second = folder("order-second");
        zip_with(&first.join("starquake.tap.zip"), &[("x.tap", GOOD)]);
        fs::write(first.join("starquak.tap"), GOOD).unwrap();
        fs::write(second.join("starquake.tap"), GOOD).unwrap();
        let tape = find(&GAME, &[first.clone(), second]).unwrap();
        assert_eq!(tape.from, first.join("starquak.tap"));
    }

    #[test]
    fn a_zip_is_searched_for_the_tape() {
        let dir = folder("zip-inside");
        let path = dir.join("download.zip");
        zip_with(
            &path,
            &[
                ("README.TXT", b"hello"),
                ("OTHER.TAP", BAD),
                ("sub/STARQUAK.TAP", GOOD),
            ],
        );
        assert_eq!(load(&GAME, &path).unwrap().bytes, GOOD);
    }

    #[test]
    fn what_a_refusal_says() {
        let dir = folder("refusals");
        let wrong = dir.join("wrong.tap");
        fs::write(&wrong, BAD).unwrap();
        assert_eq!(load(&GAME, &wrong).err(), Some(NOT_THE_TAPE));

        let wrong_zip = dir.join("wrong.zip");
        zip_with(&wrong_zip, &[("STARQUAK.TAP", BAD)]);
        assert_eq!(load(&GAME, &wrong_zip).err(), Some(NOT_THE_TAPE));

        let empty_zip = dir.join("empty.zip");
        zip_with(&empty_zip, &[("README.TXT", b"hello")]);
        assert_eq!(load(&GAME, &empty_zip).err(), Some(Refused::NoTapeInZip));

        let not_zip = dir.join("broken.zip");
        fs::write(&not_zip, b"not a zip").unwrap();
        assert!(matches!(load(&GAME, &not_zip), Err(Refused::Unreadable(_))));

        assert!(matches!(
            load(&GAME, &dir.join("missing.tap")),
            Err(Refused::Unreadable(_))
        ));
    }
    #[test]
    fn the_program_s_folder_comes_first_and_the_working_folder_last() {
        let list = folders(&GAME);
        let exe = std::env::current_exe().unwrap();
        assert_eq!(list[0], exe.parent().unwrap());
        assert_eq!(list[1], exe.parent().unwrap().join("assets"));
        assert_eq!(
            &list[list.len() - 2..],
            [PathBuf::from("."), PathBuf::from("assets")]
        );
        if let Some(data) = data_dir() {
            assert_eq!(list[2], data.join(GAME.program));
        }
    }

    #[test]
    fn not_found_says_where_it_looked_and_for_what() {
        let msg = not_found_message(&GAME, &[PathBuf::from("/one"), PathBuf::from("/two")]);
        assert!(msg.starts_with("error: no copy of Starquake found."));
        assert!(msg.contains("  /one\n  /two\n"), "{msg}");
        for name in GAME.names {
            assert!(msg.contains(name), "{name}");
        }
    }

    #[test]
    fn reading_a_file_that_is_not_the_tape_says_which_file() {
        let dir = folder("read");
        let path = dir.join("starquake.tap");
        fs::write(&path, BAD).unwrap();
        let err = read(&GAME, &path).unwrap_err();
        assert!(err.starts_with(&path.display().to_string()), "{err}");
        assert!(err.contains("not the Starquake tape"), "{err}");
        let err = read(&GAME, &dir.join("missing.tap")).unwrap_err();
        assert!(err.contains("cannot read"), "{err}");
    }

    #[test]
    fn a_zip_that_cannot_be_read_is_unreadable() {
        let dir = folder("badzip");
        let path = dir.join("starquake.zip");
        fs::write(&path, b"not a zip").unwrap();
        let Err(Refused::Unreadable(why)) = load(&GAME, &path) else {
            panic!("should be unreadable");
        };
        assert!(why.contains("is not a zip"), "{why}");
    }

    #[test]
    fn keeping_fails_where_it_cannot_write() {
        let dir = folder("keepfail");
        let blocker = dir.join("file");
        fs::write(&blocker, b"in the way").unwrap();
        let err = keep_in(&GAME, &blocker, GOOD).unwrap_err();
        assert!(err.starts_with("cannot create"), "{err}");
    }

    #[test]
    fn keeping_creates_the_folder() {
        let dir = folder("keep").join("deeper").join(GAME.program);
        let path = keep_in(&GAME, &dir, GOOD).unwrap();
        assert_eq!(path, dir.join(GAME.kept));
        assert_eq!(fs::read(&path).unwrap(), GOOD);
        assert_eq!(find(&GAME, &[dir]).unwrap().bytes, GOOD);
    }
}
