//! Drawing for the screens the host shows itself, outside the game: text in
//! Inter, rasterised by `fontdue`, and plain shapes, into an RGBA frame.
//!
//! The game's own font is on the tape, so it cannot be used for the screen
//! that asks for the tape; and #1 chose a modern, legible face for
//! everything the host draws. Inter is under the SIL Open Font License and is
//! shipped unmodified, with its licence beside it in `fonts/`.

use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle, WrapStyle};
use fontdue::{Font, FontSettings};

/// An opaque colour.
pub type Rgb = [u8; 3];

/// The colours every screen the window draws shares: a card over a dark
/// ground, its text, and the badges of a legend. From the guidance mockups
/// on starquake-recompiled#1, so the pause notice, the panel and the picker
/// come out as one.
pub mod palette {
    use super::Rgb;
    pub const CARD: Rgb = [0x15, 0x17, 0x1e];
    pub const LINE: Rgb = [0x2a, 0x2d, 0x36];
    pub const TITLE: Rgb = [0xe6, 0xe8, 0xee];
    pub const MUTED: Rgb = [0x8b, 0x90, 0xa0];
    pub const BADGE: Rgb = [0x1c, 0x1f, 0x27];
    pub const BADGE_LINE: Rgb = [0x3a, 0x3e, 0x4a];
    pub const BADGE_TEXT: Rgb = [0xd0, 0xd4, 0xdc];
}

#[derive(Clone, Copy)]
pub enum Weight {
    Regular = 0,
    SemiBold = 1,
}

/// One run of text: its words, size in logical pixels, weight and colour.
pub struct Span<'a> {
    pub text: &'a str,
    pub size: f32,
    pub weight: Weight,
    pub colour: Rgb,
}

/// An RGBA frame drawn at `scale` device pixels per logical pixel, so what
/// is laid out in logical pixels comes out sharp on a high-density screen.
pub struct Canvas<'a> {
    pub pixels: &'a mut [u8],
    pub width: usize,
    pub height: usize,
    pub scale: f32,
}

pub struct Fonts {
    fonts: [Font; 2],
    layout: Layout<Rgb>,
}

impl Fonts {
    /// # Panics
    ///
    /// If the bundled font files do not parse, which would be a broken build.
    pub fn load() -> Fonts {
        let load =
            |bytes: &[u8]| Font::from_bytes(bytes, FontSettings::default()).expect("bundled font");
        Fonts {
            fonts: [
                load(include_bytes!("../../fonts/Inter-Regular.ttf")),
                load(include_bytes!("../../fonts/Inter-SemiBold.ttf")),
            ],
            layout: Layout::new(CoordinateSystem::PositiveYDown),
        }
    }

    /// Lays out `spans` from (`x`, `y`), the top left in logical pixels,
    /// wrapping at `max_width` if given, `line_height` times the size apart,
    /// and draws them if `canvas` is.
    /// Returns the size the text took, in logical pixels.
    pub fn text(
        &mut self,
        canvas: Option<&mut Canvas>,
        x: f32,
        y: f32,
        max_width: Option<f32>,
        line_height: f32,
        spans: &[Span],
    ) -> (f32, f32) {
        let scale = canvas.as_ref().map_or(1.0, |c| c.scale);
        self.layout.reset(&LayoutSettings {
            x: x * scale,
            y: y * scale,
            max_width: max_width.map(|w| w * scale),
            wrap_style: WrapStyle::Word,
            // Given as a multiple of the size, as CSS has it; the layout
            // wants a multiple of the font's own line spacing, which for
            // Inter is 1.21 times the size.
            line_height: line_height / 1.21,
            ..LayoutSettings::default()
        });
        for span in spans {
            self.layout.append(
                &self.fonts,
                &TextStyle::with_user_data(
                    span.text,
                    span.size * scale,
                    span.weight as usize,
                    span.colour,
                ),
            );
        }
        let glyphs = self.layout.glyphs();
        let width = glyphs
            .iter()
            .map(|g| g.x + g.width as f32)
            .fold(x * scale, f32::max)
            - x * scale;
        let height = self.layout.height();
        if let Some(canvas) = canvas {
            for g in glyphs {
                if g.width == 0 || g.height == 0 {
                    continue;
                }
                let (metrics, coverage) = self.fonts[g.font_index].rasterize_config(g.key);
                for row in 0..metrics.height {
                    for col in 0..metrics.width {
                        let a = coverage[row * metrics.width + col];
                        if a > 0 {
                            canvas.blend(
                                g.x as isize + col as isize,
                                g.y as isize + row as isize,
                                g.user_data,
                                a,
                            );
                        }
                    }
                }
            }
        }
        (width / scale, height / scale)
    }

    /// How far one character moves the pen, in logical pixels, spaces
    /// included (a space draws nothing, so it cannot be measured by what it
    /// draws).
    pub fn advance(&self, c: char, size: f32, weight: Weight) -> f32 {
        self.fonts[weight as usize].metrics(c, size).advance_width
    }

    /// The width `spans` take on one line, in logical pixels.
    pub fn measure(&mut self, spans: &[Span]) -> f32 {
        self.text(None, 0.0, 0.0, None, 1.0, spans).0
    }

    /// `label` centred in the box `(x, y, w, h)`, in logical pixels.
    fn centred(
        &mut self,
        canvas: &mut Canvas,
        (x, y, w, h): (f32, f32, f32, f32),
        label: &str,
        size: f32,
    ) {
        let span = Span {
            text: label,
            size,
            weight: Weight::SemiBold,
            colour: palette::BADGE_TEXT,
        };
        let tw = self.measure(std::slice::from_ref(&span));
        // A line box is 1.21 times the size for Inter; the glyphs sit a
        // little above its middle.
        let ty = y + (h - size * 1.21) / 2.0 + size * 0.02;
        self.text(Some(canvas), x + (w - tw) / 2.0, ty, None, 1.0, &[span]);
    }

    /// A keyboard key in a legend: its name in a squarish badge `h` logical
    /// pixels high, as the guidance mockups draw `Enter` and `Esc`. Returns
    /// the badge's width.
    pub fn key_badge(&mut self, canvas: &mut Canvas, x: f32, y: f32, h: f32, label: &str) -> f32 {
        let size = h * 0.58;
        let tw = self.measure(&[Span {
            text: label,
            size,
            weight: Weight::SemiBold,
            colour: palette::BADGE_TEXT,
        }]);
        let w = (tw + h * 0.62).max(h);
        canvas.round_rect(x, y, w, h, h * 0.19, palette::BADGE);
        canvas.outline(x, y, w, h, h * 0.19, 1.5, None, palette::BADGE_LINE);
        self.centred(canvas, (x, y, w, h), label, size);
        w
    }

    /// A gamepad button in a legend: its letter in a round badge `h` logical
    /// pixels across, as on the pad. Returns the badge's width, which is `h`.
    pub fn button_badge(
        &mut self,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
        h: f32,
        label: &str,
    ) -> f32 {
        canvas.round_rect(x, y, h, h, h / 2.0, palette::BADGE);
        canvas.outline(x, y, h, h, h / 2.0, 1.5, None, palette::BADGE_LINE);
        self.centred(canvas, (x, y, h, h), label, h * 0.54);
        h
    }

    /// A direction in a legend: bare arrows with no outline, since the
    /// direction may come from the d-pad, the stick, the arrow keys or the
    /// keys the player defined. Returns the width they take.
    pub fn arrows(&mut self, canvas: &mut Canvas, x: f32, y: f32, h: f32) -> f32 {
        let size = h * 0.7;
        let mut at = x;
        for arrow in ["←", "→", "↑", "↓"] {
            let span = Span {
                text: arrow,
                size,
                weight: Weight::SemiBold,
                colour: palette::BADGE_TEXT,
            };
            let w = self.measure(std::slice::from_ref(&span));
            let ty = y + (h - size * 1.21) / 2.0;
            self.text(Some(canvas), at, ty, None, 1.0, &[span]);
            at += w + h * 0.3;
        }
        at - x - h * 0.3
    }
}

impl Canvas<'_> {
    fn blend(&mut self, x: isize, y: isize, colour: Rgb, alpha: u8) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        // Over, with the frame's colours premultiplied by its alpha, so a
        // transparent frame can be laid over something else afterwards.
        let i = (y as usize * self.width + x as usize) * 4;
        let a = u16::from(alpha);
        for (k, &c) in colour.iter().enumerate() {
            let under = u16::from(self.pixels[i + k]);
            self.pixels[i + k] = ((u16::from(c) * a + under * (255 - a)) / 255) as u8;
        }
        let under = u16::from(self.pixels[i + 3]);
        self.pixels[i + 3] = (a + under * (255 - a) / 255) as u8;
    }

    pub fn clear(&mut self, colour: Rgb) {
        self.pixels
            .as_chunks_mut::<4>()
            .0
            .fill([colour[0], colour[1], colour[2], 0xFF]);
    }

    /// Darkens the whole frame: black laid over it at `alpha` out of 255.
    pub fn dim(&mut self, alpha: u8) {
        let keep = u16::from(255 - alpha);
        for p in self.pixels.as_chunks_mut::<4>().0 {
            for c in &mut p[..3] {
                *c = ((u16::from(*c) * keep) / 255) as u8;
            }
        }
    }

    /// Copies an RGBA picture `sw` by `sh` into the frame at (`x`, `y`) in
    /// device pixels, each of its pixels `k` device pixels square, sharp.
    pub fn blit_scaled(
        &mut self,
        picture: &[u8],
        sw: usize,
        sh: usize,
        x: usize,
        y: usize,
        k: usize,
    ) {
        let src = picture.as_chunks::<4>().0;
        let dst = self.pixels.as_chunks_mut::<4>().0;
        for sy in 0..sh {
            for dy in 0..k {
                let row = y + sy * k + dy;
                if row >= self.height {
                    break;
                }
                for sx in 0..sw {
                    let p = src[sy * sw + sx];
                    for dx in 0..k {
                        let col = x + sx * k + dx;
                        if col < self.width {
                            dst[row * self.width + col] = p;
                        }
                    }
                }
            }
        }
    }

    /// A filled rectangle with rounded corners, in logical pixels.
    pub fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Rgb) {
        self.shape(x, y, w, h, radius, colour, 255, |_, _| true);
    }

    /// The outline of a rounded rectangle, `thickness` logical pixels wide,
    /// dashed when `dash` is given (the length of a dash and of a gap).
    #[allow(
        clippy::too_many_arguments,
        reason = "a rectangle, its corners, its line and its dash"
    )]
    pub fn outline(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        thickness: f32,
        dash: Option<f32>,
        colour: Rgb,
    ) {
        let s = self.scale;
        let t = thickness * s;
        let (x0, y0, x1, y1) = (x * s, y * s, (x + w) * s, (y + h) * s);
        let r = radius * s;
        self.shape(x, y, w, h, radius, colour, 255, move |px, py| {
            // Inside the shape; on the line if the same shape shrunk by the
            // thickness does not cover it.
            let inner = inside(px, py, x0 + t, y0 + t, x1 - t, y1 - t, (r - t).max(0.0));
            if inner {
                return false;
            }
            match dash {
                None => true,
                Some(d) => {
                    // Dash along whichever edge the point is nearest.
                    let along = if px - x0 < t || x1 - px < t { py } else { px };
                    (along / (d * s)).floor() as i64 % 2 == 0
                }
            }
        });
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "a rectangle, its corners and its colour"
    )]
    fn shape(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        colour: Rgb,
        alpha: u8,
        keep: impl Fn(f32, f32) -> bool,
    ) {
        let s = self.scale;
        let (x0, y0, x1, y1) = (x * s, y * s, (x + w) * s, (y + h) * s);
        let r = radius * s;
        let rows = (y0.floor().max(0.0) as usize)..(y1.ceil() as usize).min(self.height);
        for py in rows {
            for px in (x0.floor().max(0.0) as usize)..(x1.ceil() as usize).min(self.width) {
                // Four samples per pixel, for edges that are not jagged.
                let mut hits = 0u8;
                for (dx, dy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                    let (sx, sy) = (px as f32 + dx, py as f32 + dy);
                    if inside(sx, sy, x0, y0, x1, y1, r) && keep(sx, sy) {
                        hits += 1;
                    }
                }
                if hits > 0 {
                    self.blend(
                        px as isize,
                        py as isize,
                        colour,
                        ([0u16, 64, 128, 191, 255][hits as usize] * u16::from(alpha) / 255) as u8,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments, reason = "a point and a rounded rectangle")]
fn inside(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    if px < x0 || px >= x1 || py < y0 || py >= y1 {
        return false;
    }
    let cx = px.clamp(x0 + r, x1 - r);
    let cy = py.clamp(y0 + r, y1 - r);
    (px - cx).powi(2) + (py - cy).powi(2) <= r * r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_drawn_and_measured() {
        let mut fonts = Fonts::load();
        let (w, h) = (200, 40);
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale: 1.0,
        };
        canvas.clear([0, 0, 0]);
        let span = |text| Span {
            text,
            size: 20.0,
            weight: Weight::Regular,
            colour: [255, 255, 255],
        };
        fonts.text(Some(&mut canvas), 4.0, 4.0, None, 1.0, &[span("Starquake")]);
        assert!(
            pixels.as_chunks::<4>().0.iter().any(|p| p[0] > 200),
            "nothing was drawn"
        );
        assert!(fonts.measure(&[span("WWW")]) > fonts.measure(&[span("iii")]));
    }

    #[test]
    fn badges_are_sized_by_their_label_and_a_button_is_round() {
        let mut fonts = Fonts::load();
        let (w, h) = (200, 40);
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale: 1.0,
        };
        let ctrl = fonts.key_badge(&mut canvas, 2.0, 2.0, 26.0, "Ctrl");
        let a = fonts.key_badge(&mut canvas, 60.0, 2.0, 26.0, "A");
        let x = fonts.button_badge(&mut canvas, 100.0, 2.0, 26.0, "X");
        let arrows = fonts.arrows(&mut canvas, 130.0, 2.0, 26.0);
        assert!(ctrl > a, "a longer name makes a wider key");
        assert!(a >= 26.0, "a key is at least as wide as it is high");
        assert_eq!(x, 26.0, "a button is a circle");
        assert!(arrows > 40.0, "four arrows drawn in a row");
        let at = |px: usize, py: usize| pixels[(py * w + px) * 4];
        assert_eq!(at(100, 2), 0, "the circle's corner is outside it");
        assert!(at(113, 15) > 0, "the circle's middle is drawn on");
    }

    #[test]
    fn dimming_darkens_and_a_scaled_blit_is_sharp() {
        let (w, h) = (8, 8);
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale: 1.0,
        };
        canvas.clear([200, 100, 0]);
        canvas.dim(127);
        assert_eq!(&canvas.pixels[..4], &[100, 50, 0, 0xFF]);
        let picture = [[255, 0, 0, 255], [0, 255, 0, 255]].concat();
        canvas.blit_scaled(&picture, 2, 1, 1, 1, 3);
        let at = |px: usize, py: usize| &pixels[(py * w + px) * 4..(py * w + px) * 4 + 3];
        assert_eq!(at(1, 1), &[255, 0, 0]);
        assert_eq!(at(3, 3), &[255, 0, 0]);
        assert_eq!(at(4, 1), &[0, 255, 0]);
        assert_eq!(at(6, 3), &[0, 255, 0]);
        assert_eq!(at(0, 0), &[100, 50, 0], "outside the blit is untouched");
        assert_eq!(at(7, 4), &[100, 50, 0]);
    }

    #[test]
    fn a_rounded_rectangle_has_round_corners() {
        let (w, h) = (20, 20);
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale: 1.0,
        };
        canvas.round_rect(0.0, 0.0, 20.0, 20.0, 8.0, [255, 255, 255]);
        let at = |x: usize, y: usize| pixels[(y * w + x) * 4];
        assert_eq!(at(0, 0), 0, "the corner is cut");
        assert_eq!(at(10, 10), 255, "the middle is filled");
    }
}
