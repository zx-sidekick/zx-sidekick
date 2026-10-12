//! Saves: the whole machine and the program's own part of the game, in a zip; the quick slot;
//! and the checks before one is loaded.

use serde::{Deserialize, Serialize};
use sidekick_core::{Progress, Record, Session};
use std::{
    collections::BTreeSet,
    fs, io,
    io::{Cursor, Read as _, Write as _},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

/// The machine's part: a standard SZX snapshot, which any emulator opens.
const MACHINE: &str = "machine.szx";
/// The program's part.
const STATE: &str = "state.toml";
/// The quick slot's file.
pub(crate) const QUICK: &str = "quick.sav";
/// A save's format, so a later program can tell an older save.
const FORMAT: u32 = 1;
/// What an SZX snapshot starts with.
const SZX_MAGIC: &[u8] = b"ZXST";

/// Which game a session plays, from which dump, and where its quick slot is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Saves {
    /// The definition's id; none for a tape played plainly.
    pub(crate) game: Option<String>,
    /// The tape's SHA-1.
    pub(crate) dump: String,
    /// The quick slot.
    pub(crate) quick: PathBuf,
}

impl Saves {
    /// A defined game's: its quick slot in its data folder.
    pub(crate) fn of_game(id: &str, dump: &str, folder: &Path) -> Self {
        Self {
            game: Some(id.to_owned()),
            dump: dump.to_owned(),
            quick: folder.join(QUICK),
        }
    }

    /// A tape played plainly: its quick slot in a folder of the tape's name in the data
    /// folder's `tapes/`.
    pub(crate) fn of_tape(name: &str, dump: &str, data: &Path) -> Self {
        let stem = Path::new(name)
            .file_stem()
            .map_or_else(|| name.to_owned(), |s| s.to_string_lossy().into_owned());
        Self {
            game: None,
            dump: dump.to_owned(),
            quick: data.join("tapes").join(stem).join(QUICK),
        }
    }
}

/// The program's part of a save.
#[derive(Debug, Serialize, Deserialize)]
struct State {
    save: u32,
    game: Option<String>,
    dump: String,
    level: u8,
    switches: BTreeSet<String>,
    record: Played,
    progress: Got,
}

/// The game's record, as `state.toml` has it.
#[derive(Debug, Serialize, Deserialize)]
struct Played {
    level: u8,
    switches: BTreeSet<String>,
    loaded: bool,
}

/// How far the session had got, as `state.toml` has it.
#[derive(Debug, Serialize, Deserialize)]
struct Got {
    last_state: Option<String>,
    seen_title: bool,
    in_game: bool,
}

/// Why a save wasn't loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// Nothing in the slot.
    NoSave,
    /// Not a save, or one this program can't read.
    NotASave,
    /// Another game's.
    OtherGame,
    /// The game's, from another dump.
    OtherDump,
    /// The machine wouldn't take it: another model's.
    OtherMachine,
}

impl Refused {
    /// What the notice says.
    pub(crate) const fn words(self) -> &'static str {
        match self {
            Self::NoSave => "No save yet",
            Self::NotASave => "That isn't a save",
            Self::OtherGame => "That save is of another game",
            Self::OtherDump => "That save is from another tape",
            Self::OtherMachine => "That save is for another Spectrum",
        }
    }
}

/// Saves the session to `path`, between frames: the machine whole, and the guidance level,
/// switches, record and progress. Written beside it first, then moved over it, so a save cut
/// short never replaces a good one.
///
/// # Errors
/// If the machine can't be saved or the file can't be written.
pub(crate) fn write(session: &mut Session, saves: &Saves, path: &Path) -> Result<(), String> {
    let szx = session
        .machine_mut()
        .save_szx()
        .map_err(|e| e.to_string())?;
    let record = session.record();
    let progress = session.progress();
    let state = State {
        save: FORMAT,
        game: saves.game.clone(),
        dump: saves.dump.clone(),
        level: session.level(),
        switches: session.switches_on().clone(),
        record: Played {
            level: record.level,
            switches: record.switches.clone(),
            loaded: record.loaded,
        },
        progress: Got {
            last_state: progress.last_known.clone(),
            seen_title: progress.seen_title,
            in_game: progress.in_game,
        },
    };
    let text = toml::to_string(&state).map_err(|e| e.to_string())?;
    let zip = || -> zip::result::ZipResult<Vec<u8>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(MACHINE, SimpleFileOptions::default())?;
        zip.write_all(&szx)?;
        zip.start_file(STATE, SimpleFileOptions::default())?;
        zip.write_all(text.as_bytes())?;
        Ok(zip.finish()?.into_inner())
    };
    let bytes = zip().map_err(|e| e.to_string())?;
    let partial = path.with_extension("part");
    path.parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&partial, bytes))
        .and_then(|()| fs::rename(&partial, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Loads a save into the session: a `.sav` whole, once it's found to be this game's from this
/// dump; a plain SZX snapshot as the machine alone.
///
/// # Errors
/// Why it wasn't loaded; the session is left as it was, unless the machine itself refused
/// the snapshot part-way.
pub(crate) fn read(path: &Path, session: &mut Session, saves: &Saves) -> Result<(), Refused> {
    let bytes = fs::read(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => Refused::NoSave,
        _ => Refused::NotASave,
    })?;
    if bytes.starts_with(SZX_MAGIC) {
        session
            .machine_mut()
            .load_szx(&bytes)
            .map_err(|_| Refused::OtherMachine)?;
        session.loaded_snapshot();
        return Ok(());
    }
    let mut zip = ZipArchive::new(Cursor::new(&bytes)).map_err(|_| Refused::NotASave)?;
    let text = String::from_utf8(entry(&mut zip, STATE)?).map_err(|_| Refused::NotASave)?;
    let state: State = toml::from_str(&text).map_err(|_| Refused::NotASave)?;
    if state.save != FORMAT {
        return Err(Refused::NotASave);
    }
    if state.game != saves.game {
        return Err(Refused::OtherGame);
    }
    if !state.dump.eq_ignore_ascii_case(&saves.dump) {
        return Err(Refused::OtherDump);
    }
    let szx = entry(&mut zip, MACHINE)?;
    session
        .machine_mut()
        .load_szx(&szx)
        .map_err(|_| Refused::OtherMachine)?;
    session.restore(
        state.level,
        &state.switches,
        Record {
            level: state.record.level,
            switches: state.record.switches,
            loaded: state.record.loaded,
        },
        Progress {
            last_known: state.progress.last_state,
            seen_title: state.progress.seen_title,
            in_game: state.progress.in_game,
        },
    );
    Ok(())
}

/// A file in a save's zip, whole.
fn entry(zip: &mut ZipArchive<Cursor<&Vec<u8>>>, name: &str) -> Result<Vec<u8>, Refused> {
    let mut file = zip.by_name(name).map_err(|_| Refused::NotASave)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| Refused::NotASave)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick_core::{Machine, Model};
    use sidekick_def::Definition;

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
when = { value = "mode", equals = 1 }
[[states]]
name = "play"
when = { value = "mode", equals = 2 }

[[cheats]]
id = "no-harm"
label = "No harm"
patch = [{ addr = 0x8010, bytes = "00", expect = "CD" }]
"#;

    /// Frames for the 48K ROM to start up and settle into its editor.
    const BOOT_FRAMES: usize = 100;

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

    fn saves(folder: &Path) -> Saves {
        Saves::of_game("test", "ABC123", folder)
    }

    /// A session in play, guidance at 2, the switch on, a byte of its own in memory.
    fn played() -> Session {
        let mut s = session();
        mode(&mut s, 1);
        s.machine_mut().write(0x8010, 0xCD);
        s.set_level(2);
        s.switch("no-harm", true).expect("its bytes are there");
        mode(&mut s, 2);
        s.machine_mut().write(0x9000, 0x42);
        s
    }

    #[test]
    fn a_save_puts_back_the_machine_and_the_game_s_settings() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let mut s = played();
        write(&mut s, &saves, &saves.quick).unwrap();
        assert!(saves.quick.exists());
        assert!(!saves.quick.with_extension("part").exists(), "moved over");

        let mut fresh = session();
        read(&saves.quick, &mut fresh, &saves).unwrap();
        assert_eq!(fresh.machine().peek(0x9000), 0x42, "the machine's memory");
        assert_eq!(fresh.machine().peek(0x8010), 0x00, "the patch, as saved");
        assert_eq!(fresh.level(), 2);
        assert!(fresh.switched_on("no-harm"));
        assert_eq!(fresh.record().level, 2);
        assert!(fresh.record().switches.contains("no-harm"));
        assert!(fresh.record().loaded, "marked loaded");
        assert_eq!(fresh.progress(), s.progress());
        mode(&mut fresh, 2);
        assert!(fresh.record().loaded, "the same game goes on");
        assert!(fresh.take_milestones().is_empty(), "no first title again");

        fresh.switch("no-harm", false).unwrap();
        assert_eq!(fresh.machine().peek(0x8010), 0xCD, "off, its bytes go back");
    }

    #[test]
    fn another_game_s_or_dump_s_save_is_refused() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let mut s = played();
        write(&mut s, &saves, &saves.quick).unwrap();
        let mut fresh = session();
        let other_dump = Saves {
            dump: "DEF456".to_owned(),
            ..saves.clone()
        };
        assert_eq!(
            read(&saves.quick, &mut fresh, &other_dump),
            Err(Refused::OtherDump)
        );
        let other_game = Saves::of_tape("test.tap", "ABC123", folder.path());
        assert_eq!(
            read(&saves.quick, &mut fresh, &other_game),
            Err(Refused::OtherGame)
        );
        assert_ne!(fresh.machine().peek(0x9000), 0x42, "left as it was");
        assert!(!fresh.record().loaded);
        assert_eq!(
            read(&folder.path().join("none.sav"), &mut fresh, &saves),
            Err(Refused::NoSave)
        );
        let junk = folder.path().join("junk.sav");
        fs::write(&junk, b"not a zip").unwrap();
        assert_eq!(read(&junk, &mut fresh, &saves), Err(Refused::NotASave));
    }

    #[test]
    fn a_plain_snapshot_loads_the_machine_alone_and_marks_the_game() {
        let folder = tempfile::TempDir::new().unwrap();
        let mut s = played();
        let szx = folder.path().join("game.szx");
        fs::write(&szx, s.machine_mut().save_szx().unwrap()).unwrap();
        let mut fresh = session();
        fresh.set_level(1);
        read(&szx, &mut fresh, &saves(folder.path())).unwrap();
        assert_eq!(fresh.machine().peek(0x9000), 0x42);
        assert_eq!(fresh.level(), 1, "the level in use stays");
        assert!(!fresh.switched_on("no-harm"), "switches forgotten");
        mode(&mut fresh, 2);
        assert!(fresh.record().loaded, "the game it goes on into is marked");
    }

    #[test]
    fn a_plain_tape_s_quick_slot_is_under_its_name() {
        let data = Path::new("data");
        assert_eq!(
            Saves::of_tape("Jet Set.tap", "x", data).quick,
            data.join("tapes").join("Jet Set").join(QUICK)
        );
    }
}
