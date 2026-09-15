#[path = "../main.rs"] #[allow(dead_code)] mod app;
use app::search::{self, Platforms, E};
use sidekick::Machine;
use sidekick::starquake::{at, routine, PLAY_INPUT};
use std::collections::{HashMap, HashSet};
struct Rng(u64);
impl Rng { fn next(&mut self, n: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 % n } }

/// Replays `inputs` from `entry`; returns where Blob is once a room change settles, if one happens.
fn replay(entry: &Machine, inputs: &[u8], p: Platforms) -> Option<(u16, u8, u8)> {
    let mut m = entry.clone();
    m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
    let room = m.zx.read16(at::ROOM);
    for &i in inputs {
        search::frame(&mut m, i, p);
        if m.zx.read16(at::ROOM) != room {
            let now = m.zx.read16(at::ROOM);
            search::settle(&mut m, p);
            return Some((now, m.zx.mem[E + 5], m.zx.mem[E + 6]));
        }
    }
    None
}

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let walks: usize = std::env::args().nth(2).and_then(|w| w.parse().ok()).unwrap_or(100);
    let p = if std::env::args().nth(3).as_deref() == Some("platforms") { Platforms::Unlimited } else { Platforms::None };
    let base = app::into_play(&tape);
    // Entries: the start, and every exit the start room's search found (a room further on).
    let mut entries: Vec<Machine> = vec![base.clone()];
    let mut first = search::Room::default();
    first.explore(base.zx.read16(at::ROOM), vec![base.clone()], p);
    entries.extend(first.exits.values().cloned());
    let only: usize = std::env::args().nth(4).and_then(|n| n.parse().ok()).unwrap_or(6);
    let mut rng = Rng(0xC0FFEE);
    let (mut replayed, mut replay_ok, mut walk_exits, mut walk_missed) = (0, 0, 0, 0);
    for entry in entries.iter().take(only) {
        let room = entry.zx.read16(at::ROOM);
        let mut r = search::Room::default();
        let t = std::time::Instant::now();
        r.explore(room, vec![entry.clone()], p);
        let took = t.elapsed();
        // 1. Every exit the search found, replayed from the entry.
        let mut bad = Vec::new();
        for (k, (pk, inp)) in &r.exit_path {
            let (_, mut inputs) = r.path(pk.clone());
            inputs.push(*inp);
            replayed += 1;
            if replay(entry, &inputs, p) == Some(*k) { replay_ok += 1; } else { bad.push((*k, inputs.len())); }
        }
        // 2. Random walks from the entry: every exit they take should be one the search found.
        let found: HashSet<(u16, u8, u8)> = r.exit_keys.iter().copied().collect();
        let mut taken: HashMap<(u16, u8, u8), usize> = HashMap::new();
        for _ in 0..walks {
            let mut m = entry.clone();
            m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
            let mut dir = 0u8;
            for f in 0..1500 {
                if f % (5 + rng.next(20) as usize) == 0 { dir = search::INPUTS[rng.next(9) as usize]; }
                let hits = search::frame(&mut m, dir, p);
                if hits.contains(&routine::DEATH) || hits.contains(&routine::MODAL) { break; }
                let now = m.zx.read16(at::ROOM);
                if now != room {
                    search::settle(&mut m, p);
                    *taken.entry((now, m.zx.mem[E + 5], m.zx.mem[E + 6])).or_default() += 1;
                    break;
                }
            }
        }
        let missed: Vec<_> = taken.keys().filter(|k| !found.contains(k)).collect();
        walk_exits += taken.len(); walk_missed += missed.len();
        println!("room {room:3} from ({},{}): {} states, {:.1?}; search exits {}, replayed ok {}/{} {:?}; walks took {} distinct exits, missed by search {:?}",
            entry.zx.mem[E+5], entry.zx.mem[E+6], r.seen.len(), took, found.len(), r.exit_path.len() - bad.len(), r.exit_path.len(), bad, taken.len(), missed);
    }
    println!("replayed {replay_ok}/{replayed}; walk exits missed {walk_missed}/{walk_exits}");
}
