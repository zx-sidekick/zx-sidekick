//! Inter, shaped and rasterised by swash, hinted and grayscale.

use crate::Color;
use swash::{
    scale::{Render, ScaleContext, Source},
    shape::ShapeContext,
    FontRef,
};

const REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.ttf");
const BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

/// One of Inter's weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    /// 400.
    Regular,
    /// 500.
    Medium,
    /// 600.
    SemiBold,
    /// 700.
    Bold,
}

/// How a run of text looks.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// Its size in pixels (before the canvas's scale).
    pub size: f32,
    /// Its weight.
    pub weight: Weight,
    /// Its color.
    pub color: Color,
    /// Extra space after each character, in ems (CSS `letter-spacing`).
    pub tracking: f32,
}

impl Style {
    /// Regular text of this size and color.
    #[must_use]
    pub const fn new(size: f32, color: Color) -> Self {
        Self {
            size,
            weight: Weight::Regular,
            color,
            tracking: 0.0,
        }
    }

    /// The same, in this weight.
    #[must_use]
    pub const fn weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    /// The same, with this letter spacing in ems.
    #[must_use]
    pub const fn tracking(mut self, tracking: f32) -> Self {
        self.tracking = tracking;
        self
    }
}

/// A glyph placed on a line: its id and pen position, in device pixels.
struct Placed {
    id: u16,
    x: f32,
}

/// The fonts, and swash's caches for shaping and rasterising them.
pub struct Fonts {
    shape: ShapeContext,
    scale: ScaleContext,
}

impl Default for Fonts {
    fn default() -> Self {
        Self::new()
    }
}

impl Fonts {
    /// Inter, ready to draw.
    #[must_use]
    pub fn new() -> Self {
        Self {
            shape: ShapeContext::new(),
            scale: ScaleContext::new(),
        }
    }

    fn font(weight: Weight) -> FontRef<'static> {
        let data = match weight {
            Weight::Regular => REGULAR,
            Weight::Medium => MEDIUM,
            Weight::SemiBold => SEMIBOLD,
            Weight::Bold => BOLD,
        };
        FontRef::from_index(data, 0).expect("Inter's fonts are embedded and valid")
    }

    /// Shapes `text` at `size` device pixels, with `tracking` device pixels after each glyph.
    fn shape(
        &mut self,
        text: &str,
        weight: Weight,
        size: f32,
        tracking: f32,
    ) -> (Vec<Placed>, f32) {
        let font = Self::font(weight);
        let mut shaper = self.shape.builder(font).size(size).build();
        shaper.add_str(text);
        let mut glyphs = Vec::new();
        let mut pen = 0.0;
        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                glyphs.push(Placed {
                    id: glyph.id,
                    x: pen + glyph.x,
                });
                pen += glyph.advance;
            }
            pen += tracking;
        });
        (glyphs, pen)
    }

    /// The width of `text` in `style`, in logical pixels.
    pub fn measure(&mut self, text: &str, style: &Style) -> f32 {
        self.shape(text, style.weight, style.size, style.tracking * style.size)
            .1
    }

    /// The distance from a line box's top to its baseline, for a line `line_height` tall,
    /// in logical pixels, as CSS centres a line's ascent and descent in its line box.
    #[must_use]
    pub fn baseline(style: &Style, line_height: f32) -> f32 {
        let metrics = Self::font(style.weight).metrics(&[]).scale(style.size);
        let content = metrics.ascent + metrics.descent;
        (line_height - content) / 2.0 + metrics.ascent
    }

    /// The height CSS gives a line of this style with `line-height: normal`.
    #[must_use]
    pub fn normal_line_height(style: &Style) -> f32 {
        let metrics = Self::font(style.weight).metrics(&[]).scale(style.size);
        metrics.ascent + metrics.descent + metrics.leading
    }

    /// Rasterises `text` with its baseline at (`x`, `baseline`) in device pixels, calling
    /// `plot(x, y, coverage)` for every pixel it covers.
    pub(crate) fn rasterise(
        &mut self,
        text: &str,
        style: &Style,
        scale: f32,
        x: f32,
        baseline: f32,
        mut plot: impl FnMut(i32, i32, u8),
    ) {
        let size = style.size * scale;
        let (glyphs, _) = self.shape(text, style.weight, size, style.tracking * size);
        let font = Self::font(style.weight);
        let mut scaler = self.scale.builder(font).size(size).hint(true).build();
        let baseline = baseline.round();
        for glyph in glyphs {
            let pen = x + glyph.x;
            let whole = pen.floor();
            let Some(image) = Render::new(&[Source::Outline])
                .offset(swash::zeno::Vector::new(pen - whole, 0.0))
                .render(&mut scaler, glyph.id)
            else {
                continue;
            };
            let p = image.placement;
            let width = usize::try_from(p.width).unwrap_or(0);
            for (i, &coverage) in image.data.iter().enumerate() {
                if coverage == 0 || width == 0 {
                    continue;
                }
                let (col, row) = (i % width, i / width);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap,
                    reason = "glyph images and canvases are far smaller than i32's range"
                )]
                let (px, py) = (
                    whole as i32 + p.left + col as i32,
                    baseline as i32 - p.top + row as i32,
                );
                plot(px, py, coverage);
            }
        }
    }

    /// Splits `text` into lines no wider than `max_width` logical pixels, breaking at spaces.
    pub fn wrap(&mut self, text: &str, style: &Style, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split(' ') {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && self.measure(&candidate, style) > max_width {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }
}
