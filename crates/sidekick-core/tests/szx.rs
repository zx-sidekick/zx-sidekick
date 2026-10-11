//! A machine saved as SZX loads into a fresh one and runs on as the original.

use sidekick_core::{Error, Machine, Model, Stop};

/// The ROM's maskable interrupt handler, entered once a frame.
const INTERRUPT: u16 = 0x0038;

fn booted(model: Model) -> Machine {
    let mut machine = Machine::new(model).expect("a machine");
    for _ in 0..100 {
        machine.run_frame().expect("a frame");
    }
    machine
}

fn ram(machine: &Machine) -> Vec<u8> {
    (0..8)
        .map_while(|bank| machine.ram_bank(bank))
        .flatten()
        .copied()
        .collect()
}

fn round_trip(model: Model) {
    let mut original = booted(model);
    original.write(0x9000, 0x42);
    let szx = original.save_szx().expect("it saves");
    let mut loaded = Machine::new(model).expect("a machine");
    loaded.load_szx(&szx).expect("it loads");
    assert!(ram(&loaded) == ram(&original), "the same memory");

    for machine in [&mut original, &mut loaded] {
        machine.set_sync(Some(INTERRUPT));
    }
    let stops: Vec<Stop> = (0..20).map(|_| original.run().expect("a run")).collect();
    let loaded_stops: Vec<Stop> = (0..20).map(|_| loaded.run().expect("a run")).collect();
    assert_eq!(stops, loaded_stops);
    assert!(
        ram(&loaded) == ram(&original),
        "the same memory ten frames on"
    );
    assert_eq!(loaded.registers().get_pc(), original.registers().get_pc());
    assert_eq!(loaded.picture(), original.picture());
}

#[test]
fn a_48k_saved_loads_and_runs_on_as_the_original() {
    round_trip(Model::Sinclair48K);
}

#[test]
fn a_128k_saved_loads_and_runs_on_as_the_original() {
    round_trip(Model::Sinclair128K);
}

#[test]
fn a_snapshot_of_the_other_model_is_refused() {
    let szx = booted(Model::Sinclair128K).save_szx().expect("it saves");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a machine");
    assert!(matches!(machine.load_szx(&szx), Err(Error::OtherModel)));
}

#[test]
fn something_that_isnt_a_snapshot_is_refused() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a machine");
    assert!(machine.load_szx(b"not a snapshot").is_err());
    assert!(machine.load_szx(&[]).is_err());
}
