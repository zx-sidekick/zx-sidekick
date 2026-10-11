//! ZX Sidekick's emulated machine, and everything the program reads from it and writes to it
//! between frames.
//!
//! The machine is `rustzx-core` from the zx-sidekick fork of `RustZX`, built on that fork's
//! `rustzx-z80`. For now this crate only wraps it headless: a 48K or 128K Spectrum with the
//! bundled ROM, run a frame at a time. The frame runner, named values, cheats, input mapping
//! and saves described in `ZX-SIDEKICK.md` are built on top of this.

mod host;

pub use host::{HeadlessHost, Picture};
pub use rustzx_core::zx::machine::ZXMachine as Model;
pub use rustzx_z80::Breakpoints;

use rustzx_core::{EmulationMode, Emulator, RustzxSettings};
use std::time::Duration;

/// How long one call to `run_frame` may take on the host before the core gives up on it.
/// Only reached if the host is far too slow; a frame normally takes well under a millisecond.
const FRAME_HOST_LIMIT: Duration = Duration::from_millis(100);

/// Sample rate the beeper and AY are mixed at.
pub const SAMPLE_RATE: usize = 44_100;

/// A Spectrum with the bundled ROM and nothing loaded, run a frame at a time.
pub struct Machine {
    emulator: Emulator<HeadlessHost>,
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
        let emulator = Emulator::new(settings, host::HeadlessContext)?;
        Ok(Self { emulator })
    }

    /// Runs the machine to the end of the next frame.
    ///
    /// # Errors
    /// If the core reports an emulation error (for example a malformed tape block).
    pub fn run_frame(&mut self) -> Result<(), rustzx_core::error::Error> {
        self.emulator.emulate_frames(FRAME_HOST_LIMIT)?;
        // The mixer fills a frame's worth of samples; nothing plays them headless.
        while self.emulator.next_audio_sample().is_some() {}
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

    /// The picture as it was at the end of the last frame (the screen area, without the border).
    #[must_use]
    pub fn picture(&self) -> &Picture {
        self.emulator.screen_buffer()
    }
}
