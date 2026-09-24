//! What every ZX Sidekick game shows the player through (#141): the window,
//! sound, the keyboard and a gamepad, pausing as a freeze, the screen that
//! asks for the tape, and text drawn at the window's own resolution.
//!
//! A game supplies a [`Game`], which says how its tape is found and what the
//! window and the prompt call it, and a [`video::Screen`], which draws its
//! panel and takes its keys. Its own loop runs the machine on the machine's
//! thread ([`launch`]), and paces the frames with a [`Pacer`].

pub mod audio;
pub mod cli;
pub mod freeze;
pub mod gamepad;
pub mod input;
pub mod notice;
pub mod overlay;
pub mod picker;
pub mod prompt;
pub mod tape;
pub mod text;
pub mod video;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sidekick::Input;

/// What the shared frontend needs to know about a game: how its tape is
/// found and checked, and what the window and the prompt say about it.
#[derive(Debug)]
pub struct Game {
    /// The game's name as a sentence says it: "Starquake".
    pub name: &'static str,
    /// The release this version supports, as a refusal names it: "the
    /// original Bubble Bus release".
    pub release: &'static str,
    /// The program's own name: its binary, and its folder in the data dir.
    pub program: &'static str,
    /// The window's title.
    pub title: &'static str,
    /// The names the tape goes by, in the order they are tried: the
    /// project's own and the archive's, each bare and zipped. Compared
    /// without regard to case.
    pub names: &'static [&'static str],
    /// What a located tape is kept as, in the program's folder.
    pub kept: &'static str,
    /// Whether a file is the tape this version supports, by its SHA-1.
    pub accept: fn(&[u8]) -> bool,
    /// That SHA-1, whose start a refusal quotes.
    pub sha1: &'static str,
    /// The prompt's heading, in capitals.
    pub heading: &'static str,
    /// What the prompt says the program reads from the tape.
    pub about: &'static str,
    /// The zip the archive serves the tape in, as the prompt names it.
    pub zip: &'static str,
    /// The archive's page for the game, and that page as the prompt shows it.
    pub page: &'static str,
    pub page_shown: &'static str,
    /// The game's credit, at the foot of the prompt.
    pub credit: &'static str,
}

/// How long a Spectrum frame lasts, from the clock it is derived from
/// rather than written out.
const FRAME_PERIOD: Duration = Duration::from_nanos(zx_core::timing::FRAME_NANOS);
const FRAMES_PER_SECOND: u32 = 50;

/// State shared between the machine's thread and the window.
pub struct Shared<G> {
    /// The most recent frame: display memory, border colour, frame number,
    /// and whether the game is paused: the emulation frozen.
    pub screen: Mutex<(Vec<u8>, u8, u64, bool)>,
    pub input: Mutex<Input>,
    /// Set when either side wants to stop: the window was closed, or the
    /// machine's thread finished.
    pub quit: AtomicBool,
    /// Set when the machine's thread stopped without being asked to, so the
    /// window can report it rather than sitting on a frozen picture.
    pub dead: AtomicBool,
    /// Why it stopped, for the message the player sees: a program started
    /// from a file manager has no console to read it in.
    pub why: Mutex<Option<String>>,
    /// Set by the window when it loses the focus, and taken by the game's
    /// loop, which pauses a game in play then (#25): switching away should
    /// not leave Blob or Willy to die unattended.
    pub unfocused: AtomicBool,
    /// The game's own part: what its panel shows, which both sides use.
    pub game: G,
}

impl<G> Shared<G> {
    /// The state shared between the machine and whatever is showing it,
    /// with the game's part `game`.
    pub fn new(game: G) -> Arc<Shared<G>> {
        Arc::new(Shared {
            screen: Mutex::new((vec![0; zx_core::screen::BITMAP_LEN + 768], 0, 0, false)),
            input: Mutex::new(Input::default()),
            quit: AtomicBool::new(false),
            dead: AtomicBool::new(false),
            why: Mutex::new(None),
            unfocused: AtomicBool::new(false),
            game,
        })
    }
}

/// Paces the machine's frames: shows each on the shared screen, plays its
/// sound, and waits until it is time for the next.
pub struct Pacer {
    audio: Option<audio::Output>,
    beeper: audio::Beeper,
    next_frame: Instant,
    frame: u64,
}

impl Pacer {
    fn new(audio: Option<audio::Output>) -> Pacer {
        let rate = audio.as_ref().map_or(44100, audio::Output::rate);
        Pacer {
            audio,
            beeper: audio::Beeper::new(rate),
            next_frame: Instant::now(),
            frame: 0,
        }
    }

    /// Starts the pacing again from now, after time that is not the game's
    /// has passed: a picker held open, or the emulation frozen.
    pub fn restart(&mut self) {
        self.next_frame = Instant::now();
    }

    /// Shows `memory` for a frame, plays `edges` over it, and waits until it
    /// is time for the next.
    ///
    /// # Panics
    ///
    /// If the window's thread panicked while holding the screen.
    pub fn present<G>(
        &mut self,
        shared: &Shared<G>,
        memory: &[u8],
        border: u8,
        edges: &[(u32, bool)],
    ) {
        self.beeper.play(edges, zx_spectrum::FRAME_T);
        {
            let mut screen = shared.screen.lock().unwrap();
            let n = screen.0.len();
            screen.0.copy_from_slice(&memory[..n]);
            screen.1 = border;
            screen.2 = self.frame;
            screen.3 = false;
        }
        self.frame += 1;
        // Pace by the clock, at the Spectrum's own frame rate, leaning a
        // little on the period when the sound card's buffer strays outside
        // two to three frames' worth, so the two clocks cannot drift apart.
        let mut period = FRAME_PERIOD;
        // A sound device that has gone (headphones unplugged) takes no more
        // samples, and its queue would stay full and slow every frame: pace
        // by the clock alone from then on.
        if self.audio.as_ref().is_some_and(audio::Output::lost) {
            self.audio = None;
        }
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
}

/// A game's loop on the machine's thread: given the checked tape, the
/// shared state and the pacer, it runs until the game is over or asked to
/// stop.
pub type Play<G> = fn(&[u8], &Arc<Shared<G>>, Pacer) -> Result<(), String>;

fn machine_thread<G>(
    tape: &[u8],
    shared: &Arc<Shared<G>>,
    audio: Option<audio::Output>,
    play: Play<G>,
) {
    let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        play(tape, shared, Pacer::new(audio))
    }));
    match played {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("error: {e}");
            *shared.why.lock().unwrap() = Some(e);
            shared.dead.store(true, Ordering::Relaxed);
        }
        Err(panic) => {
            // The panic has printed itself where there is a console; the
            // message is kept for the dialog where there is not.
            let said = panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "no reason given".into());
            *shared.why.lock().unwrap() = Some(format!("the game stopped unexpectedly: {said}"));
            shared.dead.store(true, Ordering::Relaxed);
        }
    }
    // Either way the game is over, so the window should come down with it.
    shared.quit.store(true, Ordering::Relaxed);
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
/// the game, running `play`. Returns the sound stream, which the caller
/// holds.
///
/// # Errors
///
/// If the thread cannot be started.
pub fn launch<G: Send + Sync + 'static>(
    shared: &Arc<Shared<G>>,
    tape: Vec<u8>,
    play: Play<G>,
) -> Result<Option<cpal::Stream>, String> {
    let (audio, stream) = open_audio();
    let machine_shared = shared.clone();
    std::thread::Builder::new()
        .name("machine".into())
        .spawn(move || machine_thread(&tape, &machine_shared, audio, play))
        .map_err(|e| e.to_string())?;
    Ok(stream)
}

/// Runs `game` in a window: from the tape in `tape` if one was found, or,
/// with none, from whatever the player locates on the screen that asks for
/// it. `play` is the game's loop on the machine's thread.
///
/// # Errors
///
/// If the window cannot be made, the tape cannot be started, or the game
/// stopped unexpectedly.
pub fn run<G: video::Screen>(
    game: &'static Game,
    shared: Arc<Shared<G>>,
    tape: Option<Vec<u8>>,
    play: Play<G>,
) -> Result<(), String> {
    let (prompt, stream) = match tape {
        Some(tape) => (None, launch(&shared, tape, play)?),
        None => (Some(prompt::Prompt::new(game)), None),
    };
    let launcher = shared.clone();
    let result = video::run(
        game,
        shared,
        prompt,
        Box::new(move |tape| launch(&launcher, tape, play)),
    );
    drop(stream);
    result
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Game;

    /// A game for the tests: Starquake's words, and a made-up tape.
    pub const GAME: Game = Game {
        name: "Starquake",
        release: "the original Bubble Bus release",
        program: "zx-sidekick-starquake",
        title: "ZX Sidekick · Starquake",
        names: &["starquake.tap"],
        kept: "starquake.tap",
        accept: |bytes| bytes == b"the tape",
        sha1: "65450d6f33692c2c2868c0b497037f2cfd0ef3bd",
        heading: "STARQUAKE",
        about: "Nothing from the original game is included. The graphics, maps and sound are \
                read from your own Starquake tape each time the game starts.",
        zip: "Starquake.tap.zip",
        page: "https://worldofspectrum.net/item/0004873/",
        page_shown: "worldofspectrum.net/item/0004873",
        credit: "Starquake \u{a9} 1985 Stephen Crow / Bubble Bus Software. Not affiliated.",
    };
}
