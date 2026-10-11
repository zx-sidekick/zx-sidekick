//! A copy of a machine runs as the original would, and apart from it.

use sidekick_core::{Machine, Model, Stop};

/// The ROM's maskable interrupt handler, entered once a frame.
const INTERRUPT: u16 = 0x0038;

fn booted() -> Machine {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..100 {
        machine.run_frame().expect("a frame");
    }
    machine
}

fn ram(machine: &Machine) -> Vec<u8> {
    (0..3)
        .filter_map(|bank| machine.ram_bank(bank))
        .flatten()
        .copied()
        .collect()
}

#[test]
fn a_copy_runs_as_the_original() {
    let mut original = booted();
    original.set_sync(Some(INTERRUPT));
    original.watch(&[INTERRUPT]);
    let mut copy = original.clone();
    let stops: Vec<Stop> = (0..20).map(|_| original.run().expect("a run")).collect();
    let copy_stops: Vec<Stop> = (0..20).map(|_| copy.run().expect("a run")).collect();
    assert_eq!(
        stops, copy_stops,
        "the same stops, the sync address included"
    );
    assert_eq!(original.passed(), copy.passed());
    assert!(ram(&original) == ram(&copy), "the same memory");
    assert_eq!(original.registers().get_pc(), copy.registers().get_pc());
    assert_eq!(original.picture(), copy.picture());
}

#[test]
fn a_copy_runs_apart_from_the_original() {
    let mut original = booted();
    let before = ram(&original);
    let mut copy = original.clone();
    copy.write(0x9000, !copy.peek(0x9000));
    copy.run_frame().expect("a frame");
    assert!(ram(&original) == before, "the original is untouched");
    original.run_frame().expect("the original still runs");
}
