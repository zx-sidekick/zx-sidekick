//! The guidance menu's state: its rows from the definition, the changes made in it until
//! they're applied, second presses, and the question before the score shows more. No SDL
//! here; the window feeds it and draws it.

use sidekick_core::{Record, Session, SwitchError};
use sidekick_def::Definition;
use sidekick_ui::{Levels, Menu as Card, PadLabels, Question, Row, SwitchRow};

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
    /// Closed to end the game with the definition's `end-game` action.
    EndGame,
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
    EndGame,
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
}

/// The guidance menu, open.
#[derive(Clone, Debug)]
pub(crate) struct Menu {
    levels: Vec<(String, String)>,
    level: usize,
    switches: Vec<Switch>,
    actions: Vec<(Action, String)>,
    chosen: Row,
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
    /// level in use; `can_load`, whether the quick slot holds a save.
    pub(crate) fn open(session: &Session, level: usize, title: &str, can_load: bool) -> Self {
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
            })
            .collect();
        let mut actions = vec![(Action::Back, "Back to the game".to_owned())];
        if session.kept().values().any(|list| !list.is_empty()) {
            actions.push((
                Action::Forget,
                format!("Forget the {}", kept_words(definition)),
            ));
        }
        if can_end_game(definition, session.state()) {
            actions.push((Action::EndGame, "End this game".to_owned()));
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
            actions,
            chosen,
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
            .chain((0..self.actions.len()).flat_map(|i| {
                let saves = (i == 0).then_some(Row::Save(0));
                std::iter::once(Row::Action(i)).chain(saves)
            }))
            .collect()
    }

    /// Takes an input; `session` is asked whether a switch could go on, and applied to.
    pub(crate) fn input(&mut self, input: Input, session: &mut Session) -> Outcome {
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
            Input::Confirm => self.press(session),
            Input::Click(row) | Input::Choose(row) => {
                if self.chosen != row {
                    self.armed = None;
                }
                self.chosen = row;
                if matches!(input, Input::Click(_)) {
                    self.press(session)
                } else {
                    Outcome::Stay
                }
            }
        }
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
            Row::Level | Row::Switch(_) => {
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
                Some(action @ (Action::Forget | Action::EndGame | Action::Exit)) => {
                    if self.armed == Some(i) {
                        self.armed = None;
                        match action {
                            Action::Exit => Outcome::Exit,
                            Action::EndGame => Outcome::EndGame,
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

    /// The guidance level chosen.
    pub(crate) const fn level(&self) -> usize {
        self.level
    }

    /// The card, as it stands, for a pad marked as `pad` says.
    pub(crate) fn card(&self, pad: PadLabels) -> Card {
        Card {
            levels: (!self.levels.is_empty()).then(|| Levels {
                levels: self.levels.clone(),
                level: self.level,
            }),
            switches: self
                .switches
                .iter()
                .map(|s| SwitchRow {
                    label: s.label.clone(),
                    on: s.on,
                    refusal: s.refusal.clone(),
                })
                .collect(),
            actions: self.actions.iter().map(|(_, a)| a.clone()).collect(),
            saves: Some(self.can_load),
            chosen: self.chosen,
            armed: self.armed.and_then(|i| {
                let (action, words) = self.actions.get(i)?;
                let what = match action {
                    Action::EndGame => "end this game".to_owned(),
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

/// Whether the menu offers End this game: the definition says how, and the game is in play
/// (or has no `play` state to say otherwise).
fn can_end_game(definition: Option<&Definition>, state: Option<&str>) -> bool {
    let Some(definition) = definition else {
        return false;
    };
    let has_action = definition.actions.iter().any(|a| a.id == "end-game");
    let has_play = definition.states.iter().any(|s| s.name == "play");
    has_action && (!has_play || state == Some("play"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick_core::{Machine, Model};

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
id = "end-game"
tap = ["A", "S", "D", "F", "G"]
frames = 5
"#;

    fn session(text: &str) -> Session {
        let definition = Definition::from_toml(text).expect("the test definition parses");
        let machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
        Session::new(machine, Some(definition))
    }

    fn press(menu: &mut Menu, s: &mut Session, inputs: &[Input]) -> Outcome {
        inputs
            .iter()
            .map(|&i| menu.input(i, s))
            .last()
            .unwrap_or(Outcome::Stay)
    }

    #[test]
    fn its_rows_come_from_the_definition() {
        let s = session(DEFINITION);
        let menu = Menu::open(&s, 0, "Test", false);
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
        let card = Menu::open(&plain, 0, "P", false).card(PadLabels::Letters);
        assert!(card.levels.is_none());
        assert!(card.switches.is_empty());
        assert_eq!(card.actions, ["Back to the game", "Exit P"]);
        assert_eq!(card.chosen, Row::Action(0));
    }

    #[test]
    fn changes_wait_for_back_to_the_game_and_esc_drops_them() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false);
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
        let mut menu = Menu::open(&s, 0, "Test", false);
        let to_back = [Input::Down, Input::Down, Input::Down, Input::Down];
        press(&mut menu, &mut s, &[Input::Right, Input::Right]);
        press(&mut menu, &mut s, &[Input::Down, Input::Confirm]);
        press(&mut menu, &mut s, &[Input::Down, Input::Confirm]);
        press(&mut menu, &mut s, &to_back[..2]);
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

        let mut again = Menu::open(&s, 2, "Test", false);
        let rows = [Input::Down, Input::Down, Input::Down, Input::Down];
        assert_eq!(
            press(&mut again, &mut s, &[&rows[..], &[Input::Confirm]].concat()),
            Outcome::Applied,
            "nothing new: no question"
        );
    }

    #[test]
    fn a_switch_that_can_t_go_on_stays_off_and_says_why() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false);
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
            !Menu::open(&s, 0, "Test", false)
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
        let mut menu = Menu::open(&s, 0, "Test", false);
        let card = menu.card(PadLabels::Letters);
        assert_eq!(card.actions[1], "Forget the teleport codes");
        let forget = Row::Action(1);
        assert_eq!(menu.input(Input::Click(forget), &mut s), Outcome::Stay);
        assert_eq!(
            menu.card(PadLabels::Letters).armed,
            Some((1, "forget the teleport codes".to_owned()))
        );
        assert_eq!(menu.input(Input::Click(forget), &mut s), Outcome::Stay);
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
        let mut menu = Menu::open(&s, 0, "Test", false);
        let end = Row::Action(1);
        assert_eq!(menu.input(Input::Click(end), &mut s), Outcome::Stay);
        assert_eq!(
            menu.card(PadLabels::Letters).armed,
            Some((1, "end this game".to_owned()))
        );
        assert_eq!(menu.input(Input::Up, &mut s), Outcome::Stay, "moving away");
        assert_eq!(menu.card(PadLabels::Letters).armed, None);
        assert_eq!(menu.input(Input::Click(end), &mut s), Outcome::Stay);
        assert_eq!(menu.input(Input::Confirm, &mut s), Outcome::EndGame);
        let exit = Row::Action(2);
        menu.input(Input::Click(exit), &mut s);
        assert_eq!(menu.input(Input::Click(exit), &mut s), Outcome::Exit);
    }

    #[test]
    fn choosing_a_row_doesn_t_press_it() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", false);
        menu.input(Input::Choose(Row::Switch(0)), &mut s);
        assert_eq!(menu.card(PadLabels::Letters).chosen, Row::Switch(0));
        assert!(!menu.card(PadLabels::Letters).switches[0].on);
        menu.input(Input::Choose(Row::Level), &mut s);
        menu.input(Input::Left, &mut s);
        assert_eq!(menu.level(), 0, "no wrapping round down");
    }

    #[test]
    fn the_saves_row_comes_after_back_to_the_game() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 0, "Test", true);
        assert_eq!(menu.card(PadLabels::Letters).saves, Some(true));
        let down = [Input::Down; 5];
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
        assert_eq!(menu.input(Input::Confirm, &mut s), Outcome::OpenSave);
        press(&mut menu, &mut s, &[Input::Left, Input::Down]);
        assert_eq!(
            menu.card(PadLabels::Letters).chosen,
            Row::Action(1),
            "down from any button"
        );
        press(&mut menu, &mut s, &[Input::Up]);
        assert_eq!(menu.card(PadLabels::Letters).chosen, Row::Save(0));
        assert_eq!(menu.input(Input::Confirm, &mut s), Outcome::Save);
        assert_eq!(
            menu.input(Input::Click(Row::Save(1)), &mut s),
            Outcome::Load
        );
    }

    #[test]
    fn enter_on_the_level_steps_up_and_wraps_round() {
        let mut s = session(DEFINITION);
        let mut menu = Menu::open(&s, 2, "Test", false);
        press(&mut menu, &mut s, &[Input::Confirm]);
        assert_eq!(menu.level(), 0);
        press(&mut menu, &mut s, &[Input::Left, Input::Confirm]);
        assert_eq!(menu.level(), 1);
    }
}
