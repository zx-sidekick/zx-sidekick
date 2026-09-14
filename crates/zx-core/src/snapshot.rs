//! `.z80` snapshot loader (versions 1, 2 and 3; 48K machines only).
//!
//! Format reference: <https://worldofspectrum.org/faq/reference/z80format.htm>

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub a_: u8,
    pub f_: u8,
    pub b_: u8,
    pub c_: u8,
    pub d_: u8,
    pub e_: u8,
    pub h_: u8,
    pub l_: u8,
    pub ix: u16,
    pub iy: u16,
    pub sp: u16,
    pub pc: u16,
    pub i: u8,
    pub r: u8,
    pub iff1: bool,
    pub iff2: bool,
    pub im: u8,
    pub border: u8,
    /// Contents of 0x4000..=0xFFFF.
    pub ram: Vec<u8>,
}

impl Snapshot {
    /// Full 64K address space with the RAM in place and zeros for the ROM.
    pub fn memory(&self) -> Vec<u8> {
        let mut mem = vec![0u8; 0x4000];
        mem.extend_from_slice(&self.ram);
        mem
    }
}

/// Expands a compressed block. Returns whether it fitted: a run that would
/// overrun the page means the file is malformed, and truncating quietly hid
/// that from the size check the caller makes afterwards.
fn decompress(src: &[u8], out: &mut Vec<u8>, limit: usize) -> bool {
    let mut i = 0;
    while i < src.len() && out.len() < limit {
        // A marker needs both its count and its byte; a trailing fragment is
        // malformed rather than two literals.
        if src[i] == 0xED && i + 1 < src.len() && src[i + 1] == 0xED {
            if i + 3 >= src.len() {
                return false;
            }
            let (count, byte) = (src[i + 2] as usize, src[i + 3]);
            if out.len() + count > limit {
                return false;
            }
            out.extend(std::iter::repeat_n(byte, count));
            i += 4;
        } else {
            out.push(src[i]);
            i += 1;
        }
    }
    // Stopping with source left over is normal: a v1 file ends with a
    // 00 ED ED 00 marker after the last page, and a page that fills exactly
    // leaves it unread. Only the malformed cases above are failures.
    true
}

/// Loads a `.z80` snapshot, v1, v2 or v3.
///
/// # Errors
///
/// If the file is too short for its header, names a machine other than a
/// 48K Spectrum, or holds a compressed page that runs off the end.
pub fn load_z80(data: &[u8]) -> Result<Snapshot, String> {
    if data.len() < 30 {
        return Err("file too short for a .z80 header".into());
    }
    let w = |i: usize| data[i] as u16 | (data[i + 1] as u16) << 8;
    let mut flags = data[12];
    if flags == 0xFF {
        flags = 1;
    }

    let mut s = Snapshot {
        a: data[0],
        f: data[1],
        c: data[2],
        b: data[3],
        l: data[4],
        h: data[5],
        pc: w(6),
        sp: w(8),
        i: data[10],
        r: (data[11] & 0x7F) | ((flags & 1) << 7),
        border: (flags >> 1) & 7,
        e: data[13],
        d: data[14],
        c_: data[15],
        b_: data[16],
        e_: data[17],
        d_: data[18],
        l_: data[19],
        h_: data[20],
        a_: data[21],
        f_: data[22],
        iy: w(23),
        ix: w(25),
        iff1: data[27] != 0,
        iff2: data[28] != 0,
        im: data[29] & 3,
        ram: Vec::with_capacity(0xC000),
    };

    if s.pc != 0 {
        // Version 1: a single 48K block, optionally compressed.
        let body = &data[30..];
        if flags & 0x20 != 0 {
            if !decompress(body, &mut s.ram, 0xC000) {
                return Err("v1 snapshot is malformed or overruns 49152 bytes".into());
            }
        } else {
            s.ram.extend_from_slice(&body[..body.len().min(0xC000)]);
        }
        if s.ram.len() != 0xC000 {
            return Err(format!(
                "v1 snapshot holds {} bytes, expected 49152",
                s.ram.len()
            ));
        }
        return Ok(s);
    }

    // Versions 2 and 3. The extended header's own length lives at 30, and
    // the fields read below reach byte 34, so both have to be there before
    // anything is read: a 30-byte file used to panic here.
    if data.len() < 32 {
        return Err("truncated snapshot: no extended header length".into());
    }
    let ext_len = w(30) as usize;
    let header_end = 32 + ext_len;
    if data.len() < header_end || header_end < 35 {
        return Err("truncated extended header".into());
    }
    s.pc = w(32);
    let hw = data[34];
    let is_48k = match ext_len {
        23 => hw <= 1,
        _ => hw <= 1 || hw == 3,
    };
    if !is_48k {
        return Err(format!(
            "snapshot is for hardware mode {hw}; only 48K snapshots are supported"
        ));
    }

    s.ram = vec![0; 0xC000];
    let mut pos = header_end;
    while pos + 3 <= data.len() {
        let len = w(pos) as usize;
        let page = data[pos + 2];
        pos += 3;
        let (raw, compressed) = if len == 0xFFFF {
            (0x4000, false)
        } else {
            (len, true)
        };
        let src = data.get(pos..pos + raw).ok_or("truncated memory block")?;
        pos += raw;
        let offset = match page {
            8 => 0x0000,
            4 => 0x4000,
            5 => 0x8000,
            _ => continue,
        };
        let mut block = Vec::with_capacity(0x4000);
        if compressed {
            if !decompress(src, &mut block, 0x4000) {
                return Err(format!(
                    "memory page {page} is malformed or overruns 16384 bytes"
                ));
            }
        } else {
            block.extend_from_slice(src);
        }
        if block.len() != 0x4000 {
            return Err(format!(
                "memory page {page} decompressed to {} bytes",
                block.len()
            ));
        }
        s.ram[offset..offset + 0x4000].copy_from_slice(&block);
    }
    Ok(s)
}
