//! Keyboard mapping: the host keyboard to the Spectrum's key matrix, and
//! the arrow keys with a fire key to the joystick, which the machine presses
//! however the game's chosen control method listens.

use std::collections::HashSet;

use sidekick::Input;
use sidekick::machine::{JOY_DOWN, JOY_FIRE, JOY_LEFT, JOY_RIGHT, JOY_UP};
use winit::keyboard::KeyCode;

/// Spectrum matrix position (half-row, bit) for a host key.
///
/// Laid out in matrix order, one arm per key, because that is the order the
/// hardware reads them in.
fn matrix(key: KeyCode) -> &'static [(usize, u8)] {
    use KeyCode::*;
    match key {
        ShiftLeft | ShiftRight => &[(0, 0)],
        KeyZ => &[(0, 1)],
        KeyX => &[(0, 2)],
        KeyC => &[(0, 3)],
        KeyV => &[(0, 4)],
        KeyA => &[(1, 0)],
        KeyS => &[(1, 1)],
        KeyD => &[(1, 2)],
        KeyF => &[(1, 3)],
        KeyG => &[(1, 4)],
        KeyQ => &[(2, 0)],
        KeyW => &[(2, 1)],
        KeyE => &[(2, 2)],
        KeyR => &[(2, 3)],
        KeyT => &[(2, 4)],
        Digit1 => &[(3, 0)],
        Digit2 => &[(3, 1)],
        Digit3 => &[(3, 2)],
        Digit4 => &[(3, 3)],
        Digit5 => &[(3, 4)],
        Digit0 => &[(4, 0)],
        Digit9 => &[(4, 1)],
        Digit8 => &[(4, 2)],
        Digit7 => &[(4, 3)],
        Digit6 => &[(4, 4)],
        KeyP => &[(5, 0)],
        KeyO => &[(5, 1)],
        KeyI => &[(5, 2)],
        KeyU => &[(5, 3)],
        KeyY => &[(5, 4)],
        Enter => &[(6, 0)],
        KeyL => &[(6, 1)],
        KeyK => &[(6, 2)],
        KeyJ => &[(6, 3)],
        KeyH => &[(6, 4)],
        Space => &[(7, 0)],
        // Left Control is the joystick's fire, below.
        ControlRight => &[(7, 1)],
        KeyM => &[(7, 2)],
        KeyN => &[(7, 3)],
        KeyB => &[(7, 4)],
        // Delete is Caps Shift + 0 on a Spectrum.
        Backspace => &[(0, 0), (4, 0)],
        _ => &[],
    }
}

/// Joystick bit for a host key, in the Kempston port's order.
///
/// The arrows and the fire keys are joystick only: they press no key of
/// the Spectrum's, so they cannot type a digit or a letter on the title
/// screen, and in play the machine presses whatever the chosen control
/// method listens for. Space stays a key (it is the pause key the game
/// ships with) and does exactly what it does on the real machine.
fn joystick(key: KeyCode) -> u8 {
    use KeyCode::*;
    match key {
        ArrowRight => JOY_RIGHT,
        ArrowLeft => JOY_LEFT,
        ArrowDown => JOY_DOWN,
        ArrowUp => JOY_UP,
        // Left Control fires; Alt is awkward on macOS, so a couple of spare
        // keys fire too.
        ControlLeft | AltLeft | AltRight | SuperRight | Period | Comma => JOY_FIRE,
        _ => 0,
    }
}

/// Builds the machine's input from the host keys currently held.
///
/// Rebuilding from the whole set rather than flipping one bit per event
/// matters where two host keys share a position: Backspace is Caps Shift
/// with 0, and every fire key is the same bit, so releasing one used to
/// report the other released as well.
pub fn build(held: &HashSet<KeyCode>) -> Input {
    let mut input = Input::default();
    for &key in held {
        for &(row, bit) in matrix(key) {
            input.keys[row] &= !(1 << bit);
        }
        input.joystick |= joystick(key);
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(keys: &[KeyCode]) -> Input {
        build(&keys.iter().copied().collect())
    }

    #[test]
    fn nothing_held_is_nothing_pressed() {
        assert_eq!(held(&[]), Input::default());
    }

    #[test]
    fn a_letter_presses_its_place_on_the_matrix() {
        // Q is bit 0 of half-row 2; M is bit 2 of half-row 7.
        let input = held(&[KeyCode::KeyQ, KeyCode::KeyM]);
        assert_eq!(input.keys[2], 0xFE);
        assert_eq!(input.keys[7], 0xFB);
        assert_eq!(input.joystick, 0);
    }

    #[test]
    fn every_key_of_the_spectrum_has_one_host_key() {
        use KeyCode::*;
        let host = [
            ShiftLeft,
            KeyZ,
            KeyX,
            KeyC,
            KeyV,
            KeyA,
            KeyS,
            KeyD,
            KeyF,
            KeyG,
            KeyQ,
            KeyW,
            KeyE,
            KeyR,
            KeyT,
            Digit1,
            Digit2,
            Digit3,
            Digit4,
            Digit5,
            Digit0,
            Digit9,
            Digit8,
            Digit7,
            Digit6,
            KeyP,
            KeyO,
            KeyI,
            KeyU,
            KeyY,
            Enter,
            KeyL,
            KeyK,
            KeyJ,
            KeyH,
            Space,
            ControlRight,
            KeyM,
            KeyN,
            KeyB,
        ];
        let mut seen = std::collections::HashSet::new();
        for key in host {
            let &[(row, bit)] = matrix(key) else {
                panic!("{key:?} should press one key");
            };
            assert!(row < 8 && bit < 5, "{key:?}");
            assert!(seen.insert((row, bit)), "{key:?} shares a key");
        }
        assert_eq!(seen.len(), 40);
        assert_eq!(matrix(ShiftRight), matrix(ShiftLeft));
        assert!(matrix(Escape).is_empty());
    }

    #[test]
    fn the_arrows_move_the_joystick_and_press_no_key() {
        let input = held(&[KeyCode::ArrowLeft, KeyCode::ArrowUp]);
        assert_eq!(input.joystick, JOY_LEFT | JOY_UP);
        assert_eq!(input.keys, [0xFF; 8]);
        assert_eq!(held(&[KeyCode::ArrowRight]).joystick, JOY_RIGHT);
        assert_eq!(held(&[KeyCode::ArrowDown]).joystick, JOY_DOWN);
    }

    #[test]
    fn fire_keys_press_only_the_joystick() {
        for key in [
            KeyCode::ControlLeft,
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::SuperRight,
            KeyCode::Period,
            KeyCode::Comma,
        ] {
            let input = held(&[key]);
            assert_eq!(input.joystick, JOY_FIRE, "{key:?}");
            assert_eq!(input.keys, [0xFF; 8], "{key:?}");
        }
    }

    #[test]
    fn right_control_is_symbol_shift_and_left_control_is_not() {
        assert_eq!(held(&[KeyCode::ControlRight]).keys[7], 0xFD);
        assert_eq!(held(&[KeyCode::ControlLeft]).keys[7], 0xFF);
    }

    #[test]
    fn space_is_a_key_not_fire() {
        let input = held(&[KeyCode::Space]);
        assert_eq!((input.keys[7], input.joystick), (0xFE, 0));
    }

    #[test]
    fn backspace_is_caps_shift_and_zero() {
        let input = held(&[KeyCode::Backspace]);
        assert_eq!((input.keys[0], input.keys[4]), (0xFE, 0xFE));
    }

    #[test]
    fn letting_go_of_one_of_two_keys_on_the_same_place_keeps_it_pressed() {
        // Shift and Backspace both press Caps Shift: with Backspace let go,
        // Shift still holds it.
        let input = held(&[KeyCode::ShiftLeft]);
        assert_eq!(input.keys[0], 0xFE);
        assert_eq!(input.keys[4], 0xFF);
        // And fire is still held with one of two fire keys let go.
        assert_eq!(held(&[KeyCode::Comma]).joystick, JOY_FIRE);
    }
}
