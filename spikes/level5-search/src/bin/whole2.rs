#[path = "../main.rs"] #[allow(dead_code)] mod app;
use app::search::{self, Platforms};
use sidekick::Machine;
use sidekick::starquake::{all_openings, at};
use std::collections::{HashMap, HashSet};
use std::sync::{Condvar, Mutex};

#[derive(Default)]
struct State {
    rooms: HashMap<u16, search::Room>,
    pending: HashMap<u16, Vec<Machine>>,
    busy: HashSet<u16>,
    frames: u64,
    searches: usize,
    room_secs: HashMap<u16, f64>,
}

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let threads: usize = std::env::args().nth(2).and_then(|t| t.parse().ok()).unwrap_or(11);
    let minutes: u64 = std::env::args().nth(3).and_then(|t| t.parse().ok()).unwrap_or(30);
    let p = Platforms::Unlimited;
    let base = app::into_play(&tape);
    let openings = all_openings(&base);
    let t0 = std::time::Instant::now();
    let st = Mutex::new(State::default());
    st.lock().unwrap().pending.insert(base.zx.read16(at::ROOM), vec![base.clone()]);
    let cv = Condvar::new();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let report = |s: &State, label: &str| {
        let (mut open, mut crossed, mut closed) = (0, 0, 0);
        let mut pairs = HashSet::new();
        for (&room, r) in &s.rooms {
            if s.busy.contains(&room) { continue; }
            let o = &openings[usize::from(room)];
            let to: HashSet<u16> = r.exit_keys.iter().map(|k| k.0).collect();
            for (d, op) in [(1i32, o.right), (-1, o.left), (16, o.down), (-16, o.up)] {
                let t = (i32::from(room) + d).rem_euclid(512) as u16;
                if op { open += 1; }
                if to.contains(&t) { crossed += 1; pairs.insert((room, t)); if !op { closed += 1; } }
            }
        }
        let other: usize = s.rooms.values().map(|r| r.exit_keys.iter().filter(|k| { let d = i32::from(k.0); true && ![1, -1, 16, -16].iter().any(|_| d >= 0) }).count()).sum();
        let rss = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        let states: usize = s.rooms.values().map(|r| r.seen.len()).sum();
        let pruned: usize = s.rooms.values().map(|r| r.pruned).sum();
        let doors: usize = s.rooms.values().filter(|r| !r.modal.is_empty()).count();
        println!("{label} {:.0?}: {} rooms known, {} pending, {} busy; {} searches, {} frames, {states} states; edges out of known rooms: {open} open by the map, {crossed} crossed ({closed} through a closed edge); rooms with a door/booth screen {doors}; rss {rss} KB, pruned {pruned}{}",
            t0.elapsed(), s.rooms.len(), s.pending.len(), s.busy.len(), s.searches, s.frames, if other > 0 { "" } else { "" });
    };
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let job = {
                    let mut s = st.lock().unwrap();
                    loop {
                        if stop.load(std::sync::atomic::Ordering::Relaxed) { break None; }
                        let pick = s.pending.keys().copied().filter(|r| !s.busy.contains(r)).min();
                        if let Some(room) = pick {
                            let entries = s.pending.remove(&room).unwrap();
                            let r = s.rooms.remove(&room).unwrap_or_default();
                            s.busy.insert(room);
                            break Some((room, r, entries));
                        }
                        if s.busy.is_empty() && s.pending.is_empty() { break None; }
                        s = cv.wait(s).unwrap();
                    }
                };
                let Some((room, mut r, entries)) = job else { cv.notify_all(); break };
                let f0 = r.frames;
                let t = std::time::Instant::now();
                r.explore(room, entries, p);
                let new: Vec<Machine> = r.exits.drain().map(|(_, m)| m).collect();
                let mut s = st.lock().unwrap();
                s.frames += r.frames - f0; s.searches += 1;
                *s.room_secs.entry(room).or_default() += t.elapsed().as_secs_f64();
                s.rooms.insert(room, r);
                s.busy.remove(&room);
                for m in new {
                    let to = m.zx.read16(at::ROOM);
                    if to >= 512 { continue; }
                    let known = s.rooms.get(&to).is_some_and(|x| x.seen.contains(&search::key(&m)));
                    if !known { s.pending.entry(to).or_default().push(m); }
                }
                cv.notify_all();
            });
        }
        sc.spawn(|| {
            let mut last = std::time::Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let s = st.lock().unwrap();
                let finished = s.busy.is_empty() && s.pending.is_empty();
                if last.elapsed().as_secs() >= 60 || finished { report(&s, "at"); last = std::time::Instant::now(); }
                if finished { break; }
                if t0.elapsed().as_secs() > minutes * 60 { stop.store(true, std::sync::atomic::Ordering::Relaxed); drop(s); cv.notify_all(); break; }
            }
        });
    });
    let s = st.into_inner().unwrap();
    report(&s, "END");
    let mut times: Vec<f64> = s.room_secs.values().copied().collect(); times.sort_by(f64::total_cmp);
    if !times.is_empty() { println!("per room seconds (all searches of it): median {:.1}, 90% {:.1}, max {:.1}", times[times.len() / 2], times[times.len() * 9 / 10], times[times.len() - 1]); }
    let mut dump = String::new();
    for (&room, r) in &s.rooms {
        let o = &openings[usize::from(room)];
        dump += &format!("room {room} open L{} R{} U{} D{} walls {:?} modal {:?}\n", o.left as u8, o.right as u8, o.up as u8, o.down as u8, o.walls.iter().flatten().count(), r.modal.iter().take(3).collect::<Vec<_>>());
        let mut ex: Vec<_> = r.exit_keys.iter().collect(); ex.sort();
        for (to, x, y) in ex { dump += &format!("  exit {room} -> {to} at ({x},{y})\n"); }
    }
    std::fs::write("exits.txt", dump).unwrap();
    let reached: HashSet<u16> = s.rooms.keys().copied().collect();
    println!("rooms reached: {}", reached.len());
    println!("reached list: {:?}", { let mut v: Vec<_> = reached.into_iter().collect(); v.sort(); v });
}
