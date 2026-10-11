//! The types `rustzx-core` asks its host for, for a machine with no window and no sound device.

use crate::watch::Watch;
use rustzx_core::error::IoError;
use rustzx_core::{
    host::{
        BufferCursor, DataRecorder, FrameBuffer, FrameBufferSource, Host, HostContext, Stopwatch,
        StubIoExtender,
    },
    zx::video::colors::{ZXBrightness, ZXColor},
};
use std::time::{Duration, Instant};

/// The host for a machine run without a window: tests, headless runs, checks.
pub struct HeadlessHost;

impl Host for HeadlessHost {
    type Context = HeadlessContext;
    type TapeAsset = BufferCursor<Vec<u8>>;
    type FrameBuffer = Picture;
    type EmulationStopwatch = WallClock;
    type IoExtender = StubIoExtender;
    type DebugInterface = Watch;
}

pub struct HeadlessContext;

impl HostContext<HeadlessHost> for HeadlessContext {
    fn frame_buffer_context(&self) -> <Picture as FrameBuffer>::Context {}
}

/// A picture as the Spectrum's 16 colors: 0–7 normal, 8–15 bright.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Picture {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Picture {
    /// Its width in pixels.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Its height in pixels.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// The color at (`x`, `y`): 0–7 normal, 8–15 bright.
    #[must_use]
    pub fn color(&self, x: usize, y: usize) -> u8 {
        self.pixels[x + y * self.width]
    }
}

impl FrameBuffer for Picture {
    type Context = ();

    fn new(width: usize, height: usize, _source: FrameBufferSource, (): ()) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    fn set_color(&mut self, x: usize, y: usize, color: ZXColor, brightness: ZXBrightness) {
        self.pixels[x + y * self.width] = color as u8 + brightness as u8 * 8;
    }
}

pub struct WallClock(Instant);

impl Stopwatch for WallClock {
    fn new() -> Self {
        Self(Instant::now())
    }

    fn measure(&self) -> Duration {
        self.0.elapsed()
    }
}

/// Collects what the core writes, for snapshots kept in memory.
pub(crate) struct Bytes<'a>(pub(crate) &'a mut Vec<u8>);

impl DataRecorder for Bytes<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
}
