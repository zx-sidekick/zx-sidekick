//! Beeper sound: turning the game's speaker toggles into samples, and
//! playing them.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const CPU_HZ: f64 = zx_core::timing::CPU_HZ as f64;
const VOLUME: f32 = 0.25;

/// Converts speaker levels over time (in T-states) into samples.
pub struct Beeper {
    rate: f64,
    level: bool,
    /// T-states into the current sample, and the level integrated over it.
    sample_t: f64,
    acc: f64,
    samples: Vec<f32>,
    /// Simple DC blocker, so a speaker left high does not sit off-centre.
    dc_in: f32,
    dc_out: f32,
}

impl Beeper {
    pub fn new(rate: u32) -> Beeper {
        Beeper {
            rate: rate as f64,
            level: false,
            sample_t: 0.0,
            acc: 0.0,
            samples: Vec::new(),
            dc_in: 0.0,
            dc_out: 0.0,
        }
    }

    /// Holds the current level for `t` T-states.
    fn advance(&mut self, mut t: f64) {
        let per_sample = CPU_HZ / self.rate;
        let level = if self.level { 1.0 } else { -1.0 };
        while t > 0.0 {
            let step = t.min(per_sample - self.sample_t);
            self.acc += level * step;
            self.sample_t += step;
            t -= step;
            if self.sample_t >= per_sample {
                let x = (self.acc / per_sample) as f32 * VOLUME;
                let y = x - self.dc_in + 0.995 * self.dc_out;
                self.dc_in = x;
                self.dc_out = y;
                self.samples.push(y);
                self.sample_t = 0.0;
                self.acc = 0.0;
            }
        }
    }

    /// Plays a frame's speaker changes (see `Game::frame_sound`) over the
    /// `t` T-states the frame lasted. The level before the first change is
    /// whatever the speaker was left at.
    pub fn play(&mut self, edges: &[(u32, bool)], t: u32) {
        let mut now = 0;
        for &(at, level) in edges {
            let at = at.min(t);
            self.advance(f64::from(at.saturating_sub(now)));
            now = now.max(at);
            self.level = level;
        }
        self.advance(f64::from(t - now));
    }

    /// The samples generated since the last [`Beeper::clear_samples`]. Kept
    /// rather than handed over, so the buffer is reused instead of a fresh
    /// one being allocated every frame.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn clear_samples(&mut self) {
        self.samples.clear();
    }
}

/// Builds the output stream for whatever sample format the device wants,
/// converting from the mono f32 the beeper produces.
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    queue: Arc<Mutex<VecDeque<f32>>>,
    lost: Arc<AtomicBool>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let mut q = queue.lock().unwrap();
                for frame in data.chunks_mut(channels) {
                    frame.fill(T::from_sample(q.pop_front().unwrap_or(0.0)));
                }
            },
            move |e| {
                eprintln!("sound error: {e}");
                lost.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())
}

/// The sound card, fed through a queue of mono samples.
pub struct Output {
    queue: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
    /// Set when the stream reports an error, such as the device being
    /// unplugged: it takes no more samples.
    lost: Arc<AtomicBool>,
}

impl Output {
    /// Opens the default output device and starts a stream on it, which
    /// plays whatever is [pushed](Output::push).
    ///
    /// # Errors
    ///
    /// If there is no output device, or it takes no format we can write.
    pub fn start() -> Result<(Output, cpal::Stream), String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.config();
        let channels = config.channels as usize;
        let rate = config.sample_rate;
        let queue = Arc::new(Mutex::new(VecDeque::<f32>::new()));
        let lost = Arc::new(AtomicBool::new(false));
        // The device decides the sample format. CoreAudio converts from f32
        // for us, but WASAPI in shared mode and ALSA `hw:` devices that
        // default to 16-bit reject an f32 stream outright, which showed up as
        // "no sound" and a silent game.
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                build::<f32>(&device, config, channels, queue.clone(), lost.clone())?
            }
            cpal::SampleFormat::I16 => {
                build::<i16>(&device, config, channels, queue.clone(), lost.clone())?
            }
            cpal::SampleFormat::U16 => {
                build::<u16>(&device, config, channels, queue.clone(), lost.clone())?
            }
            other => return Err(format!("sample format {other} is not supported")),
        };
        stream.play().map_err(|e| e.to_string())?;
        Ok((Output { queue, rate, lost }, stream))
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Whether the stream has failed, and plays nothing more.
    pub fn lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }

    /// Queues `samples` to play.
    ///
    /// # Panics
    ///
    /// If the sound card's thread panicked while holding the queue.
    pub fn push(&self, samples: &[f32]) {
        let mut q = self.queue.lock().unwrap();
        q.extend(samples);
        // If the game ever runs ahead, drop the oldest rather than let the
        // sound fall behind the picture for good. The cap has to clear the
        // longest thing the game can hand over at once, or it would eat the
        // start of it: the death explosion is 1.44s, and a core piece going
        // in queues 25 effects together, near 2.9s.
        let cap = self.rate as usize * 4;
        if q.len() > cap {
            let excess = q.len() - cap;
            q.drain(..excess);
        }
    }

    /// How many samples are waiting to play.
    ///
    /// # Panics
    ///
    /// If the sound card's thread panicked while holding the queue.
    pub fn queued(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 44100;
    const FRAME_T: u32 = zx_core::timing::FRAME_T;

    /// Samples in one frame at 44.1 kHz: 69888 T-states of 3.5 MHz.
    fn per_frame() -> f64 {
        f64::from(FRAME_T) * f64::from(RATE) / CPU_HZ
    }

    #[test]
    fn a_frame_makes_a_frame_of_samples() {
        let mut b = Beeper::new(RATE);
        for _ in 0..50 {
            b.play(&[], FRAME_T);
        }
        let expected = 50.0 * per_frame();
        assert!((b.samples().len() as f64 - expected).abs() <= 1.0);
    }

    #[test]
    fn a_speaker_left_alone_settles_to_silence() {
        let mut b = Beeper::new(RATE);
        b.play(&[(0, true)], FRAME_T);
        for _ in 0..100 {
            b.clear_samples();
            b.play(&[], FRAME_T);
        }
        assert!(b.samples().iter().all(|s| s.abs() < 0.01));
    }

    #[test]
    fn a_toggling_speaker_swings_both_ways_within_the_volume() {
        let mut b = Beeper::new(RATE);
        // 440 Hz: a change every 3977 T-states.
        let half = CPU_HZ as u32 / 880;
        for frame in 0..10u32 {
            let start = frame * FRAME_T;
            let edges: Vec<(u32, bool)> = (0..)
                .map(|n| n * half)
                .skip_while(|&t| t < start)
                .take_while(|&t| t < start + FRAME_T)
                .map(|t| (t - start, (t / half).is_multiple_of(2)))
                .collect();
            b.play(&edges, FRAME_T);
        }
        let s = b.samples();
        let (min, max) = s
            .iter()
            .fold((0f32, 0f32), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        assert!(max > 0.2 && min < -0.2, "swings {min}..{max}");
        assert!(max <= 2.0 * VOLUME && min >= -2.0 * VOLUME);
    }

    #[test]
    fn a_change_past_the_frame_counts_at_its_end() {
        let mut b = Beeper::new(RATE);
        b.play(&[(FRAME_T + 500, true)], FRAME_T);
        assert!(b.level);
        let n = b.samples().len();
        b.clear_samples();
        assert!(b.samples().is_empty());
        assert!((n as f64 - per_frame()).abs() <= 1.0);
    }
}
