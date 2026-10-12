//! Running a machine without a window, and writing its picture as a PNG.

use crate::{keys::key_named, picture::to_rgba};
use sidekick_core::{Key, Picture, Session};
use std::{fs, path::Path};

/// How many frames a `--key` holds its key: enough for a game to see it.
const KEY_FRAMES: u32 = 5;

/// A key pressed at a frame, from `NAME@FRAME`.
#[derive(Clone, Copy)]
pub(crate) struct KeyPress {
    key: Key,
    frame: u32,
}

impl std::str::FromStr for KeyPress {
    type Err = String;

    fn from_str(spec: &str) -> Result<Self, String> {
        let (name, frame) = spec
            .rsplit_once('@')
            .ok_or_else(|| format!("{spec:?}: write a key as NAME@FRAME, like SPACE@1500"))?;
        let key = key_named(name).ok_or_else(|| format!("{name:?} isn't a Spectrum key"))?;
        let frame = frame
            .parse()
            .map_err(|_| format!("{frame:?} isn't a frame number"))?;
        Ok(Self { key, frame })
    }
}

/// Runs `frames` frames, pressing the keys when their frames come, then writes the picture to
/// `screenshot`, `scale` times larger.
///
/// # Errors
/// If the machine fails, or the PNG can't be written.
pub(crate) fn run(
    session: &mut Session,
    frames: u32,
    keys: &[KeyPress],
    screenshot: Option<&Path>,
    scale: u32,
) -> Result<(), String> {
    for frame in 0..frames {
        for press in keys {
            if frame == press.frame {
                session.machine_mut().key(press.key, true);
            } else if frame == press.frame + KEY_FRAMES {
                session.machine_mut().key(press.key, false);
            }
        }
        session.run_frame().map_err(|e| e.to_string())?;
    }
    if let Some(path) = screenshot {
        write_png(session.machine().picture(), scale, path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// The game's state and every named value as of the last frame, a `name = value` line each,
/// the state first (`""` when none matched).
pub(crate) fn values(session: &Session) -> String {
    let mut lines = vec![format!("state = {:?}", session.state().unwrap_or(""))];
    lines.extend(
        session
            .reading()
            .iter()
            .map(|(name, value)| format!("{name} = {value}")),
    );
    lines.join("\n") + "\n"
}

/// Writes the picture as a PNG, each Spectrum pixel `scale` × `scale` pixels.
fn write_png(picture: &Picture, scale: u32, path: &Path) -> std::io::Result<()> {
    let mut rgba = Vec::new();
    to_rgba(picture, &mut rgba);
    let (width, height) = (picture.width(), picture.height());
    let k = usize::try_from(scale.max(1)).unwrap_or(1);
    let mut scaled = Vec::with_capacity(rgba.len() * k * k);
    for row in rgba.chunks_exact(width * 4) {
        let line: Vec<u8> = row
            .chunks_exact(4)
            .flat_map(|p| std::iter::repeat_n([p[0], p[1], p[2], p[3]], k).flatten())
            .collect();
        for _ in 0..k {
            scaled.extend_from_slice(&line);
        }
    }
    let size = |n: usize| u32::try_from(n * k).unwrap_or(u32::MAX);
    fs::write(
        path,
        sidekick_ui::png::encode(&scaled, size(width), size(height)),
    )
}
