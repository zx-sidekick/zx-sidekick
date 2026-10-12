//! The game's font and pictures read from memory, with patterns of our own written there.

use sidekick_core::{Machine, Model, Session};
use sidekick_def::Definition;

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

const DEFINITION: &str = r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[values.font]
type = "font"
addr = 0x8000
first = 0x41
count = 2
width = 8
height = 2

[values.plain]
type = "sprites"
addr = 0x8100
count = 2
width = 16
height = 2

[values.interleaved]
type = "sprites"
addr = 0x8200
count = 1
width = 8
height = 2
mask = "interleaved"

[values.after]
type = "sprites"
addr = 0x8300
count = 1
width = 8
height = 2
mask = "after"
colors = { value = "inks" }

[values.inks]
type = "bytes"
addr = 0x8400
len = 1
"#;

fn session() -> Session {
    let definition = Definition::from_toml(DEFINITION).expect("the test definition parses");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    Session::new(machine, Some(definition))
}

fn poke(s: &mut Session, addr: u16, bytes: &[u8]) {
    for (at, &byte) in (addr..).zip(bytes) {
        s.machine_mut().write(at, byte);
    }
}

/// A picture as rows of `#` and `.`.
fn rows(b: &sidekick_core::Bitmap) -> Vec<String> {
    (0..b.height)
        .map(|y| {
            (0..b.width)
                .map(|x| if b.at(x, y) { '#' } else { '.' })
                .collect()
        })
        .collect()
}

#[test]
fn a_font_is_its_characters_from_the_first_code() {
    let mut s = session();
    poke(&mut s, 0x8000, &[0b1000_0001, 0b0110_0000, 0xFF, 0x00]);
    let font = s.font("font").expect("a font");
    let a = font.glyph('A').expect("A");
    assert_eq!(rows(a), ["#......#", ".##....."]);
    assert_eq!(rows(font.glyph('B').expect("B")), ["########", "........"]);
    assert!(font.glyph('C').is_none(), "past its count");
    assert!(font.glyph(' ').is_none(), "before its first");
}

#[test]
fn pictures_are_read_with_and_without_masks() {
    let mut s = session();
    poke(
        &mut s,
        0x8100,
        &[0x80, 0x01, 0xC0, 0x03, 0xFF, 0xFF, 0x00, 0x00],
    );
    let plain = s.sprites("plain").expect("pictures");
    assert_eq!(rows(&plain[0]), ["#..............#", "##............##"]);
    assert_eq!(rows(&plain[1])[0], "################");

    // A mask byte before each byte of the picture.
    poke(&mut s, 0x8200, &[0x00, 0xF0, 0xFF, 0x0F]);
    assert_eq!(
        rows(&s.sprites("interleaved").expect("pictures")[0]),
        ["####....", "....####"]
    );

    // The mask after the picture, and the ink from an attribute: bright (bit 6) red (2).
    poke(&mut s, 0x8300, &[0x18, 0x24, 0xFF, 0xFF]);
    poke(&mut s, 0x8400, &[0x42]);
    s.run_frame().expect("a frame, to read the inks");
    let after = &s.sprites("after").expect("pictures")[0];
    assert_eq!(rows(after), ["...##...", "..#..#.."]);
    assert_eq!(after.color, Some(10));
}
