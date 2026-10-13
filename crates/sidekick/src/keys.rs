//! The keyboard reaching the Spectrum: each key by its position, as the Spectrum's own keyboard
//! lays them out.

use crate::pad::{Part, Way};
use sdl3::keyboard::Scancode;
use sidekick_core::{CompoundKey, Key};

/// What a key on the computer's keyboard is on the Spectrum.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Spectrum {
    /// One of its keys.
    Key(Key),
    /// A key it types with CAPS SHIFT held (the cursor keys, DELETE).
    Compound(CompoundKey),
}

/// The Spectrum key at this position, if there is one. Letters, digits, ENTER and SPACE are
/// where they are; Shift is CAPS SHIFT; the left Control and Alt (Option) are SYMBOL SHIFT;
/// Backspace is DELETE. The arrows and the right Control and Alt are a pad's ([`pad_part`]).
#[must_use]
pub(crate) const fn spectrum_key(scancode: Scancode) -> Option<Spectrum> {
    use Scancode as S;
    let key = match scancode {
        S::A => Key::A,
        S::B => Key::B,
        S::C => Key::C,
        S::D => Key::D,
        S::E => Key::E,
        S::F => Key::F,
        S::G => Key::G,
        S::H => Key::H,
        S::I => Key::I,
        S::J => Key::J,
        S::K => Key::K,
        S::L => Key::L,
        S::M => Key::M,
        S::N => Key::N,
        S::O => Key::O,
        S::P => Key::P,
        S::Q => Key::Q,
        S::R => Key::R,
        S::S => Key::S,
        S::T => Key::T,
        S::U => Key::U,
        S::V => Key::V,
        S::W => Key::W,
        S::X => Key::X,
        S::Y => Key::Y,
        S::Z => Key::Z,
        S::_1 => Key::N1,
        S::_2 => Key::N2,
        S::_3 => Key::N3,
        S::_4 => Key::N4,
        S::_5 => Key::N5,
        S::_6 => Key::N6,
        S::_7 => Key::N7,
        S::_8 => Key::N8,
        S::_9 => Key::N9,
        S::_0 => Key::N0,
        S::Return | S::KpEnter => Key::Enter,
        S::Space => Key::Space,
        S::LShift | S::RShift => Key::Shift,
        S::LCtrl | S::LAlt => Key::SymShift,
        S::Backspace => return Some(Spectrum::Compound(CompoundKey::Delete)),
        _ => return None,
    };
    Some(Spectrum::Key(key))
}

/// The pad part a key the Spectrum doesn't have plays as: the arrows are the D-pad, and the
/// right Control, the right Alt (Option) and `/` the fire button (`west`), so they do what the
/// definition maps those to, as a plugged-in pad does. `/` is for a Mac, whose apps may take a
/// modifier tapped twice for a shortcut of their own, whatever window has focus.
#[must_use]
pub(crate) const fn pad_part(scancode: Scancode) -> Option<Part> {
    match scancode {
        Scancode::Up => Some(Part::DPad(Way::Up)),
        Scancode::Down => Some(Part::DPad(Way::Down)),
        Scancode::Left => Some(Part::DPad(Way::Left)),
        Scancode::Right => Some(Part::DPad(Way::Right)),
        Scancode::RCtrl | Scancode::RAlt | Scancode::Slash => Some(Part::West),
        _ => None,
    }
}

/// A Spectrum key by the name definitions give it: `A`, `1`, `ENTER`, `SPACE`, `CAPS SHIFT`,
/// `SYMBOL SHIFT`.
pub(crate) fn key_named(name: &str) -> Option<Key> {
    let key = match name {
        "1" => Key::N1,
        "2" => Key::N2,
        "3" => Key::N3,
        "4" => Key::N4,
        "5" => Key::N5,
        "6" => Key::N6,
        "7" => Key::N7,
        "8" => Key::N8,
        "9" => Key::N9,
        "0" => Key::N0,
        "Q" => Key::Q,
        "W" => Key::W,
        "E" => Key::E,
        "R" => Key::R,
        "T" => Key::T,
        "Y" => Key::Y,
        "U" => Key::U,
        "I" => Key::I,
        "O" => Key::O,
        "P" => Key::P,
        "A" => Key::A,
        "S" => Key::S,
        "D" => Key::D,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "ENTER" => Key::Enter,
        "CAPS SHIFT" => Key::Shift,
        "Z" => Key::Z,
        "X" => Key::X,
        "C" => Key::C,
        "V" => Key::V,
        "B" => Key::B,
        "N" => Key::N,
        "M" => Key::M,
        "SYMBOL SHIFT" => Key::SymShift,
        "SPACE" => Key::Space,
        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_the_spectrum_s_own_keys_are_where_they_are() {
        assert!(matches!(
            spectrum_key(Scancode::Q),
            Some(Spectrum::Key(Key::Q))
        ));
        assert!(matches!(
            spectrum_key(Scancode::_0),
            Some(Spectrum::Key(Key::N0))
        ));
        assert!(matches!(
            spectrum_key(Scancode::Return),
            Some(Spectrum::Key(Key::Enter))
        ));
        assert!(matches!(
            spectrum_key(Scancode::LShift),
            Some(Spectrum::Key(Key::Shift))
        ));
        assert!(matches!(
            spectrum_key(Scancode::LCtrl),
            Some(Spectrum::Key(Key::SymShift))
        ));
        assert!(matches!(
            spectrum_key(Scancode::LAlt),
            Some(Spectrum::Key(Key::SymShift))
        ));
    }

    #[test]
    fn backspace_types_delete_with_caps_shift() {
        assert!(matches!(
            spectrum_key(Scancode::Backspace),
            Some(Spectrum::Compound(CompoundKey::Delete))
        ));
    }

    #[test]
    fn the_arrows_the_right_ctrl_and_option_and_the_slash_play_as_a_pad() {
        assert_eq!(pad_part(Scancode::Left), Some(Part::DPad(Way::Left)));
        assert_eq!(pad_part(Scancode::Up), Some(Part::DPad(Way::Up)));
        assert_eq!(pad_part(Scancode::RCtrl), Some(Part::West));
        assert_eq!(pad_part(Scancode::RAlt), Some(Part::West));
        assert_eq!(pad_part(Scancode::Slash), Some(Part::West));
        assert_eq!(pad_part(Scancode::LCtrl), None, "SYMBOL SHIFT");
        assert_eq!(pad_part(Scancode::Q), None);
        for key in [
            Scancode::Left,
            Scancode::Right,
            Scancode::Up,
            Scancode::Down,
            Scancode::RCtrl,
            Scancode::RAlt,
            Scancode::Slash,
        ] {
            assert!(
                spectrum_key(key).is_none(),
                "{key:?} reaches no Spectrum key"
            );
        }
    }

    #[test]
    fn keys_go_by_the_names_definitions_use() {
        assert!(matches!(key_named("SPACE"), Some(Key::Space)));
        assert!(matches!(key_named("CAPS SHIFT"), Some(Key::Shift)));
        assert!(matches!(key_named("7"), Some(Key::N7)));
        assert!(
            key_named("space").is_none(),
            "names are as the format writes them"
        );
        assert!(key_named("F1").is_none());
    }

    #[test]
    fn the_program_s_own_keys_reach_nothing() {
        for key in [
            Scancode::Escape,
            Scancode::F5,
            Scancode::F9,
            Scancode::F11,
            Scancode::LGui,
        ] {
            assert!(spectrum_key(key).is_none(), "{key:?}");
        }
    }
}
