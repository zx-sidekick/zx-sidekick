//! The panel beside the picture: the guidance level, and the widgets it shows
//! (`docs/mockups/starquake/play-level*.png`).

use crate::{colors, map::MapGrid, Align, Canvas, Color, Fonts, Line, Style, Weight};

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
/// The panel's height, for the room left for the map.
#[allow(clippy::cast_precision_loss, reason = "the panel's height")]
const HEIGHT_F: f32 = HEIGHT as f32;
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
    /// The map.
    Map(MapGrid),
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
    /// Its outline's colour, when it's outlined.
    pub outline: Option<Color>,
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
    /// Its outline's colour, when it's outlined.
    pub outline: Option<Color>,
}

const TEAL: Color = Color::rgb(0x6F, 0xD6, 0xC4);
const TEAL_BG: Color = Color::rgb(0x1D, 0x35, 0x32);
const TILE: Color = Color::rgb(0x1D, 0x20, 0x27);
/// Logical pixels to a game pixel: the font's.
const FONT_PIXEL: f32 = 2.0;
/// The room a picture has on its tile: an icon's, and a card's.
const ICON_ROOM: f32 = 32.0;
const CARD_ROOM: f32 = 16.0;

/// Logical pixels to a game pixel for a picture in `room`: as many whole ones as fit, so an
/// 8-pixel picture is drawn at 4 in an icon's 32, and a 16-pixel one at 2; never less than 1.
fn fit(picture: &Pixels, room: f32) -> f32 {
    #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
    let side = picture.width.max(picture.height).max(1) as f32;
    (room / side).floor().max(1.0)
}

/// Draws a picture's ink, each game pixel `size` logical pixels square, from (`x`, `y`): at the
/// nearest whole number of device pixels a game pixel, so every pixel is the same size, with
/// its middle where it would be.
pub(crate) fn draw_pixels(
    c: &mut Canvas,
    picture: &Pixels,
    at: (f32, f32),
    size: f32,
    color: Color,
) {
    let (left, top, step) = snap(c, at, (picture.width, picture.height), size);
    ink(c, picture, (left, top), step, color);
}

/// Where pixel art `width` × `height` game pixels large goes, each `size` logical pixels square
/// with its top left at (`x`, `y`): its top left in device pixels and the device pixels a game
/// pixel, the nearest whole number (at least one), its middle kept where it was.
#[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
fn snap(
    c: &Canvas,
    (x, y): (f32, f32),
    (width, height): (usize, usize),
    size: f32,
) -> (f32, f32, f32) {
    let s = c.scale();
    let step = (size * s).round().max(1.0);
    let (w, h) = (width as f32, height as f32);
    (
        ((x + w * size / 2.0) * s - w * step / 2.0).round(),
        ((y + h * size / 2.0) * s - h * step / 2.0).round(),
        step,
    )
}

/// Draws a picture's ink from (`left`, `top`) in device pixels, `step` device pixels a game
/// pixel.
fn ink(c: &mut Canvas, picture: &Pixels, (left, top): (f32, f32), step: f32, color: Color) {
    for py in 0..picture.height {
        for px in 0..picture.width {
            if picture.at(px, py) {
                #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
                let at = (left + px as f32 * step, top + py as f32 * step);
                c.device_rect((at.0, at.1, step, step), color);
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

/// Draws a word in the game's font from (`x`, `y`), its letters at the same whole device
/// pixels.
fn word(c: &mut Canvas, glyphs: &[Pixels], at: (f32, f32), color: Color) {
    let width = glyphs.iter().map(|g| g.width).sum();
    let height = glyphs.iter().map(|g| g.height).max().unwrap_or(0);
    let (mut left, top, step) = snap(c, at, (width, height), FONT_PIXEL);
    for glyph in glyphs {
        ink(c, glyph, (left, top), step, color);
        #[allow(clippy::cast_precision_loss, reason = "a glyph's few pixels")]
        let advance = glyph.width as f32 * step;
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

/// The panel's background and its left edge, filling `size` device pixels, `WIDTH` logical
/// pixels across.
#[allow(
    clippy::cast_precision_loss,
    reason = "screen sizes are far below f32's limit"
)]
fn backdrop(size: (u32, u32)) -> Canvas {
    let scale = size.0 as f32 / WIDTH as f32;
    let mut c = Canvas::with_size(size, scale);
    c.fill(colors::PANEL);
    c.rect(
        (0.0, 0.0, 1.0, size.1 as f32 / scale),
        0.0,
        Some(EDGE),
        None,
    );
    c
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
    /// Draws the panel to fill `size` device pixels: its design's width across, at whatever
    /// scale that is, and its design's height down, or more on a taller window.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, size: (u32, u32)) -> Canvas {
        let mut c = backdrop(size);
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
        // How far right the left column reaches, for the map's legend beside it.
        let mut reach = LEFT;
        for widget in &self.widgets {
            let (x, align, top) = match widget.column {
                Column::Left => (LEFT, Align::Left, left),
                Column::Right => (RIGHT, Align::Right, right),
                Column::Full => (LEFT, Align::Left, left.max(right)),
            };
            // The map's legend goes beside what's above it, so the map itself doesn't count.
            if widget.column == Column::Left && !matches!(widget.body, Body::Map(_)) {
                reach = reach.max(LEFT + Self::extent(fonts, widget));
            }
            let bottom = if let Body::Map(map) = &widget.body {
                let cell = map.cell_size((RIGHT - LEFT, HEIGHT_F - 56.0 - top));
                map.legend(c, fonts, (reach + SPACING, top - 41.0));
                map.draw(c, fonts, (LEFT, top), cell)
            } else {
                Self::widget(c, fonts, widget, (x, top), align)
            };
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

    /// How wide a widget is drawn, in logical pixels.
    fn extent(fonts: &mut Fonts, widget: &Widget) -> f32 {
        let title = widget.title.as_ref().map_or(0.0, |t| {
            fonts.measure(&t.to_uppercase(), &label(colors::LABEL))
        });
        let body = match &widget.body {
            Body::Text(t) | Body::Empty(t) => fonts.measure(t, &Style::new(13.0, BODY)),
            Body::Counter(n) => fonts.measure(
                &n.to_string(),
                &Style::new(18.0, colors::TEXT).weight(Weight::SemiBold),
            ),
            Body::Meter { .. } => METER,
            Body::Chips(chips) => chips
                .iter()
                .map(|chip| word_width(&chip.glyphs) + 16.0)
                .fold(0.0, f32::max),
            Body::Icons { columns, .. } => {
                #[allow(clippy::cast_precision_loss, reason = "a few columns")]
                let width = (*columns).max(1) as f32 * 42.0 - 6.0;
                width
            }
            Body::Cards(rows) => rows
                .iter()
                .map(|row| {
                    #[allow(clippy::cast_precision_loss, reason = "a few cards")]
                    let cards = row.cards.len() as f32 * 22.0 - 2.0;
                    word_width(&row.label) + 3.0 + cards
                })
                .fold(0.0, f32::max),
            Body::Map(_) => RIGHT - LEFT,
        };
        title.max(body)
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
            // Drawn by `widgets`, which knows the room it has.
            Body::Map(_) => {}
        }
        top + LINE
    }

    /// Chips a line each, as wide as their words; returns where they end.
    fn chips(c: &mut Canvas, chips: &[Chip], (x, top): (f32, f32), align: Align) -> f32 {
        let mut y = top;
        for chip in chips {
            let width = word_width(&chip.glyphs) + 16.0;
            let left = if align == Align::Right { x - width } else { x };
            c.rect((left, y, width, 22.0), 3.0, Some(TEAL_BG), None);
            if let Some(color) = chip.outline {
                // Outside the chip, as CSS draws an outline.
                let ring = (left - 1.5, y - 1.5, width + 3.0, 25.0);
                c.rect(ring, 4.5, None, Some(Line::solid(color, 1.5)));
            }
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
            let pixel = fit(&icon.picture, ICON_ROOM);
            #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
            let (pw, ph) = (
                icon.picture.width as f32 * pixel,
                icon.picture.height as f32 * pixel,
            );
            draw_pixels(
                c,
                &icon.picture,
                (tx + (36.0 - pw) / 2.0, ty + (36.0 - ph) / 2.0),
                pixel,
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
            if let Some(color) = row.outline {
                // Round the label and the cards, outside them, as a chip's.
                let left = cards_left - 3.0 - label - 4.0;
                let ring = (left - 1.5, y - 1.5, width + label + 7.0 + 3.0, 23.0);
                c.rect(ring, 4.5, None, Some(Line::solid(color, 1.5)));
            }
            for (i, (picture, answered)) in row.cards.iter().enumerate() {
                #[allow(clippy::cast_precision_loss, reason = "a few cards")]
                let cx = cards_left + i as f32 * 22.0;
                let alpha = if *answered { 255 } else { 115 };
                c.rect((cx, y, 20.0, 20.0), 4.0, Some(faded(TEAL_BG, alpha)), None);
                let pixel = fit(picture, CARD_ROOM);
                #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
                let (pw, ph) = (picture.width as f32 * pixel, picture.height as f32 * pixel);
                let ink = faded(picture.color.unwrap_or(TEAL), alpha);
                draw_pixels(
                    c,
                    picture,
                    (cx + (20.0 - pw) / 2.0, y + (20.0 - ph) / 2.0),
                    pixel,
                    ink,
                );
            }
            bottom = y + 20.0;
        }
        bottom
    }
}

/// A row of the high-score table after a game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreRow {
    /// The name.
    pub name: String,
    /// How its game was played: "level 3 · What you have seen", "from the tape".
    pub mark: String,
    /// Whether it's the game just played.
    pub mine: bool,
}

/// The panel after a game: what it was played with, and the high-score table
/// (`docs/mockups/starquake/game-over.png`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThisGame {
    /// "Guidance up to level 3", or "No guidance".
    pub played_with: String,
    /// The level's name, or what else marks it.
    pub detail: String,
    /// The table's heading: "CORE OF HEROES".
    pub title: String,
    /// The table, in order.
    pub rows: Vec<ScoreRow>,
}

/// A line of text on a 16-pixel line's baseline, as the mockups' smaller spans are.
fn span(c: &mut Canvas, fonts: &mut Fonts, words: &str, (x, y): (f32, f32), style: &Style) {
    let top = y + Fonts::in_body_line(style);
    line(c, fonts, words, (x, top), Align::Left, style);
}

impl ThisGame {
    /// Draws the page to fill `size` device pixels, as the panel is ([`Panel::draw`]).
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, size: (u32, u32)) -> Canvas {
        let mut c = backdrop(size);
        line(
            &mut c,
            fonts,
            "THIS GAME",
            (LEFT, 27.0),
            Align::Left,
            &label(colors::LABEL),
        );
        span(
            &mut c,
            fonts,
            "Played with",
            (LEFT, 300.0),
            &Style::new(13.0, colors::MUTED),
        );
        let headline = Style::new(21.0, colors::TEXT).weight(Weight::Medium);
        line(
            &mut c,
            fonts,
            &self.played_with,
            (LEFT, 322.0),
            Align::Left,
            &headline,
        );
        span(
            &mut c,
            fonts,
            &self.detail,
            (LEFT, 356.0),
            &Style::new(13.0, BODY),
        );
        let title = self.title.to_uppercase();
        line(
            &mut c,
            fonts,
            &title,
            (LEFT, 425.0),
            Align::Left,
            &label(colors::LABEL),
        );
        for (i, row) in self.rows.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a few rows")]
            let y = 451.0 + i as f32 * 23.0;
            let (weight, ink, mark_ink) = if row.mine {
                (Weight::SemiBold, colors::TEXT, colors::TEXT)
            } else {
                (Weight::Medium, BODY, colors::MUTED)
            };
            let name = Style::new(15.0, ink).weight(weight);
            span(
                &mut c,
                fonts,
                &format!("{} {}", i + 1, row.name),
                (LEFT, y),
                &name,
            );
            span(
                &mut c,
                fonts,
                &row.mark,
                (100.0, y + 1.0),
                &Style::new(14.0, mark_ink),
            );
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(side: usize) -> Pixels {
        Pixels {
            width: side,
            height: side,
            ink: vec![false; side * side],
            color: None,
        }
    }

    #[test]
    fn between_whole_scales_a_picture_s_pixels_are_whole_and_even() {
        // Every other pixel ink, two logical pixels each, at 1.4 device pixels a logical one:
        // three device pixels each, not 2.8 blurred over the edges.
        let stripes = Pixels {
            width: 8,
            height: 1,
            ink: (0..8).map(|x| x % 2 == 0).collect(),
            color: None,
        };
        let mut c = Canvas::with_size((40, 10), 1.4);
        draw_pixels(&mut c, &stripes, (3.3, 1.0), 2.0, colors::TEXT);
        // Its top at (1 + 1) × 1.4 less half its three device pixels: row 1.
        let row: Vec<u8> = c
            .rgba()
            .chunks_exact(4)
            .skip(40)
            .take(40)
            .map(|p| p[3])
            .collect();
        assert!(
            row.iter().all(|&a| a == 0 || a == 255),
            "no partial pixels: {row:?}"
        );
        let runs: Vec<usize> = row.chunk_by(|a, b| a == b).map(<[u8]>::len).collect();
        assert!(
            runs[1..runs.len() - 1].iter().all(|&n| n == 3),
            "every pixel three wide: {runs:?}"
        );
        // Its middle stays where it was: (3.3 + 8) × 1.4, less half its 24 device pixels.
        assert_eq!(runs[0], 4);
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "whole numbers, exact in f32")]
    fn a_picture_fits_its_tile_at_a_whole_size() {
        assert_eq!(fit(&picture(8), ICON_ROOM), 4.0, "the mockups' stand-ins");
        assert_eq!(fit(&picture(16), ICON_ROOM), 2.0, "Starquake's items");
        assert_eq!(fit(&picture(16), CARD_ROOM), 1.0);
        assert_eq!(fit(&picture(12), ICON_ROOM), 2.0, "whole pixels only");
        assert_eq!(fit(&picture(40), CARD_ROOM), 1.0, "never smaller than one");
    }
}
