//! The window: the Spectrum's picture at the largest whole scale, its sound, and the keyboard.

use crate::{
    keys::{spectrum_key, Spectrum},
    picture::{placement, to_rgba},
};
use sdl3::{
    audio::{AudioFormat, AudioSpec, AudioStreamOwner},
    event::Event,
    keyboard::{Mod, Scancode},
    pixels::{Color, PixelFormat},
    rect::Rect,
    render::{FRect, ScaleMode},
};
use sidekick_core::{Machine, SAMPLE_RATE};
use std::time::{Duration, Instant};

/// A frame of the 48K: 69,888 T-states at 3.5 MHz.
const FRAME: Duration = Duration::from_micros(19_968);

/// Frames of sound kept queued ahead: enough to ride out a slow frame, few enough that a key
/// press is heard at once.
const QUEUED_FRAMES: usize = 3;

/// The most frames run between two presented pictures, so a stall can't snowball.
const MOST_FRAMES_A_PASS: usize = 5;

/// The picture's size: the screen with 32 pixels of border all round.
const PICTURE: (u32, u32) = (320, 256);

/// Plays a machine in a window until it's closed.
///
/// # Errors
/// If SDL can't open a window or draw into it.
pub(crate) fn play(mut machine: Machine, title: &str) -> Result<(), String> {
    sdl3::hint::set("SDL_RENDER_VSYNC", "1");
    let sdl = sdl3::init().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    let window = video
        .window(&format!("ZX Sidekick — {title}"), 1368, 768)
        .high_pixel_density()
        .resizable()
        .fullscreen()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormat::RGBA32, PICTURE.0, PICTURE.1)
        .map_err(|e| e.to_string())?;
    texture.set_scale_mode(ScaleMode::Nearest);

    // No sound device (a server, a test) is no reason not to play: time paces the frames then.
    let sound = open_sound(&sdl);
    let mut events = sdl.event_pump().map_err(|e| e.to_string())?;
    let mut rgba = Vec::with_capacity((PICTURE.0 * PICTURE.1 * 4) as usize);
    let mut samples = Vec::with_capacity(SAMPLE_RATE / 50 * 2);
    let started = Instant::now();
    let mut frames_run: u64 = 0;

    'play: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'play,
                Event::KeyDown {
                    scancode: Some(scancode),
                    keymod,
                    repeat,
                    ..
                } => {
                    if is_fullscreen_toggle(scancode, keymod) {
                        if !repeat {
                            let window = canvas.window_mut();
                            let fullscreen =
                                window.fullscreen_state() != sdl3::video::FullscreenType::Off;
                            window
                                .set_fullscreen(!fullscreen)
                                .map_err(|e| e.to_string())?;
                        }
                    } else if !repeat {
                        press(&mut machine, scancode, true);
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => press(&mut machine, scancode, false),
                _ => {}
            }
        }

        let due = sound.as_ref().map_or_else(
            || frames_due(started.elapsed(), frames_run),
            |stream| {
                let queued = usize::try_from(stream.queued_bytes().unwrap_or(0)).unwrap_or(0);
                let frame_bytes = SAMPLE_RATE / 50 * 2 * size_of::<f32>();
                QUEUED_FRAMES.saturating_sub(queued / frame_bytes)
            },
        );
        if due == 0 {
            // Vsync normally paces the loop; without it (some drivers) don't spin.
            std::thread::sleep(Duration::from_millis(1));
        }
        for _ in 0..due.min(MOST_FRAMES_A_PASS) {
            machine.run_frame().map_err(|e| e.to_string())?;
            frames_run += 1;
            if let Some(stream) = &sound {
                samples.clear();
                samples.extend(
                    machine
                        .sound()
                        .iter()
                        .flat_map(|&sample| <[f32; 2]>::from(sample)),
                );
                stream.put_data_f32(&samples).map_err(|e| e.to_string())?;
            }
        }

        to_rgba(machine.picture(), &mut rgba);
        texture
            .update(None::<Rect>, &rgba, PICTURE.0 as usize * 4)
            .map_err(|e| e.to_string())?;
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let (x, y, w, h) = placement(canvas.output_size().map_err(|e| e.to_string())?, PICTURE);
        #[allow(
            clippy::cast_precision_loss,
            reason = "screen sizes are far below f32's limit"
        )]
        let dst = FRect::new(x as f32, y as f32, w as f32, h as f32);
        canvas
            .copy(&texture, None, dst)
            .map_err(|e| e.to_string())?;
        canvas.present();
    }
    Ok(())
}

/// F11 everywhere; on a Mac also Control-Command-F, since F11 never reaches a program there.
fn is_fullscreen_toggle(scancode: Scancode, keymod: Mod) -> bool {
    let control_command = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
        && keymod.intersects(Mod::LGUIMOD | Mod::RGUIMOD);
    scancode == Scancode::F11 || (scancode == Scancode::F && control_command)
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
