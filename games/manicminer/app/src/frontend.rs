//! Manic Miner in the window every game shares (`sidekick-frontend`): no
//! panel yet, only the pause notice over the picture, and the machine's
//! loop, which feeds the game the keyboard and the Kempston joystick.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use manicminer::facts::routine;
use sidekick_frontend::gamepad::{self, Layout};
use sidekick_frontend::text::{Canvas, Fonts};
use sidekick_frontend::video::{Key, Screen};
use sidekick_frontend::{Game, Pacer, freeze, notice, tape};
use winit::keyboard::KeyCode;

/// Manic Miner, as the shared frontend finds its tape and names it.
pub static GAME: Game = Game {
    name: "Manic Miner",
    release: "the original Bug-Byte release",
    program: "zx-sidekick-manicminer",
    title: "ZX Sidekick · Manic Miner",
    // This project's name and the name the archive's zip holds, each bare
    // and zipped.
    names: &[
        "manicminer.tap",
        "manic.tap",
        "manicminer.tap.zip",
        "manic.tap.zip",
    ],
    kept: "manicminer.tap",
    accept: manicminer::facts::is_supported_tape,
    sha1: manicminer::facts::TAPE_SHA1,
    heading: "MANIC MINER",
    about: "Nothing from the original game is included. The caverns, graphics and sound are \
            read from your own Manic Miner tape each time the game starts.",
    zip: "ManicMiner.tap.zip",
    page: "https://worldofspectrum.net/item/0003012/",
    page_shown: "worldofspectrum.net/item/0003012",
    credit: "Manic Miner \u{a9} 1983 Matthew Smith / Bug-Byte Software. Not affiliated.",
};

/// Both lower face buttons jump: the one thing a player of Manic Miner
/// presses besides moving.
const BUTTONS: gamepad::Buttons = gamepad::Buttons {
    south: sidekick::machine::JOY_FIRE,
    west: sidekick::machine::JOY_FIRE,
};

/// How many frames the game is taken to be in play after its main loop last
/// ran: the loop comes round about every four.
const PLAY_FRAMES: u32 = 12;

/// Manic Miner's part of the state the machine's thread and the window
/// share.
#[derive(Default)]
pub struct Guide {
    /// The pad's letters, for the pause notice's legend.
    layout: Mutex<Layout>,
}

/// The state shared between the machine's thread and the window.
pub type Shared = sidekick_frontend::Shared<Guide>;

/// The pause notice's letters.
pub struct Painter(Fonts);

impl Default for Painter {
    fn default() -> Painter {
        Painter(Fonts::load())
    }
}

impl Screen for Guide {
    const PANEL_W: usize = 0;
    type Stamp = Layout;
    type Shown = Layout;
    type Painter = Painter;

    fn stamp(&self) -> Layout {
        *self.layout.lock().unwrap()
    }

    fn shown_unless(&self, since: Option<Layout>) -> Option<(Layout, Layout)> {
        let layout = self.stamp();
        (since != Some(layout)).then_some((layout, layout))
    }

    /// Nothing over the picture but the pause notice, while paused.
    fn draw(painter: &mut Painter, canvas: &mut Canvas, layout: &Layout, paused: bool) {
        canvas.clear_transparent();
        if paused {
            notice::draw(&mut painter.0, canvas, *layout, "jump");
        }
    }

    /// Every key is the game's.
    fn key(&self, _code: KeyCode) -> Key {
        Key::Pass
    }
}

/// Manic Miner's loop on the machine's thread.
fn play(tape: &[u8], shared: &Arc<Shared>, mut pacer: Pacer) -> Result<(), String> {
    let mut machine = manicminer::start(tape)?;
    machine.watch = vec![routine::MAIN_LOOP];
    let mut pad = gamepad::Gamepad::new(BUTTONS);
    let mut freeze = freeze::Freeze::default();
    // Whether the game's pause key was pressed in the last frame, and how
    // long since its main loop ran.
    let mut pause = false;
    let mut since_loop = PLAY_FRAMES;
    while !shared.quit.load(Ordering::Relaxed) {
        let pad = pad.poll();
        *shared.game.layout.lock().unwrap() = pad.layout;
        let input = *shared.input.lock().unwrap();
        let joystick = input.joystick | pad.bits;
        // Paused: no frame runs until a key, a direction, jump or Start,
        // and the window shows the notice meanwhile.
        let held = freeze::Held {
            start: pad.start,
            keys: input.keys,
            joystick,
        };
        if freeze.poll(held, pause, since_loop < PLAY_FRAMES) {
            pause = false;
            shared.screen.lock().unwrap().3 = true;
            std::thread::sleep(Duration::from_millis(20));
            pacer.restart();
            continue;
        }
        machine.zx.keys = input.keys;
        // The game reads the Kempston port for itself, having found one
        // there at the title screen.
        machine.zx.kempston = joystick;
        machine.rules.start = pad.start;
        let hits = machine.run_frame();
        since_loop = if hits.is_empty() {
            since_loop.saturating_add(1)
        } else {
            0
        };
        pause = machine.rules.pause_pressed;
        let edges = std::mem::take(&mut machine.zx.speaker);
        let border = machine.zx.border;
        pacer.present(shared, &machine.zx.mem[0x4000..0x5B00], border, &edges);
    }
    Ok(())
}

/// Runs the game in a window: from `path`, or, with none, from whatever the
/// player locates on the screen that asks for the tape.
pub fn run(path: Option<&Path>) -> Result<(), String> {
    let tape = path.map(|path| tape::read(&GAME, path)).transpose()?;
    sidekick_frontend::run(&GAME, Shared::new(Guide::default()), tape, play)
}

/// Runs the game without a window for `frames` frames, ENTER held on the
/// title screen to start a game, and writes a picture every 250 frames into
/// `dir`.
pub fn headless(path: &Path, frames: u64, dir: &Path) -> Result<(), String> {
    use sidekick_frontend::video::{FULL_H, FULL_W, draw};
    let tape = tape::read(&GAME, path)?;
    let mut machine = manicminer::start(&tape)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let enter = zx_spectrum::Key::by_name("enter").expect("a key");
    let mut rgba = vec![0u8; FULL_W * FULL_H * 4];
    for frame in 0..frames {
        machine.zx.set_key(enter, (100..300).contains(&frame));
        machine.run_frame();
        machine.zx.speaker.clear();
        if frame % 250 == 249 {
            draw(
                &machine.zx.mem[0x4000..0x5B00],
                machine.zx.border,
                frame,
                &mut rgba,
            );
            let pixels: Vec<u32> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
                .collect();
            let file = dir.join(format!("frame-{frame:05}.png"));
            std::fs::write(&file, zx_core::png::encode(&pixels, FULL_W, FULL_H))
                .map_err(|e| format!("{}: {e}", file.display()))?;
        }
    }
    Ok(())
}
