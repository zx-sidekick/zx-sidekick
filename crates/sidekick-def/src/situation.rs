//! Crafted situations: what differs from a new game, for testing.

use crate::{Definition, Problem};
use std::collections::BTreeMap;

/// A crafted situation: `name = value` lines for the definition's writable values,
/// `cheats = [..]` for training switches, and `kept.<list> = [..]` for kept-list entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Situation {
    /// The values to write, by name.
    pub values: BTreeMap<String, i64>,
    /// The training switches to turn on, by id.
    pub cheats: Vec<String>,
    /// The entries to add to kept lists, by list.
    pub kept: BTreeMap<String, Vec<String>>,
}

impl Situation {
    /// Reads a crafted situation's TOML.
    ///
    /// # Errors
    /// If it isn't TOML, or a line isn't a whole number, a list of switches, or kept lists.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
        let mut situation = Self::default();
        for (name, value) in table {
            match (name.as_str(), value) {
                ("cheats", toml::Value::Array(ids)) => situation.cheats = words(&name, ids)?,
                ("kept", toml::Value::Table(lists)) => {
                    for (list, entries) in lists {
                        let path = format!("kept.{list}");
                        let toml::Value::Array(entries) = entries else {
                            return Err(format!("{path}: a list of entries"));
                        };
                        situation.kept.insert(list, words(&path, entries)?);
                    }
                }
                ("cheats" | "kept", _) => {
                    return Err(format!("{name}: a list (cheats) or lists (kept.<list>)"));
                }
                (_, toml::Value::Integer(number)) => {
                    situation.values.insert(name, number);
                }
                _ => return Err(format!("{name}: a whole number")),
            }
        }
        Ok(situation)
    }

    /// What's wrong with it for `def`: names it doesn't know, values not writable, switches
    /// and lists it doesn't have, or no `[situations]` to enter play with.
    #[must_use]
    pub fn check(&self, def: &Definition) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut report = |path: String, message: &str| {
            problems.push(Problem {
                path,
                message: message.to_owned(),
            });
        };
        if def.situations.as_ref().and_then(|s| s.enter).is_none() {
            report(
                "situations.enter".to_owned(),
                "the definition doesn't say how a situation enters play",
            );
        }
        for name in self.values.keys() {
            match def.values.get(name) {
                None => report(name.clone(), "no such value"),
                Some(value) if !value.writable => report(name.clone(), "not writable"),
                Some(_) => {}
            }
        }
        for id in &self.cheats {
            if !def.cheats.iter().any(|c| &c.id == id) {
                report(format!("cheats.{id}"), "no such training switch");
            }
        }
        for list in self.kept.keys() {
            if !def.persist.contains_key(list) {
                report(format!("kept.{list}"), "no such kept list");
            }
        }
        problems
    }
}

/// A TOML array of strings.
fn words(path: &str, items: Vec<toml::Value>) -> Result<Vec<String>, String> {
    items
        .into_iter()
        .map(|item| match item {
            toml::Value::String(word) => Ok(word),
            _ => Err(format!("{path}: text only")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"
sha1 = "a9993e364706816aba3e25717850c26c9cd0d89d"

[values.lives]
addr = 0x8000
type = "u8"
writable = true

[values.score]
addr = 0x8001
type = "u8"

[[cheats]]
id = "no-harm"
label = "No harm"
patch = [{ addr = 0x8010, bytes = "00", expect = "CD" }]

[persist.codes]
label = "codes"
file = "codes.txt"

[situations]
enter = 0x6400
start = ["SPACE", "0"]
"#;

    #[test]
    fn a_crafted_file_is_read_by_its_names() {
        let s = Situation::from_toml(
            "lives = 9\ncheats = [\"no-harm\"]\nkept.codes = [\"TOPAZ\", \"RUBY\"]\n",
        )
        .unwrap();
        assert_eq!(s.values, BTreeMap::from([("lives".to_owned(), 9)]));
        assert_eq!(s.cheats, ["no-harm"]);
        assert_eq!(s.kept["codes"], ["TOPAZ", "RUBY"]);
        let def = Definition::from_toml(DEFINITION).unwrap();
        assert_eq!(s.check(&def), [], "{:?}", s.check(&def));
        assert!(def.check().is_empty(), "{:?}", def.check());
    }

    #[test]
    fn what_the_definition_doesn_t_have_is_reported() {
        let def = Definition::from_toml(DEFINITION).unwrap();
        let s =
            Situation::from_toml("score = 1\nroom = 3\ncheats = [\"fly\"]\nkept.maps = [\"A\"]\n")
                .unwrap();
        let paths: Vec<String> = s.check(&def).into_iter().map(|p| p.path).collect();
        assert_eq!(paths, ["room", "score", "cheats.fly", "kept.maps"]);
        assert!(Situation::from_toml("lives = \"many\"\n").is_err());
        assert!(Situation::from_toml("cheats = 3\n").is_err());
    }
}
