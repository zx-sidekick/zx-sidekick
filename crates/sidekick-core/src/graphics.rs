//! The game's font and pictures, read from its memory as a definition's `font` and `sprites`
//! values say: never shipped, always the player's own copy's.

use crate::{Machine, Read};
use sidekick_def::{Layout, Mask, Value, ValueType};

/// A picture of one bit a pixel: its ink, where its bits are set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    /// Its width, in the game's pixels.
    pub width: usize,
    /// Its height, in the game's pixels.
    pub height: usize,
    /// Row by row, whether each pixel is ink.
    pub ink: Vec<bool>,
    /// Its ink colour, 0 to 15 (8 to 15 bright), when the game gives one.
    pub color: Option<u8>,
}

impl Bitmap {
    /// Whether the pixel at (`x`, `y`) is ink.
    #[must_use]
    pub fn at(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.ink[y * self.width + x]
    }
}

/// A game's font: its characters, from `first`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Font {
    /// The code of the first character.
    pub first: u8,
    /// Each character, in code order.
    pub glyphs: Vec<Bitmap>,
}

impl Font {
    /// The character for a letter, if the font has it.
    #[must_use]
    pub fn glyph(&self, letter: char) -> Option<&Bitmap> {
        let code = u8::try_from(u32::from(letter)).ok()?;
        self.glyphs.get(usize::from(code.checked_sub(self.first)?))
    }
}

/// The most characters or pictures read, whatever a definition says.
const MOST: u32 = 1024;

/// One bit a pixel from `bytes`, `width` pixels a row (a whole number of bytes), high bit
/// leftmost.
fn bitmap(bytes: &[u8], width: usize, height: usize) -> Bitmap {
    let row_bytes = width.div_ceil(8);
    let ink = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let byte = bytes.get(y * row_bytes + x / 8).copied().unwrap_or(0);
                byte & (0x80 >> (x % 8)) != 0
            })
        })
        .collect();
    Bitmap {
        width,
        height,
        ink,
        color: None,
    }
}

/// A picture stored in 8 × 8 cells, as rows: each row of each cell in turn.
fn from_cells(cells: &[u8], width: usize, height: usize) -> Vec<u8> {
    let across = width.div_ceil(8);
    (0..height)
        .flat_map(|y| {
            (0..across).map(move |cell| {
                let at = ((y / 8) * across + cell) * 8 + y % 8;
                cells.get(at).copied().unwrap_or(0)
            })
        })
        .collect()
}

fn bytes(machine: &Machine, addr: u32, len: u32) -> Vec<u8> {
    (addr..addr.saturating_add(len))
        .map(|a| u16::try_from(a).map_or(0, |a| machine.peek(a)))
        .collect()
}

/// Reads a `font` value.
pub(crate) fn font(machine: &Machine, value: &Value) -> Option<Font> {
    if value.kind != ValueType::Font {
        return None;
    }
    let addr = u32::from(value.addr?.0);
    let (width, height) = (value.width.unwrap_or(8), value.height.unwrap_or(8));
    let size = width.div_ceil(8) * height;
    let count = match value.count {
        Some(sidekick_def::Count::Fixed(n)) => n,
        _ => 96,
    }
    .min(MOST);
    let glyphs = (0..count)
        .map(|i| {
            let raw = bytes(machine, addr + i * size, size);
            bitmap(&raw, width as usize, height as usize)
        })
        .collect();
    Some(Font {
        first: value.first.unwrap_or(0x20),
        glyphs,
    })
}

/// Reads a `sprites` value; `colors` is its `colors` value as read, when it has one.
pub(crate) fn sprites(
    machine: &Machine,
    value: &Value,
    colors: Option<&Read>,
) -> Option<Vec<Bitmap>> {
    if value.kind != ValueType::Sprites {
        return None;
    }
    let addr = u32::from(value.addr?.0);
    let (width, height) = (value.width.unwrap_or(8), value.height.unwrap_or(8));
    // Cells are whole: a picture in cells ends on a cell's edge, or it isn't one.
    if value.layout == Some(Layout::Cells) && height % 8 != 0 {
        return None;
    }
    let size = width.div_ceil(8) * height;
    let mask = value.mask.unwrap_or(Mask::None);
    let stored = if mask == Mask::None { size } else { size * 2 };
    let stride = value.stride.unwrap_or(stored);
    let count = match value.count {
        Some(sidekick_def::Count::Fixed(n)) => n,
        _ => 1,
    }
    .min(MOST);
    let pictures = (0..count)
        .map(|i| {
            let raw = bytes(machine, addr + i * stride, stored);
            // The picture's own bytes, without its mask: only ink is drawn.
            let data: Vec<u8> = match mask {
                Mask::None => raw,
                Mask::Interleaved => raw.iter().skip(1).step_by(2).copied().collect(),
                Mask::After => raw.into_iter().take(size as usize).collect(),
            };
            let data = match value.layout.unwrap_or_default() {
                Layout::Rows => data,
                Layout::Cells => from_cells(&data, width as usize, height as usize),
            };
            let mut picture = bitmap(&data, width as usize, height as usize);
            picture.color = colors.and_then(|c| ink(c, i as usize));
            picture
        })
        .collect();
    Some(pictures)
}

/// A picture's ink from its `colors` value: an attribute byte (ink in its low bits, bright
/// in bit 6), from bytes, or a table's first number.
fn ink(colors: &Read, i: usize) -> Option<u8> {
    let attr = match colors {
        Read::Bytes(bytes) => i64::from(*bytes.get(i)?),
        Read::Table(rows) => rows.get(i)?.values().find_map(Read::number)?,
        Read::Number(_) | Read::Flag(_) | Read::Bits(_) | Read::Text(_) => return None,
    };
    let attr = u8::try_from(attr & 0xFF).ok()?;
    Some((attr & 7) + ((attr >> 6) & 1) * 8)
}
