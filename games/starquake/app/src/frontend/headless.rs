//! Running the machine without a window, for screenshots: scripted keys
//! carry it past the menu into a game, and every so often the whole window,
//! the picture with its border and the guidance panel beside it, is saved as
//! a PNG; the tape's loading picture first.

use std::path::Path;

use starquake::Machine;
use starquake::facts::{ENTRY_PC, ENTRY_SP};

use super::guidance::Guidance;
use super::panel::Panel;
use super::track::{self, Scene, Tracker};
use sidekick_frontend::overlay;
use sidekick_frontend::text::Canvas;
use sidekick_frontend::video::{FULL_H, FULL_W, draw};

/// Keys that carry an unattended run past the menu: `1` for the Kempston
/// joystick, `0` to start, Enter past the intro text, and then the joystick
/// wandering at random, a new direction every ten frames, so a run gets
/// about the planet. `seed` is the wander's state.
fn script(machine: &mut Machine, frame: u64, seed: &mut u64) {
    machine.zx.release_all_keys();
    match frame {
        50..=54 => machine.zx.keys[3] &= !0x01,
        100..=104 => machine.zx.keys[4] &= !0x01,
        330..=334 => machine.zx.keys[6] &= !0x01,
        400.. if frame.is_multiple_of(10) => {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            machine.rules.joystick = [0x01, 0x02, 0x08, 0x04, 0x10][(*seed % 5) as usize];
        }
        400.. => {}
        _ => machine.rules.joystick = 0,
    }
}

/// Runs `frames` frames at guidance `level` and writes screenshots to `dir`.
///
/// # Errors
///
/// If the tape cannot be read or the folder cannot be written.
pub fn run(path: &Path, frames: u64, dir: &Path, level: u8) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tape = sidekick_frontend::tape::read(&super::GAME, path)?;
    let mut machine = Machine::from_tape(&tape, ENTRY_PC, ENTRY_SP)?;
    machine.watch = track::WATCH.to_vec();
    let mut tracker = Tracker::default();
    let mut guidance = Guidance::default();
    guidance.set_level(level);
    let rooms = starquake::facts::all_rooms(&machine);
    tracker.graph = starquake::map::Graph::new(&rooms, starquake::facts::CORE_ROOM);
    tracker.door_rooms = starquake::facts::door_rooms(&rooms);
    guidance.set_door_spots(starquake::facts::door_spots(&rooms));
    guidance.set_openings(starquake::map::openings(
        &rooms,
        starquake::facts::CORE_ROOM,
    ));
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
    let mut seed = 0xBEEF;
    for frame in 0..frames {
        script(&mut machine, frame, &mut seed);
        for hit in machine.run_frame() {
            tracker.follow(&machine.zx.mem[..], hit, &mut guidance);
        }
        tracker.publish(&machine.zx.mem[..], &mut guidance);
        tracker.read_codes(&machine, &mut guidance);
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
    let (w, h) = (super::OVERLAY_W as usize, overlay::HEIGHT as usize);
    let mut over = vec![0u8; w * h * 4];
    let mut canvas = Canvas {
        pixels: &mut over,
        width: w,
        height: h,
        scale: 1.0,
    };
    canvas.clear_transparent();
    panel.draw(&mut canvas, guidance, scene, paused);
    overlay::composite(picture, &over, w, h)
}

/// Writes the window's pixels as a PNG.
fn save(pixels: &[u32], file: &Path) -> Result<(), String> {
    let (w, h) = (super::OVERLAY_W as usize, overlay::HEIGHT as usize);
    std::fs::write(file, zx_core::png::encode(pixels, w, h))
        .map_err(|e| format!("{}: {e}", file.display()))
}
