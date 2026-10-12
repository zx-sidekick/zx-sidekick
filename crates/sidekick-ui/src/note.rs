//! A short notice over the picture: "Game saved" (`docs/mockups/starquake/saved-note.png`).

use crate::{colors, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// The pill's height, in logical pixels.
pub const HEIGHT: u32 = 35;
/// Where the pill sits on the picture's 960 × 768 logical pixels: its centre, and its top.
pub const AT: (u32, u32) = (480, 692);

const BG: Color = Color::rgb(0x1B, 0x1D, 0x23);
const EDGE: Color = Color::rgb(0x2F, 0x32, 0x3A);
const TEXT: Style = Style::new(14.0, colors::TEXT).weight(Weight::Medium);

/// A notice: a few words in a pill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// What it says.
    pub text: String,
}

impl Note {
    /// The pill's width, in logical pixels: its words, with the mockup's room either side.
    #[must_use]
    pub fn width(&self, fonts: &mut Fonts) -> u32 {
        // 22.5 either side: the mockup's 128-pixel pill round "Game saved".
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a few words' width"
        )]
        let width = (fonts.measure(&self.text, &TEXT) + 45.0).round() as u32;
        width
    }

    /// Draws the pill at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let width = self.width(fonts);
        let mut c = Canvas::new(width, HEIGHT, scale);
        #[allow(clippy::cast_precision_loss, reason = "a pill's size")]
        let (w, h) = (width as f32, HEIGHT as f32);
        c.rect(
            (0.0, 0.0, w, h),
            h / 2.0,
            Some(BG),
            Some(Line::solid(EDGE, 1.0)),
        );
        c.text(fonts, &self.text, (w / 2.0, 0.0), h, Align::Center, &TEXT);
        c
    }
}
