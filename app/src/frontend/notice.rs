//! The notice the window draws over the picture while the game is paused:
//! that it is paused, and how to go on. The game resumes on any direction
//! or fire, never on its pause key, and a player who pressed Start expecting
//! Start to resume would otherwise be stuck.
//!
//! Drawn to the approved mockup on #20, in the language of the guidance
//! designs: the picture dimmed behind, a rounded card, a title, one
//! sentence, and a legend of bare arrows for move and a key or a pad button
//! for fire. Nothing is pressed or changed for the game: display only.

use super::text::{Canvas, Fonts, Span, Weight, palette};
use super::video::{FULL_H, FULL_W};

/// How much the picture is darkened behind the card, out of 255.
const DIM: u8 = 158;

/// The card's size at the mockup's scale, in logical pixels; it scales
/// with the picture.
const CARD_W: f32 = 440.0;
const CARD_H: f32 = 160.0;

pub struct Notice {
    fonts: Fonts,
}

impl Notice {
    pub fn new() -> Notice {
        Notice {
            fonts: Fonts::load(),
        }
    }

    /// Draws the paused game's `picture`, the Spectrum's screen with its
    /// border as RGBA, into `canvas` at the largest whole number of device
    /// pixels per picture pixel that fits, centred, then dims it and lays the
    /// card over it. The card is sized for the mockup's window, the picture
    /// three times its own size, and grows or shrinks with the picture.
    pub fn draw(&mut self, canvas: &mut Canvas, picture: &[u8]) {
        canvas.clear([0, 0, 0]);
        let k = (canvas.width / FULL_W).min(canvas.height / FULL_H).max(1);
        let (x, y) = (
            (canvas.width - FULL_W * k) / 2,
            (canvas.height - FULL_H * k) / 2,
        );
        canvas.blit_scaled(picture, FULL_W, FULL_H, x, y, k);
        canvas.dim(DIM);
        // Logical pixels from here on, scaled so the card matches the mockup
        // when the picture is three times its size.
        let u = k as f32 / canvas.scale / 3.0;
        let (cw, ch) = (CARD_W * u, CARD_H * u);
        let cx = (canvas.width as f32 / canvas.scale - cw) / 2.0;
        let cy = (canvas.height as f32 / canvas.scale - ch) / 2.0;
        canvas.round_rect(cx, cy, cw, ch, 12.0 * u, palette::CARD);
        canvas.outline(cx, cy, cw, ch, 12.0 * u, 1.5, None, palette::LINE);
        let pad = 28.0 * u;
        self.fonts.text(
            Some(canvas),
            cx + pad,
            cy + 22.0 * u,
            None,
            1.0,
            &[Span {
                text: "Paused",
                size: 24.0 * u,
                weight: Weight::SemiBold,
                colour: palette::TITLE,
            }],
        );
        self.fonts.text(
            Some(canvas),
            cx + pad,
            cy + 60.0 * u,
            None,
            1.0,
            &[Span {
                text: "Move or fire to continue.",
                size: 16.0 * u,
                weight: Weight::Regular,
                colour: palette::MUTED,
            }],
        );
        let line_y = cy + ch - 58.0 * u;
        canvas.round_rect(cx, line_y, cw, 1.5, 0.0, palette::LINE);
        // The legend: arrows for move, a key or a pad button for fire.
        let h = 26.0 * u;
        let by = cy + ch - 42.0 * u;
        let mut x = cx + pad;
        x += self.fonts.arrows(canvas, x, by, h) + 12.0 * u;
        x += self.word(canvas, x, by, h, u, "move") + 26.0 * u;
        x += self.fonts.key_badge(canvas, x, by, h, "Ctrl") + 8.0 * u;
        x += self.word(canvas, x, by, h, u, "or") + 8.0 * u;
        x += self.fonts.button_badge(canvas, x, by, h, "X") + 10.0 * u;
        self.word(canvas, x, by, h, u, "fire");
    }

    /// A word of the legend, muted, centred on the badges' height; returns
    /// its width.
    fn word(&mut self, canvas: &mut Canvas, x: f32, y: f32, h: f32, u: f32, text: &str) -> f32 {
        let size = 16.0 * u;
        let span = Span {
            text,
            size,
            weight: Weight::Regular,
            colour: palette::MUTED,
        };
        let ty = y + (h - size * 1.21) / 2.0;
        self.fonts.text(Some(canvas), x, ty, None, 1.0, &[span]).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture the size of the screen with its border, all one colour.
    fn picture(rgb: [u8; 3]) -> Vec<u8> {
        [rgb[0], rgb[1], rgb[2], 0xFF].repeat(FULL_W * FULL_H)
    }

    fn render(w: usize, h: usize, scale: f32, rgb: [u8; 3]) -> Vec<u8> {
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale,
        };
        Notice::new().draw(&mut canvas, &picture(rgb));
        pixels
    }

    #[test]
    fn the_picture_is_dimmed_and_the_card_sits_in_the_middle() {
        let (w, h) = (FULL_W * 3, FULL_H * 3);
        let pixels = render(w, h, 1.0, [200, 200, 200]);
        let at = |x: usize, y: usize| &pixels[(y * w + x) * 4..(y * w + x) * 4 + 3];
        let corner = at(1, 1);
        assert!(
            corner[0] < 100 && corner[0] > 40,
            "dimmed, not black: {corner:?}"
        );
        assert_eq!(at(w / 2, h / 2 - 20), &palette::CARD, "the card's body");
        // The title is drawn in white somewhere in the card's top left.
        let title_area = (0..40usize)
            .flat_map(|dy| (0..120usize).map(move |dx| (dx, dy)))
            .any(|(dx, dy)| at(w / 2 - 200 + dx, h / 2 - 60 + dy)[0] > 180);
        assert!(title_area, "no title drawn");
    }

    #[test]
    fn a_window_that_does_not_fit_three_times_keeps_whole_pixels() {
        // Twice the picture and a bit: the picture is drawn at twice its
        // size, centred, with black around it.
        let (w, h) = (FULL_W * 2 + 10, FULL_H * 2 + 6);
        let pixels = render(w, h, 1.0, [255, 255, 255]);
        let at = |x: usize, y: usize| pixels[(y * w + x) * 4];
        assert_eq!(at(0, 0), 0, "the margin is black");
        assert!(at(5, 3) > 0, "the picture starts where the margin ends");
    }

    /// Draws the notice over a real screenshot to a PNG in the folder
    /// `SQ_NOTICE_PNG` names, for comparing with the mockup without a
    /// window. Does nothing when it is not set.
    #[test]
    fn render_to_png() {
        let Some(out) = std::env::var_os("SQ_NOTICE_PNG") else {
            return;
        };
        let (w, h) = (FULL_W * 3, FULL_H * 3);
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale: 1.0,
        };
        let mut shot = picture([0, 0, 0]);
        // A grey block where the picture would be, so the dimming shows.
        for p in shot
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .skip(FULL_W * 40)
            .take(FULL_W * 150)
        {
            *p = [0xd7, 0, 0xd7, 0xff];
        }
        Notice::new().draw(&mut canvas, &shot);
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
