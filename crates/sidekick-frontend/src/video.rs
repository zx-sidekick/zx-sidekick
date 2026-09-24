//! The window: draws the latest frame, with its border, scaled by the GPU,
//! and beside it the game's panel, laid over at the window's own resolution
//! with whatever else the game shows over the picture ([`Screen`]).

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Fullscreen, Theme, Window, WindowId};

use crate::overlay::{self, Overlay};
use crate::prompt::{self, Outcome, Prompt};
use crate::text::Canvas;
use crate::{Game, Shared};
use sidekick::Input;
use zx_core::screen::{BITMAP_LEN, HEIGHT, WIDTH};

const BORDER: usize = 32;
pub const FULL_W: usize = WIDTH + 2 * BORDER;
pub const FULL_H: usize = HEIGHT + 2 * BORDER;
const SCALE: f64 = 3.0;

/// The window's width at the Spectrum's scale: the picture with its border,
/// and the game's panel beside it, `panel_w` wide. The window is a whole
/// multiple of both, so the picture is scaled exactly as it would be alone.
#[must_use]
pub const fn window_w(panel_w: usize) -> usize {
    FULL_W + panel_w
}

/// What a game shows in the window besides its picture: a panel beside it,
/// and anything laid over the two (a picker, the pause notice). It is the
/// game's side of the state the machine's thread and the window share.
pub trait Screen: Send + Sync + 'static {
    /// The panel's width beside the picture, at the Spectrum's scale; 0 for
    /// none.
    const PANEL_W: usize;
    /// Something that changes whenever what is drawn over the picture does,
    /// so the overlay is drawn again only then.
    type Stamp: Copy + PartialEq + Send;
    /// What drawing needs, copied out from under the game's locks.
    type Shown;
    /// Draws what is shown; kept by the window between frames.
    type Painter: Default;

    /// The stamp as things stand.
    fn stamp(&self) -> Self::Stamp;
    /// The stamp and a copy of what is shown, if the stamp is not `since`.
    fn shown_unless(&self, since: Option<Self::Stamp>) -> Option<(Self::Stamp, Self::Shown)>;
    /// Draws `shown` over the picture and the panel, `paused` saying whether
    /// the emulation is frozen.
    fn draw(painter: &mut Self::Painter, canvas: &mut Canvas, shown: &Self::Shown, paused: bool);
    /// Offers the game a key the window got, before it goes to the Spectrum.
    fn key(&self, code: KeyCode) -> Key;
}

/// What the game did with a key it was offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Not the game's: it goes to the Spectrum.
    Pass,
    /// Taken. With `release`, whatever the Spectrum has held is let go (a
    /// picker has opened over it, and the game will not see the key-ups);
    /// with `quit`, the program closes.
    Taken { release: bool, quit: bool },
}

/// A palette colour as the opaque RGBA pixel `pixels` wants.
fn rgba(c: u32) -> [u8; 4] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8, 0xFF]
}

/// Draws Spectrum display memory with a border into an RGBA frame.
pub fn draw(mem: &[u8], border: u8, frame: u64, out: &mut [u8]) {
    let out = out.as_chunks_mut::<4>().0;
    out.fill(rgba(zx_core::screen::PALETTE[(border & 7) as usize]));
    zx_core::screen::render(
        mem,
        &mem[BITMAP_LEN..],
        (frame / 16) % 2 == 1,
        out,
        FULL_W,
        BORDER * FULL_W + BORDER,
        rgba,
    );
}

/// Draws the picture into the window's buffer, `window_w` wide, which has
/// the panel's width beside it. The overlay covers the panel's part; it is
/// filled here only so nothing stale shows before the overlay's first frame.
fn draw_window(mem: &[u8], border: u8, frame: u64, out: &mut [u8], window_w: usize) {
    let edge = rgba(zx_core::screen::PALETTE[(border & 7) as usize]);
    for row in out.as_chunks_mut::<4>().0.chunks_exact_mut(window_w) {
        row[..FULL_W].fill(edge);
        row[FULL_W..].fill([0, 0, 0, 0xFF]);
    }
    zx_core::screen::render(
        mem,
        &mem[BITMAP_LEN..],
        (frame / 16) % 2 == 1,
        out.as_chunks_mut::<4>().0,
        window_w,
        BORDER * window_w + BORDER,
        rgba,
    );
}

/// A colour as the linear value `wgpu` wants for clearing an sRGB surface.
fn clear_colour([r, g, b]: [u8; 3]) -> pixels::wgpu::Color {
    let linear = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    pixels::wgpu::Color {
        r: linear(r),
        g: linear(g),
        b: linear(b),
        a: 1.0,
    }
}

/// Starts the game from a checked copy of it, returning the sound stream to
/// hold.
pub type Launcher = Box<dyn FnMut(Vec<u8>) -> Result<Option<cpal::Stream>, String>>;

struct App<G: Screen> {
    game: &'static Game,
    shared: Arc<Shared<G>>,
    /// The screen asking for the tape, while it is up. The game has not
    /// started until it is gone.
    prompt: Option<Prompt>,
    launch: Launcher,
    /// Held for as long as the game should have sound.
    stream: Option<cpal::Stream>,
    /// Device pixels per logical pixel, which the prompt draws at.
    scale: f64,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    error: Option<String>,
    /// The host keys physically down, which the machine's input is built
    /// from. winit only synthesises key-ups on focus loss on some platforms,
    /// so this is cleared when the window stops listening.
    held: HashSet<KeyCode>,
    /// The modifier keys down now, for the Mac's fullscreen shortcut.
    modifiers: ModifiersState,
    /// The game frame last painted, so the same one is not painted twice.
    shown: u64,
    /// The panel and what the game lays over the picture, at the window's
    /// resolution.
    overlay: Option<Overlay>,
    painter: G::Painter,
    /// What the overlay was last drawn from: the game's stamp, whether the
    /// game was paused, and the size it was drawn at. It is redrawn only
    /// when this changes.
    drawn: Option<(G::Stamp, bool, (u32, u32))>,
}

/// Whether `code`, with `modifiers` down, leaves or enters fullscreen: F11,
/// and on a Mac, where F11 never reaches a program (it is a media key
/// without Fn, and Show Desktop with it), the Mac's own Control-Command-F
/// (#183).
fn fullscreen_key(code: KeyCode, modifiers: ModifiersState, macos: bool) -> bool {
    code == KeyCode::F11
        || (macos
            && code == KeyCode::KeyF
            && modifiers.contains(ModifiersState::CONTROL | ModifiersState::SUPER))
}

/// The whole scale a window fits at on a screen `width` by `height`
/// logical pixels (#57): the largest whose window fits with room for the
/// window's own frame and the taskbar or dock, and never less than one.
fn windowed_scale(width: f64, height: f64, window_w: usize) -> f64 {
    let (room_w, room_h) = (width * 0.98, height - 96.0);
    let fit = (room_w / window_w as f64).min(room_h / FULL_H as f64);
    fit.floor().max(1.0)
}

impl<G: Screen> ApplicationHandler for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // The game starts fullscreen (#57), since the picture is scaled by
        // a whole number and a screen's work area rarely has room for the
        // next step up: at 1920 × 1080 a window can only reach three
        // screen pixels a Spectrum pixel, where fullscreen reaches four.
        // The size below is the one leaving fullscreen falls back to.
        let scale = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
            .map_or(SCALE, |monitor| {
                let factor = monitor.scale_factor();
                let size = monitor.size();
                windowed_scale(
                    f64::from(size.width) / factor,
                    f64::from(size.height) / factor,
                    window_w(G::PANEL_W),
                )
            });
        let attrs = Window::default_attributes()
            .with_title(self.game.title)
            // A dark title bar whatever the desktop's setting (#97). Left to
            // winit, GNOME drew it light in dark mode: on X11 the window gets
            // no dark hint unless a theme is given, and on Wayland winit asks
            // the desktop over D-Bus with a 100 ms timeout. A theme given here
            // settles both, and suits the game's black border.
            .with_theme(Some(Theme::Dark))
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .with_inner_size(LogicalSize::new(
                window_w(G::PANEL_W) as f64 * scale,
                FULL_H as f64 * scale,
            ))
            .with_min_inner_size(LogicalSize::new(window_w(G::PANEL_W) as f64, FULL_H as f64));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        // Some compositors report 0x0 before the first configure, and wgpu
        // panics when a surface is configured that size.
        let (w, h) = (size.width.max(1), size.height.max(1));
        let surface = SurfaceTexture::new(w, h, window.clone());
        self.scale = window.scale_factor();
        let (bw, bh) = self.buffer_size();
        match Pixels::new(bw, bh, surface) {
            Ok(mut p) => {
                self.overlay = Some(Overlay::new(&p.context().device, p.render_texture_format()));
                if self.prompt.is_some() {
                    // The prompt is narrower than the window; its margins
                    // should be its own colour, not black.
                    p.clear_color(clear_colour(prompt::BACKGROUND));
                }
                self.pixels = Some(p);
            }
            Err(e) => {
                self.error = Some(e.to_string());
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let WindowEvent::ModifiersChanged(modifiers) = &event {
            self.modifiers = modifiers.state();
        }
        // The fullscreen key leaves or enters fullscreen on every screen,
        // the tape prompt too, where the player may want the desktop to
        // find the file.
        if let WindowEvent::KeyboardInput { event: key, .. } = &event
            && key.state == ElementState::Pressed
            && !key.repeat
            && let PhysicalKey::Code(code) = key.physical_key
            && fullscreen_key(code, self.modifiers, cfg!(target_os = "macos"))
        {
            self.toggle_fullscreen();
            return;
        }
        if self.prompt.is_some() {
            self.prompt_event(event_loop, event);
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.shared.quit.store(true, Ordering::Relaxed);
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key
                    && !event.repeat
                {
                    if event.state == ElementState::Pressed && self.game_key(code) {
                        return;
                    }
                    if event.state == ElementState::Pressed {
                        self.held.insert(code);
                    } else {
                        self.held.remove(&code);
                    }
                    *self.shared.input.lock().unwrap() = super::input::build(&self.held);
                }
            }
            // Nothing is held once the window is not listening. Without this,
            // a key held while switching away stays down for ever on the
            // platforms where winit sends no key-ups.
            WindowEvent::Focused(false) => {
                self.held.clear();
                *self.shared.input.lock().unwrap() = Input::default();
                self.shared.unfocused.store(true, Ordering::Relaxed);
            }
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.redraw_game(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // The game side sets this when it finishes, and when it stops
        // unexpectedly; either way the window should not outlive it.
        if self.shared.quit.load(Ordering::Relaxed) {
            event_loop.exit();
            return;
        }
        // The prompt changes only when something happens to it.
        if self.prompt.is_some() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        // Paint only when the game has produced a new frame, and let the loop
        // sleep in between. Asking for a redraw every time round instead ties
        // the rate to how long `render` blocks, and the moment the window is
        // occluded it stops blocking at all: that spun this thread at over
        // 20,000 repaints a second, burning a core the game needs.
        let (latest, paused) = {
            let screen = self.shared.screen.lock().unwrap();
            (screen.2, screen.3)
        };
        let stamp = self.shared.game.stamp();
        let overlay_stale = self.drawn.is_none_or(|(s, p, _)| s != stamp || p != paused);
        if latest != self.shown || overlay_stale {
            self.shown = latest;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(2),
        ));
    }
}

impl<G: Screen> App<G> {
    /// How many pixels a unit of the prompt's layout takes: the screen's
    /// density, but never more than lets the whole prompt fit the window,
    /// which a small or scaled-up screen would otherwise crop.
    fn prompt_scale(&self) -> f64 {
        let fit = self.window.as_ref().map_or(self.scale, |w| {
            let size = w.inner_size();
            (f64::from(size.width) / f64::from(prompt::WIDTH))
                .min(f64::from(size.height) / f64::from(prompt::HEIGHT))
        });
        self.scale.min(fit).max(0.25)
    }

    /// The frame buffer's size: the Spectrum's screen with its border, or,
    /// for the prompt, the window at its real pixel density so the text is
    /// sharp.
    fn buffer_size(&self) -> (u32, u32) {
        if self.prompt.is_some() {
            let s = self.prompt_scale();
            (
                (f64::from(prompt::WIDTH) * s).round() as u32,
                (f64::from(prompt::HEIGHT) * s).round() as u32,
            )
        } else {
            (window_w(G::PANEL_W) as u32, FULL_H as u32)
        }
    }

    /// Paints the game's latest frame, then lays the overlay over it,
    /// redrawing the overlay first if what it shows has changed.
    fn redraw_game(&mut self, event_loop: &ActiveEventLoop) {
        let Some(p) = &mut self.pixels else {
            return;
        };
        let paused = {
            let screen = self.shared.screen.lock().unwrap();
            draw_window(
                &screen.0,
                screen.1,
                screen.2,
                p.frame_mut(),
                window_w(G::PANEL_W),
            );
            screen.3
        };
        let clip = p.context().scaling_renderer.clip_rect();
        // The overlay is redrawn only when what it shows has changed, so
        // what it shows is copied out from under the game's locks only then
        // (#82): for Starquake a copy is every room's openings, the items,
        // the routes and the codes, too much to take fifty times a second
        // for nothing.
        let size = (clip.2, clip.3);
        let since = self
            .drawn
            .filter(|&(_, p, s)| p == paused && s == size)
            .map(|(stamp, _, _)| stamp);
        let shown = self.shared.game.shown_unless(since);
        if let Some((stamp, shown)) = shown
            && let Some(overlay) = &mut self.overlay
        {
            let scale = clip.2 as f32 / overlay::width(G::PANEL_W);
            let mut canvas = overlay.canvas(clip.2, clip.3, scale);
            G::draw(&mut self.painter, &mut canvas, &shown, paused);
            self.drawn = Some((stamp, paused, size));
        }
        let overlay = &mut self.overlay;
        let rendered = p.render_with(|encoder, target, context| {
            context.scaling_renderer.render(encoder, target);
            if let Some(overlay) = overlay {
                let clip = context.scaling_renderer.clip_rect();
                overlay.render(&context.device, &context.queue, encoder, target, clip);
            }
            Ok(())
        });
        if let Err(e) = rendered {
            self.error = Some(e.to_string());
            event_loop.exit();
        }
    }

    /// F11 leaves fullscreen, or goes back to it (#57). The window falls
    /// back to the size it was created at, which is the largest whole
    /// scale that fits the screen.
    fn toggle_fullscreen(&mut self) {
        if let Some(window) = &self.window {
            let to = match window.fullscreen() {
                Some(_) => None,
                None => Some(Fullscreen::Borderless(None)),
            };
            window.set_fullscreen(to);
        }
    }

    /// Offers a key to the game ([`Screen::key`]) before the Spectrum gets
    /// it. Returns whether the game took it.
    fn game_key(&mut self, code: KeyCode) -> bool {
        let Key::Taken { release, quit } = self.shared.game.key(code) else {
            return false;
        };
        if quit {
            // The window closes on the next turn of the event loop.
            self.shared.quit.store(true, Ordering::Relaxed);
        }
        if release {
            self.held.clear();
            *self.shared.input.lock().unwrap() = Input::default();
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        true
    }

    fn prompt_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let scale = self.prompt_scale();
        let Some(prompt) = &mut self.prompt else {
            return;
        };
        let outcome = match event {
            WindowEvent::CloseRequested => Outcome::Quit,
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(code)
                    if event.state == ElementState::Pressed && !event.repeat =>
                {
                    prompt.key(code)
                }
                _ => Outcome::Nothing,
            },
            WindowEvent::CursorMoved { position, .. } => match &self.pixels {
                Some(p) => {
                    let (x, y) = p
                        .window_pos_to_pixel((position.x as f32, position.y as f32))
                        .unwrap_or_else(|(x, y)| (x.max(0) as usize, y.max(0) as usize));
                    let s = scale as f32;
                    prompt.cursor(x as f32 / s, y as f32 / s)
                }
                None => Outcome::Nothing,
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => prompt.clicked(),
            WindowEvent::DroppedFile(path) => prompt.dropped(&path),
            WindowEvent::Resized(size) => {
                // The prompt is drawn to fit: a buffer larger than the
                // window would be cropped, not shrunk.
                let (w, h) = self.buffer_size();
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width, size.height);
                    let _ = p.resize_buffer(w, h);
                }
                Outcome::Redraw
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                let (w, h) = self.buffer_size();
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_buffer(w, h);
                }
                Outcome::Redraw
            }
            WindowEvent::RedrawRequested => {
                let (w, h) = self.buffer_size();
                if let (Some(p), Some(prompt)) = (&mut self.pixels, &mut self.prompt) {
                    let mut canvas = Canvas {
                        pixels: p.frame_mut(),
                        width: w as usize,
                        height: h as usize,
                        scale: scale as f32,
                    };
                    prompt.draw(&mut canvas);
                    if let Err(e) = p.render() {
                        self.error = Some(e.to_string());
                        event_loop.exit();
                    }
                }
                Outcome::Nothing
            }
            _ => Outcome::Nothing,
        };
        match outcome {
            Outcome::Nothing => {}
            Outcome::Redraw => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            Outcome::Quit => {
                self.shared.quit.store(true, Ordering::Relaxed);
                event_loop.exit();
            }
            Outcome::Start(tape) => {
                let started = (self.launch)(tape.bytes);
                match started {
                    Ok(stream) => {
                        self.stream = stream;
                        self.prompt = None;
                        let (w, h) = self.buffer_size();
                        if let Some(p) = &mut self.pixels {
                            let _ = p.resize_buffer(w, h);
                            p.clear_color(pixels::wgpu::Color::BLACK);
                        }
                        self.shown = u64::MAX;
                    }
                    Err(e) => {
                        self.error = Some(e);
                        event_loop.exit();
                    }
                }
            }
        }
    }
}

/// Runs the window for `game` until it closes: the prompt for its tape
/// first if `prompt` is given, then the game, which `launch` starts.
///
/// # Errors
///
/// If the window or its surface cannot be made, or the game stopped
/// unexpectedly, with why.
///
/// # Panics
///
/// If the machine's thread panicked while holding the reason it stopped.
pub fn run<G: Screen>(
    game: &'static Game,
    shared: Arc<Shared<G>>,
    prompt: Option<Prompt>,
    launch: Launcher,
) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        game,
        shared,
        prompt,
        launch,
        stream: None,
        scale: 1.0,
        window: None,
        pixels: None,
        error: None,
        shown: u64::MAX,
        held: HashSet::new(),
        modifiers: ModifiersState::empty(),
        overlay: None,
        painter: G::Painter::default(),
        drawn: None,
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    if let Some(e) = app.error {
        return Err(e);
    }
    if app.shared.dead.load(Ordering::Relaxed) {
        return Err(app
            .shared
            .why
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(|| "the game stopped unexpectedly".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use winit::keyboard::{KeyCode, ModifiersState};

    use super::{fullscreen_key, windowed_scale};

    #[test]
    fn f11_everywhere_and_control_command_f_on_a_mac() {
        let none = ModifiersState::empty();
        let both = ModifiersState::CONTROL | ModifiersState::SUPER;
        assert!(fullscreen_key(KeyCode::F11, none, false));
        assert!(fullscreen_key(KeyCode::F11, none, true));
        assert!(fullscreen_key(KeyCode::KeyF, both, true));
        assert!(!fullscreen_key(KeyCode::KeyF, both, false), "not off a Mac");
        assert!(!fullscreen_key(KeyCode::KeyF, ModifiersState::SUPER, true));
        assert!(
            !fullscreen_key(KeyCode::KeyF, none, true),
            "the Spectrum's F"
        );
    }

    #[test]
    fn the_window_opens_at_the_largest_whole_scale_that_fits() {
        // With Starquake's panel the window is 456 by 256 Spectrum pixels a
        // scale.
        let scale = |w, h| windowed_scale(w, h, WINDOW_W);
        assert_eq!(
            scale(1920.0, 1080.0),
            3.0,
            "1024 tall does not fit under the chrome"
        );
        assert_eq!(scale(2560.0, 1440.0), 5.0);
        assert_eq!(scale(3840.0, 2160.0), 8.0);
        assert_eq!(scale(1366.0, 768.0), 2.0);
        assert_eq!(scale(1600.0, 900.0), 3.0);
        assert_eq!(scale(640.0, 480.0), 1.0, "never less than one");
    }

    /// Starquake's window: its panel, 136 wide, beside the picture.
    const WINDOW_W: usize = window_w(136);

    use super::*;

    fn screen(bitmap: u8, attr: u8) -> Vec<u8> {
        let mut mem = vec![bitmap; BITMAP_LEN];
        mem.extend(std::iter::repeat_n(attr, 768));
        mem
    }

    fn pixel(frame: &[u8], x: usize, y: usize) -> [u8; 4] {
        let at = (y * FULL_W + x) * 4;
        frame[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn the_picture_sits_inside_a_border_of_its_colour() {
        let mut out = vec![0u8; FULL_W * FULL_H * 4];
        // All ink, blue ink on red paper; a green border.
        draw(&screen(0xFF, 0o21), 4, 0, &mut out);
        assert_eq!(pixel(&out, 0, 0), [0, 0xD8, 0, 0xFF]);
        assert_eq!(pixel(&out, BORDER - 1, BORDER), [0, 0xD8, 0, 0xFF]);
        assert_eq!(pixel(&out, BORDER, BORDER), [0, 0, 0xD8, 0xFF]);
        assert_eq!(pixel(&out, BORDER + 255, BORDER + 191), [0, 0, 0xD8, 0xFF]);
        assert_eq!(pixel(&out, BORDER + 256, BORDER + 191), [0, 0xD8, 0, 0xFF]);
        assert_eq!(pixel(&out, FULL_W - 1, FULL_H - 1), [0, 0xD8, 0, 0xFF]);
    }

    #[test]
    fn the_window_is_the_picture_with_the_panel_beside_it() {
        let mut out = vec![0u8; WINDOW_W * FULL_H * 4];
        draw_window(&screen(0xFF, 0o21), 4, 0, &mut out, WINDOW_W);
        let at = |x: usize, y: usize| {
            let i = (y * WINDOW_W + x) * 4;
            <[u8; 4]>::try_from(&out[i..i + 4]).unwrap()
        };
        assert_eq!(at(0, 0), [0, 0xD8, 0, 0xFF], "the border");
        assert_eq!(at(BORDER, BORDER), [0, 0, 0xD8, 0xFF], "the picture");
        assert_eq!(at(FULL_W - 1, FULL_H - 1), [0, 0xD8, 0, 0xFF]);
        assert_eq!(at(FULL_W, 0), [0, 0, 0, 0xFF], "the panel's place");
        assert_eq!(at(WINDOW_W - 1, FULL_H - 1), [0, 0, 0, 0xFF]);
        assert_eq!((WINDOW_W * 3, FULL_H * 3), (1368, 768));
    }

    #[test]
    fn flashing_cells_swap_every_16_frames() {
        let mem = screen(0xFF, 0x80 | 0o21);
        let at = |frame| {
            let mut out = vec![0u8; FULL_W * FULL_H * 4];
            draw(&mem, 0, frame, &mut out);
            pixel(&out, BORDER, BORDER)
        };
        assert_eq!(at(0), [0, 0, 0xD8, 0xFF]);
        assert_eq!(at(15), [0, 0, 0xD8, 0xFF]);
        assert_eq!(at(16), [0xD8, 0, 0, 0xFF]);
        assert_eq!(at(32), [0, 0, 0xD8, 0xFF]);
    }

    #[test]
    fn clearing_colours_are_converted_to_linear_light() {
        let c = clear_colour([0, 255, 188]);
        assert!(c.r.abs() < 1e-9);
        assert!((c.g - 1.0).abs() < 1e-9);
        assert!((c.b - 0.5).abs() < 0.01, "sRGB 188 is about half the light");
        assert!((clear_colour([10, 10, 10]).r - 10.0 / 255.0 / 12.92).abs() < 1e-9);
        assert_eq!(c.a, 1.0);
    }
}
