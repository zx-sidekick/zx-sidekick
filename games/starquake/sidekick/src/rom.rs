//! The Spectrum ROM routines a game calls, answered without a ROM.
//!
//! Each is reached by the program counter arriving at the routine's address,
//! either by a call from the game or, for the interrupt, by the processor. The
//! answer does what the routine does to memory and registers, charges the
//! time it takes, and returns to the caller as the routine's own `RET` would.
//! See `games/starquake/docs/rom.md` for why these three are all Starquake needs.

use zx_spectrum::{CF, Zx};

/// MASK-INT: the 50 Hz maskable interrupt in interrupt mode 1.
pub const MASK_INT: u16 = 0x0038;
/// PRINT-A-2: prints the character in A to the current channel.
pub const PRINT_A_2: u16 = 0x15F2;
/// HL-HL*DE: multiplies HL by DE, setting carry on overflow.
pub const HL_HL_X_DE: u16 = 0x30A9;

/// FRAMES: the three-byte frame counter the interrupt keeps.
pub const FRAMES: u16 = 0x5C78;

/// T-states the interrupt routine takes, counted from its first instruction
/// to its `RET`. The ROM's keyboard scan makes it vary a little with what is
/// held; this is its mean cost over play, measured against the ROM (`games/starquake/sk-check`).
pub const MASK_INT_T: u32 = 882;

/// Answers the routine at the program counter, if it is one of these. Returns whether it
/// did.
pub fn answer(z: &mut Zx) -> bool {
    match z.pc() {
        MASK_INT => {
            mask_int(z);
            true
        }
        PRINT_A_2 => {
            let t = crate::print::put(z, z.a());
            z.spend(t, 1);
            ret(z);
            true
        }
        HL_HL_X_DE => {
            hl_hl_x_de(z);
            true
        }
        _ => false,
    }
}

/// Returns from a routine: pops the program counter.
fn ret(z: &mut Zx) {
    let pc = z.pop();
    z.set_pc(pc);
}

/// Counts the frame, then `EI; RET`.
fn mask_int(z: &mut Zx) {
    let frames =
        (u32::from(z.read16(FRAMES)) | u32::from(z.mem[FRAMES as usize + 2]) << 16).wrapping_add(1);
    z.write16(FRAMES, frames as u16);
    z.mem[FRAMES as usize + 2] = (frames >> 16) as u8;
    z.spend(MASK_INT_T, 10);
    z.set_interrupts(true);
    ret(z);
}

/// Shift-and-add multiplication, sixteen rounds, leaving the registers and
/// flags as the routine does: HL the product, A and the flags from its last
/// step, BC and DE as they were, carry set if the product overflowed.
fn hl_hl_x_de(z: &mut Zx) {
    // Each instruction's T-states and opcode fetches, as it runs.
    let (mut t, mut m1) = (0u32, 0u8);
    let mut charge = |cost: u32, fetches: u8| {
        t += cost;
        m1 += fetches;
    };
    // PUSH BC; LD B,16; LD A,H; LD C,L; LD HL,0
    let bc = z.bc();
    charge(11 + 7 + 4 + 4 + 10, 5);
    z.set_a(z.h());
    z.set_c(z.l());
    z.set_hl(0);
    let mut b = 16u8;
    loop {
        // ADD HL,HL; JR C,end
        let hl = z.add16(z.hl(), z.hl());
        z.set_hl(hl);
        charge(11, 1);
        if z.f() & CF != 0 {
            charge(12, 1);
            break;
        }
        charge(7, 1);
        // RL C; RLA; JR NC,again
        let c = z.rl(z.c());
        z.set_c(c);
        z.rla();
        charge(8 + 4, 3);
        if z.f() & CF != 0 {
            charge(7, 1);
            // ADD HL,DE; JR C,end
            let hl = z.add16(z.hl(), z.de());
            z.set_hl(hl);
            charge(11, 1);
            if z.f() & CF != 0 {
                charge(12, 1);
                break;
            }
            charge(7, 1);
        } else {
            charge(12, 1);
        }
        // again: DJNZ loop
        b -= 1;
        if b == 0 {
            charge(8, 1);
            break;
        }
        charge(13, 1);
    }
    // POP BC; RET
    z.set_bc(bc);
    charge(10 + 10, 2);
    z.spend(t, m1);
    ret(z);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Machine;

    /// A machine about to run `routine`, called from `0x8000`.
    fn calling(routine: u16) -> Machine {
        let mut m = Machine::blank(routine, 0x7000);
        m.zx.push(0x8000);
        m
    }

    #[test]
    fn the_multiply_leaves_the_product_in_hl_and_bc_de_as_they_were() {
        let mut m = calling(HL_HL_X_DE);
        m.zx.set_hl(300);
        m.zx.set_de(200);
        m.zx.set_bc(0x1234);
        assert!(answer(&mut m.zx));
        let z = &m.zx;
        assert_eq!(z.hl(), 60000);
        assert_eq!((z.bc(), z.de()), (0x1234, 200));
        assert_eq!(z.f() & CF, 0, "no overflow");
        assert_eq!((z.pc(), z.sp()), (0x8000, 0x7000), "returned to the caller");
    }

    #[test]
    fn the_multiply_sets_carry_on_overflow() {
        let mut m = calling(HL_HL_X_DE);
        m.zx.set_hl(300);
        m.zx.set_de(300);
        assert!(answer(&mut m.zx));
        assert_ne!(m.zx.f() & CF, 0);
    }

    #[test]
    fn the_interrupt_counts_the_frame_and_enables_interrupts() {
        let mut m = calling(MASK_INT);
        m.zx.write16(FRAMES, 0xFFFF);
        m.zx.mem[FRAMES as usize + 2] = 7;
        assert!(answer(&mut m.zx));
        let z = &m.zx;
        assert_eq!((z.read16(FRAMES), z.mem[FRAMES as usize + 2]), (0, 8));
        assert!(z.iff1());
        assert_eq!(z.pc(), 0x8000);
    }

    #[test]
    fn the_multiply_is_exact_until_it_overflows_and_says_when_it_does() {
        let values = [
            0u16, 1, 2, 3, 7, 255, 256, 257, 1000, 0x7FFF, 0x8000, 0xFFFF,
        ];
        for &hl in &values {
            for &de in &values {
                let mut m = calling(HL_HL_X_DE);
                m.zx.set_hl(hl);
                m.zx.set_de(de);
                assert!(answer(&mut m.zx));
                let product = u32::from(hl) * u32::from(de);
                let overflow = product > 0xFFFF;
                assert_eq!(m.zx.f() & CF != 0, overflow, "{hl} x {de}");
                if !overflow {
                    assert_eq!(u32::from(m.zx.hl()), product, "{hl} x {de}");
                }
                assert_eq!(m.zx.de(), de);
                assert_eq!(m.zx.pc(), 0x8000);
            }
        }
    }

    #[test]
    fn the_multiply_takes_longer_for_each_bit_set_in_hl() {
        // Each set bit of HL adds DE in, which takes time; none overflow.
        let time = |hl: u16| {
            let mut m = calling(HL_HL_X_DE);
            m.zx.set_hl(hl);
            m.zx.set_de(1);
            assert!(answer(&mut m.zx));
            m.zx.t
        };
        assert!(time(0x00FF) > time(0x000F));
        assert!(time(0x000F) > time(0x0001));
        assert_eq!(time(0x0010), time(0x0001), "the same number of set bits");
    }

    #[test]
    fn printing_is_answered_and_returns_to_the_caller() {
        let mut m = calling(PRINT_A_2);
        m.zx.write16(0x5C51, 0x6000);
        m.zx.write16(0x6000, 0x09F4);
        m.zx.mem[0x5C88] = 33;
        m.zx.mem[0x5C89] = 24;
        m.zx.set_a(0x16);
        let r = m.zx.r();
        assert!(answer(&mut m.zx));
        let z = &m.zx;
        assert_eq!((z.pc(), z.sp()), (0x8000, 0x7000));
        assert_eq!(z.t, 541, "a control code's time");
        assert_eq!(z.r(), r + 1);
        assert_eq!(z.read16(0x6000), 0x0A6D, "now waiting for AT's operands");
    }

    #[test]
    fn anywhere_else_is_left_to_the_interpreter() {
        let mut m = calling(0x9000);
        assert!(!answer(&mut m.zx));
        assert_eq!(m.zx.pc(), 0x9000);
    }
}
