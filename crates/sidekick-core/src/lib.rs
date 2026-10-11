//! ZX Sidekick's emulated machine, and everything the program reads from it and writes to it
//! between frames.
//!
//! The machine is `rustzx-core` from the zx-sidekick fork of `RustZX`, built on that fork's
//! `rustzx-z80`. For now this crate wraps it headless: a 48K or 128K Spectrum with the bundled
//! ROM, run a frame at a time, stopping at a game's sync address on the way. The named values,
//! cheats, input mapping and saves described in `ZX-SIDEKICK.md` are built on top of this.

mod host;
mod watch;

pub use host::{HeadlessHost, Picture};
pub use rustzx_core::zx::machine::ZXMachine as Model;
pub use rustzx_z80::Regs;

use rustzx_core::{EmulationMode, EmulationStopReason, Emulator, RustzxSettings};
use std::time::Duration;
use watch::Watch;

/// How long one call to `run_frame` may take on the host before the core gives up on it.
/// Only reached if the host is far too slow; a frame normally takes well under a millisecond.
const FRAME_HOST_LIMIT: Duration = Duration::from_millis(100);

const WATCH_SET: &str = "the watch is set when the machine is built";

/// Sample rate the beeper and AY are mixed at.
pub const SAMPLE_RATE: usize = 44_100;

/// Why [`Machine::run`] returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The processor reached the sync address, partway through a frame: the game is between two
    /// of its own frames.
    Sync,
    /// The frame ended: time to present the picture and sound.
    FrameEnd,
}

/// A Spectrum with the bundled ROM and nothing loaded, run a frame at a time.
pub struct Machine {
    emulator: Emulator<HeadlessHost>,
    /// The last `run` returned at a frame's end, so the next begins a new frame.
    frame_ended: bool,
    /// The watch's addresses passed, as of the last `run` (the core lends its debug interface
    /// mutably only).
    passed: Vec<u16>,
}

impl Machine {
    /// A freshly switched-on machine of the given model.
    ///
    /// # Errors
    /// If the core fails to build the machine.
    pub fn new(model: Model) -> Result<Self, rustzx_core::error::Error> {
        let settings = RustzxSettings {
            machine: model,
            emulation_mode: EmulationMode::FrameCount(1),
            tape_fastload_enabled: true,
            kempston_enabled: true,
            mouse_enabled: false,
            ay_mode: rustzx_core::zx::sound::ay::ZXAYMode::ABC,
            ay_enabled: matches!(model, Model::Sinclair128K),
            beeper_enabled: true,
            sound_enabled: true,
            sound_volume: 100,
            sound_sample_rate: SAMPLE_RATE,
            load_default_rom: true,
        };
        let mut emulator = Emulator::new(settings, host::HeadlessContext)?;
        emulator.set_debug_interface(Watch::new());
        Ok(Self {
            emulator,
            frame_ended: false,
            passed: Vec::new(),
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
    pub fn run(&mut self) -> Result<Stop, rustzx_core::error::Error> {
        if std::mem::take(&mut self.frame_ended) {
            self.watch_mut().new_frame();
        }
        let stop = match self.emulator.emulate_frames(FRAME_HOST_LIMIT)?.stop_reason {
            EmulationStopReason::Breakpoint => Stop::Sync,
            // `FrameCount(1)` never times out; a timeout would only mean a slow host.
            EmulationStopReason::Completed | EmulationStopReason::Timeout => {
                self.frame_ended = true;
                // The mixer fills a frame's worth of samples; nothing plays them headless.
                while self.emulator.next_audio_sample().is_some() {}
                Stop::FrameEnd
            }
        };
        self.copy_passed();
        Ok(stop)
    }

    /// Runs the machine to the end of the frame, through any sync stop.
    ///
    /// # Errors
    /// If the core reports an emulation error (for example a malformed tape block).
    pub fn run_frame(&mut self) -> Result<(), rustzx_core::error::Error> {
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

    /// The picture as it was at the end of the last frame (the screen area, without the border).
    #[must_use]
    pub fn picture(&self) -> &Picture {
        self.emulator.screen_buffer()
    }

    fn watch_mut(&mut self) -> &mut Watch {
        self.emulator.debug_interface().expect(WATCH_SET)
    }

    /// Copies the watch's addresses passed, reusing the copy's memory.
    fn copy_passed(&mut self) {
        let watch = self.emulator.debug_interface().expect(WATCH_SET);
        self.passed.clear();
        self.passed.extend_from_slice(watch.passed());
    }
}
