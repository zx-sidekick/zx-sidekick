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
    /// Training mode's switches (#8): what the machine holds still for the
    /// player. Each puts back, or fills, after a frame, what the game took.
    pub training: Training,
    /// Whether a bar was filled after the last frame, so the panel still
    /// shows it as the game last drew it (#104, #116). The game draws a bar only
    /// where it changes it; the next time play reaches the top of its loop
    /// the machine has the game draw the panel again,
    /// [`starquake::routine::PANEL`].
    redraw: bool,
}

/// Training mode's five switches (#8, #116). Each is off by default, and
/// with all of them off the machine writes nothing into the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Training {
    /// Full energy: the energy bar is full, however much is taken, by time
    /// or by contact: filled when the switch goes on, and kept there (#116).
    pub energy: bool,
    /// Full bridging platforms: the bar is full however many Blob lays,
    /// filled and kept there as energy is (#104, #116).
    pub bridges: bool,
    /// Full laser: the bar is full however many shots are fired (#104,
    /// #116).
    pub laser: bool,
    /// The lives left never fall, and the panel's digit with them.
    pub lives: bool,
    /// Touching an enemy costs no energy: the push it gives the counter is
    /// taken back, so only time takes energy. And nothing kills outright
    /// (#68): the things that kill on touch, the impalers and the
    /// zap rays are steered past their kill at the instruction the game
    /// decides each with, [`starquake::decide`], with nothing written into
    /// the game.
    pub unharmed: bool,
}

/// What training mode read before a frame, to put back after it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    drain: u8,
    lives: u8,
    digit: u8,
}

impl Training {
    /// Whether any switch is on: with none, nothing is read or written.
    #[must_use]
    pub fn any(self) -> bool {
        self.energy || self.bridges || self.laser || self.lives || self.unharmed
    }

    /// What the switches in force need to know before a frame.
    #[must_use]
    pub fn read(self, z: &Zx) -> Held {
        if !self.any() {
            return Held::default();
        }
        let at = |a: u16| z.mem[usize::from(a)];
        Held {
            drain: at(starquake::at::DRAIN),
            lives: at(starquake::at::LIVES),
            digit: at(starquake::at::LIVES_DIGIT),
        }
    }

    /// Puts back what the switches in force hold still, after a frame.
    /// Whether it filled a bar, which the panel does not show until the game
    /// draws it again.
    pub fn hold(self, z: &mut Zx, before: Held) -> bool {
        if !self.any() {
            return false;
        }
        let at = |z: &Zx, a: u16| z.mem[usize::from(a)];
        let put = |z: &mut Zx, a: u16, v: u8| z.mem[usize::from(a)] = v;
        // The drain counter rises by one a frame, and contact pushes it on
        // further: no harm from enemies undoes the push and leaves the rise,
        // so energy still falls with time. Once every so many frames the
        // rise reaches the drop, where the game sets the counter to zero and
        // takes energy (#73): a push on that frame is taken back to zero.
        if self.unharmed {
            let now = at(z, starquake::at::DRAIN);
            let by_time = if before.drain.wrapping_add(1) >= starquake::DRAIN_DROP {
                0
            } else {
                before.drain.wrapping_add(1)
            };
            if now != by_time && now != before.drain {
                put(z, starquake::at::DRAIN, by_time);
            }
        }
        // Full is what the panel draws a bar up to (#104), so a bar its
        // switch finds run down is filled, one used is filled again, and one
        // a pack took higher is left there.
        let mut filled = false;
        for (on, a) in [
            (self.energy, starquake::at::ENERGY),
            (self.bridges, starquake::at::BRIDGES),
            (self.laser, starquake::at::LASER),
        ] {
            if on && at(z, a) < starquake::BAR_FULL {
                put(z, a, starquake::BAR_FULL);
                filled = true;
            }
        }
        if self.lives && at(z, starquake::at::LIVES) < before.lives {
            put(z, starquake::at::LIVES, before.lives);
            put(z, starquake::at::LIVES_DIGIT, before.digit);
        }
        filled
    }
}

/// Steers the game past an outright death when `pc` is where it decides one
/// (#68, [`starquake::decide`]): the register the instruction there reads is
/// given the value the harmless case has, and nothing is written into the
/// game. At the enemy-touch compare a page that kills becomes the harmless
/// page; at the marker compare the deadly kind becomes the spent kind, and
/// every other kind is left as it is, since the compare is reached for
/// every marker Blob touches, an item or a pad included; at the force-field
/// call the zero flag is set so the call is not made.
fn survive(z: &mut Zx, pc: u16) {
    use starquake::decide::{ENEMY_KILL, FIELD_KILL, PATCH_KILL};
    if pc == ENEMY_KILL.0 {
        if z.a() < starquake::HARMLESS_GRAPHICS {
            z.set_a(starquake::HARMLESS_GRAPHICS);
        }
    } else if pc == PATCH_KILL.0 {
        if z.a() == starquake::DANGER_MARKER {
            z.set_a(starquake::SPENT_MARKER);
        }
    } else if pc == FIELD_KILL.0 {
        let f = z.f();
        z.set_f(f | zx_spectrum::ZF);
    }
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
        Machine {
            zx,
            watch: Vec::new(),
            hold: None,
            holding: false,
            joystick: 0,
            start: false,
            pause_pressed: false,
            pause_was_down: false,
            training: Training::default(),
            redraw: false,
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
        let training = self.training;
        // What training mode holds still is read before the frame and put
        // back after it, so the game runs its own way in between (#8).
        let before = training.read(&self.zx);
        let start_game = start || joystick & JOY_FIRE != 0;
        let Machine {
            zx,
            watch,
            hold,
            holding,
            pause_pressed,
            pause_was_down,
            redraw,
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
            if training.unharmed {
                survive(z, pc);
            }
            // The play loop begins with a call, so nothing is carried in a
            // register at its top: the panel is drawn from there, returning
            // to it.
            if *redraw && pc == starquake::routine::MAIN_LOOP {
                *redraw = false;
                z.push(pc);
                z.set_pc(starquake::routine::PANEL.0);
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
        if training.hold(&mut self.zx, before) {
            self.redraw = true;
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine's memory with what training mode watches set to `v`.
    fn watched(v: u8) -> Zx {
        let mut z = Machine::from_ram(vec![0; 0xC000], 0, 0).zx;
        for a in [
            starquake::at::DRAIN,
            starquake::at::BRIDGES,
            starquake::at::LASER,
            starquake::at::LIVES,
            starquake::at::LIVES_DIGIT,
        ] {
            z.mem[usize::from(a)] = v;
        }
        z
    }

    fn at(z: &Zx, a: u16) -> u8 {
        z.mem[usize::from(a)]
    }

    #[test]
    fn with_every_switch_off_training_writes_nothing() {
        let mut z = watched(10);
        let training = Training::default();
        let before = training.read(&z);
        assert_eq!(before, Held::default(), "and reads nothing");
        z.mem[usize::from(starquake::at::LIVES)] = 0;
        training.hold(&mut z, before);
        assert_eq!(
            at(&z, starquake::at::LIVES),
            0,
            "the game's own doing stands"
        );
    }

    #[test]
    fn full_energy_fills_the_bar_and_keeps_it_full() {
        // The switch goes on with energy run down (#116): the next frame it
        // is full, and says it filled a bar, so the panel is drawn again.
        let mut z = watched(10);
        let training = Training {
            energy: true,
            ..Training::default()
        };
        z.mem[usize::from(starquake::at::ENERGY)] = 20;
        let before = training.read(&z);
        assert!(training.hold(&mut z, before), "a bar was filled");
        assert_eq!(at(&z, starquake::at::ENERGY), starquake::BAR_FULL);
        // Time and contact take four at a drop; it is filled again.
        let before = training.read(&z);
        z.mem[usize::from(starquake::at::ENERGY)] = starquake::BAR_FULL - 4;
        assert!(training.hold(&mut z, before));
        assert_eq!(at(&z, starquake::at::ENERGY), starquake::BAR_FULL);
        // Full already: nothing to fill, nothing to draw.
        let before = training.read(&z);
        assert!(!training.hold(&mut z, before), "nothing filled");
        // The drain counter is the game's: the switch does not touch it.
        let before = training.read(&z);
        z.mem[usize::from(starquake::at::DRAIN)] = 40;
        training.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::DRAIN), 40);
    }

    #[test]
    fn being_unharmed_leaves_only_what_time_took() {
        let mut z = watched(10);
        let training = Training {
            unharmed: true,
            ..Training::default()
        };
        let before = training.read(&z);
        z.mem[usize::from(starquake::at::DRAIN)] = 40;
        training.hold(&mut z, before);
        assert_eq!(
            at(&z, starquake::at::DRAIN),
            11,
            "as if only a frame had passed"
        );
        z.mem[usize::from(starquake::at::DRAIN)] = 11;
        training.hold(&mut z, before);
        assert_eq!(
            at(&z, starquake::at::DRAIN),
            11,
            "a plain frame is left alone"
        );
    }

    #[test]
    fn a_touch_on_the_wrap_frame_is_put_back_to_the_zero_the_game_left() {
        // The counter before the frame is one short of the drop (#73): the
        // game's own frame sets it to zero and takes energy, and a touch
        // pushes it on from there.
        let wrap = |training: Training, now: u8| {
            let mut z = watched(starquake::DRAIN_DROP - 1);
            let before = training.read(&z);
            z.mem[usize::from(starquake::at::DRAIN)] = now;
            training.hold(&mut z, before);
            at(&z, starquake::at::DRAIN)
        };
        let off = Training::default();
        let unharmed = Training {
            unharmed: true,
            ..off
        };
        assert_eq!(
            wrap(unharmed, 10),
            0,
            "the frame's own value is zero, not the drop"
        );
        assert_eq!(wrap(unharmed, 0), 0, "no touch: left alone");
        // And a frame short of the wrap rises by its one.
        let mut z = watched(starquake::DRAIN_DROP - 2);
        let before = unharmed.read(&z);
        z.mem[usize::from(starquake::at::DRAIN)] = starquake::DRAIN_DROP - 1;
        unharmed.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::DRAIN), starquake::DRAIN_DROP - 1);
    }

    #[test]
    fn full_energy_and_no_harm_together_keep_energy_full() {
        let mut z = watched(10);
        let training = Training {
            energy: true,
            unharmed: true,
            ..Training::default()
        };
        z.mem[usize::from(starquake::at::ENERGY)] = starquake::BAR_FULL;
        let before = training.read(&z);
        // Contact on top of the frame's own rise: the push is taken back,
        // and energy a drop took is filled.
        z.mem[usize::from(starquake::at::DRAIN)] = 40;
        z.mem[usize::from(starquake::at::ENERGY)] = starquake::BAR_FULL - 4;
        training.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::DRAIN), 11, "only time's rise");
        assert_eq!(at(&z, starquake::at::ENERGY), starquake::BAR_FULL);
    }

    #[test]
    fn each_full_switch_fills_only_its_own_bar() {
        // A switch goes on with its bar run down (#104, #116): the next frame
        // that bar is full, and the others are as the game left them.
        for (switch, bar) in [
            (
                Training {
                    energy: true,
                    ..Training::default()
                },
                starquake::at::ENERGY,
            ),
            (
                Training {
                    bridges: true,
                    ..Training::default()
                },
                starquake::at::BRIDGES,
            ),
            (
                Training {
                    laser: true,
                    ..Training::default()
                },
                starquake::at::LASER,
            ),
        ] {
            let mut z = watched(10);
            let bars = [
                starquake::at::ENERGY,
                starquake::at::BRIDGES,
                starquake::at::LASER,
            ];
            for a in bars {
                z.mem[usize::from(a)] = 0;
            }
            let before = switch.read(&z);
            switch.hold(&mut z, before);
            for a in bars {
                let want = if a == bar { starquake::BAR_FULL } else { 0 };
                assert_eq!(at(&z, a), want, "{switch:?} at {a:#06x}");
            }
        }
    }

    #[test]
    fn full_bridging_platforms_and_laser_never_fall() {
        let mut z = watched(starquake::BAR_FULL);
        let training = Training {
            bridges: true,
            laser: true,
            ..Training::default()
        };
        let before = training.read(&z);
        z.mem[usize::from(starquake::at::BRIDGES)] = 4;
        z.mem[usize::from(starquake::at::LASER)] = 0;
        training.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::BRIDGES), starquake::BAR_FULL);
        assert_eq!(at(&z, starquake::at::LASER), starquake::BAR_FULL);
        // A pack that takes a bar past full is kept, not put back.
        let before = training.read(&z);
        z.mem[usize::from(starquake::at::LASER)] = 0x90;
        training.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::LASER), 0x90, "picked up, not put back");
    }

    #[test]
    fn endless_lives_puts_back_the_count_and_its_digit() {
        let mut z = watched(3);
        let training = Training {
            lives: true,
            ..Training::default()
        };
        let before = training.read(&z);
        z.mem[usize::from(starquake::at::LIVES)] = 2;
        z.mem[usize::from(starquake::at::LIVES_DIGIT)] = b'2';
        training.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::LIVES), 3);
        assert_eq!(at(&z, starquake::at::LIVES_DIGIT), 3, "the digit with it");
        // A life won is not taken away.
        z.mem[usize::from(starquake::at::LIVES)] = 4;
        training.hold(&mut z, before);
        assert_eq!(at(&z, starquake::at::LIVES), 4);
    }

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

    /// Runs a frame of a small program with the pieces of `code` in place,
    /// from `pc`, with no harm from enemies off or on, and returns the byte
    /// the program leaves at `mark` (#68).
    fn steered(pc: u16, code: &[(u16, &[u8])], mark: u16, unharmed: bool) -> u8 {
        let mut m = Machine::blank(pc, 0xFF00);
        for &(at, bytes) in code {
            let at = usize::from(at);
            m.zx.mem[at..at + bytes.len()].copy_from_slice(bytes);
        }
        m.training = Training {
            unharmed,
            ..Training::default()
        };
        m.run_frame();
        m.zx.mem[usize::from(mark)]
    }

    #[test]
    fn a_thing_that_kills_on_touch_is_seen_as_harmless_with_no_harm_on() {
        let (at, cp) = starquake::decide::ENEMY_KILL;
        // LD A,B1 (a page that kills); CP B4; JR NC,+3; LD (mark),A; JR $.
        let code: &[(u16, &[u8])] = &[
            (at - 2, &[0x3E, 0xB1]),
            (at, &cp),
            (at + 2, &[0x30, 0x03, 0x32, 0x00, 0x90, 0x18, 0xFE]),
        ];
        assert_eq!(steered(at - 2, code, 0x9000, false), 0xB1, "the kill path");
        assert_eq!(steered(at - 2, code, 0x9000, true), 0, "steered past it");
    }

    #[test]
    fn a_deadly_patch_is_seen_as_spent_with_no_harm_on() {
        let (at, cp) = starquake::decide::PATCH_KILL;
        // LD A,06 (the deadly kind); CP 06; JR NZ,+3; LD (mark),A; JR $.
        let code: &[(u16, &[u8])] = &[
            (at - 2, &[0x3E, starquake::DANGER_MARKER]),
            (at, &cp),
            (at + 2, &[0x20, 0x03, 0x32, 0x00, 0x90, 0x18, 0xFE]),
        ];
        assert_eq!(steered(at - 2, code, 0x9000, false), 0x06, "the kill path");
        assert_eq!(steered(at - 2, code, 0x9000, true), 0, "steered past it");
    }

    #[test]
    fn every_other_marker_is_left_as_it_is_with_no_harm_on() {
        // The compare is reached for every marker Blob touches, an item
        // (kinds from 0x14) or a flying platform (0x0C) included, and those must
        // still be what they are (#68, found playing).
        let (at, cp) = starquake::decide::PATCH_KILL;
        for kind in [0x0C, 0x0D, 0x0E, 0x14, 0x20] {
            let code: &[(u16, &[u8])] = &[
                (at - 2, &[0x3E, kind]),
                (at, &cp),
                // JR NZ,+3 lands on LD (mark),A, so a kind that is not the
                // deadly one is written as it stands.
                (at + 2, &[0x20, 0x00, 0x32, 0x00, 0x90, 0x18, 0xFE]),
            ];
            assert_eq!(
                steered(at - 2, code, 0x9000, true),
                kind,
                "kind {kind:#04x}"
            );
        }
    }

    #[test]
    fn a_thing_that_only_drains_keeps_its_own_page_with_no_harm_on() {
        let (at, cp) = starquake::decide::ENEMY_KILL;
        // LD A,B6; CP B4; LD (mark),A; JR $: the page as the compare left it.
        let code: &[(u16, &[u8])] = &[
            (at - 2, &[0x3E, 0xB6]),
            (at, &cp),
            (at + 2, &[0x32, 0x00, 0x90, 0x18, 0xFE]),
        ];
        assert_eq!(steered(at - 2, code, 0x9000, true), 0xB6);
    }

    #[test]
    fn a_zapper_s_call_is_not_made_with_no_harm_on() {
        let (at, _) = starquake::decide::FIELD_KILL;
        // LD A,01; OR A (not zero); CALL NZ,9000; JR $. At 9000: LD
        // (mark),A; RET. The call's own target is the death routine's
        // address on the tape; here it is the mark's writer.
        let code: &[(u16, &[u8])] = &[
            (at - 3, &[0x3E, 0x01, 0xB7]),
            (at, &[0xC4, 0x00, 0x90, 0x18, 0xFE]),
            (0x9000, &[0x32, 0x10, 0x90, 0xC9]),
        ];
        assert_eq!(steered(at - 3, code, 0x9010, false), 0x01, "the kill path");
        assert_eq!(steered(at - 3, code, 0x9010, true), 0, "steered past it");
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
