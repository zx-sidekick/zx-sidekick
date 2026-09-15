pub mod search;
use sidekick::Machine;
use sidekick::starquake::{CORE_ROOM, ENTRY_PC, ENTRY_SP, at, routine};
struct Rng(u64);
impl Rng { fn next(&mut self, n: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 % n } }
pub fn into_play(tape: &[u8]) -> Machine {
    let mut base = Machine::from_tape(tape, ENTRY_PC, ENTRY_SP).unwrap();
    let key = |n: &str| zx_spectrum::Key::by_name(n).unwrap();
    base.watch = vec![routine::MAIN_LOOP];
    for f in 0..600u64 {
        base.zx.release_all_keys();
        match f { 50..=54 => base.zx.set_key(key("1"), true), 100..=104 => base.zx.set_key(key("0"), true), 330..=334 => base.zx.set_key(key("enter"), true), _ => {} }
        if base.run_frame().contains(&routine::MAIN_LOOP) && f > 400 { break; }
    }
    base
}
fn main() {
    use std::collections::{HashMap, HashSet, VecDeque};
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let budget: usize = std::env::args().nth(2).and_then(|b| b.parse().ok()).unwrap_or(12);
    let base = into_play(&tape);
    for p in [search::Platforms::None, search::Platforms::Unlimited] {
        let name = match p { search::Platforms::None => "no platforms", _ => "unlimited platforms" };
        let t0 = std::time::Instant::now();
        let mut rooms: HashMap<u16, search::Room> = HashMap::new();
        let mut pending: HashMap<u16, Vec<Machine>> = HashMap::new();
        let mut order = VecDeque::new();
        let start = base.zx.read16(at::ROOM);
        pending.insert(start, vec![base.clone()]); order.push_back(start);
        let mut searched = 0;
        let mut total_frames = 0u64;
        while let Some(room) = order.pop_front() {
            if searched >= budget { break; }
            let Some(entries) = pending.remove(&room) else { continue };
            let r = rooms.entry(room).or_default();
            let (s0, f0, e0) = (r.seen.len(), r.frames, r.exits.len());
            let t = std::time::Instant::now();
            r.explore(room, entries, p);
            searched += 1; total_frames += r.frames - f0;
            let targets: HashSet<u16> = r.exits.keys().map(|k| k.0).collect();
            println!("  room {room:3}: +{} states, +{} frames, {:.1?}; exits {} into {:?}; doors/booths at {:?}; deaths {}", r.seen.len() - s0, r.frames - f0, t.elapsed(), r.exits.len() - e0, targets, r.modal, r.deaths);
            let new: Vec<(u16, Machine)> = r.exits.iter().skip(e0).map(|(k, m)| (k.0, m.clone())).collect();
            // The new exits: seed their rooms (HashMap order is arbitrary; skip() is only a rough "new").
            for (to, m) in new {
                let known = rooms.get(&to).is_some_and(|x| x.seen.contains(&search::key(&m)));
                if !known { pending.entry(to).or_default().push(m); if !order.contains(&to) { order.push_back(to); } }
            }
        }
        println!("{name}: {searched} room searches, {total_frames} frames, {:.1?} ({:.0} frames/s)", t0.elapsed(), total_frames as f64 / t0.elapsed().as_secs_f64());
    }
}
