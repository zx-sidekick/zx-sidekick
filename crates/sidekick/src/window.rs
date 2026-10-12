//! The window: the tape prompt when a game's tape is missing, then the Spectrum's picture at
//! the largest whole scale, its sound, and the keyboard.

use crate::{
    entry::{self, Entry, Stroke, Typing},
    games::{self, Game, Playing},
    keys::{spectrum_key, Spectrum},
    menu::{self, Menu, Outcome},
    pad::{self, Changes, Input, Maker, Mapping, Nav, Pads, Part, Play, Way},
    panel,
    picture::{layout, placement, to_rgba, Layout},
    saves::{self, Saves, Unsaved},
    scores,
};
use sdl3::{
    audio::{AudioFormat, AudioSpec, AudioStreamOwner},
    dialog::{show_open_file_dialog, DialogFileFilter},
    event::Event,
    gamepad::{Axis, Button, Gamepad, GamepadType},
    keyboard::{Mod, Scancode},
    mouse::MouseButton,
    pixels::{Color, PixelFormat},
    rect::Rect,
    render::{BlendMode, FRect, ScaleMode, Texture, TextureCreator, WindowCanvas},
    video::{FullscreenType, WindowContext},
    EventPump, GamepadSubsystem,
};
use sidekick_core::{Machine, Milestone, Session, SAMPLE_RATE};
use sidekick_def::Definition;
use sidekick_ui::{
    Canvas, EntryCard, Face, Fonts, LibraryPage, Menu as MenuCard, Note, NoticeHint, Offer,
    PadLabels, PadMark, Panel, PauseNotice, Pin, Question, Refusal, Row, TapePrompt, ThisGame,
    ENTRY_HEIGHT, ENTRY_WIDTH, MENU_WIDTH, NOTE_AT, NOTE_HEIGHT, OFFER_HEIGHT, OFFER_WIDTH,
    PAUSE_NOTICE_HEIGHT, PAUSE_NOTICE_WIDTH, QUESTION_WIDTH, TAPE_PROMPT_HEIGHT, TAPE_PROMPT_WIDTH,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// A frame of the 48K: 69,888 T-states at 3.5 MHz.
const FRAME: Duration = Duration::from_micros(19_968);

/// Frames of sound kept queued ahead: enough to ride out a slow frame, few enough that a key
/// press is heard at once.
const QUEUED_FRAMES: usize = 3;

/// The most frames run between two presented pictures, so a stall can't snowball.
const MOST_FRAMES_A_PASS: usize = 5;

/// How long a notice such as "Game saved" shows.
const NOTE_TIME: Duration = Duration::from_secs(2);

/// The menu card's width, for halving it.
#[allow(clippy::cast_precision_loss, reason = "a card's width")]
const MENU_WIDTH_F: f32 = MENU_WIDTH as f32;

/// What came of an event for the guidance menu.
enum MenuEvent {
    /// Not the menu's: the game has it.
    NotMine,
    /// The menu's.
    Taken,
    /// Exit, from the menu.
    Quit,
}

/// Draws a card into a texture kept with what it was drawn from, drawing it anew only when
/// that or the scale changed. Uploaded un-premultiplied, for blending over the picture.
fn cached<'a, T: Clone + PartialEq>(
    creator: &'a TextureCreator<WindowContext>,
    kept: &mut Option<(Texture<'a>, T, u32)>,
    fonts: &mut Fonts,
    what: &T,
    scale: u32,
    draw: impl FnOnce(&T, &mut Fonts, u32) -> Canvas,
) -> Result<(), String> {
    if kept
        .as_ref()
        .is_some_and(|(_, drawn, s)| *s == scale && drawn == what)
    {
        return Ok(());
    }
    let card = draw(what, fonts, scale);
    let (w, h) = card.size();
    let mut texture = new_texture(creator, w, h)?;
    texture
        .update(None::<Rect>, &card.straight_rgba(), w as usize * 4)
        .map_err(|e| e.to_string())?;
    texture.set_blend_mode(BlendMode::Blend);
    *kept = Some((texture, what.clone(), scale));
    Ok(())
}

/// The keyboard, as a pad, for the keys that do a game's actions: an id no pad SDL opens has.
const KEYBOARD: u32 = u32::MAX;

/// The picture's size: the screen with 32 pixels of border all round.
const PICTURE: (u32, u32) = (320, 256);

/// The tape prompt's size, in its logical pixels.
const PROMPT: (u32, u32) = (TAPE_PROMPT_WIDTH, TAPE_PROMPT_HEIGHT);

/// What the window opens on.
pub(crate) enum Start {
    /// A game to play.
    Play(Box<Playing>),
    /// A game whose tape has to be located first.
    Prompt(Box<Game>),
    /// Several games, to choose one.
    Library(Vec<Game>),
}

/// The library, choosing a game.
struct Library {
    games: Vec<Game>,
    page: LibraryPage,
    /// The scale the page was last drawn at; drawn again when it or the page changes.
    drawn: Option<u32>,
}

/// The tape prompt, waiting for a tape.
struct Prompt {
    game: Game,
    page: TapePrompt,
    /// The scale the page was last drawn at; drawn again when it or the page changes.
    drawn: Option<u32>,
    /// What the file dialog chose, or a file dropped on the window.
    chosen: Arc<Mutex<Option<PathBuf>>>,
}

/// A game being played, and what keeps its time and sound.
struct Player {
    session: Box<Session>,
    play: Play,
    title: String,
    /// Where the game's kept lists are saved; none for a tape played plainly.
    folder: Option<PathBuf>,
    /// The guidance level in use.
    level: usize,
    /// The guidance menu, while it's open, and where its card was last drawn: its top left
    /// in the window's device pixels, its scale, and device pixels to a window point.
    menu: Option<Menu>,
    menu_at: Option<(f32, f32, f32, f32)>,
    /// The text-entry card while it's open, and a word being typed.
    entry: Option<Entry>,
    typing: Option<Typing>,
    /// The high-score table as kept, and the panel's page after a game, until the next.
    table: Vec<scores::Kept>,
    this_game: Option<ThisGame>,
    /// What the game's saves are of; a notice and when it was said; and the save the file
    /// dialog chose.
    saves: Saves,
    note: Option<(Note, Instant)>,
    opened: Arc<Mutex<Option<PathBuf>>>,
    /// The menu asked for the file dialog.
    choosing: bool,
    /// The game kept on quitting last time, while it's offered.
    offer: Option<Vec<u8>>,
    started: Instant,
    frames_run: u64,
    rgba: Vec<u8>,
    samples: Vec<f32>,
}

enum Screen {
    Library(Box<Library>),
    Prompt(Box<Prompt>),
    Play(Box<Player>),
}

/// The pads plugged in, what each holds, and the maker of the one used last.
struct Gamepads {
    subsystem: Option<GamepadSubsystem>,
    /// Each pad open, by its id, and who made it. Closed when it's dropped.
    open: BTreeMap<u32, (Gamepad, Maker)>,
    pads: Pads,
    last: Maker,
}

/// What the window draws with.
struct Output<'a> {
    canvas: WindowCanvas,
    creator: &'a TextureCreator<WindowContext>,
    picture: Texture<'a>,
    prompt: Option<Texture<'a>>,
    /// The pause notice drawn, and what it was drawn from and at.
    notice: Option<(Texture<'a>, PauseNotice, u32)>,
    /// The panel, likewise, and the pins it puts over the picture.
    panel: Option<(Texture<'a>, Panel, u32)>,
    pins: Option<(Texture<'a>, Vec<Pin>, u32)>,
    /// The panel's page after a game, likewise.
    this_game: Option<(Texture<'a>, ThisGame, u32)>,
    /// The text-entry card, likewise.
    entry: Option<(Texture<'a>, EntryCard, u32)>,
    /// The notice, likewise.
    note: Option<(Texture<'a>, Note, u32)>,
    /// The offer to continue, likewise.
    offer: Option<(Texture<'a>, Offer, u32)>,
    /// The guidance menu and its question, likewise.
    menu: Option<(Texture<'a>, MenuCard, u32)>,
    question: Option<(Texture<'a>, Question, u32)>,
    sound: Option<AudioStreamOwner>,
    fonts: Fonts,
}

/// Runs the window until it's closed.
///
/// # Errors
/// If SDL can't open a window or draw into it, or the machine fails.
pub(crate) fn run(start: Start) -> Result<(), String> {
    sdl3::hint::set("SDL_RENDER_VSYNC", "1");
    let sdl = sdl3::init().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    let title = match &start {
        Start::Play(playing) => playing.title.clone(),
        Start::Prompt(game) => game.title(),
        Start::Library(_) => "Choose a game".to_owned(),
    };
    let window = video
        .window(&format!("ZX Sidekick — {title}"), 1368, 768)
        .high_pixel_density()
        .resizable()
        .fullscreen()
        .build()
        .map_err(|e| e.to_string())?;
    let canvas = window.into_canvas();
    let creator = canvas.texture_creator();
    let mut out = Output {
        picture: new_texture(&creator, PICTURE.0, PICTURE.1)?,
        canvas,
        creator: &creator,
        prompt: None,
        notice: None,
        panel: None,
        pins: None,
        this_game: None,
        entry: None,
        note: None,
        offer: None,
        menu: None,
        question: None,
        // No sound device (a server, a test) is no reason not to play: time paces the frames.
        sound: open_sound(&sdl),
        fonts: Fonts::new(),
    };
    let mut events = sdl.event_pump().map_err(|e| e.to_string())?;
    let mut screen = first_screen(start);
    let mut gamepads = Gamepads {
        // No gamepad support is no reason not to play by keyboard.
        subsystem: sdl.gamepad().ok(),
        open: BTreeMap::new(),
        pads: Pads::default(),
        last: Maker::default(),
    };

    // Whichever way out, a game under way is kept.
    let ran = run_loop(&mut events, &mut out, &mut screen, &mut gamepads);
    screen.quit();
    ran
}

/// Takes events and draws the screen until the program is to quit.
///
/// # Errors
/// If SDL can't draw, or the machine fails.
fn run_loop(
    events: &mut EventPump,
    out: &mut Output,
    screen: &mut Screen,
    gamepads: &mut Gamepads,
) -> Result<(), String> {
    loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::KeyDown {
                    scancode: Some(scancode),
                    keymod,
                    repeat,
                    ..
                } if is_fullscreen_toggle(scancode, keymod) => {
                    if !repeat {
                        toggle_fullscreen(&mut out.canvas)?;
                    }
                }
                event => {
                    let mut presses = gamepads.event(&event);
                    if let Screen::Play(player) = &mut *screen {
                        if player.offer_event(&event, &presses, &mut gamepads.pads)
                            || player.save_keys(&event, &mut gamepads.pads)
                            || player.held(&event, &presses, &mut gamepads.pads)
                        {
                            continue;
                        }
                        match player.menu_event(&event, &presses, &mut gamepads.pads) {
                            MenuEvent::NotMine => {}
                            MenuEvent::Taken => continue,
                            MenuEvent::Quit => return Ok(()),
                        }
                        if player.entry_event(&presses, &mut gamepads.pads) {
                            continue;
                        }
                        presses.extend(gamepads.host_key(&event, player.play.mapping()));
                    }
                    match screen.event(&event, &presses, &out.canvas) {
                        Choice::Stay => {}
                        Choice::Quit => return Ok(()),
                        Choice::Play(i) => {
                            if let Screen::Library(library) = &mut *screen {
                                let game = library.games.swap_remove(i);
                                *screen = open_game(game, &mut out.canvas)?;
                                gamepads.pads.hold_back();
                            }
                        }
                    }
                }
            }
        }
        screen.between_passes(&out.canvas, &mut gamepads.pads);
        gamepads.label(screen);
        screen.draw(out, gamepads)?;
    }
}

impl Screen {
    /// What comes between passes: a save the file dialog chose is loaded; a tape offered to
    /// the prompt starts its game.
    fn between_passes(&mut self, canvas: &WindowCanvas, pads: &mut Pads) {
        match self {
            Self::Play(player) => player.take_opened(pads, canvas),
            Self::Prompt(prompt) => {
                if let Some(playing) = prompt.take_offer() {
                    *self = Self::Play(Box::new(Player::new(playing)));
                    pads.hold_back();
                }
            }
            Self::Library(_) => {}
        }
    }

    /// The program is quitting: a game under way is kept for the next start.
    fn quit(&mut self) {
        if let Self::Play(player) = self {
            // Quitting at the offer keeps the offered game for next time.
            let kept = match &player.offer {
                Some(bytes) => saves::put_back_quit(&player.saves, bytes),
                None => saves::keep_on_quit(&mut player.session, &player.saves),
            };
            if let Err(Unsaved::Failed(why)) = kept {
                eprintln!("The game couldn't be kept on quitting: {why}");
            }
        }
    }

    /// Runs the frames due, and draws the screen.
    fn draw(&mut self, out: &mut Output, gamepads: &mut Gamepads) -> Result<(), String> {
        out.canvas.set_draw_color(Color::RGB(0, 0, 0));
        out.canvas.clear();
        match self {
            Self::Play(player) => {
                player.frame(out, &mut gamepads.pads)?;
                if player.menu.is_some() {
                    player.draw_menu(out, gamepads.last.labels())?;
                } else {
                    player.notice(out, gamepads.last.labels())?;
                    player.draw_entry(out, gamepads.last.labels())?;
                    player.draw_offer(out, gamepads.last.labels())?;
                }
                player.draw_note(out)?;
            }
            Self::Prompt(prompt) => {
                let page = &prompt.page;
                show(out, &mut prompt.drawn, |fonts, scale| {
                    page.draw(fonts, scale)
                })?;
            }
            Self::Library(library) => {
                let page = &library.page;
                show(out, &mut library.drawn, |fonts, scale| {
                    page.draw(fonts, scale)
                })?;
            }
        }
        out.canvas.present();
        Ok(())
    }

    /// Handles an event, and the pad presses it made.
    fn event(&mut self, event: &Event, presses: &[(Part, Maker)], canvas: &WindowCanvas) -> Choice {
        let navs = presses.iter().filter_map(|&(p, m)| pad::nav(p, m));
        match self {
            Self::Play(player) => {
                play_event(player.session.machine_mut(), event);
                for &(part, _) in presses {
                    if let Some(function) = player.play.pressed(part) {
                        // A failure turns the script off, which the panel shows.
                        let _ = player.session.script_action(&function);
                    }
                }
                Choice::Stay
            }
            Self::Prompt(prompt) => {
                if prompt.event(event, canvas) || prompt.navs(navs, canvas) {
                    Choice::Quit
                } else {
                    Choice::Stay
                }
            }
            Self::Library(library) => match library.event(event, canvas) {
                Choice::Stay => library.navs(navs),
                choice => choice,
            },
        }
    }
}

impl Gamepads {
    /// Takes in a pad event: a pad plugged in or out, a button, an axis. Returns the parts
    /// pressed that count, each with the maker of the pad it's on.
    fn event(&mut self, event: &Event) -> Vec<(Part, Maker)> {
        let (pad, parts) = match *event {
            Event::GamepadAdded { which, .. } => {
                if let Some(gamepad) = self.subsystem.as_ref().and_then(|s| s.open(which).ok()) {
                    let maker = maker(gamepad.r#type());
                    self.open.insert(which.raw(), (gamepad, maker));
                    self.last = maker;
                }
                return Vec::new();
            }
            Event::GamepadRemoved { which, .. } => {
                self.open.remove(&which.raw());
                self.pads.removed(which.raw());
                return Vec::new();
            }
            Event::GamepadButtonDown { which, button, .. } => (
                which.raw(),
                part(button)
                    .and_then(|p| self.pads.button(which.raw(), p, true))
                    .into_iter()
                    .collect(),
            ),
            Event::GamepadButtonUp { which, button, .. } => {
                if let Some(p) = part(button) {
                    self.pads.button(which.raw(), p, false);
                }
                return Vec::new();
            }
            Event::GamepadAxisMotion {
                which, axis, value, ..
            } => match pad_axis(axis) {
                Some(axis) => (which.raw(), self.pads.axis(which.raw(), axis, value)),
                None => return Vec::new(),
            },
            _ => return Vec::new(),
        };
        let maker = self.open.get(&pad).map_or(self.last, |&(_, m)| m);
        if !parts.is_empty() {
            self.last = maker;
        }
        parts.into_iter().map(|p| (p, maker)).collect()
    }

    /// A key the Spectrum doesn't have that does one of the game's actions, pressed or let go
    /// like a pad's button. Returns its part when pressed.
    fn host_key(&mut self, event: &Event, mapping: &Mapping) -> Vec<(Part, Maker)> {
        let (scancode, down) = match *event {
            Event::KeyDown {
                scancode: Some(scancode),
                repeat: false,
                ..
            } => (scancode, true),
            Event::KeyUp {
                scancode: Some(scancode),
                ..
            } => (scancode, false),
            _ => return Vec::new(),
        };
        mapping
            .host_key(scancode.name())
            .and_then(|part| self.pads.button(KEYBOARD, part, down))
            .map(|part| (part, self.last))
            .into_iter()
            .collect()
    }

    /// Shows the last pad's buttons on the pages: always on the library, on the tape prompt
    /// once a pad is plugged in.
    fn label(&self, screen: &mut Screen) {
        let labels = self.last.labels();
        match screen {
            Screen::Library(library) if library.page.pad != labels => {
                library.page.pad = labels;
                library.drawn = None;
            }
            Screen::Prompt(prompt) => {
                let pad = (!self.open.is_empty()).then_some(labels);
                if prompt.page.pad != pad {
                    prompt.page.pad = pad;
                    prompt.drawn = None;
                }
            }
            _ => {}
        }
    }
}

/// A pad's button, by its position.
const fn part(button: Button) -> Option<Part> {
    Some(match button {
        Button::DPadUp => Part::DPad(Way::Up),
        Button::DPadDown => Part::DPad(Way::Down),
        Button::DPadLeft => Part::DPad(Way::Left),
        Button::DPadRight => Part::DPad(Way::Right),
        Button::West => Part::West,
        Button::South => Part::South,
        Button::East => Part::East,
        Button::North => Part::North,
        Button::LeftShoulder => Part::L1,
        Button::RightShoulder => Part::R1,
        Button::Start => Part::Start,
        Button::Back => Part::Select,
        _ => return None,
    })
}

const fn pad_axis(axis: Axis) -> Option<pad::Axis> {
    Some(match axis {
        Axis::LeftX => pad::Axis::StickX,
        Axis::LeftY => pad::Axis::StickY,
        Axis::TriggerLeft => pad::Axis::L2,
        Axis::TriggerRight => pad::Axis::R2,
        _ => return None,
    })
}

const fn maker(kind: GamepadType) -> Maker {
    match kind {
        GamepadType::PS3 | GamepadType::PS4 | GamepadType::PS5 => Maker::PlayStation,
        GamepadType::NintendoSwitchPro
        | GamepadType::NintendoSwitchJoyconLeft
        | GamepadType::NintendoSwitchJoyconRight
        | GamepadType::NintendoSwitchJoyconPair => Maker::Nintendo,
        _ => Maker::Other,
    }
}

impl Player {
    fn new(playing: Playing) -> Self {
        let offer = saves::take_quit(&playing.saves);
        Self {
            session: Box::new(playing.session),
            play: Play::new(playing.mapping),
            title: playing.title,
            folder: playing.folder,
            level: 0,
            menu: None,
            menu_at: None,
            entry: None,
            typing: None,
            table: Vec::new(),
            this_game: None,
            saves: playing.saves,
            note: None,
            opened: Arc::new(Mutex::new(None)),
            choosing: false,
            offer,
            started: Instant::now(),
            frames_run: 0,
            rgba: Vec::with_capacity((PICTURE.0 * PICTURE.1 * 4) as usize),
            samples: Vec::with_capacity(SAMPLE_RATE / 50 * 2),
        }
    }

    /// Whether the game is held on its loading screen, taking the event. Any key or button
    /// lets it go, and doesn't reach the game; its time starts again from then.
    fn held(&mut self, event: &Event, presses: &[(Part, Maker)], pads: &mut Pads) -> bool {
        if !self.session.holding() {
            return false;
        }
        if is_press(event) || !presses.is_empty() {
            self.session.release();
            self.started = Instant::now();
            self.frames_run = 0;
            pads.hold_back();
        }
        true
    }

    /// Runs the frames that are due, queues their sound, and draws the latest picture.
    fn frame(&mut self, out: &mut Output, pads: &mut Pads) -> Result<(), String> {
        let due = out.sound.as_ref().map_or_else(
            || frames_due(self.started.elapsed(), self.frames_run),
            |stream| {
                let queued = usize::try_from(stream.queued_bytes().unwrap_or(0)).unwrap_or(0);
                QUEUED_FRAMES.saturating_sub(queued / (SAMPLE_RATE / 50 * 2 * size_of::<f32>()))
            },
        );
        if due == 0 {
            // Vsync normally paces the loop; without it (some drivers) don't spin.
            std::thread::sleep(Duration::from_millis(1));
        }
        for _ in 0..due.min(MOST_FRAMES_A_PASS) {
            if self.session.holding() || self.menu.is_some() || self.offer.is_some() {
                break;
            }
            pads.hold_back_parts(&self.play.enter(self.session.state()));
            self.type_word();
            // While the card is open, the pad drives only the card.
            let held: Vec<Part> = if self.entry.is_some() {
                Vec::new()
            } else {
                pads.held().collect()
            };
            let mut changes = self.play.update(held.into_iter());
            let patches = std::mem::take(&mut changes.patches);
            send(self.session.machine_mut(), changes);
            let applied: Vec<_> = patches
                .into_iter()
                .filter(|p| self.session.patch(p))
                .collect();
            let ran = self.session.run_frame();
            for patch in &applied {
                self.session.unpatch(patch);
            }
            ran.map_err(|e| e.to_string())?;
            self.play.frame_ran();
            // The card goes when the game leaves its state, press or no press.
            if self.entry.is_some() && self.text_entry().is_none() {
                self.entry = None;
                pads.hold_back();
            }
            self.save_kept();
            self.milestones();
            self.frames_run += 1;
            if let Some(stream) = &out.sound {
                self.samples.clear();
                self.samples.extend(
                    self.session
                        .machine()
                        .sound()
                        .iter()
                        .flat_map(|&s| <[f32; 2]>::from(s)),
                );
                stream
                    .put_data_f32(&self.samples)
                    .map_err(|e| e.to_string())?;
            }
        }
        to_rgba(self.session.machine().picture(), &mut self.rgba);
        out.picture
            .update(None::<Rect>, &self.rgba, PICTURE.0 as usize * 4)
            .map_err(|e| e.to_string())?;
        // Dimmed to 55% while the game is paused, under the notice.
        let dim = if self.paused() { 140 } else { 255 };
        out.picture.set_color_mod(dim, dim, dim);
        let output = out.canvas.output_size().map_err(|e| e.to_string())?;
        let layout = self.layout(output);
        out.canvas
            .copy(&out.picture, None, frect(layout.picture))
            .map_err(|e| e.to_string())?;
        self.draw_panel(out, layout)
    }

    /// Draws the panel beside the picture, and the pins it puts over the picture.
    fn draw_panel(&self, out: &mut Output, layout: Layout) -> Result<(), String> {
        if let (Some(at), Some(page)) = (layout.panel, &self.this_game) {
            cached(
                out.creator,
                &mut out.this_game,
                &mut out.fonts,
                page,
                layout.scale,
                ThisGame::draw,
            )?;
            if let Some((texture, _, _)) = &out.this_game {
                out.canvas
                    .copy(texture, None, frect(at))
                    .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        if let Some(at) = layout.panel {
            let chosen = self.entry.as_ref().and_then(Entry::chosen);
            let panel = panel::of(&self.session, self.level, chosen);
            if let Some(panel) = panel {
                cached(
                    out.creator,
                    &mut out.panel,
                    &mut out.fonts,
                    &panel,
                    layout.scale,
                    Panel::draw,
                )?;
                if let Some((texture, _, _)) = &mut out.panel {
                    // Dimmed under the menu, as the picture is; not under the pause notice.
                    let dim = if self.menu.is_some() { 140 } else { 255 };
                    texture.set_color_mod(dim, dim, dim);
                    out.canvas
                        .copy(texture, None, frect(at))
                        .map_err(|e| e.to_string())?;
                }
                let pins = panel::pins(&panel);
                if !pins.is_empty() {
                    cached(
                        out.creator,
                        &mut out.pins,
                        &mut out.fonts,
                        &pins,
                        layout.scale,
                        |p, f, s| sidekick_ui::pins(p, f, s),
                    )?;
                    if let Some((texture, _, _)) = &mut out.pins {
                        // Dimmed with the picture.
                        let dim = if self.paused() { 140 } else { 255 };
                        texture.set_color_mod(dim, dim, dim);
                        out.canvas
                            .copy(texture, None, frect(layout.picture))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Acts on the session's milestones: the kept high scores put in at the first title, and
    /// read back after each game.
    fn milestones(&mut self) {
        if self.session.state() == Some("play") {
            self.this_game = None;
        }
        for milestone in self.session.take_milestones() {
            match milestone {
                Milestone::FirstTitle => self.put_scores_in(),
                Milestone::GameEnded => self.game_ended(),
            }
        }
    }

    /// Puts the kept high scores into the game.
    fn put_scores_in(&mut self) {
        let kept = self.folder.as_deref().map(scores::load).unwrap_or_default();
        let entries: Vec<_> = kept.iter().map(|k| k.entry.clone()).collect();
        self.session.set_scores(&entries);
        // What the game has now: the kept entries, and any of its own they didn't cover
        // (fewer kept than it holds), which are the tape's.
        let now = self.session.scores().unwrap_or_default();
        self.table = scores::after_game(&kept, now, "from the tape", false).0;
    }

    /// F5 saves to the quick slot and F9 loads it, wherever the game is. Returns whether the
    /// event was one of them.
    fn save_keys(&mut self, event: &Event, pads: &mut Pads) -> bool {
        let Event::KeyDown {
            scancode: Some(scancode @ (Scancode::F5 | Scancode::F9)),
            repeat,
            ..
        } = *event
        else {
            return false;
        };
        if !repeat {
            if scancode == Scancode::F5 {
                self.quick_save();
            } else {
                self.quick_load(pads);
            }
        }
        true
    }

    /// Saves to the quick slot, and says how it went.
    fn quick_save(&mut self) {
        match saves::write(&mut self.session, &self.saves, &self.saves.quick) {
            Ok(()) => {
                if let Some(menu) = &mut self.menu {
                    menu.saved();
                }
                self.say("Game saved");
            }
            Err(unsaved) => {
                if let Unsaved::Failed(why) = &unsaved {
                    eprintln!("The game couldn't be saved: {why}");
                }
                self.say(unsaved.words());
            }
        }
    }

    /// Loads the quick slot.
    fn quick_load(&mut self, pads: &mut Pads) {
        let quick = self.saves.quick.clone();
        self.load(&quick, pads);
    }

    /// Loads a save, and says how it went. The game goes on from it with nothing held, and
    /// the menu, card and page after a game closed.
    fn load(&mut self, path: &Path, pads: &mut Pads) {
        let result = saves::read(path, &mut self.session, &self.saves);
        if self.after_load(result, pads) {
            self.say("Game loaded");
        }
    }

    /// Answers the offer to continue: `go_on` loads the game kept on quitting; either way the
    /// offer goes, and the game starts.
    fn answer_offer(&mut self, go_on: bool, pads: &mut Pads) {
        let Some(bytes) = self.offer.take() else {
            return;
        };
        if go_on {
            let result = saves::continue_from(&bytes, &mut self.session, &self.saves);
            self.after_load(result, pads);
        }
        self.started = Instant::now();
        self.frames_run = 0;
        pads.hold_back();
    }

    /// After a save was loaded, or refused: the game goes on from it with nothing held, and
    /// the menu, card and page after a game closed. A refusal is said. Returns whether it
    /// loaded.
    fn after_load(&mut self, result: Result<(), saves::Refused>, pads: &mut Pads) -> bool {
        if let Err(refused) = result {
            self.say(refused.words());
            return false;
        }
        self.level = usize::from(self.session.level());
        if let Some(typing) = self.typing.take() {
            for &key in typing.keys() {
                self.session.machine_mut().key(key, false);
            }
        }
        self.menu = None;
        self.entry = None;
        self.this_game = None;
        // The kept table, not the save's: a new entry is told against it after a game.
        if self.session.progress().seen_title {
            self.put_scores_in();
        }
        self.started = Instant::now();
        self.frames_run = 0;
        pads.hold_back();
        true
    }

    /// Takes an event while the offer to continue is open: Enter or the confirm button goes
    /// on with the kept game; any other key, button or click starts afresh. Returns whether
    /// it took the event.
    fn offer_event(&mut self, event: &Event, presses: &[(Part, Maker)], pads: &mut Pads) -> bool {
        if self.offer.is_none() {
            return false;
        }
        let key = match *event {
            Event::KeyDown {
                scancode: Some(scancode),
                repeat: false,
                ..
            } => key_answers_offer(scancode),
            Event::MouseButtonDown { .. } => Some(false),
            _ => None,
        };
        let pad = presses
            .first()
            .map(|&(part, maker)| pad::nav(part, maker) == Some(Nav::Confirm));
        if let Some(go_on) = key.or(pad) {
            self.answer_offer(go_on, pads);
        }
        true
    }

    /// Draws the offer to continue over the picture, centred on it.
    fn draw_offer(&self, out: &mut Output, labels: PadLabels) -> Result<(), String> {
        if self.offer.is_none() {
            out.offer = None;
            return Ok(());
        }
        let output = out.canvas.output_size().map_err(|e| e.to_string())?;
        let layout = self.layout(output);
        let (px, py, pw, ph) = layout.picture;
        let scale = layout.scale;
        let offer = Offer {
            title: self.title.clone(),
            pad: labels,
        };
        cached(
            out.creator,
            &mut out.offer,
            &mut out.fonts,
            &offer,
            scale,
            Offer::draw,
        )?;
        let (w, h) = (OFFER_WIDTH * scale, OFFER_HEIGHT * scale);
        let at = (px + (pw - w.min(pw)) / 2, py + (ph - h.min(ph)) / 2, w, h);
        if let Some((texture, _, _)) = &out.offer {
            out.canvas
                .copy(texture, None, frect(at))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Opens the file dialog to choose a save; it's loaded on the pass after it's chosen.
    fn choose_save(&self, canvas: &WindowCanvas) {
        let opened = Arc::clone(&self.opened);
        let filters = [DialogFileFilter {
            name: "Saves",
            pattern: "sav;szx;SAV;SZX",
        }];
        let folder = self.saves.quick.parent().map(Path::to_owned);
        // A dialog that won't open leaves the game as it was.
        let _ = show_open_file_dialog(
            &filters,
            folder.filter(|f| f.is_dir()),
            false,
            canvas.window(),
            Box::new(move |result, _| {
                if let (Ok(paths), Ok(mut opened)) = (result, opened.lock()) {
                    *opened = paths.into_iter().next();
                }
            }),
        );
    }

    /// Opens the file dialog when the menu asked for it, and loads the save it chose, once it
    /// has.
    fn take_opened(&mut self, pads: &mut Pads, canvas: &WindowCanvas) {
        if std::mem::take(&mut self.choosing) {
            self.choose_save(canvas);
        }
        let path = self.opened.lock().ok().and_then(|mut o| o.take());
        if let Some(path) = path {
            self.load(&path, pads);
        }
    }

    fn say(&mut self, words: &str) {
        self.note = Some((
            Note {
                text: words.to_owned(),
            },
            Instant::now(),
        ));
    }

    /// Draws the notice over the picture's foot for a while after it's said.
    fn draw_note(&mut self, out: &mut Output) -> Result<(), String> {
        if self
            .note
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() >= NOTE_TIME)
        {
            self.note = None;
        }
        let Some((note, _)) = &self.note else {
            out.note = None;
            return Ok(());
        };
        let output = out.canvas.output_size().map_err(|e| e.to_string())?;
        let layout = self.layout(output);
        let (px, py, pw, ph) = layout.picture;
        let scale = layout.scale;
        cached(
            out.creator,
            &mut out.note,
            &mut out.fonts,
            note,
            scale,
            Note::draw,
        )?;
        let (w, h) = (note.width(&mut out.fonts) * scale, NOTE_HEIGHT * scale);
        // Placed as on the mockup's picture, three logical pixels to a Spectrum one, whatever
        // the picture's own scale.
        let (lw, lh) = (PICTURE.0 * 3, PICTURE.1 * 3);
        let at = (
            (px + pw * NOTE_AT.0 / lw).saturating_sub(w / 2),
            py + ph * NOTE_AT.1 / lh,
            w,
            h,
        );
        if let Some((texture, _, _)) = &out.note {
            out.canvas
                .copy(texture, None, frect(at))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Reads the high scores back after a game, marks this game's entry, keeps the table
    /// (without training games), and shows the page after a game.
    fn game_ended(&mut self) {
        let Some(read) = self.session.scores() else {
            return;
        };
        let definition = self.session.definition();
        let record = self.session.record().clone();
        let level_name = definition
            .and_then(|d| d.guidance.as_ref())
            .and_then(|g| g.levels.get(usize::from(record.level)))
            .map(|l| l.name.clone());
        let title = definition
            .and_then(|d| d.high_scores.as_ref())
            .and_then(|t| t.title.clone())
            .unwrap_or_else(|| "High scores".to_owned());
        let mark = scores::mark(&record, level_name.as_deref());
        let training = !record.switches.is_empty();
        let (table, mine) = scores::after_game(&self.table, read, &mark, training);
        if let Some(folder) = &self.folder {
            if let Err(e) = scores::save(folder, &table) {
                eprintln!(
                    "{}: the high scores couldn't be kept: {e}",
                    folder.display()
                );
            }
        }
        self.this_game = Some(scores::this_game(
            &record,
            level_name.as_deref(),
            &title,
            &table,
            mine,
        ));
        self.table = table;
    }

    /// Saves the kept lists that changed, at once, so nothing is lost if the program stops.
    fn save_kept(&mut self) {
        let changed = self.session.take_kept_changes();
        let (Some(folder), Some(definition)) = (&self.folder, self.session.definition()) else {
            return;
        };
        if !changed.is_empty() {
            if let Err(e) = games::save_kept(folder, definition, self.session.kept(), &changed) {
                eprintln!(
                    "{}: the kept lists couldn't be saved: {e}",
                    folder.display()
                );
            }
        }
    }

    /// Where the picture and, when the game has guidance levels, the panel go.
    fn layout(&self, output: (u32, u32)) -> Layout {
        let panel = self
            .session
            .definition()
            .and_then(|d| d.guidance.as_ref())
            .is_some_and(|g| !g.levels.is_empty());
        layout(output, PICTURE, panel)
    }

    fn paused(&self) -> bool {
        self.menu.is_some() || self.offer.is_some() || self.session.state() == Some("paused")
    }

    /// Takes an event for the guidance menu: Esc or Select opens it in play; while it's open,
    /// every key, button and click is its. Keys let go still reach the game, so none sticks.
    fn menu_event(
        &mut self,
        event: &Event,
        presses: &[(Part, Maker)],
        pads: &mut Pads,
    ) -> MenuEvent {
        let Some(menu) = &mut self.menu else {
            let esc = matches!(
                event,
                Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    repeat: false,
                    ..
                }
            );
            if esc || presses.iter().any(|&(p, _)| p == Part::Select) {
                let can_load = self.saves.quick.exists();
                self.menu = Some(Menu::open(&self.session, self.level, &self.title, can_load));
                return MenuEvent::Taken;
            }
            return MenuEvent::NotMine;
        };
        if let Event::KeyUp { .. } = event {
            play_event(self.session.machine_mut(), event);
        }
        let mut inputs: Vec<menu::Input> = presses
            .iter()
            .filter_map(|&(part, maker)| {
                if part == Part::Select {
                    return Some(menu::Input::Back);
                }
                Some(match pad::nav(part, maker)? {
                    Nav::Up => menu::Input::Up,
                    Nav::Down => menu::Input::Down,
                    Nav::Left => menu::Input::Left,
                    Nav::Right => menu::Input::Right,
                    Nav::Confirm => menu::Input::Confirm,
                    Nav::Back => menu::Input::Back,
                })
            })
            .collect();
        match *event {
            Event::KeyDown {
                scancode: Some(scancode),
                ..
            } => inputs.extend(match scancode {
                Scancode::Up => Some(menu::Input::Up),
                Scancode::Down => Some(menu::Input::Down),
                Scancode::Left => Some(menu::Input::Left),
                Scancode::Right => Some(menu::Input::Right),
                Scancode::Return | Scancode::KpEnter => Some(menu::Input::Confirm),
                Scancode::Escape => Some(menu::Input::Back),
                _ => None,
            }),
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if let Some((left, top, scale, density)) = self.menu_at {
                    let card = menu.card(PadLabels::default());
                    // Clicks come in window points; the card was placed in device pixels.
                    let (lx, ly) = (
                        x.mul_add(density, -left) / scale,
                        y.mul_add(density, -top) / scale,
                    );
                    match card.row_at(lx, ly) {
                        // The level box's halves step it the way their arrows point.
                        Some(Row::Level) => inputs.extend([
                            menu::Input::Choose(Row::Level),
                            if lx < f32::midpoint(0.0, MENU_WIDTH_F) {
                                menu::Input::Left
                            } else {
                                menu::Input::Right
                            },
                        ]),
                        row => inputs.extend(row.map(menu::Input::Click)),
                    }
                }
            }
            _ => {}
        }
        let mut outcome = Outcome::Stay;
        for input in inputs {
            outcome = menu.input(input, &mut self.session);
            if outcome != Outcome::Stay {
                break;
            }
        }
        let level = menu.level();
        // The menu may have forgotten the kept lists; no frame runs while it's open, so save
        // now, before an Exit could leave the old ones on disk.
        self.save_kept();
        self.menu_outcome(outcome, level, pads)
    }

    /// Acts on what came of the menu's inputs; `level` is the guidance level it had chosen.
    fn menu_outcome(&mut self, outcome: Outcome, level: usize, pads: &mut Pads) -> MenuEvent {
        match outcome {
            Outcome::Stay => {}
            Outcome::Cancelled => self.close_menu(pads),
            Outcome::Applied => {
                self.level = level;
                self.close_menu(pads);
            }
            Outcome::EndGame => {
                self.close_menu(pads);
                if let Some(function) = self.play.act("end-game") {
                    let _ = self.session.script_action(&function);
                }
            }
            Outcome::Exit => return MenuEvent::Quit,
            // The menu stays open, its changes still waiting for Back to the game.
            Outcome::Save => self.quick_save(),
            Outcome::Load => {
                self.close_menu(pads);
                self.quick_load(pads);
            }
            Outcome::OpenSave => {
                self.close_menu(pads);
                self.choosing = true;
            }
        }
        MenuEvent::Taken
    }

    /// The definition's text entry, when the game is in its state.
    fn text_entry(&self) -> Option<sidekick_def::TextEntry> {
        let text_entry = self.session.definition()?.text_entry.as_ref()?;
        (self.session.state() == Some(text_entry.state.as_str())).then(|| text_entry.clone())
    }

    /// Takes pad presses for the text-entry card: the first one in its state opens it, and
    /// while it's open they're all its. Returns whether it took them.
    fn entry_event(&mut self, presses: &[(Part, Maker)], pads: &mut Pads) -> bool {
        let Some(text_entry) = self.text_entry() else {
            self.entry = None;
            return false;
        };
        if presses.is_empty() || self.typing.is_some() {
            return self.entry.is_some();
        }
        let Some(entry) = &mut self.entry else {
            self.entry = Some(Entry::open(&text_entry, self.session.kept(), self.level));
            pads.hold_back();
            return true;
        };
        for &(part, maker) in presses {
            let input = if part == Part::West {
                Some(entry::Input::Switch)
            } else {
                pad::nav(part, maker).map(|nav| match nav {
                    Nav::Up => entry::Input::Up,
                    Nav::Down => entry::Input::Down,
                    Nav::Left => entry::Input::Left,
                    Nav::Right => entry::Input::Right,
                    Nav::Confirm => entry::Input::Confirm,
                    Nav::Back => entry::Input::Back,
                })
            };
            if let Some(word) = input.and_then(|i| entry.input(i)) {
                self.typing = Some(Typing::new(&word, &text_entry));
                self.entry = None;
                pads.hold_back();
                break;
            }
        }
        true
    }

    /// Presses or lets go the next key of a word being typed, before a frame.
    fn type_word(&mut self) {
        let Some(typing) = &mut self.typing else {
            return;
        };
        match typing.next() {
            Some(Stroke::Press(key)) => self.session.machine_mut().key(key, true),
            Some(Stroke::Release(key)) => self.session.machine_mut().key(key, false),
            Some(Stroke::Wait) => {}
            None => self.typing = None,
        }
    }

    /// Draws the text-entry card over the picture, its top at the definition's character row.
    fn draw_entry(&self, out: &mut Output, labels: PadLabels) -> Result<(), String> {
        let Some(entry) = &self.entry else {
            out.entry = None;
            return Ok(());
        };
        let row = self.text_entry().and_then(|t| t.card_row).unwrap_or(20);
        let output = out.canvas.output_size().map_err(|e| e.to_string())?;
        let layout = self.layout(output);
        let (px, py, pw, _) = layout.picture;
        let scale = layout.scale;
        let card = entry.card(labels);
        cached(
            out.creator,
            &mut out.entry,
            &mut out.fonts,
            &card,
            scale,
            EntryCard::draw,
        )?;
        let (w, h) = (ENTRY_WIDTH * scale, ENTRY_HEIGHT * scale);
        // Three logical pixels to a Spectrum pixel, below the 32-pixel border.
        let at = (px + (pw - w) / 2, py + (32 + row * 8) * 3 * scale, w, h);
        if let Some((texture, _, _)) = &out.entry {
            out.canvas
                .copy(texture, None, frect(at))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Closes the menu: the game's time starts again from now, and what's held is held back.
    fn close_menu(&mut self, pads: &mut Pads) {
        self.menu = None;
        self.started = Instant::now();
        self.frames_run = 0;
        pads.hold_back();
    }

    /// Draws the guidance menu over the picture while it's open, centred on the window at the
    /// notice's scale, and the question over it while it's asked.
    fn draw_menu(&mut self, out: &mut Output, labels: PadLabels) -> Result<(), String> {
        let Some(menu) = &self.menu else {
            out.menu = None;
            return Ok(());
        };
        let output = out.canvas.output_size().map_err(|e| e.to_string())?;
        let scale = self.layout(output).scale;
        let card = menu.card(labels);
        let (w, h) = (MENU_WIDTH * scale, card.height() * scale);
        let at = (
            (output.0 - w.min(output.0)) / 2,
            (output.1 - h.min(output.1)) / 2,
        );
        cached(
            out.creator,
            &mut out.menu,
            &mut out.fonts,
            &card,
            scale,
            MenuCard::draw,
        )?;
        if let Some((texture, _, _)) = &out.menu {
            out.canvas
                .copy(texture, None, frect((at.0, at.1, w, h)))
                .map_err(|e| e.to_string())?;
        }
        let (window_w, _) = out.canvas.window().size();
        #[allow(
            clippy::cast_precision_loss,
            reason = "screen sizes are far below f32's limit"
        )]
        let place = (
            at.0 as f32,
            at.1 as f32,
            scale as f32,
            output.0 as f32 / window_w.max(1) as f32,
        );
        self.menu_at = Some(place);
        if let Some(question) = menu.question(labels) {
            let (qw, qh) = (QUESTION_WIDTH * scale, question.height() * scale);
            let q = (
                (output.0 - qw.min(output.0)) / 2,
                (output.1 - qh.min(output.1)) / 2,
            );
            cached(
                out.creator,
                &mut out.question,
                &mut out.fonts,
                &question,
                scale,
                Question::draw,
            )?;
            if let Some((texture, _, _)) = &out.question {
                out.canvas
                    .copy(texture, None, frect((q.0, q.1, qw, qh)))
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Draws the pause notice over the picture while the game is paused, centred on it, at the
    /// layout's scale (the notice is laid out on the picture at three logical pixels to the
    /// Spectrum's one).
    fn notice(&self, out: &mut Output, labels: PadLabels) -> Result<(), String> {
        if !self.paused() {
            return Ok(());
        }
        let output = out.canvas.output_size().map_err(|e| e.to_string())?;
        let layout = self.layout(output);
        let (px, py, pw, ph) = layout.picture;
        let scale = layout.scale;
        let notice = pause_notice(self.session.definition(), labels);
        if out
            .notice
            .as_ref()
            .is_none_or(|(_, n, s)| *s != scale || *n != notice)
        {
            let card = notice.draw(&mut out.fonts, scale);
            let (w, h) = card.size();
            let mut texture = new_texture(out.creator, w, h)?;
            texture
                .update(None::<Rect>, &card.straight_rgba(), w as usize * 4)
                .map_err(|e| e.to_string())?;
            texture.set_blend_mode(BlendMode::Blend);
            out.notice = Some((texture, notice, scale));
        }
        let (w, h) = (PAUSE_NOTICE_WIDTH * scale, PAUSE_NOTICE_HEIGHT * scale);
        let at = (px + (pw - w) / 2, py + (ph - h) / 2, w, h);
        if let Some((texture, _, _)) = &out.notice {
            out.canvas
                .copy(texture, None, frect(at))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// The pause notice a definition words, for a pad marked as `labels` says.
fn pause_notice(definition: Option<&Definition>, labels: PadLabels) -> PauseNotice {
    let notice = definition
        .into_iter()
        .flat_map(|d| &d.states)
        .find(|s| s.name == "paused")
        .and_then(|s| s.notice.as_ref());
    let Some(notice) = notice else {
        return PauseNotice {
            pad: labels,
            ..PauseNotice::default()
        };
    };
    PauseNotice {
        text: notice.text.clone(),
        hints: notice
            .hints
            .iter()
            .map(|hint| NoticeHint {
                keys: hint.keys.clone(),
                pad: hint.pad.as_deref().and_then(|part| match part {
                    "west" => Some(PadMark::Face(Face::West)),
                    "south" => Some(PadMark::Face(Face::South)),
                    "east" => Some(PadMark::Face(Face::East)),
                    "north" => Some(PadMark::Face(Face::North)),
                    "dpad" | "stick" => Some(PadMark::Directions),
                    _ => None,
                }),
                does: hint.does.clone(),
            })
            .collect(),
        pad: labels,
    }
}

impl Prompt {
    /// Handles an event. Returns true to quit.
    fn event(&self, event: &Event, canvas: &WindowCanvas) -> bool {
        match event {
            Event::KeyDown {
                scancode: Some(scancode),
                repeat: false,
                ..
            } => match scancode {
                Scancode::Escape => return true,
                Scancode::Return | Scancode::KpEnter => self.locate(canvas),
                Scancode::W => self.open_download(),
                _ => {}
            },
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if let Some((lx, ly)) = logical_point(canvas, *x, *y) {
                    if inside(self.page.locate_button(), lx, ly) {
                        self.locate(canvas);
                    } else if self
                        .page
                        .download_button()
                        .is_some_and(|b| inside(b, lx, ly))
                    {
                        self.open_download();
                    }
                }
            }
            Event::DropFile { filename, .. } => {
                if let Ok(mut chosen) = self.chosen.lock() {
                    *chosen = Some(PathBuf::from(filename));
                }
            }
            _ => {}
        }
        false
    }

    /// Pad presses: confirm locates the tape, back quits. Returns true to quit.
    fn navs(&self, navs: impl Iterator<Item = Nav>, canvas: &WindowCanvas) -> bool {
        for nav in navs {
            match nav {
                Nav::Confirm => self.locate(canvas),
                Nav::Back => return true,
                Nav::Up | Nav::Down | Nav::Left | Nav::Right => {}
            }
        }
        false
    }

    /// A file offered since the last pass, taken as the tape or refused on the page. A tape
    /// that's the game's but can't be played yet (a `.tzx`) is refused too, not fatal.
    fn take_offer(&mut self) -> Option<Playing> {
        let file = self.chosen.lock().ok().and_then(|mut c| c.take())?;
        let refusal = match self.game.offer(&file) {
            Ok(tape) => match self.game.playing(&tape) {
                Ok(playing) => return Some(playing),
                Err(e) => Refusal {
                    heading: "This tape can't be played yet".to_owned(),
                    detail: e,
                },
            },
            Err(refusal) => refusal,
        };
        self.page.refusal = Some(refusal);
        self.drawn = None;
        None
    }

    fn locate(&self, canvas: &WindowCanvas) {
        let chosen = Arc::clone(&self.chosen);
        let filters = [DialogFileFilter {
            name: "Spectrum tapes",
            pattern: "tap;tzx;zip;TAP;TZX;ZIP",
        }];
        // A dialog that won't open leaves the prompt as it was; dropping a file still works.
        let _ = show_open_file_dialog(
            &filters,
            None::<PathBuf>,
            false,
            canvas.window(),
            Box::new(move |result, _| {
                if let (Ok(paths), Ok(mut chosen)) = (result, chosen.lock()) {
                    *chosen = paths.into_iter().next();
                }
            }),
        );
    }

    fn open_download(&self) {
        if let Some(url) = self.game.download_url() {
            // No browser opening leaves the address on the page to type.
            let _ = sdl3::url::open_url(&url);
        }
    }
}

/// What the library's event means.
enum Choice {
    Stay,
    Quit,
    Play(usize),
}

impl Library {
    fn event(&mut self, event: &Event, canvas: &WindowCanvas) -> Choice {
        let count = self.games.len();
        match event {
            Event::KeyDown {
                scancode: Some(scancode),
                ..
            } => match scancode {
                Scancode::Escape => return Choice::Quit,
                Scancode::Return | Scancode::KpEnter => return Choice::Play(self.page.chosen),
                Scancode::Up if self.page.chosen > 0 => self.choose(self.page.chosen - 1),
                Scancode::Down if self.page.chosen + 1 < count => self.choose(self.page.chosen + 1),
                _ => {}
            },
            Event::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                x,
                y,
                ..
            } => {
                if let Some(game) =
                    logical_point(canvas, *x, *y).and_then(|(lx, ly)| self.page.game_at(lx, ly))
                {
                    return Choice::Play(game);
                }
            }
            _ => {}
        }
        Choice::Stay
    }

    /// Pad presses: up and down choose, confirm plays, back quits.
    fn navs(&mut self, navs: impl Iterator<Item = Nav>) -> Choice {
        for nav in navs {
            match nav {
                Nav::Up if self.page.chosen > 0 => self.choose(self.page.chosen - 1),
                Nav::Down if self.page.chosen + 1 < self.games.len() => {
                    self.choose(self.page.chosen + 1);
                }
                Nav::Confirm => return Choice::Play(self.page.chosen),
                Nav::Back => return Choice::Quit,
                Nav::Up | Nav::Down | Nav::Left | Nav::Right => {}
            }
        }
        Choice::Stay
    }

    const fn choose(&mut self, game: usize) {
        self.page.chosen = game;
        self.drawn = None;
    }
}

fn first_screen(start: Start) -> Screen {
    match start {
        Start::Play(playing) => Screen::Play(Box::new(Player::new(*playing))),
        Start::Prompt(game) => prompt_screen(*game),
        Start::Library(games) => Screen::Library(Box::new(Library {
            page: LibraryPage {
                games: games.iter().map(Game::entry).collect(),
                chosen: 0,
                pad: PadLabels::default(),
            },
            games,
            drawn: None,
        })),
    }
}

/// A game chosen from the library, opened from its kept tape or on its tape prompt, the
/// window titled for it.
fn open_game(game: Game, canvas: &mut WindowCanvas) -> Result<Screen, String> {
    canvas
        .window_mut()
        .set_title(&format!("ZX Sidekick — {}", game.title()))
        .map_err(|e| e.to_string())?;
    match game.kept_tape() {
        Some(tape) => Ok(Screen::Play(Box::new(Player::new(game.playing(&tape)?)))),
        None => Ok(prompt_screen(game)),
    }
}

fn prompt_screen(game: Game) -> Screen {
    Screen::Prompt(Box::new(Prompt {
        page: game.prompt(),
        game,
        drawn: None,
        chosen: Arc::new(Mutex::new(None)),
    }))
}

/// Shows a page (the prompt, the library) at the largest whole scale that fits, drawing it
/// anew when the scale changed or the page did (`drawn` cleared).
fn show(
    out: &mut Output,
    drawn: &mut Option<u32>,
    draw: impl FnOnce(&mut Fonts, u32) -> Canvas,
) -> Result<(), String> {
    let output = out.canvas.output_size().map_err(|e| e.to_string())?;
    let (x, y, w, h) = placement(output, PROMPT);
    let scale = w / PROMPT.0;
    if *drawn != Some(scale) {
        let page = draw(&mut out.fonts, scale);
        let (pw, ph) = page.size();
        let mut texture = new_texture(out.creator, pw, ph)?;
        texture
            .update(None::<Rect>, page.rgba(), pw as usize * 4)
            .map_err(|e| e.to_string())?;
        out.prompt = Some(texture);
        *drawn = Some(scale);
    }
    if let Some(texture) = &out.prompt {
        out.canvas
            .copy(texture, None, frect((x, y, w, h)))
            .map_err(|e| e.to_string())?;
    }
    // Nothing moves on a page; don't spin.
    std::thread::sleep(Duration::from_millis(10));
    Ok(())
}

fn new_texture(
    creator: &TextureCreator<WindowContext>,
    w: u32,
    h: u32,
) -> Result<Texture<'_>, String> {
    let mut texture = creator
        .create_texture_streaming(PixelFormat::RGBA32, w, h)
        .map_err(|e| e.to_string())?;
    texture.set_scale_mode(ScaleMode::Nearest);
    Ok(texture)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "screen sizes are far below f32's limit"
)]
fn frect((x, y, w, h): (u32, u32, u32, u32)) -> FRect {
    FRect::new(x as f32, y as f32, w as f32, h as f32)
}

fn play_event(machine: &mut Machine, event: &Event) {
    match *event {
        Event::KeyDown {
            scancode: Some(scancode),
            repeat: false,
            ..
        } => press(machine, scancode, true),
        Event::KeyUp {
            scancode: Some(scancode),
            ..
        } => press(machine, scancode, false),
        _ => {}
    }
}

/// A point in window coordinates, as one in the prompt's logical pixels.
#[allow(
    clippy::cast_precision_loss,
    reason = "screen sizes are far below f32's limit"
)]
#[allow(
    clippy::suboptimal_flops,
    reason = "plain arithmetic, the same on every platform"
)]
fn logical_point(canvas: &WindowCanvas, x: f32, y: f32) -> Option<(f32, f32)> {
    let output = canvas.output_size().ok()?;
    let (window_w, _) = canvas.window().size();
    let density = output.0 as f32 / window_w.max(1) as f32;
    let (ox, oy, w, _) = placement(output, PROMPT);
    let scale = (w / PROMPT.0) as f32;
    Some((
        (x * density - ox as f32) / scale,
        (y * density - oy as f32) / scale,
    ))
}

fn inside((x, y, w, h): (f32, f32, f32, f32), px: f32, py: f32) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// A key pressed (not repeating), or a mouse button.
const fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::KeyDown { repeat: false, .. } | Event::MouseButtonDown { .. }
    )
}

/// What a key says to the offer to continue: Enter goes on, other keys start afresh, and a
/// modifier says nothing, since it may begin Cmd-Q, Alt-F4 or the fullscreen keys.
const fn key_answers_offer(scancode: Scancode) -> Option<bool> {
    match scancode {
        Scancode::LCtrl
        | Scancode::RCtrl
        | Scancode::LShift
        | Scancode::RShift
        | Scancode::LAlt
        | Scancode::RAlt
        | Scancode::LGui
        | Scancode::RGui
        | Scancode::CapsLock
        | Scancode::Mode => None,
        Scancode::Return | Scancode::KpEnter => Some(true),
        _ => Some(false),
    }
}

/// Switches the window between fullscreen and windowed.
fn toggle_fullscreen(canvas: &mut WindowCanvas) -> Result<(), String> {
    let window = canvas.window_mut();
    let fullscreen = window.fullscreen_state() != FullscreenType::Off;
    window
        .set_fullscreen(!fullscreen)
        .map_err(|e| e.to_string())
}

/// F11 everywhere; on a Mac also Control-Command-F, since F11 never reaches a program there.
fn is_fullscreen_toggle(scancode: Scancode, keymod: Mod) -> bool {
    let control_command = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
        && keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
    scancode == Scancode::F11 || (scancode == Scancode::F && control_command)
}

/// Presses and lets go on the Spectrum what a pad changed.
fn send(machine: &mut Machine, changes: Changes) {
    let inputs = changes
        .release
        .into_iter()
        .map(|i| (i, false))
        .chain(changes.press.into_iter().map(|i| (i, true)));
    for (input, pressed) in inputs {
        match input {
            Input::Kempston(direction) => machine.kempston(direction.kempston(), pressed),
            Input::Key(key) => machine.key(key.0, pressed),
        }
    }
}

fn press(machine: &mut Machine, scancode: Scancode, pressed: bool) {
    match spectrum_key(scancode) {
        Some(Spectrum::Key(key)) => machine.key(key, pressed),
        Some(Spectrum::Compound(key)) => machine.compound_key(key, pressed),
        None => {}
    }
}

fn open_sound(sdl: &sdl3::Sdl) -> Option<AudioStreamOwner> {
    let audio = sdl.audio().ok()?;
    let spec = AudioSpec {
        freq: Some(i32::try_from(SAMPLE_RATE).ok()?),
        channels: Some(2),
        format: Some(AudioFormat::f32_sys()),
    };
    let stream = audio
        .default_playback_device()
        .open_device_stream(Some(&spec))
        .ok()?;
    stream.resume().ok()?;
    Some(stream)
}

/// Frames that should have run by now at the 48K's rate, minus those that have.
fn frames_due(elapsed: Duration, run: u64) -> usize {
    let should = elapsed.as_nanos() / FRAME.as_nanos();
    usize::try_from(should.saturating_sub(u128::from(run))).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offer_goes_on_with_enter_and_ignores_modifiers() {
        assert_eq!(key_answers_offer(Scancode::Return), Some(true));
        assert_eq!(key_answers_offer(Scancode::KpEnter), Some(true));
        assert_eq!(key_answers_offer(Scancode::Escape), Some(false));
        assert_eq!(key_answers_offer(Scancode::F9), Some(false));
        for modifier in [
            Scancode::LGui,
            Scancode::RGui,
            Scancode::LAlt,
            Scancode::LCtrl,
        ] {
            assert_eq!(
                key_answers_offer(modifier),
                None,
                "{modifier:?} may begin a way out"
            );
        }
    }
}
