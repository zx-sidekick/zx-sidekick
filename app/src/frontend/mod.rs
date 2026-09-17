//! Window, input and sound around the emulated machine.

mod audio;
mod freeze;
mod gamepad;
mod guidance;
pub mod headless;
mod input;
mod notice;
mod overlay;
mod panel;
mod prompt;
mod scores;
pub mod tape;
mod text;
mod track;
mod video;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sidekick::machine::Training;
use sidekick::starquake::{
    ENTRY_PC, ENTRY_SP, end_game_hold, high_scores, routine, write_high_scores,
};
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
    /// The guidance level, training mode and the picker.
    pub guidance: Mutex<guidance::Guidance>,
    /// Which part of the program the game is in, for the panel.
    pub scene: Mutex<track::Scene>,
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
    /// Holds the machine between frames while the guidance picker is open,
    /// taking the gamepad's side of it: up and down choose a row, left and
    /// right change a setting, A does the highlighted thing, and B or Select
    /// goes back. No time passes for the game, so its pacing starts again
    /// from now. Returns no input for the frame it resumes on, so the button
    /// that closed the picker is not also a shot in the game.
    fn hold_for_picker(&mut self) -> gamepad::Pad {
        while self.shared.guidance.lock().unwrap().picker_open()
            && !self.shared.quit.load(Ordering::Relaxed)
        {
            std::thread::sleep(Duration::from_millis(20));
            let pad = self.pad.poll();
            let mut guidance = self.shared.guidance.lock().unwrap();
            guidance.set_pad(pad.layout);
            if pad.select || pad.cancel() {
                guidance.back();
            }
            if pad.up {
                guidance.focus_up();
            }
            if pad.down {
                guidance.focus_down();
            }
            if pad.left {
                guidance.change(false);
            }
            if pad.right {
                guidance.change(true);
            }
            if pad.confirm() {
                guidance.enter();
                if guidance.take(guidance::Action::Exit) {
                    self.shared.quit.store(true, Ordering::Relaxed);
                }
            }
        }
        self.next_frame = Instant::now();
        gamepad::Pad::default()
    }

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
            screen.3 = false;
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
        // Every room, for the map's openings and level 5's graph (#10): the same every game, so read
        // once, by having the game draw each room on a copy of the machine.
        // It takes about a third of a second, before the loading picture.
        let rooms = sidekick::starquake::all_rooms(&machine);
        // Which rooms hold a security door, for level 6's codes (#66): the
        // tape's own, from the rooms just read (#80).
        let doors = sidekick::starquake::door_rooms(&rooms);
        let graph = sidekick::map::Graph::new(&rooms, sidekick::starquake::CORE_ROOM);
        let openings = sidekick::map::openings(&rooms, sidekick::starquake::CORE_ROOM);
        self.shared.guidance.lock().unwrap().set_openings(openings);
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
                    self.shared.guidance.lock().unwrap().open();
                }
                if self.shared.guidance.lock().unwrap().picker_open() {
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
            .guidance
            .lock()
            .unwrap()
            .set_high_scores(keeper.kept, keeper.this_game);
        let mut tracker = track::Tracker::default();
        tracker.graph = graph;
        let mut freeze = freeze::Freeze::default();
        // Whether the game's pause key was pressed in the last frame.
        let mut pause = false;
        // Whether level 6's codes are waiting to be read, once the new game
        // they belong to is actually being played (#66).
        let mut reading_due = false;
        while !self.shared.quit.load(Ordering::Relaxed) {
            let mut pad = self.pad.poll();
            // The letters the legends show follow the pad (#101).
            self.shared.guidance.lock().unwrap().set_pad(pad.layout);
            if pad.north {
                let mut guidance = self.shared.guidance.lock().unwrap();
                if !guidance.picker_open() {
                    guidance.switch_piece();
                }
            }
            if pad.select {
                let mut guidance = self.shared.guidance.lock().unwrap();
                if !guidance.picker_open() {
                    guidance.open();
                }
            }
            if self.shared.guidance.lock().unwrap().picker_open() {
                pad = self.hold_for_picker();
            }
            // End this game holds the game's own keys for abandoning a game,
            // from the top of the play loop; the request lasts until the game
            // has left play.
            if self
                .shared
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
            if freeze.poll(held, pause, tracker.scene == track::Scene::Play) {
                pause = false;
                self.shared.screen.lock().unwrap().3 = true;
                std::thread::sleep(Duration::from_millis(20));
                self.next_frame = Instant::now();
                continue;
            }
            // Training mode holds things still only while a game is played;
            // anywhere else the machine writes nothing into the game (#8).
            machine.training = match tracker.scene {
                track::Scene::Play => self.shared.guidance.lock().unwrap().training(),
                _ => Training::default(),
            };
            machine.zx.keys = input.keys;
            machine.zx.kempston = 0;
            // The keyboard's joystick and the pad together; the machine
            // presses them as the game's chosen control method listens.
            machine.joystick = input.joystick | pad.bits;
            machine.start = pad.start;
            let hits = machine.run_frame();
            // Quit the game on the title screen, answered Y (#90): the game
            // has said goodbye to Olly for its own five seconds and is about
            // to wipe itself, which a Spectrum follows with a reset. Here the
            // program closes, the goodbye still on the screen.
            if hits.contains(&routine::QUIT) {
                return Ok(());
            }
            pause = machine.pause_pressed;
            {
                let mut guidance = self.shared.guidance.lock().unwrap();
                for &hit in &hits {
                    if let Some(scene) = tracker.follow(&machine.zx.mem[..], hit, &mut guidance) {
                        *self.shared.scene.lock().unwrap() = scene;
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
                // Level 6 tells every code, shown or not (#66). A new game
                // makes new door codes, so they are read again on a copy of
                // the machine, on a thread of its own: it takes about a
                // third of a second, and the game plays on meanwhile.
                if hits.contains(&routine::NEW_GAME) {
                    guidance.forget_all_codes();
                    // Not yet: the new game's seed, which its door codes are
                    // made from, is not written until play is under way, and
                    // a reading taken here gives the last game's codes.
                    reading_due = true;
                }
                if reading_due && tracker.scene == track::Scene::Play {
                    reading_due = false;
                    let reading = guidance.game();
                    let copy = machine.clone();
                    let doors = doors.clone();
                    let shared = Arc::clone(&self.shared);
                    std::thread::spawn(move || {
                        let teleporters = sidekick::starquake::all_teleporters(&copy.zx.mem[..]);
                        let codes: Vec<guidance::DoorCode> = doors
                            .iter()
                            .filter_map(|&(room, spot)| {
                                let chips = sidekick::starquake::read_door_code(&copy, room, spot)?;
                                Some(guidance::DoorCode {
                                    room,
                                    chips,
                                    graphics: chips
                                        .map(|g| sidekick::starquake::graphic(&copy.zx.mem[..], g)),
                                })
                            })
                            .collect();
                        let mut guidance = shared.guidance.lock().unwrap();
                        if guidance.game() == reading {
                            guidance.set_all_codes(&teleporters, &codes);
                        }
                    });
                }
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
        screen: Mutex::new((vec![0; zx_core::screen::BITMAP_LEN + 768], 0, 0, false)),
        input: Mutex::new(Input::default()),
        quit: AtomicBool::new(false),
        dead: AtomicBool::new(false),
        guidance: Mutex::new(guidance::Guidance::default()),
        scene: Mutex::new(track::Scene::Loading),
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
