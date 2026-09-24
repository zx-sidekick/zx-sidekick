//! The machine around the processor: memory, ports, time and the helpers
//! the ROM answers use.

use rustzx_z80::Z80Bus;
use zx_core::snapshot::Snapshot;
use zx_core::timing::contention;

use super::*;

/// A snapshot with every register zero and RAM empty, apart from `pc`, `sp`
/// and interrupt mode 1.
fn snapshot(pc: u16, sp: u16) -> Snapshot {
    Snapshot {
        a: 0,
        f: 0,
        b: 0,
        c: 0,
        d: 0,
        e: 0,
        h: 0,
        l: 0,
        a_: 0,
        f_: 0,
        b_: 0,
        c_: 0,
        d_: 0,
        e_: 0,
        h_: 0,
        l_: 0,
        ix: 0,
        iy: 0,
        sp,
        pc,
        i: 0,
        r: 0,
        iff1: false,
        iff2: false,
        im: 1,
        border: 0,
        ram: vec![0; 0xC000],
    }
}

/// A machine running `program` at 0x8000, with the stack at 0xC000.
fn machine(program: &[u8]) -> Zx {
    let mut z = Zx::new(&snapshot(0x8000, 0xC000), None);
    z.mem[0x8000..0x8000 + program.len()].copy_from_slice(program);
    z
}

#[test]
fn a_snapshot_sets_every_register_and_the_border() {
    let mut snap = snapshot(0x1234, 0x5678);
    (
        snap.a, snap.f, snap.b, snap.c, snap.d, snap.e, snap.h, snap.l,
    ) = (1, 2, 3, 4, 5, 6, 7, 8);
    (snap.a_, snap.f_, snap.b_, snap.c_) = (9, 10, 11, 12);
    (snap.d_, snap.e_, snap.h_, snap.l_) = (13, 14, 15, 16);
    (snap.ix, snap.iy, snap.i, snap.r) = (0xAAAA, 0xBBBB, 0x3F, 0x42);
    (snap.iff1, snap.iff2, snap.border) = (true, true, 5);
    snap.ram[0] = 0x99;
    let mut z = Zx::new(&snap, None);
    assert_eq!(
        (z.a(), z.f(), z.bc(), z.de(), z.hl()),
        (1, 2, 0x0304, 0x0506, 0x0708)
    );
    assert_eq!(
        (z.ix(), z.iy(), z.sp(), z.pc()),
        (0xAAAA, 0xBBBB, 0x5678, 0x1234)
    );
    assert_eq!((z.r(), z.iff1(), z.border), (0x42, true, 5));
    assert_eq!(z.mem[0x4000], 0x99);
    assert!(!z.rom_loaded);
    let regs = &mut z.cpu().regs;
    regs.swap_af_alt();
    regs.exx();
    assert_eq!(
        (regs.get_af(), regs.get_bc(), regs.get_de(), regs.get_hl()),
        (0x090A, 0x0B0C, 0x0D0E, 0x0F10)
    );
    assert_eq!(regs.get_i(), 0x3F);
}

#[test]
fn a_rom_fills_the_bottom_16k_and_ignores_writes() {
    let rom = vec![0xC3; 0x4000];
    let mut z = Zx::new(&snapshot(0, 0), Some(&rom));
    assert!(z.rom_loaded);
    assert_eq!(z.mem[0x3FFF], 0xC3);
    z.write16(0x3FFF, 0x1234);
    assert_eq!(z.mem[0x3FFF], 0xC3, "the ROM is not written");
    assert_eq!(z.mem[0x4000], 0x12, "RAM is");
}

#[test]
fn the_bottom_16k_is_read_only_unless_made_writable() {
    // LD (0x1000),A with A = 0x55, then the same into RAM.
    let mut z = machine(&[0x3E, 0x55, 0x32, 0x00, 0x10, 0x32, 0x00, 0x90]);
    z.interrupts = false;
    for _ in 0..3 {
        z.step();
    }
    assert_eq!(z.mem[0x1000], 0);
    assert_eq!(z.mem[0x9000], 0x55);
    z.low_writable = true;
    z.write16(0x1000, 0xBEEF);
    assert_eq!(z.read16(0x1000), 0xBEEF);
}

#[test]
fn the_keyboard_port_reads_the_selected_half_rows() {
    let mut z = machine(&[]);
    z.set_key(Key::by_name("a").unwrap(), true);
    z.set_key(Key::by_name("space").unwrap(), true);
    // 0xFDFE selects the A..G half-row: A is down.
    assert_eq!(z.bus.read_io(0xFDFE), 0xA0 | 0x1E);
    // 0x7FFE selects SPACE..B: space is down.
    assert_eq!(z.bus.read_io(0x7FFE), 0xA0 | 0x1E);
    // 0xFEFE selects CAPS..V, where nothing is pressed.
    assert_eq!(z.bus.read_io(0xFEFE), 0xBF & !0x40);
    // Selecting both rows at once gives the keys of either.
    assert_eq!(z.bus.read_io(0x7DFE), 0xA0 | 0x1E);
    // Bit 6 follows the EAR output.
    z.ear = true;
    assert_eq!(z.bus.read_io(0xFEFE), 0xFF);
    z.set_key(Key::by_name("a").unwrap(), false);
    z.ear = false;
    assert_eq!(z.bus.read_io(0xFDFE), 0xBF);
}

#[test]
fn the_kempston_port_and_an_unattached_port() {
    let mut z = machine(&[]);
    z.set_key(Key::by_name("joy_fire").unwrap(), true);
    z.set_key(Key::by_name("joy_up").unwrap(), true);
    assert_eq!(z.bus.read_io(0x001F), 0x18);
    assert_eq!(z.bus.read_io(0x00FF), 0xFF);
    z.release_all_keys();
    assert_eq!(z.bus.read_io(0x001F), 0);
    assert_eq!(z.keys, [0xFF; 8]);
}

#[test]
fn keys_are_pressed_and_let_go_one_at_a_time() {
    let mut z = machine(&[]);
    let (fire, up) = (
        Key::by_name("joy_fire").unwrap(),
        Key::by_name("joy_up").unwrap(),
    );
    z.set_key(fire, true);
    z.set_key(up, true);
    z.set_key(fire, false);
    assert_eq!(z.kempston, 0x08);
    let a = Key::by_name("a").unwrap();
    z.set_key(a, true);
    z.set_key(a, false);
    assert_eq!(z.keys, [0xFF; 8]);
    // A place that is not on the matrix presses nothing.
    z.set_key(Key::Matrix(8, 0), true);
    z.set_key(Key::Matrix(0, 5), true);
    assert_eq!(z.keys, [0xFF; 8]);
}

#[test]
fn ix_can_be_set() {
    let mut z = machine(&[]);
    z.set_ix(0x1234);
    assert_eq!(z.ix(), 0x1234);
}

#[test]
fn a_port_reader_replaces_the_hardware() {
    let mut z = machine(&[]);
    z.port_in = Some(|port| (port >> 8) as u8);
    assert_eq!(z.bus.read_io(0x42FE), 0x42);
}

#[test]
fn the_ula_port_sets_the_border_and_records_speaker_changes() {
    let mut z = machine(&[]);
    z.bus.t = 100;
    z.bus.write_io(0x00FE, 0x12);
    assert_eq!(z.border, 2);
    assert_eq!(z.speaker, [(104, true)]);
    // The same level again is not a change.
    z.bus.write_io(0x00FE, 0x13);
    assert_eq!(z.border, 3);
    assert_eq!(z.speaker.len(), 1);
    z.bus.write_io(0x00FE, 0x00);
    assert!(!z.speaker[1].1);
    // A port with bit 0 set is not the ULA's.
    z.bus.write_io(0x00FF, 0x17);
    assert_eq!(z.border, 0);
    assert_eq!(z.speaker.len(), 2);
}

#[test]
fn the_interrupt_line_is_up_for_the_first_32_t_states() {
    let mut z = machine(&[]);
    for (t, up) in [(0, true), (31, true), (32, false), (FRAME_T - 1, false)] {
        z.bus.t = t;
        assert_eq!(z.bus.int_active(), up, "t = {t}");
    }
    z.bus.t = 0;
    z.interrupts = false;
    assert!(!z.bus.int_active());
    assert!(!z.bus.nmi_active());
    assert_eq!(z.bus.read_interrupt(), 0xFF);
}

#[test]
fn memory_in_the_ulas_quarter_waits_while_it_draws() {
    let mut z = machine(&[]);
    let t0 = 14335;
    z.bus.t = t0;
    z.bus.wait_mreq(0x4000, 3);
    assert_eq!(z.bus.t, t0 + contention(t0) + 3);
    z.bus.t = t0;
    z.bus.wait_mreq(0x8000, 3);
    assert_eq!(z.bus.t, t0 + 3, "RAM above 0x8000 never waits");
    z.bus.t = t0;
    z.bus.wait_no_mreq(0x4000, 1);
    assert_eq!(z.bus.t, t0 + contention(t0) + 1);
    z.bus.wait_internal(5);
    assert_eq!(z.bus.t, t0 + contention(t0) + 6);
}

#[test]
fn bus_events_are_recorded_when_asked_for() {
    let mut z = machine(&[]);
    z.events = Some(Vec::new());
    z.bus.t = 10;
    z.bus.wait_mreq(0x9000, 4);
    z.bus.wait_no_mreq(0x9001, 1);
    assert_eq!(z.events.as_deref(), Some(&[(10, 0x9000), (14, 0x9001)][..]));
}

#[test]
fn an_interrupt_is_a_step_of_its_own_with_none_of_its_handler_run() {
    // Interrupts on, a NOP at 0x8000 and, at the vector, an instruction
    // that would show if it ran.
    let mut z = machine(&[0x00, 0x00]);
    z.set_interrupts(true);
    z.low_writable = true;
    z.mem[0x0038..0x003C].copy_from_slice(&[0x32, 0x00, 0x90, 0x00]); // LD (9000),A; NOP
    z.set_a(0x5A);
    let (sp, r) = (z.sp(), z.r());
    assert_eq!(z.step(), Step::Interrupt);
    assert_eq!(z.pc(), 0x0038, "at the handler");
    assert_eq!(z.mem[0x9000], 0, "with none of it run");
    assert_eq!(z.read16(z.sp()), 0x8000, "the return address pushed");
    assert_eq!(z.sp(), sp.wrapping_sub(2));
    assert!(!z.iff1());
    assert_eq!(
        (z.t, z.r()),
        (13, (r + 1) & 0x7F),
        "13 T-states and one fetch"
    );
    assert_eq!(z.step(), Step::Instruction);
    assert_eq!(z.mem[0x9000], 0x5A, "and then it runs");
}

#[test]
fn spending_time_advances_r_but_keeps_its_top_bit() {
    let mut z = machine(&[]);
    z.set_r(0xFE);
    z.spend(100, 3);
    assert_eq!(z.bus.t, 100);
    assert_eq!(z.r(), 0x81);
}

#[test]
fn the_stack_and_16_bit_reads_are_little_endian_and_wrap() {
    let mut z = machine(&[]);
    z.push(0x1234);
    assert_eq!(z.sp(), 0xBFFE);
    assert_eq!((z.mem[0xBFFE], z.mem[0xBFFF]), (0x34, 0x12));
    assert_eq!(z.pop(), 0x1234);
    assert_eq!(z.sp(), 0xC000);
    z.mem[0xFFFF] = 0x78;
    z.low_writable = true;
    z.mem[0x0000] = 0x56;
    assert_eq!(z.read16(0xFFFF), 0x5678);
}

#[test]
fn a_frame_runs_until_its_time_is_used_and_carries_the_rest() {
    // JR $ forever, with interrupts off.
    let mut z = machine(&[0x18, 0xFE]);
    z.interrupts = false;
    let mut asked = 0;
    z.run_frame(|_| {
        asked += 1;
        false
    });
    assert_eq!(z.frame, 1);
    assert!(z.bus.t < 12, "carried {} T-states", z.bus.t);
    assert_eq!(asked, FRAME_T.div_ceil(12));
}

#[test]
fn a_hook_that_handles_the_instruction_is_not_stepped_over() {
    let mut z = machine(&[0x18, 0xFE]);
    z.interrupts = false;
    z.run_frame(|z| {
        z.bus.t += 1000;
        z.bus.t < 5000
    });
    assert_eq!(z.pc(), 0x8000);
}

#[test]
fn an_interrupt_wakes_a_halt_and_returns_past_it() {
    // EI; HALT; INC A; JR $ with MASK-INT at 0x0038 as EI; RET.
    let mut z = machine(&[0xFB, 0x76, 0x3C, 0x18, 0xFE]);
    z.low_writable = true;
    z.mem[0x38] = 0xFB;
    z.mem[0x39] = 0xC9;
    z.bus.t = 100;
    z.step();
    z.step();
    assert!(z.halted());
    assert!(z.run_until_any(&[0x8002], 2));
    assert!(!z.halted());
    assert_eq!(z.a(), 0);
    assert_eq!(z.frame, 1, "woken by the next frame's interrupt");
}

#[test]
fn running_to_an_address_gives_up_after_its_frames() {
    let mut z = machine(&[0x18, 0xFE]);
    z.interrupts = false;
    assert!(!z.run_until_any(&[0x9000], 2));
    assert_eq!(z.frame, 3);
    assert!(z.run_until_any(&[0x8000], 0), "the jump arrives back");
}

#[test]
fn a_clone_runs_on_its_own() {
    let mut z = machine(&[0x3C, 0x3C]);
    z.interrupts = false;
    let copy = z.clone();
    z.step();
    assert_eq!((z.a(), z.pc()), (1, 0x8001));
    assert_eq!((copy.a(), copy.pc()), (0, 0x8000));
}

/// Runs one instruction with the given A, F, HL, DE and C, and returns the
/// machine after it.
fn run(program: &[u8], a: u8, f: u8, hl: u16, de: u16, c: u8) -> Zx {
    let mut z = machine(program);
    z.interrupts = false;
    z.set_a(a);
    z.set_f(f);
    z.set_hl(hl);
    z.set_de(de);
    z.set_c(c);
    z.step();
    z
}

#[test]
fn add16_sets_the_flags_as_the_processor_does() {
    let values = [
        0x0000, 0x0001, 0x0FFF, 0x1000, 0x7FFF, 0x8000, 0xFFFF, 0x1234, 0xEDCB,
    ];
    for &f in &[0x00, 0xFF] {
        for &hl in &values {
            for &de in &values {
                // ADD HL,DE
                let cpu = run(&[0x19], 0, f, hl, de, 0);
                let mut z = run(&[], 0, f, 0, 0, 0);
                let sum = z.add16(hl, de);
                assert_eq!(
                    (sum, z.f()),
                    (cpu.hl(), cpu.f()),
                    "{hl:04x}+{de:04x} f={f:02x}"
                );
            }
        }
    }
}

#[test]
fn rl_and_rla_set_the_flags_as_the_processor_does() {
    for v in 0..=255u8 {
        for f in [0x00, 0x01, 0xFE, 0xFF] {
            // RL C
            let cpu = run(&[0xCB, 0x11], 0, f, 0, 0, v);
            let mut z = run(&[], 0, f, 0, 0, 0);
            assert_eq!((z.rl(v), z.f()), (cpu.c(), cpu.f()), "RL {v:02x} f={f:02x}");
            // RLA
            let cpu = run(&[0x17], v, f, 0, 0, 0);
            let mut z = run(&[], v, f, 0, 0, 0);
            z.rla();
            assert_eq!((z.a(), z.f()), (cpu.a(), cpu.f()), "RLA {v:02x} f={f:02x}");
        }
    }
}

#[test]
fn a_kempston_bit_past_the_port_does_nothing() {
    let mut z = machine(&[]);
    z.set_key(Key::Kempston(8), true);
    z.set_key(Key::Kempston(5), true);
    assert_eq!(z.kempston, 0);
    z.set_key(Key::Kempston(4), true);
    assert_eq!(z.kempston, 0x10);
}
