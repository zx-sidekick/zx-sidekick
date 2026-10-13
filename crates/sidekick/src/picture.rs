//! The Spectrum's picture on the computer's screen: its colors, and the largest whole scale.

use sidekick_core::Picture;
use sidekick_ui::{PANEL_HEIGHT, PANEL_WIDTH};

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

/// One of the Spectrum's 16 colors, as RGBA.
pub(crate) const fn color(index: u8) -> [u8; 4] {
    PALETTE[(index & 0x0F) as usize]
}

/// A part of the machine's picture, in Spectrum pixels: x, y, width, height.
pub(crate) type Part = (u32, u32, u32, u32);

/// The whole of the machine's picture: the screen with 32 pixels of border all round.
pub(crate) const FULL: Part = (0, 0, 320, 256);

/// The screen alone, inside the border.
pub(crate) const SCREEN: Part = (32, 32, 256, 192);

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

/// Where the picture and the panel go in the window, in device pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The picture: x, y, width, height.
    pub(crate) picture: (u32, u32, u32, u32),
    /// The panel, when there is one: x, y, width, height.
    pub(crate) panel: Option<(u32, u32, u32, u32)>,
    /// Device pixels to a Spectrum pixel: a whole number.
    pub(crate) pixel: u32,
}

impl Layout {
    /// Device pixels to a logical one, for what's drawn over the picture: the mockups draw
    /// the picture at three logical pixels to a Spectrum one.
    #[allow(clippy::cast_precision_loss, reason = "a small whole number")]
    pub(crate) fn scale(&self) -> f32 {
        self.pixel as f32 / LOGICAL_PIXEL as f32
    }
}

/// Logical pixels to a Spectrum pixel, on the mockups.
pub(crate) const LOGICAL_PIXEL: u32 = 3;

/// The smallest scale the picture is shown at beside the panel.
const LEAST_PIXEL: u32 = 2;

/// The smallest window for a picture `picture` Spectrum pixels large: the picture at twice its
/// size beside the panel at its design size, or, without the panel, the picture alone at
/// twice its size.
pub(crate) const fn smallest(picture: (u32, u32), panel: bool) -> (u32, u32) {
    let (w, h) = (picture.0 * LEAST_PIXEL, picture.1 * LEAST_PIXEL);
    if !panel {
        return (w, h);
    }
    let h = if h > PANEL_HEIGHT { h } else { PANEL_HEIGHT };
    (w + PANEL_WIDTH, h)
}

/// Where the picture (`picture`, its size in Spectrum pixels) and, with `panel`, the panel go
/// in a window of `output` device pixels. The panel takes the window's full height; its width
/// follows from its design's at the smallest of three scales: the window's height to the
/// design's, the one that leaves the picture's side at least as wide as the panel, and the one
/// that leaves the picture room for twice its size. So it shrinks with the window's width as
/// well as its height, never below its design size. The picture takes the largest whole scale
/// that fits what's left, centred there. A window smaller than the smallest ([`smallest`])
/// gets the picture alone.
pub(crate) fn layout(output: (u32, u32), picture: (u32, u32), panel: bool) -> Layout {
    let least = smallest(picture, panel);
    if !panel || output.0 < least.0 || output.1 < least.1 {
        let at = placement(output, picture);
        return Layout {
            picture: at,
            panel: None,
            pixel: at.2 / picture.0,
        };
    }
    let tall = (PANEL_WIDTH * output.1 + PANEL_HEIGHT / 2) / PANEL_HEIGHT;
    let width = tall
        .min(output.0 / 2)
        .min(output.0 - picture.0 * LEAST_PIXEL)
        .max(PANEL_WIDTH);
    let rest = output.0 - width;
    let at = placement((rest, output.1), picture);
    Layout {
        picture: at,
        panel: Some((rest, 0, width, output.1)),
        pixel: at.2 / picture.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN_SIZE: (u32, u32) = (SCREEN.2, SCREEN.3);
    const FULL_SIZE: (u32, u32) = (FULL.2, FULL.3);

    #[test]
    fn the_panel_takes_the_window_s_height_and_the_picture_a_whole_scale_in_the_rest() {
        let wide = layout((1920, 1080), SCREEN_SIZE, true);
        assert_eq!(wide.panel, Some((1346, 0, 574, 1080)), "408 × 1080 / 768");
        assert_eq!(wide.pixel, 5);
        assert_eq!(wide.picture, (33, 60, 1280, 960), "centred in the rest");
        let laptop = layout((1440, 900), SCREEN_SIZE, true);
        assert_eq!(laptop.panel, Some((962, 0, 478, 900)));
        assert_eq!(laptop.pixel, 3);
        assert_eq!(laptop.picture, (97, 162, 768, 576));
        let bordered = layout((1920, 1080), FULL_SIZE, true);
        assert_eq!(bordered.panel, Some((1346, 0, 574, 1080)));
        assert_eq!(bordered.pixel, 4, "the border costs a step");
        assert_eq!(bordered.picture, (33, 28, 1280, 1024));
        let retina = layout((2940, 1912), SCREEN_SIZE, true);
        assert_eq!(retina.panel, Some((1924, 0, 1016, 1912)));
        assert_eq!(retina.pixel, 7);
        assert_eq!(retina.picture, (66, 284, 1792, 1344));
    }

    #[test]
    fn the_smallest_window_is_a_2x_picture_beside_the_panel_at_its_design_size() {
        assert_eq!(smallest(SCREEN_SIZE, true), (920, 768));
        assert_eq!(smallest(FULL_SIZE, true), (1048, 768));
        assert_eq!(
            smallest(SCREEN_SIZE, false),
            (512, 384),
            "no panel: the 2x picture"
        );
        assert_eq!(smallest(FULL_SIZE, false), (640, 512));
        let least = layout((920, 768), SCREEN_SIZE, true);
        assert_eq!(least.panel, Some((512, 0, 408, 768)));
        assert_eq!((least.picture, least.pixel), ((0, 192, 512, 384), 2));
        let bordered = layout((1048, 768), FULL_SIZE, true);
        assert_eq!(bordered.panel, Some((640, 0, 408, 768)));
        assert_eq!((bordered.picture, bordered.pixel), ((0, 128, 640, 512), 2));
    }

    #[test]
    fn a_tall_narrow_window_keeps_the_picture_at_2x_and_narrows_the_panel() {
        let tall = layout((1000, 1400), SCREEN_SIZE, true);
        assert_eq!(tall.panel, Some((512, 0, 488, 1400)));
        assert_eq!((tall.picture, tall.pixel), ((0, 508, 512, 384), 2));
    }

    #[test]
    fn a_narrower_window_narrows_the_panel_too_never_wider_than_the_picture_s_side() {
        // Tall enough for a 744-wide panel; half the width is 550.
        let narrow = layout((1100, 1400), SCREEN_SIZE, true);
        assert_eq!(narrow.panel, Some((550, 0, 550, 1400)));
        assert_eq!((narrow.picture, narrow.pixel), ((19, 508, 512, 384), 2));
        let wider = layout((1500, 1400), SCREEN_SIZE, true);
        assert_eq!(wider.panel, Some((756, 0, 744, 1400)), "the height's own");
        assert_eq!(wider.pixel, 2);
    }

    #[test]
    fn a_shorter_window_shrinks_both() {
        let tall = layout((2560, 1440), SCREEN_SIZE, true);
        assert_eq!(tall.panel, Some((1795, 0, 765, 1440)));
        assert_eq!(tall.pixel, 7);
        let short = layout((2560, 800), SCREEN_SIZE, true);
        assert_eq!(short.panel, Some((2135, 0, 425, 800)));
        assert_eq!((short.picture, short.pixel), ((555, 16, 1024, 768), 4));
    }

    #[test]
    fn smaller_than_the_smallest_or_without_the_panel_the_picture_is_alone() {
        let small = layout((919, 768), SCREEN_SIZE, true);
        assert_eq!(small.panel, None);
        assert_eq!((small.picture, small.pixel), ((75, 96, 768, 576), 3));
        let short = layout((1920, 767), SCREEN_SIZE, true);
        assert_eq!(short.panel, None);
        let without = layout((1920, 1080), SCREEN_SIZE, false);
        assert_eq!(
            (without.picture, without.panel, without.pixel),
            ((320, 60, 1280, 960), None, 5)
        );
        assert!(
            (without.scale() - 5.0 / 3.0).abs() < f32::EPSILON,
            "three Spectrum pixels to a logical one"
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
