//! Key hints along a screen's bottom: keys in squarish badges, pad buttons in round ones,
//! arrows bare.

use crate::{colors, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// One thing a hint shows to press.
#[derive(Clone, Copy, Debug)]
pub enum Press<'a> {
    /// A keyboard key, in a squarish badge: `Enter`, `Esc`, `W`.
    Key(&'a str),
    /// A pad button, in a round badge: `A`, `B`, `X`.
    Pad(&'a str),
    /// Arrows, bare: `↑↓`, `←→`.
    Arrows(&'a str),
    /// A pad's middle button, in a pill, round at both ends: `Start`, `Options`, `+`.
    Pill(&'a str),
    /// A PlayStation pad's × button, in a round badge.
    Cross,
    /// A PlayStation pad's ○ button, in a round badge.
    Circle,
    /// A PlayStation pad's □ button, in a round badge.
    Square,
    /// A PlayStation pad's △ button, in a round badge.
    Triangle,
    /// A Nintendo pad's − button, in a round badge.
    Minus,
}

/// A pad's face button, by position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// Left.
    West,
    /// Bottom.
    South,
    /// Right.
    East,
    /// Top.
    North,
}

/// What a pad's buttons are marked with, by its maker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadLabels {
    /// Xbox's letters, and most others': A at the bottom confirms, B on the right goes back.
    #[default]
    Letters,
    /// Nintendo's letters: A on the right confirms, B at the bottom goes back.
    Nintendo,
    /// PlayStation's shapes: × at the bottom confirms, ○ on the right goes back.
    Shapes,
}

impl PadLabels {
    /// The confirm button.
    #[must_use]
    pub const fn confirm(self) -> Press<'static> {
        match self {
            Self::Letters | Self::Nintendo => Press::Pad("A"),
            Self::Shapes => Press::Cross,
        }
    }

    /// The back button.
    #[must_use]
    pub const fn back(self) -> Press<'static> {
        match self {
            Self::Letters | Self::Nintendo => Press::Pad("B"),
            Self::Shapes => Press::Circle,
        }
    }

    /// The Start button, as it's marked.
    #[must_use]
    pub const fn start(self) -> Press<'static> {
        match self {
            Self::Letters => Press::Pill("Start"),
            Self::Nintendo => Press::Pill("+"),
            Self::Shapes => Press::Pill("Options"),
        }
    }

    /// The Select button, as it's marked.
    #[must_use]
    pub const fn select(self) -> Press<'static> {
        match self {
            Self::Letters => Press::Pill("Back"),
            Self::Nintendo => Press::Minus,
            Self::Shapes => Press::Pill("Create"),
        }
    }

    /// A face button, as it's marked.
    #[must_use]
    pub const fn face(self, face: Face) -> Press<'static> {
        match (self, face) {
            (Self::Letters, Face::South) | (Self::Nintendo, Face::East) => Press::Pad("A"),
            (Self::Letters, Face::East) | (Self::Nintendo, Face::South) => Press::Pad("B"),
            (Self::Letters, Face::West) | (Self::Nintendo, Face::North) => Press::Pad("X"),
            (Self::Letters, Face::North) | (Self::Nintendo, Face::West) => Press::Pad("Y"),
            (Self::Shapes, Face::South) => Press::Cross,
            (Self::Shapes, Face::East) => Press::Circle,
            (Self::Shapes, Face::West) => Press::Square,
            (Self::Shapes, Face::North) => Press::Triangle,
        }
    }
}

/// A hint: what to press (alternatives, shown with `/` between them), and what it does.
#[derive(Clone, Copy, Debug)]
pub struct Hint<'a> {
    /// The keys, buttons or arrows that do it.
    pub press: &'a [Press<'a>],
    /// What it does.
    pub does: &'a str,
}

const KEY: Style = Style::new(12.0, colors::TEXT).weight(Weight::SemiBold);
const PAD: Style = Style::new(10.0, colors::TEXT).weight(Weight::SemiBold);
const WHAT: Style = Style::new(12.0, colors::MUTED);
/// Between a press and the next, or the words.
pub(crate) const GAP: f32 = 6.0;
/// Between one hint and the next.
const SPACING: f32 = 14.0;
const PAD_SIZE: f32 = 18.0;

fn width(fonts: &mut Fonts, press: Press) -> f32 {
    match press {
        // 5 pixels of padding and a 1-pixel border either side, as the mockups' CSS has it.
        Press::Key(k) => fonts.measure(k, &KEY) + 12.0,
        Press::Pad(_)
        | Press::Cross
        | Press::Circle
        | Press::Square
        | Press::Triangle
        | Press::Minus => PAD_SIZE,
        Press::Arrows(a) => fonts.measure(a, &WHAT),
        // As round as a pad button at its ends, and at least as wide.
        Press::Pill(p) => (fonts.measure(p, &PAD) + 14.0).max(PAD_SIZE),
    }
}

fn hint_width(fonts: &mut Fonts, hint: &Hint) -> f32 {
    let slash = fonts.measure("/", &WHAT);
    let presses: f32 = hint.press.iter().map(|&p| width(fonts, p)).sum();
    let between = hint.press.len().saturating_sub(1);
    #[allow(clippy::cast_precision_loss, reason = "a hint has a few presses")]
    let separators = between as f32 * (slash + 2.0 * GAP);
    presses + separators + GAP + fonts.measure(hint.does, &WHAT)
}

/// The width of a row of hints.
pub fn row_width(fonts: &mut Fonts, list: &[Hint]) -> f32 {
    let widths: f32 = list.iter().map(|h| hint_width(fonts, h)).sum();
    let spacing: f32 = list.iter().skip(1).map(|_| SPACING).sum();
    widths + spacing
}

/// Draws hints in a row 20 pixels tall from `y`, the first one starting at `left`.
pub fn hints_from(c: &mut Canvas, fonts: &mut Fonts, (left, y): (f32, f32), list: &[Hint]) {
    let width = row_width(fonts, list);
    hints(c, fonts, (left + width, y), list);
}

/// Draws hints in a row 20 pixels tall from `y`, the last one ending at `right`.
pub fn hints(c: &mut Canvas, fonts: &mut Fonts, (right, y): (f32, f32), hints: &[Hint]) {
    row(c, fonts, (right, y), hints, Ink::Plain);
}

/// What hints are drawn on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ink {
    /// A card.
    Plain,
    /// An accent button: dark marks.
    OnAccent,
}

/// The marks' colors for `ink`: a badge's edge, a key's letters, the words and slashes.
const fn inks(ink: Ink) -> (Color, Color, Color) {
    match ink {
        Ink::Plain => (colors::KEY_EDGE, colors::TEXT, colors::MUTED),
        Ink::OnAccent => (ON_ACCENT_EDGE, colors::PROMPT, ON_ACCENT_WORDS),
    }
}

const ON_ACCENT_EDGE: Color = Color::rgb(0x3A, 0x4A, 0x6A);
const ON_ACCENT_WORDS: Color = Color::rgb(0x2A, 0x36, 0x50);

/// Draws hints in a row 20 pixels tall from `y`, the first starting at `left`, on an accent
/// button.
pub(crate) fn hints_on_accent(
    c: &mut Canvas,
    fonts: &mut Fonts,
    (left, y): (f32, f32),
    list: &[Hint],
) {
    let width = row_width(fonts, list);
    row(c, fonts, (left + width, y), list, Ink::OnAccent);
}

fn row(c: &mut Canvas, fonts: &mut Fonts, (right, y): (f32, f32), hints: &[Hint], ink: Ink) {
    let (edge, letters, words) = inks(ink);
    let key_style = Style {
        color: letters,
        ..KEY
    };
    let pad_style = Style {
        color: letters,
        ..PAD
    };
    let what = Style {
        color: words,
        ..WHAT
    };
    let widths: Vec<f32> = hints.iter().map(|h| hint_width(fonts, h)).collect();
    let spacing: f32 = widths.iter().skip(1).map(|_| SPACING).sum();
    let mut x = right - widths.iter().sum::<f32>() - spacing;
    for hint in hints {
        for (i, &press) in hint.press.iter().enumerate() {
            if i > 0 {
                x += GAP;
                x += c.text(fonts, "/", (x, y + 1.0), 18.0, Align::Left, &what);
                x += GAP;
            }
            let w = width(fonts, press);
            match press {
                Press::Key(k) => {
                    c.rect((x, y, w, 20.0), 3.0, None, Some(Line::solid(edge, 1.0)));
                    c.text(
                        fonts,
                        k,
                        (x + w / 2.0, y + 1.0),
                        18.0,
                        Align::Center,
                        &key_style,
                    );
                }
                Press::Pad(b) => {
                    let center = (x + PAD_SIZE / 2.0, y + 10.0);
                    c.circle(center, PAD_SIZE / 2.0, None, Some(Line::solid(edge, 1.0)));
                    c.text(
                        fonts,
                        b,
                        (center.0, y + 1.0),
                        18.0,
                        Align::Center,
                        &pad_style,
                    );
                }
                Press::Arrows(a) => {
                    c.text(fonts, a, (x, y + 1.0), 18.0, Align::Left, &what);
                }
                Press::Pill(p) => {
                    let top = y + 10.0 - PAD_SIZE / 2.0;
                    c.rect(
                        (x, top, w, PAD_SIZE),
                        PAD_SIZE / 2.0,
                        None,
                        Some(Line::solid(edge, 1.0)),
                    );
                    c.text(
                        fonts,
                        p,
                        (x + w / 2.0, y + 1.0),
                        18.0,
                        Align::Center,
                        &pad_style,
                    );
                }
                Press::Cross | Press::Circle | Press::Square | Press::Triangle | Press::Minus => {
                    let (cx, cy) = (x + PAD_SIZE / 2.0, y + 10.0);
                    c.circle((cx, cy), PAD_SIZE / 2.0, None, Some(Line::solid(edge, 1.0)));
                    let mark = Line::solid(letters, 1.5);
                    match press {
                        Press::Cross => {
                            c.line((cx - 3.5, cy - 3.5), (cx + 3.5, cy + 3.5), mark);
                            c.line((cx - 3.5, cy + 3.5), (cx + 3.5, cy - 3.5), mark);
                        }
                        Press::Square => {
                            c.rect((cx - 4.0, cy - 4.0, 8.0, 8.0), 0.0, None, Some(mark));
                        }
                        Press::Minus => c.line((cx - 4.0, cy), (cx + 4.0, cy), mark),
                        Press::Triangle => {
                            c.line((cx, cy - 4.5), (cx + 4.5, cy + 3.5), mark);
                            c.line((cx + 4.5, cy + 3.5), (cx - 4.5, cy + 3.5), mark);
                            c.line((cx - 4.5, cy + 3.5), (cx, cy - 4.5), mark);
                        }
                        _ => c.circle((cx, cy), 4.5, None, Some(mark)),
                    }
                }
            }
            x += w;
        }
        x += GAP;
        x += c.text(fonts, hint.does, (x, y + 1.0), 18.0, Align::Left, &what);
        x += SPACING;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_is_marked_as_each_maker_marks_it() {
        assert!(matches!(PadLabels::Letters.select(), Press::Pill("Back")));
        assert!(matches!(PadLabels::Shapes.select(), Press::Pill("Create")));
        assert!(matches!(PadLabels::Nintendo.select(), Press::Minus));
    }
}
