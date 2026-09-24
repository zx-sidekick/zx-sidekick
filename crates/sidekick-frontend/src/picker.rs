//! Driving a game's picker from the keyboard and the pad, the same in every
//! game (#25, #148): up and down choose a row, left and right change it,
//! Enter or the confirm button does the highlighted thing, and Esc, the
//! cancel button or Select go back.

use winit::keyboard::KeyCode;

use crate::gamepad::Pad;

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
