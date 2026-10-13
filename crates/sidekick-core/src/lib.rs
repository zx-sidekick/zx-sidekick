//! ZX Sidekick's emulated machine, and everything the program reads from it and writes to it
//! between frames.
//!
//! The machine is `rustzx-core` from the zx-sidekick fork of `RustZX`, built on that fork's
//! `rustzx-z80`. For now this crate wraps it headless: a 48K or 128K Spectrum with the bundled
//! ROM, run a frame at a time, stopping at a game's sync address on the way. A [`Session`] plays
//! a game on it, reading the named values and deciding the state its definition describes, and
//! writing to it by name. The cheats and saves described in `ZX-SIDEKICK.md` build on these.

mod error;
mod graphics;
mod host;
mod session;
mod value;
mod watch;

pub use error::Error;
pub use graphics::{Bitmap, Font};
pub use host::{HeadlessHost, Picture};
pub use rustzx_core::zx::joy::kempston::KempstonKey as Kempston;
pub use rustzx_core::zx::keys::{CompoundKey, ZXKey as Key};
pub use rustzx_core::zx::machine::ZXMachine as Model;
pub use rustzx_z80::Regs;
pub use session::{Milestone, Progress, Reading, Record, ScoreEntry, Session, SwitchError};
pub use sidekick_script::{Fact, Script};
pub use value::{Read, WriteError};

use host::Bytes;
use rustzx_core::{
    host::{BufferCursor, FrameBuffer, FrameBufferSource, Snapshot, SnapshotRecorder, Tape},
    zx::video::BorderSize,
    EmulationMode, EmulationStopReason, Emulator, RustzxSettings,
};
use std::time::Duration;
use watch::Watch;

/// How long one call to `run_frame` may take on the host before the core gives up on it.
/// Only reached if the host is far too slow; a frame normally takes well under a millisecond.
const FRAME_HOST_LIMIT: Duration = Duration::from_millis(100);

const WATCH_SET: &str = "the watch is set when the machine is built";

/// The picture's border: 32 pixels on every side of the 256×192 screen, 320×256 in all.
const BORDER: BorderSize = BorderSize::Tall;

/// Sample rate the beeper and AY are mixed at.
pub const SAMPLE_RATE: usize = 44_100;
/// The 48K ROM's `PAUSE` loop, from its `HALT` to the jump back: where a BASIC loader's
/// `PAUSE` waits. Its count is in BC, 0 for `PAUSE 0`, which waits for a key alone.
const ROM_PAUSE_LOOP: std::ops::RangeInclusive<u16> = 0x1F3D..=0x1F4E;

/// Why [`Machine::run`] returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The processor reached the sync address, partway through a frame: the game is between two
    /// of its own frames.
    Sync,
    /// The frame ended: time to present the picture and sound.
    FrameEnd,
    /// The tape's last block has just loaded, before the loader runs another instruction: the
    /// screen is as the tape left it, and the game's code hasn't run. Only when asked for, with
    /// [`Machine::stop_when_loaded`], and only for tapes loaded through the ROM's loader.
    Loaded,
}

/// A Spectrum with the bundled ROM and nothing loaded, run a frame at a time.
///
/// A clone is the whole machine, and runs apart from the original.
#[derive(Clone)]
pub struct Machine {
    emulator: Emulator<HeadlessHost>,
    model: Model,
    /// The last `run` returned at a frame's end, so the next begins a new frame.
    frame_ended: bool,
    /// The watch's addresses passed, as of the last `run` (the core lends its debug interface
    /// mutably only).
    passed: Vec<u16>,
    /// Whether the tape has loaded, and whether it's loading, as of the last `run`.
    tape_loaded: bool,
    tape_loading: bool,
    /// The border and screen together, as of the last frame's end.
    picture: Picture,
    /// The last frame's sound, left and right.
    sound: Vec<(f32, f32)>,
}

impl Machine {
    /// A freshly switched-on machine of the given model.
    ///
    /// # Errors
    /// If the core fails to build the machine.
    pub fn new(model: Model) -> Result<Self, Error> {
        let settings = RustzxSettings {
            machine: model,
            emulation_mode: EmulationMode::FrameCount(1),
            tape_fastload_enabled: true,
            kempston_enabled: true,
            mouse_enabled: false,
            border_size: BORDER,
            ay_mode: rustzx_core::zx::sound::ay::ZXAYMode::ABC,
            ay_enabled: matches!(model, Model::Sinclair128K),
            beeper_enabled: true,
            sound_enabled: true,
            sound_volume: 100,
            sound_sample_rate: SAMPLE_RATE,
            load_default_rom: true,
            autoload_enabled: true,
        };
        let mut emulator = Emulator::new(settings, host::HeadlessContext)?;
        emulator.set_debug_interface(Watch::new());
        Ok(Self {
            emulator,
            model,
            frame_ended: false,
            passed: Vec::new(),
            tape_loaded: false,
            tape_loading: false,
            picture: Picture::new(320, BORDER.height(), FrameBufferSource::Border, ()),
            sound: Vec::with_capacity(SAMPLE_RATE / 50),
        })
    }

    /// Sets the address the machine stops at, at most once a frame, when the processor reaches
    /// it: the top of a game's main loop (`[run] sync`). `None` runs whole frames.
    pub fn set_sync(&mut self, sync: Option<u16>) {
        self.watch_mut().set_sync(sync);
    }

    /// Sets the addresses whose running [`Machine::passed`] reports, replacing any set before.
    pub fn watch(&mut self, addrs: &[u16]) {
        self.watch_mut().watch(addrs);
        self.copy_passed();
    }

    /// The watched addresses the processor has executed in this frame so far, in the order it
    /// first reached them. Still readable when `run` returns `FrameEnd`; cleared when the next
    /// frame begins.
    #[must_use]
    pub fn passed(&self) -> &[u16] {
        &self.passed
    }

    /// Runs the machine to the sync address or the end of the frame, whichever comes first.
    ///
    /// # Errors
    /// If the core reports an emulation error (for example a malformed tape block).
    pub fn run(&mut self) -> Result<Stop, Error> {
        if std::mem::take(&mut self.frame_ended) {
            self.watch_mut().new_frame();
        }
        let stop = match self.emulator.emulate_frames(FRAME_HOST_LIMIT)?.stop_reason {
            EmulationStopReason::Breakpoint if self.watch_mut().take_loaded() => Stop::Loaded,
            EmulationStopReason::Breakpoint => Stop::Sync,
            // `FrameCount(1)` never times out; a timeout would only mean a slow host.
            EmulationStopReason::Completed | EmulationStopReason::Timeout => {
                self.frame_ended = true;
                self.picture.compose(
                    self.emulator.border_buffer(),
                    self.emulator.screen_buffer(),
                    32,
                    BORDER.canvas_y(),
                );
                // The mixer fills exactly a frame's worth, sampled as the frame's T-states pass.
                self.sound.clear();
                while let Some(sample) = self.emulator.next_audio_sample() {
                    self.sound.push((sample.left, sample.right));
                }
                Stop::FrameEnd
            }
        };
        self.copy_passed();
        Ok(stop)
    }

    /// Asks `run` to stop once the tape has loaded ([`Stop::Loaded`]), once.
    pub fn stop_when_loaded(&mut self, stop: bool) {
        self.watch_mut().set_stop_when_loaded(stop);
    }

    /// Draws the screen into the picture from screen memory as it is now, inside the border as
    /// last drawn: the picture mid-frame, as when the tape has just loaded. Flashing
    /// characters show their ink as ink.
    pub fn draw_screen_from_memory(&mut self) {
        let top = BORDER.canvas_y();
        for y in 0..192u16 {
            let row = 0x4000 | ((y & 0xC0) << 5) | ((y & 0x07) << 8) | ((y & 0x38) << 2);
            for column in 0..32u16 {
                let bits = self.peek(row | column);
                let attr = self.peek(0x5800 + (y / 8) * 32 + column);
                let bright = (attr >> 6) & 1;
                let ink = (attr & 7) + bright * 8;
                let paper = ((attr >> 3) & 7) + bright * 8;
                for bit in 0..8u16 {
                    let color = if bits & (0x80 >> bit) == 0 {
                        paper
                    } else {
                        ink
                    };
                    self.picture.set(
                        32 + usize::from(column * 8 + bit),
                        top + usize::from(y),
                        color,
                    );
                }
            }
        }
    }

    /// Runs the machine to the end of the frame, through any sync stop.
    ///
    /// # Errors
    /// If the core reports an emulation error (for example a malformed tape block).
    pub fn run_frame(&mut self) -> Result<(), Error> {
        while self.run()? != Stop::FrameEnd {}
        Ok(())
    }

    /// The byte at `addr`, as the processor would read it now.
    #[must_use]
    pub fn peek(&self, addr: u16) -> u8 {
        self.emulator.peek(addr)
    }

    /// The little-endian word at `addr`.
    #[must_use]
    pub fn peek16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.peek(addr), self.peek(addr.wrapping_add(1))])
    }

    /// Writes a byte as the processor would: the ROM is left alone, and a write to display
    /// memory reaches the picture. No time passes.
    pub fn write(&mut self, addr: u16, value: u8) {
        self.emulator.write(addr, value);
    }

    /// The processor's registers.
    #[must_use]
    pub fn registers(&self) -> &Regs {
        &self.emulator.cpu().regs
    }

    /// Calls the routine at `addr` as a `CALL` at `back` would: `back` pushed as its return,
    /// and the processor sent to `addr`.
    pub fn call(&mut self, addr: u16, back: u16) {
        let regs = self.registers_mut();
        let sp = regs.get_sp().wrapping_sub(2);
        regs.set_sp(sp);
        regs.set_pc(addr);
        let [low, high] = back.to_le_bytes();
        self.write(sp, low);
        self.write(sp.wrapping_add(1), high);
    }

    /// The processor's registers, to change between frames.
    pub fn registers_mut(&mut self) -> &mut Regs {
        &mut self.emulator.cpu_mut().regs
    }

    /// A 16K RAM bank, whether paged in or not: 0 to 7 on a 128K, 0 to 2 on a 48K (from
    /// `0x4000` up). `None` past the last.
    #[must_use]
    pub fn ram_bank(&self, bank: u8) -> Option<&[u8]> {
        self.emulator.ram_bank(bank)
    }

    /// Inserts a `.tap` and types `LOAD ""`, as a player would: the core's own start-up state
    /// with the command typed, then the tape fast-loaded block by block as the ROM asks for
    /// each. The next `run` starts a new frame.
    ///
    /// # Errors
    /// If the tape isn't a well-formed `.tap`: blocks that run past its end, or one too short
    /// for its flag and checksum. (Checksums aren't checked: some tapes break them on purpose.)
    pub fn load_tap(&mut self, tap: &[u8]) -> Result<(), Error> {
        let mut rest = tap;
        let mut blocks = 0;
        while !rest.is_empty() {
            let [lo, hi, after @ ..] = rest else {
                return Err(Error::BadTape);
            };
            let len = usize::from(u16::from_le_bytes([*lo, *hi]));
            if len < 2 || len > after.len() {
                return Err(Error::BadTape);
            }
            rest = &after[len..];
            blocks += 1;
        }
        if tap.is_empty() {
            return Err(Error::BadTape);
        }
        self.emulator
            .load_tape(Tape::Tap(BufferCursor::new(tap.to_vec())))?;
        self.watch_mut().new_tape(blocks);
        self.copy_passed();
        self.frame_ended = true;
        Ok(())
    }

    /// Presses or releases a Spectrum key.
    pub fn key(&mut self, key: Key, pressed: bool) {
        self.emulator.send_key(key, pressed);
    }

    /// Moves or releases the Kempston joystick in one direction, or its fire button.
    pub fn kempston(&mut self, direction: Kempston, pressed: bool) {
        self.emulator.send_kempston_key(direction, pressed);
    }

    /// Presses or releases a key the Spectrum types with CAPS SHIFT (the cursor keys, DELETE,
    /// BREAK…), keeping CAPS SHIFT held while any of them is.
    pub fn compound_key(&mut self, key: CompoundKey, pressed: bool) {
        self.emulator.send_compound_key(key, pressed);
    }

    /// The whole machine as a standard SZX snapshot: memory, processor, paging, AY registers,
    /// border and beeper. Not the tape, and not the AY's tone and envelope position, which SZX
    /// has no place for.
    ///
    /// # Errors
    /// If the core fails to write it.
    pub fn save_szx(&mut self) -> Result<Vec<u8>, Error> {
        let mut szx = Vec::new();
        self.emulator
            .save_snapshot(SnapshotRecorder::Szx(Bytes(&mut szx)))?;
        Ok(szx)
    }

    /// Puts the machine in the state an SZX snapshot holds, and counts the tape as loaded,
    /// since what it was loading into is gone. The next `run` starts a new frame.
    ///
    /// # Errors
    /// If the snapshot is for another model, or the core can't read it.
    pub fn load_szx(&mut self, szx: &[u8]) -> Result<(), Error> {
        let model = match szx.get(6) {
            Some(1) => Model::Sinclair48K,
            Some(2) => Model::Sinclair128K,
            _ => return Err(Error::UnknownModel),
        };
        if model != self.model {
            return Err(Error::OtherModel);
        }
        self.emulator
            .load_snapshot(Snapshot::Szx(BufferCursor::new(szx.to_vec())))?;
        self.frame_ended = true;
        self.watch_mut().finish_tape();
        self.copy_passed();
        Ok(())
    }

    /// The last frame's sound, as stereo samples at [`SAMPLE_RATE`]: one frame's worth (882 at
    /// 44.1 kHz), beeper and AY mixed, sampled in emulated time.
    #[must_use]
    pub fn sound(&self) -> &[(f32, f32)] {
        &self.sound
    }

    /// The picture as it was at the end of the last frame: 320×256, the 256×192 screen with
    /// 32 pixels of border on every side, border colour changes drawn where they happened.
    #[must_use]
    pub const fn picture(&self) -> &Picture {
        &self.picture
    }

    fn watch_mut(&mut self) -> &mut Watch {
        self.emulator.debug_interface().expect(WATCH_SET)
    }

    /// Copies the watch's addresses passed, reusing the copy's memory, and whether the tape
    /// has loaded.
    fn copy_passed(&mut self) {
        let watch = self.emulator.debug_interface().expect(WATCH_SET);
        self.tape_loaded = watch.tape_loaded();
        self.tape_loading = watch.tape_loading();
        self.passed.clear();
        self.passed.extend_from_slice(watch.passed());
    }

    /// Whether the tape's every block has gone in through the ROM's loader.
    #[must_use]
    pub const fn tape_loaded(&self) -> bool {
        self.tape_loaded
    }

    /// Whether a tape is in and still loading: a snapshot now wouldn't hold the rest of it.
    #[must_use]
    pub const fn tape_loading(&self) -> bool {
        self.tape_loading
    }

    /// Whether the tape's loader waits for a key on its loading screen: in the ROM's
    /// `PAUSE 0` with blocks still to load, as a loader does that shows its screen until a key
    /// is pressed. A timed `PAUSE` goes on by itself, so it isn't waiting.
    #[must_use]
    pub fn waits_on_loading_screen(&self) -> bool {
        let regs = self.registers();
        // PAUSE 0 counts BC down from 0 to 0xFFFF and back up each time round.
        self.tape_loading
            && ROM_PAUSE_LOOP.contains(&regs.get_pc())
            && matches!(regs.get_bc(), 0 | 0xFFFF)
    }
}
