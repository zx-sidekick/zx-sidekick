//! Manic Miner's rules for the machine (#142): what the machine does at the
//! game's own points. The pause keys are kept from the game's pause read, so
//! the game never waits in its own pause loop and the window freezes the
//! emulation instead, as it does for Starquake; and Start on a pad presses
//! ENTER where the title screen and its tune read it, so a game can be
//! started without a keyboard. Everything else the game reads for itself:
//! its keys from the keyboard, and the Kempston joystick from its port.

use zx_spectrum::Zx;

use crate::facts::{PAUSE_KEYS, reads};

/// ENTER, which starts a game from the title screen.
const ENTER: zx_spectrum::Key = zx_spectrum::Key::Matrix(6, 0);

/// Manic Miner's [`sidekick::Rules`].
#[derive(Clone, Debug, Default)]
pub struct Play {
    /// Whether Start is held on a pad: ENTER at the title screen's reads.
    pub start: bool,
    /// Whether a pause key was pressed during the last frame: down at the
    /// game's pause read and not at the read before. The keys are kept from
    /// the game, which never pauses itself; the window freezes instead.
    pub pause_pressed: bool,
    /// Whether a pause key was down at the last pause read.
    pause_was_down: bool,
    /// The pause keys kept from the game at this read, to give back after it.
    kept: u8,
}

impl sidekick::Rules for Play {
    fn before_frame(&mut self, _z: &Zx) {
        self.pause_pressed = false;
    }

    fn at(&mut self, z: &mut Zx, pc: u16) {
        let (row, bits) = PAUSE_KEYS;
        if pc == reads::PAUSE {
            let down = !z.keys[row] & bits;
            self.pause_pressed |= down != 0 && !self.pause_was_down;
            self.pause_was_down = down != 0;
            // Kept for the read, and given back at the next instruction.
            z.keys[row] |= down;
            self.kept = down;
        } else if self.kept != 0 {
            z.keys[row] &= !self.kept;
            self.kept = 0;
        }
        if self.start && (pc == reads::TITLE_ENTER || pc == reads::TUNE_ENTER) {
            z.set_key(ENTER, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Machine;

    /// A machine at `pc` with `code` there, then a jump to itself.
    fn at(pc: u16, code: &[u8]) -> Machine {
        let mut m = Machine::blank(pc, 0xFF00);
        let p = usize::from(pc);
        m.zx.mem[p..p + code.len()].copy_from_slice(code);
        m.zx.mem[p + code.len()..p + code.len() + 2].copy_from_slice(&[0x18, 0xFE]);
        m
    }

    /// `IN A,(C)` with BC set for the pause row, as the game reads it.
    fn pause_read() -> Machine {
        let mut m = at(reads::PAUSE, &[0xED, 0x78]);
        m.zx.set_bc(0xFDFE);
        m
    }

    #[test]
    fn a_pause_key_is_kept_from_the_game_and_a_fresh_press_is_reported() {
        let mut m = pause_read();
        m.zx.set_key(zx_spectrum::Key::by_name("a").unwrap(), true);
        m.run_frame();
        assert!(m.rules.pause_pressed);
        assert_eq!(m.zx.a() & 0x1F, 0x1F, "the game read no key down");
        assert_eq!(m.zx.keys[1] & 0x01, 0, "given back after the read");
        // Still held: not a new press.
        m.run_frame();
        assert!(!m.rules.pause_pressed);
    }

    #[test]
    fn with_no_pause_key_nothing_is_reported() {
        let mut m = pause_read();
        m.run_frame();
        assert!(!m.rules.pause_pressed);
    }

    #[test]
    fn start_is_enter_only_where_the_title_reads_it() {
        let mut m = at(reads::TITLE_ENTER, &[0x00]);
        m.rules.start = true;
        m.run_frame();
        assert_eq!(m.zx.keys[6] & 0x01, 0, "ENTER at the title's read");
        let mut m = at(0x9000, &[0x00]);
        m.rules.start = true;
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8], "nothing elsewhere");
    }
}
