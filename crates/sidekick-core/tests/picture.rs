//! The picture: 320×256, the screen with 32 pixels of border on every side.

use sidekick_core::{Machine, Model, Picture};

const WHITE: u8 = 7;
const RED: u8 = 2;
const BLACK: u8 = 0;

fn booted() -> Machine {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..100 {
        machine.run_frame().expect("a frame");
    }
    machine
}

/// The colors of the 32-pixel border on all four sides.
fn border(picture: &Picture) -> Vec<u8> {
    (0..256)
        .flat_map(|y| (0..320).map(move |x| (x, y)))
        .filter(|&(x, y)| !(32..288).contains(&x) || !(32..224).contains(&y))
        .map(|(x, y)| picture.color(x, y))
        .collect()
}

#[test]
fn the_picture_is_320_by_256() {
    let machine = booted();
    assert_eq!(machine.picture().width(), 320);
    assert_eq!(machine.picture().height(), 256);
    assert_eq!(machine.picture().pixels().len(), 320 * 256);
}

#[test]
fn after_start_up_the_border_is_white_and_the_message_is_in_the_screen() {
    let machine = booted();
    let picture = machine.picture();
    assert!(
        border(picture).iter().all(|&c| c == WHITE),
        "a white border"
    );
    // The ROM's copyright line sits on the screen's last character row.
    let ink = (216..224)
        .flat_map(|y| (32..288).map(move |x| (x, y)))
        .filter(|&(x, y)| picture.color(x, y) == BLACK)
        .count();
    assert!(ink > 0, "the copyright line is inside the screen area");
}

#[test]
fn a_red_border_reddens_all_four_sides_and_leaves_the_screen() {
    let mut machine = booted();
    let screen_before: Vec<u8> = (32..224)
        .flat_map(|y| (32..288).map(move |x| (x, y)))
        .map(|(x, y)| machine.picture().color(x, y))
        .collect();
    // DI; LD A,2; OUT (254),A; JR $
    for (addr, byte) in (0x9000..).zip([0xF3, 0x3E, 0x02, 0xD3, 0xFE, 0x18, 0xFE]) {
        machine.write(addr, byte);
    }
    machine.registers_mut().set_pc(0x9000);
    machine.run_frame().expect("a frame");
    machine.run_frame().expect("a frame");
    let picture = machine.picture();
    assert!(border(picture).iter().all(|&c| c == RED), "a red border");
    let screen_after: Vec<u8> = (32..224)
        .flat_map(|y| (32..288).map(move |x| (x, y)))
        .map(|(x, y)| picture.color(x, y))
        .collect();
    assert!(screen_after == screen_before, "the screen is untouched");
}
