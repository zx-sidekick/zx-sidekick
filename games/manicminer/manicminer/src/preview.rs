//! The jump preview (#155): what a jump from where Willy stands would do,
//! found by the game itself. Each of the three jumps is run on a copy of
//! the machine with its keys pressed as a player would press them, until
//! Willy lands or dies. Nothing about how he jumps is reimplemented, and the
//! machine given is never changed.

use zx_spectrum::Key;

use crate::Machine;
use crate::facts::{at, routine};
use crate::play::Play;

/// Which way a jump goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Left,
    Up,
    Right,
}

impl Way {
    /// All three, left to right.
    pub const ALL: [Way; 3] = [Way::Left, Way::Up, Way::Right];

    /// The key for its direction, if it has one.
    fn key(self) -> Option<&'static str> {
        match self {
            Way::Left => Some("o"),
            Way::Up => None,
            Way::Right => Some("p"),
        }
    }
}

/// How a jump ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Lands,
    Dies,
}

/// A jump: the way it goes, the middle of Willy in the cavern's pixels at
/// each step it moves, from where he stands to where it ends, and how it
/// ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Jump {
    pub way: Way,
    pub path: Vec<(u8, u8)>,
    pub end: End,
}

/// The longest a jump is followed, in frames: a jump and the longest fall
/// take far fewer.
const LONGEST: u32 = 300;

/// Whether Willy is standing, where a jump can start.
#[must_use]
pub fn standing(m: &Machine) -> bool {
    m.zx.mem[usize::from(at::AIRBORNE)] == 0
}

/// The middle of Willy's 16 pixels square, in the cavern's pixels.
#[must_use]
pub fn willy(m: &Machine) -> (u8, u8) {
    let mem = &m.zx.mem;
    let cell = u16::from_le_bytes([
        mem[usize::from(at::WILLY_CELL)],
        mem[usize::from(at::WILLY_CELL) + 1],
    ]);
    let col = (cell.wrapping_sub(at::ATTRIBUTES) & 31) as u8;
    let frame = mem[usize::from(at::WILLY_FRAME)] & 3;
    let y = mem[usize::from(at::WILLY_Y)] / 2;
    (col * 8 + frame * 2 + 8, y + 8)
}

fn facing_left(m: &Machine) -> bool {
    m.zx.mem[usize::from(at::FACING)] & 1 != 0
}

fn press(m: &mut Machine, name: &str) {
    m.zx.set_key(Key::by_name(name).expect("a key"), true);
}

/// The three jumps from where Willy stands, with the training switches in
/// force in `m`; none if he is not standing.
#[must_use]
pub fn jumps(m: &Machine) -> Vec<Jump> {
    if !standing(m) {
        return Vec::new();
    }
    Way::ALL.iter().filter_map(|&way| jump(m, way)).collect()
}

/// One jump from where Willy stands in `m`, if it starts at all. A jump
/// against the way he faces turns him first, as a player does: the game
/// spends the pass that turns him, and a jump pressed then would go
/// straight up.
#[must_use]
pub fn jump(m: &Machine, way: Way) -> Option<Jump> {
    let mut c = m.clone();
    // The copy plays nobody's keys but these, and does nothing else the
    // machine's rules might: only the switches are kept.
    c.rules = Play::default();
    c.rules.training = m.rules.training;
    c.watch = Vec::new();
    c.hold = None;
    c.zx.kempston = 0;
    let wrong_way = match way {
        Way::Left => !facing_left(&c),
        Way::Right => facing_left(&c),
        Way::Up => false,
    };
    let mut turning = wrong_way;
    let turn_key = way.key().unwrap_or("space");
    let mut path = vec![willy(&c)];
    let mut started = false;
    for _ in 0..LONGEST {
        c.zx.release_all_keys();
        if turning {
            press(&mut c, turn_key);
        } else if !started {
            press(&mut c, "space");
            if let Some(k) = way.key() {
                press(&mut c, k);
            }
        }
        let mut killed = false;
        c.run_frame_observing(|z| {
            let pc = z.pc();
            killed |= pc == routine::KILL || pc == routine::KILL_FALL;
        });
        c.zx.speaker.clear();
        let air = c.zx.mem[usize::from(at::AIRBORNE)];
        if turning {
            turning = facing_left(&c) != (way == Way::Left);
            // Turning may walk him a step: the jump starts where he is.
            path = vec![willy(&c)];
            continue;
        }
        if killed || air == 0xFF {
            return Some(Jump {
                way,
                path,
                end: End::Dies,
            });
        }
        started |= air != 0;
        let at = willy(&c);
        if path.last() != Some(&at) {
            path.push(at);
        }
        if started && air == 0 {
            return Some(Jump {
                way,
                path,
                end: End::Lands,
            });
        }
    }
    None
}
