//! The pause notice: a card over the dimmed picture while the game is paused
//! (`docs/mockups/starquake/paused.png`).

use crate::{
    colors,
    hints::{hints_from, Face, Hint, PadLabels, Press},
    Align, Canvas, Color, Fonts, Line, Style, Weight,
};

/// The card's size, in logical pixels.
pub const WIDTH: u32 = 438;
/// The card's height, in logical pixels.
pub const HEIGHT: u32 = 158;

/// Where the card sits on the picture's 960 × 768 logical pixels: its top left.
pub const AT: (u32, u32) = (261, 305);

const CARD_EDGE: Color = Color::rgb(0x2B, 0x2E, 0x36);
const BODY: Color = Color::rgb(0xC4, 0xC7, 0xCE);

/// What to press on a pad, for a hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadMark {
    /// A face button, marked as the pad's maker marks it.
    Face(Face),
    /// The D-pad or the stick, as arrows.
    Directions,
}

/// One thing to press, and what it does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeHint {
    /// The keyboard's key as shown, or arrows.
    pub keys: Option<String>,
    /// The pad's part.
    pub pad: Option<PadMark>,
    /// What it does.
    pub does: String,
}

/// The pause notice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PauseNotice {
    /// Its heading: "Paused".
    pub heading: String,
    /// Its line.
    pub text: String,
    /// What to press.
    pub hints: Vec<NoticeHint>,
    /// How the last pad used marks its buttons.
    pub pad: PadLabels,
}

impl Default for PauseNotice {
    /// The notice for a game whose definition doesn't word its own.
    fn default() -> Self {
        Self {
            heading: "Paused".to_owned(),
            text: "Press a key or a button to go on.".to_owned(),
            hints: Vec::new(),
            pad: PadLabels::default(),
        }
    }
}

/// Whether keys are only arrows (and spaces), drawn bare rather than in a badge.
fn arrows(keys: &str) -> bool {
    keys.chars()
        .all(|c| matches!(c, '←' | '↑' | '→' | '↓' | ' '))
}

impl PauseNotice {
    /// Draws the card at `scale` device pixels to a logical one; outside its rounded corners
    /// stays clear.
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
        let title = Style::new(24.0, colors::TEXT).weight(Weight::Bold);
        c.text(
            fonts,
            &self.heading,
            (27.0, 22.0),
            Fonts::normal_line_height(&title),
            Align::Left,
            &title,
        );
        // A 15-pixel span on the mockup's 16-pixel line.
        let body = Style::new(15.0, BODY);
        c.text(
            fonts,
            &self.text,
            (27.0, 60.0 + Fonts::in_body_line(&body)),
            Fonts::normal_line_height(&body),
            Align::Left,
            &body,
        );
        c.rect((1.0, 107.0, w - 2.0, 1.0), 0.0, Some(CARD_EDGE), None);

        let presses: Vec<Vec<Press>> = self
            .hints
            .iter()
            .map(|hint| {
                let keys = hint.keys.as_deref().map(|k| {
                    if arrows(k) {
                        Press::Arrows(k)
                    } else {
                        Press::Key(k)
                    }
                });
                let pad = hint.pad.and_then(|mark| match mark {
                    PadMark::Face(face) => Some(self.pad.face(face)),
                    // Arrows on the keyboard stand for the D-pad too.
                    PadMark::Directions if keys.is_some() => None,
                    PadMark::Directions => Some(Press::Arrows("← ↑ → ↓")),
                });
                keys.into_iter().chain(pad).collect()
            })
            .collect();
        let list: Vec<Hint> = self
            .hints
            .iter()
            .zip(&presses)
            .map(|(hint, press)| Hint {
                press,
                does: &hint.does,
            })
            .collect();
        hints_from(&mut c, fonts, (27.0, 124.0), &list);
        c
    }
}
