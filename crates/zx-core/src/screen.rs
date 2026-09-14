//! The ULA's picture: where a pixel line lives, and what colour it comes out.
//!
//! The Spectrum's bitmap is not in reading order. A line's address scrambles
//! the y coordinate into thirds, character rows and pixel rows, which is why
//! the formula below appears wherever anything touches the screen. It is
//! written once here.

/// The 15 colours the ULA makes, as 0RGB. The first eight are normal, the
/// second eight bright; black appears in both, which is why there are 16
/// entries and 15 colours.
pub const PALETTE: [u32; 16] = [
    0x000000, 0x0000D8, 0xD80000, 0xD800D8, 0x00D800, 0x00D8D8, 0xD8D800, 0xD8D8D8, //
    0x000000, 0x0000FF, 0xFF0000, 0xFF00FF, 0x00FF00, 0x00FFFF, 0xFFFF00, 0xFFFFFF,
];

/// Pixels across and down the picture proper, without the border.
pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 192;
/// Bytes of bitmap, then of attributes.
pub const BITMAP_LEN: usize = 6144;
pub const ATTR_LEN: usize = 768;

/// Offset into the bitmap of the leftmost byte of pixel line `y`.
///
/// The ULA splits `y` into its third (bits 6-7), its pixel row within the
/// character (bits 0-2) and its character row within the third (bits 3-5),
/// and stores them in that order.
#[must_use]
pub const fn line_offset(y: usize) -> usize {
    ((y & 0xC0) << 5) | ((y & 7) << 8) | ((y & 0x38) << 2)
}

/// Offset into the attributes of the cell holding pixel line `y`.
#[must_use]
pub const fn attr_offset(y: usize) -> usize {
    (y / 8) * 32
}

/// Renders 256 x 192 of display memory.
///
/// `bitmap` and `attrs` are the two halves of the screen; `out` is a picture
/// `stride` pixels wide with the top-left of the 256 x 192 area at `origin`,
/// so a caller drawing a border just passes the offset of the inside. Flash
/// cells swap ink and paper when `flash` is set. `pixel` turns a palette
/// colour into whatever the caller's buffer holds.
pub fn render<P: Copy>(
    bitmap: &[u8],
    attrs: &[u8],
    flash: bool,
    out: &mut [P],
    stride: usize,
    origin: usize,
    pixel: impl Fn(u32) -> P,
) {
    for y in 0..HEIGHT {
        let line = line_offset(y);
        let row = attr_offset(y);
        for col in 0..32 {
            let bits = bitmap[line + col];
            let attr = attrs[row + col];
            let bright = ((attr >> 6) & 1) as usize * 8;
            let mut ink = pixel(PALETTE[(attr & 7) as usize + bright]);
            let mut paper = pixel(PALETTE[((attr >> 3) & 7) as usize + bright]);
            if attr & 0x80 != 0 && flash {
                std::mem::swap(&mut ink, &mut paper);
            }
            let at = origin + y * stride + col * 8;
            for bit in 0..8 {
                out[at + bit] = if bits & (0x80 >> bit) != 0 {
                    ink
                } else {
                    paper
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_land_where_the_ula_puts_them() {
        // The first line of each third, and the well-known jump from line 7
        // to line 8: down a pixel row is +256, down a character row is +32.
        assert_eq!(line_offset(0), 0);
        assert_eq!(line_offset(1), 256);
        assert_eq!(line_offset(7), 1792);
        assert_eq!(line_offset(8), 32);
        assert_eq!(line_offset(64), 2048);
        assert_eq!(line_offset(128), 4096);
        assert_eq!(line_offset(191), 6112);
    }

    /// Renders `bitmap` and `attrs` into a picture with a one-pixel margin,
    /// as a caller drawing a border would, with the palette index as the
    /// pixel.
    fn draw(bitmap: &[u8], attrs: &[u8], flash: bool) -> Vec<u32> {
        let stride = WIDTH + 2;
        let mut out = vec![u32::MAX; stride * (HEIGHT + 2)];
        render(bitmap, attrs, flash, &mut out, stride, stride + 1, |c| {
            PALETTE.iter().position(|&p| p == c).unwrap() as u32
        });
        out
    }

    #[test]
    fn ink_paper_bright_and_flash_come_from_the_attribute() {
        let mut bitmap = vec![0u8; BITMAP_LEN];
        let mut attrs = vec![0u8; ATTR_LEN];
        // Cell (0, 0): the left pixel set, blue ink on red paper.
        bitmap[0] = 0x80;
        attrs[0] = 0o21;
        // Cell (1, 0), one character row down: all set, bright yellow ink.
        for y in 8..16 {
            bitmap[line_offset(y)] = 0xFF;
        }
        attrs[32] = 0x40 | 6;
        // Cell (0, 1): flashing, white ink on black paper, no pixels set.
        attrs[1] = 0x80 | 7;

        let stride = WIDTH + 2;
        let at = |pic: &[u32], x: usize, y: usize| pic[(y + 1) * stride + x + 1];
        let pic = draw(&bitmap, &attrs, false);
        assert_eq!(at(&pic, 0, 0), 1, "ink");
        assert_eq!(at(&pic, 1, 0), 2, "paper");
        assert_eq!(at(&pic, 3, 12), 14, "bright yellow ink");
        assert_eq!(at(&pic, 8, 0), 0, "flash off shows paper");
        let pic = draw(&bitmap, &attrs, true);
        assert_eq!(at(&pic, 8, 0), 7, "flash on shows ink");
        assert_eq!(at(&pic, 0, 0), 1, "a cell without flash is unchanged");
    }

    #[test]
    fn rendering_leaves_the_margin_alone() {
        let pic = draw(&vec![0xFF; BITMAP_LEN], &vec![7; ATTR_LEN], false);
        let stride = WIDTH + 2;
        assert_eq!(pic[0], u32::MAX);
        assert_eq!(pic[stride], u32::MAX);
        assert_eq!(pic[stride + 1], 7);
        assert_eq!(pic[stride + WIDTH + 1], u32::MAX);
        assert_eq!(pic[(HEIGHT + 1) * stride + 1], u32::MAX);
    }

    #[test]
    fn a_cell_covers_eight_lines() {
        assert_eq!(attr_offset(0), 0);
        assert_eq!(attr_offset(7), 0);
        assert_eq!(attr_offset(8), 32);
        assert_eq!(attr_offset(191), 23 * 32);
    }

    #[test]
    fn every_line_is_used_exactly_once() {
        let mut seen = [false; HEIGHT];
        for y in 0..HEIGHT {
            let slot = line_offset(y) / 32;
            assert!(!seen[slot], "two lines share offset {slot}");
            seen[slot] = true;
        }
    }
}
