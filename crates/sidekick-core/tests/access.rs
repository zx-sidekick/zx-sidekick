//! Writing memory, the registers and reading RAM banks between frames.

use sidekick_core::{Machine, Model};

fn booted(model: Model) -> Machine {
    let mut machine = Machine::new(model).expect("a machine");
    for _ in 0..100 {
        machine.run_frame().expect("a frame");
    }
    machine
}

#[test]
fn a_write_reaches_ram_but_not_the_rom() {
    let mut machine = booted(Model::Sinclair48K);
    machine.write(0x9000, 0xA5);
    assert_eq!(machine.peek(0x9000), 0xA5);
    let rom = machine.peek(0x0000);
    machine.write(0x0000, !rom);
    assert_eq!(machine.peek(0x0000), rom);
}

#[test]
fn a_program_counter_set_between_frames_is_where_the_game_runs() {
    const LOOP: u16 = 0x9000;
    let mut machine = booted(Model::Sinclair48K);
    machine.write(LOOP, 0x18); // JR $
    machine.write(LOOP + 1, 0xFE);
    machine.registers_mut().set_pc(LOOP);
    for _ in 0..3 {
        machine.run_frame().expect("a frame");
    }
    assert_eq!(machine.registers().get_pc(), LOOP);
}

#[test]
fn a_48k_bank_is_its_16k_of_ram() {
    let mut machine = booted(Model::Sinclair48K);
    machine.write(0xC000, 0x3C);
    assert_eq!(machine.ram_bank(2).expect("bank 2")[0], 0x3C);
    assert!(machine.ram_bank(3).is_none());
}

#[test]
fn a_128k_bank_reads_whether_paged_in_or_not() {
    let mut machine = booted(Model::Sinclair128K);
    machine.write(0x4000, 0x5A);
    assert_eq!(machine.ram_bank(5).expect("bank 5")[0], 0x5A);
    assert!(machine.ram_bank(8).is_none());
}
