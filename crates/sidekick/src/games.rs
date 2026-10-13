//! Games: where their definitions and tapes are, and which tape a game takes.

use crate::{
    pad::Mapping,
    saves::{SavePoint, Saves},
};
use sidekick_core::{Machine, Model, Script, Session};
use sidekick_def::{
    tapes_in, Definition, Library, LoadError, Machine as Spectrum, Package, Tape, TapeFormat,
};
use sidekick_ui::{Refusal, TapePrompt};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// A game ready to start: its definition and its folder in the data folder.
#[derive(Clone, Debug)]
pub(crate) struct Game {
    /// Its definition.
    pub package: Package,
    /// `<data>/<id>/`, where its tape and the rest of what's kept for it live.
    pub folder: PathBuf,
}

/// The data folder: the system's, or `ZX_SIDEKICK_DATA`.
pub(crate) fn data_folder() -> PathBuf {
    std::env::var_os("ZX_SIDEKICK_DATA").map_or_else(
        || {
            directories::ProjectDirs::from("", "", "ZX Sidekick").map_or_else(
                || PathBuf::from("zx-sidekick-data"),
                |d| d.data_dir().to_owned(),
            )
        },
        PathBuf::from,
    )
}

/// The definitions in `games/` beside the program, then the data folder's `games/`, then
/// `extra`: later folders win.
pub(crate) fn library(data: &Path, extra: &[PathBuf]) -> (Library, Vec<LoadError>) {
    let mut folders = Vec::new();
    if let Some(beside) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("games")))
    {
        folders.push(beside);
    }
    folders.push(data.join("games"));
    folders.extend(extra.iter().cloned());
    Library::search(&folders)
}

impl Game {
    /// The game a package defines, keeping its things under `data`.
    pub(crate) fn new(package: Package, data: &Path) -> Self {
        let folder = data.join(&package.definition.game.id);
        Self { package, folder }
    }

    const fn definition(&self) -> &Definition {
        &self.package.definition
    }

    fn supports(&self, tape: &Tape) -> bool {
        self.definition().media.iter().any(|m| {
            m.sha1
                .as_deref()
                .is_some_and(|sha1| sha1.eq_ignore_ascii_case(&tape.sha1))
        })
    }

    const fn kept_path(format: TapeFormat) -> &'static str {
        match format {
            TapeFormat::Tap => "tape.tap",
            TapeFormat::Tzx => "tape.tzx",
        }
    }

    /// The tape kept from an earlier start, if it's there and still one of the game's dumps.
    pub(crate) fn kept_tape(&self) -> Option<Tape> {
        [TapeFormat::Tap, TapeFormat::Tzx]
            .into_iter()
            .find_map(|format| {
                let path = self.folder.join(Self::kept_path(format));
                let tape = tapes_in(&path).ok()?.into_iter().next()?;
                self.supports(&tape).then_some(tape)
            })
    }

    /// Takes a file offered as the tape (a tape, or a zip holding tapes): the first of its tapes
    /// that is one of the game's dumps is kept, for next time, and returned.
    ///
    /// # Errors
    /// A refusal to show: no tape in the file, a different dump, or the tape couldn't be kept.
    pub(crate) fn offer(&self, file: &Path) -> Result<Tape, Refusal> {
        let name = file
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let title = &self.definition().game.title;
        let tapes = tapes_in(file).map_err(|_| Refusal {
            heading: format!("{name} holds no {title} tape"),
            detail: "Locate the .tap or .tzx, or the .zip it was downloaded in.".to_owned(),
        })?;
        let Some(tape) = tapes.into_iter().find(|t| self.supports(t)) else {
            let media = self.definition().media.first();
            let wanted = media.map_or_else(|| format!("its own {title} tape"), |m| m.name.clone());
            let sha1 = media
                .and_then(|m| m.sha1.as_deref())
                .map_or_else(String::new, |s| {
                    format!(" (SHA-1 {}…)", &s[..s.len().min(8)])
                });
            return Err(Refusal {
                heading: format!("That isn't the {title} tape this version needs"),
                detail: format!("The tape in {name} is a different dump. This version works only with {wanted}{sha1}."),
            });
        };
        fs::create_dir_all(&self.folder)
            .and_then(|()| fs::write(self.folder.join(Self::kept_path(tape.format)), &tape.bytes))
            .map_err(|e| Refusal {
                heading: format!("The {title} tape couldn't be kept"),
                detail: format!("{}: {e}", self.folder.display()),
            })?;
        Ok(tape)
    }

    /// What the tape prompt says for this game.
    pub(crate) fn prompt(&self) -> TapePrompt {
        let def = self.definition();
        let game = &def.game;
        let media = def.media.first();
        let files = media.map_or(&[][..], |m| m.files.as_slice());
        let ending = |ext: &str| {
            files
                .iter()
                .find(|f| f.to_ascii_lowercase().ends_with(ext))
                .cloned()
        };
        let mut credit = game.title.clone();
        if let Some(year) = game.year {
            let _ = write!(credit, " © {year}");
        }
        let makers: Vec<&str> = game
            .authors
            .iter()
            .map(String::as_str)
            .chain(game.publisher.as_deref())
            .collect();
        if !makers.is_empty() {
            let _ = write!(credit, " {}", makers.join(" / "));
        }
        credit.push_str(". Not affiliated.");
        TapePrompt {
            pad: None,
            pressed: None,
            title: game.title.clone(),
            credit,
            tape_file: ending(".tap")
                .or_else(|| ending(".tzx"))
                .unwrap_or_else(|| format!("{}.tap", game.id)),
            download_file: ending(".zip"),
            download: media.and_then(|m| {
                let url = m.download.as_deref()?;
                let page = m
                    .download_label
                    .clone()
                    .unwrap_or_else(|| "the download page".to_owned());
                let shown = url
                    .trim_start_matches("https://")
                    .trim_start_matches("http://");
                Some((page, shown.to_owned()))
            }),
            refusal: None,
        }
    }

    /// The download page's address, to open.
    pub(crate) fn download_url(&self) -> Option<String> {
        self.definition()
            .media
            .first()
            .and_then(|m| m.download.clone())
    }

    /// The game as the library lists it: its title, its year and publisher, whether its tape
    /// is kept.
    pub(crate) fn entry(&self) -> sidekick_ui::LibraryEntry {
        let game = &self.definition().game;
        let line: Vec<String> = game
            .year
            .map(|y| y.to_string())
            .into_iter()
            .chain(game.publisher.clone())
            .collect();
        sidekick_ui::LibraryEntry {
            title: game.title.clone(),
            line: line.join(" · "),
            ready: self.kept_tape().is_some(),
        }
    }

    /// The game's title.
    pub(crate) fn title(&self) -> String {
        self.definition().game.title.clone()
    }

    /// What a pad does in the game.
    pub(crate) fn mapping(&self) -> Mapping {
        Mapping::new(Some(self.definition()))
    }

    /// A machine of the game's model, with the tape going in.
    ///
    /// # Errors
    /// If the tape can't be played.
    pub(crate) fn machine(&self, tape: &Tape) -> Result<Machine, String> {
        start(model(self.definition().game.machine), tape)
    }

    /// The game played from a tape: a machine of its model with the tape going in, its values
    /// read and its state decided each frame.
    ///
    /// # Errors
    /// If the tape can't be played.
    pub(crate) fn session(&self, tape: &Tape) -> Result<Session, String> {
        let mut session = Session::new(self.machine(tape)?, Some(self.definition().clone()));
        if let Some(script) = script(&self.package) {
            session.set_script(script);
        }
        session.set_kept(self.kept());
        Ok(session)
    }

    /// The game's kept lists, as last saved: a file a list in its folder, an entry a line.
    pub(crate) fn kept(&self) -> BTreeMap<String, Vec<String>> {
        self.definition()
            .persist
            .iter()
            .map(|(name, persist)| {
                let entries = fs::read_to_string(self.folder.join(&persist.file))
                    .map(|text| {
                        text.lines()
                            .map(str::trim)
                            .filter(|l| !l.is_empty())
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                (name.clone(), entries)
            })
            .collect()
    }
}

/// Writes the kept lists that changed to their files in a game's folder.
///
/// # Errors
/// If the folder or a file can't be written.
pub(crate) fn save_kept(
    folder: &Path,
    definition: &Definition,
    kept: &BTreeMap<String, Vec<String>>,
    changed: &BTreeSet<String>,
) -> std::io::Result<()> {
    fs::create_dir_all(folder)?;
    for name in changed {
        let (Some(persist), Some(entries)) = (definition.persist.get(name), kept.get(name)) else {
            continue;
        };
        let mut text = entries.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        fs::write(folder.join(&persist.file), text)?;
    }
    Ok(())
}

/// A game ready to play: its session, title and pad mapping, the folder its kept lists
/// are saved in (none for a tape played plainly), and what its saves are of.
pub(crate) struct Playing {
    pub(crate) session: Session,
    pub(crate) title: String,
    pub(crate) mapping: Mapping,
    pub(crate) folder: Option<PathBuf>,
    pub(crate) saves: Saves,
    /// Whether the quit slot is used: a game kept there offered, and the game kept there on
    /// quitting. Not when it starts from `--load`, which leaves the player's slot alone.
    pub(crate) quit_slot: bool,
    /// Whether kept lists are saved: not for a crafted situation, whose entries are made up.
    pub(crate) keep_lists: bool,
    /// Where F9 goes back to: the quick slot, or the save or situation `--load` started from.
    pub(crate) save_point: SavePoint,
    /// Started with `--paused`: the game is paused once it's in play.
    pub(crate) paused: bool,
    /// Where the game's definition was read from, to read again when it changes; none for a
    /// tape played plainly.
    pub(crate) source: Option<PathBuf>,
}

impl Game {
    /// The game played from a tape.
    ///
    /// # Errors
    /// If the tape can't be played.
    pub(crate) fn playing(&self, tape: &Tape) -> Result<Playing, String> {
        Ok(Playing {
            session: self.session(tape)?,
            title: self.title(),
            mapping: self.mapping(),
            folder: Some(self.folder.clone()),
            saves: Saves::of_game(&self.definition().game.id, &tape.sha1, &self.folder),
            quit_slot: true,
            keep_lists: true,
            save_point: SavePoint::Quick,
            paused: false,
            source: Some(self.package.path.clone()),
        })
    }
}

/// When a definition's files last changed: the file itself, or the newest file in its folder
/// and the folders in that (its scripts, its tests); `None` when that can't be read.
pub(crate) fn changed(path: &Path) -> Option<SystemTime> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return meta.modified().ok();
    }
    let mut newest = meta.modified().ok();
    for entry in fs::read_dir(path).ok()?.flatten() {
        let inner = entry.path();
        let stamp = if inner.is_dir() {
            fs::read_dir(&inner)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| e.metadata().ok()?.modified().ok())
                .max()
        } else {
            entry.metadata().ok()?.modified().ok()
        };
        newest = newest.max(stamp);
    }
    newest
}

/// A package's script, compiled, or why it couldn't be; `None` without one.
pub(crate) fn script(package: &Package) -> Option<Result<Script, String>> {
    let file = &package.definition.script.as_ref()?.file;
    let compiled = package
        .read(file)
        .map_err(|e| format!("{file}: {e}"))
        .and_then(|bytes| String::from_utf8(bytes).map_err(|_| format!("{file}: not text (UTF-8)")))
        .and_then(|source| Script::compile(&source).map_err(|e| format!("{file}: {e}")));
    Some(compiled)
}

/// The emulated model for a definition's machine.
pub(crate) const fn model(machine: Spectrum) -> Model {
    match machine {
        Spectrum::Spectrum48K => Model::Sinclair48K,
        Spectrum::Spectrum128K => Model::Sinclair128K,
    }
}

/// A machine of `model` with `tape` going in, as `LOAD ""` loads it.
///
/// # Errors
/// If the tape is a `.tzx` (not read yet) or a broken `.tap`.
pub(crate) fn start(model: Model, tape: &Tape) -> Result<Machine, String> {
    if tape.format != TapeFormat::Tap {
        return Err(format!("{}: only .tap tapes can be played yet", tape.name));
    }
    let mut machine = Machine::new(model).map_err(|e| e.to_string())?;
    machine
        .load_tap(&tape.bytes)
        .map_err(|e| format!("{}: {e}", tape.name))?;
    Ok(machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_folder_changed_when_any_of_its_files_did() {
        use std::time::{Duration, UNIX_EPOCH};
        let dir = tempfile::TempDir::new().unwrap();
        let at = |secs: u64| UNIX_EPOCH + Duration::from_secs(secs);
        let stamp = |path: &Path, secs: u64| {
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(at(secs))
                .unwrap();
        };
        let game = dir.path().join("game.toml");
        fs::write(&game, "schema = 1").unwrap();
        fs::create_dir(dir.path().join("scripts")).unwrap();
        let script = dir.path().join("scripts/game.rhai");
        fs::write(&script, "fn on_frame() {}").unwrap();
        stamp(&game, 1_000);
        stamp(&script, 2_000);
        let folder = changed(dir.path()).unwrap();
        assert!(folder >= at(2_000), "the script, a folder down");
        stamp(&script, 3_000_000_000);
        assert_eq!(changed(dir.path()), Some(at(3_000_000_000)));
        assert_eq!(changed(&game), Some(at(1_000)), "a file alone");
        assert_eq!(changed(&dir.path().join("missing.toml")), None);
    }
    use sidekick_def::Definition;
    use std::io::Write as _;

    /// The SHA-1 of the three bytes "abc".
    const ABC_SHA1: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";

    fn game(data: &Path) -> Game {
        let text = format!(
            r#"schema = 1
[game]
id = "quake"
title = "Quake Example"
year = 1985
authors = ["A. Author"]
publisher = "Example Soft"
machine = "48k"
[[media]]
name = "the original Example Soft release"
sha1 = "{ABC_SHA1}"
files = ["QUAKE.TAP", "Quake.tap.zip"]
download = "https://example.org/item/1"
download_label = "Example Archive"
"#
        );
        let definition = Definition::from_toml(&text).unwrap();
        let dir = data.join("definition");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("game.toml"), text).unwrap();
        assert_eq!(definition.game.id, "quake");
        Game::new(Package::load(&dir).unwrap(), data)
    }

    #[test]
    fn the_prompt_says_what_the_definition_says() {
        let data = tempfile::TempDir::new().unwrap();
        let prompt = game(data.path()).prompt();
        assert_eq!(prompt.title, "Quake Example");
        assert_eq!(
            prompt.credit,
            "Quake Example © 1985 A. Author / Example Soft. Not affiliated."
        );
        assert_eq!(prompt.tape_file, "QUAKE.TAP");
        assert_eq!(prompt.download_file.as_deref(), Some("Quake.tap.zip"));
        assert_eq!(
            prompt.download,
            Some((
                "Example Archive".to_owned(),
                "example.org/item/1".to_owned()
            ))
        );
    }

    #[test]
    fn the_right_tape_is_kept_and_found_next_time() {
        let data = tempfile::TempDir::new().unwrap();
        let game = game(data.path());
        assert!(game.kept_tape().is_none(), "nothing kept yet");
        let zip = data.path().join("download.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        writer
            .start_file("QUAKE.TAP", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"abc").unwrap();
        writer.finish().unwrap();

        let tape = game.offer(&zip).expect("the right dump");
        assert_eq!(tape.sha1, ABC_SHA1);
        assert_eq!(game.kept_tape().map(|t| t.bytes), Some(b"abc".to_vec()));
        assert!(data.path().join("quake/tape.tap").exists());
    }

    #[test]
    fn another_dump_is_refused_by_name() {
        let data = tempfile::TempDir::new().unwrap();
        let game = game(data.path());
        let other = data.path().join("other.tap");
        fs::write(&other, b"not the game").unwrap();
        let refusal = game.offer(&other).expect_err("a different dump");
        assert_eq!(
            refusal.heading,
            "That isn't the Quake Example tape this version needs"
        );
        assert_eq!(
            refusal.detail,
            "The tape in other.tap is a different dump. This version works only with the original Example Soft release (SHA-1 a9993e36…)."
        );
        assert!(game.kept_tape().is_none(), "nothing kept");

        let notes = data.path().join("notes.txt");
        fs::write(&notes, b"hello").unwrap();
        let refusal = game.offer(&notes).expect_err("not a tape");
        assert_eq!(refusal.heading, "notes.txt holds no Quake Example tape");
    }

    #[test]
    fn a_kept_tape_that_isn_t_the_dump_any_more_is_ignored() {
        let data = tempfile::TempDir::new().unwrap();
        let game = game(data.path());
        fs::create_dir_all(data.path().join("quake")).unwrap();
        fs::write(data.path().join("quake/tape.tap"), b"changed").unwrap();
        assert!(game.kept_tape().is_none());
    }

    #[test]
    fn kept_lists_are_saved_a_line_an_entry_and_read_back() {
        let data = tempfile::TempDir::new().unwrap();
        let mut game = game(data.path());
        game.package.definition.persist.insert(
            "codes".to_owned(),
            sidekick_def::Persist {
                label: "teleport codes".to_owned(),
                file: "teleporter-codes.txt".to_owned(),
                add: None,
                most: None,
            },
        );
        assert_eq!(
            game.kept()["codes"],
            Vec::<String>::new(),
            "nothing kept yet"
        );
        let kept = BTreeMap::from([(
            "codes".to_owned(),
            vec!["TOPAZ".to_owned(), "LUMEN".to_owned()],
        )]);
        let changed = BTreeSet::from(["codes".to_owned()]);
        save_kept(&game.folder, game.definition(), &kept, &changed).unwrap();
        assert_eq!(
            fs::read_to_string(game.folder.join("teleporter-codes.txt")).unwrap(),
            "TOPAZ\nLUMEN\n"
        );
        assert_eq!(game.kept()["codes"], ["TOPAZ", "LUMEN"]);
        let none = BTreeMap::from([("codes".to_owned(), Vec::new())]);
        save_kept(&game.folder, game.definition(), &none, &changed).unwrap();
        assert!(
            game.kept()["codes"].is_empty(),
            "forgotten, and the file empty"
        );
    }
}
