//! The machine from the rustzx fork builds, runs and boots its bundled ROM.

use sidekick_core::{Machine, Model};

/// Frames for the 48K ROM to clear memory, set its system variables and print its message.
const BOOT_FRAMES: usize = 100;

/// P-RAMT, the system variable holding the last byte of RAM, set by the ROM's start-up.
const P_RAMT: u16 = 23732;

#[test]
fn the_48k_rom_is_there_and_boots() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    assert_eq!(machine.peek(0x0000), 0xF3, "the ROM starts with DI");

    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    assert_eq!(machine.peek16(P_RAMT), 0xFFFF, "the ROM found 48K of RAM");

    let picture = machine.picture();
    let ink = (0..picture.height())
        .flat_map(|y| (0..picture.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| picture.colour(x, y) == 0)
        .count();
    assert!(ink > 0, "the copyright message is on the screen");
}
