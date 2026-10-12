//! The Spectrum's picture on the computer's screen: its colors, and the largest whole scale.

use sidekick_core::Picture;

/// The Spectrum's 16 colors as RGBA: 0–7 at normal brightness, 8–15 bright.
const PALETTE: [[u8; 4]; 16] = {
    const N: u8 = 0xD7;
    const B: u8 = 0xFF;
    [
        [0, 0, 0, 255],
        [0, 0, N, 255],
        [N, 0, 0, 255],
        [N, 0, N, 255],
        [0, N, 0, 255],
        [0, N, N, 255],
        [N, N, 0, 255],
        [N, N, N, 255],
        [0, 0, 0, 255],
        [0, 0, B, 255],
        [B, 0, 0, 255],
        [B, 0, B, 255],
        [0, B, 0, 255],
        [0, B, B, 255],
        [B, B, 0, 255],
        [B, B, B, 255],
    ]
};

/// Writes the picture into `rgba`, four bytes a pixel, row by row.
pub(crate) fn to_rgba(picture: &Picture, rgba: &mut Vec<u8>) {
    rgba.clear();
    rgba.extend(
        picture
            .pixels()
            .iter()
            .flat_map(|&color| PALETTE[usize::from(color & 0x0F)]),
    );
}

/// Where the picture goes in an output of this size: the largest whole scale that fits (at
/// least 1), centered. Returns x, y, width and height in output pixels.
#[must_use]
pub(crate) fn placement(output: (u32, u32), picture: (u32, u32)) -> (u32, u32, u32, u32) {
    let scale = (output.0 / picture.0).min(output.1 / picture.1).max(1);
    let (w, h) = (picture.0 * scale, picture.1 * scale);
    (
        output.0.saturating_sub(w) / 2,
        output.1.saturating_sub(h) / 2,
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_largest_whole_scale_that_fits_centered() {
        // A 1368×768 window: ×3 (960×768), with 204 pixels either side.
        assert_eq!(placement((1368, 768), (320, 256)), (204, 0, 960, 768));
        // A Retina 2560×1600: ×6 (1920×1536).
        assert_eq!(placement((2560, 1600), (320, 256)), (320, 32, 1920, 1536));
        // Smaller than the picture: still ×1, from the corner.
        assert_eq!(placement((300, 200), (320, 256)), (0, 0, 320, 256));
    }

    #[test]
    fn colors_are_the_spectrum_s() {
        let mut machine = sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).unwrap();
        machine.run_frame().unwrap();
        let mut rgba = Vec::new();
        to_rgba(machine.picture(), &mut rgba);
        assert_eq!(rgba.len(), 320 * 256 * 4);
        assert!(rgba
            .chunks_exact(4)
            .all(|p| PALETTE.contains(&[p[0], p[1], p[2], p[3]])));
    }
}
