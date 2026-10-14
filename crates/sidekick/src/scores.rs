//! A game's high-score table kept between runs: its file, and which entry is the game just
//! played, marked with how it was played. Only games played here are kept: the game's own
//! entries stay in its memory, never in the file.

use sidekick_core::{Record, ScoreEntry, Session};
use sidekick_ui::{ScoreRow, ThisGame};
use std::{fmt::Write as _, fs, io, path::Path};

/// The file the table is kept in, in the game's data folder.
pub(crate) const FILE: &str = "high-scores.txt";

/// The mark of an entry no game here made: the game's own, never kept.
pub(crate) const FROM_TAPE: &str = "from the tape";

/// An entry of the kept table: the game's, and how its game was played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Kept {
    pub(crate) entry: ScoreEntry,
    /// "level 3 · What you have seen", "no guidance", "from the tape".
    pub(crate) mark: String,
    /// A training game's: shown, never kept.
    pub(crate) training: bool,
}

/// The kept table, a line an entry: name, score, mark and the entry's bytes in hex,
/// tab-separated.
pub(crate) fn load(folder: &Path) -> Vec<Kept> {
    fs::read_to_string(folder.join(FILE))
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let fields: Vec<&str> = line.split('\t').collect();
                    Some(Kept {
                        entry: ScoreEntry {
                            name: (*fields.first()?).to_owned(),
                            score: (*fields.get(1)?).to_owned(),
                            bytes: fields.get(3).and_then(|h| from_hex(h)).unwrap_or_default(),
                        },
                        mark: fields.get(2).unwrap_or(&FROM_TAPE).to_string(),
                        training: false,
                    })
                })
                .filter(|k| k.mark != FROM_TAPE)
                .collect()
        })
        .unwrap_or_default()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Writes the table, leaving out training games' entries and the game's own.
///
/// # Errors
/// If the folder or file can't be written.
pub(crate) fn save(folder: &Path, table: &[Kept]) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let mut text = String::new();
    for k in table.iter().filter(|k| !k.training && k.mark != FROM_TAPE) {
        let _ = write!(text, "{}\t{}\t{}\t", k.entry.name, k.entry.score, k.mark);
        for b in &k.entry.bytes {
            let _ = write!(text, "{b:02X}");
        }
        text.push('\n');
    }
    fs::write(folder.join(FILE), text)
}

/// Puts the kept entries into the game's table, ranked by score among the game's own entries
/// as they are in memory (an equal score below the game's, as a game puts a new one), and
/// returns the table as it now stands. With nothing kept, memory is left as it is.
pub(crate) fn put_in(session: &mut Session, kept: &[Kept]) -> Vec<Kept> {
    let now = session.scores().unwrap_or_default();
    if !kept.is_empty() && !now.is_empty() {
        let mut others: Vec<ScoreEntry> = now.clone();
        for k in kept {
            if let Some(at) = others.iter().position(|e| e.same(&k.entry)) {
                others.remove(at);
            }
        }
        let mut theirs = others.into_iter().peekable();
        let mut ours = kept.iter().map(|k| k.entry.clone()).peekable();
        let ranked: Vec<ScoreEntry> = std::iter::from_fn(|| match (ours.peek(), theirs.peek()) {
            (Some(o), Some(t)) if value(&o.score) > value(&t.score) => ours.next(),
            (_, Some(_)) => theirs.next(),
            (Some(_), None) => ours.next(),
            (None, None) => None,
        })
        .take(now.len())
        .collect();
        session.set_scores(&ranked);
    }
    let now = session.scores().unwrap_or_default();
    after_game(kept, now, FROM_TAPE, false).0
}

/// A score as a number, to rank it: 0 when it isn't one.
fn value(score: &str) -> u64 {
    score.trim().parse().unwrap_or(0)
}

/// Adds the name of this game's entry to the definition's kept list of names entered, as its
/// newest, so the name card offers it first next time.
pub(crate) fn keep_name(session: &mut Session, table: &[Kept], mine: Option<usize>) {
    let list = session
        .definition()
        .and_then(|d| d.name_entry.as_ref())
        .and_then(|n| n.choices.clone());
    if let (Some(list), Some(k)) = (list, mine.and_then(|i| table.get(i))) {
        let name = k.entry.name.clone();
        session.keep_latest(&list, &name);
    }
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
            if let Some(at) = unmatched.iter().position(|k| k.entry.same(&entry)) {
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
                    mark: FROM_TAPE.to_owned(),
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
            bytes: Vec::new(),
        }
    }

    fn kept(name: &str, score: &str, mark: &str) -> Kept {
        Kept {
            entry: entry(name, score),
            mark: mark.to_owned(),
            training: false,
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
        let mut ann = entry("ANN", "12500");
        ann.bytes = vec![0x41, 0x00, 0xFF];
        let read = vec![ann, entry("BOB", "900")];
        let (table, _) = after_game(&[], read, "no guidance", false);
        save(folder.path(), &table).unwrap();
        assert_eq!(
            fs::read_to_string(folder.path().join(FILE)).unwrap(),
            "ANN\t12500\tno guidance\t4100FF\n",
            "BOB, the game's own, is left out"
        );
        assert_eq!(load(folder.path()), table[..1]);
        assert_eq!(mark(&Record::default(), None), "no guidance");
        let loaded = Record {
            level: 5,
            loaded: true,
            ..Record::default()
        };
        assert_eq!(mark(&loaded, Some("Routes")), "level 5 · Routes · loaded");
    }

    #[test]
    fn a_file_without_bytes_or_marks_still_reads() {
        let folder = tempfile::TempDir::new().unwrap();
        fs::write(
            folder.path().join(FILE),
            "ANN\t12500\tlevel 1 · Codes\nBOB\t900\nCAT\t50\tno guidance\tzz\n",
        )
        .unwrap();
        assert_eq!(
            load(folder.path()),
            [
                kept("ANN", "12500", "level 1 · Codes"),
                kept("CAT", "50", "no guidance")
            ],
            "unmarked is the tape's, so not kept; broken bytes are none"
        );
    }

    /// A stand-in game with a table of four at 0x8000, ten bytes an entry: a three-letter
    /// name, six digits, and a byte the game keeps beside them.
    const STAND_IN: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"
[[states]]
name = "name"
pc = 0x9000
[name_entry]
state = "name"
letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
length = 3
choices = "names"
[persist.names]
label = "names"
file = "names.txt"
[high_scores]
addr = 0x8000
count = 4
stride = 10
name = { offset = 0, len = 3, type = "text" }
score = { offset = 3, len = 6, type = "digits" }
"#;

    /// The stand-in game, its own table in memory.
    fn stand_in() -> Session {
        let definition = sidekick_def::Definition::from_toml(STAND_IN).unwrap();
        let machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        let mut s = Session::new(machine, Some(definition));
        for (i, own) in ["AAA000900", "BBB000500", "CCC000300", "DDD000100"]
            .iter()
            .enumerate()
        {
            let at = 0x8000 + 10 * u16::try_from(i).unwrap();
            for (a, b) in (at..).zip(own.bytes().chain([u8::try_from(i).unwrap() + 1])) {
                s.machine_mut().write(a, b);
            }
        }
        s
    }

    fn memory(s: &Session) -> Vec<u8> {
        (0x8000..0x8028).map(|a| s.machine().peek(a)).collect()
    }

    fn names(s: &Session) -> Vec<String> {
        s.scores().unwrap().into_iter().map(|e| e.name).collect()
    }

    #[test]
    fn with_nothing_kept_the_game_s_own_entries_stay_and_aren_t_kept() {
        let mut s = stand_in();
        let before = memory(&s);
        let table = put_in(&mut s, &[]);
        assert_eq!(memory(&s), before, "memory left alone");
        assert_eq!(
            table
                .iter()
                .map(|k| k.entry.name.as_str())
                .collect::<Vec<_>>(),
            ["AAA", "BBB", "CCC", "DDD"],
            "entries read from where the definition says"
        );
        assert!(table.iter().all(|k| k.mark == FROM_TAPE));
        let folder = tempfile::TempDir::new().unwrap();
        save(folder.path(), &table).unwrap();
        assert!(load(folder.path()).is_empty(), "the game's own never kept");
    }

    #[test]
    fn kept_entries_go_in_ranked_among_the_game_s_own_whole() {
        let mut s = stand_in();
        let mut you = kept("YOU", "600", "no guidance");
        you.entry.bytes = b"YOU000600*".to_vec();
        let tie = kept("TIE", "500", "level 2 · Map");
        let table = put_in(&mut s, &[you, tie]);
        assert_eq!(
            names(&s),
            ["AAA", "YOU", "BBB", "TIE"],
            "ranked by score, an equal one under the game's, the lowest pushed off"
        );
        assert_eq!(memory(&s)[10..20], *b"YOU000600*", "with its bytes, whole");
        assert_eq!(
            memory(&s)[20..30],
            *b"BBB000500\x02",
            "the game's own moved whole"
        );
        assert_eq!(
            &memory(&s)[30..39],
            b"TIE000500",
            "without bytes, its fields"
        );
        let marks: Vec<&str> = table.iter().map(|k| k.mark.as_str()).collect();
        assert_eq!(
            marks,
            [FROM_TAPE, "no guidance", FROM_TAPE, "level 2 · Map"]
        );
        let kept_now: Vec<Kept> = table
            .iter()
            .filter(|k| k.mark != FROM_TAPE)
            .cloned()
            .collect();
        let again = put_in(&mut s, &kept_now);
        assert_eq!(
            again, table,
            "put in again, as after a load: nothing doubled"
        );
    }

    #[test]
    fn the_name_of_this_game_s_entry_is_kept_for_the_card() {
        let mut s = stand_in();
        let table = put_in(&mut s, &[]);
        keep_name(&mut s, &table, None);
        assert!(
            s.kept().get("names").is_none_or(Vec::is_empty),
            "no entry, no name"
        );
        keep_name(&mut s, &table, Some(2));
        keep_name(&mut s, &table, Some(0));
        keep_name(&mut s, &table, Some(2));
        assert_eq!(
            s.kept()["names"],
            ["AAA", "CCC"],
            "once, the latest last, as the card offers it first"
        );
    }

    /// Starquake's definition, and a session of it at its title, when the player's tape is in
    /// `assets/`.
    fn starquake_at_title() -> Option<Session> {
        use sidekick_core::{Key, Machine, Model};
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zip = repo.join("assets/Starquake.tap.zip");
        if !zip.exists() {
            return None;
        }
        let tapes = sidekick_def::tapes_in(&zip).expect("the zip holds a tape");
        let package = sidekick_def::Package::load(&repo.join("games/starquake")).unwrap();
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
        machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
        let mut s = Session::new(machine, Some(package.definition));
        until(&mut s, 400, |_| false);
        press(&mut s, Key::Space);
        assert!(until(&mut s, 1500, |s| s.state() == Some("title")));
        Some(s)
    }

    fn until(s: &mut Session, most: usize, done: impl Fn(&Session) -> bool) -> bool {
        (0..most).any(|_| {
            s.run_frame().expect("a frame");
            done(s)
        })
    }

    fn press(s: &mut Session, key: sidekick_core::Key) {
        s.machine_mut().key(key, true);
        until(s, 4, |_| false);
        s.machine_mut().key(key, false);
        until(s, 4, |_| false);
    }

    /// Starts a game from the title, plays its end-game action, and waits for the name.
    fn to_the_name(s: &mut Session, score: i64) {
        use sidekick_core::Key;
        let definition = s.definition().unwrap().clone();
        press(s, Key::N1);
        let started = (0..10).any(|_| {
            press(s, Key::N0);
            until(s, 200, |s| s.state() == Some("play"))
        });
        assert!(started, "a game starts");
        until(s, 50, |_| false);
        s.write("score", score).expect("the score is writable");
        let end = definition
            .actions
            .iter()
            .find(|a| a.id == "end-game")
            .unwrap();
        let keys: Vec<_> = end
            .tap
            .iter()
            .filter_map(|k| crate::keys::key_named(k))
            .collect();
        for &k in &keys {
            s.machine_mut().key(k, true);
        }
        until(s, end.frames.map_or(5, |f| f as usize), |_| false);
        for &k in &keys {
            s.machine_mut().key(k, false);
        }
        let name = definition.name_entry.as_ref().unwrap().state.clone();
        assert!(
            until(s, 3000, |s| s.state() == Some(name.as_str())),
            "the game asks for a name"
        );
    }

    /// Types the word the card entered, at the definition's timings.
    fn type_word(s: &mut Session, word: &str) {
        use crate::entry::{Stroke, Typing};
        let name_entry = s.definition().unwrap().name_entry.clone().unwrap();
        let mut typing = Typing::new(word, &name_entry);
        while let Some(stroke) = typing.next() {
            match stroke {
                Stroke::Press(k) => s.machine_mut().key(k, true),
                Stroke::Release(k) => s.machine_mut().key(k, false),
                Stroke::Wait => {}
            }
            s.run_frame().unwrap();
        }
    }

    /// Runs until the game ends with its name in, before the title comes round.
    fn until_named(s: &mut Session) {
        let ended = (0..300).any(|_| {
            s.run_frame().unwrap();
            s.take_milestones()
                .contains(&sidekick_core::Milestone::GameEnded)
        });
        assert!(ended, "the game takes the name");
        assert_ne!(s.state(), Some("title"), "before the title");
    }

    #[test]
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn on_the_tape_a_score_named_with_the_card_is_kept_at_once_and_goes_back_in() {
        let Some(mut s) = starquake_at_title() else {
            eprintln!("skipped: no assets/Starquake.tap.zip (the player's own tape)");
            return;
        };
        assert_eq!(s.take_milestones(), [sidekick_core::Milestone::FirstTitle]);
        // The run as it was at the title: a save loaded before the next title.
        let mut loaded = s.clone();
        let own = s.scores().expect("Starquake's table");
        let table = put_in(&mut s, &[]);
        assert_eq!(
            s.scores().as_ref(),
            Some(&own),
            "nothing kept: memory left alone"
        );
        assert!(table.iter().all(|k| k.mark == FROM_TAPE));

        // A score that beats the table's lowest, then a past name chosen on the card.
        let lowest = value(&own.last().unwrap().score);
        to_the_name(&mut s, i64::try_from(lowest).unwrap() + 1);
        let name_entry = s.definition().unwrap().name_entry.clone().unwrap();
        let list = name_entry.choices.clone().unwrap();
        let names = std::collections::BTreeMap::from([(list.clone(), vec!["ZOE".to_owned()])]);
        let mut card = crate::entry::Entry::open(&name_entry, crate::entry::Card::Name, &names, 0);
        let word = card.input(crate::entry::Input::Confirm);
        type_word(&mut s, &word.expect("the name chosen, entered"));
        until_named(&mut s);

        let read = s.scores().unwrap();
        let (after, mine) = after_game(&table, read.clone(), "no guidance", false);
        let at = mine.expect("this game's entry");
        assert_eq!(after[at].entry.name, "ZOE", "the name the card typed");
        assert_eq!(after[at].mark, "no guidance");
        keep_name(&mut s, &after, mine);
        assert_eq!(s.kept()[&list], ["ZOE"]);
        let folder = tempfile::TempDir::new().unwrap();
        save(folder.path(), &after).unwrap();
        let kept = load(folder.path());
        assert_eq!(kept.len(), 1, "only this game's: {kept:?}");

        // Loaded before the title: the kept entry goes back in, ranked, the game's own
        // around it.
        let table = put_in(&mut loaded, &kept);
        assert_eq!(loaded.scores(), Some(read), "the table as the game left it");
        assert_eq!(table[at].mark, "no guidance");
        assert!(
            until(&mut s, 3000, |s| s.state() == Some("title")),
            "the game goes on to its title"
        );
        assert!(s.take_milestones().is_empty(), "read back once a game");
    }

    #[test]
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn on_the_tape_a_name_with_a_space_spelled_on_the_card_lands_in_the_table() {
        use crate::entry::{Card, Entry, Input};
        let Some(mut s) = starquake_at_title() else {
            eprintln!("skipped: no assets/Starquake.tap.zip (the player's own tape)");
            return;
        };
        s.take_milestones();
        let table = put_in(&mut s, &[]);
        let lowest = value(&s.scores().unwrap().last().unwrap().score);
        to_the_name(&mut s, i64::try_from(lowest).unwrap() + 1);
        let name_entry = s.definition().unwrap().name_entry.clone().unwrap();
        let mut card = Entry::open(
            &name_entry,
            Card::Name,
            &std::collections::BTreeMap::new(),
            0,
        );
        // The first letter, then down to the last (the space), then the first again.
        let word = [Input::Up, Input::Confirm, Input::Down, Input::Confirm]
            .into_iter()
            .chain([Input::Up, Input::Confirm])
            .find_map(|input| card.input(input))
            .expect("entered at the last place");
        let first = name_entry.letters.chars().next().unwrap();
        assert_eq!(word, format!("{first} {first}"));
        type_word(&mut s, &word);
        until_named(&mut s);
        let (after, mine) = after_game(&table, s.scores().unwrap(), "no guidance", false);
        assert_eq!(after[mine.expect("this game's entry")].entry.name, word);
    }
}
