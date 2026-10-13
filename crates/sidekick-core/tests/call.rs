//! A routine called at the sync point runs, and the program goes on from there.

use sidekick_core::{Machine, Model, Session};
use sidekick_def::Definition;

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

/// FRAMES, the system variable the ROM's interrupt handler counts frames in.
const FRAMES: u16 = 23672;

/// The ROM's interrupt handler as the sync address: reached once a frame.
const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"
[run]
sync = 0x0038
"#;

/// PUSH AF; LD A,0x42; LD (0x9000),A; POP AF; RET
const ROUTINE: [u8; 8] = [0xF5, 0x3E, 0x42, 0x32, 0x00, 0x90, 0xF1, 0xC9];

#[test]
fn a_routine_called_at_the_sync_point_runs_and_returns() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let definition = Definition::from_toml(DEFINITION).expect("the test definition parses");
    let mut s = Session::new(machine, Some(definition));
    for (at, &byte) in (0x8000..).zip(&ROUTINE) {
        s.machine_mut().write(at, byte);
    }
    assert!(
        (0..3).any(|_| s.run_to_sync().expect("a run")),
        "the sync point is reached"
    );
    assert_eq!(s.machine().registers().get_pc(), 0x0038);
    s.machine_mut().call(0x8000, 0x0038);
    s.run_frame().expect("the frame finishes");
    assert_eq!(s.machine().peek(0x9000), 0x42, "the routine ran");
    let frames = s.machine().peek(FRAMES);
    for _ in 0..5 {
        s.run_frame().expect("a frame");
    }
    assert_eq!(
        s.machine().peek(FRAMES),
        frames.wrapping_add(5),
        "the interrupt handler goes on counting after it"
    );
}

#[test]
fn without_a_sync_address_running_to_it_ends_the_frame() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let definition = Definition::from_toml(&DEFINITION.replace("[run]\nsync = 0x0038\n", ""))
        .expect("the test definition parses");
    let mut s = Session::new(machine, Some(definition));
    let frames = s.machine().peek(FRAMES);
    assert!(!s.run_to_sync().expect("a run"), "no sync point to stop at");
    assert_eq!(
        s.machine().peek(FRAMES),
        frames.wrapping_add(1),
        "a whole frame ran"
    );
}

/// A switch that fills a value and calls the routine at 0x8000 once to show it.
const SWITCHED: &str = r#"
[values.mode]
addr = 0x9001
type = "u8"
[values.energy]
addr = 0x9002
type = "u8"
writable = true
max = 200
[[states]]
name = "play"
role = "play"
when = { value = "mode", equals = 2 }
[[cheats]]
id = "full-energy"
label = "Full energy"
set = { value = "energy", to = "max" }
call = 0x8000
"#;

/// LD A,(0x9000); INC A; LD (0x9000),A; RET: counts its calls at 0x9000.
const COUNTER: [u8; 8] = [0x3A, 0x00, 0x90, 0x3C, 0x32, 0x00, 0x90, 0xC9];

#[test]
fn a_switch_s_routine_is_called_once_in_play_after_it_goes_on() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let definition = Definition::from_toml(&format!("{DEFINITION}{SWITCHED}")).expect("it parses");
    let mut s = Session::new(machine, Some(definition));
    for (at, &byte) in (0x8000..).zip(&COUNTER) {
        s.machine_mut().write(at, byte);
    }
    s.switch("full-energy", true).expect("on");
    for _ in 0..3 {
        s.run_frame().expect("a frame");
    }
    assert_eq!(s.machine().peek(0x9000), 0, "not called outside play");
    s.machine_mut().write(0x9001, 2);
    for _ in 0..5 {
        s.run_frame().expect("a frame");
    }
    assert_eq!(s.machine().peek(0x9000), 1, "called once, in play");
    assert_eq!(s.machine().peek(0x9002), 200, "after the set");
    let frames = s.machine().peek(FRAMES);
    s.run_frame().expect("a frame");
    assert_eq!(
        s.machine().peek(FRAMES),
        frames.wrapping_add(1),
        "the interrupt handler goes on after it"
    );
    s.switch("full-energy", false).expect("off");
    s.switch("full-energy", true).expect("on again");
    s.run_frame().expect("a frame");
    assert_eq!(
        s.machine().peek(0x9000),
        2,
        "and once more each time it goes on"
    );
}
