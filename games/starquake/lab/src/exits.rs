//! The file the whole-map search writes into the assets folder, and the
//! comparisons read: every room it reached and every exit it found.

use std::collections::BTreeMap;

use crate::search::Exit;

/// One room the search reached.
#[derive(Clone, Debug, Default)]
pub struct Record {
    /// Whether a door, teleport or pyramid screen opened somewhere in it.
    pub modal: bool,
    pub exits: Vec<Exit>,
}

/// The whole file, by room.
pub type Dump = BTreeMap<u16, Record>;

/// The file's name in the assets folder.
pub const FILE: &str = "exits.txt";

/// Writes the dump as text: `room N modal 0|1` then `  exit N -> M at (x,y)`.
#[must_use]
pub fn write(dump: &Dump) -> String {
    let mut out = String::new();
    for (room, record) in dump {
        out += &format!("room {room} modal {}\n", u8::from(record.modal));
        let mut exits = record.exits.clone();
        exits.sort_unstable();
        for (to, x, y) in exits {
            out += &format!("  exit {room} -> {to} at ({x},{y})\n");
        }
    }
    out
}

/// Reads a dump written by [`write()`]. Lines it does not understand are
/// skipped.
#[must_use]
pub fn read(text: &str) -> Dump {
    let mut dump = Dump::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("room ") {
            let mut it = rest.split_whitespace();
            let Some(room) = it.next().and_then(|r| r.parse().ok()) else {
                continue;
            };
            let modal = it.nth(1) == Some("1");
            dump.entry(room).or_default().modal = modal;
        } else if let Some(rest) = line.trim().strip_prefix("exit ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let [from, _, to, _, xy] = parts[..] else {
                continue;
            };
            let xy = xy.trim_matches(|c| c == '(' || c == ')');
            let Some((x, y)) = xy.split_once(',') else {
                continue;
            };
            let (Ok(from), Ok(to), Ok(x), Ok(y)) = (from.parse(), to.parse(), x.parse(), y.parse())
            else {
                continue;
            };
            dump.entry(from).or_default().exits.push((to, x, y));
        }
    }
    dump
}

/// Every entry the dump records into `room`: the positions Blob settled at
/// after crossing in, each once.
#[must_use]
pub fn entries(dump: &Dump, room: u16) -> Vec<(u8, u8)> {
    let mut out: Vec<(u8, u8)> = Vec::new();
    for record in dump.values() {
        for &(to, x, y) in &record.exits {
            if to == room && !out.contains(&(x, y)) {
                out.push((x, y));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dump_round_trips() {
        let mut dump = Dump::new();
        dump.insert(
            8,
            Record {
                modal: true,
                exits: vec![(9, 0, 39), (24, 40, 143)],
            },
        );
        dump.insert(9, Record::default());
        let text = write(&dump);
        let back = read(&text);
        assert_eq!(back.len(), 2);
        assert!(back[&8].modal && !back[&9].modal);
        assert_eq!(back[&8].exits, vec![(9, 0, 39), (24, 40, 143)]);
        assert_eq!(entries(&back, 9), vec![(0, 39)]);
        assert!(entries(&back, 10).is_empty());
    }
}
