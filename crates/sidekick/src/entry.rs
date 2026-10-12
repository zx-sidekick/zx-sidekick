//! The text-entry card's state, and the typing of a word on the game's keys over frames. No
//! SDL here; the window feeds it and draws it.

use crate::keys::key_named;
use sidekick_core::Key;
use sidekick_def::TextEntry;
use sidekick_ui::{EntryCard, EntryMode, PadLabels};
use std::collections::BTreeMap;

/// What the card is asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    /// The west button: between choosing and spelling.
    Switch,
}

/// The text-entry card, open.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    title_choose: String,
    title_spell: String,
    /// The letters a slot may hold, in order.
    letters: Vec<char>,
    slots: Vec<Option<char>>,
    /// The kept words offered, when the guidance level offers them.
    words: Vec<String>,
    mode: EntryMode,
}

impl Entry {
    /// The card for a definition's `[text_entry]`, offering the words of its kept list at
    /// `level` when the definition does.
    pub(crate) fn open(
        text_entry: &TextEntry,
        kept: &BTreeMap<String, Vec<String>>,
        level: usize,
    ) -> Self {
        let length = usize::try_from(text_entry.length).unwrap_or(0);
        let from = text_entry
            .choices_from_level
            .and_then(|l| usize::try_from(l).ok())
            .unwrap_or(0);
        let words: Vec<String> = text_entry
            .choices
            .as_ref()
            .filter(|_| level >= from)
            .and_then(|list| kept.get(list))
            .cloned()
            .unwrap_or_default();
        let mut entry = Self {
            title_choose: text_entry
                .title_choose
                .clone()
                .unwrap_or_else(|| "Choose".to_owned()),
            title_spell: text_entry
                .title_spell
                .clone()
                .unwrap_or_else(|| "Spell it".to_owned()),
            letters: text_entry.letters.chars().collect(),
            slots: vec![None; length],
            mode: EntryMode::Spelling { slot: 0 },
            words,
        };
        if !entry.words.is_empty() {
            entry.choose(0);
        }
        entry
    }

    /// Chooses the kept word at `at`, filling the slots with it.
    fn choose(&mut self, at: usize) {
        let of = self.words.len();
        let mut letters = self.words[at].chars();
        for slot in &mut self.slots {
            *slot = letters.next();
        }
        self.mode = EntryMode::Choosing { at, of };
    }

    /// Takes an input. Returns the word to type when it's entered.
    pub(crate) fn input(&mut self, input: Input) -> Option<String> {
        match (self.mode, input) {
            (_, Input::Confirm) => {
                return Some(self.slots.iter().flatten().collect());
            }
            (EntryMode::Choosing { at, of }, Input::Up | Input::Left) => {
                self.choose((at + of - 1) % of);
            }
            (EntryMode::Choosing { at, of }, Input::Down | Input::Right) => {
                self.choose((at + 1) % of);
            }
            (EntryMode::Choosing { .. }, Input::Switch) => {
                self.mode = EntryMode::Spelling { slot: 0 };
            }
            (EntryMode::Spelling { .. }, Input::Switch) if !self.words.is_empty() => {
                self.choose(0);
            }
            (EntryMode::Spelling { slot }, Input::Left) => {
                self.mode = EntryMode::Spelling {
                    slot: slot.saturating_sub(1),
                };
            }
            (EntryMode::Spelling { slot }, Input::Right) => {
                self.mode = EntryMode::Spelling {
                    slot: (slot + 1).min(self.slots.len().saturating_sub(1)),
                };
            }
            (EntryMode::Spelling { slot }, Input::Up | Input::Down) => {
                self.step(slot, input == Input::Up);
            }
            (EntryMode::Spelling { .. }, Input::Back) => {
                self.slots.iter_mut().for_each(|s| *s = None);
                self.mode = EntryMode::Spelling { slot: 0 };
            }
            (EntryMode::Choosing { .. }, Input::Back)
            | (EntryMode::Spelling { .. }, Input::Switch) => {}
        }
        None
    }

    /// Steps a slot's letter through the letters allowed, round the ends.
    fn step(&mut self, slot: usize, up: bool) {
        let count = self.letters.len();
        if count == 0 {
            return;
        }
        let Some(current) = self.slots.get_mut(slot) else {
            return;
        };
        let at = current.and_then(|c| self.letters.iter().position(|&l| l == c));
        let next = match (at, up) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(i), true) => (i + 1) % count,
            (Some(i), false) => (i + count - 1) % count,
        };
        *current = Some(self.letters[next]);
    }

    /// The word chosen, while choosing, for the panel to outline.
    pub(crate) fn chosen(&self) -> Option<&str> {
        match self.mode {
            EntryMode::Choosing { at, .. } => self.words.get(at).map(String::as_str),
            EntryMode::Spelling { .. } => None,
        }
    }

    /// The card, as it stands.
    pub(crate) fn card(&self, pad: PadLabels) -> EntryCard {
        EntryCard {
            title_choose: self.title_choose.clone(),
            title_spell: self.title_spell.clone(),
            letters: self.slots.clone(),
            mode: self.mode,
            can_choose: !self.words.is_empty(),
            pad,
        }
    }
}

/// A word being typed on the game's keys: each letter held for some frames, then let go for
/// some, so a game that waits for every key to be let go reads each one.
#[derive(Clone, Debug)]
pub(crate) struct Typing {
    keys: Vec<Key>,
    down: u32,
    up: u32,
    /// Frames into the word.
    frame: u32,
}

/// What to do with a key before a frame.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Stroke {
    Press(Key),
    Release(Key),
    /// Nothing this frame: a key held, or let go, a while.
    Wait,
}

impl Typing {
    /// A word to type, with the definition's timings (a frame each when not given). A letter
    /// that isn't a Spectrum key is left out.
    pub(crate) fn new(word: &str, text_entry: &TextEntry) -> Self {
        Self {
            keys: word
                .chars()
                .filter_map(|c| key_named(&c.to_string()))
                .collect(),
            down: text_entry.key_down_frames.unwrap_or(1).max(1),
            up: text_entry.key_up_frames.unwrap_or(1).max(1),
            frame: 0,
        }
    }

    /// The word's keys, to let go if it's stopped part-way.
    pub(crate) fn keys(&self) -> &[Key] {
        &self.keys
    }

    /// What to press or let go before the next frame; `None` once the word is typed.
    pub(crate) fn next(&mut self) -> Option<Stroke> {
        let each = self.down + self.up;
        let letter = usize::try_from(self.frame / each).ok()?;
        let key = *self.keys.get(letter)?;
        let into = self.frame % each;
        self.frame += 1;
        Some(match into {
            0 => Stroke::Press(key),
            n if n == self.down => Stroke::Release(key),
            _ => Stroke::Wait,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_entry() -> TextEntry {
        TextEntry {
            state: "booth".to_owned(),
            letters: "ABC".to_owned(),
            length: 3,
            card_row: Some(20),
            title_choose: Some("Choose a code".to_owned()),
            title_spell: Some("Spell the code".to_owned()),
            choices: Some("codes".to_owned()),
            choices_from_level: Some(1),
            key_down_frames: Some(2),
            key_up_frames: Some(1),
        }
    }

    fn kept(words: &[&str]) -> BTreeMap<String, Vec<String>> {
        BTreeMap::from([(
            "codes".to_owned(),
            words.iter().map(|&w| w.to_owned()).collect(),
        )])
    }

    fn word(entry: &Entry) -> String {
        entry.slots.iter().map(|s| s.unwrap_or('_')).collect()
    }

    #[test]
    fn choosing_goes_round_the_ends() {
        let mut e = Entry::open(&text_entry(), &kept(&["ABC", "CAB", "BAC"]), 1);
        assert_eq!(e.chosen(), Some("ABC"));
        e.input(Input::Up);
        assert_eq!(e.chosen(), Some("BAC"), "round from the first to the last");
        e.input(Input::Right);
        e.input(Input::Down);
        assert_eq!(e.chosen(), Some("CAB"));
        assert_eq!(e.input(Input::Confirm), Some("CAB".to_owned()));
    }

    #[test]
    fn below_its_level_or_with_nothing_kept_it_opens_on_spelling() {
        let low = Entry::open(&text_entry(), &kept(&["ABC"]), 0);
        assert_eq!(low.chosen(), None);
        assert!(!low.card(PadLabels::Letters).can_choose);
        let none = Entry::open(&text_entry(), &kept(&[]), 3);
        assert_eq!(none.chosen(), None);
    }

    #[test]
    fn spelling_steps_letters_moves_and_clears() {
        let mut e = Entry::open(&text_entry(), &kept(&["CAB"]), 1);
        e.input(Input::Switch);
        e.input(Input::Up);
        assert_eq!(word(&e), "AAB", "C steps up round to A");
        e.input(Input::Right);
        e.input(Input::Down);
        assert_eq!(word(&e), "ACB", "A steps down round to C");
        e.input(Input::Back);
        assert_eq!(word(&e), "___");
        e.input(Input::Down);
        assert_eq!(
            word(&e),
            "C__",
            "an empty slot steps down to the last letter"
        );
        assert_eq!(
            e.input(Input::Confirm),
            Some("C".to_owned()),
            "empty slots type nothing"
        );
        e.input(Input::Switch);
        assert_eq!(e.chosen(), Some("CAB"), "back to choosing");
    }

    #[test]
    fn a_word_is_typed_with_its_timings() {
        let mut t = Typing::new("AB", &text_entry());
        let strokes: Vec<String> = std::iter::from_fn(|| t.next())
            .map(|s| match s {
                Stroke::Press(k) => format!("+{k:?}"),
                Stroke::Release(k) => format!("-{k:?}"),
                Stroke::Wait => ".".to_owned(),
            })
            .collect();
        assert_eq!(strokes, ["+A", ".", "-A", "+B", ".", "-B"]);
    }
}
