//! Tiny dependency-free PNG writer (uncompressed deflate) for screenshots.

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

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    b << 16 | a
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

    let mut zlib = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    while let Some(block) = blocks.next() {
        zlib.push(if blocks.peek().is_none() { 1 } else { 0 });
        let len = block.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

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

    /// Undoes the stored (uncompressed) deflate blocks of a zlib stream.
    fn inflate_stored(zlib: &[u8]) -> Vec<u8> {
        assert_eq!(&zlib[..2], &[0x78, 0x01]);
        let mut out = Vec::new();
        let mut i = 2;
        loop {
            let last = zlib[i];
            let len = u16::from_le_bytes([zlib[i + 1], zlib[i + 2]]);
            let nlen = u16::from_le_bytes([zlib[i + 3], zlib[i + 4]]);
            assert_eq!(nlen, !len);
            out.extend_from_slice(&zlib[i + 5..i + 5 + len as usize]);
            i += 5 + len as usize;
            if last == 1 {
                break;
            }
        }
        assert_eq!(
            u32::from_be_bytes(zlib[i..i + 4].try_into().unwrap()),
            adler32(&out)
        );
        assert_eq!(i + 4, zlib.len());
        out
    }

    #[test]
    fn the_checksums_match_their_published_values() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn a_small_image_round_trips() {
        let pixels = [0xFF0000, 0x00FF00, 0x0000FF, 0x123456];
        let png = encode(&pixels, 2, 2);
        let chunks = chunks(&png);
        let kinds: Vec<&[u8; 4]> = chunks.iter().map(|c| &c.0).collect();
        assert_eq!(kinds, [b"IHDR", b"IDAT", b"IEND"]);
        assert_eq!(chunks[0].1, [0, 0, 0, 2, 0, 0, 0, 2, 8, 2, 0, 0, 0]);
        let raw = inflate_stored(&chunks[1].1);
        assert_eq!(
            raw,
            [0, 0xFF, 0, 0, 0, 0xFF, 0, 0, 0, 0, 0xFF, 0x12, 0x34, 0x56]
        );
        assert!(chunks[2].1.is_empty());
    }

    #[test]
    fn a_large_image_spans_several_deflate_blocks() {
        // 320x256 RGB with a filter byte per row is well over one 65535-byte
        // stored block.
        let pixels: Vec<u32> = (0..320 * 256).map(|i| i as u32 * 97).collect();
        let png = encode(&pixels, 320, 256);
        let raw = inflate_stored(&chunks(&png)[1].1);
        assert_eq!(raw.len(), (320 * 3 + 1) * 256);
        let at = |x: usize, y: usize| {
            let o = y * (320 * 3 + 1) + 1 + x * 3;
            u32::from(raw[o]) << 16 | u32::from(raw[o + 1]) << 8 | u32::from(raw[o + 2])
        };
        assert_eq!(at(319, 255), pixels[320 * 256 - 1] & 0xFF_FFFF);
        assert_eq!(at(7, 100), pixels[320 * 100 + 7] & 0xFF_FFFF);
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
