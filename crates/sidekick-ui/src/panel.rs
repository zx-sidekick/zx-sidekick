//! The panel beside the picture: the guidance level, and the widgets it shows
//! (`docs/mockups/starquake/play-level*.png`).

use crate::{colors, Align, Canvas, Color, Fonts, Style, Weight};

/// The panel's width, in logical pixels.
pub const WIDTH: u32 = 408;
/// The panel's height, in logical pixels: the picture's.
pub const HEIGHT: u32 = 768;

const EDGE: Color = Color::rgb(0x23, 0x25, 0x2C);
const BODY: Color = Color::rgb(0xC4, 0xC7, 0xCE);
const BAR_OFF: Color = Color::rgb(0x2E, 0x32, 0x3C);

/// The columns' left edge, and the right column's right edge.
const LEFT: f32 = 24.0;
const RIGHT: f32 = 383.0;
/// Where the columns start, and the room between one widget and the next.
const TOP: f32 = 71.0;
const SPACING: f32 = 20.0;
/// From a widget's title to what it shows, and a line's height.
const TO_BODY: f32 = 22.0;
const LINE: f32 = 22.0;
/// A meter's length in a column, and across the panel.
const METER: f32 = 160.0;

/// Where a widget goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Column {
    /// Across the panel, below everything in the columns above it.
    #[default]
    Full,
    /// The left column.
    Left,
    /// The right column, right-aligned.
    Right,
}

/// What a widget shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// A line of text.
    Text(String),
    /// A number.
    Counter(i64),
    /// A bar, filled to `value` of `max`.
    Meter {
        /// How full.
        value: i64,
        /// Full.
        max: i64,
    },
    /// Nothing yet, in these words.
    Empty(String),
}

/// A widget in the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Widget {
    /// Where it goes.
    pub column: Column,
    /// Its heading, in capitals.
    pub title: Option<String>,
    /// What it shows.
    pub body: Body,
}

/// The panel, as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    /// The guidance level, and its name.
    pub level: usize,
    /// The level's name.
    pub name: String,
    /// The level's description, shown in the middle when no widget is.
    pub description: String,
    /// The widgets shown at this level, in order.
    pub widgets: Vec<Widget>,
}

/// A label's style: small capitals.
const fn label(color: Color) -> Style {
    Style::new(11.0, color)
        .weight(Weight::Medium)
        .tracking(0.12)
}

fn line(
    c: &mut Canvas,
    fonts: &mut Fonts,
    words: &str,
    at: (f32, f32),
    align: Align,
    style: &Style,
) {
    c.text(
        fonts,
        words,
        at,
        Fonts::normal_line_height(style),
        align,
        style,
    );
}

impl Panel {
    /// Draws the panel at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let mut c = Canvas::new(WIDTH, HEIGHT, scale);
        c.fill(colors::PANEL);
        #[allow(clippy::cast_precision_loss, reason = "the panel's height")]
        c.rect((0.0, 0.0, 1.0, HEIGHT as f32), 0.0, Some(EDGE), None);
        line(
            &mut c,
            fonts,
            "GUIDANCE",
            (LEFT, 27.0),
            Align::Left,
            &label(colors::LABEL),
        );
        let strong = Style::new(11.0, colors::TEXT)
            .weight(Weight::Bold)
            .tracking(0.12);
        if self.level == 0 {
            line(&mut c, fonts, "OFF", (RIGHT, 27.0), Align::Right, &strong);
        } else {
            let level = format!("LEVEL {}", self.level);
            line(&mut c, fonts, &level, (RIGHT, 27.0), Align::Right, &strong);
            let name = Style::new(12.0, BODY);
            line(
                &mut c,
                fonts,
                &self.name,
                (RIGHT, 43.0),
                Align::Right,
                &name,
            );
        }
        if self.widgets.is_empty() {
            self.description(&mut c, fonts);
        } else {
            self.widgets(&mut c, fonts);
        }
        c
    }

    /// The level's description in the middle, a sentence a line.
    fn description(&self, c: &mut Canvas, fonts: &mut Fonts) {
        let style = Style::new(13.0, colors::FAINT);
        let sentences: Vec<String> = self
            .description
            .split_inclusive(". ")
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        #[allow(clippy::cast_precision_loss, reason = "a few lines")]
        let top = 360.0 - LINE * sentences.len() as f32 / 2.0;
        let mut y = top;
        for sentence in &sentences {
            c.text(fonts, sentence, (204.0, y), LINE, Align::Center, &style);
            y += LINE;
        }
    }

    fn widgets(&self, c: &mut Canvas, fonts: &mut Fonts) {
        let (mut left, mut right) = (TOP, TOP);
        for widget in &self.widgets {
            let (x, align, top) = match widget.column {
                Column::Left => (LEFT, Align::Left, left),
                Column::Right => (RIGHT, Align::Right, right),
                Column::Full => (LEFT, Align::Left, left.max(right)),
            };
            let bottom = Self::widget(c, fonts, widget, (x, top), align);
            match widget.column {
                Column::Left => left = bottom + SPACING,
                Column::Right => right = bottom + SPACING,
                Column::Full => {
                    left = bottom + SPACING;
                    right = left;
                }
            }
        }
    }

    /// Draws a widget with its title's top at `y`; returns where it ends.
    fn widget(
        c: &mut Canvas,
        fonts: &mut Fonts,
        widget: &Widget,
        (x, y): (f32, f32),
        align: Align,
    ) -> f32 {
        let mut top = y;
        if let Some(title) = &widget.title {
            line(
                c,
                fonts,
                &title.to_uppercase(),
                (x, y),
                align,
                &label(colors::LABEL),
            );
            top += TO_BODY;
        }
        match &widget.body {
            Body::Text(text) => {
                line(c, fonts, text, (x, top), align, &Style::new(13.0, BODY));
            }
            Body::Empty(text) => {
                line(
                    c,
                    fonts,
                    text,
                    (x, top),
                    align,
                    &Style::new(13.0, colors::FAINT),
                );
            }
            Body::Counter(n) => {
                let style = Style::new(18.0, colors::TEXT).weight(Weight::SemiBold);
                line(c, fonts, &n.to_string(), (x, top), align, &style);
            }
            Body::Meter { value, max } => {
                let length = if widget.column == Column::Full {
                    RIGHT - LEFT
                } else {
                    METER
                };
                let left = if align == Align::Right { x - length } else { x };
                c.rect((left, top + 7.5, length, 7.0), 4.0, Some(BAR_OFF), None);
                #[allow(clippy::cast_precision_loss, reason = "a game's small numbers")]
                let part = if *max > 0 {
                    (*value).clamp(0, *max) as f32 / *max as f32
                } else {
                    0.0
                };
                // Whole pixels: the bar fills by the pixel, as the game's own would.
                let filled = (length * part).round();
                if filled > 0.0 {
                    c.rect(
                        (left, top + 7.5, filled, 7.0),
                        4.0,
                        Some(colors::ACCENT),
                        None,
                    );
                }
            }
        }
        top + LINE
    }
}
