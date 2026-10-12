//! The keyboard reaching the Spectrum: each key by its position, as the Spectrum's own keyboard
//! lays them out.

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
/// where they are; Shift is CAPS SHIFT; Control and Alt are SYMBOL SHIFT; Backspace is DELETE
/// and the arrow keys are the cursor keys.
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
        S::LCtrl | S::RCtrl | S::LAlt | S::RAlt => Key::SymShift,
        S::Backspace => return Some(Spectrum::Compound(CompoundKey::Delete)),
        S::Left => return Some(Spectrum::Compound(CompoundKey::ArrowLeft)),
        S::Right => return Some(Spectrum::Compound(CompoundKey::ArrowRight)),
        S::Up => return Some(Spectrum::Compound(CompoundKey::ArrowUp)),
        S::Down => return Some(Spectrum::Compound(CompoundKey::ArrowDown)),
        _ => return None,
    };
    Some(Spectrum::Key(key))
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
            spectrum_key(Scancode::RCtrl),
            Some(Spectrum::Key(Key::SymShift))
        ));
    }

    #[test]
    fn the_arrows_and_backspace_type_with_caps_shift() {
        assert!(matches!(
            spectrum_key(Scancode::Left),
            Some(Spectrum::Compound(CompoundKey::ArrowLeft))
        ));
        assert!(matches!(
            spectrum_key(Scancode::Backspace),
            Some(Spectrum::Compound(CompoundKey::Delete))
        ));
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
