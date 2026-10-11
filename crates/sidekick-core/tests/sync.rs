//! A frame runs to a sync address and on to its end, and reports the watched addresses it ran.

use sidekick_core::{Machine, Model, Stop};

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

/// The ROM's maskable interrupt handler, entered once a frame.
const INTERRUPT: u16 = 0x0038;

/// FRAMES, the system variable the ROM's interrupt handler counts frames in.
const FRAMES: u16 = 23672;

/// An address in the ROM's character set: data, never executed.
const FONT: u16 = 0x3D00;

fn booted() -> Machine {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    machine
}

#[test]
fn without_a_sync_address_every_run_ends_a_frame() {
    let mut machine = booted();
    for _ in 0..10 {
        assert_eq!(machine.run().expect("a run"), Stop::FrameEnd);
    }
}

#[test]
fn a_sync_address_stops_once_a_frame_before_its_end() {
    let mut machine = booted();
    machine.set_sync(Some(INTERRUPT));
    let mut stops = Vec::new();
    for _ in 0..20 {
        stops.push(machine.run().expect("a run"));
    }
    let frames = stops.iter().filter(|&&s| s == Stop::FrameEnd).count();
    let syncs = stops.iter().filter(|&&s| s == Stop::Sync).count();
    assert_eq!(frames, 10, "{stops:?}");
    assert_eq!(syncs, 10, "{stops:?}");
    assert!(
        stops.windows(2).all(|w| w[0] != w[1]),
        "sync and frame end alternate: {stops:?}"
    );
}

#[test]
fn a_frame_end_is_never_lost_over_a_sync_stop() {
    let mut machine = booted();
    machine.set_sync(Some(INTERRUPT));
    let before = machine.peek(FRAMES);
    let mut frame_ends = 0u8;
    while frame_ends < 50 {
        if machine.run().expect("a run") == Stop::FrameEnd {
            frame_ends += 1;
        }
    }
    let counted = machine.peek(FRAMES).wrapping_sub(before);
    assert_eq!(counted, frame_ends, "one interrupt per frame end");
}

#[test]
fn passed_lists_watched_addresses_that_ran_this_frame() {
    let mut machine = booted();
    machine.watch(&[INTERRUPT, FONT]);
    assert!(
        machine.passed().is_empty(),
        "nothing has run since watching began"
    );
    assert_eq!(machine.run().expect("a run"), Stop::FrameEnd);
    assert_eq!(machine.passed(), [INTERRUPT]);
    assert_eq!(machine.run().expect("a run"), Stop::FrameEnd);
    assert_eq!(
        machine.passed(),
        [INTERRUPT],
        "cleared and filled again each frame"
    );
}

#[test]
fn passed_is_readable_at_the_sync_stop() {
    let mut machine = booted();
    machine.watch(&[INTERRUPT]);
    machine.set_sync(Some(INTERRUPT));
    let synced = (0..4).any(|_| machine.run().expect("a run") == Stop::Sync);
    assert!(synced, "the sync address stops within two frames");
    assert_eq!(machine.passed(), [INTERRUPT]);
}
