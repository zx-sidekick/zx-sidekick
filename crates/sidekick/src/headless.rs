//! Running a machine without a window, and writing its picture as a PNG.

use crate::{
    compose,
    keys::key_named,
    picture::{to_rgba, Part},
};
use sidekick_core::{Key, Picture, Session};
use sidekick_ui::{Fonts, PadLabels};
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

/// A training switch turned on at a frame, from `ID@FRAME`.
#[derive(Clone)]
pub(crate) struct SwitchAt {
    id: String,
    frame: u32,
}

impl std::str::FromStr for SwitchAt {
    type Err = String;

    fn from_str(spec: &str) -> Result<Self, String> {
        let (id, frame) = spec.rsplit_once('@').ok_or_else(|| {
            format!("{spec:?}: write a switch as ID@FRAME, like endless-lives@2000")
        })?;
        let frame = frame
            .parse()
            .map_err(|_| format!("{frame:?} isn't a frame number"))?;
        Ok(Self {
            id: id.to_owned(),
            frame,
        })
    }
}

/// What a screenshot shows.
#[derive(Clone, Copy)]
pub(crate) enum Shot {
    /// The Spectrum's picture alone, each pixel `scale` × `scale` pixels.
    Picture { scale: u32 },
    /// The whole window, `size` screen pixels, played at guidance `level`.
    Window { level: u8, size: (u32, u32) },
}

/// A window size from `WIDTHxHEIGHT`, like 1368x768.
///
/// # Errors
/// If it isn't two numbers, each at least 1.
pub(crate) fn size(spec: &str) -> Result<(u32, u32), String> {
    let wrong = || format!("{spec:?}: write a size as WIDTHxHEIGHT, like 1368x768");
    let (w, h) = spec.split_once(['x', '×']).ok_or_else(wrong)?;
    let (w, h): (u32, u32) = (
        w.parse().map_err(|_| wrong())?,
        h.parse().map_err(|_| wrong())?,
    );
    if w == 0 || h == 0 {
        return Err(wrong());
    }
    Ok((w, h))
}

/// Runs `frames` frames, pressing the keys and turning the switches on when their frames
/// come, then writes the screenshot, if one is asked for, as a PNG.
///
/// # Errors
/// If the machine fails, a switch won't go on, or the PNG can't be written.
pub(crate) fn run(
    session: &mut Session,
    frames: u32,
    keys: &[KeyPress],
    switches: &[SwitchAt],
    screenshot: Option<(&Path, Shot)>,
) -> Result<(), String> {
    if let Some((_, Shot::Window { level, .. })) = screenshot {
        session.set_level(level);
    }
    for frame in 0..frames {
        for switch in switches.iter().filter(|s| s.frame == frame) {
            session
                .switch(&switch.id, true)
                .map_err(|e| format!("{} at frame {frame}: {e}", switch.id))?;
        }
        for press in keys {
            if frame == press.frame {
                session.machine_mut().key(press.key, true);
            } else if frame == press.frame + KEY_FRAMES {
                session.machine_mut().key(press.key, false);
            }
        }
        session.run_frame().map_err(|e| e.to_string())?;
        // Nobody's there to press a key: a loading screen held goes straight on.
        while session.holding() {
            session.release();
            session.run_frame().map_err(|e| e.to_string())?;
        }
    }
    let Some((path, shot)) = screenshot else {
        return Ok(());
    };
    let written = match shot {
        Shot::Picture { scale } => write_png(
            session.machine().picture(),
            compose::shown(session),
            scale,
            path,
        ),
        Shot::Window { level, size } => {
            let rgba = compose::window(
                session,
                usize::from(level),
                size,
                PadLabels::Letters,
                &mut Fonts::new(),
            );
            fs::write(path, sidekick_ui::png::encode(&rgba, size.0, size.1))
        }
    };
    written.map_err(|e| format!("{}: {e}", path.display()))
}

/// The game's state, the record of the game being played, and every named value as of the
/// last frame, a `name = value` line each (the state `""` when none matched).
pub(crate) fn values(session: &Session) -> String {
    let record = session.record();
    let switches: Vec<String> = record.switches.iter().map(|s| format!("{s:?}")).collect();
    let mut lines = vec![
        format!("state = {:?}", session.state().unwrap_or("")),
        format!(
            "record = {{ level = {}, switches = [{}] }}",
            record.level,
            switches.join(", ")
        ),
    ];
    lines.extend(
        session
            .reading()
            .iter()
            .map(|(name, value)| format!("{name} = {value}")),
    );
    lines.join("\n") + "\n"
}

/// Writes the `part` of the picture shown as a PNG, each Spectrum pixel `scale` × `scale`
/// pixels.
fn write_png(picture: &Picture, part: Part, scale: u32, path: &Path) -> std::io::Result<()> {
    let mut rgba = Vec::new();
    to_rgba(picture, &mut rgba);
    let (left, top, width, height) = (
        part.0 as usize,
        part.1 as usize,
        part.2 as usize,
        part.3 as usize,
    );
    let k = usize::try_from(scale.max(1)).unwrap_or(1);
    let mut scaled = Vec::with_capacity(width * height * 4 * k * k);
    for row in rgba
        .chunks_exact(picture.width() * 4)
        .skip(top)
        .take(height)
    {
        let line: Vec<u8> = row
            .chunks_exact(4)
            .skip(left)
            .take(width)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_size_is_two_numbers() {
        assert_eq!(size("1368x768"), Ok((1368, 768)));
        assert_eq!(size("800×600"), Ok((800, 600)));
        for wrong in ["1368", "0x768", "ax768", ""] {
            assert!(size(wrong).is_err(), "{wrong:?}");
        }
    }
}
