//! The offer to continue a game saved on quitting (`docs/mockups/starquake/offer.png`).

use crate::{
    colors,
    hints::{hints_from, Hint, PadLabels, Press},
    Align, Canvas, Color, Fonts, Line, Style, Weight,
};

/// The card's width, in logical pixels.
pub const WIDTH: u32 = 538;
/// The card's height, in logical pixels.
pub const HEIGHT: u32 = 168;

const CARD_EDGE: Color = Color::rgb(0x2B, 0x2E, 0x36);
const BODY: Color = Color::rgb(0xC4, 0xC7, 0xCE);

/// The offer, for a game by its title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The game's title, as the words name it.
    pub title: String,
    /// How the last pad used marks its buttons.
    pub pad: PadLabels,
}

impl Offer {
    /// Draws the card at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: f32) -> Canvas {
        let mut c = Canvas::new(WIDTH, HEIGHT, scale);
        #[allow(clippy::cast_precision_loss, reason = "a card's size")]
        let (w, h) = (WIDTH as f32, HEIGHT as f32);
        c.rect(
            (0.0, 0.0, w, h),
            12.0,
            Some(colors::CARD),
            Some(Line::solid(CARD_EDGE, 1.0)),
        );
        let heading = Style::new(21.0, colors::TEXT).weight(Weight::SemiBold);
        span(
            &mut c,
            fonts,
            "Continue where you left off?",
            21.0,
            &heading,
        );
        let body = Style::new(15.0, BODY);
        let words = format!(
            "The game you were playing when {} closed was saved.",
            self.title
        );
        span(&mut c, fonts, &words, 60.0, &body);
        c.rect((1.0, 117.0, w - 2.0, 1.0), 0.0, Some(CARD_EDGE), None);
        hints_from(
            &mut c,
            fonts,
            (27.0, 134.0),
            &[
                Hint {
                    press: &[Press::Key("Enter"), self.pad.confirm()],
                    does: "continue",
                },
                Hint {
                    press: &[Press::Key("Esc"), self.pad.back()],
                    does: "start afresh",
                },
            ],
        );
        c
    }
}

/// Words in a span on the mockup's 16-pixel line whose top is `top`, 27 pixels in.
fn span(c: &mut Canvas, fonts: &mut Fonts, words: &str, top: f32, style: &Style) {
    c.text(
        fonts,
        words,
        (27.0, top + Fonts::in_body_line(style)),
        Fonts::normal_line_height(style),
        Align::Left,
        style,
    );
}
