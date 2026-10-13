//! `sidekick`: ZX Sidekick's program. `sidekick [GAME-ID | TAPE]` plays in a window; the other
//! commands are for definition authors.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command line's results and errors are printed"
)]

mod compose;
mod entry;
mod games;
mod headless;
mod keys;
mod menu;
mod pad;
mod panel;
mod picture;
mod press;
mod saves;
mod scores;
mod situations;
mod window;

use clap::{Parser, Subcommand};
use games::{Game, Playing};
use pad::Mapping;
use saves::{SavePoint, Saves};
use sidekick_core::{Machine, Model, Session};
use sidekick_def::{tapes_in, Definition, Identified, Library, Package, Tape, TapeFormat};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use window::Start;

/// Play original ZX Spectrum games from your own copy, with a pad, training switches and
/// guidance.
#[derive(Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
struct Cli {
    /// A game to play, by its id, or a tape to play (a .tap, or the .zip it came in). With
    /// neither, the only game installed.
    target: Option<String>,
    /// A folder of definitions to add to those installed. Repeat for more.
    #[arg(long = "games", value_name = "DIR")]
    games: Vec<PathBuf>,
    /// Start from a save (.sav, .szx) or a crafted situation (.toml).
    #[arg(long, value_name = "FILE")]
    load: Option<PathBuf>,
    /// Start the game paused once it's in play (or at the moment --load sets up), so a test
    /// waits until you're looking.
    #[arg(long)]
    paused: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Checks a definition, and optionally that a tape is one of its dumps.
    ///
    /// Prints each problem as `path: message`, and each warning as `path: warning: message`.
    /// Exits 0 with no problems (warnings or not), 1 with problems, 2 when something couldn't
    /// be read.
    Check {
        /// The definition: a .toml or .json file, a .zip, or a folder.
        definition: PathBuf,
        /// A tape (.tap, .tzx) or a .zip holding tapes.
        tape: Option<PathBuf>,
    },
    /// Runs a tape without a window, and writes its picture as a PNG.
    Run {
        /// A tape to run: a .tap, or the .zip it came in. A tape a definition knows runs as
        /// that game.
        tape: PathBuf,
        /// Run it as the game this definition describes, whether or not it's one of its dumps.
        #[arg(long, value_name = "DEF")]
        definition: Option<PathBuf>,
        /// Also look for the tape's definition in this folder. Repeat for more.
        #[arg(long = "games", value_name = "DIR")]
        games: Vec<PathBuf>,
        /// Print the game's state and every named value after the last frame.
        #[arg(long)]
        values: bool,
        /// Frames to run (50 a second).
        #[arg(long, default_value_t = 250)]
        frames: u32,
        /// A key to press at a frame, for five frames: NAME@FRAME, like SPACE@1500. Repeat
        /// for more.
        #[arg(long = "key", value_name = "NAME@FRAME")]
        keys: Vec<headless::KeyPress>,
        /// A training switch to turn on at a frame: ID@FRAME, like endless-lives@2000. Repeat
        /// for more.
        #[arg(long = "switch", value_name = "ID@FRAME")]
        switches: Vec<headless::SwitchAt>,
        /// Start from a save (.sav, .szx) or a crafted situation (.toml); the frames run after.
        #[arg(long, value_name = "FILE")]
        load: Option<PathBuf>,
        /// Where to write the picture after the last frame, as a PNG.
        #[arg(long, value_name = "FILE")]
        screenshot: Option<PathBuf>,
        /// How many screen pixels each Spectrum pixel is in the PNG.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=16))]
        scale: u32,
        /// Play at this guidance level, and write the whole window to the PNG: the picture
        /// with the panel beside it, as the player sees it.
        #[arg(long, value_name = "N")]
        level: Option<u8>,
        /// The window's size, in screen pixels, with --level.
        #[arg(long, value_name = "WIDTHxHEIGHT", default_value = "1368x768", value_parser = headless::size)]
        size: (u32, u32),
    },
    /// Converts a definition from TOML to JSON, or from JSON to TOML.
    Convert {
        /// A .toml or .json definition file.
        file: PathBuf,
        /// Where to write it (standard output when not given).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Turns a .pok file's trainers into [[cheats]], to paste into a definition.
    ImportPok {
        /// A .pok file, the POKE format Fuse and ZXDB use.
        file: PathBuf,
        /// Where to write them (standard output when not given).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Something couldn't be read or written: exit status 2.
const UNREADABLE: u8 = 2;
/// The definition has problems: exit status 1.
const PROBLEMS: u8 = 1;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match (cli.command, cli.target) {
        (Some(Command::Check { definition, tape }), _) => check(&definition, tape.as_deref()),
        (Some(Command::Convert { file, output }), _) => convert(&file, output.as_deref()),
        (Some(Command::ImportPok { file, output }), _) => import_pok(&file, output.as_deref()),
        (
            Some(Command::Run {
                tape,
                definition,
                games,
                values,
                frames,
                keys,
                switches,
                load,
                screenshot,
                scale,
                level,
                size,
            }),
            _,
        ) => {
            let run = run_session(&tape, definition.as_deref(), &games).and_then(
                |Playing {
                     mut session, saves, ..
                 }| {
                    if let Some(file) = &load {
                        situations::load(&mut session, &saves, file)?;
                    }
                    let shot = level.map_or(headless::Shot::Picture { scale }, |level| {
                        headless::Shot::Window { level, size }
                    });
                    headless::run(
                        &mut session,
                        frames,
                        &keys,
                        &switches,
                        screenshot.as_deref().map(|path| (path, shot)),
                    )?;
                    if values {
                        print!("{}", headless::values(&session));
                    }
                    Ok(())
                },
            );
            match run {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(UNREADABLE)
                }
            }
        }
        (None, target) => match open(target.as_deref(), &cli.games)
            .and_then(|start| loading(start, cli.load.as_deref()))
            .map(|start| pausing(start, cli.paused))
            .and_then(window::run)
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(UNREADABLE)
            }
        },
    }
}

/// What to open the window on: a tape file played, a game by its id, or the only game.
fn open(target: Option<&str>, extra: &[PathBuf]) -> Result<Start, String> {
    let data = games::data_folder();
    let (library, problems) = games::library(&data, extra);
    for problem in &problems {
        eprintln!("{problem}");
    }
    if let Some(file) = target.map(Path::new).filter(|p| p.exists()) {
        return Ok(Start::Play(Box::new(tape_session(file, &library, &data)?)));
    }
    let package = if let Some(id) = target {
        library
            .get(id)
            .ok_or_else(|| format!("{id}: no game with that id, and no such file"))?
    } else {
        let mut games: Vec<Game> = library
            .games()
            .map(|p| Game::new(p.clone(), &data))
            .collect();
        return match games.len() {
            0 => Err("No games are installed: give a tape (sidekick TAPE), or a folder of definitions (--games DIR)".to_owned()),
            1 => games.pop().map_or_else(|| Err(String::new()), start_game),
            _ => {
                games.sort_by_key(Game::title);
                Ok(Start::Library(games))
            }
        };
    };
    start_game(Game::new(package.clone(), &data))
}

/// The window's start with `--load` applied: a game started from the file, and no offer to
/// continue.
/// The window's start with `--paused` applied: the game to be paused once it's in play.
fn pausing(start: Start, paused: bool) -> Start {
    match start {
        Start::Play(mut playing) => {
            playing.paused = paused;
            Start::Play(playing)
        }
        other => other,
    }
}

fn loading(start: Start, load: Option<&Path>) -> Result<Start, String> {
    let Some(file) = load else {
        return Ok(start);
    };
    match start {
        Start::Play(mut playing) => {
            situations::load(&mut playing.session, &playing.saves, file)?;
            playing.quit_slot = false;
            playing.keep_lists = !is_situation(file);
            // F9 goes back to where the session started, until an F5: a save read again, or a
            // situation's moment held once its first frame ends. The file is never written.
            playing.save_point = if is_situation(file) {
                SavePoint::Starting
            } else {
                SavePoint::File(file.to_owned())
            };
            Ok(Start::Play(playing))
        }
        Start::Prompt(_) => Err(format!(
            "{}: the game's tape is needed first: give the tape, or start the game once to locate it",
            file.display()
        )),
        Start::Library(_) => Err(format!(
            "{}: name the game or the tape to load it into",
            file.display()
        )),
    }
}

/// Whether `--load`'s file is a crafted situation rather than a save.
fn is_situation(file: &Path) -> bool {
    file.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
}

/// A game opened from its kept tape, or on its tape prompt.
fn start_game(game: Game) -> Result<Start, String> {
    match game.kept_tape() {
        Some(tape) => Ok(Start::Play(Box::new(game.playing(&tape)?))),
        None => Ok(Start::Prompt(Box::new(game))),
    }
}

/// A tape file played as the game a definition knows it to be, on its game's machine; or
/// plainly, on a 48K. With its title and what a pad does in it.
fn tape_session(file: &Path, library: &Library, data: &Path) -> Result<Playing, String> {
    if let Ok(Identified::Game { package, tape, .. }) = library.identify(file) {
        return Game::new(package.clone(), data).playing(&tape);
    }
    let (machine, tape) = tape_file(file)?;
    let title = Path::new(&tape.name)
        .file_stem()
        .map_or_else(|| tape.name.clone(), |s| s.to_string_lossy().into_owned());
    Ok(Playing {
        session: Session::new(machine, None),
        title,
        mapping: Mapping::default(),
        folder: None,
        saves: Saves::of_tape(&tape.name, &tape.sha1, data),
        quit_slot: true,
        keep_lists: true,
        save_point: SavePoint::Quick,
        paused: false,
        source: None,
    })
}

/// `sidekick run`'s game: the tape as `definition` describes it, or as the library knows it.
fn run_session(
    file: &Path,
    definition: Option<&Path>,
    extra: &[PathBuf],
) -> Result<Playing, String> {
    let data = games::data_folder();
    if let Some(definition) = definition {
        let package = Package::load(definition).map_err(|e| e.to_string())?;
        let game = Game::new(package, &data);
        let tapes = tapes_in(file).map_err(|e| e.to_string())?;
        let tape = tapes
            .first()
            .ok_or_else(|| format!("{}: no tape in it", file.display()))?;
        return game.playing(tape);
    }
    let (library, _) = games::library(&data, extra);
    tape_session(file, &library, &data)
}

/// A 48K with the tape in a file going in, as `LOAD ""` loads it, and the tape.
fn tape_file(path: &Path) -> Result<(Machine, Tape), String> {
    let tape = tapes_in(path)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|t| t.format == TapeFormat::Tap)
        .ok_or_else(|| format!("{}: only .tap tapes can be played yet", path.display()))?;
    let machine = games::start(Model::Sinclair48K, &tape)?;
    Ok((machine, tape))
}

fn check(path: &Path, tape: Option<&Path>) -> ExitCode {
    let package = match Package::load(path) {
        Ok(package) => package,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(UNREADABLE);
        }
    };
    let (warnings, errors): (Vec<_>, Vec<_>) = package
        .definition
        .check()
        .into_iter()
        .partition(|p| p.warning);
    let mut problems: Vec<String> = errors.iter().map(ToString::to_string).collect();
    match games::script(&package) {
        Some(Err(why)) => problems.push(format!("script: {why}")),
        Some(Ok(script)) => {
            for (i, action) in package.definition.actions.iter().enumerate() {
                if let Some(function) = action.script.as_ref().filter(|f| !script.has(f)) {
                    problems.push(format!(
                        "actions[{i}].script: the script has no function {function}"
                    ));
                }
            }
        }
        None => {}
    }
    if let Some(tape) = tape {
        let mut library = Library::default();
        library.insert(package);
        match library.identify(tape) {
            Ok(Identified::Game { media, .. }) => {
                println!("{}: one of its dumps (media[{media}])", tape.display());
            }
            Ok(Identified::Unknown { tapes }) => {
                for found in tapes {
                    problems.push(format!(
                        "{}: {} isn't one of its dumps (SHA-1 {})",
                        tape.display(),
                        found.name,
                        found.sha1
                    ));
                }
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(UNREADABLE);
            }
        }
    }
    // Warnings are said, but don't fail the check.
    for problem in warnings
        .iter()
        .map(ToString::to_string)
        .chain(problems.iter().cloned())
    {
        println!("{problem}");
    }
    if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(PROBLEMS)
    }
}

/// `sidekick import-pok`: a `.pok` file's trainers as `[[cheats]]`.
fn import_pok(file: &Path, output: Option<&Path>) -> ExitCode {
    let cheats = fs::read(file)
        .map_err(|e| e.to_string())
        .and_then(|bytes| {
            sidekick_def::pok::read(&String::from_utf8_lossy(&bytes)).map_err(|e| e.to_string())
        })
        .map(|trainers| sidekick_def::pok::cheats_toml(&trainers));
    let written = cheats.and_then(|toml| match output {
        None => {
            print!("{toml}");
            Ok(())
        }
        Some(path) => fs::write(path, toml).map_err(|e| format!("{}: {e}", path.display())),
    });
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", file.display());
            ExitCode::from(UNREADABLE)
        }
    }
}

fn convert(file: &Path, output: Option<&Path>) -> ExitCode {
    let text = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{}: {e}", file.display());
            return ExitCode::from(UNREADABLE);
        }
    };
    let json_in = file
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    let parsed = if json_in {
        Definition::from_json(&text)
    } else {
        Definition::from_toml(&text)
    };
    let definition = match parsed {
        Ok(definition) => definition,
        Err(e) => {
            eprintln!("{}: {e}", file.display());
            return ExitCode::from(UNREADABLE);
        }
    };
    let converted = if json_in {
        definition.to_toml()
    } else {
        definition.to_json() + "\n"
    };
    match output {
        None => {
            print!("{converted}");
            ExitCode::SUCCESS
        }
        Some(out) => match fs::write(out, converted) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}: {e}", out.display());
                ExitCode::from(UNREADABLE)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(data: &Path) -> Playing {
        let machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        Playing {
            session: Session::new(machine, None),
            title: "Plain".to_owned(),
            mapping: Mapping::default(),
            folder: None,
            saves: Saves::of_tape("plain.tap", "ABC123", data),
            quit_slot: true,
            keep_lists: true,
            save_point: SavePoint::Quick,
            paused: false,
            source: None,
        }
    }

    #[test]
    fn load_starts_from_a_save_and_leaves_the_quit_slot_alone() {
        let data = tempfile::TempDir::new().unwrap();
        let mut first = plain(data.path());
        first.session.machine_mut().write(0x9000, 0x42);
        let file = data.path().join("game.sav");
        saves::write(&mut first.session, &first.saves, &file).unwrap();
        let Ok(Start::Play(playing)) =
            loading(Start::Play(Box::new(plain(data.path()))), Some(&file))
        else {
            panic!("it plays");
        };
        assert_eq!(playing.session.machine().peek(0x9000), 0x42);
        assert!(playing.session.record().loaded);
        assert!(!playing.quit_slot, "neither offered nor written over");
        assert!(playing.keep_lists, "a save's lists are the player's");
        assert_eq!(
            playing.save_point,
            SavePoint::File(file),
            "F9 goes back to it, until an F5"
        );
        assert!(is_situation(Path::new("a/stocked-up.TOML")));
    }

    #[test]
    fn load_wants_a_game_to_load_into() {
        let file = Path::new("game.sav");
        let error = loading(Start::Library(Vec::new()), Some(file))
            .err()
            .expect("refused");
        assert!(error.contains("name the game"), "{error}");
        assert!(matches!(
            loading(Start::Library(Vec::new()), None),
            Ok(Start::Library(_))
        ));
    }
}
