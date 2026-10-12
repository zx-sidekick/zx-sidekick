//! The panel beside the picture: the guidance level, and the widgets it shows
//! (`docs/mockups/starquake/play-level*.png`).

use crate::{colors, Align, Canvas, Color, Fonts, Line, Style, Weight};

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
    /// Short words in the game's font.
    Chips(Vec<Chip>),
    /// Pictures in a grid, `columns` a row.
    Icons {
        /// Pictures a row.
        columns: usize,
        /// The pictures.
        icons: Vec<Icon>,
    },
    /// Groups of pictures, each with a label in the game's font.
    Cards(Vec<CardRow>),
}

/// A picture of the game's, one bit a pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    /// Its width, in the game's pixels.
    pub width: usize,
    /// Its height, in the game's pixels.
    pub height: usize,
    /// Row by row, whether each pixel is ink.
    pub ink: Vec<bool>,
    /// Its own ink, when the game gives one.
    pub color: Option<Color>,
}

impl Pixels {
    fn at(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height && self.ink[y * self.width + x]
    }
}

/// How a picture or chip stands out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    /// Plainly.
    #[default]
    Bright,
    /// Faded.
    Dim,
    /// With an outline.
    Outlined,
}

/// A word in the game's font: its characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    /// Its characters, in order.
    pub glyphs: Vec<Pixels>,
    /// Whether it's outlined.
    pub outlined: bool,
}

/// A picture in a grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Icon {
    /// The picture.
    pub picture: Pixels,
    /// How it stands out.
    pub look: Look,
}

/// A group of pictures, labelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardRow {
    /// Its label's characters, in the game's font.
    pub label: Vec<Pixels>,
    /// Its pictures, each answered or not yet.
    pub cards: Vec<(Pixels, bool)>,
}

const TEAL: Color = Color::rgb(0x6F, 0xD6, 0xC4);
const TEAL_BG: Color = Color::rgb(0x1D, 0x35, 0x32);
const TILE: Color = Color::rgb(0x1D, 0x20, 0x27);
/// Logical pixels to a game pixel: the font's, and the pictures'.
const FONT_PIXEL: f32 = 2.0;
const ICON_PIXEL: f32 = 4.0;
const CARD_PIXEL: f32 = 2.0;

/// Draws a picture's ink, each game pixel `size` logical pixels square, from (`x`, `y`).
fn pixels(c: &mut Canvas, picture: &Pixels, (x, y): (f32, f32), size: f32, color: Color) {
    for py in 0..picture.height {
        for px in 0..picture.width {
            if picture.at(px, py) {
                #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
                let at = (x + px as f32 * size, y + py as f32 * size);
                c.rect((at.0, at.1, size, size), 0.0, Some(color), None);
            }
        }
    }
}

/// A colour at some opacity.
const fn faded(color: Color, alpha: u8) -> Color {
    Color { a: alpha, ..color }
}

/// A word's width in the game's font, in logical pixels.
fn word_width(glyphs: &[Pixels]) -> f32 {
    #[allow(clippy::cast_precision_loss, reason = "a word's few pixels")]
    let width = glyphs.iter().map(|g| g.width).sum::<usize>() as f32;
    width * FONT_PIXEL
}

/// Draws a word in the game's font from (`x`, `y`).
fn word(c: &mut Canvas, glyphs: &[Pixels], (x, y): (f32, f32), color: Color) {
    let mut left = x;
    for glyph in glyphs {
        pixels(c, glyph, (left, y), FONT_PIXEL, color);
        #[allow(clippy::cast_precision_loss, reason = "a glyph's few pixels")]
        let advance = glyph.width as f32 * FONT_PIXEL;
        left += advance;
    }
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
            Body::Chips(chips) => return Self::chips(c, chips, (x, top), align),
            Body::Icons { columns, icons } => {
                return Self::icons(c, *columns, icons, (x, top), align);
            }
            Body::Cards(rows) => return Self::cards(c, rows, (x, top), align),
        }
        top + LINE
    }

    /// Chips a line each, as wide as their words; returns where they end.
    fn chips(c: &mut Canvas, chips: &[Chip], (x, top): (f32, f32), align: Align) -> f32 {
        let mut y = top;
        for chip in chips {
            let width = word_width(&chip.glyphs) + 16.0;
            let left = if align == Align::Right { x - width } else { x };
            let line = chip.outlined.then(|| Line::solid(colors::ACCENT, 1.5));
            c.rect((left, y, width, 22.0), 3.0, Some(TEAL_BG), line);
            let height = chip.glyphs.first().map_or(8.0, |g| {
                #[allow(clippy::cast_precision_loss, reason = "a glyph's few pixels")]
                let h = g.height as f32;
                h
            });
            word(
                c,
                &chip.glyphs,
                (left + 8.0, y + (22.0 - height * FONT_PIXEL) / 2.0),
                TEAL,
            );
            y += 23.0;
        }
        y - 1.0
    }

    /// Pictures on tiles, `columns` a row; returns where they end.
    fn icons(
        c: &mut Canvas,
        columns: usize,
        icons: &[Icon],
        (x, top): (f32, f32),
        align: Align,
    ) -> f32 {
        let columns = columns.max(1);
        #[allow(clippy::cast_precision_loss, reason = "a few columns")]
        let width = columns as f32 * 42.0 - 6.0;
        let left = if align == Align::Right { x - width } else { x };
        let mut bottom = top;
        for (i, icon) in icons.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a few pictures")]
            let (tx, ty) = (
                left + (i % columns) as f32 * 42.0,
                top - 1.0 + (i / columns) as f32 * 42.0,
            );
            let outline = (icon.look == Look::Outlined).then(|| Line::solid(colors::TEXT, 1.5));
            c.rect((tx, ty, 36.0, 36.0), 4.0, Some(TILE), outline);
            let ink = icon.picture.color.unwrap_or(colors::TEXT);
            let ink = if icon.look == Look::Dim {
                faded(ink, 89)
            } else {
                ink
            };
            #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
            let (pw, ph) = (
                icon.picture.width as f32 * ICON_PIXEL,
                icon.picture.height as f32 * ICON_PIXEL,
            );
            pixels(
                c,
                &icon.picture,
                (tx + (36.0 - pw) / 2.0, ty + (36.0 - ph) / 2.0),
                ICON_PIXEL,
                ink,
            );
            bottom = ty + 36.0;
        }
        bottom
    }

    /// Groups of pictures a row each, labelled to their left; returns where they end.
    fn cards(c: &mut Canvas, rows: &[CardRow], (x, top): (f32, f32), align: Align) -> f32 {
        let mut bottom = top;
        for (r, row) in rows.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a few rows and cards")]
            let (y, width) = (
                top - 2.0 + r as f32 * 23.0,
                row.cards.len() as f32 * 22.0 - 2.0,
            );
            let label = word_width(&row.label);
            let cards_left = if align == Align::Right {
                x - width
            } else {
                x + label + 3.0
            };
            word(c, &row.label, (cards_left - 3.0 - label, y + 2.0), TEAL);
            for (i, (picture, answered)) in row.cards.iter().enumerate() {
                #[allow(clippy::cast_precision_loss, reason = "a few cards")]
                let cx = cards_left + i as f32 * 22.0;
                let alpha = if *answered { 255 } else { 115 };
                c.rect((cx, y, 20.0, 20.0), 4.0, Some(faded(TEAL_BG, alpha)), None);
                #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
                let (pw, ph) = (
                    picture.width as f32 * CARD_PIXEL,
                    picture.height as f32 * CARD_PIXEL,
                );
                let ink = faded(picture.color.unwrap_or(TEAL), alpha);
                pixels(
                    c,
                    picture,
                    (cx + (20.0 - pw) / 2.0, y + (20.0 - ph) / 2.0),
                    CARD_PIXEL,
                    ink,
                );
            }
            bottom = y + 20.0;
        }
        bottom
    }
}
