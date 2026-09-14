//! Pausing, as the window does it (#25, decision 12): Start on a controller,
//! or the game's own pause key, freezes the emulation between frames in
//! play, and a key, a direction, fire or Start continues. The game never
//! pauses itself; the machine keeps its pause key from it and reports the
//! press instead.

/// What is held on the host at one poll.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    /// Start on a controller.
    pub start: bool,
    /// The Spectrum's keyboard half-rows, a 0 bit a key down.
    pub keys: [u8; 8],
    /// The joystick bits, from the pad and the keyboard.
    pub joystick: u8,
}

impl Default for Held {
    fn default() -> Self {
        Held {
            start: false,
            keys: [0xFF; 8],
            joystick: 0,
        }
    }
}

#[derive(Default)]
pub struct Freeze {
    frozen: bool,
    /// What was held at the last poll, to tell a press from a hold.
    was: Held,
}

impl Freeze {
    /// Takes one poll before a frame: what is held `now`, whether the game's
    /// pause key was pressed in the last frame (`pause`), and whether a game
    /// is being played, the only time it freezes. Returns whether the
    /// emulation is frozen, and so whether to skip the frame.
    pub fn poll(&mut self, now: Held, pause: bool, playing: bool) -> bool {
        let start = now.start && !self.was.start;
        let pressed = now.joystick & !self.was.joystick != 0
            || now
                .keys
                .iter()
                .zip(self.was.keys)
                .any(|(&n, w)| !n & w & 0x1F != 0);
        if !playing {
            self.frozen = false;
        } else if self.frozen {
            self.frozen = !(start || pressed);
        } else {
            self.frozen = start || pause;
        }
        self.was = now;
        self.frozen
    }

    /// Ends a freeze without a press: the game is being ended.
    pub fn thaw(&mut self) {
        self.frozen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> Held {
        Held {
            start: true,
            ..Held::default()
        }
    }

    fn key(row: usize, bit: u8) -> Held {
        let mut held = Held::default();
        held.keys[row] &= !(1 << bit);
        held
    }

    #[test]
    fn start_freezes_a_game_and_starts_it_again() {
        let mut f = Freeze::default();
        assert!(!f.poll(Held::default(), false, true));
        assert!(f.poll(start(), false, true), "pressed: frozen");
        assert!(f.poll(start(), false, true), "still held: still frozen");
        assert!(f.poll(Held::default(), false, true), "let go: still frozen");
        assert!(!f.poll(start(), false, true), "pressed again: going");
        assert!(!f.poll(start(), false, true), "held: not frozen again");
    }

    #[test]
    fn the_pause_key_freezes_and_a_key_direction_or_fire_continues() {
        let mut f = Freeze::default();
        assert!(f.poll(key(7, 0), true, true), "the game's pause key");
        assert!(f.poll(Held::default(), false, true), "let go: still frozen");
        assert!(!f.poll(key(7, 0), false, true), "pressed again: going");
        let mut f = Freeze::default();
        f.poll(Held::default(), true, true);
        assert!(!f.poll(key(2, 0), false, true), "a key");
        let mut f = Freeze::default();
        f.poll(Held::default(), true, true);
        let fire = Held {
            joystick: 0x10,
            ..Held::default()
        };
        assert!(!f.poll(fire, false, true), "fire");
    }

    #[test]
    fn a_key_held_as_it_froze_does_not_continue_until_pressed_again() {
        let mut f = Freeze::default();
        let right = Held {
            joystick: 0x01,
            start: true,
            ..Held::default()
        };
        assert!(f.poll(right, false, true));
        let still = Held {
            joystick: 0x01,
            ..Held::default()
        };
        assert!(f.poll(still, false, true), "the direction was already down");
        assert!(f.poll(Held::default(), false, true));
        assert!(!f.poll(still, false, true), "pressed again");
    }

    #[test]
    fn only_a_game_being_played_freezes() {
        let mut f = Freeze::default();
        assert!(!f.poll(start(), true, false), "the title screen");
        let mut f = Freeze::default();
        f.poll(start(), false, true);
        assert!(!f.poll(start(), false, false), "the game left play");
        let mut f = Freeze::default();
        f.poll(start(), false, true);
        f.thaw();
        assert!(
            !f.poll(start(), false, true),
            "thawed, and Start still held"
        );
    }
}
