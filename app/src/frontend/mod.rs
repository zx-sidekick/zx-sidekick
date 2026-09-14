//! Window, input and sound around the emulated machine.

mod audio;
mod gamepad;
pub mod headless;
mod input;
mod prompt;
pub mod tape;
mod text;
mod video;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sidekick::starquake::{ENTRY_PC, ENTRY_SP};
use sidekick::{Input, Machine};

/// How long a Spectrum frame lasts, from the clock it is derived from
/// rather than written out.
const FRAME_PERIOD: Duration = Duration::from_nanos(zx_core::timing::FRAME_NANOS);
const FRAMES_PER_SECOND: u32 = 50;

/// How long the tape's loading picture stays up before the game starts, as it
/// would at the end of loading from a cassette.
const LOADING_FRAMES: u32 = 150;

/// State shared between the machine's thread and the window.
pub struct Shared {
    /// The most recent frame: display memory, border colour, frame number.
    pub screen: Mutex<(Vec<u8>, u8, u64)>,
    pub input: Mutex<Input>,
    /// Set when either side wants to stop: the window was closed, or the
    /// machine's thread finished.
    pub quit: AtomicBool,
    /// Set when the machine's thread stopped without being asked to, so the
    /// window can report it rather than sitting on a frozen picture.
    pub dead: AtomicBool,
}

/// The machine's thread: runs a frame, plays its sound, shows its screen,
/// and waits for the next one.
struct Runner {
    shared: Arc<Shared>,
    audio: Option<audio::Output>,
    beeper: audio::Beeper,
    pad: gamepad::Gamepad,
    next_frame: Instant,
    frame: u64,
}

impl Runner {
    /// Shows `memory` for a frame, plays `edges` over it, and waits until it
    /// is time for the next.
    fn present(&mut self, memory: &[u8], border: u8, edges: &[(u32, bool)]) {
        self.beeper.play(edges, zx_spectrum::FRAME_T);
        {
            let mut screen = self.shared.screen.lock().unwrap();
            let n = screen.0.len();
            screen.0.copy_from_slice(&memory[..n]);
            screen.1 = border;
            screen.2 = self.frame;
        }
        self.frame += 1;
        // Pace by the clock, at the Spectrum's own frame rate, leaning a
        // little on the period when the sound card's buffer strays outside
        // two to three frames' worth, so the two clocks cannot drift apart.
        let mut period = FRAME_PERIOD;
        if let Some(out) = &self.audio {
            out.push(self.beeper.samples());
            let frame = out.rate() as usize / FRAMES_PER_SECOND as usize;
            let queued = out.queued();
            if queued < frame * 2 {
                period = period.saturating_sub(Duration::from_micros(500));
            } else if queued > frame * 3 {
                period += Duration::from_micros(500);
            }
        }
        self.beeper.clear_samples();
        self.next_frame += period;
        let now = Instant::now();
        if self.next_frame > now {
            std::thread::sleep(self.next_frame - now);
        } else {
            // Fallen behind: give up the lost time rather than race to
            // catch it back.
            self.next_frame = now;
        }
    }

    fn run(&mut self, tape: &[u8]) -> Result<(), String> {
        let mut machine = Machine::from_tape(tape, ENTRY_PC, ENTRY_SP)?;
        let loading = zx_core::tape::load_tap(tape)?.loading_screen;
        if let Some(picture) = loading {
            let mut memory = vec![0u8; 0x1B00];
            memory[..picture.len().min(0x1B00)]
                .copy_from_slice(&picture[..picture.len().min(0x1B00)]);
            for _ in 0..LOADING_FRAMES {
                if self.shared.quit.load(Ordering::Relaxed) {
                    return Ok(());
                }
                self.present(&memory, 0, &[]);
            }
        }
        while !self.shared.quit.load(Ordering::Relaxed) {
            let pad = self.pad.poll();
            let input = *self.shared.input.lock().unwrap();
            let z = &mut machine.zx;
            z.keys = input.keys;
            z.kempston = input.kempston | pad.bits;
            if pad.start {
                // Starquake's pause key, Space, as the tape ships it.
                z.keys[7] &= !0x01;
            }
            machine.run_frame();
            let edges = std::mem::take(&mut machine.zx.speaker);
            let border = machine.zx.border;
            self.present(&machine.zx.mem[0x4000..0x5B00], border, &edges);
        }
        Ok(())
    }
}

fn machine_thread(tape: Vec<u8>, shared: Arc<Shared>, audio: Option<audio::Output>) {
    let watch = shared.clone();
    let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let rate = audio.as_ref().map_or(44100, audio::Output::rate);
        let mut runner = Runner {
            shared,
            audio,
            beeper: audio::Beeper::new(rate),
            pad: gamepad::Gamepad::new(),
            next_frame: Instant::now(),
            frame: 0,
        };
        runner.run(&tape)
    }));
    match played {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("error: {e}");
            watch.dead.store(true, Ordering::Relaxed);
        }
        Err(_) => watch.dead.store(true, Ordering::Relaxed),
    }
    // Either way the game is over, so the window should come down with it.
    watch.quit.store(true, Ordering::Relaxed);
}

/// The state shared between the machine and whatever is showing it.
fn new_shared() -> Arc<Shared> {
    Arc::new(Shared {
        screen: Mutex::new((vec![0; zx_core::screen::BITMAP_LEN + 768], 0, 0)),
        input: Mutex::new(Input::default()),
        quit: AtomicBool::new(false),
        dead: AtomicBool::new(false),
    })
}

/// The sound card, if there is one. The stream has to be held for as long
/// as the sound should play.
fn open_audio() -> (Option<audio::Output>, Option<cpal::Stream>) {
    match audio::Output::start() {
        Ok((out, stream)) => (Some(out), Some(stream)),
        Err(e) => {
            eprintln!("no sound: {e}");
            (None, None)
        }
    }
}

/// Starts the machine on its own thread, with sound, from a checked copy of
/// the game. Returns the sound stream, which the caller holds.
fn launch(shared: &Arc<Shared>, tape: Vec<u8>) -> Result<Option<cpal::Stream>, String> {
    let (audio, stream) = open_audio();
    let machine_shared = shared.clone();
    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || machine_thread(tape, machine_shared, audio))
        .map_err(|e| e.to_string())?;
    Ok(stream)
}

/// Runs the game in a window: from `path`, or, with none, from whatever the
/// player locates on the screen that asks for the tape.
pub fn run(path: Option<&Path>) -> Result<(), String> {
    let shared = new_shared();
    let (prompt, stream) = match path {
        Some(path) => {
            let tape = tape::read(path)?;
            (None, launch(&shared, tape)?)
        }
        None => (Some(prompt::Prompt::new()), None),
    };
    let launcher = shared.clone();
    let result = video::run(
        shared,
        prompt,
        Box::new(move |tape| launch(&launcher, tape)),
    );
    drop(stream);
    result
}
