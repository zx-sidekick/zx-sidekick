//! A Spectrum 48K running a game from its tape, with no ROM.

use zx_core::snapshot::Snapshot;
use zx_spectrum::Zx;

use crate::rom;
use crate::starquake::{self, CONTROL_METHOD, KEY_TABLES, PAUSE_KEY};

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

/// The key that starts a game on the title screen, `0`, which the intro
/// text takes as the any-key it waits for.
const START_GAME: zx_spectrum::Key = zx_spectrum::Key::Matrix(4, 0);

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

/// The emulated machine.
#[derive(Clone)]
pub struct Machine {
    pub zx: Zx,
    /// Addresses whose arrival [`Machine::run_frame`] reports.
    pub watch: Vec<u16>,
    /// A hold in force, if any; see [`Hold`].
    pub hold: Option<Hold>,
    /// Whether the hold's keys are down now.
    holding: bool,
    /// What the joystick is asking for this frame, as the bits above; the
    /// game gets it however its chosen control method listens, see
    /// [`press`].
    pub joystick: u8,
    /// Whether the joystick's Start is held. On the title screen Start or
    /// fire is `0`, which starts a game, and on the intro text the key it
    /// waits for, held for as long as the button is, as a key would be. In
    /// play it presses nothing: there the window freezes the emulation.
    pub start: bool,
    /// Whether the player pressed the game's pause key during the last
    /// frame: it was down as the game read it ([`starquake::PLAY_INPUT`])
    /// and had not been at the read before. The key is kept from the game,
    /// so the game never pauses itself; the window freezes the emulation
    /// instead. See [`pause_key`].
    pub pause_pressed: bool,
    /// Whether the pause key was down at the last pause read.
    pause_was_down: bool,
}

/// The key the game pauses with in play: Space in the Kempston method,
/// whatever was defined, and otherwise the one it keeps at [`PAUSE_KEY`],
/// Space as the tape ships it or what the define-keys screen set.
#[must_use]
pub fn pause_key(z: &Zx) -> Option<zx_spectrum::Key> {
    if z.mem[usize::from(CONTROL_METHOD)] == 1 {
        starquake::key(SPACE)
    } else {
        starquake::key(z.mem[usize::from(PAUSE_KEY)])
    }
}

/// Space, as the game's tables name it: the Kempston method's pause key.
const SPACE: u8 = b'*';

/// Whether `key` is down on `z`'s keyboard.
fn is_down(z: &Zx, key: zx_spectrum::Key) -> bool {
    match key {
        zx_spectrum::Key::Matrix(row, bit) => z.keys[usize::from(row)] & (1 << bit) == 0,
        _ => false,
    }
}

/// Presses `joystick` the way the game's chosen control method listens for
/// it: as the Kempston port's bits in method 1, and in methods 2 to 5 as the
/// five keys the method's table names, so a joystick moves Blob whichever
/// option was chosen on the title screen. Nothing is released: the caller
/// sets the keys afresh each frame.
pub fn press(z: &mut Zx, joystick: u8) {
    let method = z.mem[usize::from(CONTROL_METHOD)];
    if method == 1 {
        z.kempston |= joystick;
    } else if (2..=5).contains(&method) {
        let table = usize::from(KEY_TABLES) + 5 * usize::from(method - 2);
        let order = [JOY_LEFT, JOY_RIGHT, JOY_DOWN, JOY_UP, JOY_FIRE];
        for (i, bit) in order.into_iter().enumerate() {
            if joystick & bit != 0
                && let Some(key) = starquake::key(z.mem[table + i])
            {
                z.set_key(key, true);
            }
        }
    }
}

/// `JR $`: an instruction that jumps to itself. With no ROM, one sits at each
/// ROM routine ZX Sidekick answers, so the processor stops there instead of
/// running into empty memory; see [`answer`].
const JUMP_TO_ITSELF: [u8; 2] = [0x18, 0xFE];

/// The time `JR $` takes, and its one opcode fetch.
const JUMP_T: u32 = 12;

/// Answers a ROM routine at the program counter, with no ROM present.
///
/// An interrupt vectors to 0x0038 and runs the instruction there in the same
/// step, so it arrives having run the `JR $` placed there: that jump's time
/// and fetch are given back first, as the ROM's routine would have begun
/// straight away.
fn answer(z: &mut Zx) -> bool {
    if z.rom_loaded {
        return false;
    }
    if z.fetched_from() == Some(rom::MASK_INT) && z.pc() == rom::MASK_INT {
        z.t -= JUMP_T;
        let r = z.r();
        z.set_r((r & 0x80) | (r.wrapping_sub(1) & 0x7F));
    }
    rom::answer(z)
}

impl Machine {
    /// A machine with empty memory and the processor at `pc`, for tests.
    #[must_use]
    pub fn blank(pc: u16, sp: u16) -> Machine {
        Machine::from_ram(vec![0; 0xC000], pc, sp)
    }

    fn from_ram(ram: Vec<u8>, pc: u16, sp: u16) -> Machine {
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
            iy: crate::starquake::ENTRY_IY,
            sp,
            pc,
            i: crate::starquake::ENTRY_I,
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
        zx.traps = vec![rom::MASK_INT];
        Machine {
            zx,
            watch: Vec::new(),
            hold: None,
            holding: false,
            joystick: 0,
            start: false,
            pause_pressed: false,
            pause_was_down: false,
        }
    }

    /// The machine as the game starts: RAM from the tape's code blocks (the
    /// loading screen first, then the rest over it), and the processor where
    /// the loader leaves it.
    ///
    /// # Errors
    ///
    /// If the tape cannot be read.
    pub fn from_tape(tape: &[u8], entry_pc: u16, entry_sp: u16) -> Result<Machine, String> {
        let loaded = zx_core::tape::load_tap(tape)?;
        Ok(Machine::from_ram(loaded.ram, entry_pc, entry_sp))
    }

    /// The same machine with a real ROM in place, for checking the answers
    /// above against the routines they stand in for. Development only.
    #[must_use]
    pub fn with_rom(&self, rom: &[u8]) -> Machine {
        let mut m = self.clone();
        let n = rom.len().min(0x4000);
        m.zx.mem[..n].copy_from_slice(&rom[..n]);
        m.zx.rom_loaded = true;
        m.zx.traps.clear();
        m
    }

    /// Calls the routine at `addr` and runs it until the program reaches
    /// `stop` or returns from the call, with the three ROM routines answered
    /// and no interrupts. Returns whether it got there within `max`
    /// instructions. For having the game do something on a copy of the
    /// machine, such as entering a room.
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

    /// Runs one 50 Hz frame, answering the ROM routines the game calls, and
    /// pressing the joystick's keys as the game's play-time key reader
    /// starts ([`starquake::PLAY_INPUT`]): the moment they reach the game in
    /// play, and no menu. Since the game never pauses itself here (below),
    /// it always gets there. Start
    /// or fire held is `0` where the title screen and the intro text wait
    /// for a key ([`starquake::MENU_INPUT`]) and as the title screen enters
    /// its key reader ([`starquake::MENU_KEY`], told from the define-keys
    /// screen's use of it by the return address); nowhere else.
    ///
    /// Keeps the game's pause key from its pause read, from
    /// [`starquake::PLAY_INPUT`] to [`starquake::CONTROLS_INPUT`], and says
    /// in [`Machine::pause_pressed`] whether it was pressed.
    ///
    /// Presses and lets go the keys of a [`Hold`] as the program reaches
    /// its ends, and returns the [watched](Machine::watch) addresses the
    /// program arrived at, in order.
    pub fn run_frame(&mut self) -> Vec<u16> {
        self.run_frame_observing(|_| {})
    }

    /// [`Machine::run_frame`], with `see` shown the machine before each
    /// instruction, for checks that follow what the game does.
    pub fn run_frame_observing(&mut self, mut see: impl FnMut(&Zx)) -> Vec<u16> {
        let (joystick, start) = (self.joystick, self.start);
        let start_game = start || joystick & JOY_FIRE != 0;
        let Machine {
            zx,
            watch,
            hold,
            holding,
            pause_pressed,
            pause_was_down,
            ..
        } = self;
        *pause_pressed = false;
        // The pause key kept from the game at this frame's pause read, to
        // give back after it.
        let mut kept = None;
        let mut hits = Vec::new();
        zx.run_frame(|z| {
            see(z);
            let pc = z.pc();
            if watch.contains(&pc) {
                hits.push(pc);
            }
            if let Some(hold) = hold.as_ref() {
                if pc == hold.from {
                    *holding = true;
                } else if hold.until.contains(&pc) {
                    *holding = false;
                    z.keys[hold.row] |= hold.bits;
                }
                if *holding {
                    z.keys[hold.row] &= !hold.bits;
                }
            }
            if pc == starquake::PLAY_INPUT
                && let Some(key) = pause_key(z)
            {
                let down = is_down(z, key);
                *pause_pressed |= down && !*pause_was_down;
                *pause_was_down = down;
                if down {
                    z.set_key(key, false);
                    kept = Some(key);
                }
            }
            if pc == starquake::CONTROLS_INPUT
                && let Some(key) = kept.take()
            {
                z.set_key(key, true);
            }
            if joystick != 0 && pc == starquake::PLAY_INPUT {
                press(z, joystick);
            }
            if start_game
                && (pc == starquake::MENU_INPUT
                    || pc == starquake::MENU_KEY
                        && z.read16(z.sp()) == starquake::MENU_KEY_FROM_TITLE)
            {
                z.set_key(START_GAME, true);
            }
            answer(z)
        });
        if hold.is_none() {
            *holding = false;
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A blank machine with the game's control variables as the tape ships
    /// them: method `method`, the four tables, Space as the pause key.
    fn with_tables(method: u8) -> Machine {
        let mut m = Machine::blank(0x8000, 0xC000);
        m.zx.mem[usize::from(CONTROL_METHOD)] = method;
        let tables = b"58670"
            .iter()
            .chain(b"12345")
            .chain(b"OPAQM")
            .chain(b"QWERT");
        for (i, &b) in tables.enumerate() {
            m.zx.mem[usize::from(KEY_TABLES) + i] = b;
        }
        m.zx.mem[usize::from(PAUSE_KEY)] = b'*';
        m
    }

    /// The keys `names` pressed on an otherwise idle keyboard.
    fn keys_named(names: &[&str]) -> [u8; 8] {
        let mut z = Machine::blank(0, 0).zx;
        for name in names {
            z.set_key(zx_spectrum::Key::by_name(name).unwrap(), true);
        }
        z.keys
    }

    #[test]
    fn in_the_kempston_method_the_joystick_is_the_kempston_port() {
        for bits in 0..32u8 {
            let mut m = with_tables(1);
            press(&mut m.zx, bits);
            assert_eq!(m.zx.kempston, bits);
            assert_eq!(m.zx.keys, [0xFF; 8], "bits {bits:#04x}");
        }
    }

    #[test]
    fn in_a_keyboard_method_the_joystick_presses_the_keys_the_table_names() {
        let tables = [
            ["5", "8", "6", "7", "0"],
            ["1", "2", "3", "4", "5"],
            ["o", "p", "a", "q", "m"],
            ["q", "w", "e", "r", "t"],
        ];
        for (method, table) in (2..=5).zip(tables) {
            for bits in 0..32u8 {
                let mut m = with_tables(method);
                press(&mut m.zx, bits);
                let order = [JOY_LEFT, JOY_RIGHT, JOY_DOWN, JOY_UP, JOY_FIRE];
                let expected: Vec<&str> = order
                    .iter()
                    .zip(table)
                    .filter(|(bit, _)| bits & **bit != 0)
                    .map(|(_, name)| name)
                    .collect();
                assert_eq!(
                    m.zx.keys,
                    keys_named(&expected),
                    "method {method} bits {bits:#04x}"
                );
                assert_eq!(m.zx.kempston, 0, "method {method} bits {bits:#04x}");
            }
        }
    }

    #[test]
    fn a_method_the_game_has_not_got_and_a_byte_naming_no_key_press_nothing() {
        for method in [0, 6, 0xFF] {
            let mut m = with_tables(method);
            press(&mut m.zx, 0x1F);
            assert_eq!(
                (m.zx.keys, m.zx.kempston),
                ([0xFF; 8], 0),
                "method {method}"
            );
        }
        let mut m = with_tables(4);
        m.zx.mem[usize::from(KEY_TABLES) + 10] = 0;
        press(&mut m.zx, JOY_LEFT | JOY_FIRE);
        assert_eq!((m.zx.keys, m.zx.kempston), (keys_named(&["m"]), 0));
    }

    /// A machine with the tables in place whose program starts at `pc`: a
    /// `NOP` at the play-time reader's address and at its controls read,
    /// each followed by a jump to itself.
    fn at_the_reader(pc: u16) -> Machine {
        let mut m = with_tables(4);
        for at in [
            starquake::PLAY_INPUT,
            starquake::CONTROLS_INPUT,
            starquake::MENU_INPUT,
            starquake::MENU_KEY,
        ] {
            let at = usize::from(at);
            m.zx.mem[at] = 0x00;
            m.zx.mem[at + 1..at + 3].copy_from_slice(&JUMP_TO_ITSELF);
        }
        m.zx.set_pc(pc);
        m
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
        let (row, bits) = starquake::END_GAME_KEYS;
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
    fn a_hold_taken_away_is_forgotten() {
        let (row, bits) = starquake::END_GAME_KEYS;
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
    fn the_joystick_is_pressed_as_the_play_time_reader_starts() {
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        m.joystick = JOY_LEFT | JOY_FIRE;
        m.run_frame();
        // O and M, from the keyboard method's table.
        assert_eq!(m.zx.keys, keys_named(&["o", "m"]));
        assert_eq!(m.zx.kempston, 0);
    }

    #[test]
    fn a_frame_that_never_reaches_the_reader_presses_nothing() {
        let mut m = at_the_reader(starquake::PLAY_INPUT + 1);
        m.joystick = 0x1F;
        m.start = true;
        m.run_frame();
        assert_eq!((m.zx.keys, m.zx.kempston), ([0xFF; 8], 0));
    }

    #[test]
    fn a_joystick_at_rest_leaves_the_keys_alone() {
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        m.zx.set_key(zx_spectrum::Key::by_name("q").unwrap(), true);
        m.run_frame();
        assert_eq!(m.zx.keys, keys_named(&["q"]));
    }

    #[test]
    fn start_or_fire_is_zero_as_the_title_screen_reads_the_keys() {
        assert_eq!(Some(START_GAME), zx_spectrum::Key::by_name("0"));
        for (joystick, start) in [(0, true), (JOY_FIRE, false), (0x1F, true)] {
            let mut m = at_the_reader(starquake::MENU_INPUT);
            m.joystick = joystick;
            m.start = start;
            m.run_frame();
            // The directions mean nothing to the title screen.
            assert_eq!((m.zx.keys, m.zx.kempston), (keys_named(&["0"]), 0));
        }
        let mut m = at_the_reader(starquake::MENU_INPUT);
        m.joystick = JOY_LEFT | JOY_UP;
        m.run_frame();
        assert_eq!((m.zx.keys, m.zx.kempston), ([0xFF; 8], 0));
    }

    #[test]
    fn the_key_reader_gets_the_zero_only_when_the_title_screen_calls_it() {
        // Entered with the title screen's return address on the stack.
        let mut m = at_the_reader(starquake::MENU_KEY);
        m.zx.push(starquake::MENU_KEY_FROM_TITLE);
        m.start = true;
        m.run_frame();
        assert_eq!(m.zx.keys, keys_named(&["0"]));
        // Entered from anywhere else, the define-keys screen included.
        let mut m = at_the_reader(starquake::MENU_KEY);
        m.zx.push(0x625A);
        m.start = true;
        m.joystick = JOY_FIRE;
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8]);
    }

    #[test]
    fn in_play_start_presses_nothing_and_fire_is_fire_not_zero() {
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        m.start = true;
        m.joystick = JOY_FIRE;
        m.run_frame();
        assert_eq!(m.zx.keys, keys_named(&["m"]));
        let mut m = at_the_reader(starquake::CONTROLS_INPUT);
        m.start = true;
        m.run_frame();
        assert_eq!((m.zx.keys, m.zx.kempston), ([0xFF; 8], 0));
    }

    #[test]
    fn the_pause_key_is_kept_from_the_game_and_a_fresh_press_is_reported() {
        let space = zx_spectrum::Key::by_name("space").unwrap();
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        let frame = |m: &mut Machine, down: bool| {
            m.zx.set_pc(starquake::PLAY_INPUT);
            m.zx.release_all_keys();
            m.zx.set_key(space, down);
            m.run_frame();
            (m.pause_pressed, m.zx.keys)
        };
        assert_eq!(frame(&mut m, true), (true, [0xFF; 8]), "pressed, and kept");
        assert_eq!(frame(&mut m, true), (false, [0xFF; 8]), "held: once");
        assert_eq!(frame(&mut m, false), (false, [0xFF; 8]));
        assert!(frame(&mut m, true).0, "pressed again");
    }

    #[test]
    fn the_pause_key_is_the_one_the_game_pauses_with_in_its_method() {
        let space = zx_spectrum::Key::by_name("space").unwrap();
        let n = zx_spectrum::Key::by_name("n").unwrap();
        for method in 1..=5 {
            // The define-keys screen set N: the Kempston method pauses with
            // Space regardless, the others with N.
            let (pause, other) = if method == 1 { (space, n) } else { (n, space) };
            for (key, pauses) in [(pause, true), (other, false)] {
                let mut m = at_the_reader(starquake::PLAY_INPUT);
                m.zx.mem[usize::from(CONTROL_METHOD)] = method;
                m.zx.mem[usize::from(PAUSE_KEY)] = b'N';
                assert_eq!(pause_key(&m.zx), Some(pause), "method {method}");
                m.zx.set_key(key, true);
                m.run_frame();
                assert_eq!(m.pause_pressed, pauses, "method {method} {key:?}");
                assert_eq!(is_down(&m.zx, key), !pauses, "method {method} {key:?}");
            }
        }
    }

    #[test]
    fn the_pause_key_is_given_back_after_the_pause_read() {
        // The reader's pause read, then on to the controls read.
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        let at = usize::from(starquake::PLAY_INPUT);
        m.zx.mem[at + 1..at + 3].copy_from_slice(&[
            0x18,
            (starquake::CONTROLS_INPUT - starquake::PLAY_INPUT - 3) as u8,
        ]);
        m.zx.set_key(zx_spectrum::Key::by_name("space").unwrap(), true);
        m.run_frame();
        assert!(m.pause_pressed);
        assert_eq!(m.zx.keys, keys_named(&["space"]));
    }

    #[test]
    fn a_new_machine_has_the_joystick_at_rest() {
        let m = Machine::blank(0, 0);
        assert_eq!((m.joystick, m.start, m.pause_pressed), (0, false, false));
    }

    #[test]
    fn a_machine_from_a_tape_starts_where_it_is_told_with_the_traps_in_place() {
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
        assert_eq!(z.traps, [rom::MASK_INT]);
        assert!(!z.iff1());
        assert_eq!(z.iy(), crate::starquake::ENTRY_IY);
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
        m.zx.step();
        assert_eq!(m.zx.pc(), rom::MASK_INT, "the jump at 0x0038 ran");
        assert!(answer(&mut m.zx));
        let z = &m.zx;
        assert_eq!(z.pc(), 0x8000, "back where it was interrupted");
        assert_eq!(z.sp(), 0x7000);
        assert_eq!(z.read16(rom::FRAMES), 1);
        assert!(z.iff1());
        // Taking the interrupt (13), then MASK-INT itself, with the jump's
        // time and fetch given back.
        assert_eq!(z.t, 13 + rom::MASK_INT_T);
        assert_eq!(z.r(), (r + 1 + 10) & 0x7F);
    }

    #[test]
    fn a_trap_reached_without_an_interrupt_is_answered_without_a_refund() {
        let mut m = Machine::blank(rom::MASK_INT, 0x7000);
        m.zx.push(0x8000);
        assert!(answer(&mut m.zx));
        assert_eq!(m.zx.t, rom::MASK_INT_T);
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
        assert!(z.traps.is_empty());
        assert_eq!(
            z.mem[usize::from(rom::MASK_INT)],
            0,
            "the ROM replaced the trap"
        );
        assert!(!answer(&mut z));
    }
}
