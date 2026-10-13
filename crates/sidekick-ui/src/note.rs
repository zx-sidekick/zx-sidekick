//! A short notice over the picture: "Game saved" (`docs/mockups/starquake/saved-note.png`).

use crate::{colors, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// The pill's height, in logical pixels, for a line of words.
pub const HEIGHT: u32 = 35;
/// The widest a pill grows before its words wrap onto another line.
const MOST_WIDTH: u32 = 640;
/// What each line after the first adds to a pill's height.
const LINE: u32 = 20;
/// The room either side of the words: the mockup's 128-pixel pill round "Game saved".
const SIDES: f32 = 45.0;
/// Where a one-line pill sits on the picture's 960 × 768 logical pixels: its centre, and its
/// top. A pill of more lines grows upwards from the same foot.
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
    /// Its words, broken into lines that fit the widest pill: one line when they fit.
    fn lines(&self, fonts: &mut Fonts) -> Vec<String> {
        #[allow(clippy::cast_precision_loss, reason = "a pill's width")]
        let room = MOST_WIDTH as f32 - SIDES;
        if fonts.measure(&self.text, &TEXT) <= room {
            return vec![self.text.clone()];
        }
        let mut lines: Vec<String> = Vec::new();
        for word in self.text.split_whitespace() {
            if let Some(line) = lines.last_mut() {
                let longer = format!("{line} {word}");
                if fonts.measure(&longer, &TEXT) <= room {
                    *line = longer;
                    continue;
                }
            }
            lines.push(word.to_owned());
        }
        lines
    }

    /// The pill's width, in logical pixels: its widest line, with room either side.
    #[must_use]
    pub fn width(&self, fonts: &mut Fonts) -> u32 {
        let widest = self
            .lines(fonts)
            .iter()
            .map(|line| fonts.measure(line, &TEXT))
            .fold(0.0, f32::max);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a few words' width"
        )]
        let width = (widest + SIDES).round() as u32;
        width
    }

    /// The pill's height, in logical pixels: a line's, and more for each line after.
    #[must_use]
    pub fn height(&self, fonts: &mut Fonts) -> u32 {
        let extra = u32::try_from(self.lines(fonts).len().saturating_sub(1)).unwrap_or(0);
        HEIGHT + extra * LINE
    }

    /// Draws the pill at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let (width, height) = (self.width(fonts), self.height(fonts));
        let mut c = Canvas::new(width, height, scale);
        #[allow(clippy::cast_precision_loss, reason = "a pill's size")]
        let (w, h, one, line) = (width as f32, height as f32, HEIGHT as f32, LINE as f32);
        c.rect(
            (0.0, 0.0, w, h),
            one / 2.0,
            Some(BG),
            Some(Line::solid(EDGE, 1.0)),
        );
        let lines = self.lines(fonts);
        if let [only] = lines.as_slice() {
            c.text(fonts, only, (w / 2.0, 0.0), one, Align::Center, &TEXT);
        } else {
            // A line's height each, the first where a one-line pill's words sit.
            let top = (one - line) / 2.0;
            for (i, text) in lines.iter().enumerate() {
                #[allow(clippy::cast_precision_loss, reason = "a few lines")]
                let y = top + i as f32 * line;
                c.text(fonts, text, (w / 2.0, y), line, Align::Center, &TEXT);
            }
        }
        c
    }

    /// How long it shows, in seconds: two, and one more for every 30 characters past the
    /// first 30, up to ten, so a reason can be read.
    #[must_use]
    pub fn seconds(&self) -> u64 {
        let extra = u64::try_from(self.text.chars().count().saturating_sub(30) / 30).unwrap_or(8);
        (2 + extra).min(10)
    }
}
