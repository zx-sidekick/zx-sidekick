//! Running a machine without a window, and writing its picture as a PNG.

use crate::{keys::key_named, picture::to_rgba};
use sidekick_core::{Key, Machine, Picture};
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
    machine: &mut Machine,
    frames: u32,
    keys: &[KeyPress],
    screenshot: Option<&Path>,
    scale: u32,
) -> Result<(), String> {
    for frame in 0..frames {
        for press in keys {
            if frame == press.frame {
                machine.key(press.key, true);
            } else if frame == press.frame + KEY_FRAMES {
                machine.key(press.key, false);
            }
        }
        machine.run_frame().map_err(|e| e.to_string())?;
    }
    if let Some(path) = screenshot {
        write_png(machine.picture(), scale, path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// Writes the picture as an RGB PNG, each Spectrum pixel `scale` × `scale` pixels.
fn write_png(picture: &Picture, scale: u32, path: &Path) -> std::io::Result<()> {
    let mut rgba = Vec::new();
    to_rgba(picture, &mut rgba);
    let (width, height) = (picture.width(), picture.height());
    let scale = usize::try_from(scale.max(1)).unwrap_or(1);
    // Each row: filter type 0 (none), then the row's RGB.
    let mut rows = Vec::with_capacity((width * scale * 3 + 1) * height * scale);
    for row in rgba.chunks_exact(width * 4) {
        let line: Vec<u8> = row
            .chunks_exact(4)
            .flat_map(|p| std::iter::repeat_n([p[0], p[1], p[2]], scale).flatten())
            .collect();
        for _ in 0..scale {
            rows.push(0);
            rows.extend_from_slice(&line);
        }
    }
    let dimension = |n: usize| u32::try_from(n * scale).unwrap_or(u32::MAX).to_be_bytes();
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&dimension(width));
    header.extend_from_slice(&dimension(height));
    header.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB, deflate, no filter, no interlace
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, *b"IHDR", &header);
    chunk(
        &mut png,
        *b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&rows, 6),
    );
    chunk(&mut png, *b"IEND", &[]);
    fs::write(path, png)
}

/// Appends a PNG chunk: its length, type, data and CRC.
fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    png.extend_from_slice(&len.to_be_bytes());
    png.extend_from_slice(&kind);
    png.extend_from_slice(data);
    png.extend_from_slice(&crc32(kind.iter().chain(data)).to_be_bytes());
}

/// The CRC-32 PNG chunks carry (ISO 3309, polynomial 0xEDB88320).
fn crc32<'a>(bytes: impl Iterator<Item = &'a u8>) -> u32 {
    !bytes.fold(!0u32, |crc, &byte| {
        (0..8).fold(crc ^ u32::from(byte), |c, _| {
            if c & 1 == 1 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_is_png_s() {
        // The CRC of an IEND chunk, as every PNG ends with.
        assert_eq!(crc32(b"IEND".iter()), 0xAE42_6082);
        assert_eq!(crc32(b"123456789".iter()), 0xCBF4_3926);
    }
}
