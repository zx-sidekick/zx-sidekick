//! The "verify only the route" idea from the level 5 spike, measured and
//! found wanting (#10): plan over the map's openings, search only the
//! route's rooms, each until it reaches the next; back up when a room runs
//! out, drop the edge and plan again when the route does. Targets come from
//! `exits.txt`: rooms the whole-map run reached, and rooms it did not that
//! the openings join to the start.
//!
//! `route <assets-dir> [cold|warm] [threads] [seconds]`

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use sidekick::Machine;
use sidekick::map::Openings;
use sidekick::starquake::{CORE_ROOM, PLAY_INPUT, routine};
use sk_lab::exits;
use sk_lab::search::{Exit, Room, Settings};
use sk_lab::{Args, Rng, into_play};

fn neighbours(o: &[Openings], r: u16) -> Vec<u16> {
    let x = &o[usize::from(r)];
    let mut v = Vec::new();
    if x.right && r % 16 != 15 {
        v.push(r + 1);
    }
    if x.left && !r.is_multiple_of(16) {
        v.push(r - 1);
    }
    if x.down && r + 16 < 512 {
        v.push(r + 16);
    }
    if x.up && r >= 16 {
        v.push(r - 16);
    }
    v
}

fn plan(o: &[Openings], from: u16, to: u16, failed: &HashSet<(u16, u16)>) -> Option<Vec<u16>> {
    let mut prev: HashMap<u16, u16> = HashMap::new();
    let mut queue = VecDeque::from([from]);
    let mut seen = HashSet::from([from]);
    while let Some(r) = queue.pop_front() {
        if r == to {
            let mut path = vec![to];
            while let Some(&p) = prev.get(path.last().unwrap_or(&to)) {
                path.push(p);
            }
            path.reverse();
            return Some(path);
        }
        for n in neighbours(o, r) {
            if (n == CORE_ROOM && n != to) || failed.contains(&(r, n)) || !seen.insert(n) {
                continue;
            }
            prev.insert(n, r);
            queue.push_back(n);
        }
    }
    None
}

struct Outcome {
    found: bool,
    timeout: bool,
    secs: f64,
    frames: u64,
    rooms: usize,
    replans: usize,
    route_len: usize,
    replay: Option<bool>,
}

/// Hands every exit of `from` into `to` not yet handed on to `to`'s search.
fn forward(
    rooms: &mut HashMap<u16, Room>,
    handed: &mut HashSet<(u16, Exit)>,
    from: u16,
    to: u16,
    settings: &Settings,
) -> usize {
    let found: Vec<(Exit, Machine)> = rooms
        .get(&from)
        .map(|r| {
            r.exits
                .iter()
                .filter(|(k, _)| k.0 == to && !handed.contains(&(from, **k)))
                .map(|(k, m)| (*k, m.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut added = 0;
    for (k, m) in found {
        handed.insert((from, k));
        let r = rooms.entry(to).or_default();
        let key = settings.key(&m);
        if r.seed(vec![m], settings) > 0 {
            r.origin.insert(key, (from, k));
            added += 1;
        }
    }
    added
}

fn replay(base: &Machine, rooms: &HashMap<u16, Room>, target: u16, settings: &Settings) -> bool {
    let Some(mut key) = rooms[&target].origin.keys().next().cloned() else {
        return false;
    };
    let mut room = target;
    let mut segments: Vec<Vec<u8>> = Vec::new();
    while let Some(&(prev, exit)) = rooms[&room].origin.get(&key) {
        let r = &rooms[&prev];
        let (parent, input) = r.exit_path[&exit].clone();
        let (seed, mut inputs) = r.path(parent);
        inputs.push(input);
        segments.push(inputs);
        key = seed;
        room = prev;
    }
    segments.reverse();
    let mut m = base.clone();
    m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
    for segment in segments {
        let here = sk_lab::room(&m);
        for (i, &input) in segment.iter().enumerate() {
            sk_lab::frame(&mut m, input, settings.platforms);
            let moved = sk_lab::room(&m) != here;
            if moved != (i + 1 == segment.len()) {
                return false;
            }
        }
        sk_lab::settle(&mut m, settings.platforms);
    }
    sk_lab::room(&m) == target
}

fn route_to(
    base: &Machine,
    o: &[Openings],
    target: u16,
    rooms: &mut HashMap<u16, Room>,
    failed: &mut HashSet<(u16, u16)>,
    cap: Duration,
    settings: &Settings,
) -> Outcome {
    let started = Instant::now();
    let start = sk_lab::room(base);
    let frames_before: u64 = rooms.values().map(|r| r.frames).sum();
    if rooms.get(&start).is_none_or(|r| r.seen.is_empty()) {
        rooms
            .entry(start)
            .or_default()
            .seed(vec![base.clone()], settings);
    }
    let mut handed = HashSet::new();
    let (mut replans, mut route_len, mut found, mut timeout) = (0, 0, false, false);
    'plan: while let Some(route) = plan(o, start, target, failed) {
        route_len = route.len();
        if rooms.get(&target).is_some_and(|r| !r.origin.is_empty()) {
            found = true;
            break;
        }
        let mut i = 0usize;
        let mut deepest = 0usize;
        loop {
            if started.elapsed() > cap {
                timeout = true;
                break 'plan;
            }
            if i + 1 == route.len() {
                found = true;
                break 'plan;
            }
            let (room, next) = (route[i], route[i + 1]);
            if forward(rooms, &mut handed, room, next, settings) > 0 {
                i += 1;
                deepest = deepest.max(i);
                continue;
            }
            let r = rooms.entry(room).or_default();
            if r.explore_until(room, settings, Some(next)) {
                forward(rooms, &mut handed, room, next, settings);
                i += 1;
                deepest = deepest.max(i);
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
    let frames = rooms.values().map(|r| r.frames).sum::<u64>() - frames_before;
    let replayed = found.then(|| replay(base, rooms, target, settings));
    Outcome {
        found,
        timeout,
        secs: started.elapsed().as_secs_f64(),
        frames,
        rooms: rooms.len(),
        replans,
        route_len,
        replay: replayed,
    }
}

fn summary(label: &str, outcomes: &[&Outcome]) {
    let mut v: Vec<&Outcome> = outcomes.to_vec();
    v.sort_by(|a, b| a.secs.total_cmp(&b.secs));
    if v.is_empty() {
        return;
    }
    let found = v.iter().filter(|x| x.found).count();
    let timeouts = v.iter().filter(|x| x.timeout).count();
    let replay_ok = v.iter().filter(|x| x.replay == Some(true)).count();
    let mut searched: Vec<usize> = v.iter().map(|x| x.rooms).collect();
    searched.sort_unstable();
    println!(
        "{label}: {} targets; found {found}, timeouts {timeouts}, replay ok {replay_ok}/{found}; seconds median {:.1}, 90% {:.1}, max {:.1}; rooms searched median {}",
        v.len(),
        v[v.len() / 2].secs,
        v[v.len() * 9 / 10].secs,
        v[v.len() - 1].secs,
        searched[searched.len() / 2]
    );
}

fn main() {
    let args = Args::parse("route <assets-dir> [cold|warm] [threads] [seconds]");
    let mode: String = args.get(0, "cold".to_string());
    let threads: usize = args.get(1, 8);
    let cap = Duration::from_secs(args.get(2, 600));
    let settings = Settings::from_args(&args);
    let text = std::fs::read_to_string(args.path(exits::FILE)).unwrap_or_else(|e| {
        eprintln!(
            "cannot read {}: {e} (run whole first)",
            args.path(exits::FILE).display()
        );
        std::process::exit(2);
    });
    let reachable: HashSet<u16> = exits::read(&text).keys().copied().collect();
    let base = into_play(&args.tape());
    let o = sidekick::starquake::all_openings(&base);
    let start = sk_lab::room(&base);
    let mut rng = Rng(0x5EED);
    let mut yes: Vec<u16> = reachable.iter().copied().filter(|&r| r != start).collect();
    yes.sort_unstable();
    let mut no: Vec<u16> = (0..512u16)
        .filter(|r| !reachable.contains(r) && plan(&o, start, *r, &HashSet::new()).is_some())
        .collect();
    let mut pick = |v: &mut Vec<u16>, n: usize| {
        let mut out = Vec::new();
        while out.len() < n && !v.is_empty() {
            out.push(v.remove(rng.below(v.len() as u64) as usize));
        }
        out
    };
    let targets_yes = pick(&mut yes, 30);
    let targets_no = pick(&mut no, 10);
    println!(
        "start {start}; {} rooms reached by the whole-map run, {} more joined to the start by openings",
        reachable.len(),
        no.len() + targets_no.len()
    );
    if mode == "warm" {
        // One thread, one cache kept across targets, as a data directory would.
        let mut rooms = HashMap::new();
        let mut failed = HashSet::new();
        for (n, &t) in targets_yes.iter().enumerate() {
            let out = route_to(&base, &o, t, &mut rooms, &mut failed, cap, &settings);
            println!(
                "warm #{n:2} target {t:3}: found {} in {:.1}s, {} frames, {} rooms known, {} replans, replay {:?}",
                out.found, out.secs, out.frames, out.rooms, out.replans, out.replay
            );
        }
        return;
    }
    let jobs = Mutex::new(
        targets_yes
            .iter()
            .map(|&t| (t, true))
            .chain(targets_no.iter().map(|&t| (t, false)))
            .collect::<Vec<_>>(),
    );
    let results = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let Some((t, truth)) = jobs.lock().unwrap().pop() else {
                        break;
                    };
                    let mut rooms = HashMap::new();
                    let mut failed = HashSet::new();
                    let out = route_to(&base, &o, t, &mut rooms, &mut failed, cap, &settings);
                    println!(
                        "target {t:3} (reachable: {truth}): found {} timeout {} in {:.1}s, {} frames, {} rooms searched, {} replans, planned route {} rooms, replay {:?}",
                        out.found, out.timeout, out.secs, out.frames, out.rooms, out.replans, out.route_len, out.replay
                    );
                    results.lock().unwrap().push((truth, out));
                }
            });
        }
    });
    let results = results.into_inner().unwrap();
    for truth in [true, false] {
        let v: Vec<&Outcome> = results
            .iter()
            .filter(|x| x.0 == truth)
            .map(|x| &x.1)
            .collect();
        summary(&format!("reachable={truth}"), &v);
    }
}
