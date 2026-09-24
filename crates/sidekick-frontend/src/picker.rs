//! Driving a game's picker from the keyboard and the pad, the same in every
//! game (#25, #148): up and down choose a row, left and right change it,
//! Enter or the confirm button does the highlighted thing, and Esc, the
//! cancel button or Select go back.

use winit::keyboard::KeyCode;

use crate::gamepad::Pad;

/// What every game's picker keeps the same way (#178): whether it is open,
/// the highlighted row, the action pressed once and waiting for its second
/// press, and a version that moves on every change so the window redraws.
/// The rows, and what they change, are the game's: `R` is its row type,
/// and the rows are passed in, since a game's can change with its state.
#[derive(Clone, Debug, Default)]
pub struct State<R> {
    open: bool,
    focus: R,
    armed: Option<R>,
    version: u64,
}

impl<R: Copy + Eq> State<R> {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Moves whenever anything the picker's state shows does.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The highlighted row.
    pub fn focus(&self) -> R {
        self.focus
    }

    /// The action pressed once, waiting for its second press.
    pub fn armed(&self) -> Option<R> {
        self.armed
    }

    /// Opens on `top`, with nothing armed.
    pub fn open(&mut self, top: R) {
        self.open = true;
        self.focus = top;
        self.armed = None;
        self.touch();
    }

    /// Closes, with nothing armed.
    pub fn close(&mut self) {
        self.open = false;
        self.armed = None;
        self.touch();
    }

    /// Moves the highlight `by` rows through `rows`, stopping at either end;
    /// moving away from an action pressed once cancels it.
    pub fn move_focus(&mut self, rows: &[R], by: isize) {
        let Some(last) = rows.len().checked_sub(1) else {
            return;
        };
        let at = rows.iter().position(|&r| r == self.focus).unwrap_or(0);
        let to = at.saturating_add_signed(by).min(last);
        self.focus = rows[to];
        self.armed = None;
        self.touch();
    }

    /// Highlights `row` without moving through the others: for a row that
    /// is gone, such as End this game once the game is over.
    pub fn set_focus(&mut self, row: R) {
        self.focus = row;
        self.touch();
    }

    /// A press of an action's row: the first arms it and returns false, the
    /// second, on the same row, disarms it and returns true: do it.
    pub fn press(&mut self, row: R) -> bool {
        self.touch();
        if self.armed == Some(row) {
            self.armed = None;
            true
        } else {
            self.armed = Some(row);
            false
        }
    }

    /// Cancels the action pressed once, if any.
    pub fn disarm(&mut self) {
        if self.armed.take().is_some() {
            self.touch();
        }
    }

    /// Marks a change the game made to what its picker shows.
    pub fn touch(&mut self) {
        self.version += 1;
    }
}

/// What a picker does with the keys it is given.
pub trait Picker {
    /// Goes back: closes the picker, keeping nothing chosen in it.
    fn back(&mut self);
    fn focus_up(&mut self);
    fn focus_down(&mut self);
    /// Changes the highlighted row, up or down.
    fn change(&mut self, up: bool);
    /// Does the highlighted thing.
    fn enter(&mut self);
}

/// The picker's side of a key, while it is open: whether the key was one of
/// its own.
pub fn key(picker: &mut impl Picker, code: KeyCode) -> bool {
    match code {
        KeyCode::Escape => picker.back(),
        KeyCode::ArrowUp => picker.focus_up(),
        KeyCode::ArrowDown => picker.focus_down(),
        KeyCode::ArrowLeft => picker.change(false),
        KeyCode::ArrowRight => picker.change(true),
        KeyCode::Enter | KeyCode::NumpadEnter => picker.enter(),
        _ => return false,
    }
    true
}

/// The picker's side of a poll of the pad, while it is open.
pub fn pad(picker: &mut impl Picker, pad: &Pad) {
    if pad.select || pad.cancel() {
        picker.back();
    }
    if pad.up {
        picker.focus_up();
    }
    if pad.down {
        picker.focus_down();
    }
    if pad.left {
        picker.change(false);
    }
    if pad.right {
        picker.change(true);
    }
    if pad.confirm() {
        picker.enter();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_focus_stops_at_either_end_and_moving_away_disarms() {
        let rows = [1, 2, 3];
        let mut s = State::default();
        s.open(1);
        s.move_focus(&rows, -1);
        assert_eq!(s.focus(), 1, "stops at the top");
        s.move_focus(&rows, 5);
        assert_eq!(s.focus(), 3, "and at the bottom");
        assert!(!s.press(3), "the first press arms");
        assert_eq!(s.armed(), Some(3));
        s.move_focus(&rows, -1);
        assert_eq!(s.armed(), None, "moving away disarms");
        s.move_focus(&rows, 1);
        assert!(!s.press(3));
        assert!(s.press(3), "the second press does it");
        assert_eq!(s.armed(), None);
    }

    #[test]
    fn every_change_moves_the_version_and_closing_disarms() {
        let mut s = State::default();
        let mut last = s.version();
        let mut moved = |s: &State<u8>| {
            let now = s.version();
            let did = now != last;
            last = now;
            did
        };
        s.open(0);
        assert!(moved(&s) && s.is_open());
        s.press(0);
        assert!(moved(&s));
        s.close();
        assert!(moved(&s) && !s.is_open());
        assert_eq!(s.armed(), None);
        s.disarm();
        assert!(!moved(&s), "nothing to disarm, nothing changed");
        s.set_focus(2);
        assert!(moved(&s));
        assert_eq!(s.focus(), 2);
    }

    #[derive(Default)]
    struct Log(Vec<&'static str>);

    impl Picker for Log {
        fn back(&mut self) {
            self.0.push("back");
        }
        fn focus_up(&mut self) {
            self.0.push("up");
        }
        fn focus_down(&mut self) {
            self.0.push("down");
        }
        fn change(&mut self, up: bool) {
            self.0.push(if up { "more" } else { "less" });
        }
        fn enter(&mut self) {
            self.0.push("enter");
        }
    }

    #[test]
    fn the_keys_and_the_pad_do_the_same_things() {
        let mut log = Log::default();
        for code in [
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::Enter,
            KeyCode::Escape,
        ] {
            assert!(key(&mut log, code));
        }
        assert!(!key(&mut log, KeyCode::KeyA), "not the picker's");
        let keys = std::mem::take(&mut log.0);
        for p in [
            Pad {
                up: true,
                ..Pad::default()
            },
            Pad {
                down: true,
                ..Pad::default()
            },
            Pad {
                left: true,
                ..Pad::default()
            },
            Pad {
                right: true,
                ..Pad::default()
            },
            Pad {
                south: true,
                ..Pad::default()
            },
            Pad {
                east: true,
                ..Pad::default()
            },
        ] {
            pad(&mut log, &p);
        }
        assert_eq!(log.0, keys);
    }
}
