//! The guidance menu's state: its rows from the definition, the changes made in it until
//! they're applied, presses shown before they act, second presses, and the question before
//! the score shows more. No SDL here; the window feeds it and draws it.

use crate::press::{self, Pressed};
use sidekick_core::{Record, Session, SwitchError};
use sidekick_def::Definition;
use sidekick_ui::{ChoiceRow, Levels, Menu as Card, PadLabels, Question, Row, SwitchRow};
use std::time::Instant;

/// What the menu is asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    /// A click on a row: chooses it and presses it.
    Click(Row),
    /// Chooses a row without pressing it.
    Choose(Row),
}

/// What comes of an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The menu stays open.
    Stay,
    /// Closed without applying.
    Cancelled,
    /// Closed with the settings applied.
    Applied,
    /// Closed to do the definition's action at this index, the one its row stands for.
    Act(usize),
    /// The program is to quit.
    Exit,
    /// Closed to save to the quick slot.
    Save,
    /// Closed to load the quick slot.
    Load,
    /// Closed to choose a save to load.
    OpenSave,
}

/// One of the actions at the menu's foot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Back,
    Forget,
    /// The definition's action at this index.
    Game(usize),
    Exit,
}

/// A training switch as the menu shows it.
#[derive(Clone, Debug)]
struct Switch {
    id: String,
    label: String,
    marks_score: bool,
    on: bool,
    refusal: Option<String>,
    /// When it last flipped, while its knob slides across.
    flipped: Option<Instant>,
}

/// The guidance menu, open.
#[derive(Clone, Debug)]
pub(crate) struct Menu {
    levels: Vec<(String, String)>,
    level: usize,
    switches: Vec<Switch>,
    /// The Spectrum keyboard's setting for the session: `None` the game's own choice, which is
    /// `game_keyboard` here.
    keyboard: Option<bool>,
    game_keyboard: bool,
    actions: Vec<(Action, String)>,
    chosen: Row,
    /// The row pressed, while its press shows; what it does waits until then.
    pressed: Option<Pressed<Row>>,
    armed: Option<usize>,
    /// What the score would show anew, while the question is asked.
    question: Option<Vec<String>>,
    /// Whether the quick slot holds a save.
    can_load: bool,
}

/// What a refused switch's row says.
const REFUSED: &str = "Not this tape's code here: the switch can't go on.";

impl Menu {
    /// The menu for the game being played, as its settings stand: `level` is the guidance
    /// level in use; `can_load`, whether the quick slot holds a save; `keyboard`, the Spectrum
    /// keyboard's setting for the session (`None`, the game's choice), and `game_keyboard` the
    /// game's choice here.
    pub(crate) fn open(
        session: &Session,
        level: usize,
        title: &str,
        can_load: bool,
        keyboard: Option<bool>,
        game_keyboard: bool,
    ) -> Self {
        let definition = session.definition();
        let levels: Vec<(String, String)> = definition
            .and_then(|d| d.guidance.as_ref())
            .map(|g| {
                g.levels
                    .iter()
                    .map(|l| (l.name.clone(), l.description.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let switches: Vec<Switch> = definition
            .into_iter()
            .flat_map(|d| &d.cheats)
            .map(|c| Switch {
                id: c.id.clone(),
                label: c.label.clone(),
                marks_score: c.marks_score,
                on: session.switched_on(&c.id),
                refusal: None,
                flipped: None,
            })
            .collect();
        let mut actions = vec![(Action::Back, "Back to the game".to_owned())];
        if session.kept().values().any(|list| !list.is_empty()) {
            actions.push((
                Action::Forget,
                format!("Forget the {}", kept_words(definition)),
            ));
        }
        for (i, words) in menu_actions(definition, session.state()) {
            actions.push((Action::Game(i), words));
        }
        actions.push((Action::Exit, format!("Exit {title}")));
        let chosen = if levels.is_empty() {
            if switches.is_empty() {
                Row::Action(0)
            } else {
                Row::Switch(0)
            }
        } else {
            Row::Level
        };
        Self {
            level: level.min(levels.len().saturating_sub(1)),
            levels,
            switches,
            keyboard,
            game_keyboard,
            actions,
            chosen,
            pressed: None,
            armed: None,
            question: None,
            can_load,
        }
    }

    /// The rows, in order: the saves row, after the first action, as its first button.
    fn rows(&self) -> Vec<Row> {
        let level = (!self.levels.is_empty()).then_some(Row::Level);
        level
            .into_iter()
            .chain((0..self.switches.len()).map(Row::Switch))
            .chain(std::iter::once(Row::Keyboard))
            .chain((0..self.actions.len()).flat_map(|i| {
                let saves = (i == 0).then_some(Row::Save(0));
                std::iter::once(Row::Action(i)).chain(saves)
            }))
            .collect()
    }

    /// Takes an input at `now`; `session` is asked whether a switch could go on, and applied
    /// to. A press (Enter, the pad's confirm, a click) shows first: what it does waits for
    /// [`Self::press_due`], or for the next input, which never waits behind it. A switch flips
    /// at once, its knob sliding across: the slide is its press.
    pub(crate) fn input(&mut self, input: Input, session: &mut Session, now: Instant) -> Outcome {
        let before: Vec<bool> = self.switches.iter().map(|s| s.on).collect();
        let outcome = self.take(input, session, now);
        for (switch, was) in self.switches.iter_mut().zip(before) {
            if switch.on != was {
                switch.flipped = Some(now);
            }
        }
        outcome
    }

    fn take(&mut self, input: Input, session: &mut Session, now: Instant) -> Outcome {
        if let Some(pressed) = self.pressed.take() {
            self.chosen = pressed.what();
            let outcome = self.press(session);
            if outcome != Outcome::Stay {
                return outcome;
            }
        }
        if let Some(items) = &self.question {
            debug_assert!(!items.is_empty());
            return match input {
                Input::Confirm => self.apply(session),
                Input::Back => {
                    self.question = None;
                    Outcome::Stay
                }
                _ => Outcome::Stay,
            };
        }
        match input {
            Input::Up | Input::Down => {
                self.armed = None;
                let rows = self.rows();
                let row = match self.chosen {
                    Row::Save(_) => Row::Save(0),
                    row => row,
                };
                let at = rows.iter().position(|&r| r == row).unwrap_or(0);
                let next = if input == Input::Up {
                    at.saturating_sub(1)
                } else {
                    (at + 1).min(rows.len() - 1)
                };
                self.chosen = rows[next];
                Outcome::Stay
            }
            Input::Left | Input::Right => {
                self.change(input == Input::Right, false, session);
                Outcome::Stay
            }
            Input::Back => Outcome::Cancelled,
            Input::Confirm => {
                if matches!(self.chosen, Row::Switch(_) | Row::Keyboard) {
                    return self.press(session);
                }
                self.pressed = Some(Pressed::new(self.chosen, now));
                Outcome::Stay
            }
            Input::Click(row) | Input::Choose(row) => {
                if self.chosen != row {
                    self.armed = None;
                }
                self.chosen = row;
                if matches!(input, Input::Click(_)) {
                    if matches!(row, Row::Switch(_) | Row::Keyboard) {
                        return self.press(session);
                    }
                    self.pressed = Some(Pressed::new(row, now));
                }
                Outcome::Stay
            }
        }
    }

    /// What a press does, once it has shown long enough at `now`; nothing while it shows, or
    /// with no press.
    pub(crate) fn press_due(&mut self, session: &mut Session, now: Instant) -> Option<Outcome> {
        self.chosen = press::take_due(&mut self.pressed, now)?;
        Some(self.press(session))
    }

    /// Changes the chosen level or switch: `up` steps the level up (or turns a switch on);
    /// `wrap` goes round from the top level to 0.
    fn change(&mut self, up: bool, wrap: bool, session: &Session) {
        match self.chosen {
            Row::Level => {
                let top = self.levels.len().saturating_sub(1);
                self.level = match (up, self.level) {
                    (true, l) if l == top && wrap => 0,
                    (true, l) => (l + 1).min(top),
                    (false, l) => l.saturating_sub(1),
                };
            }
            Row::Switch(i) => {
                let Some(switch) = self.switches.get_mut(i) else {
                    return;
                };
                let on = if wrap { !switch.on } else { up };
                switch.refusal = None;
                if on && !switch.on {
                    if let Err(SwitchError::Mismatch { .. }) = session.can_switch_on(&switch.id) {
                        switch.refusal = Some(REFUSED.to_owned());
                        return;
                    }
                }
                switch.on = on;
            }
            // Round the three, either way: the game's choice, on, off.
            Row::Keyboard => {
                self.keyboard = match (self.keyboard, up) {
                    (None, true) | (Some(false), false) => Some(true),
                    (Some(true), true) | (None, false) => Some(false),
                    (Some(false), true) | (Some(true), false) => None,
                };
            }
            Row::Save(i) => {
                self.chosen = Row::Save(if up {
                    (i + 1).min(2)
                } else {
                    i.saturating_sub(1)
                });
            }
            Row::Action(_) => {}
        }
    }

    /// Enter on the chosen row.
    fn press(&mut self, session: &mut Session) -> Outcome {
        match self.chosen {
            Row::Level | Row::Switch(_) | Row::Keyboard => {
                self.change(true, true, session);
                Outcome::Stay
            }
            Row::Save(0) => Outcome::Save,
            Row::Save(1) => Outcome::Load,
            Row::Save(_) => Outcome::OpenSave,
            Row::Action(i) => match self.actions.get(i).map(|&(a, _)| a) {
                Some(Action::Back) => {
                    let items = self.new_in_record(session.record());
                    if items.is_empty() {
                        self.apply(session)
                    } else {
                        self.question = Some(items);
                        Outcome::Stay
                    }
                }
                Some(action @ (Action::Forget | Action::Game(_) | Action::Exit)) => {
                    if self.armed == Some(i) {
                        self.armed = None;
                        match action {
                            Action::Exit => Outcome::Exit,
                            Action::Game(index) => Outcome::Act(index),
                            _ => {
                                // Forgotten at once; the row goes, its place taken by the next.
                                session.forget_kept();
                                self.actions.remove(i);
                                Outcome::Stay
                            }
                        }
                    } else {
                        self.armed = Some(i);
                        Outcome::Stay
                    }
                }
                None => Outcome::Stay,
            },
        }
    }

    /// What applying would add to the record: a higher level, switches it hasn't had.
    fn new_in_record(&self, record: &Record) -> Vec<String> {
        let level = (self.level > usize::from(record.level)).then(|| {
            let name = self.levels.get(self.level).map_or("", |(n, _)| n.as_str());
            format!("Guidance level {}: {name}", self.level)
        });
        let switches = self
            .switches
            .iter()
            .filter(|s| s.on && s.marks_score && !record.switches.contains(&s.id))
            .map(|s| s.label.clone());
        level.into_iter().chain(switches).collect()
    }

    /// Applies the level and switches. A switch refused now (the game moved on since it was
    /// chosen) keeps the menu open, showing why.
    fn apply(&mut self, session: &mut Session) -> Outcome {
        self.question = None;
        session.set_level(u8::try_from(self.level).unwrap_or(u8::MAX));
        let mut refused = false;
        for switch in &mut self.switches {
            if session.switch(&switch.id, switch.on).is_err() {
                switch.on = false;
                switch.refusal = Some(REFUSED.to_owned());
                refused = true;
            }
        }
        if refused {
            Outcome::Stay
        } else {
            Outcome::Applied
        }
    }

    /// The quick slot has been saved to: Load is there to press.
    pub(crate) const fn saved(&mut self) {
        self.can_load = true;
    }

    /// The guidance level chosen.
    pub(crate) const fn level(&self) -> usize {
        self.level
    }

    /// The Spectrum keyboard's setting chosen: on, off, or `None`, the game's choice.
    pub(crate) const fn keyboard(&self) -> Option<bool> {
        self.keyboard
    }

    /// The card, as it stands, for a pad marked as `pad` says, its switches' knobs at rest.
    pub(crate) fn card(&self, pad: PadLabels) -> Card {
        self.card_at(pad, None)
    }

    /// The card at `now`, a switch flipped in the last moment drawn with its knob on the way.
    pub(crate) fn card_at(&self, pad: PadLabels, now: Option<Instant>) -> Card {
        Card {
            levels: (!self.levels.is_empty()).then(|| Levels {
                levels: self.levels.clone(),
                level: self.level,
            }),
            switches: self.switches.iter().map(|s| switch_row(s, now)).collect(),
            keyboard: Some(ChoiceRow {
                label: "Spectrum keyboard".to_owned(),
                value: match self.keyboard {
                    None if self.game_keyboard => "Game's choice (on)",
                    None => "Game's choice (off)",
                    Some(true) => "On",
                    Some(false) => "Off",
                }
                .to_owned(),
            }),
            actions: self.actions.iter().map(|(_, a)| a.clone()).collect(),
            saves: Some(self.can_load),
            chosen: self.chosen,
            pressed: self.pressed.map(|p| p.what()),
            armed: self.armed.and_then(|i| {
                let (action, words) = self.actions.get(i)?;
                let what = match action {
                    Action::Game(_) => lower_first(words),
                    Action::Exit => "exit".to_owned(),
                    Action::Forget => {
                        format!("forget {}", words.strip_prefix("Forget ").unwrap_or(words))
                    }
                    Action::Back => return None,
                };
                Some((i, what))
            }),
            pad,
        }
    }

    /// The question, while it's asked.
    pub(crate) fn question(&self, pad: PadLabels) -> Option<Question> {
        self.question.as_ref().map(|items| Question {
            items: items.clone(),
            pad,
        })
    }
}

/// What the Forget row names: the kept list's label when there's one, the kept lists when
/// there are more.
fn kept_words(definition: Option<&Definition>) -> String {
    let labels: Vec<&str> = definition
        .into_iter()
        .flat_map(|d| d.persist.values())
        .map(|p| p.label.as_str())
        .collect();
    match labels.as_slice() {
        [one] => (*one).to_owned(),
        _ => "kept lists".to_owned(),
    }
}

/// The definition's actions with a row in the menu, offered in `state`: each one's index and
/// words.
fn menu_actions(definition: Option<&Definition>, state: Option<&str>) -> Vec<(usize, String)> {
    definition
        .into_iter()
        .flat_map(|d| d.actions.iter().enumerate())
        .filter_map(|(i, action)| {
            let words = action.menu.clone()?;
            let offered = action.menu_in.is_empty()
                || state.is_some_and(|s| action.menu_in.iter().any(|m| m == s));
            offered.then_some((i, words))
        })
        .collect()
}

/// A switch's row as the card shows it at `now`.
fn switch_row(switch: &Switch, now: Option<Instant>) -> SwitchRow {
    SwitchRow {
        label: switch.label.clone(),
        on: switch.on,
        knob: knob(switch, now),
        refusal: switch.refusal.clone(),
    }
}

/// Where a switch's knob is at `now`, 0 to 100: sliding over [`press::SHOWN`] since it flipped,
/// at rest after, or with no time given.
fn knob(switch: &Switch, now: Option<Instant>) -> u8 {
    let rest = if switch.on { 100 } else { 0 };
    let (Some(now), Some(flipped)) = (now, switch.flipped) else {
        return rest;
    };
    let gone = now.saturating_duration_since(flipped).as_millis();
    let whole = press::SHOWN.as_millis();
    if gone >= whole {
        return rest;
    }
    let way = u8::try_from(gone * 100 / whole).unwrap_or(100);
    if switch.on {
        way
    } else {
        100 - way
    }
}

/// Words as the second press's prompt says them: "End this game" as "end this game".
fn lower_first(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick_core::{Machine, Model};
    use std::time::Duration;

    const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[guidance]
levels = [
  { name = "Off", description = "No help." },
  { name = "Some", description = "Some help." },
  { name = "All", description = "All the help." },
]

[[cheats]]
id = "full-energy"
label = "Full energy"
set = { value = "energy", to = 9 }

[[cheats]]
id = "practice"
label = "Practice"
set = { value = "energy", to = 1 }
marks_score = false

[[cheats]]
id = "no-harm"
label = "No harm"
patch = [{ addr = 0x8010, bytes = "00", expect = "CD" }]

[values.energy]
addr = 0x8000
type = "u8"
writable = true

[[actions]]
id = "jump"
tap = ["Q"]

[[actions]]
id = "abandon"
tap = ["A", "S", "D", "F", "G"]
frames = 5
menu = "End this game"
"#;

    fn session(text: &str) -> Session {
        let definition = Definition::from_toml(text).expect("the test definition parses");
        let machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        Session::new(machine, Some(definition))
    }

    /// An input, and what a press it makes does once it has shown.
    fn act(menu: &mut Menu, s: &mut Session, input: Input) -> Outcome {
        let now = Instant::now();
        match menu.input(input, s, now) {
            Outcome::Stay => menu
                .press_due(s, now + press::SHOWN)
                .unwrap_or(Outcome::Stay),
            outcome => outcome,
        }
    }

    fn press(menu: &mut Menu, s: &mut Session, inputs: &[Input]) -> Outcome {
        inputs
            .iter()
            .map(|&i| act(menu, s, i))
            .last()
            .unwrap_or(Outcome::Stay)
    }

    #[test]
    fn a_switch_flips_at_once_and_its_knob_slides_across() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        let t0 = Instant::now();
        menu.input(Input::Down, &mut s, t0);
        assert_eq!(menu.input(Input::Confirm, &mut s, t0), Outcome::Stay);
        let at = |menu: &Menu, ms: u64| {
            menu.card_at(PadLabels::Letters, Some(t0 + Duration::from_millis(ms)))
                .switches[0]
                .clone()
        };
        assert!(at(&menu, 0).on, "on at once, no press to wait for");
        assert_eq!(
            [0, 60, 120, 500].map(|ms| at(&menu, ms).knob),
            [0, 50, 100, 100],
            "the knob slides right over 120 ms"
        );
        assert_eq!(
            menu.card(PadLabels::Letters).switches[0].knob,
            100,
            "at rest"
        );
        assert_eq!(
            menu.card(PadLabels::Letters).pressed,
            None,
            "no pressed look"
        );

        // A click turns it off again, the knob sliding back.
        let t1 = t0 + Duration::from_secs(1);
        menu.input(Input::Click(Row::Switch(0)), &mut s, t1);
        let back = menu.card_at(PadLabels::Letters, Some(t1 + Duration::from_millis(30)));
        assert!(!back.switches[0].on);
        assert_eq!(back.switches[0].knob, 75);
    }

    #[test]
    fn the_spectrum_keyboard_is_the_game_s_choice_on_or_off_until_applied() {
        let mut s = session(DEFINITION);
        let value = |menu: &Menu| {
            let card = menu.card(PadLabels::Letters);
            let row = card.keyboard.expect("the keyboard's row");
            assert_eq!(row.label, "Spectrum keyboard");
            row.value
        };
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        assert_eq!(value(&menu), "Game's choice (off)");
        assert_eq!(menu.keyboard(), None);
        press(&mut menu, &mut s, &[Input::Down; 4]);
        assert_eq!(
            menu.card(PadLabels::Letters).chosen,
            Row::Keyboard,
            "past the level and the three switches"
        );
        let mut seen = Vec::new();
        for _ in 0..3 {
            act(&mut menu, &mut s, Input::Right);
            seen.push((value(&menu), menu.keyboard()));
        }
        assert_eq!(
            seen,
            [
                ("On".to_owned(), Some(true)),
                ("Off".to_owned(), Some(false)),
                ("Game's choice (off)".to_owned(), None),
            ],
            "right goes round the three"
        );
        act(&mut menu, &mut s, Input::Left);
        assert_eq!(
            menu.keyboard(),
            Some(false),
            "left goes round the other way"
        );
        act(&mut menu, &mut s, Input::Confirm);
        assert_eq!(
            menu.keyboard(),
            None,
            "Enter steps on, back to the game's choice"
        );
        act(&mut menu, &mut s, Input::Click(Row::Keyboard));
        assert_eq!(menu.keyboard(), Some(true), "a click steps on");
        assert!(
            menu.card(PadLabels::Letters).switches.iter().all(|s| !s.on),
            "no training switch with it"
        );
        assert_eq!(
            act(&mut menu, &mut s, Input::Click(Row::Action(0))),
            Outcome::Applied
        );
        assert_eq!(menu.keyboard(), Some(true), "for the window to keep");
        assert!(s.record().switches.is_empty(), "not a training switch");

        let chosen = Menu::open(&s, 0, "Test", false, Some(false), true);
        assert_eq!(value(&chosen), "Off", "as chosen before");
        let game = Menu::open(&s, 0, "Test", false, None, true);
        assert_eq!(value(&game), "Game's choice (on)");
    }

    #[test]
    fn its_rows_come_from_the_definition() {
        let s = session(DEFINITION);
        let menu = Menu::open(&s, 0, "Test", false, None, false);
        let card = menu.card(PadLabels::Letters);
        assert!(card.levels.is_some());
        assert_eq!(card.switches.len(), 3);
        assert_eq!(
            card.actions,
            ["Back to the game", "End this game", "Exit Test"]
        );
        assert_eq!(card.chosen, Row::Level);

        let plain = session(
            "schema = 1\n[game]\nid = \"p\"\ntitle = \"P\"\nmachine = \"48k\"\n[[media]]\nname = \"t\"\n",
        );
        let card = Menu::open(&plain, 0, "P", false, None, true).card(PadLabels::Letters);
        assert!(card.levels.is_none());
        assert!(card.switches.is_empty());
        assert_eq!(card.actions, ["Back to the game", "Exit P"]);
        assert_eq!(card.chosen, Row::Action(0));
    }

    #[test]
    fn changes_wait_for_back_to_the_game_and_esc_drops_them() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        press(
            &mut menu,
            &mut s,
            &[Input::Right, Input::Down, Input::Confirm],
        );
        assert_eq!(menu.level(), 1);
        assert!(menu.card(PadLabels::Letters).switches[0].on);
        assert!(!s.switched_on("full-energy"), "not yet");
        assert_eq!(press(&mut menu, &mut s, &[Input::Back]), Outcome::Cancelled);
        assert!(!s.switched_on("full-energy"));
    }

    #[test]
    fn the_question_comes_first_when_the_score_would_show_more() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        let to_back = [Input::Down; 3];
        press(&mut menu, &mut s, &[Input::Right, Input::Right]);
        press(&mut menu, &mut s, &[Input::Down, Input::Confirm]);
        press(&mut menu, &mut s, &[Input::Down, Input::Confirm]);
        press(&mut menu, &mut s, &to_back);
        assert_eq!(press(&mut menu, &mut s, &[Input::Confirm]), Outcome::Stay);
        let question = menu.question(PadLabels::Letters).expect("asked");
        assert_eq!(
            question.items,
            ["Guidance level 2: All", "Full energy"],
            "not practice"
        );
        assert_eq!(press(&mut menu, &mut s, &[Input::Back]), Outcome::Stay);
        assert!(
            menu.question(PadLabels::Letters).is_none(),
            "back to the menu"
        );
        assert_eq!(press(&mut menu, &mut s, &[Input::Confirm]), Outcome::Stay);
        assert_eq!(
            press(&mut menu, &mut s, &[Input::Confirm]),
            Outcome::Applied
        );
        assert!(s.switched_on("full-energy") && s.switched_on("practice"));
        assert_eq!(s.record().level, 2);

        let mut again = Menu::open(&s, 2, "Test", false, None, false);
        let rows = [Input::Down; 5];
        assert_eq!(
            press(&mut again, &mut s, &[&rows[..], &[Input::Confirm]].concat()),
            Outcome::Applied,
            "nothing new: no question"
        );
    }

    #[test]
    fn a_switch_that_can_t_go_on_stays_off_and_says_why() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        press(
            &mut menu,
            &mut s,
            &[Input::Down, Input::Down, Input::Down, Input::Right],
        );
        let card = menu.card(PadLabels::Letters);
        assert!(!card.switches[2].on);
        assert_eq!(card.switches[2].refusal.as_deref(), Some(REFUSED));
        s.machine_mut().write(0x8010, 0xCD);
        press(&mut menu, &mut s, &[Input::Confirm]);
        let card = menu.card(PadLabels::Letters);
        assert!(card.switches[2].on, "pressed again, it goes on");
        assert_eq!(card.switches[2].refusal, None);
    }

    #[test]
    fn forgetting_the_kept_lists_wants_a_second_press() {
        let text = format!(
            "{DEFINITION}\n[persist.codes]\nlabel = \"teleport codes\"\nfile = \"codes.txt\"\n"
        );
        let mut s = session(&text);
        assert!(
            !Menu::open(&s, 0, "Test", false, None, false)
                .card(PadLabels::Letters)
                .actions
                .iter()
                .any(|a| a.starts_with("Forget")),
            "nothing kept: no row"
        );
        s.set_kept(std::collections::BTreeMap::from([(
            "codes".to_owned(),
            vec!["TOPAZ".to_owned()],
        )]));
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        let card = menu.card(PadLabels::Letters);
        assert_eq!(card.actions[1], "Forget the teleport codes");
        let forget = Row::Action(1);
        assert_eq!(act(&mut menu, &mut s, Input::Click(forget)), Outcome::Stay);
        assert_eq!(
            menu.card(PadLabels::Letters).armed,
            Some((1, "forget the teleport codes".to_owned()))
        );
        assert_eq!(act(&mut menu, &mut s, Input::Click(forget)), Outcome::Stay);
        assert!(s.kept()["codes"].is_empty(), "forgotten");
        assert_eq!(
            menu.card(PadLabels::Letters).actions,
            ["Back to the game", "End this game", "Exit Test"],
            "the row goes"
        );
    }

    #[test]
    fn end_and_exit_want_a_second_press() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        let end = Row::Action(1);
        assert_eq!(act(&mut menu, &mut s, Input::Click(end)), Outcome::Stay);
        assert_eq!(
            menu.card(PadLabels::Letters).armed,
            Some((1, "end this game".to_owned()))
        );
        assert_eq!(
            act(&mut menu, &mut s, Input::Up),
            Outcome::Stay,
            "moving away"
        );
        assert_eq!(menu.card(PadLabels::Letters).armed, None);
        assert_eq!(act(&mut menu, &mut s, Input::Click(end)), Outcome::Stay);
        assert_eq!(
            act(&mut menu, &mut s, Input::Confirm),
            Outcome::Act(1),
            "the definition's action, past one with no row"
        );
        let exit = Row::Action(2);
        act(&mut menu, &mut s, Input::Click(exit));
        assert_eq!(act(&mut menu, &mut s, Input::Click(exit)), Outcome::Exit);
    }

    #[test]
    fn an_action_s_row_is_offered_only_in_its_states() {
        let text = format!(
            "{}\nmenu_in = [\"running\"]\n\n[values.mode]\naddr = 0x8100\ntype = \"u8\"\n\n[[states]]\nname = \"running\"\nwhen = {{ value = \"mode\", equals = 2 }}\n",
            DEFINITION.trim_end()
        );
        let mut s = session(&text);
        let rows = |s: &Session| {
            Menu::open(s, 0, "Test", false, None, false)
                .card(PadLabels::Letters)
                .actions
        };
        assert_eq!(
            rows(&s),
            ["Back to the game", "Exit Test"],
            "not running yet"
        );
        s.machine_mut().write(0x8100, 2);
        s.run_frame().expect("a frame");
        assert_eq!(rows(&s), ["Back to the game", "End this game", "Exit Test"]);
    }

    #[test]
    fn choosing_a_row_doesn_t_press_it() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        act(&mut menu, &mut s, Input::Choose(Row::Switch(0)));
        assert_eq!(menu.card(PadLabels::Letters).chosen, Row::Switch(0));
        assert!(!menu.card(PadLabels::Letters).switches[0].on);
        act(&mut menu, &mut s, Input::Choose(Row::Level));
        act(&mut menu, &mut s, Input::Left);
        assert_eq!(menu.level(), 0, "no wrapping round down");
    }

    #[test]
    fn the_saves_row_comes_after_back_to_the_game() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", true, None, false);
        assert_eq!(menu.card(PadLabels::Letters).saves, Some(true));
        let down = [Input::Down; 6];
        press(&mut menu, &mut s, &down);
        assert_eq!(menu.card(PadLabels::Letters).chosen, Row::Save(0));
        press(
            &mut menu,
            &mut s,
            &[Input::Right, Input::Right, Input::Right],
        );
        assert_eq!(
            menu.card(PadLabels::Letters).chosen,
            Row::Save(2),
            "no further"
        );
        assert_eq!(act(&mut menu, &mut s, Input::Confirm), Outcome::OpenSave);
        press(&mut menu, &mut s, &[Input::Left, Input::Down]);
        assert_eq!(
            menu.card(PadLabels::Letters).chosen,
            Row::Action(1),
            "down from any button"
        );
        press(&mut menu, &mut s, &[Input::Up]);
        assert_eq!(menu.card(PadLabels::Letters).chosen, Row::Save(0));
        assert_eq!(act(&mut menu, &mut s, Input::Confirm), Outcome::Save);
        assert_eq!(
            act(&mut menu, &mut s, Input::Click(Row::Save(1))),
            Outcome::Load
        );
    }

    #[test]
    fn enter_on_the_level_steps_up_and_wraps_round() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 2, "Test", false, None, false);
        press(&mut menu, &mut s, &[Input::Confirm]);
        assert_eq!(menu.level(), 0);
        press(&mut menu, &mut s, &[Input::Left, Input::Confirm]);
        assert_eq!(menu.level(), 1);
    }

    #[test]
    fn a_press_shows_for_a_moment_before_it_acts() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", true, None, false);
        press(&mut menu, &mut s, &[Input::Down; 6]);
        let t0 = Instant::now();
        assert_eq!(menu.input(Input::Confirm, &mut s, t0), Outcome::Stay);
        let card = menu.card(PadLabels::Letters);
        assert_eq!(card.pressed, Some(Row::Save(0)), "drawn pressed");
        assert_eq!(card.chosen, Row::Save(0));
        let shy = t0 + press::SHOWN.saturating_sub(Duration::from_millis(1));
        assert_eq!(menu.press_due(&mut s, shy), None, "still showing");
        assert_eq!(
            menu.press_due(&mut s, t0 + press::SHOWN),
            Some(Outcome::Save)
        );
        assert_eq!(menu.card(PadLabels::Letters).pressed, None);
        assert_eq!(menu.press_due(&mut s, t0 + press::SHOWN * 2), None, "once");
    }

    #[test]
    fn a_click_shows_on_the_button_clicked() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", true, None, false);
        let t0 = Instant::now();
        assert_eq!(
            menu.input(Input::Click(Row::Save(2)), &mut s, t0),
            Outcome::Stay
        );
        let card = menu.card(PadLabels::Letters);
        assert_eq!(
            (card.chosen, card.pressed),
            (Row::Save(2), Some(Row::Save(2)))
        );
        assert_eq!(
            menu.press_due(&mut s, t0 + press::SHOWN),
            Some(Outcome::OpenSave)
        );

        // A switch flips at once: its knob's slide is its press.
        menu.input(Input::Click(Row::Switch(1)), &mut s, t0);
        assert_eq!(menu.card(PadLabels::Letters).pressed, None);
        assert!(menu.card(PadLabels::Letters).switches[1].on);
        assert_eq!(menu.press_due(&mut s, t0 + press::SHOWN), None);
    }

    #[test]
    fn choosing_a_row_shows_no_press() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        let t0 = Instant::now();
        menu.input(Input::Choose(Row::Switch(0)), &mut s, t0);
        menu.input(Input::Down, &mut s, t0);
        assert_eq!(menu.card(PadLabels::Letters).pressed, None);
        assert_eq!(menu.press_due(&mut s, t0 + press::SHOWN), None);
    }

    #[test]
    fn an_input_while_a_press_shows_does_the_press_first() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false, None, false);
        let t0 = Instant::now();
        menu.input(Input::Click(Row::Switch(0)), &mut s, t0);
        menu.input(Input::Down, &mut s, t0);
        let card = menu.card(PadLabels::Letters);
        assert!(card.switches[0].on, "the press wasn't lost");
        assert_eq!((card.chosen, card.pressed), (Row::Switch(1), None));

        let exit = Row::Action(2);
        menu.input(Input::Click(exit), &mut s, t0);
        menu.input(Input::Click(exit), &mut s, t0);
        assert!(
            menu.card(PadLabels::Letters).armed.is_some(),
            "the first press armed it"
        );
        assert_eq!(
            menu.input(Input::Back, &mut s, t0),
            Outcome::Exit,
            "a quick second press counts"
        );
    }
}
