//! The notice the window draws over the picture while the game is paused,
//! which is the emulation frozen (`freeze.rs`): that it is paused, and how
//! to go on.
//!
//! Drawn to the approved mockup on #20, in the language of the guidance
//! designs: the picture dimmed behind, a rounded card, a title, one
//! sentence, and a legend of bare arrows for move and a key or a pad button
//! for fire. Nothing is pressed or changed for the game: display only. It
//! is part of the overlay (`overlay.rs`), laid out in its units, and dims
//! the picture only, not the panel beside it.

use super::overlay::{HEIGHT, PICTURE_W};
use super::text::{ARROWS, Canvas, Fonts, Span, Weight, palette};

/// How much the picture is darkened behind the card, out of 255.
const DIM: u8 = 158;

/// The card's size.
const CARD_W: f32 = 440.0;
const CARD_H: f32 = 160.0;

/// Dims the picture and lays the card over its middle.
pub fn draw(fonts: &mut Fonts, canvas: &mut Canvas, layout: super::gamepad::Layout) {
    canvas.shade(0.0, 0.0, PICTURE_W, HEIGHT, [0, 0, 0], DIM);
    let cx = (PICTURE_W - CARD_W) / 2.0;
    let cy = (HEIGHT - CARD_H) / 2.0;
    canvas.round_rect(cx, cy, CARD_W, CARD_H, 12.0, palette::CARD);
    canvas.outline(cx, cy, CARD_W, CARD_H, 12.0, 1.5, None, palette::LINE);
    let pad = 28.0;
    fonts.text(
        Some(canvas),
        cx + pad,
        cy + 22.0,
        None,
        1.0,
        &[Span {
            text: "Paused",
            size: 24.0,
            weight: Weight::SemiBold,
            colour: palette::TITLE,
        }],
    );
    fonts.text(
        Some(canvas),
        cx + pad,
        cy + 60.0,
        None,
        1.0,
        &[Span {
            text: "Move or fire to continue.",
            size: 16.0,
            weight: Weight::Regular,
            colour: palette::MUTED,
        }],
    );
    let line_y = cy + CARD_H - 58.0;
    canvas.round_rect(cx, line_y, CARD_W, 1.5, 0.0, palette::LINE);
    // The legend: arrows for move, a key or a pad button for fire.
    let h = 26.0;
    let by = cy + CARD_H - 42.0;
    let mut x = cx + pad;
    x += fonts.arrows(canvas, x, by, h, &ARROWS) + 12.0;
    x += fonts.word(canvas, x, by, h, "move") + 26.0;
    x += fonts.key_badge(canvas, x, by, h, "Ctrl") + 8.0;
    x += fonts.word(canvas, x, by, h, "or") + 8.0;
    // The firing button is the west one on every pad; its letter is not
    // (#101): X on an Xbox pad, Y on a Nintendo one, the square on a
    // PlayStation one.
    x += match layout {
        super::gamepad::Layout::PlayStation => {
            fonts.mark_badge(canvas, x, by, h, super::text::PadMark::Square)
        }
        super::gamepad::Layout::Nintendo => fonts.button_badge(canvas, x, by, h, "Y"),
        super::gamepad::Layout::Xbox => fonts.button_badge(canvas, x, by, h, "X"),
    } + 10.0;
    fonts.word(canvas, x, by, h, "fire");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::overlay::WIDTH;

    /// The overlay with the notice drawn, laid over a picture of one colour
    /// and a black panel, at `scale` device pixels per unit.
    fn render(scale: f32, rgb: [u8; 3]) -> (Vec<u8>, usize, usize) {
        let (w, h) = ((WIDTH * scale) as usize, (HEIGHT * scale) as usize);
        let mut overlay = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut overlay,
            width: w,
            height: h,
            scale,
        };
        canvas.clear_transparent();
        draw(
            &mut Fonts::load(),
            &mut canvas,
            crate::frontend::gamepad::Layout::Xbox,
        );
        let picture_w = (PICTURE_W * scale) as usize;
        let mut out = vec![0u8; w * h * 4];
        for i in 0..w * h {
            let under = if i % w < picture_w { rgb } else { [0; 3] };
            let over = &overlay[i * 4..i * 4 + 4];
            for c in 0..3 {
                out[i * 4 + c] =
                    over[c] + (u16::from(under[c]) * u16::from(255 - over[3]) / 255) as u8;
            }
            out[i * 4 + 3] = 0xFF;
        }
        (out, w, h)
    }

    #[test]
    fn the_picture_is_dimmed_and_the_card_sits_in_its_middle() {
        let (pixels, w, _) = render(1.0, [200, 200, 200]);
        let at = |x: usize, y: usize| &pixels[(y * w + x) * 4..(y * w + x) * 4 + 3];
        let corner = at(1, 1);
        assert!(
            corner[0] < 100 && corner[0] > 40,
            "dimmed, not black: {corner:?}"
        );
        assert_eq!(at(1100, 1), &[0, 0, 0], "the panel is not dimmed over");
        assert_eq!(at(480, 364), &palette::CARD, "the card's body");
        // The title is drawn in white somewhere in the card's top left.
        let title_area = (0..40usize)
            .flat_map(|dy| (0..120usize).map(move |dx| (dx, dy)))
            .any(|(dx, dy)| at(280 + dx, 324 + dy)[0] > 180);
        assert!(title_area, "no title drawn");
    }

    /// Draws the notice over a stand-in picture to a PNG in the folder
    /// `SQ_NOTICE_PNG` names, for comparing with the mockup without a
    /// window. Does nothing when it is not set.
    #[test]
    fn render_to_png() {
        let Some(out) = std::env::var_os("SQ_NOTICE_PNG") else {
            return;
        };
        let (pixels, w, h) = render(1.0, [0xd7, 0, 0xd7]);
        let rgb: Vec<u32> = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
            .collect();
        std::fs::write(
            std::path::PathBuf::from(out).join("notice.png"),
            zx_core::png::encode(&rgb, w, h),
        )
        .unwrap();
    }
}
