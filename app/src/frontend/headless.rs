//! Running the machine without a window, for screenshots: scripted keys
//! carry it past the menu into a game, and every so often the picture, with
//! its border, is saved as a PNG; the tape's loading picture first.

use std::path::Path;

use sidekick::Machine;
use sidekick::starquake::{ENTRY_PC, ENTRY_SP};

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
    let mut picture = vec![0u8; FULL_W * FULL_H * 4];
    // The tape's loading picture, which the window shows before the game.
    if let Some(loading) = zx_core::tape::load_tap(&tape)?.loading_screen {
        let mut memory = vec![0u8; 0x1B00];
        let n = loading.len().min(memory.len());
        memory[..n].copy_from_slice(&loading[..n]);
        draw(&memory, 0, 0, &mut picture);
        save(&picture, &dir.join("loading.png"))?;
    }
    let every = (frames / 40).max(1);
    for frame in 0..frames {
        script(&mut machine, frame);
        machine.run_frame();
        machine.zx.speaker.clear();
        if frame % every == 0 || frame + 1 == frames {
            draw(
                &machine.zx.mem[0x4000..0x5B00],
                machine.zx.border,
                frame,
                &mut picture,
            );
            save(&picture, &dir.join(format!("frame{frame:06}.png")))?;
        }
    }
    Ok(())
}

/// Writes an RGBA picture the size of the window's Spectrum screen as a PNG.
fn save(picture: &[u8], file: &Path) -> Result<(), String> {
    let pixels: Vec<u32> = picture
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, _]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
        .collect();
    std::fs::write(file, zx_core::png::encode(&pixels, FULL_W, FULL_H))
        .map_err(|e| format!("{}: {e}", file.display()))
}
