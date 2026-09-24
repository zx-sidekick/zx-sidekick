//! `.tap` tape loader: the blocks a Spectrum would load from cassette.
//!
//! A tape is a sequence of blocks, each a two-byte length followed by that
//! many bytes: a flag (0x00 for a header, 0xFF for data), the payload, and a
//! checksum. A header says what the block after it is and, for code, where
//! it loads.

/// Bytes of a Spectrum screen: bitmap then attributes.
pub const SCREEN_LEN: usize = 6912;

const HEADER: u8 = 0x00;
const DATA: u8 = 0xFF;
const CODE: u8 = 3;
const SCREEN_ADDR: u16 = 0x4000;

/// A loaded tape.
pub struct Tape {
    /// Contents of 0x4000..=0xFFFF, as the code blocks left it.
    pub ram: Vec<u8>,
    /// The picture shown while the rest of the tape loaded, if it has one.
    pub loading_screen: Option<Vec<u8>>,
}

/// Loads every code block on a `.tap`, in the order the Spectrum would.
///
/// # Errors
///
/// If a block runs off the end of the file or fails its checksum, or if the
/// tape places nothing in RAM. The part of a block that would load outside
/// RAM is lost, as it is on a Spectrum, whose loader writes each byte and
/// the ROM keeps none; the rest loads.
pub fn load_tap(bytes: &[u8]) -> Result<Tape, String> {
    let mut ram = vec![0u8; 0xC000];
    let mut loading_screen = None;
    // Where the block after the current header will load.
    let mut pending: Option<(u16, usize)> = None;
    let mut loaded = 0usize;
    // Code blocks seen, as against bytes placed: a zero-length one would
    // otherwise be reported as no code blocks at all.
    let mut blocks = 0usize;
    let mut i = 0usize;

    while i + 2 <= bytes.len() {
        let len = bytes[i] as usize | (bytes[i + 1] as usize) << 8;
        i += 2;
        if len < 2 || i + len > bytes.len() {
            return Err(format!("truncated tape block at {i:#x}"));
        }
        let block = &bytes[i..i + len];
        i += len;
        // The last byte is a checksum: the flag and every other byte XORed
        // together. A bit-flipped header would load in the wrong place.
        let sum = block[..len - 1].iter().fold(0u8, |a, b| a ^ b);
        if sum != block[len - 1] {
            return Err(format!(
                "tape block at {:#x} is corrupt (checksum {:#04x}, expected {sum:#04x})",
                i - len,
                block[len - 1]
            ));
        }

        match block[0] {
            HEADER if len >= 19 => {
                pending = (block[1] == CODE).then(|| {
                    let length = block[12] as usize | (block[13] as usize) << 8;
                    let start = block[14] as u16 | (block[15] as u16) << 8;
                    (start, length)
                });
            }
            DATA => {
                let Some((start, length)) = pending.take() else {
                    continue;
                };
                let data = &block[1..len - 1];
                blocks += 1;
                let n = length.min(data.len());
                let at = start as usize;
                // The loading picture goes to the screen first, and the game
                // lands on top of it later.
                if at == SCREEN_ADDR as usize && n == SCREEN_LEN && loading_screen.is_none() {
                    loading_screen = Some(data[..n].to_vec());
                }
                // Only what lands in RAM stays: the bytes below 0x4000 go to
                // the ROM, and those past 0xFFFF wrap round into it.
                let from = at.max(0x4000);
                let to = (at + n).min(0x10000);
                if from < to {
                    ram[from - 0x4000..to - 0x4000].copy_from_slice(&data[from - at..to - at]);
                    loaded += to - from;
                }
            }
            _ => pending = None,
        }
    }

    if loaded == 0 {
        return Err(if blocks == 0 {
            "no code blocks on the tape".into()
        } else {
            format!("the tape's {blocks} code block(s) placed nothing in RAM")
        });
    }
    Ok(Tape {
        ram,
        loading_screen,
    })
}

/// The blocks of a `.tap` as they are on the tape, each with its flag
/// first and its checksum last, for feeding a real ROM's loader.
///
/// # Errors
///
/// If a block runs off the end of the file, or is too short to hold a flag
/// and a checksum.
pub fn blocks(bytes: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let len = bytes[i] as usize | (bytes[i + 1] as usize) << 8;
        i += 2;
        if len < 2 || i + len > bytes.len() {
            return Err(format!("truncated tape block at {i:#x}"));
        }
        out.push(&bytes[i..i + len]);
        i += len;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tape block: its length, then the flag, the payload and the checksum.
    fn block(flag: u8, payload: &[u8]) -> Vec<u8> {
        let len = (payload.len() + 2) as u16;
        let sum = payload.iter().fold(flag, |a, b| a ^ b);
        let mut out = len.to_le_bytes().to_vec();
        out.push(flag);
        out.extend_from_slice(payload);
        out.push(sum);
        out
    }

    /// A header saying the next block is `kind`, `length` bytes for `start`.
    fn header(kind: u8, length: u16, start: u16) -> Vec<u8> {
        let mut payload = vec![kind];
        payload.extend_from_slice(b"test      ");
        payload.extend_from_slice(&length.to_le_bytes());
        payload.extend_from_slice(&start.to_le_bytes());
        payload.extend_from_slice(&0x8000u16.to_le_bytes());
        block(HEADER, &payload)
    }

    fn code(start: u16, data: &[u8]) -> Vec<u8> {
        let mut out = header(CODE, data.len() as u16, start);
        out.extend(block(DATA, data));
        out
    }

    #[test]
    fn a_code_block_loads_at_its_address() {
        let tape = load_tap(&code(0x8000, &[1, 2, 3])).unwrap();
        assert_eq!(&tape.ram[0x4000..0x4003], &[1, 2, 3]);
        assert!(tape.loading_screen.is_none());
    }

    #[test]
    fn a_screen_block_is_the_loading_picture_and_later_code_lands_on_top() {
        let picture = vec![0xAA; SCREEN_LEN];
        let mut bytes = code(0x4000, &picture);
        bytes.extend(code(0x4000, &[0x55; 4]));
        let tape = load_tap(&bytes).unwrap();
        assert_eq!(tape.loading_screen.as_deref(), Some(&picture[..]));
        assert_eq!(&tape.ram[..5], &[0x55, 0x55, 0x55, 0x55, 0xAA]);
    }

    #[test]
    fn a_block_longer_than_its_header_says_loads_only_what_the_header_says() {
        let mut bytes = header(CODE, 2, 0x9000);
        bytes.extend(block(DATA, &[7, 8, 9]));
        let tape = load_tap(&bytes).unwrap();
        assert_eq!(&tape.ram[0x5000..0x5003], &[7, 8, 0]);
    }

    #[test]
    fn a_bad_checksum_is_an_error() {
        let mut bytes = code(0x8000, &[1, 2, 3]);
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        let err = load_tap(&bytes).err().unwrap();
        assert!(err.contains("corrupt"), "{err}");
    }

    #[test]
    fn a_block_running_off_the_end_is_an_error() {
        let mut bytes = code(0x8000, &[1, 2, 3]);
        bytes.truncate(bytes.len() - 2);
        let err = load_tap(&bytes).err().unwrap();
        assert!(err.contains("truncated"), "{err}");
    }

    #[test]
    fn data_without_a_code_header_is_skipped() {
        // A program header, then its data, then a stray data block: none of
        // them is code, so the tape has nothing to load.
        let mut bytes = header(0, 3, 0);
        bytes.extend(block(DATA, &[1, 2, 3]));
        bytes.extend(block(DATA, &[4, 5, 6]));
        let err = load_tap(&bytes).err().unwrap();
        assert_eq!(err, "no code blocks on the tape");
    }

    #[test]
    fn a_block_for_the_rom_is_skipped_but_counted() {
        let err = load_tap(&code(0x0000, &[1, 2, 3])).err().unwrap();
        assert!(err.contains("placed nothing in RAM"), "{err}");

        // The rest of the tape still loads.
        let mut bytes = code(0x0000, &[1, 2, 3]);
        bytes.extend(code(0xFFFE, &[4, 5]));
        let tape = load_tap(&bytes).unwrap();
        assert_eq!(&tape.ram[0xBFFE..], &[4, 5]);
    }

    #[test]
    fn a_block_past_the_end_of_memory_loads_what_fits() {
        let mut bytes = code(0xFFFE, &[1, 2, 3]);
        bytes.extend(code(0x8000, &[9]));
        let tape = load_tap(&bytes).unwrap();
        assert_eq!(&tape.ram[0xBFFE..], &[1, 2]);
        assert_eq!(tape.ram[0x4000], 9);
    }

    #[test]
    fn a_block_starting_in_the_rom_loads_its_part_in_ram() {
        let tape = load_tap(&code(0x3FFE, &[1, 2, 3, 4])).unwrap();
        assert_eq!(&tape.ram[..2], &[3, 4]);
    }

    #[test]
    fn a_header_with_a_bad_checksum_is_an_error() {
        let mut bytes = header(CODE, 3, 0x8000);
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        bytes.extend(block(DATA, &[1, 2, 3]));
        let err = load_tap(&bytes).err().unwrap();
        assert!(err.contains("corrupt"), "{err}");
    }

    #[test]
    fn a_tape_splits_into_its_blocks_and_a_short_one_is_an_error() {
        let bytes = code(0x8000, &[1, 2, 3]);
        let found = blocks(&bytes).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[1], &[DATA, 1, 2, 3, DATA ^ 1 ^ 2 ^ 3][..]);
        assert!(
            blocks(&bytes[..bytes.len() - 1]).is_err(),
            "runs off the end"
        );
        assert!(
            blocks(&[1, 0, 0xFF]).is_err(),
            "too short for flag and checksum"
        );
    }

    #[test]
    fn an_empty_tape_has_no_code() {
        assert_eq!(load_tap(&[]).err().unwrap(), "no code blocks on the tape");
    }
}
