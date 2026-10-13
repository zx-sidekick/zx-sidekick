//! The loading screen held: the frame stops the moment the tape's last block has loaded,
//! before the loader or the game runs on, with the screen drawn from memory.

use sidekick_core::{Key, Machine, Model, Session};
use sidekick_def::Definition;

/// Where our test program's code goes, and runs from.
const CODE: u16 = 0x8000;

/// Frames enough to load the whole tape, fast.
const LOADING_FRAMES: usize = 400;

/// A `.tap` block: its length, then the flag, the data and their XOR.
fn block(flag: u8, data: &[u8]) -> Vec<u8> {
    let checksum = data.iter().fold(flag, |x, b| x ^ b);
    let len = u16::try_from(data.len() + 2).expect("a short block");
    let mut out = len.to_le_bytes().to_vec();
    out.push(flag);
    out.extend_from_slice(data);
    out.push(checksum);
    out
}

/// A header block: its type, a ten-letter name, the length, and its two parameters.
fn header(kind: u8, name: &[u8; 10], len: u16, one: u16, two: u16) -> Vec<u8> {
    let mut data = vec![kind];
    data.extend_from_slice(name);
    for word in [len, one, two] {
        data.extend_from_slice(&word.to_le_bytes());
    }
    block(0x00, &data)
}

/// A tape of our own: `10 LOAD ""SCREEN$: LOAD ""CODE: CLS: RANDOMIZE USR 32768`,
/// auto-running; a screen all red paper; and two bytes of code at 32768 that loop on the spot
/// (`JR $`). The `CLS` after the last block would wipe the screen if the hold came later.
/// With `pause`, `PAUSE 0` after the screen waits for a key, as many loaders do, then
/// `PAUSE 100` holds it two seconds more.
fn tape(pause: bool) -> Vec<u8> {
    let mut line = vec![0xEF, b'"', b'"', 0xAA, b':'];
    if pause {
        line.extend_from_slice(&[0xF2, b'0', 0x0E, 0, 0, 0, 0, 0, b':']);
        line.extend_from_slice(&[0xF2, b'1', b'0', b'0', 0x0E, 0, 0, 100, 0, 0, b':']);
    }
    line.extend_from_slice(&[0xEF, b'"', b'"', 0xAF, b':', 0xFB, b':', 0xF9, 0xC0]);
    line.extend_from_slice(b"32768");
    line.extend_from_slice(&[0x0E, 0x00, 0x00, 0x00, 0x80, 0x00, 0x0D]);
    let mut program = vec![0x00, 0x0A];
    program.extend_from_slice(&u16::try_from(line.len()).expect("short").to_le_bytes());
    program.extend_from_slice(&line);
    let program_len = u16::try_from(program.len()).expect("short");

    let mut screen = vec![0u8; 6144];
    screen.extend(std::iter::repeat_n(0x10, 768)); // red paper, black ink

    let mut tap = header(0, b"loader    ", program_len, 10, program_len);
    tap.extend(block(0xFF, &program));
    tap.extend(header(3, b"screen    ", 6912, 0x4000, 0x8000));
    tap.extend(block(0xFF, &screen));
    tap.extend(header(3, b"code      ", 2, CODE, 0x8000));
    tap.extend(block(0xFF, &[0x18, 0xFE]));
    tap
}

fn session(hold: bool) -> Session {
    session_with(hold, &tape(false))
}

fn session_with(hold: bool, tape: &[u8]) -> Session {
    let text = format!(
        r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "our own test tape"
[run]
hold_loading_screen = {hold}
"#
    );
    let definition = Definition::from_toml(&text).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    machine.load_tap(tape).expect("our tape");
    Session::new(machine, Some(definition))
}

#[test]
fn the_game_waits_on_its_loading_screen_the_moment_the_tape_has_loaded() {
    let mut s = session(true);
    let mut frames = 0;
    while !s.holding() {
        assert!(frames < LOADING_FRAMES, "the tape should have loaded");
        s.run_frame().expect("a frame");
        frames += 1;
    }
    let held_at = s.machine().registers().get_pc();
    assert!(held_at < 0x4000, "still in the ROM's loader: {held_at:04X}");
    assert_eq!(s.machine().peek(CODE), 0x18, "the code is in");
    let picture = s.machine().picture();
    assert_eq!(picture.color(32, 32), 2, "the screen's top left: red paper");
    assert_eq!(picture.color(32 + 255, 32 + 191), 2, "and its bottom right");

    s.run_frame().expect("nothing runs while held");
    assert_eq!(s.machine().registers().get_pc(), held_at);

    s.release();
    for _ in 0..50 {
        s.run_frame().expect("a frame");
        assert!(!s.holding(), "held once only");
    }
    let pc = s.machine().registers().get_pc();
    assert!(
        pc == CODE || pc < 0x4000,
        "looping in the game, or in the interrupt: {pc:04X}"
    );
}

#[test]
fn a_game_that_doesn_t_ask_isn_t_held() {
    let mut s = session(false);
    for _ in 0..LOADING_FRAMES {
        s.run_frame().expect("a frame");
        assert!(!s.holding());
    }
}

#[test]
fn held_the_loading_screen_waits_for_a_key() {
    let mut s = session(true);
    for _ in 0..LOADING_FRAMES {
        s.run_frame().expect("a frame");
    }
    assert!(s.holding());
    assert!(s.on_loading_screen());
}

#[test]
fn a_loader_s_own_pause_is_a_loading_screen_waiting_for_a_key() {
    let mut s = session_with(false, &tape(true));
    let mut frames = 0;
    while !s.on_loading_screen() {
        assert!(
            frames < LOADING_FRAMES,
            "the loader should pause on its screen"
        );
        s.run_frame().expect("a frame");
        frames += 1;
    }
    assert_ne!(s.machine().peek(CODE), 0x18, "the code is still to load");
    // Not SPACE, which breaks the next LOAD while held, as on a Spectrum; nor ENTER, which the
    // ROM still counts as held from the LOAD "" that started the tape.
    s.machine_mut().key(Key::A, true);
    for _ in 0..5 {
        s.run_frame().expect("a frame");
    }
    s.machine_mut().key(Key::A, false);
    for _ in 0..LOADING_FRAMES {
        s.run_frame().expect("a frame");
        assert!(
            !s.on_loading_screen(),
            "a key goes on loading; the timed pause after it isn't a wait"
        );
    }
    assert_eq!(s.machine().peek(CODE), 0x18, "the code is in");
}

#[test]
fn a_loader_that_doesn_t_pause_has_no_loading_screen_wait() {
    let mut s = session(false);
    for _ in 0..LOADING_FRAMES {
        s.run_frame().expect("a frame");
        assert!(!s.on_loading_screen());
    }
}
