//! A gamepad, read as the Kempston joystick.
//!
//! A Kempston interface is a joystick port: the game reads five bits and
//! cannot tell what moved them, so a gamepad drives them exactly as the
//! hardware would. Pause is the odd one out — on a Spectrum it is a key, not
//! a joystick button — so Start presses Space, Starquake's pause key as the
//! tape ships it.
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
    /// The Kempston bits.
    pub bits: u8,
    /// Start is held: pause.
    pub start: bool,
}

pub struct Gamepad {
    gilrs: Option<gilrs::Gilrs>,
}

impl Gamepad {
    pub fn new() -> Gamepad {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Gamepad { gilrs: Some(gilrs) },
            Err(e) => {
                eprintln!("no gamepad support: {e}");
                Gamepad { gilrs: None }
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
            // Any of the buttons under a thumb or finger fires.
            let fire = [
                Button::South,
                Button::East,
                Button::North,
                Button::West,
                Button::RightTrigger,
                Button::LeftTrigger,
                Button::RightTrigger2,
                Button::LeftTrigger2,
            ];
            if fire.into_iter().any(pressed) {
                pad.bits |= 0x10;
            }
            pad.start |= pressed(Button::Start);
        }
        pad
    }
}
