//! `sidekick`: ZX Sidekick's program. `sidekick [GAME-ID | TAPE]` plays in a window; the other
//! commands are for definition authors.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command line's results and errors are printed"
)]

mod games;
mod headless;
mod keys;
mod pad;
mod picture;
mod window;

use clap::{Parser, Subcommand};
use games::Game;
use pad::Mapping;
use sidekick_core::{Machine, Model, Session};
use sidekick_def::{tapes_in, Definition, Identified, Library, Package, TapeFormat};
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
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Checks a definition, and optionally that a tape is one of its dumps.
    ///
    /// Prints each problem as `path: message`. Exits 0 with none, 1 with problems, 2 when
    /// something couldn't be read.
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
        /// Where to write the picture after the last frame, as a PNG.
        #[arg(long, value_name = "FILE")]
        screenshot: Option<PathBuf>,
        /// How many screen pixels each Spectrum pixel is in the PNG.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=16))]
        scale: u32,
    },
    /// Converts a definition from TOML to JSON, or from JSON to TOML.
    Convert {
        /// A .toml or .json definition file.
        file: PathBuf,
        /// Where to write it (standard output when not given).
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
        (
            Some(Command::Run {
                tape,
                definition,
                games,
                values,
                frames,
                keys,
                switches,
                screenshot,
                scale,
            }),
            _,
        ) => {
            let run = run_session(&tape, definition.as_deref(), &games).and_then(
                |(mut session, _, _)| {
                    headless::run(
                        &mut session,
                        frames,
                        &keys,
                        &switches,
                        screenshot.as_deref(),
                        scale,
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
        (None, target) => match open(target.as_deref(), &cli.games).and_then(window::run) {
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
        let (session, title, mapping) = tape_session(file, &library, &data)?;
        return Ok(Start::Play(Box::new(session), title, mapping));
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

/// A game opened from its kept tape, or on its tape prompt.
fn start_game(game: Game) -> Result<Start, String> {
    match game.kept_tape() {
        Some(tape) => {
            let session = game.session(&tape)?;
            Ok(Start::Play(Box::new(session), game.title(), game.mapping()))
        }
        None => Ok(Start::Prompt(Box::new(game))),
    }
}

/// A tape file played as the game a definition knows it to be, on its game's machine; or
/// plainly, on a 48K. With its title and what a pad does in it.
fn tape_session(
    file: &Path,
    library: &Library,
    data: &Path,
) -> Result<(Session, String, Mapping), String> {
    if let Ok(Identified::Game { package, tape, .. }) = library.identify(file) {
        let game = Game::new(package.clone(), data);
        return Ok((game.session(&tape)?, game.title(), game.mapping()));
    }
    let (machine, title) = tape_file(file)?;
    Ok((Session::new(machine, None), title, Mapping::default()))
}

/// `sidekick run`'s game: the tape as `definition` describes it, or as the library knows it.
fn run_session(
    file: &Path,
    definition: Option<&Path>,
    extra: &[PathBuf],
) -> Result<(Session, String, Mapping), String> {
    let data = games::data_folder();
    if let Some(definition) = definition {
        let package = Package::load(definition).map_err(|e| e.to_string())?;
        let game = Game::new(package, &data);
        let tapes = tapes_in(file).map_err(|e| e.to_string())?;
        let tape = tapes
            .first()
            .ok_or_else(|| format!("{}: no tape in it", file.display()))?;
        return Ok((game.session(tape)?, game.title(), game.mapping()));
    }
    let (library, _) = games::library(&data, extra);
    tape_session(file, &library, &data)
}

/// A 48K with the tape in a file going in, as `LOAD ""` loads it, and the tape's name.
fn tape_file(path: &Path) -> Result<(Machine, String), String> {
    let tapes = tapes_in(path).map_err(|e| e.to_string())?;
    let tape = tapes
        .iter()
        .find(|t| t.format == TapeFormat::Tap)
        .ok_or_else(|| format!("{}: only .tap tapes can be played yet", path.display()))?;
    let machine = games::start(Model::Sinclair48K, tape)?;
    let title = Path::new(&tape.name)
        .file_stem()
        .map_or_else(|| tape.name.clone(), |s| s.to_string_lossy().into_owned());
    Ok((machine, title))
}

fn check(path: &Path, tape: Option<&Path>) -> ExitCode {
    let package = match Package::load(path) {
        Ok(package) => package,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(UNREADABLE);
        }
    };
    let mut problems: Vec<String> = package
        .definition
        .check()
        .iter()
        .map(ToString::to_string)
        .collect();
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
    for problem in &problems {
        println!("{problem}");
    }
    if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(PROBLEMS)
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
