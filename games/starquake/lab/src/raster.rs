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

    /// The picture as a PNG, compressed: a planet picture is about 65 MB
    /// stored and a megabyte or two this way. Each row is filtered by
    /// the pixel above it, which suits pictures of rooms.
    ///
    /// # Panics
    ///
    /// If the picture has no pixels.
    #[must_use]
    pub fn png(&self) -> Vec<u8> {
        use std::io::Write as _;
        assert!(
            self.width > 0 && self.height > 0,
            "a picture with no pixels"
        );
        let rgb = |p: u32| [(p >> 16) as u8, (p >> 8) as u8, p as u8];
        let mut raw = Vec::with_capacity((self.width * 3 + 1) * self.height);
        let mut above = vec![0u8; self.width * 3];
        for row in self.pixels.chunks(self.width) {
            let bytes: Vec<u8> = row.iter().flat_map(|&p| rgb(p)).collect();
            raw.push(2); // filter: up
            raw.extend(bytes.iter().zip(&above).map(|(b, a)| b.wrapping_sub(*a)));
            above = bytes;
        }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&raw).expect("writing to memory");
        let idat = z.finish().expect("writing to memory");

        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&u32::try_from(self.width).expect("width").to_be_bytes());
        ihdr.extend_from_slice(&u32::try_from(self.height).expect("height").to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        for (kind, data) in [
            (b"IHDR", &ihdr[..]),
            (b"IDAT", &idat[..]),
            (b"IEND", &[][..]),
        ] {
            png.extend_from_slice(&u32::try_from(data.len()).expect("chunk size").to_be_bytes());
            let mut crc = flate2::Crc::new();
            crc.update(kind);
            crc.update(data);
            png.extend_from_slice(kind);
            png.extend_from_slice(data);
            png.extend_from_slice(&crc.sum().to_be_bytes());
        }
        png
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_png_decodes_to_the_same_pixels() {
        let mut img = Image::new(5, 3, 0x102030);
        img.set(4, 2, 0xFFEEDD);
        img.set(0, 1, 0x00FF00);
        let png = img.png();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // The IDAT chunk, inflated and unfiltered, gives back every pixel.
        let len = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let idat_at = 8 + 12 + len(8);
        assert_eq!(&png[idat_at + 4..idat_at + 8], b"IDAT");
        let mut raw = Vec::new();
        std::io::Read::read_to_end(
            &mut flate2::read::ZlibDecoder::new(&png[idat_at + 8..idat_at + 8 + len(idat_at)]),
            &mut raw,
        )
        .unwrap();
        let mut above = vec![0u8; 15];
        for (y, line) in raw.chunks(16).enumerate() {
            assert_eq!(line[0], 2);
            let row: Vec<u8> = line[1..]
                .iter()
                .zip(&above)
                .map(|(b, a)| b.wrapping_add(*a))
                .collect();
            for x in 0..5 {
                let p = img.pixels[y * 5 + x];
                assert_eq!(
                    &row[x * 3..x * 3 + 3],
                    &[(p >> 16) as u8, (p >> 8) as u8, p as u8]
                );
            }
            above = row;
        }
    }
}
