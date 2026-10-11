//! Finding definitions on disk, and matching a tape to its game.

use crate::{Definition, TapeFormat};
use sha1::{Digest, Sha1};
use std::{
    collections::BTreeMap,
    fmt, fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

/// The most a single file read from a zip may hold. Tapes and definitions are far smaller; a
/// zip claiming more is broken or hostile.
const ZIP_ENTRY_LIMIT: u64 = 16 * 1024 * 1024;

/// The names a definition has inside a zip or folder.
const DEFINITION_NAMES: [&str; 2] = ["game.toml", "game.json"];

/// Something that went wrong reading a definition or a tape, with the file it's about.
#[derive(Debug)]
pub struct LoadError {
    /// The file or folder.
    pub path: PathBuf,
    /// What went wrong.
    pub problem: LoadProblem,
}

/// What went wrong reading a definition or a tape.
#[derive(Debug)]
pub enum LoadProblem {
    /// It couldn't be read.
    Io(std::io::Error),
    /// It isn't a zip, or a zip that can be read.
    Zip(String),
    /// A zip entry is past the size limit.
    TooLarge(String),
    /// It holds no `game.toml` or `game.json`.
    NoDefinition,
    /// Its definition doesn't parse.
    Definition(crate::Error),
    /// A file asked of a package isn't in it, or its path leaves the package.
    NoSuchFile(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path.display();
        match &self.problem {
            LoadProblem::Io(e) => write!(f, "{path}: {e}"),
            LoadProblem::Zip(e) => write!(f, "{path}: not a readable zip: {e}"),
            LoadProblem::TooLarge(name) => write!(f, "{path}: {name} is too large"),
            LoadProblem::NoDefinition => write!(f, "{path}: no game.toml or game.json"),
            LoadProblem::Definition(e) => write!(f, "{path}: {e}"),
            LoadProblem::NoSuchFile(name) => write!(f, "{path}: no file {name}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.problem {
            LoadProblem::Io(e) => Some(e),
            LoadProblem::Definition(e) => Some(e),
            _ => None,
        }
    }
}

fn fail<T>(path: &Path, problem: LoadProblem) -> Result<T, LoadError> {
    Err(LoadError {
        path: path.to_owned(),
        problem,
    })
}

/// Where a package's files are.
#[derive(Clone, Debug)]
enum Source {
    /// A single definition file: there are no other files.
    File,
    /// A folder in the zip's layout.
    Folder(PathBuf),
    /// A zip, kept whole.
    Zip(Vec<u8>),
}

/// A definition, and the files that came with it.
#[derive(Clone, Debug)]
pub struct Package {
    /// The definition.
    pub definition: Definition,
    /// Where it was read from.
    pub path: PathBuf,
    source: Source,
}

impl Package {
    /// Reads a definition from a `.toml` or `.json` file, a `.zip` with `game.toml` or
    /// `game.json` at its top, or a folder holding one.
    ///
    /// # Errors
    /// If it can't be read, holds no definition, or its definition doesn't parse.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        if path.is_dir() {
            for name in DEFINITION_NAMES {
                let file = path.join(name);
                if file.is_file() {
                    let definition = parse_file(&file)?;
                    return Ok(Self {
                        definition,
                        path: path.to_owned(),
                        source: Source::Folder(path.to_owned()),
                    });
                }
            }
            return fail(path, LoadProblem::NoDefinition);
        }
        match extension(path).as_deref() {
            Some("zip") => {
                let bytes = fs::read(path).or_else(|e| fail(path, LoadProblem::Io(e)))?;
                let (name, text) = DEFINITION_NAMES
                    .iter()
                    .find_map(|name| {
                        zip_entry(path, &bytes, name)
                            .transpose()
                            .map(|entry| entry.map(|text| (*name, text)))
                    })
                    .ok_or_else(|| LoadError {
                        path: path.to_owned(),
                        problem: LoadProblem::NoDefinition,
                    })??;
                let definition = parse(path, name, &text)?;
                Ok(Self {
                    definition,
                    path: path.to_owned(),
                    source: Source::Zip(bytes),
                })
            }
            _ => Ok(Self {
                definition: parse_file(path)?,
                path: path.to_owned(),
                source: Source::File,
            }),
        }
    }

    /// Reads one of the package's other files, by its path within the package
    /// (`scripts/starquake.rhai`).
    ///
    /// # Errors
    /// If there's no such file, or the path would leave the package.
    pub fn read(&self, relative: &str) -> Result<Vec<u8>, LoadError> {
        let inside = Path::new(relative)
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
        if !inside {
            return fail(&self.path, LoadProblem::NoSuchFile(relative.to_owned()));
        }
        match &self.source {
            Source::File => fail(&self.path, LoadProblem::NoSuchFile(relative.to_owned())),
            Source::Folder(folder) => fs::read(folder.join(relative))
                .or_else(|_| fail(&self.path, LoadProblem::NoSuchFile(relative.to_owned()))),
            Source::Zip(bytes) => {
                zip_entry(&self.path, bytes, relative)?.ok_or_else(|| LoadError {
                    path: self.path.clone(),
                    problem: LoadProblem::NoSuchFile(relative.to_owned()),
                })
            }
        }
    }
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

fn parse_file(path: &Path) -> Result<Definition, LoadError> {
    let bytes = fs::read(path).or_else(|e| fail(path, LoadProblem::Io(e)))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    parse(path, name, &bytes)
}

/// Parses a definition from `name`'s bytes: JSON if the name ends in `.json`, else TOML.
fn parse(path: &Path, name: &str, bytes: &[u8]) -> Result<Definition, LoadError> {
    let text = String::from_utf8_lossy(bytes);
    let parsed = if extension(Path::new(name)).as_deref() == Some("json") {
        Definition::from_json(&text)
    } else {
        Definition::from_toml(&text)
    };
    parsed.or_else(|e| fail(path, LoadProblem::Definition(e)))
}

fn open_zip<'a>(
    path: &Path,
    bytes: &'a [u8],
) -> Result<zip::ZipArchive<Cursor<&'a [u8]>>, LoadError> {
    zip::ZipArchive::new(Cursor::new(bytes))
        .or_else(|e| fail(path, LoadProblem::Zip(e.to_string())))
}

fn read_entry(
    path: &Path,
    mut entry: zip::read::ZipFile<'_, Cursor<&[u8]>>,
) -> Result<Vec<u8>, LoadError> {
    let name = entry_name(path, &entry)?;
    if entry.size() > ZIP_ENTRY_LIMIT {
        return fail(path, LoadProblem::TooLarge(name));
    }
    let mut data = Vec::new();
    entry
        .by_ref()
        .take(ZIP_ENTRY_LIMIT + 1)
        .read_to_end(&mut data)
        .or_else(|e| fail(path, LoadProblem::Io(e)))?;
    if data.len() as u64 > ZIP_ENTRY_LIMIT {
        return fail(path, LoadProblem::TooLarge(name));
    }
    Ok(data)
}

fn entry_name(
    path: &Path,
    entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>,
) -> Result<String, LoadError> {
    entry
        .name()
        .map(std::borrow::Cow::into_owned)
        .or_else(|e| fail(path, LoadProblem::Zip(e.to_string())))
}

/// The zip entry with this exact name, if there is one.
fn zip_entry(path: &Path, bytes: &[u8], name: &str) -> Result<Option<Vec<u8>>, LoadError> {
    let mut zip = open_zip(path, bytes)?;
    let entry = match zip.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return fail(path, LoadProblem::Zip(e.to_string())),
    };
    read_entry(path, entry).map(Some)
}

/// The definitions found, by game id.
#[derive(Clone, Debug, Default)]
pub struct Library {
    games: BTreeMap<String, Package>,
}

impl Library {
    /// Searches these folders, in order, for definitions: `.toml`, `.json` and `.zip` files,
    /// and folders holding `game.toml` or `game.json`. A game found again in a later folder
    /// replaces the earlier one. A folder that doesn't exist is skipped; anything that looks
    /// like a definition but doesn't read is returned as a problem, and the search goes on.
    #[must_use]
    pub fn search(folders: &[PathBuf]) -> (Self, Vec<LoadError>) {
        let mut library = Self::default();
        let mut problems = Vec::new();
        for folder in folders {
            let Ok(entries) = fs::read_dir(folder) else {
                continue;
            };
            let mut paths: Vec<PathBuf> =
                entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
            paths.sort();
            for path in paths {
                let candidate = if path.is_dir() {
                    DEFINITION_NAMES.iter().any(|n| path.join(n).is_file())
                } else {
                    matches!(extension(&path).as_deref(), Some("toml" | "json" | "zip"))
                };
                if !candidate {
                    continue;
                }
                match Package::load(&path) {
                    Ok(package) => {
                        library
                            .games
                            .insert(package.definition.game.id.clone(), package);
                    }
                    Err(e) => problems.push(e),
                }
            }
        }
        (library, problems)
    }

    /// A game, by its id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Package> {
        self.games.get(id)
    }

    /// Every game found, by id.
    pub fn games(&self) -> impl Iterator<Item = &Package> {
        self.games.values()
    }

    /// Matches the tapes in a file (a `.tap` or `.tzx`, or a `.zip` holding them) to a game,
    /// by each tape's SHA-1. The file's name plays no part.
    ///
    /// # Errors
    /// If the file can't be read, or holds no tape.
    pub fn identify(&self, path: &Path) -> Result<Identified<'_>, LoadError> {
        let tapes = tapes_in(path)?;
        for tape in &tapes {
            for package in self.games.values() {
                let media = package.definition.media.iter().position(|m| {
                    m.sha1
                        .as_deref()
                        .is_some_and(|sha1| sha1.eq_ignore_ascii_case(&tape.sha1))
                });
                if let Some(media) = media {
                    return Ok(Identified::Game {
                        package,
                        media,
                        tape: tape.clone(),
                    });
                }
            }
        }
        Ok(Identified::Unknown { tapes })
    }
}

/// What a tape file turned out to be.
#[derive(Debug)]
pub enum Identified<'a> {
    /// A tape a definition supports.
    Game {
        /// The game's definition.
        package: &'a Package,
        /// Which of its `[[media]]` it is.
        media: usize,
        /// The tape.
        tape: Tape,
    },
    /// No definition supports any tape in it.
    Unknown {
        /// The tapes it holds, for plain play and for saying which dump this is.
        tapes: Vec<Tape>,
    },
}

/// A tape, read whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tape {
    /// Its file name (within the zip, if it came in one).
    pub name: String,
    /// Its format.
    pub format: TapeFormat,
    /// Its SHA-1, as 40 lowercase hex digits.
    pub sha1: String,
    /// Its bytes.
    pub bytes: Vec<u8>,
}

impl Tape {
    fn new(name: String, format: TapeFormat, bytes: Vec<u8>) -> Self {
        let sha1 = Sha1::digest(&bytes)
            .iter()
            .fold(String::with_capacity(40), |mut hex, b| {
                use fmt::Write as _;
                let _ = write!(hex, "{b:02x}");
                hex
            });
        Self {
            name,
            format,
            sha1,
            bytes,
        }
    }
}

fn tape_format(name: &str) -> Option<TapeFormat> {
    match extension(Path::new(name)).as_deref() {
        Some("tap") => Some(TapeFormat::Tap),
        Some("tzx") => Some(TapeFormat::Tzx),
        _ => None,
    }
}

/// The tapes in a file: the file itself, or each tape in a zip.
///
/// # Errors
/// If the file can't be read, or holds no tape.
pub fn tapes_in(path: &Path) -> Result<Vec<Tape>, LoadError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_owned();
    let bytes = fs::read(path).or_else(|e| fail(path, LoadProblem::Io(e)))?;
    if let Some(format) = tape_format(&name) {
        return Ok(vec![Tape::new(name, format, bytes)]);
    }
    let mut zip = open_zip(path, &bytes)?;
    let mut tapes = Vec::new();
    for i in 0..zip.len() {
        let entry = zip
            .by_index(i)
            .or_else(|e| fail(path, LoadProblem::Zip(e.to_string())))?;
        let entry_name = entry_name(path, &entry)?;
        if entry.is_dir() {
            continue;
        }
        if let Some(format) = tape_format(&entry_name) {
            let data = read_entry(path, entry)?;
            tapes.push(Tape::new(entry_name, format, data));
        }
    }
    if tapes.is_empty() {
        return fail(path, LoadProblem::NoSuchFile("a .tap or .tzx".to_owned()));
    }
    Ok(tapes)
}
