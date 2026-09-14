//! Keyboard mapping: the host keyboard to the Spectrum's key matrix, and
//! the arrow keys to a Kempston joystick.

use std::collections::HashSet;

use sidekick::Input;
use winit::keyboard::KeyCode;

/// Spectrum matrix position (half-row, bit) for a host key.
///
/// Laid out in matrix order, one arm per key, because that is the order the
/// hardware reads them in. Arms that repeat a position do so because two
/// host keys reach the same Spectrum key; merging them would break the
/// layout and separate the arrow keys from the note explaining them.
#[allow(
    clippy::match_same_arms,
    reason = "the arms are the keyboard's own layout"
)]
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
        ControlLeft | ControlRight => &[(7, 1)],
        KeyM => &[(7, 2)],
        KeyN => &[(7, 3)],
        KeyB => &[(7, 4)],
        // Delete is Caps Shift + 0 on a Spectrum.
        Backspace => &[(0, 0), (4, 0)],
        // The Spectrum's cursor keys are 5, 6, 7 and 8 — the arrows are
        // printed on those very keys — so the host arrows press them as well
        // as moving the joystick below. That makes them work in the cursor
        // control method, and means they type those digits just as the real
        // keys do: an arrow at the title screen picks that menu option.
        ArrowLeft => &[(3, 4)],
        ArrowDown => &[(4, 4)],
        ArrowUp => &[(4, 3)],
        ArrowRight => &[(4, 2)],
        _ => &[],
    }
}

/// Kempston joystick bit for a host key.
///
/// A Kempston interface is a joystick port: the game reads five bits and
/// cannot tell what moved them, so pointing host keys at them is invisible
/// to it. Only keys the Spectrum itself has no use for are used here —
/// Space in particular is a Spectrum key (it is the pause key the game
/// ships with), so it stays out of this and does exactly what it does on
/// the real machine.
fn kempston(key: KeyCode) -> u8 {
    use KeyCode::*;
    match key {
        ArrowRight => 0x01,
        ArrowLeft => 0x02,
        ArrowDown => 0x04,
        ArrowUp => 0x08,
        // Alt is awkward on macOS, so a couple of spare keys fire too.
        AltLeft | AltRight | SuperRight | Period | Comma => 0x10,
        _ => 0,
    }
}

/// Builds the machine's input from the host keys currently held.
///
/// Rebuilding from the whole set rather than flipping one bit per event
/// matters where two host keys share a matrix position: Backspace is Caps
/// Shift + 0 and the arrows are 5, 6, 7 and 8, so releasing one used to
/// report the other released as well.
pub fn build(held: &HashSet<KeyCode>) -> Input {
    let mut input = Input::default();
    for &key in held {
        for &(row, bit) in matrix(key) {
            input.keys[row] &= !(1 << bit);
        }
        input.kempston |= kempston(key);
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
        assert_eq!(input.kempston, 0);
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
            ControlLeft,
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
        assert_eq!(matrix(ControlRight), matrix(ControlLeft));
        assert!(matrix(Escape).is_empty());
    }

    #[test]
    fn the_arrows_press_the_cursor_keys_and_move_the_joystick() {
        let input = held(&[KeyCode::ArrowLeft, KeyCode::ArrowUp]);
        // 5 is bit 4 of half-row 3; 7 is bit 3 of half-row 4.
        assert_eq!(input.keys[3], 0xEF);
        assert_eq!(input.keys[4], 0xF7);
        assert_eq!(input.kempston, 0x02 | 0x08);
        assert_eq!(held(&[KeyCode::ArrowRight]).kempston, 0x01);
        assert_eq!(held(&[KeyCode::ArrowDown]).kempston, 0x04);
    }

    #[test]
    fn fire_keys_press_only_the_joystick() {
        for key in [
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::SuperRight,
            KeyCode::Period,
            KeyCode::Comma,
        ] {
            let input = held(&[key]);
            assert_eq!(input.kempston, 0x10, "{key:?}");
            assert_eq!(input.keys, [0xFF; 8], "{key:?}");
        }
    }

    #[test]
    fn space_is_a_key_not_fire() {
        let input = held(&[KeyCode::Space]);
        assert_eq!((input.keys[7], input.kempston), (0xFE, 0));
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
        // And 5 is still down with the left arrow let go.
        assert_eq!(held(&[KeyCode::Digit5]).keys[3], 0xEF);
    }
}
