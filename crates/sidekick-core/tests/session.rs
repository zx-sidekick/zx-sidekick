//! A game's named values read each frame, its state decided, and writes to it by name.

use sidekick_core::{Machine, Model, Read, Session, WriteError};
use sidekick_def::Definition;

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

/// FRAMES, the system variable the ROM's interrupt handler counts frames in.
const FRAMES: u16 = 23672;

/// Free RAM, above the ROM editor's workspace, where these tests put their bytes.
const FREE: u16 = 0x8000;

fn session(values_and_states: &str) -> Session {
    let text = format!(
        r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"
{values_and_states}
"#
    );
    let definition = Definition::from_toml(&text).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    Session::new(machine, Some(definition))
}

fn poke(session: &mut Session, addr: u16, bytes: &[u8]) {
    for (at, &byte) in (addr..).zip(bytes) {
        session.machine_mut().write(at, byte);
    }
}

#[test]
fn each_type_reads_its_bytes() {
    let mut s = session(
        r#"
[values.u8]
addr = 0x8000
type = "u8"
[values.i8]
addr = 0x8001
type = "i8"
[values.u16]
addr = 0x8002
type = "u16"
[values.u16be]
addr = 0x8002
type = "u16be"
[values.bcd]
addr = 0x8004
type = "bcd"
len = 2
[values.digits]
addr = 0x8006
type = "digits"
len = 3
[values.screen_digits]
addr = 0x8011
type = "digits"
len = 2
zero = 0x31
[values.bitset]
addr = 0x8009
type = "bitset"
len = 2
[values.bytes]
addr = 0x8000
type = "bytes"
len = 3
[values.text]
addr = 0x800B
type = "text"
len = 3
[values.own_text]
addr = 0x800E
type = "text"
len = 3
charset = { first = 1, chars = "ABC" }
"#,
    );
    poke(&mut s, FREE, &[200, 0xFE, 0x34, 0x12, 0x12, 0x34]);
    poke(&mut s, FREE + 6, b"407");
    poke(&mut s, FREE + 9, &[0b1000_0001, 0b0000_0010]);
    poke(&mut s, FREE + 11, b"Hi\x01");
    poke(&mut s, FREE + 14, &[3, 1, 9]);
    poke(&mut s, FREE + 17, &[0x35, 0x33]);
    let reading = s.read();
    let number = |name: &str| reading[name].number();
    assert_eq!(number("u8"), Some(200));
    assert_eq!(number("i8"), Some(-2));
    assert_eq!(number("u16"), Some(0x1234));
    assert_eq!(number("u16be"), Some(0x3412));
    assert_eq!(number("bcd"), Some(1234), "first byte most significant");
    assert_eq!(number("digits"), Some(407));
    assert_eq!(number("screen_digits"), Some(42), "its own zero");
    let mut bits = vec![false; 16];
    bits[0] = true;
    bits[7] = true;
    bits[9] = true;
    assert_eq!(reading["bitset"], Read::Bits(bits));
    assert_eq!(reading["bitset"].to_string(), "[0, 7, 9]");
    assert_eq!(reading["bytes"], Read::Bytes(vec![200, 0xFE, 0x34]));
    assert_eq!(reading["bytes"].to_string(), "[0xC8, 0xFE, 0x34]");
    assert_eq!(reading["text"], Read::Text("Hi?".to_owned()));
    assert_eq!(reading["own_text"], Read::Text("CA?".to_owned()));
}

#[test]
fn a_table_reads_as_many_records_as_its_count_says() {
    let mut s = session(
        r#"
[values.carried]
addr = 0x8000
type = "u8"
[values.items]
type = "table"
addr = 0x8010
count = "carried"
stride = 2
fields.room = { offset = 0, type = "u8" }
fields.held = { offset = 1, type = "u8", bits = 0x80 }
"#,
    );
    poke(&mut s, FREE, &[2]);
    poke(&mut s, FREE + 0x10, &[7, 0x80, 9, 0x7F, 11, 0xFF]);
    let reading = s.read();
    assert_eq!(
        reading["items"].to_string(),
        "[{ held = true, room = 7 }, { held = false, room = 9 }]"
    );
    poke(&mut s, FREE, &[0]);
    assert_eq!(s.read()["items"], Read::Table(Vec::new()));
}

#[test]
fn the_first_state_that_matches_is_the_game_s() {
    let mut s = session(
        r#"
[values.mode]
addr = 0x8000
type = "u8"

[[states]]
name = "paused"
when = { value = "mode", equals = 2 }

[[states]]
name = "both"
pc = 0x0038
when = { value = "mode", bits = 0x81 }

[[states]]
name = "interrupted"
pc = [0x3D00, 0x0038]

[[states]]
name = "never"
pc = 0x3D00
"#,
    );
    let mut state_with = |mode: u8| {
        poke(&mut s, FREE, &[mode]);
        s.run_frame().expect("a frame");
        s.state().map(str::to_owned)
    };
    assert_eq!(state_with(2).as_deref(), Some("paused"), "first in order");
    assert_eq!(state_with(0x83).as_deref(), Some("both"), "pc and when");
    assert_eq!(
        state_with(0x80).as_deref(),
        Some("interrupted"),
        "when fails"
    );
}

#[test]
fn nothing_matching_is_no_state() {
    let mut s = session(
        r#"
[[states]]
name = "draft"

[[states]]
name = "never"
pc = 0x3D00
"#,
    );
    s.run_frame().expect("a frame");
    assert_eq!(s.state(), None);
}

#[test]
fn tests_combine_with_all_and_any() {
    let mut s = session(
        r#"
[values.a]
addr = 0x8000
type = "u8"
[values.b]
addr = 0x8001
type = "u8"
[values.name]
addr = 0x8002
type = "text"
len = 2

[[states]]
name = "text"
when = { value = "name", equals = 0 }

[[states]]
name = "combined"
when = { all = [{ value = "a", at_least = 3 }, { value = "a", at_most = 5 }], any = [{ value = "b", in = [1, 2] }, { value = "b", not = 0 }] }
"#,
    );
    let mut state_with = |a: u8, b: u8| {
        poke(&mut s, FREE, &[a, b]);
        s.run_frame().expect("a frame");
        s.state().map(str::to_owned)
    };
    assert_eq!(state_with(4, 1).as_deref(), Some("combined"));
    assert_eq!(state_with(4, 0), None, "no `any` holds");
    assert_eq!(state_with(6, 1), None, "an `all` fails");
}

#[test]
fn values_are_read_at_the_sync_address_when_the_frame_reaches_it() {
    let frames = r#"
[values.frames]
addr = 23672
type = "u8"
"#;
    // The ROM's interrupt handler counts FRAMES just after 0x0038.
    let mut synced = session(&format!("[run]\nsync = 0x0038\n{frames}"));
    synced.run_frame().expect("a frame");
    let after = i64::from(synced.machine().peek(FRAMES));
    assert_eq!(
        synced.reading()["frames"].number(),
        Some((after - 1).rem_euclid(256)),
        "before the handler counted"
    );

    let mut unsynced = session(frames);
    unsynced.run_frame().expect("a frame");
    let after = i64::from(unsynced.machine().peek(FRAMES));
    assert_eq!(unsynced.reading()["frames"].number(), Some(after));

    let mut never = session(&format!("[run]\nsync = 0x3D00\n{frames}"));
    never.run_frame().expect("a frame");
    let after = i64::from(never.machine().peek(FRAMES));
    assert_eq!(
        never.reading()["frames"].number(),
        Some(after),
        "at the frame's end when the sync address isn't reached"
    );
}

#[test]
fn writes_go_by_name_and_fit_the_value() {
    let mut s = session(
        r#"
[values.lives]
addr = 0x8000
type = "u8"
writable = true
[values.energy]
addr = 0x8001
type = "i8"
writable = true
[values.score]
addr = 0x8002
type = "bcd"
len = 3
writable = true
[values.shown]
addr = 0x8005
type = "digits"
len = 2
writable = true
[values.word]
addr = 0x8007
type = "u16be"
writable = true
[values.seen]
addr = 0x8009
type = "u8"
[values.name]
addr = 0x800A
type = "text"
len = 2
writable = true
[values.rom]
addr = 0x0100
type = "u8"
writable = true
"#,
    );
    s.write("lives", 9).expect("lives");
    s.write("energy", -3).expect("energy");
    s.write("score", 12_345).expect("score");
    s.write("shown", 7).expect("shown");
    s.write("word", 0x1234).expect("word");
    let m = s.machine();
    let bytes: Vec<u8> = (FREE..FREE + 9).map(|a| m.peek(a)).collect();
    assert_eq!(bytes, [9, 0xFD, 0x01, 0x23, 0x45, b'0', b'7', 0x12, 0x34]);
    assert_eq!(s.read()["score"].number(), Some(12_345));

    assert_eq!(s.write("lifes", 1), Err(WriteError::Unknown));
    assert_eq!(s.write("seen", 1), Err(WriteError::NotWritable));
    assert_eq!(s.write("name", 1), Err(WriteError::NotANumber));
    assert_eq!(s.write("rom", 1), Err(WriteError::NotInRam));
    assert_eq!(s.write("lives", 256), Err(WriteError::OutOfRange));
    assert_eq!(s.write("energy", 128), Err(WriteError::OutOfRange));
    assert_eq!(s.write("score", 1_000_000), Err(WriteError::OutOfRange));
    assert_eq!(s.write("shown", -1), Err(WriteError::OutOfRange));
}

#[test]
fn a_tape_without_a_definition_names_nothing() {
    let machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    let mut s = Session::new(machine, None);
    s.run_frame().expect("a frame");
    assert!(s.reading().is_empty());
    assert_eq!(s.state(), None);
}
