//! Running the machine without a window, for screenshots: scripted keys
//! carry it past the menu into a game, and every so often the whole window,
//! the picture with its border and the guidance panel beside it, is saved as
//! a PNG; the tape's loading picture first.

use std::path::Path;

use sidekick::Machine;
use sidekick::starquake::{ENTRY_PC, ENTRY_SP};

use super::guidance::Guidance;
use super::overlay;
use super::panel::Panel;
use super::text::Canvas;
use super::track::{self, Scene, Tracker};
use super::video::{FULL_H, FULL_W, draw};

/// Keys that carry an unattended run past the menu: `1` for the Kempston
/// joystick, `0` to start, and then the joystick wandering.
fn script(machine: &mut Machine, frame: u64) {
    machine.zx.release_all_keys();
    machine.joystick = 0;
    match frame {
        50..=54 => machine.zx.keys[3] &= !0x01,
        100..=104 => machine.zx.keys[4] &= !0x01,
        200.. => machine.joystick = [0x01, 0x02, 0x09, 0x0A, 0x11][(frame as usize / 25) % 5],
        _ => {}
    }
}

/// Runs `frames` frames and writes screenshots to `dir`.
///
/// # Errors
///
/// If the tape cannot be read or the folder cannot be written.
pub fn run(path: &Path, frames: u64, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tape = super::tape::read(path)?;
    let mut machine = Machine::from_tape(&tape, ENTRY_PC, ENTRY_SP)?;
    machine.watch = track::WATCH.to_vec();
    let mut tracker = Tracker::default();
    let mut guidance = Guidance::default();
    let mut panel = Panel::new();
    let mut picture = vec![0u8; FULL_W * FULL_H * 4];
    // The tape's loading picture, which the window shows before the game.
    if let Some(loading) = zx_core::tape::load_tap(&tape)?.loading_screen {
        let mut memory = vec![0u8; 0x1B00];
        let n = loading.len().min(memory.len());
        memory[..n].copy_from_slice(&loading[..n]);
        draw(&memory, 0, 0, &mut picture);
        let shot = window(&picture, &mut panel, &guidance, Scene::Loading, false);
        save(&shot, &dir.join("loading.png"))?;
    }
    let every = (frames / 40).max(1);
    for frame in 0..frames {
        script(&mut machine, frame);
        for hit in machine.run_frame() {
            tracker.follow(&machine.zx.mem[..], hit, &mut guidance);
        }
        machine.zx.speaker.clear();
        if frame % every == 0 || frame + 1 == frames {
            draw(
                &machine.zx.mem[0x4000..0x5B00],
                machine.zx.border,
                frame,
                &mut picture,
            );
            let shot = window(&picture, &mut panel, &guidance, tracker.scene, false);
            save(&shot, &dir.join(format!("frame{frame:06}.png")))?;
        }
    }
    Ok(())
}

/// The window as it would look at its first size: the picture scaled up on
/// the left and the overlay laid over it, as 0xRRGGBB pixels.
fn window(
    picture: &[u8],
    panel: &mut Panel,
    guidance: &Guidance,
    scene: Scene,
    paused: bool,
) -> Vec<u32> {
    let (w, h) = (overlay::WIDTH as usize, overlay::HEIGHT as usize);
    let mut over = vec![0u8; w * h * 4];
    let mut canvas = Canvas {
        pixels: &mut over,
        width: w,
        height: h,
        scale: 1.0,
    };
    canvas.clear_transparent();
    panel.draw(&mut canvas, guidance, scene, paused);
    let k = h / FULL_H;
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let under = if x < FULL_W * k {
                let at = ((y / k) * FULL_W + x / k) * 4;
                [picture[at], picture[at + 1], picture[at + 2]]
            } else {
                [0; 3]
            };
            let o = &over[i * 4..i * 4 + 4];
            let alpha = u32::from(o[3]);
            let channel = |c: usize| u32::from(o[c]) + u32::from(under[c]) * (255 - alpha) / 255;
            channel(0) << 16 | channel(1) << 8 | channel(2)
        })
        .collect()
}

/// Writes the window's pixels as a PNG.
fn save(pixels: &[u32], file: &Path) -> Result<(), String> {
    let (w, h) = (overlay::WIDTH as usize, overlay::HEIGHT as usize);
    std::fs::write(file, zx_core::png::encode(pixels, w, h))
        .map_err(|e| format!("{}: {e}", file.display()))
}
