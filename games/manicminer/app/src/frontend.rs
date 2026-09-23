//! Manic Miner in the window every game shares (`sidekick-frontend`): no
//! panel yet; over the picture the training picker (#148) and the pause
//! notice; and the machine's loop, which feeds the game the keyboard and
//! the Kempston joystick, and carries out what the picker asks.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use manicminer::facts::{at, routine};
use sidekick_frontend::gamepad::{self, Layout};
use sidekick_frontend::text::{Canvas, Fonts};
use sidekick_frontend::video::{Key, Screen};
use sidekick_frontend::{Game, Pacer, freeze, notice, tape};
use winit::keyboard::KeyCode;

use crate::picker::{self, Action, Picker};

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
    /// The pad's letters, for the legends.
    layout: Mutex<Layout>,
    /// The training picker.
    picker: Mutex<Picker>,
    /// The cavern being played, which Go to cavern starts from.
    cavern: Mutex<u8>,
}

impl Guide {
    /// Opens the picker, starting Go to cavern from the cavern being played.
    fn open(&self) {
        let cavern = *self.cavern.lock().unwrap();
        self.picker.lock().unwrap().open(cavern);
    }
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
    /// The pad's letters and the picker's version.
    type Stamp = (Layout, u64);
    type Shown = (Layout, Picker);
    type Painter = Painter;

    fn stamp(&self) -> Self::Stamp {
        let layout = *self.layout.lock().unwrap();
        (layout, self.picker.lock().unwrap().version())
    }

    fn shown_unless(&self, since: Option<Self::Stamp>) -> Option<(Self::Stamp, Self::Shown)> {
        let layout = *self.layout.lock().unwrap();
        let picker = self.picker.lock().unwrap();
        let stamp = (layout, picker.version());
        (since != Some(stamp)).then(|| (stamp, (layout, picker.clone())))
    }

    /// The picker while it is open, and otherwise the pause notice while
    /// paused.
    fn draw(
        painter: &mut Painter,
        canvas: &mut Canvas,
        (layout, picker): &Self::Shown,
        paused: bool,
    ) {
        canvas.clear_transparent();
        if picker.is_open() {
            picker::draw(&mut painter.0, canvas, picker, *layout);
        } else if paused {
            notice::draw(&mut painter.0, canvas, *layout, "jump");
        }
    }

    /// Esc opens the picker and goes back; while it is open the arrows and
    /// Enter work it, and it has the keyboard to itself. Every other key is
    /// the game's.
    fn key(&self, code: KeyCode) -> Key {
        let open = self.picker.lock().unwrap().is_open();
        if code == KeyCode::Escape && !open {
            self.open();
            // Opening it: whatever was held is let go, as the game will not
            // see the key-ups.
            return Key::Taken {
                release: true,
                quit: false,
            };
        }
        if !open {
            return Key::Pass;
        }
        let mut picker = self.picker.lock().unwrap();
        match code {
            KeyCode::Escape => picker.back(),
            KeyCode::ArrowUp => picker.focus_up(),
            KeyCode::ArrowDown => picker.focus_down(),
            KeyCode::ArrowLeft => picker.change(false),
            KeyCode::ArrowRight => picker.change(true),
            KeyCode::Enter | KeyCode::NumpadEnter => picker.enter(),
            _ => {}
        }
        Key::Taken {
            release: false,
            quit: picker.exiting(),
        }
    }
}

/// Holds the game while the picker is open, taking the pad's side of it: up
/// and down choose a row, left and right change it, A does the highlighted
/// thing, and B or Select goes back. No time passes for the game, so its
/// pacing starts again from now. Returns no input for the frame it resumes
/// on, so the button that closed the picker is not also a jump.
fn hold_for_picker(shared: &Shared, pad: &mut gamepad::Gamepad, pacer: &mut Pacer) -> gamepad::Pad {
    while shared.game.picker.lock().unwrap().is_open() && !shared.quit.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(20));
        let now = pad.poll();
        *shared.game.layout.lock().unwrap() = now.layout;
        let mut picker = shared.game.picker.lock().unwrap();
        if now.select || now.cancel() {
            picker.back();
        }
        if now.up {
            picker.focus_up();
        }
        if now.down {
            picker.focus_down();
        }
        if now.left {
            picker.change(false);
        }
        if now.right {
            picker.change(true);
        }
        if now.confirm() {
            picker.enter();
        }
    }
    pacer.restart();
    gamepad::Pad::default()
}

/// The caverns' names, as the game has them, from the machine's memory.
fn cavern_names(machine: &manicminer::Machine) -> Vec<String> {
    (0..20)
        .map(|n| {
            let at = usize::from(at::CAVERNS) + 1024 * n + 512;
            String::from_utf8_lossy(&machine.zx.mem[at..at + 32])
                .trim()
                .to_string()
        })
        .collect()
}

/// Manic Miner's loop on the machine's thread.
fn play(tape: &[u8], shared: &Arc<Shared>, mut pacer: Pacer) -> Result<(), String> {
    let mut machine = manicminer::start(tape)?;
    machine.watch = vec![routine::MAIN_LOOP, routine::TITLE];
    shared
        .game
        .picker
        .lock()
        .unwrap()
        .set_names(cavern_names(&machine));
    let mut pad = gamepad::Gamepad::new(BUTTONS);
    // Whether End this game is holding the game's own quit keys, CAPS SHIFT
    // and SPACE, until it is back at the title screen.
    let mut ending = false;
    // A cavern to go to, waiting for a game: the demo reads the cheat's keys
    // too, and must not take it.
    let mut going = None;
    let mut freeze = freeze::Freeze::default();
    // Whether the game's pause key was pressed in the last frame, and how
    // long since its main loop ran.
    let mut pause = false;
    let mut since_loop = PLAY_FRAMES;
    while !shared.quit.load(Ordering::Relaxed) {
        let mut now = pad.poll();
        *shared.game.layout.lock().unwrap() = now.layout;
        if now.select && !shared.game.picker.lock().unwrap().is_open() {
            shared.game.open();
        }
        if shared.game.picker.lock().unwrap().is_open() {
            now = hold_for_picker(shared, &mut pad, &mut pacer);
        }
        let (action, training) = {
            let mut picker = shared.game.picker.lock().unwrap();
            (picker.take(), picker.training())
        };
        // A game, not the title screen or the demo: what the picker does to
        // a game waits for one, as Starquake's does.
        let in_game = since_loop < PLAY_FRAMES && machine.zx.mem[usize::from(at::DEMO)] == 0;
        match action {
            Some(Action::Exit) => return Ok(()),
            Some(Action::EndGame) if in_game => {
                ending = true;
                freeze.thaw();
            }
            Some(Action::GoTo(cavern)) => {
                going = Some(cavern);
                freeze.thaw();
            }
            Some(Action::EndGame) | None => {}
        }
        // Handed to the machine in a game, and taken back if the game ends
        // before the cheat is typed.
        if in_game {
            if let Some(cavern) = going.take() {
                machine.rules.go_to = Some(cavern);
            }
        } else if let Some(cavern) = machine.rules.go_to.take() {
            going = Some(cavern);
        }
        // Training steers only a game; anywhere else the machine does
        // nothing the game would not.
        machine.rules.training = if in_game {
            training
        } else {
            manicminer::play::Training::default()
        };
        let pad = now;
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
        if ending {
            machine.zx.set_key(zx_spectrum::Key::Matrix(0, 0), true);
            machine.zx.set_key(zx_spectrum::Key::Matrix(7, 0), true);
        }
        // The game reads the Kempston port for itself, having found one
        // there at the title screen.
        machine.zx.kempston = joystick;
        machine.rules.start = pad.start;
        let hits = machine.run_frame();
        since_loop = if hits.contains(&routine::MAIN_LOOP) {
            0
        } else {
            since_loop.saturating_add(1)
        };
        if hits.contains(&routine::TITLE) {
            ending = false;
        }
        *shared.game.cavern.lock().unwrap() = machine.zx.mem[usize::from(at::CAVERN)];
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
