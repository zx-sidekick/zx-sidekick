//! What the lab tools share: the game brought into play from the player's
//! tape, Blob stood somewhere in a room, every room's cells, and the facts
//! the tools rest on that `starquake::facts` does not record yet.
//!
//! Every tool takes the assets folder first, as `sk-check` does, reads
//! `starquake.tap` from it and writes whatever it produces into it, which is
//! ignored by git: nothing derived from the game is ever committed
//! (`GOAL.md`, rule 1).

use std::path::{Path, PathBuf};

use starquake::Machine;
use starquake::facts::{ENTRY_PC, ENTRY_SP, PLAY_INPUT, at, routine};
use zx_spectrum::Key;

pub mod exits;
pub mod raster;
pub mod rooms;
pub mod search;

/// Blob's energy, 127 as play starts (#8).
pub const ENERGY: u16 = 0xD2CD;
/// The platform bar, `0x32` as play starts; building one takes 2 (#8).
pub const PLATFORMS: u16 = 0xD2CE;
/// The energy drain counter's offset in Blob's slot: it rises by one a frame
/// and enemy contact pushes it past its threshold (#8).
pub const DRAIN: usize = 0x18;
/// The built platform: its cell, then a life that runs out after about 160
/// frames (#10).
pub const PLATFORM_RECORD: u16 = 0xDBBB;
/// The attributes of a vacuum tube's cells, bright with green paper, used for
/// nothing else on the planet: standing on one carries Blob up (#10).
pub const LIFT_ATTRS: [u8; 2] = [0x60, 0x64];
/// Blob's slot in memory, as a byte offset.
pub const BLOB: usize = at::ENTITIES as usize;

/// The Kempston bits the game reads: right, left, down (which builds a
/// platform), up (which picks up, and hovers on a pad).
pub const RIGHT: u8 = 1;
pub const LEFT: u8 = 2;
pub const DOWN: u8 = 4;
pub const UP: u8 = 8;

/// The command line: the assets folder, then the rest, with `--flag` and
/// `--name=value` taken out.
pub struct Args {
    pub dir: PathBuf,
    pub rest: Vec<String>,
    flags: Vec<String>,
}

impl Args {
    /// Reads the command line, printing `usage` and exiting when the folder
    /// is missing.
    #[must_use]
    pub fn parse(usage: &str) -> Args {
        let mut all = std::env::args().skip(1);
        let Some(dir) = all.next() else {
            eprintln!("usage: {usage}");
            std::process::exit(2);
        };
        let (flags, rest): (Vec<String>, Vec<String>) = all.partition(|a| a.starts_with("--"));
        Args {
            dir: PathBuf::from(dir),
            rest,
            flags,
        }
    }

    /// The `n`th argument after the folder, parsed, or `default`.
    #[must_use]
    pub fn get<T: std::str::FromStr>(&self, n: usize, default: T) -> T {
        self.rest
            .get(n)
            .and_then(|a| a.parse().ok())
            .unwrap_or(default)
    }

    /// Whether `--name` was given.
    #[must_use]
    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == &format!("--{name}"))
    }

    /// The value of `--name=value`, parsed, or `default`.
    #[must_use]
    pub fn value<T: std::str::FromStr>(&self, name: &str, default: T) -> T {
        self.flags
            .iter()
            .find_map(|f| f.strip_prefix(&format!("--{name}="))?.parse().ok())
            .unwrap_or(default)
    }

    /// A list of rooms from argument `n`, comma-separated.
    #[must_use]
    pub fn rooms(&self, n: usize) -> Vec<u16> {
        self.rest
            .get(n)
            .map(|a| a.split(',').filter_map(|r| r.parse().ok()).collect())
            .unwrap_or_default()
    }

    /// A path inside the assets folder.
    #[must_use]
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// The tape, read from the folder; exits when it cannot be.
    #[must_use]
    pub fn tape(&self) -> Vec<u8> {
        read(&self.dir, "starquake.tap")
    }
}

fn read(dir: &Path, name: &str) -> Vec<u8> {
    std::fs::read(dir.join(name)).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {e}", dir.join(name).display());
        std::process::exit(2);
    })
}

/// The game brought into play from the tape: Kempston chosen on the menu, a
/// game started, the intro text passed, and the play loop reached.
///
/// # Panics
///
/// If the tape does not hold the game.
#[must_use]
pub fn into_play(tape: &[u8]) -> Machine {
    let mut m = Machine::from_tape(tape, ENTRY_PC, ENTRY_SP).expect("the game's tape");
    let key = |n: &str| Key::by_name(n).expect("a key name");
    m.watch = vec![routine::MAIN_LOOP];
    for frame in 0..600u64 {
        m.zx.release_all_keys();
        match frame {
            50..=54 => m.zx.set_key(key("1"), true),
            100..=104 => m.zx.set_key(key("0"), true),
            330..=334 => m.zx.set_key(key("enter"), true),
            _ => {}
        }
        if m.run_frame().contains(&routine::MAIN_LOOP) && frame > 400 {
            break;
        }
    }
    m
}

/// The room Blob is in.
#[must_use]
pub fn room(m: &Machine) -> u16 {
    m.zx.read16(at::ROOM)
}

/// Blob's position: pixels from the left, and from the bottom.
#[must_use]
pub fn blob(m: &Machine) -> (u8, u8) {
    (m.zx.mem[BLOB + 5], m.zx.mem[BLOB + 6])
}

/// The screen row of Blob's top cell when he stands at `y`: he is two cells
/// tall, and standing on the floor of room row 15 he reads `y = 39`, his
/// top cell at screen row 19.
#[must_use]
pub fn top_row(y: u8) -> u8 {
    (0xBF - y) >> 3
}

/// The `y` at which Blob stands with his top cell in room row `row` (0 to
/// 15, from the top of the play area): the inverse of [`top_row`].
#[must_use]
pub fn y_for_row(row: u8) -> u8 {
    143 - 8 * row
}

/// One frame with `input` on the Kempston port, energy held full, the drain
/// counter at 0 and the platform bar as `platforms` says, so enemies do no
/// harm and platforms never run out (or never exist).
pub fn frame(m: &mut Machine, input: u8, platforms: search::Platforms) -> Vec<u16> {
    m.zx.release_all_keys();
    m.zx.kempston = input;
    m.zx.mem[BLOB + DRAIN] = 0;
    m.zx.mem[usize::from(ENERGY)] = 127;
    m.zx.mem[usize::from(PLATFORMS)] = match platforms {
        search::Platforms::None => 0,
        search::Platforms::Unlimited => 0x32,
    };
    m.run_frame()
}

/// After a room change: frames with no input until play reads the controls
/// again, which takes 13 frames of drawing. Whether it did within 200, and
/// how many frames it took.
pub fn settle(m: &mut Machine, platforms: search::Platforms) -> (bool, u64) {
    for i in 0..200 {
        if frame(m, 0, platforms).contains(&PLAY_INPUT) {
            return (true, i + 1);
        }
    }
    (false, 200)
}

/// A copy of `base` with the game entered into `room` as if walking in, and
/// Blob then stood at (`x`, `y`) and the room settled. `None` when the room
/// does not settle or Blob leaves it while settling.
///
/// # Panics
///
/// If the game's room entry does not reach its play loop.
#[must_use]
pub fn stand(base: &Machine, room: u16, x: u8, y: u8) -> Option<Machine> {
    let mut m = base.clone();
    m.zx.write16(at::ROOM, room);
    m.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
    assert!(
        m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000),
        "room {room} did not reach the play loop"
    );
    m.zx.t = 0;
    m.zx.set_interrupts(true);
    m.zx.mem[BLOB + 5] = x;
    m.zx.mem[BLOB + 6] = y;
    m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
    let (settled, _) = settle(&mut m, search::Platforms::Unlimited);
    (settled && self::room(&m) == room).then_some(m)
}

/// The inventory: four slots of (graphic, colour), which the game consults
/// at a door or a space lock (#10).
pub const INVENTORY: u16 = 0xD2D2;
/// The eight space locks: (room low byte, flags with the room's high
/// bit), the low seven bits of the flags cleared once the pad has been
/// switched with item `0x10` (#10).
pub const PADS: u16 = 0x95F0;
pub const PAD_COUNT: usize = 8;

/// The per-game random seed, a word, which the security door screen mixes
/// with the room to choose what a door asks for (#33).
pub const SEED: u16 = 0xD2C6;
/// The code a screen asks for: its column and row on screen, its length,
/// then that many (graphic, matched) pairs (#33).
pub const CODE: u16 = 0xD5F4;

/// Puts the first item drawn with `graphic` into Blob's inventory on this
/// machine, as the game keeps a carried item: its row byte set to 2, the
/// first inventory slot, its room bits kept, and the slot itself at
/// [`INVENTORY`] holding the graphic and its colour. Whether an item with
/// that graphic exists.
pub fn carry(m: &mut Machine, graphic: u8) -> bool {
    for i in 0..at::ITEM_COUNT {
        let a = usize::from(at::ITEMS) + i * 4;
        if m.zx.mem[a + 3] == graphic {
            m.zx.mem[a + 1] = (m.zx.mem[a + 1] & 0x80) | 2;
            // The first free slot of the four.
            let slot = (0..4)
                .map(|s| usize::from(INVENTORY) + s * 2)
                .find(|&s| m.zx.mem[s] == 0)
                .unwrap_or(usize::from(INVENTORY));
            m.zx.mem[slot] = graphic;
            m.zx.mem[slot + 1] = m.zx.mem[a] >> 5;
            return true;
        }
    }
    false
}

/// A small xorshift, for walks that are the same every run.
pub struct Rng(pub u64);

impl Rng {
    /// A number below `n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}
