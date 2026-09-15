//! A picture to draw on and write as a PNG: pixels, boxes, lines and tiny
//! digits, enough for the planet picture.

/// A picture of 0RGB pixels.
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl Image {
    /// A picture filled with `colour`.
    #[must_use]
    pub fn new(width: usize, height: usize, colour: u32) -> Image {
        Image {
            width,
            height,
            pixels: vec![colour; width * height],
        }
    }

    /// Sets a pixel; outside the picture is ignored.
    pub fn set(&mut self, x: i64, y: i64, colour: u32) {
        if x >= 0 && y >= 0 {
            let (x, y) = (x as usize, y as usize);
            if x < self.width && y < self.height {
                self.pixels[y * self.width + x] = colour;
            }
        }
    }

    /// Fills a box.
    pub fn fill(&mut self, x: i64, y: i64, w: i64, h: i64, colour: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, colour);
            }
        }
    }

    /// Outlines a box, `thick` pixels wide, inside its bounds.
    pub fn outline(&mut self, x: i64, y: i64, w: i64, h: i64, thick: i64, colour: u32) {
        self.fill(x, y, w, thick, colour);
        self.fill(x, y + h - thick, w, thick, colour);
        self.fill(x, y, thick, h, colour);
        self.fill(x + w - thick, y, thick, h, colour);
    }

    /// A line, `thick` pixels wide, dashed if asked.
    #[allow(clippy::too_many_arguments)]
    pub fn line(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        thick: i64,
        dash: bool,
        colour: u32,
    ) {
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0).ceil().max(1.0) as usize;
        for i in 0..=steps {
            if dash && (i / 8) % 2 == 1 {
                continue;
            }
            let t = i as f64 / steps as f64;
            let (x, y) = (
                (x0 + (x1 - x0) * t).round() as i64,
                (y0 + (y1 - y0) * t).round() as i64,
            );
            self.fill(x - thick / 2, y - thick / 2, thick, thick, colour);
        }
    }

    /// A digit, three by five pixels, scaled up by `scale`.
    pub fn digit(&mut self, x: i64, y: i64, digit: u8, scale: i64, colour: u32) {
        const FONT: [[u8; 5]; 10] = [
            [7, 5, 5, 5, 7],
            [2, 6, 2, 2, 7],
            [7, 1, 7, 4, 7],
            [7, 1, 7, 1, 7],
            [5, 5, 7, 1, 1],
            [7, 4, 7, 1, 7],
            [7, 4, 7, 5, 7],
            [7, 1, 1, 1, 1],
            [7, 5, 7, 5, 7],
            [7, 5, 7, 1, 7],
        ];
        for (r, row) in FONT[usize::from(digit)].iter().enumerate() {
            for b in 0..3 {
                if row & (4 >> b) != 0 {
                    self.fill(x + b * scale, y + r as i64 * scale, scale, scale, colour);
                }
            }
        }
    }

    /// A number in those digits, left-aligned at (`x`, `y`).
    pub fn number(&mut self, x: i64, y: i64, n: u16, scale: i64, colour: u32) {
        for (i, ch) in n.to_string().bytes().enumerate() {
            self.digit(x + i as i64 * 4 * scale, y, ch - b'0', scale, colour);
        }
    }

    /// The picture as a PNG.
    #[must_use]
    pub fn png(&self) -> Vec<u8> {
        zx_core::png::encode(&self.pixels, self.width, self.height)
    }
}
