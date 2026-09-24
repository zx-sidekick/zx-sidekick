//! Starquake in the window every game shares (`sidekick-frontend`): its
//! guidance panel beside the picture, the picker over it, and the machine's
//! loop, which follows the game for the panel and keeps its high scores.

mod guidance;
pub mod headless;
mod panel;
mod scores;
mod track;

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sidekick_frontend::video::{Key, Screen};
use sidekick_frontend::{Game, Pacer, freeze, gamepad, overlay, picker, tape, text::Canvas};
use starquake::Machine;
use starquake::facts::{
    ENTRY_PC, ENTRY_SP, end_game_hold, high_scores, routine, write_high_scores,
};
use starquake::play::Training;
use winit::keyboard::KeyCode;

/// Starquake, as the shared frontend finds its tape and names it.
pub static GAME: Game = Game {
    name: "Starquake",
    release: "the original Bubble Bus release",
    program: "zx-sidekick-starquake",
    title: "ZX Sidekick · Starquake",
    // This project's name, and the 8.3 name World of Spectrum's zip holds
    // (`STARQUAK.TAP`), each bare and zipped.
    names: &[
        "starquake.tap",
        "starquak.tap",
        "starquake.tap.zip",
        "starquak.tap.zip",
    ],
    kept: "starquake.tap",
    accept: starquake::facts::is_supported_tape,
    sha1: starquake::facts::TAPE_SHA1,
    heading: "STARQUAKE",
    about: "Nothing from the original game is included. The graphics, maps and sound are read \
            from your own Starquake tape each time the game starts.",
    zip: "Starquake.tap.zip",
    page: "https://worldofspectrum.net/item/0004873/",
    page_shown: "worldofspectrum.net/item/0004873",
    credit: "Starquake \u{a9} 1985 Stephen Crow / Bubble Bus Software. Not affiliated.",
};

/// The pad's bottom face button is down, which lays a platform under Blob,
/// the move a player makes most, and the left one fires, as platformers lay
/// them out (#22).
const BUTTONS: gamepad::Buttons = gamepad::Buttons {
    south: sidekick::machine::JOY_DOWN,
    west: sidekick::machine::JOY_FIRE,
};

/// The guidance panel's width at the Spectrum's scale, beside the picture.
pub const PANEL_W: usize = 136;
/// The width the overlay is laid out in, the panel beside the picture.
pub const OVERLAY_W: f32 = overlay::width(PANEL_W);

/// How long the tape's loading picture stays up before the game starts, as it
/// would at the end of loading from a cassette.
const LOADING_FRAMES: u32 = 150;

/// Starquake's part of the state the machine's thread and the window share.
pub struct Guide {
    /// The guidance level, training mode and the picker.
    pub guidance: Mutex<guidance::Guidance>,
    /// Which part of the program the game is in, for the panel.
    pub scene: Mutex<track::Scene>,
}

impl Default for Guide {
    fn default() -> Guide {
        Guide {
            guidance: Mutex::new(guidance::Guidance::default()),
            scene: Mutex::new(track::Scene::Loading),
        }
    }
}

/// The state shared between the machine's thread and the window.
pub type Shared = sidekick_frontend::Shared<Guide>;

impl Default for panel::Panel {
    fn default() -> panel::Panel {
        panel::Panel::new()
    }
}

impl Screen for Guide {
    const PANEL_W: usize = PANEL_W;
    /// The guidance's version and the scene: the panel is drawn from both.
    type Stamp = (u64, track::Scene);
    type Shown = (guidance::Guidance, track::Scene);
    type Painter = panel::Panel;

    fn stamp(&self) -> Self::Stamp {
        let version = self.guidance.lock().unwrap().version();
        (version, *self.scene.lock().unwrap())
    }

    fn shown_unless(&self, since: Option<Self::Stamp>) -> Option<(Self::Stamp, Self::Shown)> {
        let scene = *self.scene.lock().unwrap();
        let guidance = self.guidance.lock().unwrap();
        let stamp = (guidance.version(), scene);
        (since != Some(stamp)).then(|| (stamp, (guidance.clone(), scene)))
    }

    fn draw(panel: &mut panel::Panel, canvas: &mut Canvas, shown: &Self::Shown, paused: bool) {
        panel.draw(canvas, &shown.0, shown.1, paused);
    }

    /// The keys the guidance panel takes. Esc opens the picker and goes back,
    /// and Tab switches the piece route; while it is open the arrows and
    /// Enter work it, and it has the keyboard to itself, so nothing typed
    /// into it reaches the game.
    fn key(&self, code: KeyCode) -> Key {
        let mut guidance = self.guidance.lock().unwrap();
        let open = guidance.picker_open();
        let mut quit = false;
        match code {
            KeyCode::Escape if !open => guidance.open(),
            // Tab, no key of the Spectrum's, switches the piece route (#51).
            KeyCode::Tab if !open => guidance.switch_piece(),
            _ if !open => return Key::Pass,
            _ => {
                picker::key(&mut *guidance, code);
                quit = guidance.take(guidance::Action::Exit);
            }
        }
        // Opening it: whatever was held is let go, as the game will not see
        // the key-ups.
        let release = guidance.picker_open() && !open;
        Key::Taken { release, quit }
    }
}

/// The machine's thread: runs a frame, plays its sound, shows its screen,
/// and waits for the next one.
struct Runner {
    shared: Arc<Shared>,
    pacer: Pacer,
    pad: gamepad::Gamepad,
}

impl Runner {
    /// Holds the machine between frames while the guidance picker is open,
    /// taking the gamepad's side of it: up and down choose a row, left and
    /// right change a setting, A does the highlighted thing, and B or Select
    /// goes back. No time passes for the game, so its pacing starts again
    /// from now. The button that closed the picker is kept from the
    /// game until it is let go, so it is not also a shot or a platform.
    fn hold_for_picker(&mut self) -> gamepad::Pad {
        while self.shared.game.guidance.lock().unwrap().picker_open()
            && !self.shared.quit.load(Ordering::Relaxed)
        {
            std::thread::sleep(Duration::from_millis(20));
            let pad = self.pad.poll();
            let mut guidance = self.shared.game.guidance.lock().unwrap();
            guidance.set_pad(pad.layout);
            picker::pad(&mut *guidance, &pad);
            if guidance.take(guidance::Action::Exit) {
                self.shared.quit.store(true, Ordering::Relaxed);
            }
        }
        self.pacer.restart();
        // The button that closed it, still held, is kept from the game.
        self.pad.hold_back_held();
        gamepad::Pad::default()
    }

    /// Shows `memory` for a frame, plays `edges` over it, and waits until it
    /// is time for the next.
    fn present(&mut self, memory: &[u8], border: u8, edges: &[(u32, bool)]) {
        self.pacer.present(&self.shared, memory, border, edges);
    }

    fn run(&mut self, tape: &[u8]) -> Result<(), String> {
        let mut machine = Machine::from_tape(tape, ENTRY_PC, ENTRY_SP)?;
        // Every room, for the map's openings and level 5's graph (#10): the same every game, so read
        // once, by having the game draw each room on a copy of the machine.
        // It takes about a third of a second, before the loading picture.
        let rooms = starquake::facts::all_rooms(&machine);
        let graph = starquake::map::Graph::new(&rooms, starquake::facts::CORE_ROOM);
        let openings = starquake::map::openings(&rooms, starquake::facts::CORE_ROOM);
        self.shared
            .game
            .guidance
            .lock()
            .unwrap()
            .set_openings(openings);
        let loading = zx_core::tape::load_tap(tape)?.loading_screen;
        if let Some(picture) = loading {
            let mut memory = vec![0u8; 0x1B00];
            memory[..picture.len().min(0x1B00)]
                .copy_from_slice(&picture[..picture.len().min(0x1B00)]);
            for _ in 0..LOADING_FRAMES {
                if self.shared.quit.load(Ordering::Relaxed) {
                    return Ok(());
                }
                if self.pad.poll().select {
                    self.shared.game.guidance.lock().unwrap().open();
                }
                if self.shared.game.guidance.lock().unwrap().picker_open() {
                    self.hold_for_picker();
                }
                self.present(&memory, 0, &[]);
            }
        }
        machine.watch = track::WATCH.to_vec();
        // The high scores kept between runs (#47), put into the game's
        // memory before its first frame. A file that cannot be read is
        // left alone.
        let scores_file = scores::path();
        let text = scores_file
            .as_ref()
            .and_then(|p| match std::fs::read_to_string(p) {
                Ok(text) => Some(text),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(_) => Some(String::new()),
            });
        let shipped = high_scores(&machine.zx.mem[..]).ok_or("no room for the high scores")?;
        let mut keeper = scores::Keeper::new(text.as_deref(), shipped);
        write_high_scores(&mut machine.zx.mem[..], &keeper.kept.entries);
        self.shared
            .game
            .guidance
            .lock()
            .unwrap()
            .set_high_scores(keeper.kept, keeper.this_game);
        let mut tracker = track::Tracker::default();
        tracker.graph = graph;
        // Which rooms hold a security door, for level 6's codes (#66): the
        // tape's own, from the rooms read above (#80).
        tracker.door_rooms = starquake::facts::door_rooms(&rooms);
        let spots = starquake::facts::door_spots(&rooms);
        self.shared
            .game
            .guidance
            .lock()
            .unwrap()
            .set_door_spots(spots);
        let mut freeze = freeze::Freeze::default();
        // Whether the game's pause key was pressed in the last frame.
        let mut pause = false;
        // Whether level 6's codes are waiting to be read, once the new game
        // they belong to is actually being played (#66).
        while !self.shared.quit.load(Ordering::Relaxed) {
            let mut pad = self.pad.poll();
            // The letters the legends show follow the pad (#101).
            self.shared
                .game
                .guidance
                .lock()
                .unwrap()
                .set_pad(pad.layout);
            if pad.north {
                let mut guidance = self.shared.game.guidance.lock().unwrap();
                if !guidance.picker_open() {
                    guidance.switch_piece();
                }
            }
            if pad.select {
                let mut guidance = self.shared.game.guidance.lock().unwrap();
                if !guidance.picker_open() {
                    guidance.open();
                }
            }
            if self.shared.game.guidance.lock().unwrap().picker_open() {
                pad = self.hold_for_picker();
            }
            // End this game holds the game's own keys for abandoning a game,
            // from the top of the play loop; the request lasts until the game
            // has left play.
            if self
                .shared
                .game
                .guidance
                .lock()
                .unwrap()
                .take(guidance::Action::EndGame)
                && tracker.scene == track::Scene::Play
            {
                machine.hold = Some(end_game_hold());
                freeze.thaw();
            }
            let input = *self.shared.input.lock().unwrap();
            // Paused: no frame runs until a key, a direction, fire or Start,
            // and the window shows the notice meanwhile.
            let held = freeze::Held {
                start: pad.start,
                keys: input.keys,
                joystick: input.joystick | pad.bits,
            };
            // The window switched away from pauses a game in play, as the
            // pause key does.
            if self.shared.unfocused.swap(false, Ordering::Relaxed) {
                pause = true;
            }
            if freeze.poll(held, pause, tracker.scene == track::Scene::Play) {
                pause = false;
                self.shared.screen.lock().unwrap().3 = true;
                std::thread::sleep(Duration::from_millis(20));
                self.pacer.restart();
                continue;
            }
            // Training mode holds things still only while a game is played;
            // anywhere else the machine writes nothing into the game (#8).
            machine.rules.training = match tracker.scene {
                track::Scene::Play => self.shared.game.guidance.lock().unwrap().training(),
                _ => Training::default(),
            };
            machine.zx.keys = input.keys;
            machine.zx.kempston = 0;
            // The keyboard's joystick and the pad together; the machine
            // presses them as the game's chosen control method listens.
            machine.rules.joystick = input.joystick | pad.bits;
            machine.rules.start = pad.start;
            let hits = machine.run_frame();
            // Quit the game on the title screen, answered Y (#90): the game
            // has said goodbye to Olly for its own five seconds and is about
            // to wipe itself, which a Spectrum follows with a reset. Here the
            // program closes, the goodbye still on the screen.
            if hits.contains(&routine::QUIT) {
                return Ok(());
            }
            pause = machine.rules.pause_pressed;
            {
                let mut guidance = self.shared.game.guidance.lock().unwrap();
                for &hit in &hits {
                    if let Some(scene) = tracker.follow(&machine.zx.mem[..], hit, &mut guidance) {
                        *self.shared.game.scene.lock().unwrap() = scene;
                    }
                }
                tracker.publish(&machine.zx.mem[..], &mut guidance);
                // The CORE OF HEROES screen after a game over shows the table
                // final: kept, with this game's guidance (#47).
                if hits.contains(&routine::HEROES)
                    && tracker.scene == track::Scene::GameOver
                    && let Some(table) = high_scores(&machine.zx.mem[..])
                {
                    if let Some(kept) = keeper.heroes(&table, guidance.record())
                        && let Some(path) = &scores_file
                        && let Err(e) = scores::save(path, &kept)
                    {
                        eprintln!("{e}");
                    }
                    guidance.set_high_scores(keeper.kept, keeper.this_game);
                }
                tracker.read_codes(&machine, &mut guidance);
                // After a game with training, its table is put back.
                if hits.contains(&routine::MENU)
                    && let Some(table) = keeper.menu()
                {
                    write_high_scores(&mut machine.zx.mem[..], &table);
                    guidance.set_high_scores(keeper.kept, keeper.this_game);
                }
            }
            if tracker.scene != track::Scene::Play {
                machine.hold = None;
            }
            let edges = std::mem::take(&mut machine.zx.speaker);
            let border = machine.zx.border;
            self.present(&machine.zx.mem[0x4000..0x5B00], border, &edges);
        }
        Ok(())
    }
}

/// Starquake's loop on the machine's thread.
fn play(tape: &[u8], shared: &Arc<Shared>, pacer: Pacer) -> Result<(), String> {
    let mut runner = Runner {
        shared: shared.clone(),
        pacer,
        pad: gamepad::Gamepad::new(BUTTONS),
    };
    runner.run(tape)
}

/// Runs the game in a window: from `path`, or, with none, from whatever the
/// player locates on the screen that asks for the tape.
pub fn run(path: Option<&Path>) -> Result<(), String> {
    let tape = path.map(|path| tape::read(&GAME, path)).transpose()?;
    sidekick_frontend::run(&GAME, Shared::new(Guide::default()), tape, play)
}
