//! The whole map from the start: rooms searched as their entries are found,
//! on several threads, with a report every minute and `exits.txt` written
//! into the assets folder at the end. Doors and teleports stop the search, so
//! it covers the part of the planet reachable without them.
//!
//! `whole <assets-dir> [threads] [minutes] [--full] [--diagonals] [--no-prune] [--no-platforms] [--hold=N]`

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Instant;

use starquake::Machine;
use starquake_lab::exits::{self, Dump, Record};
use starquake_lab::rooms::Planet;
use starquake_lab::search::{Room, Settings};
use starquake_lab::{Args, into_play};

#[derive(Default)]
struct State {
    rooms: HashMap<u16, Room>,
    pending: HashMap<u16, Vec<Machine>>,
    busy: HashSet<u16>,
    frames: u64,
    searches: usize,
    seconds: HashMap<u16, f64>,
}

fn report(s: &State, planet: &Planet, started: Instant) {
    let (mut open, mut crossed, mut closed) = (0, 0, 0);
    for (&room, r) in &s.rooms {
        if s.busy.contains(&room) {
            continue;
        }
        let o = &planet.openings[usize::from(room)];
        let to: HashSet<u16> = r.exit_keys.iter().map(|k| k.0).collect();
        for (d, is_open) in [(1i32, o.right), (-1, o.left), (16, o.down), (-16, o.up)] {
            let t = (i32::from(room) + d).rem_euclid(512) as u16;
            if is_open {
                open += 1;
            }
            if to.contains(&t) {
                crossed += 1;
                if !is_open {
                    closed += 1;
                }
            }
        }
    }
    let states: usize = s.rooms.values().map(|r| r.seen.len()).sum();
    let pruned: usize = s.rooms.values().map(|r| r.pruned).sum();
    let modal = s.rooms.values().filter(|r| !r.modal.is_empty()).count();
    println!(
        "{:.0?}: {} rooms known, {} pending, {} busy; {} searches, {} frames, {states} states; edges out of known rooms: {open} open by the map, {crossed} crossed ({closed} through a closed edge); rooms with a door or booth screen {modal}; pruned {pruned}",
        started.elapsed(),
        s.rooms.len(),
        s.pending.len(),
        s.busy.len(),
        s.searches,
        s.frames
    );
}

fn main() {
    let args = Args::parse(
        "whole <assets-dir> [threads] [minutes] [--full] [--diagonals] [--no-prune] [--no-platforms] [--hold=N]",
    );
    let threads: usize = args.get(0, 4);
    let minutes: u64 = args.get(1, 30);
    let settings = Settings::from_args(&args);
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let started = Instant::now();
    let st = Mutex::new(State::default());
    st.lock()
        .unwrap()
        .pending
        .insert(starquake_lab::room(&base), vec![base.clone()]);
    let cv = Condvar::new();
    let stop = AtomicBool::new(false);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                loop {
                    let job = {
                        let mut s = st.lock().unwrap();
                        loop {
                            if stop.load(Ordering::Relaxed) {
                                break None;
                            }
                            let pick = s
                                .pending
                                .keys()
                                .copied()
                                .filter(|r| !s.busy.contains(r))
                                .min();
                            if let Some(room) = pick {
                                let entries = s.pending.remove(&room).unwrap_or_default();
                                let r = s.rooms.remove(&room).unwrap_or_default();
                                s.busy.insert(room);
                                break Some((room, r, entries));
                            }
                            if s.busy.is_empty() && s.pending.is_empty() {
                                break None;
                            }
                            s = cv.wait(s).unwrap();
                        }
                    };
                    let Some((room, mut r, entries)) = job else {
                        cv.notify_all();
                        break;
                    };
                    let before = r.frames;
                    let t = Instant::now();
                    r.explore(room, entries, &settings);
                    let new: Vec<Machine> = r.exits.drain().map(|(_, m)| m).collect();
                    let mut s = st.lock().unwrap();
                    s.frames += r.frames - before;
                    s.searches += 1;
                    *s.seconds.entry(room).or_default() += t.elapsed().as_secs_f64();
                    s.rooms.insert(room, r);
                    s.busy.remove(&room);
                    for m in new {
                        let to = starquake_lab::room(&m);
                        if to >= 512 {
                            continue;
                        }
                        let known = s
                            .rooms
                            .get(&to)
                            .is_some_and(|x| x.seen.contains(&settings.key(&m)));
                        if !known {
                            s.pending.entry(to).or_default().push(m);
                        }
                    }
                    cv.notify_all();
                }
            });
        }
        sc.spawn(|| {
            let mut last = Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let s = st.lock().unwrap();
                let finished = s.busy.is_empty() && s.pending.is_empty();
                if last.elapsed().as_secs() >= 60 || finished {
                    report(&s, &planet, started);
                    last = Instant::now();
                }
                if finished {
                    break;
                }
                if started.elapsed().as_secs() > minutes * 60 {
                    stop.store(true, Ordering::Relaxed);
                    drop(s);
                    cv.notify_all();
                    break;
                }
            }
        });
    });
    let s = st.into_inner().unwrap();
    report(&s, &planet, started);
    let mut times: Vec<f64> = s.seconds.values().copied().collect();
    times.sort_by(f64::total_cmp);
    if !times.is_empty() {
        println!(
            "seconds a room, all searches of it: median {:.1}, 90% {:.1}, max {:.1}",
            times[times.len() / 2],
            times[times.len() * 9 / 10],
            times[times.len() - 1]
        );
    }
    let mut dump = Dump::new();
    for (&room, r) in &s.rooms {
        dump.insert(
            room,
            Record {
                modal: !r.modal.is_empty(),
                exits: r.exit_keys.iter().copied().collect(),
            },
        );
    }
    let path = args.path(exits::FILE);
    std::fs::write(&path, exits::write(&dump)).unwrap_or_else(|e| {
        eprintln!("cannot write {}: {e}", path.display());
        std::process::exit(2);
    });
    let mut reached: Vec<u16> = s.rooms.keys().copied().collect();
    reached.sort_unstable();
    println!(
        "rooms reached: {} {reached:?}; written to {}",
        reached.len(),
        path.display()
    );
}
