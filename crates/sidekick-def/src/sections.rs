//! The sections of a definition, as `docs/definition-format.md` describes them.

use crate::scalars::{Address, Bytes, OneOrMany};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[game]`: which game this is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// Unique, lowercase letters, digits, `-` and `_`: names the data folder and the file.
    pub id: String,
    /// In plain text, as the game is known.
    pub title: String,
    /// The year it was published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    /// Who published it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// Who wrote it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// The Spectrum it runs on.
    pub machine: Machine,
    /// Replaces the standard "not affiliated" notice; keep its meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

/// The Spectrum a game runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Machine {
    /// The 48K.
    #[serde(rename = "48k")]
    Spectrum48K,
    /// The 128K.
    #[serde(rename = "128k")]
    Spectrum128K,
}

/// `[[media]]`: a dump the definition supports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Media {
    /// How the tape prompt names the dump.
    pub name: String,
    /// The SHA-1 of the tape file itself (not a zip around it), 40 hex digits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    /// The tape's format.
    #[serde(default, skip_serializing_if = "TapeFormat::is_tap")]
    pub format: TapeFormat,
    /// File names it's distributed under, matched in any case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// Where the player can get it legally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<String>,
    /// The download link's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_label: Option<String>,
}

/// A tape's format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TapeFormat {
    /// `.tap`.
    #[default]
    Tap,
    /// `.tzx`.
    Tzx,
}

impl TapeFormat {
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's skip_serializing_if passes a reference"
    )]
    const fn is_tap(&self) -> bool {
        matches!(self, Self::Tap)
    }
}

/// `[run]`: how the game is run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// The top of the game's main loop, reached once per frame of play.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<Address>,
    /// Whether the loading screen stays until a key is pressed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hold_loading_screen: bool,
    /// What the pad's directions drive in play.
    #[serde(default)]
    pub joystick: Joystick,
    /// With `joystick = "keys"`: the game's own keys for each direction and fire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<DirectionKeys>,
}

/// A game's own keys for the joystick's directions and fire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectionKeys {
    /// Up.
    pub up: String,
    /// Down.
    pub down: String,
    /// Left.
    pub left: String,
    /// Right.
    pub right: String,
    /// Fire.
    pub fire: String,
}

/// What a pad's directions drive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Joystick {
    /// The Kempston joystick.
    #[default]
    Kempston,
    /// Sinclair joystick 1 (keys 6–0).
    Sinclair1,
    /// Sinclair joystick 2 (keys 1–5).
    Sinclair2,
    /// The cursor keys (5–8, 0 to fire).
    Cursor,
    /// The game's own keys, named in `[run] keys`.
    Keys,
}

/// `[[states]]`: a moment of the game. The first that matches is the game's state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct State {
    /// Its name, the definition's own.
    pub name: String,
    /// What it means to the program, if anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// Matches if the processor executed one of these during the frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pc: Option<OneOrMany<Address>>,
    /// Matches if this test of a value holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<Test>,
    /// For the `paused` role: what the pause notice says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notice: Option<Notice>,
}

/// What a state means to the program. At most one state has each.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The title screen: the kept high scores go in, and a game in hand ends.
    Title,
    /// A game being played: training switches apply, a game's record starts, and a crafted
    /// situation enters.
    Play,
    /// Paused: the picture dims, and the notice shows.
    Paused,
    /// Game over: no game is under way.
    GameOver,
}

impl Role {
    /// The role's name, as a definition writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Play => "play",
            Self::Paused => "paused",
            Self::GameOver => "game_over",
        }
    }
}

/// What a card over the picture says: a line, and hints along its bottom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Notice {
    /// The line: "Move or fire to go on."
    pub text: String,
    /// What to press, along the bottom.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
}

/// One thing to press, and what it does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Hint {
    /// The keyboard's key as shown (`"Alt"`), or arrows (`"← ↑ → ↓"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<String>,
    /// The pad part that does it, by position: `west`, `dpad`…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad: Option<String>,
    /// What it does: "fire".
    pub does: String,
}

/// A test of a value, or a combination of tests.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Test {
    /// The value tested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// It equals this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equals: Option<i64>,
    /// It doesn't equal this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not: Option<i64>,
    /// It's one of these.
    #[serde(default, rename = "in", skip_serializing_if = "Vec::is_empty")]
    pub one_of: Vec<i64>,
    /// It's at least this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_least: Option<i64>,
    /// It's at most this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_most: Option<i64>,
    /// All these bits are set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bits: Option<u64>,
    /// All of these tests hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all: Vec<Self>,
    /// Any of these tests holds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any: Vec<Self>,
}

/// `[values.*]`: a named fact about the game's memory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Value {
    /// What kind of value it is.
    #[serde(rename = "type")]
    pub kind: ValueType,
    /// Where it starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Address>,
    /// On a 128K, the RAM bank (0–7) for an address from 0xC000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank: Option<u8>,
    /// Its length in bytes, for the multi-byte types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<u32>,
    /// Its full amount, for meters and cheats that fill it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    /// Whether cheats, scripts and crafted situations may write it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub writable: bool,
    /// For `digits`: the code of the digit 0 (0x30 when not given).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zero: Option<u8>,
    /// For `text`: how codes map to letters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charset: Option<Charset>,
    /// For tables, fonts and sprites: how many there are, or the value that says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<Count>,
    /// For tables and sprites: bytes from one to the next.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stride: Option<u32>,
    /// For tables: each record's fields.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, Field>,
    /// For fonts: the code of the first character.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<u8>,
    /// For fonts and sprites: the width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// For fonts and sprites: the height in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// For sprites: where their masks are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<Mask>,
    /// For sprites: the order of their bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    /// For sprites: a value giving each picture's ink.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<ValueRef>,
    /// What it is, for authors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// What kind of value a value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    /// One unsigned byte.
    U8,
    /// One signed byte.
    I8,
    /// A little-endian word.
    U16,
    /// A big-endian word.
    U16be,
    /// `len` bytes, two decimal digits each.
    Bcd,
    /// `len` bytes, one digit each, counting from `zero`.
    Digits,
    /// `len` bytes of bits; bit 0 of the first byte is item 0.
    Bitset,
    /// `len` raw bytes.
    Bytes,
    /// `len` bytes of text in `charset`.
    Text,
    /// Records of `fields`, `count` of them, `stride` apart.
    Table,
    /// The game's font.
    Font,
    /// The game's pictures.
    Sprites,
}

impl ValueType {
    /// Whether the type needs a `len`.
    #[must_use]
    pub const fn has_len(self) -> bool {
        matches!(
            self,
            Self::Bcd | Self::Digits | Self::Bitset | Self::Bytes | Self::Text
        )
    }

    /// Whether the type is one number, which cheats can set, hold and fill.
    #[must_use]
    pub const fn is_number(self) -> bool {
        matches!(
            self,
            Self::U8 | Self::I8 | Self::U16 | Self::U16be | Self::Bcd | Self::Digits
        )
    }
}

/// How a `text` value's codes map to letters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Charset {
    /// `"ascii"`.
    Named(NamedCharset),
    /// A table of the game's own.
    Table {
        /// The code of the first letter in `chars`.
        first: u8,
        /// The letters, in code order.
        chars: String,
    },
}

/// A charset with a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NamedCharset {
    /// ASCII.
    Ascii,
}

/// How many: a number, or the value that holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Count {
    /// A fixed number.
    Fixed(u32),
    /// The value that holds it.
    Value(String),
}

/// A field of a table's records, or of a high-score entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// Its offset within the record.
    pub offset: u32,
    /// What kind of value it is.
    #[serde(rename = "type")]
    pub kind: ValueType,
    /// Its length, for the multi-byte types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub len: Option<u32>,
    /// Only these bits: true when they're set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bits: Option<u8>,
    /// For `text`: how codes map to letters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charset: Option<Charset>,
}

/// The order of a picture's bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Row by row, top to bottom, each row left to right.
    #[default]
    Rows,
    /// In 8 × 8 character cells, left to right then top to bottom, each cell's eight rows in
    /// turn, as the screen's characters are drawn.
    Cells,
}

/// Where sprites keep their masks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Mask {
    /// No mask.
    None,
    /// A mask byte before each byte.
    Interleaved,
    /// The mask after the picture.
    After,
}

/// A reference to a value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueRef {
    /// The value's name.
    pub value: String,
}

/// `[[cheats]]`: a training switch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cheat {
    /// Its name.
    pub id: String,
    /// Its text in the menu.
    pub label: String,
    /// Values written at each sync point while it's on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<OneOrMany<Set>>,
    /// Values that may not go down while it's on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold: Option<OneOrMany<String>>,
    /// Patches applied while it's on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patch: Vec<Patch>,
    /// Whether a game played with it on is a training game.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub marks_score: bool,
    /// Why and how it works.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A value to set, and what to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Set {
    /// The value's name.
    pub value: String,
    /// A number, or `"max"`.
    pub to: Amount,
}

/// A number, or a value's full amount.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Amount {
    /// This number.
    Number(i64),
    /// `"max"`: the value's `max`.
    Max(Max),
}

/// The word `max`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Max {
    /// The value's `max`.
    Max,
}

/// Bytes written over the game's, checked first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    /// Where.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Address>,
    /// On a 128K, the RAM bank for an address from 0xC000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank: Option<u8>,
    /// The bytes written.
    pub bytes: Bytes,
    /// The bytes that must be there first.
    pub expect: Bytes,
}

/// `[pad.*]`: what each part of a pad does in a state (`default` for all states).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pad {
    /// The D-pad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpad: Option<PadTarget>,
    /// The left stick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stick: Option<PadTarget>,
    /// The left face button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub west: Option<PadTarget>,
    /// The bottom face button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub south: Option<PadTarget>,
    /// The right face button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub east: Option<PadTarget>,
    /// The top face button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub north: Option<PadTarget>,
    /// The left shoulder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1: Option<PadTarget>,
    /// The right shoulder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r1: Option<PadTarget>,
    /// The left trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l2: Option<PadTarget>,
    /// The right trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r2: Option<PadTarget>,
    /// Start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<PadTarget>,
    /// Select (always the guidance menu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub select: Option<PadTarget>,
}

impl Pad {
    /// Each part that's set, by name.
    #[must_use]
    pub fn parts(&self) -> Vec<(&'static str, &PadTarget)> {
        [
            ("dpad", &self.dpad),
            ("stick", &self.stick),
            ("west", &self.west),
            ("south", &self.south),
            ("east", &self.east),
            ("north", &self.north),
            ("l1", &self.l1),
            ("r1", &self.r1),
            ("l2", &self.l2),
            ("r2", &self.r2),
            ("start", &self.start),
            ("select", &self.select),
        ]
        .into_iter()
        .filter_map(|(name, target)| target.as_ref().map(|t| (name, t)))
        .collect()
    }
}

/// What a part of a pad does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PadTarget {
    /// `joystick`, `up`, `down`, `left`, `right`, `fire`, `menu`, `none`, or an action's id.
    Named(String),
    /// Keys, a tap, or an action.
    Table(PadTable),
}

/// Keys, a tap, or an action.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PadTable {
    /// Keys held as long as the button is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
    /// Keys pressed for `frames` frames, however long the button is held.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tap: Vec<String>,
    /// How many frames a tap lasts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<u32>,
    /// An action's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// `[[actions]]`: something a button or key does beyond a key or a direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Action {
    /// Its name.
    pub id: String,
    /// Its row in the guidance menu, in these words ("End this game"); wants a second press.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu: Option<String>,
    /// The states in which the menu offers it; every state when left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub menu_in: Vec<String>,
    /// Its text where shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Keys held while it lasts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keys: Vec<String>,
    /// Keys pressed for `frames` frames.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tap: Vec<String>,
    /// How many frames it lasts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frames: Option<u32>,
    /// Joystick directions or fire held while it lasts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joystick: Vec<String>,
    /// Patches applied while it lasts, then undone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patch_for_frame: Vec<Patch>,
    /// A function of the script to call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// A key the Spectrum doesn't have that also does it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_key: Option<String>,
    /// Why and how it works.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `[text_entry]`: typing a word the game asks for, with a pad.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextEntry {
    /// The state in which the game asks.
    pub state: String,
    /// The letters allowed.
    pub letters: String,
    /// How many letters.
    pub length: u32,
    /// The character row the card's top sits at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_row: Option<u32>,
    /// The card's title when choosing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_choose: Option<String>,
    /// The card's title when spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_spell: Option<String>,
    /// The kept list offered as choices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<String>,
    /// The guidance level from which choices are offered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices_from_level: Option<u32>,
    /// Frames each letter's key is held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_down_frames: Option<u32>,
    /// Frames between letters, with no key held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_up_frames: Option<u32>,
}

/// `[persist.*]`: a list kept between games.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Persist {
    /// What the list holds, for "Forget the …".
    pub label: String,
    /// Its file in the game's data folder.
    pub file: String,
    /// Where its entries come from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<Add>,
    /// The most entries it keeps: past that, the oldest go first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub most: Option<u32>,
}

/// Where a kept list's entries come from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Add {
    /// `"script"`: the script returns them.
    Script(ScriptWord),
    /// A value read while the game is in a state.
    Value {
        /// The value.
        value: String,
        /// The state.
        in_state: String,
    },
}

/// The word `script`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScriptWord {
    /// The script.
    Script,
}

/// `[high_scores]`: the game's table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HighScores {
    /// The table's heading in the panel after a game: the game's own name for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Where the table starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<Address>,
    /// How many entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Bytes from one entry to the next.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stride: Option<u32>,
    /// The entry's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Field>,
    /// The entry's score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<Field>,
}

/// `[guidance]`: the guidance levels.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Guidance {
    /// From 0 (no help) up.
    #[serde(default)]
    pub levels: Vec<Level>,
}

/// A guidance level.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Level {
    /// Its name.
    pub name: String,
    /// What it adds, in a line.
    pub description: String,
}

/// `[[panel]]`: a widget in the panel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    /// What it draws.
    pub kind: WidgetKind,
    /// Its heading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The guidance level from which it shows.
    #[serde(default)]
    pub from_level: u32,
    /// Where in the panel it goes.
    #[serde(default, skip_serializing_if = "is_default")]
    pub column: Column,
    /// For `icon-grid`: pictures a row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<u32>,
    /// A `sprites` value its pictures come from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pictures: Option<String>,
    /// A `font` value its words are drawn in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// `value:<name>`, `persist:<list>` or `script:<result>`.
    pub source: String,
    /// What it says when there's nothing to show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty: Option<String>,
    /// For `map-grid`: the shape each kind of marker the script uses is drawn as.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub markers: BTreeMap<String, MarkerShape>,
}

/// A shape the map draws a marker as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MarkerShape {
    /// Where the player is: a box, with a bar above its cell.
    Player,
    /// A diamond.
    Diamond,
    /// A small square.
    Square,
    /// A ring.
    Ring,
    /// The marker's own picture, from the widget's `pictures`.
    Picture,
}

/// Where a widget goes in the panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    /// The panel's whole width, below everything in the columns above it.
    #[default]
    Full,
    /// The left column.
    Left,
    /// The right column, right-aligned.
    Right,
}

/// What a widget draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WidgetKind {
    /// A line of text.
    Text,
    /// A number.
    Counter,
    /// A bar.
    Meter,
    /// Short words in the game's font.
    ChipList,
    /// Pictures, bright, dim or outlined.
    IconGrid,
    /// Groups of pictures, each dim until answered.
    CardSet,
    /// Places on a grid, with walls, markers and routes.
    MapGrid,
}

/// `[situations]`: how a crafted situation enters play.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Situations {
    /// The routine that starts play in the current room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<Address>,
    /// The keys that take the game from its loaded tape to play, in turn: each tapped a
    /// second after the game's state last changed, and again every forty seconds while it
    /// stays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start: Vec<String>,
}

/// `[script]`: the definition's script.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// Its file, within the definition's folder or zip.
    pub file: String,
}

/// Whether a value is its type's default, to leave it out when writing.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_false(b: &bool) -> bool {
    !*b
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_true(b: &bool) -> bool {
    *b
}

const fn yes() -> bool {
    true
}
