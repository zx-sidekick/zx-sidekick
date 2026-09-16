//! A gamepad, read as a joystick.
//!
//! The pad reports the five joystick bits in the Kempston port's order, and
//! the machine presses them however the game's chosen control method
//! listens, so the pad works whichever option was picked on the title
//! screen. The d-pad and the left stick move; the bottom face button is
//! down and the left one fires, as platformers lay them out. Start pauses,
//! which freezes the emulation, and Start again continues (`freeze.rs`).
//! Select opens the guidance picker (#25), where the d-pad and stick work
//! it, A does the highlighted thing, and B or Select goes back.
//!
//! How the pad is attached is not this code's business, or `gilrs`'s. A
//! Bluetooth controller the operating system has paired is an ordinary
//! gamepad by the time it reaches here, exactly as a USB one is; both arrive
//! through the same platform API. Hot-plugging is handled either way, since
//! `poll` drains the event queue before reading, which is where a pad that
//! has just connected turns up.

/// How far a stick must move before it counts as a direction.
const DEADZONE: f32 = 0.5;

/// What the pads are asking for this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pad {
    /// The joystick bits, in the Kempston port's order.
    pub bits: u8,
    /// Start is held: pause.
    pub start: bool,
    /// Pressed since the last poll, for the picker: each is one press, not
    /// a button held down.
    pub select: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// The bottom face button (A on an Xbox pad): the picker's OK.
    pub south: bool,
    /// The right face button (B on an Xbox pad): the picker's back.
    pub east: bool,
    /// The top face button (Y on an Xbox pad): switches the piece route
    /// between the nearest missing pieces (#51).
    pub north: bool,
}

/// What the picker's buttons were at the last poll and are now, in the
/// order Select, up, down, left, right, A, B, and Y.
type Held = [bool; 8];

/// Sets the picker's presses in `pad`: the buttons down `now` that were not
/// at the last poll, `was`.
fn presses(pad: &mut Pad, now: Held, was: Held) {
    let pressed = |i: usize| now[i] && !was[i];
    pad.select = pressed(0);
    pad.up = pressed(1);
    pad.down = pressed(2);
    pad.left = pressed(3);
    pad.right = pressed(4);
    pad.south = pressed(5);
    pad.east = pressed(6);
    pad.north = pressed(7);
}

pub struct Gamepad {
    gilrs: Option<gilrs::Gilrs>,
    /// The picker's buttons at the last poll, to tell a press from a hold.
    was: Held,
}

impl Gamepad {
    pub fn new() -> Gamepad {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Gamepad {
                gilrs: Some(gilrs),
                was: [false; 8],
            },
            Err(e) => {
                eprintln!("no gamepad support: {e}");
                Gamepad {
                    gilrs: None,
                    was: [false; 8],
                }
            }
        }
    }

    /// What every connected pad together is asking for.
    pub fn poll(&mut self) -> Pad {
        let Some(gilrs) = &mut self.gilrs else {
            return Pad::default();
        };
        // Reading the state is what the events feed, so drain them first;
        // this is also where hot-plugged pads arrive.
        while gilrs.next_event().is_some() {}

        let mut pad = Pad::default();
        let mut now = [false; 8];
        for (_id, gamepad) in gilrs.gamepads() {
            use gilrs::{Axis, Button};
            let pressed = |b| gamepad.is_pressed(b);
            let (x, y) = (
                gamepad.value(Axis::LeftStickX),
                gamepad.value(Axis::LeftStickY),
            );
            if pressed(Button::DPadRight) || x > DEADZONE {
                pad.bits |= 0x01;
            }
            if pressed(Button::DPadLeft) || x < -DEADZONE {
                pad.bits |= 0x02;
            }
            if pressed(Button::DPadDown) || y < -DEADZONE {
                pad.bits |= 0x04;
            }
            if pressed(Button::DPadUp) || y > DEADZONE {
                pad.bits |= 0x08;
            }
            // The face buttons as a platformer lays them out: the bottom
            // one (A on an Xbox pad) is down, which in Starquake lays a
            // platform under Blob, the move a player makes most; the left
            // one (X) fires. The others do nothing.
            if pressed(Button::South) {
                pad.bits |= 0x04;
            }
            if pressed(Button::West) {
                pad.bits |= 0x10;
            }
            pad.start |= pressed(Button::Start);
            now[0] |= pressed(Button::Select);
            now[1] |= pressed(Button::DPadUp) || y > DEADZONE;
            now[2] |= pressed(Button::DPadDown) || y < -DEADZONE;
            now[3] |= pressed(Button::DPadLeft) || x < -DEADZONE;
            now[4] |= pressed(Button::DPadRight) || x > DEADZONE;
            now[5] |= pressed(Button::South);
            now[6] |= pressed(Button::East);
            now[7] |= pressed(Button::North);
        }
        presses(&mut pad, now, self.was);
        self.was = now;
        pad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_button_held_down_is_one_press() {
        let mut pad = Pad::default();
        let select_and_up = [true, true, false, false, false, false, false, false];
        presses(&mut pad, select_and_up, [false; 8]);
        assert!(pad.select && pad.up && !pad.down && !pad.south);
        presses(&mut pad, select_and_up, select_and_up);
        assert!(!pad.select && !pad.up, "still down is not pressed again");
        let b = [false, false, false, false, false, false, true, false];
        presses(&mut pad, b, select_and_up);
        assert!(pad.east && !pad.select);
        let y = [false, false, false, false, false, false, false, true];
        presses(&mut pad, y, b);
        assert!(pad.north && !pad.east, "Y switches the piece route");
        presses(&mut pad, y, y);
        assert!(!pad.north, "a held Y is one switch");
    }
}
