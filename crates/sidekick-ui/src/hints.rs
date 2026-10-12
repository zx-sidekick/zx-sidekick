//! Key hints along a screen's bottom: keys in squarish badges, pad buttons in round ones,
//! arrows bare.

use crate::{colors, Align, Canvas, Fonts, Line, Style, Weight};

/// One thing a hint shows to press.
#[derive(Clone, Copy, Debug)]
pub enum Press<'a> {
    /// A keyboard key, in a squarish badge: `Enter`, `Esc`, `W`.
    Key(&'a str),
    /// A pad button, in a round badge: `A`, `B`, `X`.
    Pad(&'a str),
    /// Arrows, bare: `↑↓`, `←→`.
    Arrows(&'a str),
    /// A PlayStation pad's × button, in a round badge.
    Cross,
    /// A PlayStation pad's ○ button, in a round badge.
    Circle,
}

/// What a pad's confirm and back buttons are marked with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadLabels {
    /// A confirms and B goes back: Xbox and Nintendo pads, and most others.
    #[default]
    Letters,
    /// × confirms and ○ goes back: PlayStation pads.
    Shapes,
}

impl PadLabels {
    /// The confirm button.
    #[must_use]
    pub const fn confirm(self) -> Press<'static> {
        match self {
            Self::Letters => Press::Pad("A"),
            Self::Shapes => Press::Cross,
        }
    }

    /// The back button.
    #[must_use]
    pub const fn back(self) -> Press<'static> {
        match self {
            Self::Letters => Press::Pad("B"),
            Self::Shapes => Press::Circle,
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
const GAP: f32 = 6.0;
/// Between one hint and the next.
const SPACING: f32 = 14.0;
const PAD_SIZE: f32 = 18.0;

fn width(fonts: &mut Fonts, press: Press) -> f32 {
    match press {
        Press::Key(k) => fonts.measure(k, &KEY) + 10.0,
        Press::Pad(_) | Press::Cross | Press::Circle => PAD_SIZE,
        Press::Arrows(a) => fonts.measure(a, &WHAT),
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

/// Draws hints in a row 20 pixels tall from `y`, the last one ending at `right`.
pub fn hints(c: &mut Canvas, fonts: &mut Fonts, (right, y): (f32, f32), hints: &[Hint]) {
    let widths: Vec<f32> = hints.iter().map(|h| hint_width(fonts, h)).collect();
    let spacing: f32 = widths.iter().skip(1).map(|_| SPACING).sum();
    let mut x = right - widths.iter().sum::<f32>() - spacing;
    for hint in hints {
        for (i, &press) in hint.press.iter().enumerate() {
            if i > 0 {
                x += GAP;
                x += c.text(fonts, "/", (x, y + 1.0), 18.0, Align::Left, &WHAT);
                x += GAP;
            }
            let w = width(fonts, press);
            match press {
                Press::Key(k) => {
                    c.rect(
                        (x, y, w, 20.0),
                        3.0,
                        None,
                        Some(Line::solid(colors::KEY_EDGE, 1.0)),
                    );
                    c.text(fonts, k, (x + w / 2.0, y + 1.0), 18.0, Align::Center, &KEY);
                }
                Press::Pad(b) => {
                    let center = (x + PAD_SIZE / 2.0, y + 10.0);
                    c.circle(
                        center,
                        PAD_SIZE / 2.0,
                        None,
                        Some(Line::solid(colors::KEY_EDGE, 1.0)),
                    );
                    c.text(fonts, b, (center.0, y + 1.0), 18.0, Align::Center, &PAD);
                }
                Press::Arrows(a) => {
                    c.text(fonts, a, (x, y + 1.0), 18.0, Align::Left, &WHAT);
                }
                Press::Cross | Press::Circle => {
                    let (cx, cy) = (x + PAD_SIZE / 2.0, y + 10.0);
                    c.circle(
                        (cx, cy),
                        PAD_SIZE / 2.0,
                        None,
                        Some(Line::solid(colors::KEY_EDGE, 1.0)),
                    );
                    let mark = Line::solid(colors::TEXT, 1.5);
                    if matches!(press, Press::Cross) {
                        c.line((cx - 3.5, cy - 3.5), (cx + 3.5, cy + 3.5), mark);
                        c.line((cx - 3.5, cy + 3.5), (cx + 3.5, cy - 3.5), mark);
                    } else {
                        c.circle((cx, cy), 4.5, None, Some(mark));
                    }
                }
            }
            x += w;
        }
        x += GAP;
        x += c.text(fonts, hint.does, (x, y + 1.0), 18.0, Align::Left, &WHAT);
        x += SPACING;
    }
}
