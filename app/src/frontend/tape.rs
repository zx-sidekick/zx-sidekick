//! Finding the player's copy of the game: where to look, which names count,
//! reading a tape out of the zip an archive serves it in, and keeping it.
//!
//! Nothing here knows what the tape holds. Whether a file is the one this
//! version supports is decided by its SHA-1, which the caller passes in as
//! `accept`, so the tests can run on made-up files.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The names a tape goes by, in the order they are tried: this project's,
/// and the 8.3 name World of Spectrum's zip holds (`STARQUAK.TAP`), each
/// bare and zipped. Compared without regard to case.
pub const NAMES: [&str; 4] = [
    "starquake.tap",
    "starquak.tap",
    "starquake.tap.zip",
    "starquak.tap.zip",
];

/// What the program keeps a located tape as, in the data dir.
const KEPT: &str = "starquake.tap";

/// The folder under the data dir that is this program's.
const APP: &str = "zx-sidekick-starquake";

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
    /// It was read, and it is not the tape.
    NotTheTape,
    /// A zip with no tape in it at all.
    NoTapeInZip,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Refused::Unreadable(why) => f.write_str(why),
            Refused::NotTheTape => f.write_str(
                "that is not the Starquake tape this version needs (the original Bubble Bus release)",
            ),
            Refused::NoTapeInZip => f.write_str("that zip has no .tap file in it"),
        }
    }
}

/// Where to look, in order, when the player has not said.
///
/// Beside the executable comes first, because that is what somebody who has
/// just unpacked a release archive will have done. The data dir is where a
/// located tape is kept. The working directory comes last, so a development
/// checkout still finds `assets/starquake.tap`.
pub fn folders() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.to_path_buf());
        out.push(dir.join("assets"));
    }
    if let Some(dir) = data_dir() {
        out.push(dir.join(APP));
    }
    out.push(PathBuf::from("."));
    out.push(PathBuf::from("assets"));
    out
}

/// This program's folder in the data dir, where the located tape and the
/// high scores are kept.
pub fn app_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join(APP))
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

/// The first file in `folders` that goes by one of [`NAMES`] and holds a
/// tape `accept` takes.
///
/// Each folder is listed once and its names compared without regard to
/// case, since asking a case-sensitive file system for `starquake.tap`
/// would miss `STARQUAK.TAP`. A candidate that fails is passed over, so a
/// wrong file with a right name cannot hide a good one after it.
pub fn find(folders: &[PathBuf], accept: impl Fn(&[u8]) -> bool) -> Option<Tape> {
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
        for name in NAMES {
            for file in &files {
                let matches = file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(name));
                if matches && let Ok(tape) = load(file, &accept) {
                    return Some(tape);
                }
            }
        }
    }
    None
}

/// Reads the tape in `path`: the file itself, or, for a zip, the first
/// `.tap` inside it that `accept` takes, whatever that is called.
///
/// # Errors
///
/// If the file cannot be read, or holds no tape `accept` takes.
pub fn load(path: &Path, accept: impl Fn(&[u8]) -> bool) -> Result<Tape, Refused> {
    let bytes = fs::read(path)
        .map_err(|e| Refused::Unreadable(format!("cannot read {}: {e}", path.display())))?;
    let from = path.to_path_buf();
    if !is_zip(path) {
        return if accept(&bytes) {
            Ok(Tape { bytes, from })
        } else {
            Err(Refused::NotTheTape)
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
        if !is_tap || !entry.is_file() {
            continue;
        }
        any_tape = true;
        let mut inner = Vec::new();
        if entry.read_to_end(&mut inner).is_ok() && accept(&inner) {
            return Ok(Tape { bytes: inner, from });
        }
    }
    Err(if any_tape {
        Refused::NotTheTape
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
pub fn keep(bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = data_dir()
        .ok_or("this system has no folder for application data")?
        .join(APP);
    keep_in(&dir, bytes)
}

fn keep_in(dir: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(KEPT);
    fs::write(&path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// The tape in a file named on the command line: a tape, or a zip holding
/// one.
///
/// # Errors
///
/// If the file cannot be read or is not a supported copy of the game.
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    load(path, sidekick::starquake::is_supported_tape)
        .map(|tape| tape.bytes)
        .map_err(|why| format!("{}: {why}", path.display()))
}

/// What to tell a player when [`find`] came back empty, with no window to
/// ask in: where it looked, and what it looked for.
pub fn not_found_message(folders: &[PathBuf]) -> String {
    let mut msg = String::from(
        "error: no copy of Starquake found.\n\n\
         ZX Sidekick contains no part of the original game: it runs your own copy.\n\
         Put the tape, or the zip it was downloaded in, next to the program, or name\n\
         it on the command line:\n\n    \
         zx-sidekick-starquake path/to/starquake.tap\n\nLooked in:\n",
    );
    for f in folders {
        msg.push_str(&format!("  {}\n", f.display()));
    }
    msg.push_str(&format!("\nfor any of: {}\n", NAMES.join(", ")));
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

    fn accept(bytes: &[u8]) -> bool {
        bytes == GOOD
    }

    /// A fresh, empty folder of the test's own.
    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("starquake-tape-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
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
            let tape = find(std::slice::from_ref(&dir), accept)
                .unwrap_or_else(|| panic!("{name} not found"));
            assert_eq!(tape.bytes, GOOD);
        }
    }

    #[test]
    fn zips_in_any_case() {
        for name in ["starquake.tap.zip", "Starquake.tap.zip", "STARQUAK.TAP.ZIP"] {
            let dir = folder(&format!("zip-{name}"));
            zip_with(&dir.join(name), &[("STARQUAK.TAP", GOOD)]);
            let tape = find(std::slice::from_ref(&dir), accept)
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
        assert!(find(&[dir], accept).is_none());
    }

    #[test]
    fn a_wrong_file_does_not_hide_a_good_one() {
        let dir = folder("wrong-first");
        fs::write(dir.join("starquake.tap"), BAD).unwrap();
        fs::write(dir.join("STARQUAK.TAP"), GOOD).unwrap();
        assert_eq!(
            find(std::slice::from_ref(&dir), accept).unwrap().from,
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
        let tape = find(&[first.clone(), second], accept).unwrap();
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
        assert_eq!(load(&path, accept).unwrap().bytes, GOOD);
    }

    #[test]
    fn what_a_refusal_says() {
        let dir = folder("refusals");
        let wrong = dir.join("wrong.tap");
        fs::write(&wrong, BAD).unwrap();
        assert_eq!(load(&wrong, accept).err(), Some(Refused::NotTheTape));

        let wrong_zip = dir.join("wrong.zip");
        zip_with(&wrong_zip, &[("STARQUAK.TAP", BAD)]);
        assert_eq!(load(&wrong_zip, accept).err(), Some(Refused::NotTheTape));

        let empty_zip = dir.join("empty.zip");
        zip_with(&empty_zip, &[("README.TXT", b"hello")]);
        assert_eq!(load(&empty_zip, accept).err(), Some(Refused::NoTapeInZip));

        let not_zip = dir.join("broken.zip");
        fs::write(&not_zip, b"not a zip").unwrap();
        assert!(matches!(
            load(&not_zip, accept),
            Err(Refused::Unreadable(_))
        ));

        assert!(matches!(
            load(&dir.join("missing.tap"), accept),
            Err(Refused::Unreadable(_))
        ));
    }
    #[test]
    fn the_program_s_folder_comes_first_and_the_working_folder_last() {
        let list = folders();
        let exe = std::env::current_exe().unwrap();
        assert_eq!(list[0], exe.parent().unwrap());
        assert_eq!(list[1], exe.parent().unwrap().join("assets"));
        assert_eq!(
            &list[list.len() - 2..],
            [PathBuf::from("."), PathBuf::from("assets")]
        );
        if let Some(data) = data_dir() {
            assert_eq!(list[2], data.join(APP));
        }
    }

    #[test]
    fn not_found_says_where_it_looked_and_for_what() {
        let msg = not_found_message(&[PathBuf::from("/one"), PathBuf::from("/two")]);
        assert!(msg.starts_with("error: no copy of Starquake found."));
        assert!(msg.contains("  /one\n  /two\n"), "{msg}");
        for name in NAMES {
            assert!(msg.contains(name), "{name}");
        }
    }

    #[test]
    fn reading_a_file_that_is_not_the_tape_says_which_file() {
        let dir = folder("read");
        let path = dir.join("starquake.tap");
        fs::write(&path, BAD).unwrap();
        let err = read(&path).unwrap_err();
        assert!(err.starts_with(&path.display().to_string()), "{err}");
        assert!(err.contains("not the Starquake tape"), "{err}");
        let err = read(&dir.join("missing.tap")).unwrap_err();
        assert!(err.contains("cannot read"), "{err}");
    }

    #[test]
    fn a_zip_that_cannot_be_read_is_unreadable() {
        let dir = folder("badzip");
        let path = dir.join("starquake.zip");
        fs::write(&path, b"not a zip").unwrap();
        let Err(Refused::Unreadable(why)) = load(&path, accept) else {
            panic!("should be unreadable");
        };
        assert!(why.contains("is not a zip"), "{why}");
    }

    #[test]
    fn keeping_fails_where_it_cannot_write() {
        let dir = folder("keepfail");
        let blocker = dir.join("file");
        fs::write(&blocker, b"in the way").unwrap();
        let err = keep_in(&blocker, GOOD).unwrap_err();
        assert!(err.starts_with("cannot create"), "{err}");
    }

    #[test]
    fn keeping_creates_the_folder() {
        let dir = folder("keep").join("deeper").join(APP);
        let path = keep_in(&dir, GOOD).unwrap();
        assert_eq!(path, dir.join(KEPT));
        assert_eq!(fs::read(&path).unwrap(), GOOD);
        assert_eq!(find(&[dir], accept).unwrap().bytes, GOOD);
    }
}
