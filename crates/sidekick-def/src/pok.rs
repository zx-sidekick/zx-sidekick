//! `.pok` files, the POKE format Fuse and ZXDB use, read and written as `[[cheats]]`.
//!
//! A file is a list of trainers. `N` starts one with its name; each `M` line after it is a
//! poke (bank, address, value, the original byte), and a `Z` line is its last poke. `Y` ends
//! the file. Bank 8 is no bank; a value of 256 means the player is asked for one.

use std::{collections::BTreeSet, fmt};

/// A trainer: a name and its pokes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trainer {
    /// Its name, as the file gives it.
    pub name: String,
    /// What it writes.
    pub pokes: Vec<Poke>,
}

/// One byte a trainer writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Poke {
    /// On a 128K, the RAM bank; `None` for any.
    pub bank: Option<u8>,
    /// Where.
    pub addr: u16,
    /// What; `None` when the player is asked.
    pub value: Option<u8>,
    /// The byte there before.
    pub original: u8,
}

/// Why a `.pok` file couldn't be read: the line, and what's wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PokError {
    /// The line, from 1.
    pub line: usize,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for PokError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for PokError {}

/// Reads a `.pok` file's trainers.
///
/// # Errors
/// A poke before any name, a line that isn't `N`, `M`, `Z` or `Y`, or a number out of range.
pub fn read(text: &str) -> Result<Vec<Trainer>, PokError> {
    let mut trainers: Vec<Trainer> = Vec::new();
    let mut open = false;
    for (i, line) in text.lines().enumerate() {
        let wrong = |message: &str| PokError {
            line: i + 1,
            message: message.to_owned(),
        };
        let line = line.trim_end();
        let (kind, rest) = line.split_at(line.len().min(1));
        match kind {
            "" => {}
            "Y" => break,
            "N" => {
                trainers.push(Trainer {
                    name: rest.trim().to_owned(),
                    pokes: Vec::new(),
                });
                open = true;
            }
            "M" | "Z" => {
                let trainer = trainers
                    .last_mut()
                    .filter(|_| open)
                    .ok_or_else(|| wrong("a poke before a trainer's name (N)"))?;
                trainer.pokes.push(poke(rest).map_err(|m| wrong(&m))?);
                open = kind == "M";
            }
            other => return Err(wrong(&format!("{other:?} isn't N, M, Z or Y"))),
        }
    }
    Ok(trainers)
}

/// A poke from its four numbers: bank, address, value, original.
fn poke(rest: &str) -> Result<Poke, String> {
    let numbers = rest
        .split_whitespace()
        .map(|n| {
            n.parse::<u32>()
                .map_err(|_| format!("{n:?} isn't a number"))
        })
        .collect::<Result<Vec<u32>, String>>()?;
    let [bank, addr, value, original] = numbers[..] else {
        return Err("a poke is a bank, an address, a value and the original byte".to_owned());
    };
    let byte = |n: u32, what: &str| u8::try_from(n).map_err(|_| format!("{what} {n} isn't a byte"));
    Ok(Poke {
        bank: match bank {
            8 => None,
            0..=7 => Some(byte(bank, "bank")?),
            _ => return Err(format!("bank {bank} isn't 0 to 8")),
        },
        addr: u16::try_from(addr).map_err(|_| format!("address {addr} is past 65535"))?,
        value: if value == 256 {
            None
        } else {
            Some(byte(value, "value")?)
        },
        original: byte(original, "original")?,
    })
}

/// The trainers as `[[cheats]]` TOML, each a switch with a `patch` for each poke. A trainer
/// that asks the player for a value is left out, with a comment saying so.
#[must_use]
pub fn cheats_toml(trainers: &[Trainer]) -> String {
    let mut blocks: Vec<String> = Vec::new();
    let mut ids = BTreeSet::new();
    for trainer in trainers {
        if trainer.pokes.iter().any(|p| p.value.is_none()) {
            blocks.push(format!(
                "# {}: asks the player for a value, so isn't imported\n\n",
                trainer.name
            ));
            continue;
        }
        let base = id(&trainer.name);
        let mut id = base.clone();
        let mut n = 2;
        while !ids.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }
        let patches: Vec<String> = trainer
            .pokes
            .iter()
            .map(|p| {
                let bank = p.bank.map_or_else(String::new, |b| format!(", bank = {b}"));
                format!(
                    "  {{ addr = 0x{:04X}{bank}, bytes = \"{:02X}\", expect = \"{:02X}\" }},",
                    p.addr,
                    p.value.unwrap_or_default(),
                    p.original
                )
            })
            .collect();
        blocks.push(format!(
            "[[cheats]]\nid = \"{id}\"\nlabel = \"{}\"\npatch = [\n{}\n]\n\n",
            trainer.name.replace('\\', "\\\\").replace('"', "\\\""),
            patches.join("\n")
        ));
    }
    blocks.concat()
}

/// A switch's id from a trainer's name: lower case, words joined by hyphens.
fn id(name: &str) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if words.is_empty() {
        "trainer".to_owned()
    } else {
        words.join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Definition;

    const POK: &str = "\
NInfinite lives
Z 8 36731 0 53
NInfinite time
M 8 40000 201 58
Z 8 40001 0 50
NStart level
Z 8 30000 256 1
NInfinite lives
Z 3 49152 0 1
Y
";

    #[test]
    fn a_pok_file_s_trainers_are_read() {
        let trainers = read(POK).unwrap();
        assert_eq!(trainers.len(), 4);
        assert_eq!(trainers[0].name, "Infinite lives");
        assert_eq!(
            trainers[0].pokes,
            [Poke {
                bank: None,
                addr: 36731,
                value: Some(0),
                original: 53
            }]
        );
        assert_eq!(trainers[1].pokes.len(), 2);
        assert_eq!(trainers[2].pokes[0].value, None, "256 asks the player");
        assert_eq!(trainers[3].pokes[0].bank, Some(3));
    }

    #[test]
    fn a_broken_pok_file_says_where() {
        assert_eq!(read("M 8 1 2 3\n").unwrap_err().line, 1);
        let wrong = read("NLives\nZ 8 70000 0 0\n").unwrap_err();
        assert_eq!((wrong.line, wrong.message.contains("65535")), (2, true));
        assert!(read("NLives\nQ\n").is_err());
        assert!(read("NLives\nZ 8 1 2\n").is_err(), "three numbers");
        assert!(
            read("NOne\nZ 8 1 2 3\nZ 8 1 2 3\n").is_err(),
            "a poke after the last"
        );
    }

    #[test]
    fn trainers_become_switches_that_check_clean() {
        let toml = cheats_toml(&read(POK).unwrap());
        assert!(toml.contains("id = \"infinite-lives\""));
        assert!(toml.contains("id = \"infinite-lives-2\""), "ids kept apart");
        assert!(toml.contains("{ addr = 0x8F7B, bytes = \"00\", expect = \"35\" }"));
        assert!(toml.contains("bank = 3"));
        assert!(toml.contains("# Start level: asks the player for a value"));
        assert!(!toml.contains("id = \"start-level\""));

        let definition = format!(
            "schema = 1\n[game]\nid = \"t\"\ntitle = \"T\"\nmachine = \"128k\"\n[[media]]\nname = \"t\"\nsha1 = \"a9993e364706816aba3e25717850c26c9cd0d89d\"\n\n{toml}"
        );
        let def = Definition::from_toml(&definition).expect("it parses");
        assert_eq!(def.cheats.len(), 3);
        let problems: Vec<String> = def.check().iter().map(ToString::to_string).collect();
        assert!(
            !problems.iter().any(|p| p.starts_with("cheats")),
            "{problems:#?}"
        );
    }
}
