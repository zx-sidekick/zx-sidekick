//! A game's high-score table kept between runs: its file, and which entry is the game just
//! played, marked with how it was played.

use sidekick_core::{Record, ScoreEntry};
use sidekick_ui::{ScoreRow, ThisGame};
use std::{fmt::Write as _, fs, io, path::Path};

/// The file the table is kept in, in the game's data folder.
pub(crate) const FILE: &str = "high-scores.txt";

/// An entry of the kept table: the game's, and how its game was played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Kept {
    pub(crate) entry: ScoreEntry,
    /// "level 3 · What you have seen", "no guidance", "from the tape".
    pub(crate) mark: String,
    /// A training game's: shown, never kept.
    pub(crate) training: bool,
}

/// The kept table, a line an entry: name, score and mark, tab-separated.
pub(crate) fn load(folder: &Path) -> Vec<Kept> {
    fs::read_to_string(folder.join(FILE))
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let mut fields = line.split('\t');
                    Some(Kept {
                        entry: ScoreEntry {
                            name: fields.next()?.to_owned(),
                            score: fields.next()?.to_owned(),
                        },
                        mark: fields.next().unwrap_or("from the tape").to_owned(),
                        training: false,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Writes the table, leaving out training games' entries.
///
/// # Errors
/// If the folder or file can't be written.
pub(crate) fn save(folder: &Path, table: &[Kept]) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let mut text = String::new();
    for k in table.iter().filter(|k| !k.training) {
        let _ = writeln!(text, "{}\t{}\t{}", k.entry.name, k.entry.score, k.mark);
    }
    fs::write(folder.join(FILE), text)
}

/// How a game was played, as its entry says: its guidance and whether it trained.
pub(crate) fn mark(record: &Record, level_name: Option<&str>) -> String {
    let mut mark = match (record.level, level_name) {
        (0, _) => "no guidance".to_owned(),
        (n, Some(name)) => format!("level {n} · {name}"),
        (n, None) => format!("level {n}"),
    };
    if record.loaded {
        mark.push_str(" · loaded");
    }
    if !record.switches.is_empty() {
        mark.push_str(" · training");
    }
    mark
}

/// The table read back after a game, against the one before it: an entry that wasn't there
/// is this game's, given `mark`; the rest keep theirs. Returns the table, and where this
/// game's entry is, if it made one.
pub(crate) fn after_game(
    before: &[Kept],
    read: Vec<ScoreEntry>,
    mark: &str,
    training: bool,
) -> (Vec<Kept>, Option<usize>) {
    let mut unmatched: Vec<&Kept> = before.iter().collect();
    let mut mine = None;
    let table = read
        .into_iter()
        .enumerate()
        .map(|(i, entry)| {
            if let Some(at) = unmatched.iter().position(|k| k.entry == entry) {
                let was = unmatched.remove(at);
                Kept {
                    entry,
                    ..was.clone()
                }
            } else if mine.is_none() {
                mine = Some(i);
                Kept {
                    entry,
                    mark: mark.to_owned(),
                    training,
                }
            } else {
                Kept {
                    entry,
                    mark: "from the tape".to_owned(),
                    training: false,
                }
            }
        })
        .collect();
    (table, mine)
}

/// The panel's page after a game.
pub(crate) fn this_game(
    record: &Record,
    level_name: Option<&str>,
    title: &str,
    table: &[Kept],
    mine: Option<usize>,
) -> ThisGame {
    let played_with = if record.level == 0 {
        "No guidance".to_owned()
    } else {
        format!("Guidance up to level {}", record.level)
    };
    let mut detail = level_name
        .filter(|_| record.level > 0)
        .unwrap_or_default()
        .to_owned();
    if !record.switches.is_empty() {
        if !detail.is_empty() {
            detail.push_str(" · ");
        }
        detail.push_str("training, so not kept");
    }
    ThisGame {
        played_with,
        detail,
        title: title.to_owned(),
        rows: table
            .iter()
            .enumerate()
            .map(|(i, k)| ScoreRow {
                name: k.entry.name.clone(),
                mark: k.mark.clone(),
                mine: mine == Some(i),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, score: &str) -> ScoreEntry {
        ScoreEntry {
            name: name.to_owned(),
            score: score.to_owned(),
        }
    }

    #[test]
    fn the_new_entry_is_this_game_s_and_the_rest_keep_their_marks() {
        let before = vec![
            Kept {
                entry: entry("BOB", "900"),
                mark: "no guidance".to_owned(),
                training: false,
            },
            Kept {
                entry: entry("CAT", "50"),
                mark: "from the tape".to_owned(),
                training: false,
            },
        ];
        let read = vec![
            entry("ANN", "1200"),
            entry("BOB", "900"),
            entry("CAT", "50"),
        ];
        let (table, mine) = after_game(&before, read, "level 3 · Seen", false);
        assert_eq!(mine, Some(0));
        let marks: Vec<&str> = table.iter().map(|k| k.mark.as_str()).collect();
        assert_eq!(marks, ["level 3 · Seen", "no guidance", "from the tape"]);
        let (same, none) = after_game(
            &table,
            table.iter().map(|k| k.entry.clone()).collect(),
            "x",
            false,
        );
        assert_eq!((none, same), (None, table), "no new entry: nothing changes");
    }

    #[test]
    fn a_training_game_is_shown_but_not_kept() {
        let folder = tempfile::TempDir::new().unwrap();
        let record = Record {
            level: 2,
            switches: std::collections::BTreeSet::from(["full-energy".to_owned()]),
            ..Record::default()
        };
        let mark = mark(&record, Some("Map"));
        assert_eq!(mark, "level 2 · Map · training");
        let (table, mine) = after_game(&[], vec![entry("ANN", "5")], &mark, true);
        let page = this_game(&record, Some("Map"), "HEROES", &table, mine);
        assert!(page.rows[0].mine);
        assert_eq!(page.detail, "Map · training, so not kept");
        save(folder.path(), &table).unwrap();
        assert!(load(folder.path()).is_empty(), "left out of the file");
    }

    #[test]
    fn rows_the_kept_table_doesn_t_cover_are_the_tape_s() {
        let kept = vec![Kept {
            entry: entry("ANN", "500"),
            mark: "level 1 · Codes".to_owned(),
            training: false,
        }];
        let now = vec![entry("ANN", "500"), entry("HAL", "10")];
        let (table, _) = after_game(&kept, now, "from the tape", false);
        let marks: Vec<&str> = table.iter().map(|k| k.mark.as_str()).collect();
        assert_eq!(marks, ["level 1 · Codes", "from the tape"]);
        let (_, mine) = after_game(
            &table,
            vec![entry("ANN", "500"), entry("HAL", "10")],
            "x",
            false,
        );
        assert_eq!(mine, None, "so HAL isn't taken for the next game's");
    }

    #[test]
    fn the_file_is_read_back_as_written() {
        let folder = tempfile::TempDir::new().unwrap();
        let read = vec![entry("ANN", "12500"), entry("BOB", "900")];
        let (table, _) = after_game(&[], read, "from the tape", false);
        save(folder.path(), &table).unwrap();
        assert_eq!(
            fs::read_to_string(folder.path().join(FILE)).unwrap(),
            "ANN\t12500\tfrom the tape\nBOB\t900\tfrom the tape\n"
        );
        assert_eq!(load(folder.path()), table);
        assert_eq!(mark(&Record::default(), None), "no guidance");
        let loaded = Record {
            level: 5,
            loaded: true,
            ..Record::default()
        };
        assert_eq!(mark(&loaded, Some("Routes")), "level 5 · Routes · loaded");
    }
}
