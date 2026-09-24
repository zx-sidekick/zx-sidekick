//! A gamepad, read as a joystick.
//!
//! The pad reports the five joystick bits in the Kempston port's order, and
//! the machine presses them however the game's chosen control method
//! listens, so the pad works whichever option was picked on the title
//! screen. The d-pad and the left stick move; what the bottom and left face
//! buttons press is the game's ([`Buttons`]): in Starquake the bottom one is
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

/// Which letters a pad's face buttons carry, from the maker it reports
/// itself as (#101). The buttons are read by position — `gilrs` names them
/// South, East, West and North whatever is printed on them — so this
/// changes only the letters shown in a legend, and which button confirms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    /// A on the bottom, B right, X left, Y top. The default: it is what
    /// most pads for a computer are printed with, and what an unrecognised
    /// pad is taken to be (#101, decision 6).
    #[default]
    Xbox,
    /// A right, B bottom, X top, Y left.
    Nintendo,
    /// The cross at the bottom, the circle right, the square left, the
    /// triangle on top.
    PlayStation,
}

/// Nintendo's USB vendor: a Pro Controller, Joy-Cons, or a third-party pad
/// in its Nintendo mode, which reports itself as one.
const NINTENDO: u16 = 0x057E;
/// Sony's.
const PLAYSTATION: u16 = 0x054C;

impl Layout {
    /// The layout a pad reporting `vendor` carries.
    #[must_use]
    pub fn of(vendor: Option<u16>) -> Layout {
        match vendor {
            Some(NINTENDO) => Layout::Nintendo,
            Some(PLAYSTATION) => Layout::PlayStation,
            _ => Layout::Xbox,
        }
    }

    /// Whether the button that confirms is the right-hand one rather than
    /// the bottom one: A is on the right of a Nintendo pad, and A confirms
    /// (#101, decision 1). The cross confirms on a PlayStation pad, which
    /// is at the bottom as on an Xbox one.
    #[must_use]
    pub fn confirms_east(self) -> bool {
        self == Layout::Nintendo
    }

    /// The confirm button's name in a sentence, as this pad has it printed:
    /// A on an Xbox or a Nintendo pad, the cross on a PlayStation one.
    #[must_use]
    pub fn confirm_name(self) -> &'static str {
        match self {
            Layout::PlayStation => "cross",
            Layout::Xbox | Layout::Nintendo => "A",
        }
    }
}

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
    /// The letters the first connected pad carries (#101, decision 7).
    pub layout: Layout,
}

impl Pad {
    /// The press that confirms in the picker, and the one that goes back:
    /// the bottom and right buttons, or the other way round on a pad whose
    /// A is on the right.
    #[must_use]
    pub fn confirm(&self) -> bool {
        if self.layout.confirms_east() {
            self.east
        } else {
            self.south
        }
    }

    #[must_use]
    pub fn cancel(&self) -> bool {
        if self.layout.confirms_east() {
            self.south
        } else {
            self.east
        }
    }
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

/// The joystick bits the bottom and left face buttons press: the game's
/// choice, since what a platformer's player presses most differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Buttons {
    /// The bottom face button (A on an Xbox pad).
    pub south: u8,
    /// The left one (X on an Xbox pad).
    pub west: u8,
}

pub struct Gamepad {
    gilrs: Option<gilrs::Gilrs>,
    buttons: Buttons,
    /// The picker's buttons at the last poll, to tell a press from a hold.
    was: Held,
    /// Joystick bits, and Start, kept from the game until they are let go:
    /// what was held as a picker closed (#25).
    held_back: u8,
    start_held_back: bool,
    /// Whether the next poll starts holding back whatever is down.
    hold_back_next: bool,
}

impl Gamepad {
    pub fn new(buttons: Buttons) -> Gamepad {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Gamepad {
                gilrs: Some(gilrs),
                buttons,
                was: [false; 8],
                held_back: 0,
                start_held_back: false,
                hold_back_next: false,
            },
            Err(e) => {
                eprintln!("no gamepad support: {e}");
                Gamepad {
                    gilrs: None,
                    buttons,
                    was: [false; 8],
                    held_back: 0,
                    start_held_back: false,
                    hold_back_next: false,
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
        // The first pad listed decides the letters; the rest are read for
        // what they are pressing (#101, decision 7).
        if let Some((_, first)) = gilrs.gamepads().next() {
            pad.layout = Layout::of(first.vendor_id());
        }
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
            // The bottom and left face buttons (A and X on an Xbox pad) as
            // the game lays them out. The others do nothing.
            if pressed(Button::South) {
                pad.bits |= self.buttons.south;
            }
            if pressed(Button::West) {
                pad.bits |= self.buttons.west;
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
        self.hold_back(&mut pad);
        pad
    }

    /// From the next poll, keeps whatever is held then from the game until
    /// each is let go: the button that closed a picker is not also a jump,
    /// a platform, or a press that ends a pause (#25).
    pub fn hold_back_held(&mut self) {
        self.hold_back_next = true;
    }

    fn hold_back(&mut self, pad: &mut Pad) {
        if std::mem::take(&mut self.hold_back_next) {
            self.held_back = pad.bits;
            self.start_held_back = pad.start;
        }
        self.held_back &= pad.bits;
        self.start_held_back &= pad.start;
        pad.bits &= !self.held_back;
        pad.start &= !self.start_held_back;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_comes_from_the_maker_the_pad_reports() {
        assert_eq!(Layout::of(Some(0x057E)), Layout::Nintendo);
        assert_eq!(Layout::of(Some(0x054C)), Layout::PlayStation);
        // An Xbox pad, an 8BitDo in its own mode, and a pad that reports no
        // maker at all: the letters most pads are printed with (#101).
        assert_eq!(Layout::of(Some(0x045E)), Layout::Xbox);
        assert_eq!(Layout::of(Some(0x2DC8)), Layout::Xbox);
        assert_eq!(Layout::of(None), Layout::Xbox);
    }

    #[test]
    fn a_is_the_button_that_confirms_wherever_it_is() {
        // The same press means opposite things on the two pads: A confirms
        // and B cancels, and A is the bottom button on an Xbox pad and the
        // right one on a Nintendo pad (#101, decision 1).
        let press = |layout, south, east| Pad {
            layout,
            south,
            east,
            ..Pad::default()
        };
        let bottom = press(Layout::Xbox, true, false);
        assert!(bottom.confirm() && !bottom.cancel(), "A on an Xbox pad");
        let right = press(Layout::Xbox, false, true);
        assert!(right.cancel() && !right.confirm(), "B on an Xbox pad");

        let bottom = press(Layout::Nintendo, true, false);
        assert!(bottom.cancel() && !bottom.confirm(), "B on a Switch pad");
        let right = press(Layout::Nintendo, false, true);
        assert!(right.confirm() && !right.cancel(), "A on a Switch pad");

        // Sony settled on the cross confirming, and it is at the bottom.
        let bottom = press(Layout::PlayStation, true, false);
        assert!(bottom.confirm(), "the cross on a PlayStation pad");
    }

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
        assert!(
            pad.north && !pad.east,
            "the top button switches the piece route"
        );
        presses(&mut pad, y, y);
        assert!(!pad.north, "a held top button is one switch");
    }

    #[test]
    fn the_confirm_button_is_named_as_the_pad_has_it() {
        assert_eq!(Layout::Xbox.confirm_name(), "A");
        assert_eq!(Layout::Nintendo.confirm_name(), "A");
        assert_eq!(Layout::PlayStation.confirm_name(), "cross");
    }

    #[test]
    fn what_was_held_as_a_picker_closed_waits_to_be_let_go() {
        let mut g = Gamepad {
            gilrs: None,
            buttons: Buttons {
                south: 0x10,
                west: 0x10,
            },
            was: [false; 8],
            held_back: 0,
            start_held_back: false,
            hold_back_next: false,
        };
        let held = |bits, start| Pad {
            bits,
            start,
            ..Pad::default()
        };
        g.hold_back_held();
        // A (fire) and Start still down: kept from the game.
        let mut pad = held(0x10, true);
        g.hold_back(&mut pad);
        assert_eq!((pad.bits, pad.start), (0, false));
        // Right pressed as well: that one is new, and gets through.
        let mut pad = held(0x11, true);
        g.hold_back(&mut pad);
        assert_eq!((pad.bits, pad.start), (0x01, false));
        // Let go, then pressed again: a press of its own.
        let mut pad = held(0, false);
        g.hold_back(&mut pad);
        let mut pad = held(0x10, true);
        g.hold_back(&mut pad);
        assert_eq!((pad.bits, pad.start), (0x10, true));
    }
}
