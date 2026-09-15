//! Option 1 from the spike report: plan over the map's openings, then search
//! only the rooms on the route, stopping each room as soon as it reaches the
//! next, backing up or replanning when a room runs out.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use app::search::{self, Platforms, Room};
use sidekick::Machine;
use sidekick::map::Openings;
use sidekick::starquake::{all_openings, at, CORE_ROOM};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

const P: Platforms = Platforms::Unlimited;

fn neighbours(o: &[Openings], r: u16) -> Vec<u16> {
    let x = &o[usize::from(r)];
    let mut v = Vec::new();
    if x.right && r % 16 != 15 { v.push(r + 1); }
    if x.left && r % 16 != 0 { v.push(r - 1); }
    if x.down && r + 16 < 512 { v.push(r + 16); }
    if x.up && r >= 16 { v.push(r - 16); }
    v
}

fn plan(o: &[Openings], from: u16, to: u16, failed: &HashSet<(u16, u16)>) -> Option<Vec<u16>> {
    let mut prev: HashMap<u16, u16> = HashMap::new();
    let mut q = VecDeque::from([from]);
    let mut seen = HashSet::from([from]);
    while let Some(r) = q.pop_front() {
        if r == to {
            let mut path = vec![to];
            while let Some(&p) = prev.get(path.last().unwrap()) { path.push(p); }
            path.reverse();
            return Some(path);
        }
        for n in neighbours(o, r) {
            if (n == CORE_ROOM && n != to) || failed.contains(&(r, n)) || !seen.insert(n) { continue; }
            prev.insert(n, r);
            q.push_back(n);
        }
    }
    None
}

struct Outcome { found: bool, timeout: bool, secs: f64, frames: u64, rooms: usize, replans: usize, route_len: usize, replay: Option<bool> }

/// Moves every exit of `from` into `to` not yet handed on into `to`'s search.
fn forward(rooms: &mut HashMap<u16, Room>, handed: &mut HashSet<(u16, (u16, u8, u8))>, from: u16, to: u16) -> usize {
    let exits: Vec<((u16, u8, u8), Machine)> = rooms.get(&from).map(|r| r.exits.iter().filter(|(k, _)| k.0 == to && !handed.contains(&(from, **k))).map(|(k, m)| (*k, m.clone())).collect()).unwrap_or_default();
    let mut added = 0;
    for (k, m) in exits {
        handed.insert((from, k));
        let r = rooms.entry(to).or_default();
        let mk = search::key(&m);
        if r.seed(vec![m]) > 0 { r.origin.insert(mk, (from, k)); added += 1; }
    }
    added
}

fn replay(base: &Machine, rooms: &HashMap<u16, Room>, target: u16) -> bool {
    let Some((mut key, _)) = rooms[&target].origin.iter().next().map(|(k, v)| (k.clone(), *v)) else { return false };
    let mut room = target;
    let mut segments: Vec<Vec<u8>> = Vec::new();
    while let Some(&(prev, exit)) = rooms[&room].origin.get(&key) {
        let r = &rooms[&prev];
        let (pk, inp) = r.exit_path[&exit].clone();
        let (seed, mut inputs) = r.path(pk);
        inputs.push(inp);
        segments.push(inputs);
        key = seed;
        room = prev;
    }
    segments.reverse();
    let mut m = base.clone();
    m.watch = vec![sidekick::starquake::routine::MODAL, sidekick::starquake::routine::DEATH, sidekick::starquake::PLAY_INPUT];
    for seg in segments {
        let here = m.zx.read16(at::ROOM);
        for (i, &inp) in seg.iter().enumerate() {
            search::frame(&mut m, inp, P);
            let moved = m.zx.read16(at::ROOM) != here;
            if moved != (i + 1 == seg.len()) { return false; }
        }
        search::settle(&mut m, P);
    }
    m.zx.read16(at::ROOM) == target
}

fn route_to(base: &Machine, o: &[Openings], target: u16, rooms: &mut HashMap<u16, Room>, failed: &mut HashSet<(u16, u16)>, cap: Duration) -> Outcome {
    let t0 = Instant::now();
    let start = base.zx.read16(at::ROOM);
    let f0: u64 = rooms.values().map(|r| r.frames).sum();
    if rooms.get(&start).is_none_or(|r| r.seen.is_empty()) { rooms.entry(start).or_default().seed(vec![base.clone()]); }
    let mut handed = HashSet::new();
    let (mut replans, mut route_len, mut found, mut timeout) = (0, 0, false, false);
    'plan: while let Some(route) = plan(o, start, target, failed) {
        route_len = route.len();
        if rooms.get(&target).is_some_and(|r| !r.origin.is_empty()) { found = true; break; }
        let mut i = 0usize;
        let mut deepest = 0usize;
        loop {
            if t0.elapsed() > cap { timeout = true; break 'plan; }
            if i + 1 == route.len() { found = true; break 'plan; }
            let (room, next) = (route[i], route[i + 1]);
            if forward(rooms, &mut handed, room, next) > 0 { i += 1; deepest = deepest.max(i); continue; }
            let r = rooms.entry(room).or_default();
            if r.explore_until(room, P, Some(next)) {
                forward(rooms, &mut handed, room, next);
                i += 1; deepest = deepest.max(i);
                continue;
            }
            if i == 0 {
                // Nothing reachable along this route gets past route[deepest].
                failed.insert((route[deepest], route[deepest + 1]));
                replans += 1;
                continue 'plan;
            }
            i -= 1;
        }
    }
    let frames = rooms.values().map(|r| r.frames).sum::<u64>() - f0;
    let replay = found.then(|| replay(base, rooms, target));
    Outcome { found, timeout, secs: t0.elapsed().as_secs_f64(), frames, rooms: rooms.len(), replans, route_len, replay }
}

fn ground() -> HashSet<u16> {
    std::fs::read_to_string("exits-ground.txt").unwrap().lines().filter_map(|l| l.strip_prefix("room ")).filter_map(|l| l.split(' ').next()?.parse().ok()).collect()
}

struct Rng(u64);
impl Rng { fn next(&mut self, n: u64) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 % n } }

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mode = std::env::args().nth(2).unwrap_or("cold".into());
    let threads: usize = std::env::args().nth(3).and_then(|t| t.parse().ok()).unwrap_or(8);
    let cap = Duration::from_secs(std::env::args().nth(4).and_then(|t| t.parse().ok()).unwrap_or(600));
    let base = app::into_play(&tape);
    let o = all_openings(&base);
    let start = base.zx.read16(at::ROOM);
    let reachable = ground();
    let mut rng = Rng(0x5EED);
    let mut yes: Vec<u16> = reachable.iter().copied().filter(|&r| r != start).collect(); yes.sort();
    let mut no: Vec<u16> = (0..512u16).filter(|r| !reachable.contains(r) && plan(&o, start, *r, &HashSet::new()).is_some()).collect();
    let pick = |v: &mut Vec<u16>, n: usize, rng: &mut Rng| { let mut out = Vec::new(); while out.len() < n && !v.is_empty() { out.push(v.remove(rng.next(v.len() as u64) as usize)); } out };
    let targets_yes = pick(&mut yes, 30, &mut rng);
    let targets_no = pick(&mut no, 10, &mut rng);
    println!("start {start}; {} rooms reachable by the whole-map run, {} more joined to the start by openings", reachable.len(), no.len() + targets_no.len());
    match mode.as_str() {
        "cold" => {
            let jobs = std::sync::Mutex::new(targets_yes.iter().map(|&t| (t, true)).chain(targets_no.iter().map(|&t| (t, false))).collect::<Vec<_>>());
            let results = std::sync::Mutex::new(Vec::new());
            std::thread::scope(|s| for _ in 0..threads { s.spawn(|| loop {
                let Some((t, truth)) = jobs.lock().unwrap().pop() else { break };
                let mut rooms = HashMap::new();
                let mut failed = HashSet::new();
                let out = route_to(&base, &o, t, &mut rooms, &mut failed, cap);
                println!("target {t:3} (reachable: {truth}): found {} timeout {} in {:.1}s, {} frames, {} rooms searched, {} replans, planned route {} rooms, replay {:?}", out.found, out.timeout, out.secs, out.frames, out.rooms, out.replans, out.route_len, out.replay);
                results.lock().unwrap().push((truth, out));
            }); });
            let r = results.into_inner().unwrap();
            for truth in [true, false] {
                let mut v: Vec<&Outcome> = r.iter().filter(|x| x.0 == truth).map(|x| &x.1).collect();
                v.sort_by(|a, b| a.secs.total_cmp(&b.secs));
                if v.is_empty() { continue; }
                let found = v.iter().filter(|x| x.found).count();
                let timeouts = v.iter().filter(|x| x.timeout).count();
                let replay_ok = v.iter().filter(|x| x.replay == Some(true)).count();
                println!("SUMMARY reachable={truth}: {} targets; found {found}, timeouts {timeouts}, replay ok {replay_ok}/{found}; seconds median {:.1}, 90% {:.1}, max {:.1}; rooms searched median {}",
                    v.len(), v[v.len() / 2].secs, v[v.len() * 9 / 10].secs, v[v.len() - 1].secs, { let mut n: Vec<usize> = v.iter().map(|x| x.rooms).collect(); n.sort(); n[n.len() / 2] });
            }
        }
        _ => {
            // Warm: one thread, one cache kept across targets, as the data directory would.
            let mut rooms = HashMap::new();
            let mut failed = HashSet::new();
            for (n, &t) in targets_yes.iter().enumerate() {
                let out = route_to(&base, &o, t, &mut rooms, &mut failed, cap);
                println!("warm #{n:2} target {t:3}: found {} in {:.1}s, {} frames, {} rooms known, {} replans, replay {:?}", out.found, out.secs, out.frames, out.rooms, out.replans, out.replay);
            }
        }
    }
}
