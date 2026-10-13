//! The text-entry card: choosing a word from a kept list, or spelling one, with a pad
//! (`docs/mockups/starquake/booth-choose.png`, `booth-spell.png`).

use crate::{
    colors,
    hints::{hints_from, Face, Hint, PadLabels, Press},
    Align, Canvas, Color, Fonts, Line, Style, Weight,
};

/// The card's size, in logical pixels.
pub const WIDTH: u32 = 458;
/// The card's height, in logical pixels.
pub const HEIGHT: u32 = 162;

const CARD_EDGE: Color = Color::rgb(0x2B, 0x2E, 0x36);
const BOX_EDGE: Color = Color::rgb(0x50, 0x54, 0x5E);
const BOX: Color = Color::rgb(0x1C, 0x1F, 0x26);
const LIT: Color = Color::rgb(0x23, 0x2A, 0x38);

/// Which way the card is working.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryMode {
    /// Choosing one of the kept words: which, of how many.
    Choosing {
        /// The chosen one, from 0.
        at: usize,
        /// How many there are.
        of: usize,
    },
    /// Spelling, the slot being changed.
    Spelling {
        /// The slot.
        slot: usize,
    },
}

/// The text-entry card, as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryCard {
    /// Its title when choosing.
    pub title_choose: String,
    /// Its title when spelling.
    pub title_spell: String,
    /// Each slot's letter; `None` for one left empty.
    pub letters: Vec<Option<char>>,
    /// Choosing or spelling.
    pub mode: EntryMode,
    /// Whether there are words to choose from, so spelling can go back to them.
    pub can_choose: bool,
    /// How the last pad used marks its buttons.
    pub pad: PadLabels,
}

impl EntryCard {
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
        let title = Style::new(17.0, colors::TEXT).weight(Weight::Medium);
        let words = match self.mode {
            EntryMode::Choosing { .. } => &self.title_choose,
            EntryMode::Spelling { .. } => &self.title_spell,
        };
        c.text(
            fonts,
            words,
            (23.0, 18.0),
            Fonts::normal_line_height(&title),
            Align::Left,
            &title,
        );
        if let EntryMode::Choosing { at, of } = self.mode {
            // A 13-pixel span on the mockup's 16-pixel line.
            let count = Style::new(13.0, colors::MUTED);
            c.text(
                fonts,
                &format!("{} of {of}", at + 1),
                (398.0, 21.0 + Fonts::in_body_line(&count)),
                Fonts::normal_line_height(&count),
                Align::Left,
                &count,
            );
        }
        self.slots(&mut c, fonts);
        c.rect((1.0, 119.0, w - 2.0, 1.0), 0.0, Some(CARD_EDGE), None);
        self.hints(&mut c, fonts);
        c
    }

    fn slots(&self, c: &mut Canvas, fonts: &mut Fonts) {
        #[allow(clippy::cast_precision_loss, reason = "a few slots")]
        let width = self.letters.len() as f32 * 58.0 - 10.0;
        #[allow(clippy::cast_precision_loss, reason = "a card's size")]
        let left = ((WIDTH as f32 - width) / 2.0).round();
        let letter = Style::new(24.0, colors::TEXT).weight(Weight::Medium);
        for (i, slot) in self.letters.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a few slots")]
            let x = left + i as f32 * 58.0;
            let (fill, edge) = match self.mode {
                EntryMode::Choosing { .. } => (BOX, colors::ACCENT),
                EntryMode::Spelling { slot } if slot == i => (LIT, colors::ACCENT),
                EntryMode::Spelling { .. } => (BOX, BOX_EDGE),
            };
            c.rect(
                (x, 47.0, 48.0, 52.0),
                6.0,
                Some(fill),
                Some(Line::solid(edge, 1.0)),
            );
            if let Some(ch) = slot {
                c.text(
                    fonts,
                    &ch.to_string(),
                    (x + 24.0, 47.0),
                    52.0,
                    Align::Center,
                    &letter,
                );
            }
        }
    }

    fn hints(&self, c: &mut Canvas, fonts: &mut Fonts) {
        let switch = [self.pad.face(Face::West)];
        let enter = [self.pad.confirm()];
        let clear = [self.pad.back()];
        let list: Vec<Hint> = match self.mode {
            EntryMode::Choosing { .. } => vec![
                Hint {
                    press: &[Press::Arrows("↑←")],
                    does: "previous",
                },
                Hint {
                    press: &[Press::Arrows("→↓")],
                    does: "next",
                },
                Hint {
                    press: &enter,
                    does: "enter",
                },
                Hint {
                    press: &switch,
                    does: "spell it",
                },
            ],
            EntryMode::Spelling { .. } => {
                let mut list = vec![
                    Hint {
                        press: &[Press::Arrows("↑↓")],
                        does: "letter",
                    },
                    Hint {
                        press: &[Press::Arrows("←→")],
                        does: "place",
                    },
                    Hint {
                        press: &enter,
                        does: "enter",
                    },
                    Hint {
                        press: &clear,
                        does: "clear",
                    },
                ];
                if self.can_choose {
                    list.push(Hint {
                        press: &switch,
                        does: "choose",
                    });
                }
                list
            }
        };
        hints_from(c, fonts, (23.0, 132.0), &list);
    }
}
