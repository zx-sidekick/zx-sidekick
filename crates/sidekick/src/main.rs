//! `sidekick`: ZX Sidekick's program. `sidekick TAPE` plays a tape in a window; the other
//! commands are for definition authors.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command line's results and errors are printed"
)]

mod keys;
mod picture;
mod window;

use clap::{Parser, Subcommand};
use sidekick_core::{Machine, Model};
use sidekick_def::{tapes_in, Definition, Identified, Library, Package, TapeFormat};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

/// Play original ZX Spectrum games from your own copy, with a pad, training switches and
/// guidance.
#[derive(Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
struct Cli {
    /// A tape to play: a .tap, or the .zip it came in.
    tape: Option<PathBuf>,
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
    match (cli.command, cli.tape) {
        (Some(Command::Check { definition, tape }), _) => check(&definition, tape.as_deref()),
        (Some(Command::Convert { file, output }), _) => convert(&file, output.as_deref()),
        (None, Some(tape)) => play(&tape),
        (None, None) => {
            eprintln!("Give a tape to play: sidekick TAPE (sidekick --help for more)");
            ExitCode::from(UNREADABLE)
        }
    }
}

/// Plays a tape plainly: a 48K, the tape loading as `LOAD ""` loads it, the keyboard reaching
/// the Spectrum.
fn play(path: &Path) -> ExitCode {
    let tapes = match tapes_in(path) {
        Ok(tapes) => tapes,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(UNREADABLE);
        }
    };
    let Some(tape) = tapes.iter().find(|t| t.format == TapeFormat::Tap) else {
        eprintln!("{}: only .tap tapes can be played yet", path.display());
        return ExitCode::from(UNREADABLE);
    };
    let started = Machine::new(Model::Sinclair48K).and_then(|mut machine| {
        machine.load_tap(&tape.bytes)?;
        Ok(machine)
    });
    let machine = match started {
        Ok(machine) => machine,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::from(UNREADABLE);
        }
    };
    let title = Path::new(&tape.name)
        .file_stem()
        .map_or_else(|| tape.name.clone(), |s| s.to_string_lossy().into_owned());
    match window::play(machine, &title) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(UNREADABLE)
        }
    }
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
