//! A small PNG writer for screenshots and pictures: RGB, one row filter,
//! the image data deflated (#81).

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Encodes a 0RGB pixel buffer as a PNG.
///
/// # Panics
///
/// If either dimension is zero, or `pixels` holds fewer than
/// `width * height` entries.
pub fn encode(pixels: &[u32], width: usize, height: usize) -> Vec<u8> {
    // `chunks` panics on a zero width, and a buffer shorter than the image
    // would emit fewer rows than the header promises.
    assert!(
        width > 0 && height > 0,
        "a {width}x{height} image has no pixels"
    );
    assert!(
        pixels.len() >= width * height,
        "{} pixels is short of the {width}x{height} declared",
        pixels.len()
    );
    let mut raw = Vec::with_capacity((width * 3 + 1) * height);
    for row in pixels.chunks(width).take(height) {
        raw.push(0); // filter: none
        for &p in row {
            raw.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }

    // Deflated, as every PNG reader expects and as makes a screenshot a few
    // hundred kilobytes rather than several megabytes (#81).
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &raw).expect("writing to memory");
    let zlib = encoder.finish().expect("writing to memory");

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &zlib);
    chunk(&mut png, b"IEND", &[]);
    png
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chunks of a PNG, as (kind, data), checking each one's CRC.
    fn chunks(png: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let mut out = Vec::new();
        let mut i = 8;
        while i < png.len() {
            let len = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
            let kind: [u8; 4] = png[i + 4..i + 8].try_into().unwrap();
            let data = png[i + 8..i + 8 + len].to_vec();
            let crc = u32::from_be_bytes(png[i + 8 + len..i + 12 + len].try_into().unwrap());
            assert_eq!(crc, crc32(&png[i + 4..i + 8 + len]), "CRC of {kind:?}");
            out.push((kind, data));
            i += 12 + len;
        }
        out
    }

    /// Inflates a zlib stream, as a PNG reader would.
    fn inflate(zlib: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(zlib), &mut out)
            .expect("a zlib stream");
        out
    }

    #[test]
    fn the_checksum_matches_its_published_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn a_small_image_round_trips() {
        let pixels = [0xFF0000, 0x00FF00, 0x0000FF, 0x123456];
        let png = encode(&pixels, 2, 2);
        let chunks = chunks(&png);
        let kinds: Vec<&[u8; 4]> = chunks.iter().map(|c| &c.0).collect();
        assert_eq!(kinds, [b"IHDR", b"IDAT", b"IEND"]);
        assert_eq!(chunks[0].1, [0, 0, 0, 2, 0, 0, 0, 2, 8, 2, 0, 0, 0]);
        let raw = inflate(&chunks[1].1);
        assert_eq!(
            raw,
            [0, 0xFF, 0, 0, 0, 0xFF, 0, 0, 0, 0, 0xFF, 0x12, 0x34, 0x56]
        );
        assert!(chunks[2].1.is_empty());
    }

    #[test]
    fn a_large_image_reads_back_and_a_plain_one_is_small() {
        let pixels: Vec<u32> = (0..320 * 256).map(|i| i as u32 * 97).collect();
        let png = encode(&pixels, 320, 256);
        let raw = inflate(&chunks(&png)[1].1);
        assert_eq!(raw.len(), (320 * 3 + 1) * 256);
        let at = |x: usize, y: usize| {
            let o = y * (320 * 3 + 1) + 1 + x * 3;
            u32::from(raw[o]) << 16 | u32::from(raw[o + 1]) << 8 | u32::from(raw[o + 2])
        };
        assert_eq!(at(319, 255), pixels[320 * 256 - 1] & 0xFF_FFFF);
        assert_eq!(at(7, 100), pixels[320 * 100 + 7] & 0xFF_FFFF);
        // A picture of one colour, as most of a screenshot is, deflates to
        // a sliver of its raw size.
        let plain = encode(&vec![0x0F1117; 320 * 256], 320, 256);
        assert!(plain.len() < 320 * 256 * 3 / 100, "{} bytes", plain.len());
    }

    #[test]
    #[should_panic(expected = "has no pixels")]
    fn a_zero_width_image_is_refused() {
        let _ = encode(&[], 0, 4);
    }

    #[test]
    #[should_panic(expected = "is short of")]
    fn a_short_buffer_is_refused() {
        let _ = encode(&[0; 3], 2, 2);
    }
}
