//! Starquake's rules for the machine (#141): what it does at the game's own
//! points that no other game has. The joystick is pressed as the game's
//! chosen control method listens, the pause key is kept from the game so the
//! window can freeze the emulation, training mode holds still what its
//! switches promise, and Start or fire starts a game on the title screen.

use sidekick::machine::{JOY_DOWN, JOY_FIRE, JOY_LEFT, JOY_RIGHT, JOY_UP};
use zx_spectrum::Zx;

use crate::facts::{self as starquake, CONTROL_METHOD, KEY_TABLES, PAUSE_KEY};

/// The key that starts a game on the title screen, `0`, which the intro
/// text takes as the any-key it waits for.
const START_GAME: zx_spectrum::Key = zx_spectrum::Key::Matrix(4, 0);

/// Starquake's [`sidekick::Rules`]: the machine's part of playing it.
#[derive(Clone, Debug, Default)]
pub struct Play {
    /// What the joystick is asking for this frame, as the machine's joystick
    /// bits; the game gets it however its chosen control method listens, see
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
    /// This frame's: what training mode read before it, the pause key kept
    /// from the game at its pause read (to give back after it), and whether
    /// Start or fire is starting a game.
    before: Held,
    kept: Option<zx_spectrum::Key>,
    start_game: bool,
}

/// Every address [`Play`]'s `at` does anything at (#187): the key reads,
/// the play loop's top, and training mode's steer points.
const ADDRESSES: [u16; 9] = [
    starquake::PLAY_INPUT,
    starquake::CONTROLS_INPUT,
    starquake::routine::MAIN_LOOP,
    starquake::MENU_INPUT,
    starquake::MENU_KEY,
    starquake::decide::ENEMY_KILL.0,
    starquake::decide::PATCH_KILL.0,
    starquake::decide::FIELD_KILL.0,
    starquake::decide::BAR_TAKE.0,
];

impl sidekick::Rules for Play {
    fn addresses(&self) -> Option<&[u16]> {
        Some(&ADDRESSES)
    }

    /// What training mode holds still is read before the frame and put back
    /// after it, so the game runs its own way in between (#8).
    fn before_frame(&mut self, z: &Zx) {
        self.before = self.training.read(z);
        self.start_game = self.start || self.joystick & JOY_FIRE != 0;
        self.pause_pressed = false;
        self.kept = None;
    }

    /// Presses the joystick's keys as the game's play-time key reader starts
    /// ([`starquake::PLAY_INPUT`]): the moment they reach the game in play,
    /// and no menu. Since the game never pauses itself here (below), it
    /// always gets there. Start or fire held is `0` where the title screen
    /// and the intro text wait for a key ([`starquake::MENU_INPUT`]) and as
    /// the title screen enters its key reader ([`starquake::MENU_KEY`], told
    /// from the define-keys screen's use of it by the return address);
    /// nowhere else.
    ///
    /// Keeps the game's pause key from its pause read, from
    /// [`starquake::PLAY_INPUT`] to [`starquake::CONTROLS_INPUT`], and says
    /// in [`Play::pause_pressed`] whether it was pressed.
    fn at(&mut self, z: &mut Zx, pc: u16) {
        if pc == starquake::PLAY_INPUT
            && let Some(key) = pause_key(z)
        {
            let down = is_down(z, key);
            self.pause_pressed |= down && !self.pause_was_down;
            self.pause_was_down = down;
            if down {
                z.set_key(key, false);
                self.kept = Some(key);
            }
        }
        if pc == starquake::CONTROLS_INPUT
            && let Some(key) = self.kept.take()
        {
            z.set_key(key, true);
        }
        if self.joystick != 0 && pc == starquake::PLAY_INPUT {
            press(z, self.joystick);
        }
        if self.training.unharmed {
            survive(z, pc);
        }
        keep_bars(z, pc, self.training);
        // The play loop begins with a call, so nothing is carried in a
        // register at its top: the panel is drawn from there, returning
        // to it.
        if self.redraw && pc == starquake::routine::MAIN_LOOP {
            self.redraw = false;
            z.push(pc);
            z.set_pc(starquake::routine::PANEL.0);
        }
        if self.start_game
            && (pc == starquake::MENU_INPUT
                || pc == starquake::MENU_KEY && z.read16(z.sp()) == starquake::MENU_KEY_FROM_TITLE)
        {
            z.set_key(START_GAME, true);
        }
    }

    fn after_frame(&mut self, z: &mut Zx) {
        if self.training.hold(z, self.before) {
            self.redraw = true;
        }
    }
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
        // switch finds run down is filled, and one a pack took higher is left
        // there. Once it is full nothing is taken from it again: the game is
        // kept from taking at the instruction it takes with (keep_bars), so
        // this only fills a bar the switch found low.
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

/// Keeps a bar whose Full switch is on from being taken from (#116), when
/// `pc` is where the game takes from a bar ([`starquake::decide::BAR_TAKE`]):
/// HL points at the bar, and the routine goes on from its `RET` instead, so
/// nothing is taken and nothing printed. Nothing is written into the game,
/// and the panel never shows the bar short: filling it back after the
/// frame, as [`Training::hold`] does, left it drawn short for a frame each
/// time; making C zero instead printed the full bar's end a notch short,
/// since only the panel's own loop draws the full end cell.
fn keep_bars(z: &mut Zx, pc: u16, training: Training) {
    if pc != starquake::decide::BAR_TAKE.0 {
        return;
    }
    let kept = match z.hl() {
        starquake::at::ENERGY => training.energy,
        starquake::at::BRIDGES => training.bridges,
        starquake::at::LASER => training.laser,
        _ => false,
    };
    if kept {
        z.set_pc(starquake::decide::BAR_TAKE_END.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Machine;

    /// `JR $`, as the machine puts at each ROM routine it answers.
    const JUMP_TO_ITSELF: [u8; 2] = [0x18, 0xFE];

    /// A machine's memory with what training mode watches set to `v`.
    fn watched(v: u8) -> Zx {
        let mut z = Machine::blank(0, 0).zx;
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
        assert_eq!(
            at(&z, starquake::at::LASER),
            0x90,
            "picked up, not put back"
        );
    }

    #[test]
    fn a_kept_bar_is_not_taken_from() {
        // At the instruction where the game takes from a bar, HL points at
        // it: a bar whose switch is on sends the routine to its RET, so
        // nothing is taken or printed (#116); every other bar is left to the
        // game.
        use starquake::decide::{BAR_TAKE, BAR_TAKE_END};
        let at_take = |hl: u16, training: Training| {
            let mut z = watched(10);
            z.set_pc(BAR_TAKE.0);
            z.set_hl(hl);
            keep_bars(&mut z, BAR_TAKE.0, training);
            z.pc()
        };
        let laser = Training {
            laser: true,
            ..Training::default()
        };
        assert_eq!(at_take(starquake::at::LASER, laser), BAR_TAKE_END.0);
        assert_eq!(
            at_take(starquake::at::BRIDGES, laser),
            BAR_TAKE.0,
            "not its bar"
        );
        assert_eq!(at_take(starquake::at::ENERGY, laser), BAR_TAKE.0);
        assert_eq!(
            at_take(starquake::at::LASER, Training::default()),
            BAR_TAKE.0,
            "switch off"
        );
        let energy = Training {
            energy: true,
            ..Training::default()
        };
        assert_eq!(at_take(starquake::at::ENERGY, energy), BAR_TAKE_END.0);
        // Anywhere else nothing is steered.
        let mut z = watched(10);
        z.set_pc(0x8000);
        z.set_hl(starquake::at::LASER);
        keep_bars(&mut z, 0x8000, laser);
        assert_eq!(z.pc(), 0x8000);
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

    #[test]
    fn the_joystick_is_pressed_as_the_play_time_reader_starts() {
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        m.rules.joystick = JOY_LEFT | JOY_FIRE;
        m.run_frame();
        // O and M, from the keyboard method's table.
        assert_eq!(m.zx.keys, keys_named(&["o", "m"]));
        assert_eq!(m.zx.kempston, 0);
    }

    #[test]
    fn a_frame_that_never_reaches_the_reader_presses_nothing() {
        let mut m = at_the_reader(starquake::PLAY_INPUT + 1);
        m.rules.joystick = 0x1F;
        m.rules.start = true;
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
            m.rules.joystick = joystick;
            m.rules.start = start;
            m.run_frame();
            // The directions mean nothing to the title screen.
            assert_eq!((m.zx.keys, m.zx.kempston), (keys_named(&["0"]), 0));
        }
        let mut m = at_the_reader(starquake::MENU_INPUT);
        m.rules.joystick = JOY_LEFT | JOY_UP;
        m.run_frame();
        assert_eq!((m.zx.keys, m.zx.kempston), ([0xFF; 8], 0));
    }

    #[test]
    fn the_key_reader_gets_the_zero_only_when_the_title_screen_calls_it() {
        // Entered with the title screen's return address on the stack.
        let mut m = at_the_reader(starquake::MENU_KEY);
        m.zx.push(starquake::MENU_KEY_FROM_TITLE);
        m.rules.start = true;
        m.run_frame();
        assert_eq!(m.zx.keys, keys_named(&["0"]));
        // Entered from anywhere else, the define-keys screen included.
        let mut m = at_the_reader(starquake::MENU_KEY);
        m.zx.push(0x625A);
        m.rules.start = true;
        m.rules.joystick = JOY_FIRE;
        m.run_frame();
        assert_eq!(m.zx.keys, [0xFF; 8]);
    }

    #[test]
    fn in_play_start_presses_nothing_and_fire_is_fire_not_zero() {
        let mut m = at_the_reader(starquake::PLAY_INPUT);
        m.rules.start = true;
        m.rules.joystick = JOY_FIRE;
        m.run_frame();
        assert_eq!(m.zx.keys, keys_named(&["m"]));
        let mut m = at_the_reader(starquake::CONTROLS_INPUT);
        m.rules.start = true;
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
        m.rules.training = Training {
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
            (m.rules.pause_pressed, m.zx.keys)
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
                assert_eq!(m.rules.pause_pressed, pauses, "method {method} {key:?}");
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
        assert!(m.rules.pause_pressed);
        assert_eq!(m.zx.keys, keys_named(&["space"]));
    }

    #[test]
    fn a_new_machine_has_the_joystick_at_rest() {
        let m = Machine::blank(0, 0);
        assert_eq!(
            (m.rules.joystick, m.rules.start, m.rules.pause_pressed),
            (0, false, false)
        );
    }
}
