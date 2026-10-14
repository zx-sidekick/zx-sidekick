//! ZX Sidekick's game definitions, format 1: what the program knows about one game.
//!
//! A definition is read from TOML or JSON into a [`Definition`], which accepts a draft (an
//! address not found yet may be missing). [`Definition::check`] then lists every problem with
//! its path, so a draft parses and says what's left. `docs/definition-format.md` is the
//! reference; [`schema`] is its JSON Schema.

mod check;
mod library;
pub mod pok;
mod scalars;
mod sections;
mod situation;

pub use check::Problem;
pub use library::{tapes_in, Identified, Library, LoadError, LoadProblem, Package, Tape};
pub use scalars::{Address, Bytes, OneOrMany};
pub use sections::*;
pub use situation::Situation;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};

/// The format this crate reads.
pub const SCHEMA: u32 = 1;

/// A game definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "ZX Sidekick game definition, format 1")]
pub struct Definition {
    /// The format, 1.
    pub schema: u32,
    /// Which game this is.
    pub game: Game,
    /// The dumps supported.
    pub media: Vec<Media>,
    /// How the game is run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<Run>,
    /// Moments of the game, checked in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<State>,
    /// Named facts about the game's memory.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, Value>,
    /// Training switches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cheats: Vec<Cheat>,
    /// What a pad does, by state (`default` for every state).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pad: BTreeMap<String, Pad>,
    /// Things a button or key can do.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// Typing a word the game asks for, with a pad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_entry: Option<TextEntry>,
    /// Typing the name for a high score with a pad: `text_entry`'s keys, for a second card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_entry: Option<TextEntry>,
    /// Lists kept between games.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub persist: BTreeMap<String, Persist>,
    /// The game's high-score table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_scores: Option<HighScores>,
    /// The guidance levels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance: Option<Guidance>,
    /// The panel's widgets, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panel: Vec<Widget>,
    /// Hints, a line in the panel while their conditions hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<PanelHint>,
    /// The How to play screen: the goal, and the main moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<About>,
    /// How a crafted situation enters play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub situations: Option<Situations>,
    /// The definition's script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<Script>,
}

/// A definition that couldn't be read.
#[derive(Debug)]
pub enum Error {
    /// The TOML isn't a definition; the message says where.
    Toml(toml::de::Error),
    /// The JSON isn't a definition; the message says where.
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Toml(e) => Some(e),
            Self::Json(e) => Some(e),
        }
    }
}

impl Definition {
    /// The name of the state with `role`, if one has it.
    #[must_use]
    pub fn state_for(&self, role: Role) -> Option<&str> {
        self.states
            .iter()
            .find(|s| s.role == Some(role))
            .map(|s| s.name.as_str())
    }

    /// Whether `state` is the state with `role`.
    #[must_use]
    pub fn is(&self, state: Option<&str>, role: Role) -> bool {
        state.is_some_and(|s| self.state_for(role) == Some(s))
    }

    /// Reads a definition from TOML.
    ///
    /// # Errors
    /// If it isn't valid TOML, or isn't a definition's shape (an unknown key, a wrong type).
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        toml::from_str(text).map_err(Error::Toml)
    }

    /// Reads a definition from JSON.
    ///
    /// # Errors
    /// If it isn't valid JSON, or isn't a definition's shape.
    pub fn from_json(text: &str) -> Result<Self, Error> {
        serde_json::from_str(text).map_err(Error::Json)
    }

    /// The definition as JSON, pretty-printed.
    ///
    /// # Panics
    /// Never: every part of a definition serializes.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a definition always serializes to JSON")
    }

    /// The definition as TOML.
    ///
    /// # Panics
    /// Never: every part of a definition serializes.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("a definition always serializes to TOML")
    }

    /// Every problem that keeps this definition from being complete and consistent, each with
    /// its path. Empty when it's ready to use.
    #[must_use]
    pub fn check(&self) -> Vec<Problem> {
        check::check(self)
    }
}

/// The JSON Schema of format 1, pretty-printed, as committed at `schema/definition.schema.json`.
///
/// # Panics
/// Never: the schema always serializes.
#[must_use]
pub fn schema() -> String {
    let schema = schemars::schema_for!(Definition);
    serde_json::to_string_pretty(&schema).expect("the schema always serializes") + "\n"
}
