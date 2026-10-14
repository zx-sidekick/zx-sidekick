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

/// Whether the Spectrum's keyboard is on in `state`: as the player chose in the menu
/// (`chosen`), else as the definition's `[run] keyboard` and `keyboard_in` say. With no
/// definition (a tape played plainly) it's on.
pub(crate) fn spectrum_on(
    definition: Option<&Definition>,
    state: Option<&str>,
    chosen: Option<bool>,
) -> bool {
    if let Some(on) = chosen {
        return on;
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

/// What the keyboard is in `state`: typing when the definition's `[text_entry]` or
/// `[name_entry]` asks for text there; the Spectrum's keyboard where it's on
/// ([`spectrum_on`]); a pad otherwise, the loading screen included, where Enter is Start.
pub(crate) fn mode(
    definition: Option<&Definition>,
    state: Option<&str>,
    chosen: Option<bool>,
) -> KeyboardMode {
    if asks_for_text(definition, state) {
        KeyboardMode::Typing
    } else if spectrum_on(definition, state, chosen) {
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

/// Everything a key let go may have pressed, in any mode: its Spectrum key, and its pad parts.
/// Letting go of what wasn't pressed does nothing, so whatever the mode was when it went down,
/// nothing stays held.
pub(crate) fn let_go(scancode: Scancode) -> (Option<Spectrum>, Vec<Part>) {
    let mut parts: Vec<Part> = pad_key(scancode).into_iter().collect();
    parts.extend(pad_part(scancode).filter(|p| !parts.contains(p)));
    (spectrum_key(scancode), parts)
}

/// The pad part a key plays as when the keyboard is a pad: the arrows and W A S D are the
/// D-pad; Space, the right Control, the right Alt (Option) and `/` the west button; Z, X, C and
/// V the south, east, west and north buttons, by position; Enter is Start. `/` is for a Mac,
/// whose apps may take a modifier tapped twice for a shortcut of their own, whatever window has
/// focus.
#[must_use]
pub(crate) const fn pad_key(scancode: Scancode) -> Option<Part> {
    use Scancode as S;
    Some(match scancode {
        S::Up | S::W => Part::DPad(Way::Up),
        S::Down | S::S => Part::DPad(Way::Down),
        S::Left | S::A => Part::DPad(Way::Left),
        S::Right | S::D => Part::DPad(Way::Right),
        S::Space | S::RCtrl | S::RAlt | S::Slash | S::C => Part::West,
        S::Z => Part::South,
        S::X => Part::East,
        S::V => Part::North,
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
        Scancode::Up => Some(Part::DPad(Way::Up)),
        Scancode::Down => Some(Part::DPad(Way::Down)),
        Scancode::Left => Some(Part::DPad(Way::Left)),
        Scancode::Right => Some(Part::DPad(Way::Right)),
        Scancode::RCtrl | Scancode::RAlt | Scancode::Slash => Some(Part::West),
        _ => None,
    }
}

/// How a card names the key that plays `part`: with the keyboard a pad ([`pad_key`]), the arrows,
/// Space for west, Z, X and V for south, east and north, and Enter for Start; with the Spectrum's
/// keyboard on (`spectrum`, [`pad_part`]), the arrows and the right Control for fire (a Mac's
/// cards name `/` instead), and no others.
pub(crate) const fn key_for(part: Part, spectrum: bool) -> Option<&'static str> {
    match (part, spectrum) {
        (Part::DPad(Way::Up), _) => Some("↑"),
        (Part::DPad(Way::Down), _) => Some("↓"),
        (Part::DPad(Way::Left), _) => Some("←"),
        (Part::DPad(Way::Right), _) => Some("→"),
        (Part::West, true) => Some("Right Ctrl"),
        (Part::West, false) => Some("Space"),
        (Part::South, false) => Some("Z"),
        (Part::East, false) => Some("X"),
        (Part::North, false) => Some("V"),
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
        let pad = mode(Some(&def("")), Some("play"), None);
        assert_eq!(pad, KeyboardMode::Pad);
        for (keys, want) in [
            (&[Scancode::Up, Scancode::W][..], Part::DPad(Way::Up)),
            (&[Scancode::Down, Scancode::S], Part::DPad(Way::Down)),
            (&[Scancode::Left, Scancode::A], Part::DPad(Way::Left)),
            (&[Scancode::Right, Scancode::D], Part::DPad(Way::Right)),
            (
                &[
                    Scancode::Space,
                    Scancode::RCtrl,
                    Scancode::RAlt,
                    Scancode::Slash,
                    Scancode::C,
                ],
                Part::West,
            ),
            (&[Scancode::Z], Part::South),
            (&[Scancode::X], Part::East),
            (&[Scancode::V], Part::North),
            (&[Scancode::Return, Scancode::KpEnter], Part::Start),
        ] {
            for &key in keys {
                assert_eq!(part(key, &pad), Some(want), "{key:?}");
            }
        }
        for key in [
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
        let plain = mode(None, None, None);
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
        assert_eq!(part(Scancode::Left, &plain), Some(Part::DPad(Way::Left)));
        assert_eq!(part(Scancode::RCtrl, &plain), Some(Part::West));
        assert!(pressed(Scancode::Escape, &plain).is_none());
    }

    #[test]
    fn a_definition_turns_the_spectrum_keyboard_on_throughout_or_in_its_states() {
        let throughout = def("[run]\nkeyboard = \"spectrum\"");
        for state in [None, Some("define"), Some("play")] {
            assert_eq!(
                mode(Some(&throughout), state, None),
                KeyboardMode::Spectrum,
                "{state:?}"
            );
        }
        let some = def("[run]\nkeyboard = \"spectrum\"\nkeyboard_in = [\"define\"]");
        assert_eq!(
            mode(Some(&some), Some("define"), None),
            KeyboardMode::Spectrum
        );
        assert_eq!(mode(Some(&some), Some("play"), None), KeyboardMode::Pad);
        assert_eq!(mode(Some(&some), None, None), KeyboardMode::Pad);
    }

    #[test]
    fn the_player_s_choice_outranks_the_definition() {
        let pad = def("");
        assert!(spectrum_on(Some(&pad), Some("play"), Some(true)));
        assert_eq!(
            mode(Some(&pad), Some("play"), Some(true)),
            KeyboardMode::Spectrum
        );
        let on = def("[run]\nkeyboard = \"spectrum\"");
        assert!(!spectrum_on(Some(&on), Some("play"), Some(false)));
        assert!(!spectrum_on(None, None, Some(false)), "plain play too");
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
        for chosen in [None, Some(false), Some(true)] {
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
        let (key, parts) = let_go(Scancode::W);
        assert!(matches!(key, Some(Spectrum::Key(Key::W))));
        assert_eq!(parts, [Part::DPad(Way::Up)]);
        let (key, parts) = let_go(Scancode::Up);
        assert!(key.is_none());
        assert_eq!(parts, [Part::DPad(Way::Up)], "once");
        let (key, parts) = let_go(Scancode::Q);
        assert!(matches!(key, Some(Spectrum::Key(Key::Q))));
        assert!(parts.is_empty());
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
            let mode = mode(self.s.definition(), self.known(), None);
            match pressed(scancode, &mode) {
                Some(KeyDoes::Spectrum(Spectrum::Key(key))) => self.s.machine_mut().key(key, true),
                Some(KeyDoes::Pad(part)) if self.pads.button(0, part, true).is_some() => {
                    if part == Part::Start && self.s.machine().waits_on_loading_screen() {
                        self.play.start_on_loading_screen();
                    } else {
                        self.play.pressed(part);
                    }
                }
                _ => {}
            }
        }

        fn frames(&mut self, n: usize) {
            for _ in 0..n {
                let known = self.s.progress().last_known.clone();
                self.pads
                    .hold_back_parts(&self.play.enter(known.as_deref()));
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

    #[test]
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn starquake_plays_from_the_keyboard_as_a_pad_with_the_spectrum_keyboard_off() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zip = repo.join("assets/Starquake.tap.zip");
        if !zip.exists() {
            eprintln!("skipped: no {} (the player's own tape)", zip.display());
            return;
        }
        let tapes = sidekick_def::tapes_in(&zip).expect("the zip holds a tape");
        let package = sidekick_def::Package::load(&repo.join("games/starquake"))
            .expect("Starquake's definition");
        let mut machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
        let mut k = Played {
            play: Play::new(Mapping::new(Some(&package.definition))),
            s: sidekick_core::Session::new(machine, Some(package.definition)),
            pads: Pads::default(),
        };
        k.frames(400);
        assert!(k.s.on_loading_screen(), "the loader waits for a key");
        k.tap(Scancode::Q, 4);
        k.frames(50);
        assert!(
            k.s.on_loading_screen(),
            "Q does nothing: the keyboard is a pad"
        );
        k.tap(Scancode::Return, 4);
        assert!(
            k.until(1500, "title"),
            "any key goes on from the loading screen"
        );
        // The title is passed only every few frames: Enter starts a game in a frame between.
        let between = (0..200).any(|_| {
            k.frames(1);
            k.s.state().is_none()
        });
        assert!(between && k.known() == Some("title"), "a frame between");
        assert_eq!(mode(k.s.definition(), k.known(), None), KeyboardMode::Pad);

        k.tap(Scancode::Return, 4);
        let mut seen = Vec::new();
        for _ in 0..300 {
            k.frames(1);
            seen.push(k.s.state().map(str::to_owned));
        }
        assert!(
            seen[100..].iter().all(Option::is_none),
            "Enter, the pad's Start, starts a game: its story shows, untouched, for seconds"
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
        k.key(Scancode::D, true);
        k.frames(40);
        k.key(Scancode::D, false);
        assert!(k.blob_x() > left, "D moves him right");

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
            (Scancode::Z, "Z"),
            (Scancode::X, "X"),
            (Scancode::V, "V"),
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
        assert_eq!(key_for(Part::South, true), None, "Z is the Spectrum's then");
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
