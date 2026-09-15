//! The level 5 spike's search (#10): from a way into a room, on copies of
//! the machine, every input on every frame play reads it, keeping each
//! state of Blob once, until he leaves the room, dies, or a door, booth or
//! pyramid screen opens. Nothing here moves Blob: the game does, on the
//! copies. Its positive findings replay; its negative ones are not to be
//! trusted (the ticket's report says why).

use std::collections::{HashMap, HashSet, VecDeque};

use sidekick::Machine;
use sidekick::map::free;
use sidekick::starquake::{PLAY_INPUT, routine};

use crate::{BLOB, DOWN, LEFT, RIGHT, UP};

/// Whether the search may build platforms: the bar held at `0x32`, or at 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platforms {
    None,
    Unlimited,
}

/// The nine inputs: nothing, the four directions, then the diagonals.
pub const INPUTS: [u8; 9] = [
    0,
    RIGHT,
    LEFT,
    DOWN,
    UP,
    UP | RIGHT,
    UP | LEFT,
    DOWN | RIGHT,
    DOWN | LEFT,
];

/// The bytes of Blob's slot that make a state: all but the animation
/// counters, the pointers and the drain counter.
pub const FULL_KEY: &[usize] = &[
    5, 6, 10, 12, 15, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 29, 30, 31,
];
/// The coarse key the spike's whole-map runs used: position, the jump and
/// fall counter and the standing-on-a-platform byte. About 2.5 times
/// faster, and search-order dependent.
pub const COARSE_KEY: &[usize] = &[5, 6, 17, 20];

/// How a search runs.
#[derive(Clone, Debug)]
pub struct Settings {
    pub key: Vec<usize>,
    pub diagonals: bool,
    /// Drop states with Blob inside a solid cell, which stacking platforms
    /// can put him in.
    pub prune: bool,
    /// Frames each input is held before the next choice.
    pub hold: usize,
    pub platforms: Platforms,
}

impl Default for Settings {
    /// The settings every number in the report used: the coarse key, no
    /// diagonals, pruning, one frame a choice, unlimited platforms.
    fn default() -> Settings {
        Settings {
            key: COARSE_KEY.to_vec(),
            diagonals: false,
            prune: true,
            hold: 1,
            platforms: Platforms::Unlimited,
        }
    }
}

impl Settings {
    /// The settings from a tool's flags: `--full` for the full key,
    /// `--diagonals`, `--no-prune`, `--no-platforms`, `--hold=N`.
    #[must_use]
    pub fn from_args(args: &crate::Args) -> Settings {
        Settings {
            key: if args.flag("full") {
                FULL_KEY.to_vec()
            } else {
                COARSE_KEY.to_vec()
            },
            diagonals: args.flag("diagonals"),
            prune: !args.flag("no-prune"),
            hold: args.value("hold", 1),
            platforms: if args.flag("no-platforms") {
                Platforms::None
            } else {
                Platforms::Unlimited
            },
        }
    }

    /// A state's key: the chosen bytes of Blob's slot.
    #[must_use]
    pub fn key(&self, m: &Machine) -> Vec<u8> {
        self.key.iter().map(|&i| m.zx.mem[BLOB + i]).collect()
    }

    fn inputs(&self) -> &[u8] {
        if self.diagonals {
            &INPUTS
        } else {
            &INPUTS[..5]
        }
    }
}

/// Whether Blob overlaps a solid cell, which the game's collision test would
/// never let him walk into.
#[must_use]
pub fn in_wall(m: &Machine) -> bool {
    let (x, y) = crate::blob(m);
    let top = 0x18i32 - ((i32::from(y) + 1) >> 3);
    let (c0, c1) = (i32::from(x) >> 3, (i32::from(x) + 7) >> 3);
    for row in [top, top + 1] {
        for col in c0..=c1 + 1 {
            if !(6..24).contains(&row) || !(0..32).contains(&col) {
                continue;
            }
            if !free(m.zx.mem[0x5800 + row as usize * 32 + col as usize]) {
                return true;
            }
        }
    }
    false
}

/// An exit found: the room entered and where Blob was once it settled.
pub type Exit = (u16, u8, u8);

/// One room's search, which can stop and carry on.
#[derive(Default)]
pub struct Room {
    pub seen: HashSet<Vec<u8>>,
    pub frames: u64,
    pub deaths: usize,
    pub pruned: usize,
    /// Where a door, booth or pyramid screen opened.
    pub modal: HashSet<(u8, u8)>,
    /// The machine just after each exit, for the next room's search (taken
    /// away as they are handed on).
    pub exits: HashMap<Exit, Machine>,
    /// Every exit ever found.
    pub exit_keys: HashSet<Exit>,
    /// How each state was first reached: its parent's key and the input.
    pub parent: HashMap<Vec<u8>, (Vec<u8>, u8)>,
    /// How each exit was first reached.
    pub exit_path: HashMap<Exit, (Vec<u8>, u8)>,
    /// States still to expand.
    pub queue: VecDeque<Machine>,
    /// Where each entry came from: the room before and its exit.
    pub origin: HashMap<Vec<u8>, (u16, Exit)>,
}

impl Room {
    /// The inputs from an entry to the state `key`, and the entry's key.
    #[must_use]
    pub fn path(&self, mut key: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
        let mut inputs = Vec::new();
        while let Some((parent, input)) = self.parent.get(&key) {
            inputs.push(*input);
            key = parent.clone();
        }
        inputs.reverse();
        (key, inputs)
    }

    /// Adds entries to search from; how many were new.
    pub fn seed(&mut self, entries: Vec<Machine>, settings: &Settings) -> usize {
        let mut added = 0;
        for mut m in entries {
            m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
            if self.seen.insert(settings.key(&m)) {
                self.queue.push_back(m);
                added += 1;
            }
        }
        added
    }

    /// Explores from `entries`, machines standing in `room`, until nothing
    /// is left.
    pub fn explore(&mut self, room: u16, entries: Vec<Machine>, settings: &Settings) {
        self.seed(entries, settings);
        self.explore_until(room, settings, None);
    }

    /// Explores until an exit into `until` is newly found (true) or nothing
    /// is left (false).
    pub fn explore_until(&mut self, room: u16, settings: &Settings, until: Option<u16>) -> bool {
        while let Some(m) = self.queue.pop_front() {
            let mut found = false;
            let mut reads = true;
            for &input in settings.inputs() {
                if !reads {
                    break;
                }
                let mut n = m.clone();
                let mut hits = crate::frame(&mut n, input, settings.platforms);
                self.frames += 1;
                for _ in 1..settings.hold {
                    if crate::room(&n) != room
                        || hits.contains(&routine::DEATH)
                        || hits.contains(&routine::MODAL)
                    {
                        break;
                    }
                    hits.extend(crate::frame(&mut n, input, settings.platforms));
                    self.frames += 1;
                }
                // A frame that reads no input is not worth branching on.
                if input == 0 && !hits.contains(&PLAY_INPUT) {
                    reads = false;
                }
                if hits.contains(&routine::DEATH) {
                    self.deaths += 1;
                    continue;
                }
                if hits.contains(&routine::MODAL) {
                    self.modal.insert(crate::blob(&m));
                    continue;
                }
                let now = crate::room(&n);
                if now != room {
                    let (settled, frames) = crate::settle(&mut n, settings.platforms);
                    self.frames += frames;
                    if settled && crate::room(&n) == now {
                        let (x, y) = crate::blob(&n);
                        let exit = (now, x, y);
                        if self.exit_keys.insert(exit) {
                            self.exit_path.insert(exit, (settings.key(&m), input));
                            self.exits.insert(exit, n);
                            if until == Some(now) {
                                found = true;
                            }
                        }
                    }
                    continue;
                }
                if settings.prune && in_wall(&n) {
                    self.pruned += 1;
                    continue;
                }
                let key = settings.key(&n);
                if !self.seen.contains(&key) {
                    self.parent.insert(key.clone(), (settings.key(&m), input));
                    self.seen.insert(key);
                    self.queue.push_back(n);
                }
            }
            if found {
                return true;
            }
        }
        false
    }
}

/// Replays `inputs` from `entry`: where Blob is once a room change settles,
/// if one happens.
#[must_use]
pub fn replay(entry: &Machine, inputs: &[u8], platforms: Platforms) -> Option<Exit> {
    let mut m = entry.clone();
    m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
    let room = crate::room(&m);
    for &input in inputs {
        crate::frame(&mut m, input, platforms);
        if crate::room(&m) != room {
            let now = crate::room(&m);
            crate::settle(&mut m, platforms);
            let (x, y) = crate::blob(&m);
            return Some((now, x, y));
        }
    }
    None
}
