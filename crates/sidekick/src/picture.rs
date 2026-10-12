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

/// The picture and the panel, laid out as the mockups are: the picture at three logical
/// pixels to the Spectrum's one, and the panel 408 logical pixels wide beside it, 1,368 × 768
/// in all, centred at the largest whole scale that fits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The picture: x, y, width, height in device pixels.
    pub(crate) picture: (u32, u32, u32, u32),
    /// The panel, when there is one.
    pub(crate) panel: Option<(u32, u32, u32, u32)>,
    /// Device pixels to a logical one, for what's drawn over the picture.
    pub(crate) scale: u32,
}

/// The logical width of the picture and of the panel beside it.
const PICTURE_WIDTH: u32 = 960;
const PANEL_WIDTH: u32 = 408;
const HEIGHT: u32 = 768;

/// Where the picture (`picture`, its size in Spectrum pixels) and, with `panel`, the panel go
/// in a window of `output` device pixels.
pub(crate) fn layout(output: (u32, u32), picture: (u32, u32), panel: bool) -> Layout {
    if !panel {
        let picture = placement(output, picture);
        let scale = (picture.2 / PICTURE_WIDTH).max(1);
        return Layout {
            picture,
            panel: None,
            scale,
        };
    }
    let width = PICTURE_WIDTH + PANEL_WIDTH;
    let scale = (output.0 / width).min(output.1 / HEIGHT).max(1);
    let x = output.0.saturating_sub(width * scale) / 2;
    let y = output.1.saturating_sub(HEIGHT * scale) / 2;
    Layout {
        picture: (x, y, PICTURE_WIDTH * scale, HEIGHT * scale),
        panel: Some((
            x + PICTURE_WIDTH * scale,
            y,
            PANEL_WIDTH * scale,
            HEIGHT * scale,
        )),
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_the_panel_both_go_at_the_largest_whole_scale_that_fits() {
        let at_1 = layout((1368, 768), (320, 256), true);
        assert_eq!(at_1.picture, (0, 0, 960, 768));
        assert_eq!(at_1.panel, Some((960, 0, 408, 768)));
        assert_eq!(at_1.scale, 1);
        let retina = layout((2940, 1912), (320, 256), true);
        assert_eq!(retina.scale, 2);
        assert_eq!(retina.picture, (102, 188, 1920, 1536));
        assert_eq!(retina.panel, Some((2022, 188, 816, 1536)));
        let without = layout((2560, 1600), (320, 256), false);
        assert_eq!(
            (without.picture, without.panel, without.scale),
            ((320, 32, 1920, 1536), None, 2)
        );
    }

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
