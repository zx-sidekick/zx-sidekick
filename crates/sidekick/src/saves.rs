//! Saves: the whole machine and the program's own part of the game, in a zip; the quick slot;
//! and the checks before one is loaded.

use serde::{Deserialize, Serialize};
use sidekick_core::{Error, Progress, Record, Session};
use sidekick_def::Role;
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
/// The file a game under way is kept in on quitting.
const QUIT: &str = "quit.sav";
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
    /// The slot a game under way is kept in on quitting, beside the quick slot.
    pub(crate) fn quit(&self) -> PathBuf {
        self.quick.with_file_name(QUIT)
    }

    /// A defined game's: its quick slot in its data folder.
    pub(crate) fn of_game(id: &str, dump: &str, folder: &Path) -> Self {
        Self {
            game: Some(id.to_owned()),
            dump: dump.to_owned(),
            quick: folder.join(QUICK),
        }
    }

    /// A tape played plainly: its quick slot in the data folder's `tapes/`, in a folder of the
    /// tape's name and the start of its SHA-1, so two tapes of one name keep apart.
    pub(crate) fn of_tape(name: &str, dump: &str, data: &Path) -> Self {
        let stem = Path::new(name)
            .file_stem()
            .map_or_else(|| name.to_owned(), |s| s.to_string_lossy().into_owned());
        let folder = format!("{stem}-{}", dump.get(..8).unwrap_or(dump));
        Self {
            game: None,
            dump: dump.to_owned(),
            quick: data.join("tapes").join(folder).join(QUICK),
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

/// Why the game wasn't saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Unsaved {
    /// The tape is still loading, and a save doesn't hold the tape.
    TapeLoading,
    /// The machine or the file failed, and how.
    Failed(String),
}

impl Unsaved {
    /// What the notice says.
    pub(crate) const fn words(&self) -> &'static str {
        match self {
            Self::TapeLoading => "Not while the tape loads",
            Self::Failed(_) => "The game couldn't be saved",
        }
    }
}

/// Why a save wasn't loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// Nothing in the slot.
    NoSave,
    /// Not a save, or one this program can't read.
    NotASave,
    /// A save whose snapshot the machine couldn't read.
    Damaged,
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
            Self::Damaged => "That save is damaged",
            Self::OtherGame => "That save is of another game",
            Self::OtherDump => "That save is from another tape",
            Self::OtherMachine => "That save is for another Spectrum",
        }
    }
}

/// Where F9 goes back to: the session's last save point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SavePoint {
    /// The quick slot, which F5 writes.
    Quick,
    /// A save the session started from (`--load`) or opened: read again each time, never
    /// written.
    File(PathBuf),
    /// A crafted situation being set up: held once its first frame ends.
    Starting,
    /// The moment a crafted situation had been set up, held as a save.
    Held(Vec<u8>),
}

impl SavePoint {
    /// Whether there's a save to go back to.
    pub(crate) fn exists(&self, saves: &Saves) -> bool {
        match self {
            Self::Quick => saves.quick.exists(),
            Self::File(path) => path.exists(),
            Self::Starting => false,
            Self::Held(_) => true,
        }
    }

    /// Goes back to it: the session as it was there, its game marked loaded.
    ///
    /// # Errors
    /// Why it wasn't loaded; the session is left as it was.
    pub(crate) fn load(&self, session: &mut Session, saves: &Saves) -> Result<(), Refused> {
        match self {
            Self::Quick => read(&saves.quick, session, saves),
            Self::File(path) => read(path, session, saves),
            Self::Starting => Err(Refused::NoSave),
            Self::Held(bytes) => from_bytes(bytes, session, saves, true),
        }
    }
}

/// Saves the session to `path`, between frames: the machine whole, and the guidance level,
/// switches, record and progress. Written beside it first, then moved over it, so a save cut
/// short never replaces a good one.
///
/// # Errors
/// While the tape loads, or if the machine can't be saved or the file can't be written.
pub(crate) fn write(session: &mut Session, saves: &Saves, path: &Path) -> Result<(), Unsaved> {
    let bytes = to_bytes(session, saves)?;
    write_whole(path, &bytes).map_err(|e| Unsaved::Failed(format!("{}: {e}", path.display())))
}

/// The session as a save's bytes, between frames.
///
/// # Errors
/// While the tape loads, or if the machine can't be saved.
pub(crate) fn to_bytes(session: &mut Session, saves: &Saves) -> Result<Vec<u8>, Unsaved> {
    if session.machine().tape_loading() {
        return Err(Unsaved::TapeLoading);
    }
    let szx = session
        .machine_mut()
        .save_szx()
        .map_err(|e| Unsaved::Failed(e.to_string()))?;
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
    let text = toml::to_string(&state).map_err(|e| Unsaved::Failed(e.to_string()))?;
    let zip = || -> zip::result::ZipResult<Vec<u8>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(MACHINE, SimpleFileOptions::default())?;
        zip.write_all(&szx)?;
        zip.start_file(STATE, SimpleFileOptions::default())?;
        zip.write_all(text.as_bytes())?;
        Ok(zip.finish()?.into_inner())
    };
    zip().map_err(|e| Unsaved::Failed(e.to_string()))
}

/// Loads a save into the session, its game marked loaded: a `.sav` whole, once it's found to
/// be this game's from this dump; a plain SZX snapshot as the machine alone.
///
/// # Errors
/// Why it wasn't loaded; the session is left as it was.
pub(crate) fn read(path: &Path, session: &mut Session, saves: &Saves) -> Result<(), Refused> {
    let bytes = fs::read(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => Refused::NoSave,
        _ => Refused::NotASave,
    })?;
    from_bytes(&bytes, session, saves, true)
}

/// Takes the save kept on quitting, if there is one: its bytes, the file removed, so it's
/// offered once whatever comes of it.
pub(crate) fn take_quit(saves: &Saves) -> Option<Vec<u8>> {
    let quit = saves.quit();
    let bytes = fs::read(&quit).ok()?;
    // One that can't be removed would be offered again and again: offered not at all.
    if let Err(e) = fs::remove_file(&quit) {
        eprintln!("{}: couldn't be removed: {e}", quit.display());
        return None;
    }
    Some(bytes)
}

/// Puts back the game kept on quitting, offered and not answered.
///
/// # Errors
/// If the file can't be written.
pub(crate) fn put_back_quit(saves: &Saves, bytes: &[u8]) -> Result<(), Unsaved> {
    let quit = saves.quit();
    write_whole(&quit, bytes).map_err(|e| Unsaved::Failed(format!("{}: {e}", quit.display())))
}

/// Writes a file beside `path` first, then moves it over, so one cut short never replaces a
/// good one.
fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let partial = path.with_extension("part");
    path.parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&partial, bytes))
        .and_then(|()| fs::rename(&partial, path))
}

/// Goes on with the game kept on quitting: the same game, so not marked loaded.
///
/// # Errors
/// Why it wasn't loaded; the session is left as it was.
pub(crate) fn continue_from(
    bytes: &[u8],
    session: &mut Session,
    saves: &Saves,
) -> Result<(), Refused> {
    from_bytes(bytes, session, saves, false)
}

/// Keeps the game on quitting, when one is under way, in its own slot; otherwise leaves no
/// slot behind. A game is under way between its start and game over; a game without a
/// `play` state, once its tape has loaded.
///
/// # Errors
/// If a game under way couldn't be saved.
pub(crate) fn keep_on_quit(session: &mut Session, saves: &Saves) -> Result<(), Unsaved> {
    let definition = session.definition();
    let has_play = definition.is_some_and(|d| d.state_for(Role::Play).is_some());
    let progress = session.progress();
    let over = definition.is_some_and(|d| d.is(progress.last_known.as_deref(), Role::GameOver));
    let under_way = if has_play {
        progress.in_game && !over
    } else {
        session.machine().tape_loaded()
    };
    let quit = saves.quit();
    if under_way {
        return write(session, saves, &quit);
    }
    match fs::remove_file(&quit) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => {
            Err(Unsaved::Failed(format!("{}: {e}", quit.display())))
        }
        _ => Ok(()),
    }
}

fn from_bytes(
    bytes: &[u8],
    session: &mut Session,
    saves: &Saves,
    mark: bool,
) -> Result<(), Refused> {
    if bytes.starts_with(SZX_MAGIC) {
        load_machine(session, bytes)?;
        session.loaded_snapshot();
        return Ok(());
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| Refused::NotASave)?;
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
    load_machine(session, &szx)?;
    session.restore(
        state.level,
        &state.switches,
        Record {
            level: state.record.level,
            switches: state.record.switches,
            loaded: state.record.loaded || mark,
        },
        Progress {
            last_known: state.progress.last_state,
            seen_title: state.progress.seen_title,
            in_game: state.progress.in_game,
        },
    );
    Ok(())
}

/// Loads a snapshot into the machine, which refuses a damaged one whole: it's left as it was.
fn load_machine(session: &mut Session, szx: &[u8]) -> Result<(), Refused> {
    session.machine_mut().load_szx(szx).map_err(|e| match e {
        Error::OtherModel | Error::UnknownModel => Refused::OtherMachine,
        _ => Refused::Damaged,
    })
}

/// A file in a save's zip, whole.
fn entry(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>, Refused> {
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
role = "title"
when = { value = "mode", equals = 1 }
[[states]]
name = "play"
role = "play"
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
    fn f9_goes_back_to_the_save_point_and_never_writes_a_file_it_came_from() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let started = folder.path().join("started.sav");
        let mut s = played();
        write(&mut s, &saves, &started).unwrap();
        let bytes = fs::read(&started).unwrap();

        // Played on from the file, then back to it.
        s.machine_mut().write(0x9000, 0x99);
        let point = SavePoint::File(started.clone());
        assert!(point.exists(&saves));
        point.load(&mut s, &saves).unwrap();
        assert_eq!(s.machine().peek(0x9000), 0x42, "back where it started");
        assert!(s.record().loaded, "marked loaded");
        assert_eq!(fs::read(&started).unwrap(), bytes, "never written");

        // A situation's start, held.
        let held = SavePoint::Held(to_bytes(&mut s, &saves).unwrap());
        s.machine_mut().write(0x9000, 0x77);
        assert!(held.exists(&saves));
        held.load(&mut s, &saves).unwrap();
        assert_eq!(s.machine().peek(0x9000), 0x42);

        // The quick slot, once an F5 has written it.
        assert!(!SavePoint::Quick.exists(&saves), "nothing saved yet");
        s.machine_mut().write(0x9000, 0x55);
        write(&mut s, &saves, &saves.quick).unwrap();
        s.machine_mut().write(0x9000, 0x66);
        SavePoint::Quick.load(&mut s, &saves).unwrap();
        assert_eq!(s.machine().peek(0x9000), 0x55);
        assert_eq!(fs::read(&started).unwrap(), bytes, "still never written");
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
    fn a_game_under_way_is_kept_on_quitting_and_offered_once() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let mut s = played();
        keep_on_quit(&mut s, &saves).unwrap();
        assert!(saves.quit().exists(), "kept in play");
        assert_eq!(saves.quit(), folder.path().join("quit.sav"));

        let bytes = take_quit(&saves).expect("offered");
        assert!(!saves.quit().exists(), "removed as it's offered");
        assert_eq!(take_quit(&saves), None, "once");
        put_back_quit(&saves, &bytes).unwrap();
        assert_eq!(
            take_quit(&saves).as_deref(),
            Some(&bytes[..]),
            "put back, unanswered"
        );
        let mut fresh = session();
        continue_from(&bytes, &mut fresh, &saves).unwrap();
        assert_eq!(fresh.machine().peek(0x9000), 0x42);
        assert!(fresh.switched_on("no-harm"));
        assert!(!fresh.record().loaded, "the same game goes on, not marked");
    }

    #[test]
    fn nothing_is_kept_at_the_title_or_game_over() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let mut s = played();
        keep_on_quit(&mut s, &saves).unwrap();
        mode(&mut s, 1);
        keep_on_quit(&mut s, &saves).unwrap();
        assert!(
            !saves.quit().exists(),
            "at the title: none, and the old one gone"
        );
        let mut fresh = session();
        mode(&mut fresh, 0);
        keep_on_quit(&mut fresh, &saves).unwrap();
        assert!(!saves.quit().exists(), "before any game");
    }

    #[test]
    fn a_plain_tape_is_kept_once_it_has_loaded() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = Saves::of_tape("plain.tap", "ABC123", folder.path());
        let header = [
            19, 0, 0, 3, b'x', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', 1, 0, 0, 0x80,
            0, 0, 0,
        ];
        let data = [3, 0, 0xFF, 0, 0xFF];
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        machine
            .load_tap(&[&header[..], &data[..]].concat())
            .expect("the tape goes in");
        let mut s = Session::new(machine, None);
        keep_on_quit(&mut s, &saves).unwrap();
        assert!(!saves.quit().exists(), "not while it loads");
        s.run_frame().expect("a frame");
        keep_on_quit(&mut s, &saves).unwrap();
        assert!(saves.quit().exists(), "once it has");
    }

    #[test]
    fn game_over_is_the_state_with_its_role_whatever_its_name() {
        let text = DEFINITION.replace(
            "[[cheats]]",
            "[[states]]\nname = \"wrecked\"\nrole = \"game_over\"\nwhen = { value = \"mode\", equals = 4 }\n\n[[cheats]]",
        );
        let definition = Definition::from_toml(&text).expect("the test definition parses");
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        for _ in 0..BOOT_FRAMES {
            machine.run_frame().expect("a frame");
        }
        let mut s = Session::new(machine, Some(definition));
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        mode(&mut s, 1);
        mode(&mut s, 2);
        keep_on_quit(&mut s, &saves).unwrap();
        assert!(saves.quit().exists(), "kept in play");
        mode(&mut s, 4);
        keep_on_quit(&mut s, &saves).unwrap();
        assert!(
            !saves.quit().exists(),
            "the state with role game_over: nothing kept"
        );
    }

    #[test]
    fn a_patch_a_switch_gained_since_the_save_is_applied_on_loading() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let mut s = played();
        s.machine_mut().write(0x8020, 0x22);
        write(&mut s, &saves, &saves.quick).unwrap();

        // The same switch, given a second patch since.
        let text = DEFINITION.replace(
            r#"patch = [{ addr = 0x8010, bytes = "00", expect = "CD" }]"#,
            r#"patch = [{ addr = 0x8010, bytes = "00", expect = "CD" }, { addr = 0x8020, bytes = "11", expect = "22" }]"#,
        );
        assert_ne!(text, DEFINITION, "the definition changed");
        let definition = Definition::from_toml(&text).expect("the test definition parses");
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        for _ in 0..BOOT_FRAMES {
            machine.run_frame().expect("a frame");
        }
        let mut later = Session::new(machine, Some(definition));
        read(&saves.quick, &mut later, &saves).unwrap();
        assert!(later.switched_on("no-harm"));
        assert_eq!(later.machine().peek(0x8010), 0x00, "the saved patch stays");
        assert_eq!(later.machine().peek(0x8020), 0x11, "the new patch goes on");
    }

    #[test]
    fn a_plain_tape_s_quick_slot_is_under_its_name_and_dump() {
        let data = Path::new("data");
        assert_eq!(
            Saves::of_tape("Jet Set.tap", "0123456789abcdef", data).quick,
            data.join("tapes").join("Jet Set-01234567").join(QUICK)
        );
    }

    #[test]
    fn a_damaged_snapshot_leaves_the_machine_as_it_was() {
        let folder = tempfile::TempDir::new().unwrap();
        let mut s = played();
        let mut szx = s.machine_mut().save_szx().unwrap();
        szx.truncate(szx.len() - 100);
        let damaged = folder.path().join("damaged.szx");
        fs::write(&damaged, &szx).unwrap();
        let mut fresh = session();
        fresh.machine_mut().write(0x9000, 0x17);
        let pc = fresh.machine().registers().get_pc();
        assert_eq!(
            read(&damaged, &mut fresh, &saves(folder.path())),
            Err(Refused::Damaged)
        );
        assert_eq!(fresh.machine().peek(0x9000), 0x17);
        assert_eq!(fresh.machine().registers().get_pc(), pc);
        assert!(!fresh.record().loaded);
    }

    #[test]
    fn a_hold_switch_holds_the_loaded_game_s_value_not_the_one_before() {
        let text = format!(
            "{DEFINITION}\n[values.lives]\naddr = 0x8020\ntype = \"u8\"\nwritable = true\n\n[[cheats]]\nid = \"lives\"\nlabel = \"Endless lives\"\nhold = \"lives\"\n"
        );
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let definition = Definition::from_toml(&text).expect("the definition parses");
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        for _ in 0..BOOT_FRAMES {
            machine.run_frame().expect("a frame");
        }
        let mut s = Session::new(machine, Some(definition));
        mode(&mut s, 2);
        s.machine_mut().write(0x8020, 3);
        s.switch("lives", true).expect("it goes on");
        mode(&mut s, 2);
        write(&mut s, &saves, &saves.quick).unwrap();
        s.machine_mut().write(0x8020, 4);
        mode(&mut s, 2);
        read(&saves.quick, &mut s, &saves).unwrap();
        mode(&mut s, 2);
        assert_eq!(s.machine().peek(0x8020), 3, "the saved game's lives");
    }

    #[test]
    fn a_save_isn_t_taken_while_the_tape_loads() {
        let folder = tempfile::TempDir::new().unwrap();
        let saves = saves(folder.path());
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        // A header block and a data block, of 1 byte, neither loaded yet.
        let header = [
            19, 0, 0, 3, b'x', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', 1, 0, 0, 0x80,
            0, 0, 0,
        ];
        let data = [3, 0, 0xFF, 0, 0xFF];
        machine
            .load_tap(&[&header[..], &data[..]].concat())
            .expect("the tape goes in");
        let mut s = Session::new(machine, None);
        assert_eq!(
            write(&mut s, &saves, &saves.quick),
            Err(Unsaved::TapeLoading)
        );
        assert!(!saves.quick.exists());
    }
}
