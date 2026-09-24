//! Manic Miner in the window every game shares (`sidekick-frontend`): the
//! guidance panel beside the picture (#153); over them the picker (#148)
//! and the pause notice; and the machine's loop, which feeds the game the
//! keyboard and the Kempston joystick, carries out what the picker asks,
//! and follows the game for the panel.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use manicminer::facts::{at, routine};
use manicminer::preview::{self, Jump};
use sidekick_frontend::gamepad::{self, Layout};
use sidekick_frontend::text::{Canvas, Fonts};
use sidekick_frontend::video::{Key, Screen};
use sidekick_frontend::{Game, Pacer, freeze, notice, tape};
use winit::keyboard::KeyCode;

use crate::panel::{self, Follow, View};
use crate::picker::{self, Action, Picker};

/// The panel beside the picture, in Spectrum pixels: as wide as
/// Starquake's.
pub const PANEL_W: usize = 136;

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
    /// What the panel shows, and a count that moves whenever it changes.
    view: Mutex<(View, u64)>,
    /// The last jump preview the worker finished (#155), and where Willy
    /// stood for it.
    jumps: Mutex<Option<(Place, Vec<Jump>)>>,
}

/// Where Willy is, as a preview is for: the cavern, his middle and the way
/// he faces.
type Place = (u8, (u8, u8), u8);

fn place(machine: &manicminer::Machine) -> Place {
    (
        machine.zx.mem[usize::from(at::CAVERN)],
        preview::willy(machine),
        machine.zx.mem[usize::from(at::FACING)] & 1,
    )
}

/// The jump preview's worker (#155): three jumps take longer than a frame,
/// so they are run beside the machine, on a copy handed over each pass
/// while Willy stands and the last is done. It stops when the machine's
/// thread lets go of its end of the channel.
fn preview_worker(
    shared: &Arc<Shared>,
) -> std::sync::mpsc::SyncSender<(Place, manicminer::Machine)> {
    let (send, receive) = std::sync::mpsc::sync_channel::<(Place, manicminer::Machine)>(0);
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        while let Ok((at, machine)) = receive.recv() {
            let jumps = preview::jumps(&machine);
            *shared.game.jumps.lock().unwrap() = Some((at, jumps));
        }
    });
    send
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

/// The panel's, the picker's and the pause notice's letters.
pub struct Painter(Fonts);

impl Default for Painter {
    fn default() -> Painter {
        Painter(Fonts::load())
    }
}

impl Screen for Guide {
    const PANEL_W: usize = PANEL_W;
    /// The pad's letters, the picker's version and the view's.
    type Stamp = (Layout, u64, u64);
    type Shown = (Layout, Picker, View);
    type Painter = Painter;

    fn stamp(&self) -> Self::Stamp {
        let layout = *self.layout.lock().unwrap();
        let picker = self.picker.lock().unwrap().version();
        (layout, picker, self.view.lock().unwrap().1)
    }

    fn shown_unless(&self, since: Option<Self::Stamp>) -> Option<(Self::Stamp, Self::Shown)> {
        let layout = *self.layout.lock().unwrap();
        let picker = self.picker.lock().unwrap();
        let view = self.view.lock().unwrap();
        let stamp = (layout, picker.version(), view.1);
        (since != Some(stamp)).then(|| (stamp, (layout, picker.clone(), view.0.clone())))
    }

    /// The panel at the level in force; over it the picker while it is
    /// open, and otherwise the pause notice while paused.
    fn draw(
        painter: &mut Painter,
        canvas: &mut Canvas,
        (layout, picker, view): &Self::Shown,
        paused: bool,
    ) {
        canvas.clear_transparent();
        panel::draw(&mut painter.0, canvas, view, picker.level());
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

/// Whether a game is being played after a frame that reached `hits`: from
/// the main loop's first pass outside the demo until the title screen.
fn playing_after(was: bool, hits: &[u16], mem: &[u8]) -> bool {
    if hits.contains(&routine::TITLE) {
        false
    } else {
        was || (hits.contains(&routine::MAIN_LOOP) && mem[usize::from(at::DEMO)] == 0)
    }
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
    let mut follow = Follow::default();
    // A game is played from its first pass of the main loop to the title.
    let mut playing = false;
    let preview_to = preview_worker(shared);
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
        // What the picker does to a game waits for one, as Starquake's
        // does. `playing` runs from a game's first pass to the title, through a
        // death or the air counted into the score; `in_game` is the part
        // with its main loop running, where the cheat's keys are read and
        // the switches steer. Neither is ever true in the demo (#153).
        let in_game = playing && since_loop < PLAY_FRAMES;
        match action {
            Some(Action::Exit) => return Ok(()),
            // Ended whenever a game is on, the bonus count too: CAPS SHIFT
            // and SPACE are read on its main loop's next pass.
            Some(Action::EndGame) if playing => {
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
        // Only a game freezes: the demo reads the pause keys too, and
        // pausing it left a paused demo where the player wanted a game.
        if freeze.poll(held, pause, in_game) {
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
        playing = playing_after(playing, &hits, &machine.zx.mem[..]);
        let mut view = follow.frame(
            &machine.zx.mem[..],
            since_loop == 0,
            since_loop < PLAY_FRAMES,
            playing,
        );
        // Level 4: a preview each pass while Willy is on the ground, when
        // the worker is free (a copy handed over only then). Standing
        // still, it is shown where it was worked out; walking, the latest
        // is shown until the next replaces it, up to a step behind him
        // (#156).
        let level = shared.game.picker.lock().unwrap().level();
        if level >= 4 && playing && preview::on_ground(&machine) {
            let here = place(&machine);
            if since_loop == 0 {
                let _busy = preview_to.try_send((here, machine.clone()));
            }
            let walking = preview::walking(&machine);
            if let Some((at, jumps)) = &*shared.game.jumps.lock().unwrap()
                && (*at == here || (walking && at.0 == here.0))
            {
                view.jumps.clone_from(jumps);
            }
        }
        {
            let mut shown = shared.game.view.lock().unwrap();
            if shown.0 != view {
                *shown = (view, shown.1 + 1);
            }
        }
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
/// title screen to start a game, and writes the window every 250 frames
/// into `dir`: the picture, and beside it the panel at guidance `level`.
///
/// # Errors
///
/// If the tape cannot be read or the folder cannot be written.
pub fn headless(path: &Path, frames: u64, dir: &Path, level: u8) -> Result<(), String> {
    use sidekick_frontend::overlay::{self, HEIGHT};
    use sidekick_frontend::video::{FULL_H, FULL_W, draw};
    let tape = tape::read(&GAME, path)?;
    let mut machine = manicminer::start(&tape)?;
    machine.watch = vec![routine::MAIN_LOOP, routine::TITLE];
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let enter = zx_spectrum::Key::by_name("enter").expect("a key");
    let mut picker = Picker::default();
    picker.set_level(level);
    let mut fonts = Fonts::load();
    let mut follow = Follow::default();
    let mut since_loop = PLAY_FRAMES;
    let mut playing = false;
    let mut rgba = vec![0u8; FULL_W * FULL_H * 4];
    let (w, h) = (overlay::width(PANEL_W) as usize, HEIGHT as usize);
    let mut over = vec![0u8; w * h * 4];
    for frame in 0..frames {
        machine.zx.set_key(enter, (100..300).contains(&frame));
        let hits = machine.run_frame();
        machine.zx.speaker.clear();
        let passed = hits.contains(&routine::MAIN_LOOP);
        since_loop = if passed {
            0
        } else {
            since_loop.saturating_add(1)
        };
        playing = playing_after(playing, &hits, &machine.zx.mem[..]);
        let mut view = follow.frame(
            &machine.zx.mem[..],
            passed,
            since_loop < PLAY_FRAMES,
            playing,
        );
        if frame % 250 == 249 {
            // Worked out here, for the picture taken: no worker headless.
            if level >= 4 && playing {
                view.jumps = preview::jumps(&machine);
            }
            draw(
                &machine.zx.mem[0x4000..0x5B00],
                machine.zx.border,
                frame,
                &mut rgba,
            );
            let mut canvas = Canvas {
                pixels: &mut over,
                width: w,
                height: h,
                scale: 1.0,
            };
            canvas.clear_transparent();
            panel::draw(&mut fonts, &mut canvas, &view, picker.level());
            let pixels = overlay::composite(&rgba, &over, w, h);
            let file = dir.join(format!("frame-{frame:05}.png"));
            std::fs::write(&file, zx_core::png::encode(&pixels, w, h))
                .map_err(|e| format!("{}: {e}", file.display()))?;
        }
    }
    Ok(())
}
