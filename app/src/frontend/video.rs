//! The window: draws the latest frame, with its border, scaled by the GPU.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use super::Shared;
use super::prompt::{self, Outcome, Prompt};
use super::text::Canvas;
use sidekick::Input;
use zx_core::screen::{BITMAP_LEN, HEIGHT, WIDTH};

const BORDER: usize = 32;
pub const FULL_W: usize = WIDTH + 2 * BORDER;
pub const FULL_H: usize = HEIGHT + 2 * BORDER;
const SCALE: f64 = 3.0;

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

struct App {
    shared: Arc<Shared>,
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
    /// The game frame last painted, so the same one is not painted twice.
    shown: u64,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("ZX Sidekick · Starquake")
            .with_inner_size(LogicalSize::new(
                FULL_W as f64 * SCALE,
                FULL_H as f64 * SCALE,
            ))
            .with_min_inner_size(LogicalSize::new(FULL_W as f64, FULL_H as f64));
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
            }
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(p) = &mut self.pixels {
                    {
                        let screen = self.shared.screen.lock().unwrap();
                        draw(&screen.0, screen.1, screen.2, p.frame_mut());
                    }
                    if let Err(e) = p.render() {
                        self.error = Some(e.to_string());
                        event_loop.exit();
                    }
                }
            }
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
        let latest = self.shared.screen.lock().unwrap().2;
        if latest != self.shown {
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

impl App {
    /// The frame buffer's size: the Spectrum's screen with its border, or,
    /// for the prompt, the window at its real pixel density so the text is
    /// sharp.
    fn buffer_size(&self) -> (u32, u32) {
        if self.prompt.is_some() {
            (
                (f64::from(prompt::WIDTH) * self.scale).round() as u32,
                (f64::from(prompt::HEIGHT) * self.scale).round() as u32,
            )
        } else {
            (FULL_W as u32, FULL_H as u32)
        }
    }

    fn prompt_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
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
                    let s = self.scale as f32;
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
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width, size.height);
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
                        scale: self.scale as f32,
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

pub fn run(shared: Arc<Shared>, prompt: Option<Prompt>, launch: Launcher) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
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
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    if let Some(e) = app.error {
        return Err(e);
    }
    if app.shared.dead.load(Ordering::Relaxed) {
        return Err("the game stopped unexpectedly; see the panic above".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
