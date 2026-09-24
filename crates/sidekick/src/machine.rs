//! A Spectrum 48K running a game from its tape, with no ROM.
//!
//! What is the same for every game lives here: loading the tape, running
//! frames, watching for addresses, holding keys, calling a routine on a
//! copy, and answering the few ROM routines a game calls. What a game does
//! differently (how its keys are read, what pauses it, what training mode
//! holds still) is its [`Rules`], which the machine asks at each frame and
//! each instruction.

use zx_core::snapshot::Snapshot;
use zx_spectrum::Zx;

use crate::rom;

/// What the player is pressing: the Spectrum's eight keyboard half-rows (a
/// 0 bit is a key down) and the joystick, as the bits below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub keys: [u8; 8],
    pub joystick: u8,
}

impl Default for Input {
    /// Nothing pressed.
    fn default() -> Self {
        Input {
            keys: [0xFF; 8],
            joystick: 0,
        }
    }
}

/// The joystick's five bits, in the Kempston port's order, which is the
/// order a gamepad and the keyboard joystick report in.
pub const JOY_RIGHT: u8 = 0x01;
pub const JOY_LEFT: u8 = 0x02;
pub const JOY_DOWN: u8 = 0x04;
pub const JOY_UP: u8 = 0x08;
pub const JOY_FIRE: u8 = 0x10;

/// IY as the ROM leaves it for a machine-code program started from BASIC:
/// the system variables' base, which the ROM's routines address from.
pub const BASIC_IY: u16 = 0x5C3A;
/// I as the ROM sets it at start-up.
pub const BASIC_I: u8 = 0x3F;

/// Keys the machine presses itself while the program is between two points:
/// from arriving at `from` until arriving at any of `until`. For holding a
/// game's own keys only while a particular loop of it is reading them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    pub from: u16,
    pub until: Vec<u16>,
    /// Keyboard half-row, and the bits of it to press.
    pub row: usize,
    pub bits: u8,
}

/// What a game does to the machine that no other game does: asked before
/// each frame, at each instruction before it runs, and after each frame.
/// Each step may read and steer the machine; with nothing to do, a game's
/// rules are `()`.
pub trait Rules {
    /// Before a frame is run.
    fn before_frame(&mut self, _z: &Zx) {}
    /// At `pc`, before the instruction there runs, after the machine's own
    /// watches and holds and before a ROM routine is answered.
    fn at(&mut self, _z: &mut Zx, _pc: u16) {}
    /// After a frame has run.
    fn after_frame(&mut self, _z: &mut Zx) {}
}

/// No rules: the game runs as the machine runs it.
impl Rules for () {}

/// The emulated machine, running a game by its [`Rules`].
#[derive(Clone)]
pub struct Machine<R> {
    pub zx: Zx,
    /// Addresses whose arrival [`Machine::run_frame`] reports.
    pub watch: Vec<u16>,
    /// A hold in force, if any; see [`Hold`].
    pub hold: Option<Hold>,
    /// The hold whose keys are down now, if any: a hold put in its place
    /// starts from its own `from`, with these keys let go.
    holding: Option<Hold>,
    /// What the game does differently.
    pub rules: R,
}

/// `JR $`: an instruction that jumps to itself. With no ROM, one sits at each
/// ROM routine ZX Sidekick answers, as a safety stop: the program counter is
/// answered before any instruction there runs, by [`answer`], so these run
/// only if an answer were ever missed, and then the processor stops there
/// instead of running into empty memory.
const JUMP_TO_ITSELF: [u8; 2] = [0x18, 0xFE];

/// Answers a ROM routine at the program counter, with no ROM present. The
/// interrupt is a step of its own ([`Zx::step`]), so it arrives here at
/// its vector with none of the handler run, like a call arrives at a
/// routine.
fn answer(z: &mut Zx) -> bool {
    if z.rom_loaded {
        return false;
    }
    rom::answer(z)
}

impl<R: Rules + Default> Machine<R> {
    /// A machine with empty memory and the processor at `pc`, for tests.
    #[must_use]
    pub fn blank(pc: u16, sp: u16) -> Machine<R> {
        Machine::from_ram(vec![0; 0xC000], pc, sp)
    }

    fn from_ram(ram: Vec<u8>, pc: u16, sp: u16) -> Machine<R> {
        let snap = Snapshot {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            a_: 0,
            f_: 0,
            b_: 0,
            c_: 0,
            d_: 0,
            e_: 0,
            h_: 0,
            l_: 0,
            ix: 0,
            iy: BASIC_IY,
            sp,
            pc,
            i: BASIC_I,
            r: 0,
            iff1: false,
            iff2: false,
            im: 1,
            border: 0,
            ram,
        };
        let mut zx = Zx::new(&snap, None);
        for at in [rom::MASK_INT, rom::PRINT_A_2, rom::HL_HL_X_DE] {
            zx.mem[usize::from(at)..usize::from(at) + 2].copy_from_slice(&JUMP_TO_ITSELF);
        }
        Machine {
            zx,
            watch: Vec::new(),
            hold: None,
            holding: None,
            rules: R::default(),
        }
    }

    /// The machine as the game starts: RAM from the tape's code blocks (the
    /// loading screen first, then the rest over it), and the processor where
    /// the loader leaves it.
    ///
    /// # Errors
    ///
    /// If the tape cannot be read.
    pub fn from_tape(tape: &[u8], entry_pc: u16, entry_sp: u16) -> Result<Machine<R>, String> {
        let loaded = zx_core::tape::load_tap(tape)?;
        Ok(Machine::from_ram(loaded.ram, entry_pc, entry_sp))
    }
}

impl<R: Rules + Clone> Machine<R> {
    /// The same machine with a real ROM in place, for checking the answers
    /// above against the routines they stand in for. Development only.
    #[must_use]
    pub fn with_rom(&self, rom: &[u8]) -> Machine<R> {
        let mut m = self.clone();
        let n = rom.len().min(0x4000);
        m.zx.mem[..n].copy_from_slice(&rom[..n]);
        m.zx.rom_loaded = true;
        m
    }
}

impl<R: Rules> Machine<R> {
    /// Calls the routine at `addr` and runs it until the program reaches
    /// `stop` or returns from the call, with the three ROM routines answered
    /// and no interrupts. Returns whether it got there within `max`
    /// instructions. For having the game do something on a copy of the
    /// machine, such as entering a room. The game's rules are not asked.
    ///
    /// Stopped at `stop`, the return address pushed for the call is still on
    /// the stack, the clock can be far past a frame, and interrupts are off:
    /// a caller that goes on to [`Machine::run_frame`] sets `zx.t` to 0 and
    /// turns interrupts back on first.
    pub fn call(&mut self, addr: u16, stop: u16, max: u64) -> bool {
        self.call_observing(addr, stop, max, |_| {})
    }

    /// [`Machine::call`], with `see` shown the machine before each
    /// instruction.
    pub fn call_observing(
        &mut self,
        addr: u16,
        stop: u16,
        max: u64,
        mut see: impl FnMut(&Zx),
    ) -> bool {
        let z = &mut self.zx;
        let sp = z.sp();
        z.push(0);
        z.set_pc(addr);
        z.set_interrupts(false);
        for _ in 0..max {
            if z.pc() == stop || (z.pc() == 0 && z.sp() == sp) {
                return true;
            }
            see(z);
            if !answer(z) {
                z.step();
            }
        }
        false
    }

    /// Runs one 50 Hz frame: the game's [`Rules`] are asked before it, at
    /// each instruction and after it, and the ROM routines the game calls
    /// are answered. Presses and lets go the keys of a [`Hold`] as the
    /// program reaches its ends, and returns the [watched](Machine::watch)
    /// addresses the program arrived at, in order.
    pub fn run_frame(&mut self) -> Vec<u16> {
        self.run_frame_observing(|_| {})
    }

    /// [`Machine::run_frame`], with `see` shown the machine before each
    /// instruction, for checks that follow what the game does.
    pub fn run_frame_observing(&mut self, mut see: impl FnMut(&Zx)) -> Vec<u16> {
        let Machine {
            zx,
            watch,
            hold,
            holding,
            rules,
        } = self;
        rules.before_frame(zx);
        let mut hits = Vec::new();
        // A hold put in place of the one whose keys are down lets them go.
        if let Some(held) = holding.take_if(|held| hold.as_ref() != Some(&*held)) {
            zx.keys[held.row] |= held.bits;
        }
        let mut pressing = holding.is_some();
        zx.run_frame(|z| {
            see(z);
            let pc = z.pc();
            if watch.contains(&pc) {
                hits.push(pc);
            }
            if let Some(hold) = hold.as_ref() {
                if pc == hold.from {
                    pressing = true;
                } else if hold.until.contains(&pc) {
                    pressing = false;
                    z.keys[hold.row] |= hold.bits;
                }
                if pressing {
                    z.keys[hold.row] &= !hold.bits;
                }
            }
            rules.at(z, pc);
            answer(z)
        });
        *holding = if pressing { hold.clone() } else { None };
        self.rules.after_frame(&mut self.zx);
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The machine with no rules, as any game's is underneath.
    type Machine = super::Machine<()>;

    /// A, S, D, F and G: a half-row to hold.
    const ASDFG: (usize, u8) = (1, 0x1F);

    /// A tape with one code block of `data` at `start`.
    fn tape(start: u16, data: &[u8]) -> Vec<u8> {
        let block = |flag: u8, payload: &[u8]| {
            let sum = payload.iter().fold(flag, |a, b| a ^ b);
            let mut out = ((payload.len() + 2) as u16).to_le_bytes().to_vec();
            out.push(flag);
            out.extend_from_slice(payload);
            out.push(sum);
            out
        };
        let mut header = vec![3];
        header.extend_from_slice(b"code      ");
        header.extend_from_slice(&(data.len() as u16).to_le_bytes());
        header.extend_from_slice(&start.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        let mut out = block(0x00, &header);
        out.extend(block(0xFF, data));
        out
    }

    #[test]
    fn nothing_is_pressed_to_begin_with() {
        let input = Input::default();
        assert_eq!(input.keys, [0xFF; 8]);
        assert_eq!(input.joystick, 0);
    }

    /// The keys `names` pressed on an otherwise idle keyboard.
    fn keys_named(names: &[&str]) -> [u8; 8] {
        let mut z = Machine::blank(0, 0).zx;
        for name in names {
            z.set_key(zx_spectrum::Key::by_name(name).unwrap(), true);
        }
        z.keys
    }

    /// A machine running a NOP at 0x8000 and then a `JR $` at 0x8001, for
    /// ever.
    fn nop_then_loop() -> Machine {
        let mut m = Machine::blank(0x8000, 0xC000);
        m.zx.mem[0x8000] = 0x00;
        m.zx.mem[0x8001..0x8003].copy_from_slice(&JUMP_TO_ITSELF);
        m
    }

    #[test]
    fn a_call_runs_a_routine_to_its_return_or_its_stop() {
        let mut m = Machine::blank(0x1234, 0xC000);
        m.zx.mem[0x8000] = 0x00; // NOP
        m.zx.mem[0x8001] = 0xC9; // RET
        assert!(m.call(0x8000, 0x9000, 10), "returned");
        assert_eq!(m.zx.sp(), 0xC000, "the stack as it was");
        let mut m = nop_then_loop();
        assert!(m.call(0x8000, 0x8001, 10), "reached the stop");
        let mut m = nop_then_loop();
        assert!(!m.call(0x8000, 0x9000, 10), "neither, within ten");
    }

    #[test]
    fn an_observer_sees_every_instruction() {
        let mut m = nop_then_loop();
        let mut seen = vec![];
        m.run_frame_observing(|z| seen.push(z.pc()));
        assert_eq!(seen[..2], [0x8000, 0x8001]);
        assert!(seen.len() > 1000, "the whole frame");
    }

    #[test]
    fn watched_addresses_are_reported_as_the_program_arrives() {
        let mut m = nop_then_loop();
        m.watch = vec![0x8000, 0x8001, 0x9000];
        let hits = m.run_frame();
        assert_eq!(hits[..2], [0x8000, 0x8001]);
        assert!(hits[2..].iter().all(|&h| h == 0x8001), "the loop, again");
        assert!(!hits.contains(&0x9000));
        m.watch.clear();
        assert!(m.run_frame().is_empty());
    }

    #[test]
    fn a_hold_presses_its_keys_from_one_address_until_another() {
        let (row, bits) = ASDFG;
        let hold = |from, until| Hold {
            from,
            until: vec![until],
            row,
            bits,
        };
        // Held from the NOP and never let go.
        let mut m = nop_then_loop();
        m.hold = Some(hold(0x8000, 0x9000));
        m.run_frame();
        assert_eq!(m.zx.keys, keys_named(&["a", "s", "d", "f", "g"]));
        // Let go on arriving at the loop.
        let mut m = nop_then_loop();
        m.hold = Some(hold(0x8000, 0x8001));
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8]);
        // Not held before the program gets to where it starts.
        let mut m = nop_then_loop();
        m.hold = Some(hold(0x9000, 0x9001));
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8]);
    }

    #[test]
    fn a_hold_put_in_place_of_another_lets_its_keys_go() {
        let (row, bits) = ASDFG;
        let mut m = nop_then_loop();
        m.hold = Some(Hold {
            from: 0x8000,
            until: vec![],
            row,
            bits,
        });
        m.run_frame();
        assert_ne!(m.zx.keys[row] & bits, bits, "held");
        // Another hold, from an address never reached: nothing held.
        m.hold = Some(Hold {
            from: 0x9000,
            until: vec![],
            row: 0,
            bits: 1,
        });
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8]);
    }

    #[test]
    fn a_hold_taken_away_is_forgotten() {
        let (row, bits) = ASDFG;
        let mut m = nop_then_loop();
        m.hold = Some(Hold {
            from: 0x8000,
            until: vec![],
            row,
            bits,
        });
        m.run_frame();
        m.hold = None;
        m.zx.release_all_keys();
        m.run_frame();
        // A new hold starts from its own address, not already held.
        m.hold = Some(Hold {
            from: 0x9000,
            until: vec![],
            row,
            bits,
        });
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8]);
    }

    #[test]
    fn a_machine_from_a_tape_starts_where_it_is_told_with_the_safety_stops_in_place() {
        let m = Machine::from_tape(&tape(0x8000, &[0xAB, 0xCD]), 0x8000, 0x7FF0).unwrap();
        let z = &m.zx;
        assert_eq!((z.pc(), z.sp()), (0x8000, 0x7FF0));
        assert_eq!(&z.mem[0x8000..0x8002], &[0xAB, 0xCD]);
        for at in [rom::MASK_INT, rom::PRINT_A_2, rom::HL_HL_X_DE] {
            assert_eq!(
                &z.mem[usize::from(at)..usize::from(at) + 2],
                &JUMP_TO_ITSELF
            );
        }
        assert!(!z.iff1());
        assert_eq!(z.iy(), BASIC_IY);
        assert!(!z.rom_loaded);
    }

    #[test]
    fn a_bad_tape_is_an_error() {
        assert!(Machine::from_tape(&[1, 2, 3], 0, 0).is_err());
    }

    #[test]
    fn an_interrupt_is_answered_as_the_rom_would_take_it() {
        // Interrupts on, and a NOP at 0x8000 that the interrupt comes before.
        let mut m = Machine::blank(0x8000, 0x7000);
        m.zx.set_interrupts(true);
        let r = m.zx.r();
        assert_eq!(m.zx.step(), zx_spectrum::Step::Interrupt);
        assert_eq!(m.zx.pc(), rom::MASK_INT, "at the vector, none of it run");
        assert!(answer(&mut m.zx));
        let z = &m.zx;
        assert_eq!(z.pc(), 0x8000, "back where it was interrupted");
        assert_eq!(z.sp(), 0x7000);
        assert_eq!(z.read16(rom::FRAMES), 1);
        assert!(z.iff1());
        // Taking the interrupt (13 T-states and a fetch), then MASK-INT
        // itself.
        assert_eq!(z.t, 13 + rom::MASK_INT_T);
        assert_eq!(z.r(), (r + 1 + 10) & 0x7F);
        assert_eq!(
            &z.mem[usize::from(rom::MASK_INT)..usize::from(rom::MASK_INT) + 2],
            &JUMP_TO_ITSELF,
            "the safety stop was never run"
        );
    }

    #[test]
    fn the_vector_reached_by_a_jump_is_answered_the_same_way() {
        // As `RST 38` reaches it: no interrupt was taken, so only MASK-INT
        // itself is charged.
        let mut m = Machine::blank(rom::MASK_INT, 0x7000);
        m.zx.push(0x8000);
        assert!(answer(&mut m.zx));
        assert_eq!((m.zx.pc(), m.zx.t), (0x8000, rom::MASK_INT_T));
    }

    #[test]
    fn frames_count_on_frames_answered() {
        // EI; HALT; JR back to the HALT.
        let mut m = Machine::blank(0x8000, 0x7000);
        m.zx.mem[0x8000..0x8004].copy_from_slice(&[0xFB, 0x76, 0x18, 0xFD]);
        for _ in 0..5 {
            m.run_frame();
        }
        assert_eq!(m.zx.frame, 5);
        assert_eq!(m.zx.read16(rom::FRAMES), 5);
        assert!(m.zx.halted());
    }

    #[test]
    fn with_a_rom_nothing_is_answered() {
        let rom = vec![0u8; 0x4000];
        let m = Machine::blank(rom::MASK_INT, 0x7000).with_rom(&rom);
        let mut z = m.zx;
        assert!(z.rom_loaded);
        assert_eq!(
            z.mem[usize::from(rom::MASK_INT)],
            0,
            "the ROM replaced the safety stop"
        );
        assert!(!answer(&mut z));
    }
}
