use sidekick::Machine;
use sidekick::starquake::{at, routine};
use std::collections::{HashMap, HashSet, VecDeque};

pub const E: usize = at::ENTITIES as usize;
pub const INPUTS: [u8; 9] = [0, 0x01, 0x02, 0x04, 0x08, 0x09, 0x0a, 0x05, 0x06];
pub const MASK: [usize; 19] = [5, 6, 10, 12, 15, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 29, 30, 31];

pub fn key(m: &Machine) -> Vec<u8> {
    thread_local! {
        static M: Vec<usize> = std::env::var("MASK").ok().map(|v| v.split(',').map(|x| x.parse().unwrap()).collect()).unwrap_or(MASK.to_vec());
    }
    M.with(|mask| mask.iter().map(|&i| m.zx.mem[E + i]).collect())
}

#[derive(Clone, Copy)]
pub enum Platforms { None, Unlimited }

/// Whether Blob overlaps a solid cell (a place the game's collision test would not let him walk into).
pub fn in_wall(m: &Machine) -> bool {
    let (x, y) = (m.zx.mem[E + 5], m.zx.mem[E + 6]);
    let top = 0x18i32 - ((i32::from(y) + 1) >> 3);
    let (c0, c1) = (i32::from(x) >> 3, (i32::from(x) + 7) >> 3);
    for row in [top, top + 1] {
        for col in c0..=c1 + 1 {
            if !(6..24).contains(&row) || !(0..32).contains(&col) { continue; }
            if !sidekick::map::free(m.zx.mem[0x5800 + row as usize * 32 + col as usize]) { return true; }
        }
    }
    false
}

pub fn prepare(m: &mut Machine, p: Platforms) {
    m.zx.mem[E + 0x18] = 0;
    m.zx.mem[0xD2CD] = 127;
    m.zx.mem[0xD2CE] = match p { Platforms::None => 0, Platforms::Unlimited => 0x32 };
}

#[derive(Default)]
pub struct Room {
    pub seen: HashSet<Vec<u8>>,
    pub frames: u64,
    pub deaths: usize,
    pub pruned: usize,
    pub modal: HashSet<(u8, u8)>,
    /// (room entered, x, y there) -> the machine just after crossing.
    pub exits: HashMap<(u16, u8, u8), Machine>,
    /// Every exit ever found (the machines in `exits` may be taken away).
    pub exit_keys: HashSet<(u16, u8, u8)>,
    /// How each state was first reached: its parent's key and the input.
    pub parent: HashMap<Vec<u8>, (Vec<u8>, u8)>,
    /// How each exit was first reached.
    pub exit_path: HashMap<(u16, u8, u8), (Vec<u8>, u8)>,
    /// States still to expand, so a search can stop early and carry on later.
    pub queue: VecDeque<Machine>,
    /// Where each entry came from: the room before and its exit.
    pub origin: HashMap<Vec<u8>, (u16, (u16, u8, u8))>,
}

/// One frame as the search runs it.
pub fn frame(n: &mut Machine, inp: u8, p: Platforms) -> Vec<u16> {
    n.zx.release_all_keys();
    n.zx.kempston = inp;
    prepare(n, p);
    n.run_frame()
}

/// After a room change: frames with no input until play reads input again.
pub fn settle(n: &mut Machine, p: Platforms) -> (bool, u64) {
    for i in 0..200 {
        if frame(n, 0, p).contains(&sidekick::starquake::PLAY_INPUT) { return (true, i + 1); }
    }
    (false, 200)
}

impl Room {
    /// The inputs from an entry to `k`, and the entry's key.
    pub fn path(&self, mut k: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
        let mut inputs = Vec::new();
        while let Some((p, i)) = self.parent.get(&k) { inputs.push(*i); k = p.clone(); }
        inputs.reverse();
        (k, inputs)
    }
}

impl Room {
    /// Adds entries to search from.
    pub fn seed(&mut self, entries: Vec<Machine>) -> usize {
        let mut added = 0;
        for mut s in entries {
            s.watch = vec![routine::MODAL, routine::DEATH, sidekick::starquake::PLAY_INPUT];
            if self.seen.insert(key(&s)) { self.queue.push_back(s); added += 1; }
        }
        added
    }

    /// Explores from `entries` (machines standing in `room`), adding to what is known.
    pub fn explore(&mut self, room: u16, entries: Vec<Machine>, p: Platforms) {
        self.seed(entries);
        self.explore_until(room, p, None);
    }

    /// Explores until an exit into `until` is newly found (returns true) or nothing is left.
    pub fn explore_until(&mut self, room: u16, p: Platforms, until: Option<u16>) -> bool {
        while let Some(m) = self.queue.pop_front() {
            let mut found = false;
            let mut reads = true;
            let inputs: &[u8] = if std::env::var("NO_DIAG").is_ok() { &INPUTS[..5] } else { &INPUTS };
            let hold: usize = std::env::var("HOLD").ok().and_then(|h| h.parse().ok()).unwrap_or(1);
            for &inp in inputs {
                if !reads { break; }
                let mut n = m.clone();
                n.zx.release_all_keys();
                n.zx.kempston = inp;
                prepare(&mut n, p);
                let mut hits = n.run_frame();
                self.frames += 1;
                for _ in 1..hold {
                    if n.zx.read16(at::ROOM) != room || hits.contains(&routine::DEATH) || hits.contains(&routine::MODAL) { break; }
                    n.zx.release_all_keys(); n.zx.kempston = inp; prepare(&mut n, p);
                    hits.extend(n.run_frame()); self.frames += 1;
                }
                if inp == 0 && !hits.contains(&sidekick::starquake::PLAY_INPUT) { reads = false; }
                if hits.contains(&routine::DEATH) { self.deaths += 1; continue; }
                if hits.contains(&routine::MODAL) { self.modal.insert((m.zx.mem[E + 5], m.zx.mem[E + 6])); continue; }
                let now = n.zx.read16(at::ROOM);
                if now != room {
                    let (settled, f) = settle(&mut n, p);
                    self.frames += f;
                    if settled && n.zx.read16(at::ROOM) == now {
                        let k = (now, n.zx.mem[E + 5], n.zx.mem[E + 6]);
                        if self.exit_keys.insert(k) {
                            self.exit_path.insert(k, (key(&m), inp));
                            self.exits.insert(k, n);
                            if until == Some(now) { found = true; }
                        }
                    }
                    continue;
                }
                if std::env::var("PRUNE").is_ok() && in_wall(&n) { self.pruned += 1; continue; }
                let nk = key(&n);
                if !self.seen.contains(&nk) {
                    self.parent.insert(nk.clone(), (key(&m), inp));
                    self.seen.insert(nk);
                    self.queue.push_back(n);
                }
            }
            if found { return true; }
        }
        false
    }
}
