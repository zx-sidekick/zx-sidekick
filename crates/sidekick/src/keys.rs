//! The computer's keyboard in play: a pad by default, through the definition's `[pad]`; the
//! Spectrum's own keyboard, each key by its position, where the definition or the player turns
//! it on; and the letters a game asks for, typed straight into it.

use crate::entry;
use crate::pad::{Part, Way};
use sdl3::keyboard::Scancode;
use sidekick_core::{CompoundKey, Key};
use sidekick_def::{Definition, Keyboard};

/// What the keyboard is, at a moment of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyboardMode {
    /// A pad ([`pad_key`]); no Spectrum key reaches the game.
    Pad,
    /// The Spectrum's own keyboard ([`spectrum_key`]); the keys it doesn't have are a pad's
    /// ([`pad_part`]).
    Spectrum,
    /// The game asks for text: every Spectrum key reaches it, ENTER and DELETE included, and
    /// nothing plays as a pad.
    Typing,
}

/// What a key pressed does.
#[derive(Clone, Copy, Debug)]
pub(crate) enum KeyDoes {
    /// Presses a Spectrum key.
    Spectrum(Spectrum),
    /// Presses a pad's part.
    Pad(Part),
}

/// Whether the Spectrum's keyboard is on in `state`: turned on in the menu for the session
/// (`forced`), or as the definition's `[run] keyboard` and `keyboard_in` say. With no
/// definition (a tape played plainly) it's on.
pub(crate) fn spectrum_on(
    definition: Option<&Definition>,
    state: Option<&str>,
    forced: bool,
) -> bool {
    if forced {
        return true;
    }
    let Some(definition) = definition else {
        return true;
    };
    definition.run.as_ref().is_some_and(|run| {
        run.keyboard == Keyboard::Spectrum
            && (run.keyboard_in.is_empty()
                || state.is_some_and(|s| run.keyboard_in.iter().any(|k| k == s)))
    })
}

/// Whether the definition has the Spectrum's keyboard off in any state: everywhere but in plain
/// play and with `keyboard = "spectrum"` throughout, so the menu's toggle can turn it on.
pub(crate) fn spectrum_off_anywhere(definition: Option<&Definition>) -> bool {
    definition.is_some_and(|d| {
        d.run
            .as_ref()
            .is_none_or(|run| run.keyboard == Keyboard::Pad || !run.keyboard_in.is_empty())
    })
}

/// What the keyboard is in `state`: typing when the definition's `[text_entry]` or
/// `[name_entry]` asks for text there; the Spectrum's keyboard where it's on
/// ([`spectrum_on`]); a pad otherwise, the loading screen included, where Enter is Start.
pub(crate) fn mode(
    definition: Option<&Definition>,
    state: Option<&str>,
    forced: bool,
) -> KeyboardMode {
    if asks_for_text(definition, state) {
        KeyboardMode::Typing
    } else if spectrum_on(definition, state, forced) {
        KeyboardMode::Spectrum
    } else {
        KeyboardMode::Pad
    }
}

/// Whether the game asks for text in `state`, by its `[text_entry]` or `[name_entry]`: the
/// joystick waits then, the pad driving the text card and the keyboard typing.
pub(crate) fn asks_for_text(definition: Option<&Definition>, state: Option<&str>) -> bool {
    definition
        .and_then(|d| entry::section_for(d, state))
        .is_some()
}

/// What a key pressed does in `mode`, if anything.
pub(crate) fn pressed(scancode: Scancode, mode: &KeyboardMode) -> Option<KeyDoes> {
    match mode {
        KeyboardMode::Pad => pad_key(scancode).map(KeyDoes::Pad),
        KeyboardMode::Spectrum => pad_part(scancode)
            .map(KeyDoes::Pad)
            .or_else(|| spectrum_key(scancode).map(KeyDoes::Spectrum)),
        KeyboardMode::Typing => spectrum_key(scancode).map(KeyDoes::Spectrum),
    }
}

/// What a key pressed does in `mode`, as [`pressed`] says; and where `any` (the state's
/// `[pad]` has `any`, or a loader waits for a key, and no shortcut is under way), a key that
/// does nothing else while the keyboard is a pad plays as [`any_key`]'s part.
pub(crate) fn pressed_or_any(
    scancode: Scancode,
    mode: &KeyboardMode,
    any: bool,
) -> Option<KeyDoes> {
    pressed(scancode, mode).or_else(|| {
        (any && *mode == KeyboardMode::Pad)
            .then(|| any_key(scancode))
            .flatten()
            .map(KeyDoes::Pad)
    })
}

/// Everything a key let go may have pressed, in any mode: its Spectrum key, and its pad parts.
/// Letting go of what wasn't pressed does nothing, so whatever the mode was when it went down,
/// nothing stays held.
pub(crate) fn let_go(scancode: Scancode) -> (Option<Spectrum>, Vec<Part>) {
    let mut parts: Vec<Part> = pad_key(scancode).into_iter().collect();
    parts.extend(pad_part(scancode).filter(|p| !parts.contains(p)));
    parts.extend(any_key(scancode));
    (spectrum_key(scancode), parts)
}

/// Whether a key goes on from a loading screen, with Cmd or the Windows key held (`shortcut`)
/// or not: a key that plays as a button, or any other ([`any_key`]); not a direction's.
pub(crate) fn key_goes_on(scancode: Scancode, shortcut: bool) -> bool {
    !shortcut && pad_key(scancode).map_or_else(|| any_key(scancode).is_some(), Part::goes_on)
}

/// The part a key the pad doesn't have plays as where a state's `[pad]` has `any`, or on a
/// loading screen, the keyboard a pad: letters, digits and the rest, but not Esc (the guidance
/// menu), the F keys (the program's own) or a modifier, which may begin a shortcut.
pub(crate) fn any_key(scancode: Scancode) -> Option<Part> {
    use Scancode as S;
    if pad_key(scancode).is_some()
        || matches!(
            scancode,
            S::Escape
                | S::F1
                | S::F2
                | S::F3
                | S::F4
                | S::F5
                | S::F6
                | S::F7
                | S::F8
                | S::F9
                | S::F10
                | S::F11
                | S::F12
                | S::F13
                | S::F14
                | S::F15
                | S::F16
                | S::F17
                | S::F18
                | S::F19
                | S::F20
                | S::F21
                | S::F22
                | S::F23
                | S::F24
                | S::LCtrl
                | S::RCtrl
                | S::LShift
                | S::RShift
                | S::LAlt
                | S::RAlt
                | S::LGui
                | S::RGui
                | S::CapsLock
                | S::Mode
        )
    {
        return None;
    }
    u16::try_from(scancode.to_i32()).ok().map(Part::Key)
}

/// The pad part a key plays as when the keyboard is a pad: the arrows (which play as the D-pad
/// unless `[pad] arrows` says otherwise), Space the west button (fire), Enter Start. No other
/// key plays (#194); Esc and the F keys are the program's.
#[must_use]
pub(crate) const fn pad_key(scancode: Scancode) -> Option<Part> {
    use Scancode as S;
    Some(match scancode {
        S::Up => Part::Arrow(Way::Up),
        S::Down => Part::Arrow(Way::Down),
        S::Left => Part::Arrow(Way::Left),
        S::Right => Part::Arrow(Way::Right),
        S::Space => Part::West,
        S::Return | S::KpEnter => Part::Start,
        _ => return None,
    })
}

/// The Spectrum key that types a letter of a `[text_entry]`: `A`, `7`, a space on SPACE.
pub(crate) fn key_typing(letter: char) -> Option<Key> {
    match letter {
        ' ' => Some(Key::Space),
        c => key_named(&c.to_ascii_uppercase().to_string()),
    }
}

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
/// Only while the Spectrum's keyboard is on ([`KeyboardMode`]).
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

/// The pad part a key the Spectrum doesn't have plays as while the Spectrum's keyboard is on:
/// the arrows are the D-pad, and the right Control, the right Alt (Option) and `/` the fire
/// button (`west`), so they do what the definition maps those to, as a plugged-in pad does.
#[must_use]
pub(crate) const fn pad_part(scancode: Scancode) -> Option<Part> {
    match scancode {
        Scancode::Up => Some(Part::Arrow(Way::Up)),
        Scancode::Down => Some(Part::Arrow(Way::Down)),
        Scancode::Left => Some(Part::Arrow(Way::Left)),
        Scancode::Right => Some(Part::Arrow(Way::Right)),
        Scancode::RCtrl | Scancode::RAlt | Scancode::Slash => Some(Part::West),
        _ => None,
    }
}

/// How a card names the key that plays `part`: with the keyboard a pad ([`pad_key`]), the arrows,
/// Space for west and Enter for Start; with the Spectrum's
/// keyboard on (`spectrum`, [`pad_part`]), the arrows and the right Control for fire (a Mac's
/// cards name `/` instead), and no others.
pub(crate) const fn key_for(part: Part, spectrum: bool) -> Option<&'static str> {
    match (part, spectrum) {
        (Part::Arrow(Way::Up), _) => Some("↑"),
        (Part::Arrow(Way::Down), _) => Some("↓"),
        (Part::Arrow(Way::Left), _) => Some("←"),
        (Part::Arrow(Way::Right), _) => Some("→"),
        (Part::West, true) => Some("Right Ctrl"),
        (Part::West, false) => Some("Space"),
        (Part::Start, false) => Some("Enter"),
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
    use crate::pad::{Input, Mapping, Pads, Play};

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
        assert_eq!(pad_part(Scancode::Left), Some(Part::Arrow(Way::Left)));
        assert_eq!(pad_part(Scancode::Up), Some(Part::Arrow(Way::Up)));
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

    fn def(toml: &str) -> Definition {
        let base = r#"
            schema = 1
            [game]
            id = "test"
            title = "Test"
            machine = "48k"
            [[media]]
            name = "the tape"
            [[states]]
            name = "define"
            pc = 0x8000
            [[states]]
            name = "booth"
            pc = 0x8100
            [[states]]
            name = "name"
            pc = 0x8200
            [[states]]
            name = "play"
            pc = 0x9000
        "#;
        Definition::from_toml(&format!("{base}\n{toml}")).expect("the test definition parses")
    }

    fn spectrum(scancode: Scancode, mode: &KeyboardMode) -> Option<Key> {
        match pressed(scancode, mode) {
            Some(KeyDoes::Spectrum(Spectrum::Key(key))) => Some(key),
            _ => None,
        }
    }

    fn part(scancode: Scancode, mode: &KeyboardMode) -> Option<Part> {
        match pressed(scancode, mode) {
            Some(KeyDoes::Pad(part)) => Some(part),
            _ => None,
        }
    }

    #[test]
    fn by_default_the_keyboard_plays_as_a_pad_and_no_spectrum_key_reaches_the_game() {
        let pad = mode(Some(&def("")), Some("play"), false);
        assert_eq!(pad, KeyboardMode::Pad);
        for (keys, want) in [
            (&[Scancode::Up][..], Part::Arrow(Way::Up)),
            (&[Scancode::Down], Part::Arrow(Way::Down)),
            (&[Scancode::Left], Part::Arrow(Way::Left)),
            (&[Scancode::Right], Part::Arrow(Way::Right)),
            (&[Scancode::Space], Part::West),
            (&[Scancode::Return, Scancode::KpEnter], Part::Start),
        ] {
            for &key in keys {
                assert_eq!(part(key, &pad), Some(want), "{key:?}");
            }
        }
        // Only the cursor keys and Space play (#194).
        for key in [
            Scancode::W,
            Scancode::A,
            Scancode::S,
            Scancode::D,
            Scancode::Z,
            Scancode::X,
            Scancode::C,
            Scancode::V,
            Scancode::RCtrl,
            Scancode::RAlt,
            Scancode::Slash,
            Scancode::Q,
            Scancode::_0,
            Scancode::LShift,
            Scancode::LCtrl,
            Scancode::Backspace,
            Scancode::Escape,
            Scancode::F5,
            Scancode::F9,
            Scancode::F11,
        ] {
            assert!(pressed(key, &pad).is_none(), "{key:?}");
        }
    }

    #[test]
    fn played_plainly_the_spectrum_keyboard_is_on_and_the_arrows_are_the_joystick() {
        let plain = mode(None, None, false);
        assert_eq!(plain, KeyboardMode::Spectrum);
        assert!(matches!(spectrum(Scancode::Q, &plain), Some(Key::Q)));
        assert!(matches!(spectrum(Scancode::W, &plain), Some(Key::W)));
        assert!(matches!(
            spectrum(Scancode::Space, &plain),
            Some(Key::Space)
        ));
        assert!(matches!(
            spectrum(Scancode::Return, &plain),
            Some(Key::Enter)
        ));
        assert!(matches!(
            pressed(Scancode::Backspace, &plain),
            Some(KeyDoes::Spectrum(Spectrum::Compound(CompoundKey::Delete)))
        ));
        assert_eq!(part(Scancode::Left, &plain), Some(Part::Arrow(Way::Left)));
        assert_eq!(part(Scancode::RCtrl, &plain), Some(Part::West));
        assert!(pressed(Scancode::Escape, &plain).is_none());
    }

    #[test]
    fn a_definition_turns_the_spectrum_keyboard_on_throughout_or_in_its_states() {
        let throughout = def("[run]\nkeyboard = \"spectrum\"");
        for state in [None, Some("define"), Some("play")] {
            assert_eq!(
                mode(Some(&throughout), state, false),
                KeyboardMode::Spectrum,
                "{state:?}"
            );
        }
        let some = def("[run]\nkeyboard = \"spectrum\"\nkeyboard_in = [\"define\"]");
        assert_eq!(
            mode(Some(&some), Some("define"), false),
            KeyboardMode::Spectrum
        );
        assert_eq!(mode(Some(&some), Some("play"), false), KeyboardMode::Pad);
        assert_eq!(mode(Some(&some), None, false), KeyboardMode::Pad);
    }

    #[test]
    fn the_menu_s_toggle_is_for_a_game_with_the_spectrum_keyboard_off_anywhere() {
        assert!(spectrum_off_anywhere(Some(&def(""))), "a pad by default");
        assert!(spectrum_off_anywhere(Some(&def("[run]\nsync = 0x8000"))));
        assert!(!spectrum_off_anywhere(Some(&def(
            "[run]\nkeyboard = \"spectrum\""
        ))));
        assert!(spectrum_off_anywhere(Some(&def(
            "[run]\nkeyboard = \"spectrum\"\nkeyboard_in = [\"define\"]"
        ))));
        assert!(!spectrum_off_anywhere(None), "played plainly");
    }

    #[test]
    fn the_player_can_turn_the_spectrum_keyboard_on_but_not_off() {
        let pad = def("");
        assert!(spectrum_on(Some(&pad), Some("play"), true));
        assert_eq!(mode(Some(&pad), Some("play"), true), KeyboardMode::Spectrum);
        let on = def("[run]\nkeyboard = \"spectrum\"");
        assert!(spectrum_on(Some(&on), Some("play"), false), "the game's on");
        assert!(spectrum_on(None, None, false), "plain play's on");
    }

    const TEXT: &str = r#"
        [text_entry]
        state = "booth"
        letters = "ABC"
        length = 3
        [name_entry]
        state = "name"
        letters = "AB0 "
        length = 3
    "#;

    #[test]
    fn asked_for_text_the_spectrum_keyboard_is_on_and_nothing_plays_as_a_pad() {
        let text = def(TEXT);
        assert!(
            asks_for_text(Some(&text), Some("booth")),
            "the joystick waits"
        );
        assert!(!asks_for_text(Some(&text), Some("play")));
        assert!(!asks_for_text(None, None), "played plainly");
        for chosen in [false, true] {
            for state in ["booth", "name"] {
                let typing = mode(Some(&text), Some(state), chosen);
                assert_eq!(typing, KeyboardMode::Typing, "{state} {chosen:?}");
                assert!(matches!(spectrum(Scancode::A, &typing), Some(Key::A)));
                assert!(
                    matches!(spectrum(Scancode::D, &typing), Some(Key::D)),
                    "any letter"
                );
                assert!(matches!(spectrum(Scancode::_0, &typing), Some(Key::N0)));
                assert!(matches!(
                    spectrum(Scancode::Space, &typing),
                    Some(Key::Space)
                ));
                assert!(matches!(
                    spectrum(Scancode::Return, &typing),
                    Some(Key::Enter)
                ));
                assert!(matches!(
                    pressed(Scancode::Backspace, &typing),
                    Some(KeyDoes::Spectrum(Spectrum::Compound(CompoundKey::Delete)))
                ));
                for key in [
                    Scancode::Left,
                    Scancode::RCtrl,
                    Scancode::Slash,
                    Scancode::Escape,
                ] {
                    assert!(pressed(key, &typing).is_none(), "no pad: {key:?}");
                }
            }
        }
    }

    #[test]
    fn a_key_let_go_lets_go_of_whatever_it_may_have_pressed() {
        let (key, parts) = let_go(Scancode::Space);
        assert!(matches!(key, Some(Spectrum::Key(Key::Space))));
        assert_eq!(parts, [Part::West]);
        let (key, parts) = let_go(Scancode::RCtrl);
        assert!(key.is_none());
        assert_eq!(parts, [Part::West], "fire, with the Spectrum's keyboard on");
        let (key, parts) = let_go(Scancode::Up);
        assert!(key.is_none());
        assert_eq!(parts, [Part::Arrow(Way::Up)], "once");
        let (key, parts) = let_go(Scancode::Q);
        assert!(matches!(key, Some(Spectrum::Key(Key::Q))));
        assert!(
            matches!(parts[..], [Part::Key(_)]),
            "what a state's `any` made it"
        );
    }

    /// Starquake from its real tape, played from the keyboard as the window plays it: each key
    /// through [`mode`] and [`pressed`], its pad parts through the definition's `[pad]`.
    struct Played {
        s: sidekick_core::Session,
        play: Play,
        pads: Pads,
    }

    impl Played {
        fn key(&mut self, scancode: Scancode, down: bool) {
            if !down {
                let (key, parts) = let_go(scancode);
                if let Some(Spectrum::Key(key)) = key {
                    self.s.machine_mut().key(key, false);
                }
                for part in parts {
                    self.pads.button(0, part, false);
                }
                return;
            }
            self.enter();
            let mode = mode(self.s.definition(), self.known(), false);
            let any = self.play.takes_any_key() || self.s.machine().waits_on_loading_screen();
            match pressed_or_any(scancode, &mode, any) {
                Some(KeyDoes::Spectrum(Spectrum::Key(key))) => self.s.machine_mut().key(key, true),
                Some(KeyDoes::Pad(part)) => self.button(0, part, true),
                _ => {}
            }
        }

        /// The state last known, for what's pressed now, as the window takes an event.
        fn enter(&mut self) {
            let known = self.s.progress().last_known.clone();
            let s = &self.s;
            self.pads
                .hold_back_parts(&self.play.enter(known.as_deref(), &|t| s.holds(t)));
        }

        /// A part of pad `pad` pressed or let go, as the window takes it.
        fn button(&mut self, pad: u32, part: Part, down: bool) {
            self.enter();
            if self.pads.button(pad, part, down).is_some() {
                let loading = self.s.machine().waits_on_loading_screen();
                self.play.press(part, loading);
            }
        }

        /// The game as it is, played on from here apart from this one.
        fn fork(&self) -> Self {
            let mut play = Play::new(Mapping::new(self.s.definition()));
            play.enter(self.known(), &|t| self.s.holds(t));
            Self {
                s: self.s.clone(),
                play,
                pads: Pads::default(),
            }
        }

        fn frames(&mut self, n: usize) {
            for _ in 0..n {
                self.enter();
                let changes = self.play.update(self.pads.held());
                let machine = self.s.machine_mut();
                for (input, down) in changes
                    .release
                    .into_iter()
                    .map(|i| (i, false))
                    .chain(changes.press.into_iter().map(|i| (i, true)))
                {
                    match input {
                        Input::Kempston(d) => machine.kempston(d.kempston(), down),
                        Input::Key(key) => machine.key(key.0, down),
                    }
                }
                self.s.run_frame().expect("a frame");
                self.play.frame_ran();
            }
        }

        /// Holds `scancode` for `frames`, then lets it go.
        fn tap(&mut self, scancode: Scancode, frames: usize) {
            self.key(scancode, true);
            self.frames(frames);
            self.key(scancode, false);
        }

        fn until(&mut self, most: usize, state: &str) -> bool {
            (0..most).any(|_| {
                self.frames(1);
                self.s.state() == Some(state)
            })
        }

        /// The state last known, as the window maps the keyboard and the pad by.
        fn known(&self) -> Option<&str> {
            self.s.progress().last_known.as_deref()
        }

        fn blob_x(&self) -> i64 {
            self.s.reading()["blob_x"].number().expect("BLOB's x")
        }
    }

    /// The first record of a table value, by its field: its address, from the definition.
    fn field_addr(s: &sidekick_core::Session, table: &str, field: &str) -> u16 {
        let value = &s.definition().expect("its definition").values[table];
        value.addr.expect("an address").0
            + u16::try_from(value.fields[field].offset).expect("a small offset")
    }

    /// Starquake in play, entered into room `room` with BLOB at (`x`, `height`), the first item
    /// laid there first when `item`, creatures harmless.
    fn starquake_in(room: i64, x: i64, height: i64, item: bool) -> Option<Played> {
        let mut k = starquake()?;
        crate::situations::start(&mut k.s, &sidekick_def::Situation::default())
            .expect("a game starts");
        if item {
            // The room's low byte, its ninth bit, and y 0: placed on entering the room.
            let low = u8::try_from(room & 0xFF).expect("a byte");
            let high = if room >= 256 { 0x80 } else { 0 };
            let at = field_addr(&k.s, "items", "room_low");
            k.s.machine_mut().write(at, low);
            let at = field_addr(&k.s, "items", "second");
            k.s.machine_mut().write(at, high);
        }
        let situation = sidekick_def::Situation::from_toml(&format!(
            "room = {room}\nblob_x = {x}\nblob_y = {height}\ncheats = [\"no-harm\"]\n"
        ))
        .expect("it parses");
        crate::situations::enter(&mut k.s, &situation).expect("the room");
        k.frames(30);
        Some(k)
    }

    impl Played {
        fn number(&self, name: &str) -> i64 {
            self.s.reading()[name].number().expect(name)
        }

        /// The first carried slot's color: 0 while nothing is carried.
        fn carried(&self) -> u8 {
            let at = field_addr(&self.s, "carried", "color");
            self.s.machine().peek(at)
        }

        /// Holds a pad's part for `frames`, then lets it go.
        fn hold(&mut self, part: Part, frames: usize) {
            self.button(0, part, true);
            self.frames(frames);
            self.button(0, part, false);
            self.frames(10);
        }
    }

    #[test]
    fn on_foot_the_pad_s_up_and_down_do_nothing_but_its_buttons_and_the_arrows_do() {
        // Room 399's one place for an item, where BLOB meets it from x 200 (docs/re/starquake.md).
        let Some(k) = starquake_in(399, 200, 135, true) else {
            return;
        };
        assert_eq!(k.number("blob_mode"), 0, "on foot");
        assert_eq!(k.carried(), 0);
        for part in [Part::DPad(Way::Up), Part::Stick(Way::Up)] {
            let mut g = k.fork();
            g.hold(part, 15);
            assert_eq!(g.carried(), 0, "{part:?} picks nothing up");
        }
        let mut g = k.fork();
        g.hold(Part::East, 15);
        assert_ne!(g.carried(), 0, "east picks it up");
        let mut g = k.fork();
        g.tap(Scancode::Up, 15);
        g.frames(10);
        assert_ne!(g.carried(), 0, "the up arrow picks it up");

        let platforms = k.number("platforms");
        for part in [Part::DPad(Way::Down), Part::Stick(Way::Down)] {
            let mut g = k.fork();
            g.hold(part, 15);
            assert_eq!(g.number("platforms"), platforms, "{part:?} builds nothing");
        }
        let mut g = k.fork();
        g.hold(Part::South, 15);
        assert_eq!(g.number("platforms"), platforms - 2, "south builds");
        let mut g = k.fork();
        g.tap(Scancode::Down, 15);
        g.frames(10);
        assert_eq!(
            g.number("platforms"),
            platforms - 2,
            "the down arrow builds"
        );

        let mut g = k.fork();
        let x = g.number("blob_x");
        g.hold(Part::DPad(Way::Left), 10);
        assert!(g.number("blob_x") < x, "the D-pad still walks");
    }

    #[test]
    fn on_a_hover_platform_the_pad_s_up_and_down_fly_it() {
        // Room 234's hover pad, where its platform waits (games/starquake/tests/hover-pad.toml).
        let Some(base) = starquake_in(234, 72, 87, false) else {
            return;
        };
        let mut east = base.fork();
        east.hold(Part::East, 10);
        assert_eq!(east.number("blob_mode"), 2, "east boards it");
        let mut away = base.fork();
        away.s.write("blob_x", 88).expect("writable");
        away.frames(10);
        let height = away.number("blob_y");
        away.hold(Part::DPad(Way::Up), 15);
        assert_eq!(
            away.number("blob_mode"),
            0,
            "a step from the pad, the D-pad's up does nothing"
        );
        assert_eq!(away.number("blob_y"), height);
        let mut k = base.fork();
        k.hold(Part::DPad(Way::Up), 10);
        assert_eq!(
            k.number("blob_mode"),
            2,
            "at the pad, the D-pad's up boards it"
        );
        let height = k.number("blob_y");
        let mut g = k.fork();
        g.hold(Part::DPad(Way::Up), 15);
        let up = g.number("blob_y");
        assert!(up > height, "the D-pad's up flies up: {height} to {up}");
        g.hold(Part::DPad(Way::Down), 10);
        assert!(g.number("blob_y") < up, "and down flies down");
        let mut g = k.fork();
        g.hold(Part::Stick(Way::Up), 15);
        assert!(g.number("blob_y") > height, "so does the stick");
        let mut g = k.fork();
        g.tap(Scancode::Up, 15);
        assert!(g.number("blob_y") > height, "and the up arrow");
    }

    /// Starquake from its real tape, played as the window plays it; `None`, saying so, without
    /// the player's tape in `assets/`.
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn starquake() -> Option<Played> {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zip = repo.join("assets/Starquake.tap.zip");
        if !zip.exists() {
            eprintln!("skipped: no {} (the player's own tape)", zip.display());
            return None;
        }
        let tapes = sidekick_def::tapes_in(&zip).expect("the zip holds a tape");
        let package = sidekick_def::Package::load(&repo.join("games/starquake"))
            .expect("Starquake's definition");
        let mut machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
        Some(Played {
            play: Play::new(Mapping::new(Some(&package.definition))),
            s: sidekick_core::Session::new(machine, Some(package.definition)),
            pads: Pads::default(),
        })
    }

    /// A stand-in tape's loader waiting for a key, played as the window plays it.
    fn at_a_loader_s_wait() -> Played {
        let s = crate::pad::tests::at_a_loader_s_wait();
        Played {
            play: Play::new(Mapping::new(s.definition())),
            s,
            pads: Pads::default(),
        }
    }

    #[test]
    fn any_key_goes_on_from_a_loader_s_wait_letters_and_digits_too() {
        let waiting = at_a_loader_s_wait();
        assert_eq!(
            mode(waiting.s.definition(), waiting.known(), false),
            KeyboardMode::Pad
        );
        for key in [
            Scancode::Return,
            Scancode::KpEnter,
            Scancode::Space,
            Scancode::Z,
            Scancode::X,
            Scancode::C,
            Scancode::V,
            Scancode::Slash,
            Scancode::Q,
            Scancode::M,
            Scancode::_1,
            Scancode::Tab,
            Scancode::Backspace,
            // Not directions since #194: only the arrows are.
            Scancode::W,
            Scancode::A,
            Scancode::S,
            Scancode::D,
        ] {
            let mut k = waiting.fork();
            k.tap(key, 4);
            let went_on = (0..50).any(|_| {
                k.frames(1);
                !k.s.on_loading_screen()
            });
            assert!(went_on, "{key:?} goes on");
        }
    }

    #[test]
    fn a_direction_esc_an_f_key_or_a_modifier_doesn_t_go_on_from_a_loader_s_wait() {
        let waiting = at_a_loader_s_wait();
        for key in [
            Scancode::Left,
            Scancode::Up,
            // Modifiers since #194, no longer fire.
            Scancode::RCtrl,
            Scancode::RAlt,
            Scancode::Escape,
            Scancode::F1,
            Scancode::LShift,
        ] {
            let mut k = waiting.fork();
            k.tap(key, 4);
            k.frames(50);
            assert!(k.s.on_loading_screen(), "{key:?} doesn't reach the game");
        }
    }

    #[test]
    fn where_a_state_takes_any_key_every_key_but_esc_the_f_keys_and_the_modifiers_does() {
        let pad = KeyboardMode::Pad;
        for key in [
            Scancode::Q,
            Scancode::_0,
            Scancode::Tab,
            Scancode::Backspace,
        ] {
            assert!(pressed_or_any(key, &pad, false).is_none(), "{key:?}");
            assert!(
                matches!(
                    pressed_or_any(key, &pad, true),
                    Some(KeyDoes::Pad(Part::Key(_)))
                ),
                "{key:?}"
            );
        }
        assert_eq!(
            part(Scancode::Left, &pad),
            Some(Part::Arrow(Way::Left)),
            "the pad's own keys stay its parts"
        );
        for key in [
            Scancode::Escape,
            Scancode::F1,
            Scancode::F5,
            Scancode::F9,
            Scancode::F11,
            Scancode::F12,
            Scancode::LShift,
            Scancode::LCtrl,
            Scancode::LAlt,
            Scancode::LGui,
            Scancode::RGui,
            Scancode::CapsLock,
        ] {
            assert!(pressed_or_any(key, &pad, true).is_none(), "{key:?}");
        }
        assert!(
            matches!(
                pressed_or_any(Scancode::Q, &KeyboardMode::Spectrum, true),
                Some(KeyDoes::Spectrum(Spectrum::Key(Key::Q)))
            ),
            "the Spectrum's keyboard on, the letters are its"
        );
    }

    /// A key tapped, or a pad's part.
    enum How {
        Key(Scancode),
        Button(Part),
    }

    #[test]
    fn starquake_s_title_starts_a_game_on_any_key_or_button_but_a_direction() {
        let Some(mut k) = starquake() else {
            return;
        };
        k.frames(400);
        assert!(k.s.on_loading_screen(), "the loader waits for a key");
        // West fires a Kempston joystick in play, which the loader's wait doesn't read.
        k.button(1, Part::West, true);
        k.frames(4);
        k.button(1, Part::West, false);
        assert!(
            k.until(1500, "title"),
            "a face button goes on from the loading screen"
        );
        k.enter();
        assert!(k.play.takes_any_key());
        let press = |g: &mut Played, how: &How| match *how {
            How::Key(key) => g.tap(key, 4),
            How::Button(part) => {
                g.button(1, part, true);
                g.frames(4);
                g.button(1, part, false);
            }
        };
        let after = |g: &mut Played| -> Vec<Option<String>> {
            (0..300)
                .map(|_| {
                    g.frames(1);
                    g.s.state().map(str::to_owned)
                })
                .collect()
        };
        for (what, how) in [
            ("Esc", How::Key(Scancode::Escape)),
            ("an F key", How::Key(Scancode::F1)),
            ("an arrow", How::Key(Scancode::Left)),
            ("the D-pad", How::Button(Part::DPad(Way::Down))),
            ("the stick", How::Button(Part::Stick(Way::Right))),
        ] {
            let mut g = k.fork();
            press(&mut g, &how);
            let seen = after(&mut g);
            assert!(
                seen[100..].iter().any(|s| s.as_deref() == Some("title")),
                "{what} does nothing: still the title"
            );
        }

        for (what, how) in [
            ("a letter", How::Key(Scancode::Q)),
            ("W, no direction since #194", How::Key(Scancode::W)),
            ("a digit", How::Key(Scancode::_5)),
            ("Space", How::Key(Scancode::Space)),
            ("a face button", How::Button(Part::West)),
            ("Start", How::Button(Part::Start)),
        ] {
            let mut g = k.fork();
            press(&mut g, &how);
            let seen = after(&mut g);
            assert!(
                seen[100..].iter().all(Option::is_none),
                "{what} starts a game: its story shows"
            );
            g.tap(Scancode::M, 4);
            assert!(
                g.until(100, "play"),
                "after {what}, a key goes on from the story"
            );
        }
    }

    #[test]
    fn starquake_plays_from_the_keyboard_as_a_pad_with_the_spectrum_keyboard_off() {
        let Some(mut k) = starquake() else {
            return;
        };
        k.frames(400);
        assert!(k.s.on_loading_screen(), "the loader waits for a key");
        k.tap(Scancode::Left, 4);
        k.frames(50);
        assert!(k.s.on_loading_screen(), "a direction does nothing");
        k.tap(Scancode::Q, 4);
        assert!(
            k.until(1500, "title"),
            "a letter goes on from the loading screen"
        );
        // The title is passed only every few frames: Enter starts a game in a frame between.
        let between = (0..200).any(|_| {
            k.frames(1);
            k.s.state().is_none()
        });
        assert!(between && k.known() == Some("title"), "a frame between");
        assert_eq!(mode(k.s.definition(), k.known(), false), KeyboardMode::Pad);

        k.tap(Scancode::Return, 4);
        let mut seen = Vec::new();
        for _ in 0..300 {
            k.frames(1);
            seen.push(k.s.state().map(str::to_owned));
        }
        assert!(
            seen[100..].iter().all(Option::is_none),
            "Enter starts a game, as any key does on the title: its story shows, untouched, for seconds"
        );
        k.tap(Scancode::Return, 4);
        assert!(k.until(100, "play"), "Enter goes on from the story");
        let paused = (0..300).any(|_| {
            k.frames(1);
            k.s.state() == Some("paused")
        });
        assert!(!paused, "and doesn't pause the game it starts");
        let before = k.blob_x();
        k.key(Scancode::Left, true);
        k.frames(40);
        k.key(Scancode::Left, false);
        let left = k.blob_x();
        assert!(
            left < before,
            "the left arrow moves BLOB left on the joystick"
        );
        k.key(Scancode::Right, true);
        k.frames(40);
        k.key(Scancode::Right, false);
        let right = k.blob_x();
        assert!(right > left, "the right arrow moves him right");
        k.key(Scancode::D, true);
        k.frames(40);
        k.key(Scancode::D, false);
        assert_eq!(k.blob_x(), right, "D doesn't: only the cursor keys play");

        // The game's own keys don't reach it: A S D F G together would end the game.
        let together = [
            Scancode::A,
            Scancode::S,
            Scancode::D,
            Scancode::F,
            Scancode::G,
        ];
        for key in together {
            k.key(key, true);
        }
        k.frames(20);
        for key in together {
            k.key(key, false);
        }
        assert_ne!(
            k.s.state(),
            Some("game_over"),
            "the Spectrum keyboard is off"
        );

        k.tap(Scancode::Return, 2);
        assert!(k.until(50, "paused"), "Enter pauses, tapping SPACE");
        k.frames(20);
        assert_eq!(k.s.state(), Some("paused"), "and stays paused");
        k.tap(Scancode::Right, 4);
        assert!(k.until(50, "play"), "a move goes on");
    }

    #[test]
    fn a_card_names_the_keys_that_play_as_a_pad() {
        let named = |scancode, spectrum| {
            let part = if spectrum {
                pad_part(scancode)
            } else {
                pad_key(scancode)
            };
            part.and_then(|p| key_for(p, spectrum))
        };
        for (scancode, name) in [
            (Scancode::Up, "↑"),
            (Scancode::Left, "←"),
            (Scancode::Space, "Space"),
            (Scancode::Return, "Enter"),
        ] {
            assert_eq!(named(scancode, false), Some(name), "{scancode:?}, as a pad");
        }
        for (scancode, name) in [(Scancode::Down, "↓"), (Scancode::RCtrl, "Right Ctrl")] {
            assert_eq!(
                named(scancode, true),
                Some(name),
                "{scancode:?}, the Spectrum's on"
            );
        }
        assert_eq!(
            key_for(Part::South, false),
            None,
            "no key plays a face button but west"
        );
        assert_eq!(key_for(Part::South, true), None);
        assert_eq!(key_for(Part::L1, false), None, "no key plays it");
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
