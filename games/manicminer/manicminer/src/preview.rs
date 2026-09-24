//! The jump preview (#155): what a jump from where Willy stands would do,
//! found by the game itself. Each of the three jumps is run on a copy of
//! the machine with its keys pressed as a player would press them, until
//! Willy lands or dies. Nothing about how he jumps is reimplemented, and the
//! machine given is never changed.

use crate::Machine;
use crate::facts::{at, routine};
use crate::play::{Jumping, Play};

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

    /// The key a player holds for its direction, if it has one: O for
    /// left, P for right.
    #[must_use]
    pub fn key(self) -> Option<&'static str> {
        match self {
            Way::Left => Some("o"),
            Way::Up => None,
            Way::Right => Some("p"),
        }
    }

    /// Whether Willy, facing left or not, faces this way: straight up
    /// needs no turn.
    #[must_use]
    pub fn faced(self, facing_left: bool) -> bool {
        match self {
            Way::Left => facing_left,
            Way::Up => true,
            Way::Right => !facing_left,
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
/// each pass of the main loop he moves in, from where he stands to where it
/// ends, and how it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Jump {
    pub way: Way,
    pub path: Vec<(u8, u8)>,
    pub end: End,
}

/// The longest a jump is followed, in frames: a jump and the longest fall
/// take far fewer.
const LONGEST: u32 = 300;

/// Whether Willy is on the ground, where a jump can start.
#[must_use]
pub fn on_ground(m: &Machine) -> bool {
    m.zx.mem[usize::from(at::AIRBORNE)] == 0
}

/// Whether Willy is walking: bit 1 of [`at::FACING`], which the game sets
/// while he moves.
#[must_use]
pub fn walking(m: &Machine) -> bool {
    m.zx.mem[usize::from(at::FACING)] & 2 != 0
}

/// The middle of Willy's 16 pixels square, in the cavern's pixels.
#[must_use]
pub fn willy(m: &Machine) -> (u8, u8) {
    middle(&m.zx.mem[..])
}

/// [`willy`], from the machine's memory.
fn middle(mem: &[u8]) -> (u8, u8) {
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

/// The jumps from where Willy is, with the training switches in force in
/// `m`: all three while he stands still; while he walks, the two he can
/// make without turning, the way he walks and straight up (#156). None in
/// the air.
#[must_use]
pub fn jumps(m: &Machine) -> Vec<Jump> {
    if !on_ground(m) {
        return Vec::new();
    }
    let ahead = if facing_left(m) {
        Way::Left
    } else {
        Way::Right
    };
    let ways: Vec<Way> = Way::ALL
        .into_iter()
        .filter(|&way| !walking(m) || way == Way::Up || way == ahead)
        .collect();
    // Each on a thread of its own: the same work, done in a third of the
    // time (#156).
    std::thread::scope(|s| {
        let running: Vec<_> = ways
            .iter()
            .map(|&way| s.spawn(move || jump(m, way)))
            .collect();
        running
            .into_iter()
            .filter_map(|r| r.join().ok().flatten())
            .collect()
    })
}

/// One jump from where Willy is in `m`, if it starts at all. A jump
/// against the way he faces turns him first, as a player does: the game
/// spends the pass that turns him, and a jump pressed then would go
/// straight up. The keys are pressed a pass at a time
/// ([`Play::jumping`]), so how many passes the copy makes a frame changes
/// nothing.
#[must_use]
pub fn jump(m: &Machine, way: Way) -> Option<Jump> {
    run(m, way, true)
}

/// [`jump`] with the picture, the scores and the tune left in, as the check
/// runs it to prove going without them changes nothing (#156).
#[must_use]
pub fn jump_shown(m: &Machine, way: Way) -> Option<Jump> {
    run(m, way, false)
}

fn run(m: &Machine, way: Way, unseen: bool) -> Option<Jump> {
    let mut c = m.clone();
    // The copy plays nobody's keys but these, and does nothing else the
    // machine's rules might: only the switches are kept.
    c.rules = Play::default();
    c.rules.training = m.rules.training;
    // Nobody sees or hears the copy: it goes without the picture, the
    // scores and the tune, a third of the work (#156).
    c.rules.unseen = unseen;
    c.rules.jumping = Some(Jumping {
        way,
        started: false,
    });
    c.watch = Vec::new();
    c.hold = None;
    c.zx.kempston = 0;
    c.zx.release_all_keys();
    // Where he is at the top of each pass, and whether he is off the
    // ground there.
    let mut passes: Vec<((u8, u8), bool)> = vec![(willy(&c), false)];
    for _ in 0..LONGEST {
        let mut killed = false;
        c.run_frame_observing(|z| {
            let pc = z.pc();
            killed |= pc == routine::KILL || pc == routine::KILL_FALL;
            if pc == routine::MAIN_LOOP {
                passes.push((middle(&z.mem[..]), z.mem[usize::from(at::AIRBORNE)] != 0));
            }
        });
        c.zx.speaker.clear();
        let air = c.zx.mem[usize::from(at::AIRBORNE)];
        let dies = killed || air == 0xFF;
        let started = c.rules.jumping.is_some_and(|j| j.started) || air != 0;
        if dies || (started && air == 0) {
            passes.push((willy(&c), air != 0));
            // From the last pass on the ground before he left it.
            let first = passes.iter().position(|&(_, up)| up).unwrap_or(1);
            let mut path: Vec<(u8, u8)> = Vec::new();
            for &(at, _) in &passes[first.saturating_sub(1)..] {
                if path.last() != Some(&at) {
                    path.push(at);
                }
            }
            let end = if dies { End::Dies } else { End::Lands };
            return Some(Jump { way, path, end });
        }
    }
    None
}
