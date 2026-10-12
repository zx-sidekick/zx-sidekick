//! ZX Sidekick's own screens, drawn in software into an RGBA canvas, pixel-exact.
//!
//! Screens are laid out in the mockups' pixels (`docs/mockups/starquake/`) and drawn at a
//! whole-number scale, so every line lands on whole pixels. Text is Inter; shapes are
//! tiny-skia's scalar path, which gives the same pixels on every platform.

#![allow(
    clippy::many_single_char_names,
    reason = "geometry reads best as x, y, w, h, r"
)]
#![allow(
    clippy::suboptimal_flops,
    reason = "layout arithmetic stays as written: plain sums and products, the same on every platform"
)]

mod entry;
pub mod hints;
mod library;
mod map;
mod menu;
mod note;
mod offer;
mod panel;
mod pause;
pub mod png;
mod tape_prompt;
mod text;

pub use entry::{EntryCard, EntryMode, HEIGHT as ENTRY_HEIGHT, WIDTH as ENTRY_WIDTH};
pub use hints::{Face, PadLabels};
pub use library::{LibraryEntry, LibraryPage};
pub use map::{
    pins, Cell, Edge, MapGrid, Marker, MarkerKind, Pin, Route, RouteColor, PICTURE as PINS_SIZE,
};
pub use menu::{Levels, Menu, Question, Row, SwitchRow, QUESTION_WIDTH, WIDTH as MENU_WIDTH};
pub use note::{Note, AT as NOTE_AT, HEIGHT as NOTE_HEIGHT};
pub use offer::{Offer, HEIGHT as OFFER_HEIGHT, WIDTH as OFFER_WIDTH};
pub use panel::{
    Body as PanelBody, CardRow, Chip, Column as PanelColumn, Icon, Look, Panel, Pixels, ScoreRow,
    ThisGame, Widget as PanelWidget, HEIGHT as PANEL_HEIGHT, WIDTH as PANEL_WIDTH,
};
pub use pause::{
    NoticeHint, PadMark, PauseNotice, AT as PAUSE_NOTICE_AT, HEIGHT as PAUSE_NOTICE_HEIGHT,
    WIDTH as PAUSE_NOTICE_WIDTH,
};
pub use tape_prompt::{
    Refusal, TapePrompt, HEIGHT as TAPE_PROMPT_HEIGHT, WIDTH as TAPE_PROMPT_WIDTH,
};
pub use text::{Fonts, Style, Weight};

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

/// An opaque or translucent color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha, 255 opaque.
    pub a: u8,
}

impl Color {
    /// An opaque color, as CSS's `#RRGGBB` would give it.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    fn skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

/// The mockups' colors.
pub mod colors {
    use super::Color;

    /// The tape prompt's background.
    pub const PROMPT: Color = Color::rgb(0x0B, 0x0C, 0x0F);
    /// The panel's background.
    pub const PANEL: Color = Color::rgb(0x12, 0x13, 0x18);
    /// Small capitals labels.
    pub const LABEL: Color = Color::rgb(0x6B, 0x6F, 0x7A);
    /// Headings and strong text.
    pub const TEXT: Color = Color::rgb(0xE8, 0xE9, 0xEC);
    /// Body text.
    pub const BODY: Color = Color::rgb(0xC4, 0xC7, 0xCE);
    /// Secondary text.
    pub const MUTED: Color = Color::rgb(0x9A, 0x9E, 0xA8);
    /// The faintest text.
    pub const FAINT: Color = Color::rgb(0x6B, 0x6F, 0x7A);
    /// A card's or button's background.
    pub const CARD: Color = Color::rgb(0x15, 0x17, 0x1C);
    /// A card's or button's edge.
    pub const EDGE: Color = Color::rgb(0x3A, 0x3E, 0x48);
    /// A key badge's edge.
    pub const KEY_EDGE: Color = Color::rgb(0x50, 0x54, 0x5E);
    /// The accent: buttons, selections.
    pub const ACCENT: Color = Color::rgb(0x8F, 0xB4, 0xFF);
    /// An error's red.
    pub const RED: Color = Color::rgb(0xE5, 0x48, 0x4D);
}

/// An outline: its color, width and, for a dashed one, its dash and gap.
#[derive(Clone, Copy, Debug)]
pub struct Line {
    /// Its color.
    pub color: Color,
    /// Its width, in logical pixels.
    pub width: f32,
    /// Dash and gap lengths, for a dashed line.
    pub dash: Option<(f32, f32)>,
}

impl Line {
    /// A solid line.
    #[must_use]
    pub const fn solid(color: Color, width: f32) -> Self {
        Self {
            color,
            width,
            dash: None,
        }
    }
}

/// Where a line of text sits horizontally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// Its left edge at `x`.
    Left,
    /// Centred on `x`.
    Center,
    /// Its right edge at `x`.
    Right,
}

/// A canvas `width` × `height` logical pixels, drawn at a whole-number scale.
pub struct Canvas {
    pixmap: Pixmap,
    scale: f32,
}

impl Canvas {
    /// A canvas of this logical size, `scale` device pixels to a logical one.
    ///
    /// # Panics
    /// If the size is zero or too large to allocate.
    #[must_use]
    pub fn new(width: u32, height: u32, scale: u32) -> Self {
        let scale = scale.max(1);
        Self {
            pixmap: Pixmap::new(width * scale, height * scale).expect("a canvas of a sane size"),
            #[allow(clippy::cast_precision_loss, reason = "scales are small whole numbers")]
            scale: scale as f32,
        }
    }

    /// Its size in device pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.pixmap.width(), self.pixmap.height())
    }

    /// Its pixels, RGBA row by row. Opaque wherever something opaque was drawn first.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// Its pixels, RGBA row by row, not premultiplied: for blending over something else.
    #[must_use]
    pub fn straight_rgba(&self) -> Vec<u8> {
        self.pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect()
    }

    /// The canvas as a PNG.
    #[must_use]
    pub fn png(&self) -> Vec<u8> {
        let (w, h) = self.size();
        png::encode(self.rgba(), w, h)
    }

    /// Fills the whole canvas.
    pub fn fill(&mut self, color: Color) {
        self.pixmap.fill(color.skia());
    }

    /// A rectangle with rounded corners, filled, outlined, or both.
    pub fn rect(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        radius: f32,
        fill: Option<Color>,
        line: Option<Line>,
    ) {
        let s = self.scale;
        let Some(path) = rounded(x * s, y * s, w * s, h * s, radius * s) else {
            return;
        };
        if let Some(color) = fill {
            self.pixmap.fill_path(
                &path,
                &paint(color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        if let Some(line) = line {
            // Inset by half the line, so the line sits inside the box, as CSS borders do.
            let inset = line.width * s / 2.0;
            let Some(path) = rounded(
                x * s + inset,
                y * s + inset,
                w * s - 2.0 * inset,
                h * s - 2.0 * inset,
                (radius * s - inset).max(0.0),
            ) else {
                return;
            };
            let mut stroke = Stroke {
                width: line.width * s,
                ..Stroke::default()
            };
            if let Some((dash, gap)) = line.dash {
                stroke.dash = StrokeDash::new(vec![dash * s, gap * s], 0.0);
            }
            self.pixmap.stroke_path(
                &path,
                &paint(line.color),
                &stroke,
                Transform::identity(),
                None,
            );
        }
    }

    /// A circle, filled, outlined, or both.
    pub fn circle(
        &mut self,
        (cx, cy): (f32, f32),
        radius: f32,
        fill: Option<Color>,
        line: Option<Line>,
    ) {
        self.rect(
            (cx - radius, cy - radius, radius * 2.0, radius * 2.0),
            radius,
            fill,
            line,
        );
    }

    /// A filled polygon through these points.
    pub fn polygon(&mut self, points: &[(f32, f32)], color: Color) {
        let s = self.scale;
        let mut pb = PathBuilder::new();
        for (i, &(x, y)) in points.iter().enumerate() {
            if i == 0 {
                pb.move_to(x * s, y * s);
            } else {
                pb.line_to(x * s, y * s);
            }
        }
        pb.close();
        if let Some(path) = pb.finish() {
            self.pixmap.fill_path(
                &path,
                &paint(color),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Lines through these points in turn, square-ended, dashed when the line is.
    pub fn polyline(&mut self, points: &[(f32, f32)], line: Line) {
        let s = self.scale;
        let mut pb = PathBuilder::new();
        for (i, &(x, y)) in points.iter().enumerate() {
            if i == 0 {
                pb.move_to(x * s, y * s);
            } else {
                pb.line_to(x * s, y * s);
            }
        }
        let Some(path) = pb.finish() else {
            return;
        };
        let mut stroke = Stroke {
            width: line.width * s,
            ..Stroke::default()
        };
        if let Some((dash, gap)) = line.dash {
            stroke.dash = StrokeDash::new(vec![dash * s, gap * s], 0.0);
        }
        self.pixmap.stroke_path(
            &path,
            &paint(line.color),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    /// A straight line between two points, with round ends.
    pub fn line(&mut self, (x1, y1): (f32, f32), (x2, y2): (f32, f32), line: Line) {
        let s = self.scale;
        let mut pb = PathBuilder::new();
        pb.move_to(x1 * s, y1 * s);
        pb.line_to(x2 * s, y2 * s);
        let Some(path) = pb.finish() else {
            return;
        };
        let stroke = Stroke {
            width: line.width * s,
            line_cap: tiny_skia::LineCap::Round,
            ..Stroke::default()
        };
        self.pixmap.stroke_path(
            &path,
            &paint(line.color),
            &stroke,
            Transform::identity(),
            None,
        );
    }

    /// One line of text, its line box's top at `y` and `line_height` tall, aligned on `x`.
    /// Returns its width in logical pixels.
    pub fn text(
        &mut self,
        fonts: &mut Fonts,
        text: &str,
        (x, y): (f32, f32),
        line_height: f32,
        align: Align,
        style: &Style,
    ) -> f32 {
        let width = fonts.measure(text, style);
        let left = match align {
            Align::Left => x,
            Align::Center => x - width / 2.0,
            Align::Right => x - width,
        };
        let s = self.scale;
        let baseline = (y + Fonts::baseline(style, line_height)) * s;
        let (w, h) = self.size();
        let color = style.color;
        let data = self.pixmap.data_mut();
        fonts.rasterise(text, style, s, left * s, baseline, |px, py, coverage| {
            let (Ok(px), Ok(py)) = (u32::try_from(px), u32::try_from(py)) else {
                return;
            };
            if px >= w || py >= h {
                return;
            }
            let i = ((py * w + px) * 4) as usize;
            let a = u16::from(coverage) * u16::from(color.a) / 255;
            for (channel, value) in [color.r, color.g, color.b].into_iter().enumerate() {
                let dst = u16::from(data[i + channel]);
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "a blend of two bytes is a byte"
                )]
                let blended = ((u16::from(value) * a + dst * (255 - a) + 127) / 255) as u8;
                data[i + channel] = blended;
            }
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a blend of two bytes is a byte"
            )]
            let alpha = (a + u16::from(data[i + 3]) * (255 - a) / 255) as u8;
            data[i + 3] = alpha;
        });
        width
    }

    /// Text wrapped to `max_width`, each line `line_height` below the last, from `y`. Returns
    /// the number of lines.
    pub fn paragraph(
        &mut self,
        fonts: &mut Fonts,
        text: &str,
        (x, y): (f32, f32),
        max_width: f32,
        line_height: f32,
        style: &Style,
    ) -> usize {
        let lines = fonts.wrap(text, style, max_width);
        for (i, line) in lines.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a few lines")]
            let top = y + i as f32 * line_height;
            self.text(fonts, line, (x, top), line_height, Align::Left, style);
        }
        lines.len()
    }
}

fn paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color.skia());
    paint.anti_alias = true;
    paint
}

fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r == 0.0 {
        return Some(PathBuilder::from_rect(Rect::from_xywh(x, y, w, h)?));
    }
    // A cubic's control points sit 0.5523 of the radius in, for a quarter circle.
    let k = r * (1.0 - 0.552_284_8);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - k, y, x + w, y + k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - k, x + w - k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + k, y + h, x, y + h - k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + k, x + k, y, x + r, y);
    pb.close();
    pb.finish()
}
