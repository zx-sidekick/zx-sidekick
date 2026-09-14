//! How much help the player has asked for: the guidance level, training
//! mode, the record of both for the game in progress, and the picker that
//! changes them (#25).
//!
//! Nothing here reaches the game. The window and the game thread share it:
//! the window changes it from the keyboard and draws it, and the game thread
//! changes it from a gamepad and holds the game while the picker is open.
//! What each level shows is its own ticket's (#3).

/// The levels, each including the ones before it (#3).
pub const LEVELS: [&str; 6] = [
    "Off",
    "Teleporter codes",
    "Map",
    "Missing pieces",
    "Arrow, known routes",
    "Arrow, whole map",
];

/// How much help one game has had: the highest level in use at any point,
/// and whether training mode was ever on. It only ever rises within a game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub highest: u8,
    pub training: bool,
}

/// The rows of the picker, top to bottom: two settings, then two actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    #[default]
    Level,
    Training,
    EndGame,
    Exit,
}

/// The answers to "This will show on your score".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Use,
    Undo,
}

/// What the picker was asked to do, once confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Abandon the game in progress, as A S D F G does.
    EndGame,
    /// Close the program.
    Exit,
}

#[derive(Clone, Debug, Default)]
pub struct Guidance {
    level: u8,
    training: bool,
    record: Record,
    picker: bool,
    /// The level and training mode when the picker opened, which Undo goes
    /// back to.
    opened: (u8, bool),
    /// "This will show on your score", asked when leaving the picker would add to
    /// the record, and which answer is highlighted.
    asking: Option<Choice>,
    /// The row the picker has highlighted.
    focus: Setting,
    /// An action pressed once, waiting for the second press.
    armed: Option<Setting>,
    /// Whether a game is being played, which is when it can be ended.
    playing: bool,
    /// An action confirmed and not yet carried out.
    requested: Option<Action>,
    /// Bumped on every change, so a watcher can tell something changed.
    version: u64,
}

impl Guidance {
    /// The guidance level in effect.
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Whether training mode is in effect.
    pub fn training(&self) -> bool {
        self.training
    }

    pub fn record(&self) -> Record {
        self.record
    }

    pub fn picker_open(&self) -> bool {
        self.picker
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn focus(&self) -> Setting {
        self.focus
    }

    pub fn armed(&self) -> Option<Setting> {
        self.armed
    }

    /// The level and training mode as they were when the picker opened,
    /// which Undo goes back to.
    pub fn opened(&self) -> (u8, bool) {
        self.opened
    }

    /// The question, if it is up, and the highlighted answer.
    pub fn asking(&self) -> Option<Choice> {
        self.asking
    }

    /// Whether keeping the settings as they are would add to this game's
    /// record: a level above the highest used, or training mode for the
    /// first time. Lowering either never does.
    pub fn raises_record(&self) -> bool {
        self.level > self.record.highest || (self.training && !self.record.training)
    }

    /// The rows the picker shows: ending a game only while one is played.
    pub fn rows(&self) -> Vec<Setting> {
        let mut rows = vec![Setting::Level, Setting::Training];
        if self.playing {
            rows.push(Setting::EndGame);
        }
        rows.push(Setting::Exit);
        rows
    }

    /// Whether a game is being played, as the game thread sees it.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
        if !playing && self.focus == Setting::EndGame {
            self.focus = Setting::Exit;
        }
        if self.armed == Some(Setting::EndGame) {
            self.armed = None;
        }
        self.version += 1;
    }

    /// Opens the picker on its top row.
    pub fn open(&mut self) {
        self.picker = true;
        self.opened = (self.level, self.training);
        self.asking = None;
        self.focus = Setting::Level;
        self.armed = None;
        self.version += 1;
    }

    /// Esc, B or Select. With the question up, back to the picker.
    /// Otherwise leave it.
    pub fn back(&mut self) {
        if self.asking.is_some() {
            self.asking = None;
            self.version += 1;
        } else {
            self.leave();
        }
    }

    /// Leaves the picker, asking first if that would add to the record,
    /// with Undo highlighted so a reflex press changes nothing.
    fn leave(&mut self) {
        if self.raises_record() {
            self.asking = Some(Choice::Undo);
            self.armed = None;
            self.version += 1;
        } else {
            self.close();
        }
    }

    /// Closes the picker, keeping what was set in it. The record takes the
    /// settings as they are now, so passing through a level on the way to
    /// another does not count as having used it.
    pub fn close(&mut self) {
        self.picker = false;
        self.asking = None;
        self.armed = None;
        self.record.highest = self.record.highest.max(self.level);
        self.record.training |= self.training;
        self.version += 1;
    }

    /// Enter or A. With the question up, it takes the highlighted answer.
    /// On a setting it leaves the picker, as Esc does. On an action the
    /// first press asks for a second, and the second requests the action
    /// and closes the picker.
    pub fn enter(&mut self) {
        if let Some(choice) = self.asking {
            if choice == Choice::Undo {
                (self.level, self.training) = self.opened;
            }
            self.close();
            return;
        }
        let action = match self.focus {
            Setting::Level | Setting::Training => {
                self.leave();
                return;
            }
            Setting::EndGame => Action::EndGame,
            Setting::Exit => Action::Exit,
        };
        if self.armed == Some(self.focus) {
            self.requested = Some(action);
            // An action is not a decision about the settings: anything that
            // would add to the record without being confirmed is undone, so
            // an ended game's score note cannot pick it up by accident.
            if self.raises_record() {
                (self.level, self.training) = self.opened;
            }
            self.close();
        } else {
            self.armed = Some(self.focus);
            self.version += 1;
        }
    }

    /// Up and down in the picker: which row is highlighted. Moving away
    /// from an action that was pressed once cancels it.
    pub fn focus_up(&mut self) {
        self.move_focus(-1);
    }

    pub fn focus_down(&mut self) {
        self.move_focus(1);
    }

    fn move_focus(&mut self, by: isize) {
        if self.asking.is_some() {
            return;
        }
        let rows = self.rows();
        let at = rows.iter().position(|&r| r == self.focus).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, rows.len() as isize - 1) as usize;
        self.focus = rows[to];
        self.armed = None;
        self.version += 1;
    }

    /// Takes the confirmed action, if there is one and it is `which`.
    pub fn take(&mut self, which: Action) -> bool {
        if self.requested == Some(which) {
            self.requested = None;
            true
        } else {
            false
        }
    }

    /// Left and right in the picker: the highlighted setting down or up a
    /// step, in effect at once. It is recorded when the picker closes.
    pub fn change(&mut self, up: bool) {
        if let Some(choice) = &mut self.asking {
            *choice = if up { Choice::Undo } else { Choice::Use };
            self.version += 1;
            return;
        }
        let max = LEVELS.len() as u8 - 1;
        match (self.focus, up) {
            (Setting::Level, true) => self.level = (self.level + 1).min(max),
            (Setting::Level, false) => self.level = self.level.saturating_sub(1),
            (Setting::Training, on) => self.training = on,
            (Setting::EndGame | Setting::Exit, _) => return,
        }
        self.version += 1;
    }

    /// Puts a level into effect and records it, outside the picker: for
    /// tests, and for screenshots taken without a window.
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(LEVELS.len() as u8 - 1);
        self.record.highest = self.record.highest.max(self.level);
        self.version += 1;
    }

    /// Puts training mode into effect or out of it and records it. For
    /// tests, which start from a setting without going through the picker.
    #[cfg(test)]
    pub fn set_training(&mut self, on: bool) {
        self.training = on;
        self.record.training |= on;
        self.version += 1;
    }

    /// A new game has started: its record begins with what is in use now.
    pub fn new_game(&mut self) {
        self.record = Record {
            highest: self.level,
            training: self.training,
        };
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_no_help() {
        let g = Guidance::default();
        assert_eq!(g.level(), 0);
        assert!(!g.training());
        assert_eq!(g.record(), Record::default());
        assert!(!g.picker_open());
    }

    #[test]
    fn the_record_only_rises() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.set_level(1);
        assert_eq!(g.record().highest, 3);
        g.set_training(true);
        g.set_training(false);
        assert!(g.record().training);
    }

    #[test]
    fn changes_are_in_effect_at_once_and_kept_on_closing() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.change(true);
        assert_eq!(g.level(), 2, "in effect at once");
        g.focus_down();
        g.change(true);
        assert!(g.training());
        g.close();
        assert_eq!((g.level(), g.training()), (2, true), "kept");
    }

    #[test]
    fn only_what_is_in_effect_on_closing_is_recorded() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..5 {
            g.change(true);
        }
        assert_eq!(g.record().highest, 0, "not while the picker is open");
        for _ in 0..4 {
            g.change(false);
        }
        g.focus_down();
        g.change(true);
        g.change(false);
        g.close();
        assert_eq!(
            g.record(),
            Record {
                highest: 1,
                training: false
            }
        );
    }

    #[test]
    fn lowering_or_browsing_never_asks() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.open();
        g.change(false);
        g.back();
        assert!(!g.picker_open(), "a lower level: no question");
        assert_eq!(g.level(), 2);

        g.open();
        g.change(true);
        g.change(true);
        g.change(false);
        g.back();
        assert!(!g.picker_open(), "back to level 3, already recorded");
    }

    #[test]
    fn raising_asks_with_undo_highlighted() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.change(true);
        g.back();
        assert!(g.picker_open());
        assert_eq!(g.asking(), Some(Choice::Undo));
        g.enter();
        assert!(!g.picker_open());
        assert_eq!(g.level(), 0, "undone");
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn use_it_keeps_and_records() {
        let mut g = Guidance::default();
        g.open();
        g.focus_down();
        g.change(true);
        g.enter();
        assert_eq!(
            g.asking(),
            Some(Choice::Undo),
            "Enter on a setting asks too"
        );
        g.change(false);
        assert_eq!(g.asking(), Some(Choice::Use));
        g.enter();
        assert!(g.training());
        assert!(g.record().training);
    }

    #[test]
    fn back_from_the_question_returns_to_the_picker() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.back();
        g.back();
        assert!(g.picker_open());
        assert_eq!(g.asking(), None);
        assert_eq!(g.level(), 1, "still changed");
    }

    #[test]
    fn ending_a_game_drops_unconfirmed_raises() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        g.change(true);
        g.focus_down();
        g.focus_down();
        g.enter();
        g.enter();
        assert!(g.take(Action::EndGame));
        assert_eq!(g.level(), 0);
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn levels_stop_at_the_ends() {
        let mut g = Guidance::default();
        g.open();
        g.change(false);
        assert_eq!(g.level(), 0);
        for _ in 0..10 {
            g.change(true);
        }
        assert_eq!(g.level(), 5);
    }

    #[test]
    fn an_action_needs_two_presses() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        g.focus_down();
        g.focus_down();
        assert_eq!(g.focus(), Setting::EndGame);
        g.enter();
        assert_eq!(g.armed(), Some(Setting::EndGame));
        assert!(!g.take(Action::EndGame), "one press does nothing yet");
        g.enter();
        assert!(g.take(Action::EndGame));
        assert!(!g.picker_open(), "the picker closes");
    }

    #[test]
    fn moving_away_cancels_a_first_press() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..5 {
            g.focus_down();
        }
        assert_eq!(g.focus(), Setting::Exit);
        g.enter();
        g.focus_up();
        g.focus_down();
        g.enter();
        assert!(!g.take(Action::Exit), "the first press was cancelled");
    }

    #[test]
    fn the_picker_opens_on_its_top_row() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        for _ in 0..5 {
            g.focus_down();
        }
        g.close();
        g.open();
        assert_eq!(g.focus(), Setting::Level);
    }

    #[test]
    fn ending_a_game_is_offered_only_while_playing() {
        let mut g = Guidance::default();
        assert_eq!(g.rows(), [Setting::Level, Setting::Training, Setting::Exit]);
        g.set_playing(true);
        assert_eq!(
            g.rows(),
            [
                Setting::Level,
                Setting::Training,
                Setting::EndGame,
                Setting::Exit
            ]
        );
    }

    #[test]
    fn a_new_game_starts_its_record_from_what_is_in_use() {
        let mut g = Guidance::default();
        g.set_level(4);
        g.set_training(true);
        g.set_level(2);
        g.set_training(false);
        g.new_game();
        assert_eq!(g.level(), 2, "the chosen level is kept");
        assert_eq!(
            g.record(),
            Record {
                highest: 2,
                training: false
            }
        );
    }
}
