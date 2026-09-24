//! A ZX Spectrum 48K: the Z80 from `rustzx-z80` (MIT, the RustZX project),
//! and the machine around it written here: 64K of memory, the keyboard and a
//! Kempston joystick on their ports, the border and speaker, the 50 Hz
//! interrupt, and the delays the ULA adds while it draws the picture
//! (`zx_core::timing::contention`).

mod keys;
#[cfg(test)]
mod tests;

use std::ops::{Deref, DerefMut};

use rustzx_z80::{RegName8, Z80, Z80Bus};
use zx_core::bus::{charge_io, contended};
use zx_core::snapshot::Snapshot;
use zx_core::timing::contention;

pub use keys::Key;
pub use rustzx_z80;
pub use rustzx_z80::Step;
pub use zx_core::timing::FRAME_T;

/// Flag bits of F.
pub const CF: u8 = 0x01;
pub const NF: u8 = 0x02;
pub const PF: u8 = 0x04;
pub const XF: u8 = 0x08;
pub const HF: u8 = 0x10;
pub const YF: u8 = 0x20;
pub const ZF: u8 = 0x40;
pub const SF: u8 = 0x80;

/// T-states at the start of a frame during which the ULA holds the
/// interrupt line.
pub const INT_LEN: u32 = 32;

/// Everything on the processor's bus.
#[derive(Clone)]
pub struct Bus {
    pub mem: Box<[u8; 0x10000]>,
    /// Whether a ROM image was put in the bottom 16K. Either way that quarter
    /// of memory ignores writes, as a Spectrum's ROM does.
    pub rom_loaded: bool,
    /// Lets writes into the bottom 16K through: for testing the processor
    /// on its own, which treats all 64K as memory.
    pub low_writable: bool,
    /// T-states into the current frame.
    pub t: u32,
    /// Frames run.
    pub frame: u64,
    /// Keyboard half-rows (address lines A8–A15); a 0 bit is a key down.
    pub keys: [u8; 8],
    /// Kempston joystick: bit 0 right, 1 left, 2 down, 3 up, 4 fire.
    pub kempston: u8,
    pub border: u8,
    pub ear: bool,
    /// Each change of the speaker level, with the T-state it happened at,
    /// since whoever plays the sound last took them.
    pub speaker: Vec<(u32, bool)>,
    /// What a port read returns instead of the ULA and joystick, if set: for
    /// testing the processor on its own.
    pub port_in: Option<fn(u16) -> u8>,
    /// If set, each address the processor puts on the bus for a memory or
    /// internal cycle, with the T-state it did so at: for testing.
    pub events: Option<Vec<(u32, u16)>>,
    /// Whether the ULA raises the interrupt line at the start of each frame:
    /// always on a Spectrum, off for testing the processor on its own.
    pub interrupts: bool,
}

impl Z80Bus for Bus {
    fn read_internal(&mut self, addr: u16) -> u8 {
        self.mem[usize::from(addr)]
    }

    fn write_internal(&mut self, addr: u16, data: u8) {
        if addr >= 0x4000 || self.low_writable {
            self.mem[usize::from(addr)] = data;
        }
    }

    fn wait_mreq(&mut self, addr: u16, clk: usize) {
        if let Some(events) = &mut self.events {
            events.push((self.t, addr));
        }
        if contended(addr) {
            self.t += contention(self.t);
        }
        self.t += clk as u32;
    }

    fn wait_no_mreq(&mut self, addr: u16, clk: usize) {
        if let Some(events) = &mut self.events {
            events.push((self.t, addr));
        }
        if contended(addr) {
            self.t += contention(self.t);
        }
        self.t += clk as u32;
    }

    /// Clocks with no memory request, one at a time where each may be held
    /// up by the screen, all at once where none can be: above 0x7FFF or
    /// below 0x4000 nothing is contended (#187). Manic Miner's block copies
    /// spent much of a frame here.
    fn wait_loop(&mut self, addr: u16, clk: usize) {
        if self.events.is_none() && !contended(addr) {
            self.t += clk as u32;
            return;
        }
        for _ in 0..clk {
            self.wait_no_mreq(addr, 1);
        }
    }

    fn wait_internal(&mut self, clk: usize) {
        self.t += clk as u32;
    }

    fn read_io(&mut self, port: u16) -> u8 {
        charge_io(&mut self.t, port);
        if let Some(read) = self.port_in {
            return read(port);
        }
        if port & 1 == 0 {
            let high = (port >> 8) as u8;
            let mut keys = 0x1F;
            for (row, bits) in self.keys.iter().enumerate() {
                if high & (1 << row) == 0 {
                    keys &= bits;
                }
            }
            // Issue 3 behaviour: bit 6 follows the EAR output.
            let ear = if self.ear { 0x40 } else { 0 };
            0xA0 | ear | keys
        } else if port & 0x20 == 0 {
            self.kempston
        } else {
            0xFF
        }
    }

    fn write_io(&mut self, port: u16, data: u8) {
        charge_io(&mut self.t, port);
        if port & 1 == 0 {
            self.border = data & 7;
            let ear = data & 0x10 != 0;
            if ear != self.ear {
                self.speaker.push((self.t, ear));
            }
            self.ear = ear;
        }
    }

    fn read_interrupt(&mut self) -> u8 {
        // The Spectrum's data bus floats high during the acknowledge.
        0xFF
    }

    fn reti(&mut self) {}

    fn halt(&mut self, _halted: bool) {}

    fn int_active(&self) -> bool {
        self.interrupts && self.t < INT_LEN
    }

    fn nmi_active(&self) -> bool {
        false
    }

    fn pc_callback(&mut self, _addr: u16) {}
}

/// The machine: the processor and its bus. It dereferences to the bus, so
/// `z.mem`, `z.keys` and the rest read as they did. A clone is an
/// independent machine: the processor derives `Clone` in our fork of
/// `rustzx-z80`.
#[derive(Clone)]
pub struct Zx {
    cpu: Z80,
    pub bus: Bus,
}

impl Deref for Zx {
    type Target = Bus;
    fn deref(&self) -> &Bus {
        &self.bus
    }
}

impl DerefMut for Zx {
    fn deref_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }
}

macro_rules! reg8 {
    ($($get:ident $set:ident $name:ident),*) => {$(
        #[must_use]
        pub fn $get(&self) -> u8 {
            self.cpu.regs.get_reg_8(RegName8::$name)
        }
        pub fn $set(&mut self, v: u8) {
            self.cpu.regs.set_reg_8(RegName8::$name, v);
        }
    )*};
}

impl Zx {
    /// A machine in the state a snapshot describes, with `rom` in the bottom
    /// 16K if one is given.
    #[must_use]
    pub fn new(snap: &Snapshot, rom: Option<&[u8]>) -> Zx {
        let mut mem = Box::new([0u8; 0x10000]);
        mem[0x4000..].copy_from_slice(&snap.ram[..0xC000]);
        if let Some(rom) = rom {
            let n = rom.len().min(0x4000);
            mem[..n].copy_from_slice(&rom[..n]);
        }
        let mut z = Zx {
            cpu: Z80::default(),
            bus: Bus {
                mem,
                rom_loaded: rom.is_some(),
                low_writable: false,
                t: 0,
                frame: 0,
                keys: [0xFF; 8],
                kempston: 0,
                border: snap.border,
                ear: false,
                speaker: Vec::new(),
                port_in: None,
                events: None,
                interrupts: true,
            },
        };
        let r = &mut z.cpu.regs;
        r.set_af(u16::from_be_bytes([snap.a_, snap.f_]));
        r.set_bc(u16::from_be_bytes([snap.b_, snap.c_]));
        r.set_de(u16::from_be_bytes([snap.d_, snap.e_]));
        r.set_hl(u16::from_be_bytes([snap.h_, snap.l_]));
        r.swap_af_alt();
        r.exx();
        r.set_af(u16::from_be_bytes([snap.a, snap.f]));
        r.set_bc(u16::from_be_bytes([snap.b, snap.c]));
        r.set_de(u16::from_be_bytes([snap.d, snap.e]));
        r.set_hl(u16::from_be_bytes([snap.h, snap.l]));
        r.set_ix(snap.ix);
        r.set_iy(snap.iy);
        r.set_sp(snap.sp);
        r.set_pc(snap.pc);
        r.set_i(snap.i);
        r.set_r(snap.r);
        r.set_iff1(snap.iff1);
        r.set_iff2(snap.iff2);
        z.cpu.set_im(snap.im);
        z
    }

    /// The processor itself, for what the accessors here do not cover (the
    /// alternate registers, the interrupt mode): tests of the processor.
    pub fn cpu(&mut self) -> &mut Z80 {
        &mut self.cpu
    }

    reg8!(a set_a A, f set_f F, b set_b B, c set_c C, d set_d D, e set_e E, h set_h H, l set_l L);

    #[must_use]
    pub fn bc(&self) -> u16 {
        self.cpu.regs.get_bc()
    }
    #[must_use]
    pub fn de(&self) -> u16 {
        self.cpu.regs.get_de()
    }
    #[must_use]
    pub fn hl(&self) -> u16 {
        self.cpu.regs.get_hl()
    }
    pub fn set_bc(&mut self, v: u16) {
        self.cpu.regs.set_bc(v);
    }
    pub fn set_de(&mut self, v: u16) {
        self.cpu.regs.set_de(v);
    }
    pub fn set_hl(&mut self, v: u16) {
        self.cpu.regs.set_hl(v);
    }
    #[must_use]
    pub fn ix(&self) -> u16 {
        self.cpu.regs.get_ix()
    }
    pub fn set_ix(&mut self, v: u16) {
        self.cpu.regs.set_ix(v);
    }
    #[must_use]
    pub fn iy(&self) -> u16 {
        self.cpu.regs.get_iy()
    }
    #[must_use]
    pub fn pc(&self) -> u16 {
        self.cpu.regs.get_pc()
    }
    pub fn set_pc(&mut self, v: u16) {
        self.cpu.regs.set_pc(v);
    }
    #[must_use]
    pub fn sp(&self) -> u16 {
        self.cpu.regs.get_sp()
    }
    pub fn set_sp(&mut self, v: u16) {
        self.cpu.regs.set_sp(v);
    }
    /// Whether the processor is sitting on a `HALT`, waiting for an interrupt.
    #[must_use]
    pub fn halted(&self) -> bool {
        self.cpu.is_halted()
    }
    #[must_use]
    pub fn iff1(&self) -> bool {
        self.cpu.regs.get_iff1()
    }
    /// Sets both interrupt flip-flops, as `EI` and `DI` do.
    pub fn set_interrupts(&mut self, on: bool) {
        self.cpu.regs.set_iff1(on);
        self.cpu.regs.set_iff2(on);
    }
    #[must_use]
    pub fn r(&self) -> u8 {
        self.cpu.regs.get_r()
    }
    pub fn set_r(&mut self, v: u8) {
        self.cpu.regs.set_r(v);
    }

    /// Charges `t` T-states and `m1` opcode fetches (which advance R).
    pub fn spend(&mut self, t: u32, m1: u8) {
        self.bus.t += t;
        let r = self.r();
        self.set_r((r & 0x80) | (r.wrapping_add(m1) & 0x7F));
    }

    #[must_use]
    pub fn read16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([
            self.mem[usize::from(addr)],
            self.mem[usize::from(addr.wrapping_add(1))],
        ])
    }

    pub fn write16(&mut self, addr: u16, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.bus.write_internal(addr, lo);
        self.bus.write_internal(addr.wrapping_add(1), hi);
    }

    /// Pushes `v`, as `PUSH` does, untimed: the processor's own
    /// (zx-sidekick/rustzx#9).
    pub fn push(&mut self, v: u16) {
        self.cpu.push(&mut self.bus, v);
    }

    /// Pops a word, as `POP` does, untimed: the processor's own.
    pub fn pop(&mut self) -> u16 {
        self.cpu.pop(&mut self.bus)
    }

    /// Returns from a routine, as `RET` does, untimed: the program counter
    /// popped, and MEMPTR and the rest as the instruction leaves them.
    pub fn ret(&mut self) {
        self.cpu.ret(&mut self.bus);
    }

    /// Takes the interrupt if it is due, as a step of its own, or else runs
    /// one instruction, and says which. After an interrupt the program
    /// counter is at its handler with none of it run, so a caller can act
    /// there before the next step.
    pub fn step(&mut self) -> Step {
        self.cpu.step(&mut self.bus)
    }

    /// Runs one 50 Hz frame. `hook` is asked before each instruction, and
    /// when it returns true it has dealt with the program counter itself.
    pub fn run_frame(&mut self, mut hook: impl FnMut(&mut Zx) -> bool) {
        while self.bus.t < FRAME_T {
            if hook(self) {
                continue;
            }
            self.step();
        }
        self.bus.t -= FRAME_T;
        self.bus.frame += 1;
    }

    /// Runs, frame by frame, until execution reaches one of `targets` (after
    /// at least one instruction). Returns `false` if that takes more than
    /// `max_frames` frames.
    pub fn run_until_any(&mut self, targets: &[u16], max_frames: u32) -> bool {
        let mut frames = 0;
        let mut first = true;
        loop {
            if self.bus.t >= FRAME_T {
                self.bus.t -= FRAME_T;
                self.bus.frame += 1;
                frames += 1;
                if frames > max_frames {
                    return false;
                }
            }
            if !first && targets.contains(&self.pc()) {
                return true;
            }
            first = false;
            self.step();
        }
    }

    /// `ADD HL,rr` on two values: the sum, with H, the undocumented bits 3
    /// and 5 and C set from it, and S, Z and P/V kept.
    pub fn add16(&mut self, a: u16, b: u16) -> u16 {
        // The processor's own arithmetic (zx-sidekick/rustzx#5), stored as
        // a routine's last instruction leaves F: not as the add itself,
        // which a following SCF or CCF would see (the Z80's Q).
        let (sum, f) = rustzx_z80::alu::add16_flags(self.f(), a, b);
        self.set_f(f);
        sum
    }

    /// `RL` on a value, as the CB-prefixed instruction sets the flags.
    pub fn rl(&mut self, v: u8) -> u8 {
        let r = (v << 1) | (self.f() & CF);
        let parity = if r.count_ones().is_multiple_of(2) {
            PF
        } else {
            0
        };
        let zero = if r == 0 { ZF } else { 0 };
        self.set_f((r & (SF | XF | YF)) | zero | parity | (v >> 7));
        r
    }

    /// `RLA`: A rotated left through carry, S, Z and P/V kept.
    pub fn rla(&mut self) {
        let a = self.a();
        let r = (a << 1) | (self.f() & CF);
        let f = self.f();
        self.set_a(r);
        self.set_f((f & (SF | ZF | PF)) | (r & (XF | YF)) | (a >> 7));
    }
}
