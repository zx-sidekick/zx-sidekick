//! Keeping the game's high-score table between runs (#47), with the
//! guidance each entry was played with. The table lives in the game's
//! memory as it always has; this keeps a copy in `high-scores.txt` beside
//! the kept tape, puts it back into memory when the tape is loaded, and
//! takes the table again whenever the CORE OF HEROES screen shows it.

use std::path::{Path, PathBuf};

use sidekick::starquake::{HighScore, at};

use super::guidance::Record;

const FILE: &str = "high-scores.txt";

type Table = [HighScore; at::HIGH_SCORE_COUNT];

/// The table and, for each entry, the highest guidance level its game had:
/// `None` for an entry the tape came with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kept {
    pub entries: Table,
    pub levels: [Option<u8>; at::HIGH_SCORE_COUNT],
}

impl Kept {
    /// The table as the game has it, none of it played here.
    pub fn from_tape(entries: Table) -> Kept {
        Kept {
            entries,
            levels: [None; at::HIGH_SCORE_COUNT],
        }
    }

    /// The table after a game played with guidance up to `level`, which the
    /// game now shows as `now`: the entry that is new gets `level`, and the
    /// others keep theirs as they moved down.
    #[must_use]
    pub fn after(&self, now: &Table, level: u8) -> Kept {
        let n = at::HIGH_SCORE_COUNT;
        // The new entry is where the tables first differ, if everything
        // below it is the old table moved down one.
        let new = (0..n)
            .find(|&i| now[i] != self.entries[i])
            .filter(|&i| now[..i] == self.entries[..i] && now[i + 1..] == self.entries[i..n - 1]);
        let levels = std::array::from_fn(|i| match new {
            Some(k) if i == k => Some(level),
            Some(k) if i > k => self.levels[i - 1],
            Some(_) => self.levels[i],
            // No new entry, or a table changed some other way: each entry
            // keeps the level of the same entry before, if there was one.
            None if now == &self.entries => self.levels[i],
            None => self
                .entries
                .iter()
                .position(|e| *e == now[i])
                .map_or(Some(level), |j| self.levels[j]),
        });
        Kept {
            entries: *now,
            levels,
        }
    }

    /// The file's text: a line an entry, best first, of its name, score,
    /// percentage and guidance level, or `-` for the tape's own.
    pub fn text(&self) -> String {
        self.entries
            .iter()
            .zip(self.levels)
            .map(|(e, level)| {
                format!(
                    "{} {} {} {}\n",
                    String::from_utf8_lossy(&e.name),
                    String::from_utf8_lossy(&e.score),
                    e.percent,
                    level.map_or("-".to_string(), |l| l.to_string())
                )
            })
            .collect()
    }

    /// A table from the file's text; `None` when it is not one.
    pub fn parse(text: &str) -> Option<Kept> {
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() != at::HIGH_SCORE_COUNT {
            return None;
        }
        let mut kept = Kept::from_tape([HighScore::default(); at::HIGH_SCORE_COUNT]);
        for (i, line) in lines.iter().enumerate() {
            let bytes = line.as_bytes();
            let name: [u8; 3] = bytes.get(..3)?.try_into().ok()?;
            if !name.iter().all(|b| (0x20..0x7F).contains(b)) {
                return None;
            }
            let mut rest = line.get(3..)?.split_whitespace();
            let score: [u8; 6] = rest.next()?.as_bytes().try_into().ok()?;
            if !score.iter().all(u8::is_ascii_digit) {
                return None;
            }
            let percent = rest.next()?.parse().ok()?;
            let level = match rest.next()? {
                "-" => None,
                l => Some(l.parse::<u8>().ok().filter(|&l| l <= 5)?),
            };
            if rest.next().is_some() {
                return None;
            }
            kept.entries[i] = HighScore {
                name,
                score,
                percent,
            };
            kept.levels[i] = level;
        }
        Some(kept)
    }
}

/// What happens to the table around a game over.
#[derive(Debug)]
pub struct Keeper {
    pub kept: Kept,
    /// Whether the file may be written: not when one is there that could
    /// not be read, which is left alone.
    writable: bool,
    /// The table to put back once the screen is done, after a game with
    /// training.
    restore: Option<Table>,
}

impl Keeper {
    /// Starts from the file's text, if there is a file, or from the tape's
    /// own table `shipped`.
    pub fn new(file: Option<&str>, shipped: Table) -> Keeper {
        let read = file.map(Kept::parse);
        Keeper {
            kept: read.flatten().unwrap_or_else(|| Kept::from_tape(shipped)),
            writable: !matches!(read, Some(None)),
            restore: None,
        }
    }

    /// At the CORE OF HEROES screen, showing `now` after a game with
    /// `record`: the table to save when it changed, `None` otherwise. A game
    /// with training changes nothing, and its table is put back at the menu.
    pub fn heroes(&mut self, now: &Table, record: Record) -> Option<Kept> {
        if record.training {
            if now != &self.kept.entries {
                self.restore = Some(self.kept.entries);
            }
            return None;
        }
        let after = self.kept.after(now, record.highest);
        if after == self.kept {
            return None;
        }
        self.kept = after;
        self.writable.then_some(after)
    }

    /// Back at the menu: the table to put back into the game's memory, if a
    /// game with training changed it.
    pub fn menu(&mut self) -> Option<Table> {
        self.restore.take()
    }
}

/// Where the table is kept: beside the kept tape.
pub fn path() -> Option<PathBuf> {
    super::tape::app_dir().map(|d| d.join(FILE))
}

/// Writes `kept` to `path`, making its folder if need be.
///
/// # Errors
///
/// If the folder or the file cannot be written.
pub fn save(path: &Path, kept: &Kept) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, kept.text()).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &[u8; 3], score: &[u8; 6], percent: u8) -> HighScore {
        HighScore {
            name: *name,
            score: *score,
            percent,
        }
    }

    /// The tape's table, with levels: 1 to 8 down the table.
    fn kept() -> Kept {
        let names = [
            b"STA", b"TAR", b"ARQ", b"RQU", b"QUA", b"UAK", b"AKE", b"KES",
        ];
        let scores = [
            b"109825", b"093900", b"082975", b"062050", b"046125", b"030210", b"014275", b"009875",
        ];
        Kept {
            entries: std::array::from_fn(|i| entry(names[i], scores[i], 31 - i as u8)),
            levels: std::array::from_fn(|i| (i < 5).then_some(i as u8)),
        }
    }

    /// `k`'s table with `new` put in at `at`, the rest moved down.
    fn with(k: &Kept, at: usize, new: HighScore) -> Table {
        let mut t = k.entries;
        t.copy_within(at..7, at + 1);
        t[at] = new;
        t
    }

    #[test]
    fn a_new_entry_gets_the_game_s_level_and_the_rest_move_down() {
        let k = kept();
        let new = entry(b"SQK", b"200000", 50);
        for at in [0, 3, 7] {
            let after = k.after(&with(&k, at, new), 4);
            assert_eq!(after.levels[at], Some(4), "new at {at}");
            assert_eq!(after.levels[..at], k.levels[..at], "above {at}");
            assert_eq!(after.levels[at + 1..], k.levels[at..7], "below {at}");
        }
    }

    #[test]
    fn a_score_that_does_not_make_it_changes_nothing() {
        let k = kept();
        assert_eq!(k.after(&k.entries, 3), k);
    }

    #[test]
    fn an_entry_the_same_as_one_below_it_is_told_apart_by_place() {
        let k = kept();
        // A second RQU 062050 put in just above the first.
        let now = with(&k, 3, k.entries[3]);
        let after = k.after(&now, 5);
        assert_eq!(
            after.levels[3..5],
            [Some(3), Some(5)],
            "the lower of the two is new"
        );
        assert_eq!(after.levels.iter().filter(|&&l| l == Some(5)).count(), 1);
    }

    #[test]
    fn training_saves_nothing_and_puts_the_table_back_at_the_menu() {
        let mut keeper = Keeper::new(None, kept().entries);
        let now = with(&keeper.kept, 0, entry(b"SQK", b"200000", 50));
        let training = Record {
            highest: 2,
            training: true,
        };
        assert_eq!(keeper.heroes(&now, training), None);
        assert_eq!(keeper.menu(), Some(kept().entries));
        assert_eq!(keeper.menu(), None, "once");
        let played = Record {
            highest: 2,
            training: false,
        };
        let saved = keeper.heroes(&now, played).expect("a change to save");
        assert_eq!(saved.levels[0], Some(2));
        assert_eq!(keeper.heroes(&now, played), None, "the same screen again");
    }

    #[test]
    fn the_file_reads_back_and_a_damaged_one_is_left_alone() {
        let k = kept();
        let text = k.text();
        assert!(text.starts_with("STA 109825 31 0\n"));
        assert!(text.ends_with("KES 009875 24 -\n"));
        assert_eq!(Kept::parse(&text), Some(k));
        for damaged in [
            "",
            "STA 109825 31 0\n",
            &text.replace("109825", "10982x"),
            &text.replace(" 0\n", " 9\n"),
        ] {
            assert_eq!(Kept::parse(damaged), None, "{damaged:?}");
        }
        let mut keeper = Keeper::new(Some("not a table"), k.entries);
        assert_eq!(keeper.kept.levels, [None; 8], "the tape's table");
        let now = with(&k, 0, entry(b"SQK", b"200000", 50));
        let played = Record {
            highest: 1,
            training: false,
        };
        assert_eq!(keeper.heroes(&now, played), None, "never written over");
        assert_eq!(keeper.kept.levels[0], Some(1), "but kept for the panel");
    }
}
