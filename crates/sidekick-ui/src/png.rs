//! A small PNG encoder: RGB, one IDAT, deterministic, so golden images compare byte for byte.

/// Encodes opaque RGBA pixels (`width` × `height`, row by row) as an RGB PNG.
#[must_use]
pub fn encode(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row_len = usize::try_from(width).unwrap_or(0) * 4;
    let mut rows = Vec::with_capacity(rgba.len() / 4 * 3 + rgba.len() / row_len.max(1));
    for row in rgba.chunks_exact(row_len.max(1)) {
        rows.push(0); // filter type 0: none
        rows.extend(row.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]));
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB, deflate, no filter, no interlace
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, *b"IHDR", &header);
    chunk(
        &mut png,
        *b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&rows, 6),
    );
    chunk(&mut png, *b"IEND", &[]);
    png
}

/// Appends a PNG chunk: its length, type, data and CRC.
fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    png.extend_from_slice(&len.to_be_bytes());
    png.extend_from_slice(&kind);
    png.extend_from_slice(data);
    png.extend_from_slice(&crc32(kind.iter().chain(data)).to_be_bytes());
}

/// The CRC-32 PNG chunks carry (ISO 3309, polynomial 0xEDB88320).
fn crc32<'a>(bytes: impl Iterator<Item = &'a u8>) -> u32 {
    !bytes.fold(!0u32, |crc, &byte| {
        (0..8).fold(crc ^ u32::from(byte), |c, _| {
            if c & 1 == 1 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_is_png_s() {
        assert_eq!(crc32(b"IEND".iter()), 0xAE42_6082);
        assert_eq!(crc32(b"123456789".iter()), 0xCBF4_3926);
    }

    #[test]
    fn a_png_starts_with_its_signature_and_header() {
        let png = encode(&[255, 0, 0, 255, 0, 255, 0, 255], 2, 1);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 1]);
    }
}
