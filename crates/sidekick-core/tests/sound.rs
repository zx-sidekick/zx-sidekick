//! Sound follows emulated time, through an effect that holds the program for many frames.

use sidekick_core::{Machine, Model, SAMPLE_RATE};

/// A beeper loop that never returns: DI; loop: XOR 0x10; OUT (254),A; LD B,100; DJNZ $; JR loop.
/// Each half period is 7 + 11 + 7 + (13 × 99 + 8) + 12 = 1,332 T-states.
const TONE: [u8; 11] = [
    0xF3, 0xEE, 0x10, 0xD3, 0xFE, 0x06, 100, 0x10, 0xFE, 0x18, 0xF6,
];
const HALF_PERIOD: usize = 1_332;
const FRAME: usize = 69_888;

#[test]
fn a_blocking_beeper_effect_sounds_in_every_frame_at_its_pitch() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..100 {
        machine.run_frame().expect("a frame");
    }
    for (addr, &byte) in (0x9000..).zip(&TONE) {
        machine.write(addr, byte);
    }
    machine.registers_mut().set_pc(0x9000);
    machine.run_frame().expect("a frame");

    let mut samples = Vec::new();
    for _ in 0..10 {
        machine.run_frame().expect("a frame");
        assert_eq!(
            machine.sound().len(),
            SAMPLE_RATE / 50,
            "a frame's worth of samples"
        );
        samples.extend(machine.sound().iter().map(|&(left, _)| left));
    }
    let changes = samples
        .windows(2)
        .filter(|w| (w[0] - w[1]).abs() > 1e-6)
        .count();
    let expected = 10 * FRAME / HALF_PERIOD;
    assert!(
        changes.abs_diff(expected) <= 3,
        "{changes} level changes in ten frames, expected about {expected}"
    );
}
