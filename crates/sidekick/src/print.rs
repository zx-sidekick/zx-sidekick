//! PRINT-A-2's behaviour on the screen: the Spectrum ROM's character output
//! for the upper screen, with the control codes games use.
//!
//! - `0x16 row col` (AT) moves the print position.
//! - `0x10`–`0x13 n` (INK, PAPER, FLASH, BRIGHT) set *temporary* colours: an
//!   attribute value plus a mask of bits to keep from the screen (`n = 8`,
//!   "transparent").
//! - `0x14 n` / `0x15 n` (INVERSE, OVER) invert glyphs / XOR them onto the
//!   screen.
//! - `0x08` moves back one place.
//! - Printing past column 31 wraps to the next line on the next character.
//!
//! A control code's operands can come in later calls. The ROM remembers that
//! it is waiting for them in memory, not in a register: the current channel's
//! output routine address is switched while it waits, and the code and first
//! operand go in `TVDATA`. This keeps the same state in the same places, so a
//! game that resets the channel (Starquake does, between strings) gets what
//! the ROM would do.
//!
//! Glyphs come from where the system variables say: `CHARS` for `0x20`–`0x7F`,
//! `UDG` for `0x90`–`0xA4`, and the block graphics the ROM builds for
//! `0x80`–`0x8F`. The print position and colours live in the system variables
//! too, read and written on every character, so a game that reads them sees
//! what the ROM would leave.
//!
//! Adapted from starquake-recompiled's printer (see `REUSED.md`): there it
//! drew into a copy of the screen, here into the emulated machine's memory.

use zx_spectrum::Zx;

/// `CHARS`: 256 less than the address of the font's space character.
const CHARS: u16 = 0x5C36;
/// `UDG`: the user-defined graphics.
const UDG: u16 = 0x5C7B;
/// `DF_CC`: the display-file address of the print position.
const DF_CC: u16 = 0x5C84;
/// `S_POSN`: 33 less the column, then 24 less the line.
const S_POSN: u16 = 0x5C88;
/// `ATTR_T`, `MASK_T` and `P_FLAG`: the temporary colours and print flags.
const ATTR_T: u16 = 0x5C8F;
const MASK_T: u16 = 0x5C90;
const P_FLAG: u16 = 0x5C91;

const P_OVER: u8 = 0x01;
const P_INVERSE: u8 = 0x04;
const P_INK9: u8 = 0x10;
const P_PAPER9: u8 = 0x40;

/// T-states a printed character takes in the ROM, and a control code or its
/// operand. Measured against the ROM in development (`games/starquake/check`).
const GLYPH_T: u32 = 1610;
const CODE_T: u32 = 541;

/// `CURCHL`: the address of the current channel, whose first word is the
/// routine its output goes to.
const CURCHL: u16 = 0x5C51;
/// `TVDATA`: the control code waiting for operands, then the first operand.
const TVDATA: u16 = 0x5C0E;
/// The output routine addresses the ROM switches the channel between:
/// ordinary output, waiting for the first of two operands, and waiting for
/// the last operand.
const PRINT_OUT: u16 = 0x09F4;
const PO_TV_2: u16 = 0x0A6D;
const PO_CONT: u16 = 0x0A87;

/// The ROM's block graphics, `0x80`–`0x8F`, built from the low four bits:
/// bit 0 the top right quarter of the cell, then top left, bottom right,
/// bottom left.
fn block_graphic(n: u8) -> [u8; 8] {
    let half =
        |bits: u8| (if bits & 1 != 0 { 0x0F } else { 0 }) | (if bits & 2 != 0 { 0xF0 } else { 0 });
    let (top, bottom) = (half(n), half(n >> 2));
    [top, top, top, top, bottom, bottom, bottom, bottom]
}

/// The eight bytes at `addr`.
fn glyph_at(z: &Zx, addr: u16) -> [u8; 8] {
    std::array::from_fn(|i| z.mem[addr.wrapping_add(i as u16) as usize])
}

/// Prints the byte in A, as PRINT-A-2 does, and returns the T-states it
/// takes.
pub fn put(z: &mut Zx, b: u8) -> u32 {
    let channel = z.read16(CURCHL);
    match z.read16(channel) {
        PO_TV_2 => {
            z.mem[TVDATA as usize + 1] = b;
            z.write16(channel, PO_CONT);
            return CODE_T;
        }
        PO_CONT => {
            z.write16(channel, PRINT_OUT);
            let code = z.mem[TVDATA as usize];
            if code == 0x16 {
                let row = z.mem[TVDATA as usize + 1];
                set_position(z, row.min(23), b.min(31));
            } else {
                control(z, code, b);
            }
            return CODE_T;
        }
        _ => {}
    }
    match b {
        0x08 => {
            let (row, col) = position(z);
            if col == 0 {
                set_position(z, row.saturating_sub(1), 31);
            } else {
                set_position(z, row, col - 1);
            }
            CODE_T
        }
        0x10..=0x15 => {
            z.mem[TVDATA as usize] = b;
            z.write16(channel, PO_CONT);
            CODE_T
        }
        0x16 => {
            z.mem[TVDATA as usize] = b;
            z.write16(channel, PO_TV_2);
            CODE_T
        }
        0x20..=0x7F => {
            let base = z.read16(CHARS).wrapping_add(u16::from(b) * 8);
            glyph(z, &glyph_at(z, base));
            GLYPH_T
        }
        0x80..=0x8F => {
            glyph(z, &block_graphic(b - 0x80));
            GLYPH_T
        }
        0x90..=0xA4 => {
            let base = z.read16(UDG).wrapping_add(u16::from(b - 0x90) * 8);
            glyph(z, &glyph_at(z, base));
            GLYPH_T
        }
        // Other control codes and the BASIC keyword tokens: not used by the
        // games ZX Sidekick runs, and printed as nothing.
        _ => CODE_T,
    }
}

/// The print position as (row, column), the column 32 when the next character
/// wraps.
fn position(z: &Zx) -> (u8, u8) {
    let col = 33u8.saturating_sub(z.mem[S_POSN as usize]);
    let row = 24u8.saturating_sub(z.mem[S_POSN as usize + 1]);
    (row, col)
}

/// Moves the print position, keeping `S_POSN` and `DF_CC` in step.
fn set_position(z: &mut Zx, row: u8, col: u8) {
    z.mem[S_POSN as usize] = 33 - col;
    z.mem[S_POSN as usize + 1] = 24 - row;
    let at = cell(row, col.min(31));
    z.write16(DF_CC, at);
}

/// The display-file address of the top line of character cell (`row`, `col`).
fn cell(row: u8, col: u8) -> u16 {
    0x4000 + (u16::from(row / 8) << 11) + (u16::from(row % 8) << 5) + u16::from(col)
}

fn control(z: &mut Zx, code: u8, n: u8) {
    let (mut attr_t, mut mask_t, mut p_flag) = (
        z.mem[ATTR_T as usize],
        z.mem[MASK_T as usize],
        z.mem[P_FLAG as usize],
    );
    match code {
        0x10 | 0x11 => {
            let (bits, value, contrast) = if code == 0x10 {
                (0x07, n & 7, P_INK9)
            } else {
                (0x38, (n & 7) << 3, P_PAPER9)
            };
            match n {
                0..=7 => {
                    attr_t = (attr_t & !bits) | value;
                    mask_t &= !bits;
                    p_flag &= !contrast;
                }
                8 => {
                    mask_t |= bits;
                    p_flag &= !contrast;
                }
                _ => {
                    mask_t |= bits;
                    p_flag |= contrast;
                }
            }
        }
        0x12 | 0x13 => {
            let bit = if code == 0x12 { 0x80 } else { 0x40 };
            match n {
                0 | 1 => {
                    attr_t = (attr_t & !bit) | if n == 1 { bit } else { 0 };
                    mask_t &= !bit;
                }
                _ => mask_t |= bit,
            }
        }
        _ => {
            let bit = if code == 0x14 { P_INVERSE } else { P_OVER };
            p_flag = (p_flag & !bit) | if n & 1 != 0 { bit } else { 0 };
        }
    }
    z.mem[ATTR_T as usize] = attr_t;
    z.mem[MASK_T as usize] = mask_t;
    z.mem[P_FLAG as usize] = p_flag;
}

/// Draws `glyph` at the print position and moves it on.
fn glyph(z: &mut Zx, glyph: &[u8; 8]) {
    let (mut row, mut col) = position(z);
    if col >= 32 {
        col = 0;
        row += 1;
    }
    if row >= 24 {
        // Off the bottom: the ROM would scroll. No game here prints there;
        // keep the column moving so a caller never waits on it.
        set_position(z, 23, col.min(31));
        return;
    }
    let p_flag = z.mem[P_FLAG as usize];
    let invert = if p_flag & P_INVERSE != 0 { 0xFF } else { 0 };
    let over = p_flag & P_OVER != 0;
    let base = cell(row, col) as usize;
    for (line, &g) in glyph.iter().enumerate() {
        let at = base + (line << 8);
        let old = if over { z.mem[at] } else { 0 };
        z.mem[at] = g ^ invert ^ old;
    }
    let at = 0x5800 + usize::from(row) * 32 + usize::from(col);
    let (attr_t, mask_t) = (z.mem[ATTR_T as usize], z.mem[MASK_T as usize]);
    let mut attr = (z.mem[at] & mask_t) | (attr_t & !mask_t);
    if p_flag & P_PAPER9 != 0 {
        attr &= 0xC7;
        if attr & 0x04 == 0 {
            attr ^= 0x38;
        }
    }
    if p_flag & P_INK9 != 0 {
        attr &= 0xF8;
        if attr & 0x20 == 0 {
            attr ^= 0x07;
        }
    }
    z.mem[at] = attr;
    set_position(z, row, col + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    type Machine = crate::Machine<()>;

    /// A machine set up as the ROM leaves the upper screen: the channel on
    /// ordinary output, the position at the top left, white ink on black, and
    /// a font at 0x9000 whose every character is a stripe.
    fn screen() -> Machine {
        let mut m = Machine::blank(0x8000, 0x7000);
        let z = &mut m.zx;
        z.write16(CURCHL, 0x6000);
        z.write16(0x6000, PRINT_OUT);
        z.mem[S_POSN as usize] = 33;
        z.mem[S_POSN as usize + 1] = 24;
        z.mem[ATTR_T as usize] = 0x07;
        z.write16(CHARS, 0x9000 - 0x100);
        z.mem[0x9000..0x9000 + 96 * 8].fill(0xAA);
        m
    }

    fn print(m: &mut Machine, bytes: &[u8]) {
        for &b in bytes {
            put(&mut m.zx, b);
        }
    }

    #[test]
    fn a_character_draws_its_glyph_and_colour_and_moves_on() {
        let mut m = screen();
        print(&mut m, b"A");
        let z = &m.zx;
        for line in 0..8 {
            assert_eq!(z.mem[0x4000 + (line << 8)], 0xAA);
        }
        assert_eq!(z.mem[0x5800], 0x07);
        assert_eq!(position(z), (0, 1));
    }

    #[test]
    fn at_moves_the_print_position_over_three_calls() {
        let mut m = screen();
        print(&mut m, &[0x16, 10]);
        assert_eq!(
            m.zx.read16(m.zx.read16(CURCHL)),
            PO_CONT,
            "waiting for the column"
        );
        print(&mut m, &[5, b'A']);
        let z = &m.zx;
        assert_eq!(z.mem[0x5800 + 10 * 32 + 5], 0x07);
        assert_eq!(z.mem[cell(10, 5) as usize], 0xAA);
        assert_eq!(position(z), (10, 6));
    }

    #[test]
    fn a_reset_channel_cancels_a_waiting_control_code() {
        let mut m = screen();
        print(&mut m, &[0x10]);
        // The game puts the channel back on ordinary output.
        let channel = m.zx.read16(CURCHL);
        m.zx.write16(channel, PRINT_OUT);
        print(&mut m, &[0x10, 2]);
        assert_eq!(m.zx.mem[ATTR_T as usize] & 7, 2, "INK 2");
    }

    /// The eight display-file bytes of character cell (`row`, `col`).
    fn cell_bytes(m: &Machine, row: u8, col: u8) -> [u8; 8] {
        let base = cell(row, col) as usize;
        std::array::from_fn(|line| m.zx.mem[base + (line << 8)])
    }

    #[test]
    fn block_graphics_fill_the_quarters_their_bits_name() {
        let mut m = screen();
        // 0x81: top right; 0x82: top left; 0x84: bottom right; 0x8F: all.
        print(&mut m, &[0x81, 0x82, 0x84, 0x8F, 0x80]);
        let top = |b| [b, b, b, b, 0, 0, 0, 0];
        assert_eq!(cell_bytes(&m, 0, 0), top(0x0F));
        assert_eq!(cell_bytes(&m, 0, 1), top(0xF0));
        assert_eq!(cell_bytes(&m, 0, 2), [0, 0, 0, 0, 0x0F, 0x0F, 0x0F, 0x0F]);
        assert_eq!(cell_bytes(&m, 0, 3), [0xFF; 8]);
        assert_eq!(cell_bytes(&m, 0, 4), [0; 8]);
    }

    #[test]
    fn user_defined_graphics_come_from_udg() {
        let mut m = screen();
        m.zx.write16(UDG, 0xA000);
        m.zx.mem[0xA008..0xA010].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        print(&mut m, &[0x91]);
        assert_eq!(cell_bytes(&m, 0, 0), [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn backspace_moves_back_and_up_a_line_from_the_left_edge() {
        let mut m = screen();
        print(&mut m, &[0x16, 3, 0, 0x08]);
        assert_eq!(position(&m.zx), (2, 31));
        print(&mut m, &[0x08]);
        assert_eq!(position(&m.zx), (2, 30));
        print(&mut m, &[0x16, 0, 0, 0x08]);
        assert_eq!(position(&m.zx), (0, 31), "nothing above the top line");
    }

    #[test]
    fn other_codes_and_tokens_print_nothing() {
        let mut m = screen();
        for b in [0x00, 0x07, 0x0D, 0x17, 0x1F, 0xA5, 0xFF] {
            assert_eq!(put(&mut m.zx, b), CODE_T);
        }
        assert_eq!(position(&m.zx), (0, 0));
        assert!(m.zx.mem[0x4000..0x5800].iter().all(|&b| b == 0));
    }

    #[test]
    fn a_full_line_wraps_on_the_next_character() {
        let mut m = screen();
        print(&mut m, &[0x16, 5, 31, b'A']);
        assert_eq!(position(&m.zx), (5, 32), "waits at the end of the line");
        print(&mut m, b"B");
        assert_eq!(position(&m.zx), (6, 1));
        assert_eq!(m.zx.mem[cell(6, 0) as usize], 0xAA);
    }

    #[test]
    fn printing_past_the_bottom_draws_nothing() {
        let mut m = screen();
        print(&mut m, &[0x16, 23, 31, b'A', b'B']);
        assert_eq!(position(&m.zx), (23, 0));
        assert_eq!(m.zx.mem[cell(23, 0) as usize], 0);
    }

    #[test]
    fn at_is_limited_to_the_screen() {
        let mut m = screen();
        print(&mut m, &[0x16, 40, 99]);
        assert_eq!(position(&m.zx), (23, 31));
        assert_eq!(m.zx.read16(DF_CC), cell(23, 31));
    }

    #[test]
    fn paper_flash_and_bright_set_their_bits() {
        let mut m = screen();
        print(&mut m, &[0x11, 5, 0x12, 1, 0x13, 1, b'A']);
        assert_eq!(m.zx.mem[0x5800], 0x80 | 0x40 | (5 << 3) | 7);
        print(&mut m, &[0x12, 0, 0x13, 0, b'A']);
        assert_eq!(m.zx.mem[0x5801], (5 << 3) | 7);
    }

    #[test]
    fn transparent_flash_and_bright_keep_the_screen_s() {
        let mut m = screen();
        m.zx.mem[0x5800] = 0xC0;
        print(&mut m, &[0x12, 8, 0x13, 8, b'A']);
        assert_eq!(m.zx.mem[0x5800], 0xC7);
    }

    #[test]
    fn ink_9_contrasts_with_the_paper() {
        let mut m = screen();
        // Dark paper (blue): white ink. Light paper (yellow): black ink.
        print(&mut m, &[0x10, 9, 0x11, 1, b'A', 0x11, 6, b'A']);
        assert_eq!(m.zx.mem[0x5800] & 0x3F, (1 << 3) | 7);
        assert_eq!(m.zx.mem[0x5801] & 0x3F, 6 << 3);
        // Setting an ink turns contrast off again.
        print(&mut m, &[0x10, 2, b'A']);
        assert_eq!(m.zx.mem[0x5802] & 7, 2);
    }

    #[test]
    fn paper_9_contrasts_with_the_ink() {
        let mut m = screen();
        print(&mut m, &[0x11, 9, 0x10, 1, b'A', 0x10, 6, b'A']);
        assert_eq!(m.zx.mem[0x5800] & 0x3F, (7 << 3) | 1);
        assert_eq!(m.zx.mem[0x5801] & 0x3F, 6);
    }

    #[test]
    fn inverse_and_over_change_how_a_glyph_lands() {
        let mut m = screen();
        print(&mut m, &[0x14, 1, b'A']);
        assert_eq!(cell_bytes(&m, 0, 0), [0x55; 8], "inverse");
        print(&mut m, &[0x14, 0, 0x16, 0, 0, 0x15, 1, b'A']);
        assert_eq!(cell_bytes(&m, 0, 0), [0xFF; 8], "over XORs onto the screen");
        print(&mut m, &[0x15, 0, 0x16, 0, 0, b'A']);
        assert_eq!(cell_bytes(&m, 0, 0), [0xAA; 8], "over off replaces it");
    }

    #[test]
    fn a_glyph_and_a_control_code_take_the_rom_s_time() {
        let mut m = screen();
        assert_eq!(put(&mut m.zx, b'A'), GLYPH_T);
        assert_eq!(put(&mut m.zx, 0x16), CODE_T);
        assert_eq!(put(&mut m.zx, 1), CODE_T);
        assert_eq!(put(&mut m.zx, 1), CODE_T);
    }

    #[test]
    fn transparent_ink_keeps_the_screen_s_ink() {
        let mut m = screen();
        m.zx.mem[0x5800] = 0x3A;
        print(&mut m, &[0x10, 8, 0x11, 1, b'A']);
        assert_eq!(m.zx.mem[0x5800], 0x0A, "ink 2 kept, paper 1 set");
    }
}
