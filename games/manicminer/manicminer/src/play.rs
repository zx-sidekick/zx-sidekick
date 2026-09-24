//! Manic Miner's rules for the machine (#142, #148): what the machine does at
//! the game's own points. The pause keys are kept from the game's pause
//! read, so the game never waits in its own pause loop and the window
//! freezes the emulation instead, as it does for Starquake; Start on a pad
//! presses ENTER where the title screen and its tune read it, so a game can
//! be started without a keyboard; training mode steers the game at the one
//! instruction it decides each thing with; and going to a cavern types the
//! game's own cheat at the reads that follow it. Nothing is ever written
//! into the game: a switch changes a register, a flag or where the program
//! goes next, and a key is pressed only for the one read. Everything else
//! the game reads for itself: its keys from the keyboard, and the Kempston
//! joystick from its port.

use zx_spectrum::{CF, Key, ZF, Zx};

use crate::facts::{PAUSE_KEYS, at, cheat, reads, routine, steer, unseen};
use crate::preview::Way;

/// ENTER, which starts a game from the title screen.
const ENTER: zx_spectrum::Key = zx_spectrum::Key::Matrix(6, 0);

/// Training mode's five switches (#148). Each is off by default, and with
/// all of them off the machine steers the game nowhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Training {
    /// A life lost is not taken from the lives left.
    pub lives: bool,
    /// The air never runs down, in play or under the light beam; the
    /// end-of-cavern bonus still counts it down.
    pub air: bool,
    /// A fall of any height lands safely.
    pub falls: bool,
    /// The guardians (Eugene, the Kong Beast and the Skylabs among them)
    /// pass through Willy.
    pub guardians: bool,
    /// Nasty tiles do not kill.
    pub nasties: bool,
}

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
    /// Training mode's switches in force.
    pub training: Training,
    /// A cavern to go to (0 to 19), by typing the game's own cheat; taken
    /// once the game has been sent there.
    pub go_to: Option<u8>,
    /// A key pressed for the read at the instruction before, to let go of
    /// now: its half-row and bit.
    pressed: Option<(usize, u8)>,
    /// A copy of the machine nobody sees or hears, as the jump preview runs
    /// (#156): the main loop's showing and sounding ([`unseen`]) are
    /// skipped. Never set on the machine the player plays.
    pub unseen: bool,
    /// A jump the preview makes on a copy (#155): the keys a player would
    /// press for it, set at the top of each pass of the main loop, so a
    /// frame holding more than one pass never reads them twice. Never set
    /// on the machine the player plays.
    pub jumping: Option<Jumping>,
}

/// A jump made a pass at a time: its way, turning first if Willy faces the
/// other way, and whether he has left the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Jumping {
    pub way: Way,
    pub started: bool,
}

impl Jumping {
    /// The keys for this pass, from where Willy is: none once he is off the
    /// ground; the direction alone while he faces the other way; then jump,
    /// with the direction.
    fn press(&mut self, z: &mut Zx) {
        for name in ["o", "p", "space"] {
            z.set_key(Key::by_name(name).expect("a key"), false);
        }
        self.started |= z.mem[usize::from(at::AIRBORNE)] != 0;
        if self.started {
            return;
        }
        let facing_left = z.mem[usize::from(at::FACING)] & 1 != 0;
        if let Some(d) = self.way.key() {
            z.set_key(Key::by_name(d).expect("a key"), true);
        }
        if self.way.faced(facing_left) {
            z.set_key(Key::by_name("space").expect("a key"), true);
        }
    }
}

/// The half-row and bit of a digit key.
fn digit(name: &str) -> (usize, u8) {
    match Key::by_name(name) {
        Some(Key::Matrix(row, bit)) => (usize::from(row), 1 << bit),
        _ => unreachable!("a digit is on the keyboard"),
    }
}

impl Play {
    /// Types the game's cheat, and then the teleport to [`Play::go_to`], at
    /// the reads that follow them: the digit the count at
    /// [`at::CHEAT_COUNT`] has reached, or, once it is in, 6 and the
    /// cavern's keys. Returns the key to press for this read.
    fn go_to_key(&mut self, z: &Zx, pc: u16) -> Option<(usize, u8)> {
        let cavern = self.go_to?;
        let count = usize::from(z.mem[usize::from(at::CHEAT_COUNT)]);
        if count < cheat::CODE.len() {
            let (row, bit) = digit(cheat::CODE[count]);
            let read = match row {
                cheat::LOW_ROW => cheat::READ_LOW,
                _ => cheat::READ_HIGH,
            };
            return (pc == read).then_some((row, bit));
        }
        match pc {
            cheat::TELEPORT_SIX => Some(digit("6")),
            cheat::TELEPORT_CAVERN => {
                // Taken: the game goes there from this read.
                self.go_to = None;
                Some((cheat::LOW_ROW, cavern & 0x1F))
            }
            _ => None,
        }
    }
}

/// Every address [`Play`]'s `at` does anything at (#187): the main loop's
/// top, what an unseen copy skips, the cheat's reads, the training
/// switches' steer points, the pause read and the title's ENTER reads.
/// What is pressed or kept for one read is let go at the instruction after
/// it, which the machine asks at too.
const ADDRESSES: [u16; 20] = [
    routine::MAIN_LOOP,
    unseen::PICTURE.0,
    unseen::SHOWN.0,
    unseen::TUNE.0,
    cheat::READ_LOW,
    cheat::READ_HIGH,
    cheat::TELEPORT_SIX,
    cheat::TELEPORT_CAVERN,
    steer::LOSE_LIFE,
    steer::AIR,
    steer::FALL_KILL,
    steer::GUARDIAN_DRAWS[0],
    steer::GUARDIAN_DRAWS[1],
    steer::GUARDIAN_DRAWS[2],
    steer::GUARDIAN_DRAWS[3],
    steer::NASTY_KILLS[0],
    steer::NASTY_KILLS[1],
    reads::PAUSE,
    reads::TITLE_ENTER,
    reads::TUNE_ENTER,
];

impl sidekick::Rules for Play {
    fn addresses(&self) -> Option<&[u16]> {
        Some(&ADDRESSES)
    }

    fn before_frame(&mut self, _z: &Zx) {
        self.pause_pressed = false;
    }

    fn at(&mut self, z: &mut Zx, pc: u16) {
        if pc == routine::MAIN_LOOP
            && let Some(jumping) = &mut self.jumping
        {
            jumping.press(z);
        }
        if self.unseen {
            for (from, to) in [unseen::PICTURE, unseen::SHOWN, unseen::TUNE] {
                if pc == from {
                    z.set_pc(to);
                    return;
                }
            }
        }
        if let Some((row, bits)) = self.pressed.take() {
            z.keys[row] |= bits;
        }
        if let Some((row, bits)) = self.go_to_key(z, pc) {
            z.keys[row] &= !bits;
            self.pressed = Some((row, bits));
        }
        let t = self.training;
        if t.lives && pc == steer::LOSE_LIFE {
            z.set_pc(steer::LOSE_LIFE + 1);
        }
        if t.air && pc == steer::AIR && steer::AIR_FROM.contains(&z.read16(z.sp())) {
            z.set_pc(steer::AIR_DRAW);
        }
        if t.falls && pc == steer::FALL_KILL {
            z.set_f(z.f() | CF);
        }
        if t.guardians && steer::GUARDIAN_DRAWS.contains(&pc) {
            z.set_bc(z.bc() & 0xFF00);
        }
        if t.nasties && steer::NASTY_KILLS.contains(&pc) {
            z.set_f(z.f() & !ZF);
        }
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
