//! Proves the level 5 graph by play (#10): every connection the (room, part)
//! graph offers from the start, with doors shut, tried on copies of the
//! machine from the ways Blob really gets into that part, and every real
//! crossing checked against the graph. Writes `proof.txt` and `proof.png`
//! into the assets folder.
//!
//! An edge is **proven** when the search crosses it from a state play really
//! reaches (a chain of searches from the start), **proven from the map's
//! entry** when only a search seeded where the graph says Blob comes in
//! crosses it, and **not proven** otherwise. A crossing into a part the
//! graph has no edge to is a **gap** in the graph.
//!
//! `proof <assets-dir> [threads] [--half]`

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::sync::{Condvar, Mutex};
use std::time::Instant;

use sidekick::Machine;
use sidekick::map::COLS;
use sidekick::starquake::CORE_ROOM;
use sk_lab::picture::{self, ROOM_H, ROOM_W};
use sk_lab::rooms::{Planet, lift};
use sk_lab::search::{self, FULL_KEY, Room, Settings};
use sk_lab::{Args, into_play, stand, top_row};

type Node = (u16, u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Dir {
    Left,
    Right,
    Up,
    Down,
    Passage,
}

/// A directed edge of the graph and where along the shared edge it joins:
/// rows for left and right, columns for up and down, screen coordinates.
#[derive(Clone, Debug)]
struct Edge {
    to: Node,
    dir: Dir,
    places: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    Proven,
    FromMapEntry,
    NotProven,
}

/// The graph's edges out of `node`, grouped by the part they lead to.
fn edges(planet: &Planet, (room, part): Node) -> Vec<Edge> {
    let r = &planet.rooms[usize::from(room)];
    let col = room % COLS;
    let mut by: BTreeMap<(Node, Dir), Vec<u8>> = BTreeMap::new();
    let mut join = |to: u16, p: u8, dir: Dir, place: u8| {
        if p != 0 {
            by.entry(((to, p), dir)).or_default().push(place);
        }
    };
    if col > 0 {
        let o = &planet.rooms[usize::from(room) - 1];
        for row in 6..23 {
            if r.shut.at(row, 0) == part {
                join(room - 1, o.shut.at(row, 30), Dir::Left, row);
            }
        }
    }
    if col < COLS - 1 {
        let o = &planet.rooms[usize::from(room) + 1];
        for row in 6..23 {
            if r.shut.at(row, 30) == part {
                join(room + 1, o.shut.at(row, 0), Dir::Right, row);
            }
        }
    }
    if room >= COLS {
        let o = &planet.rooms[usize::from(room - COLS)];
        for c in 0..31 {
            if r.shut.at(6, c) == part {
                join(room - COLS, o.shut.at(22, c), Dir::Up, c);
            }
        }
    }
    if room + COLS < 512 {
        let o = &planet.rooms[usize::from(room + COLS)];
        for c in 0..31 {
            if r.shut.at(22, c) == part {
                join(room + COLS, o.shut.at(6, c), Dir::Down, c);
            }
        }
    }
    if planet.passage_part(room) == Some(part) {
        for (to, ok) in [(room.wrapping_sub(1), col > 0), (room + 1, col < COLS - 1)] {
            if ok && let Some(p) = planet.passage_part(to) {
                join(to, p, Dir::Passage, 0);
            }
        }
    }
    by.into_iter()
        .map(|((to, dir), places)| Edge { to, dir, places })
        .collect()
}

/// Where Blob stands in the room an edge leads into, by the graph: one
/// place for each place along the edge.
fn entry_places(planet: &Planet, e: &Edge) -> Vec<(u8, u8)> {
    let y = |row: u8| 143 - 8 * (row - 6);
    match e.dir {
        Dir::Right => e.places.iter().map(|&r| (0, y(r))).collect(),
        Dir::Left => e.places.iter().map(|&r| (240, y(r))).collect(),
        Dir::Down => e.places.iter().map(|&c| (c * 8, 143)).collect(),
        Dir::Up => e.places.iter().map(|&c| (c * 8, 15)).collect(),
        Dir::Passage => {
            let r = &planet.rooms[usize::from(e.to.0)];
            let Some((row, col)) = r.passage else {
                return vec![];
            };
            (row.saturating_sub(1)..row + 3)
                .flat_map(|rr| (col.saturating_sub(2)..col + 4).map(move |c| (rr, c)))
                .filter(|&(rr, c)| r.shut.at(rr, c) == e.to.1)
                .take(1)
                .map(|(rr, c)| (c * 8, y(rr)))
                .collect()
        }
    }
}

/// The part Blob is in at (`x`, `y`) in `room`, looking a cell around when
/// he is between cells.
fn part_at(planet: &Planet, room: u16, x: u8, y: u8) -> u8 {
    let r = &planet.rooms[usize::from(room)];
    let (row, col) = (top_row(y), x >> 3);
    for (dr, dc) in [
        (0i16, 0i16),
        (0, 1),
        (1, 0),
        (1, 1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (1, -1),
    ] {
        let (rr, cc) = (i16::from(row) + dr, i16::from(col) + dc);
        if (6..23).contains(&rr) && (0..31).contains(&cc) {
            let p = r.shut.at(rr as u8, cc as u8);
            if p != 0 {
                return p;
            }
        }
    }
    0
}

/// Whether the cells either side of a vertical edge hold a lift.
fn lift_edge(planet: &Planet, from: u16, e: &Edge) -> bool {
    let (a_rows, b_rows) = match e.dir {
        Dir::Up => ([0usize, 1], [16usize, 17]),
        Dir::Down => ([16, 17], [0, 1]),
        _ => return false,
    };
    let a = &planet.cells[usize::from(from)];
    let b = &planet.cells[usize::from(e.to.0)];
    e.places.iter().any(|&c| {
        let c = usize::from(c);
        a_rows
            .iter()
            .chain(&[])
            .any(|&r| lift(a[r][c]) || lift(a[r][c + 1]))
            || b_rows.iter().any(|&r| lift(b[r][c]) || lift(b[r][c + 1]))
    })
}

/// One part's search: the machines it was seeded with, whether each seed
/// is assumed (from the map's entry rather than play), and the search.
#[derive(Default)]
struct Part {
    search: Room,
    seeds: HashMap<Vec<u8>, (Machine, bool)>,
    pending: Vec<(Machine, bool)>,
    searched: bool,
}

#[derive(Default)]
struct Shared {
    parts: HashMap<Node, Part>,
    busy: HashSet<Node>,
    frames: u64,
}

fn run(
    shared: &Mutex<Shared>,
    cv: &Condvar,
    settings: &Settings,
    threads: usize,
    planet: &Planet,
    propagate: bool,
) {
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let job = {
                        let mut sh = shared.lock().unwrap();
                        loop {
                            let pick = sh
                                .parts
                                .iter()
                                .filter(|(n, p)| !p.pending.is_empty() && !sh.busy.contains(n))
                                .map(|(n, _)| *n)
                                .min();
                            if let Some(node) = pick {
                                let mut part = sh.parts.remove(&node).unwrap();
                                let pending = std::mem::take(&mut part.pending);
                                sh.busy.insert(node);
                                break Some((node, part, pending));
                            }
                            if sh.busy.is_empty() {
                                break None;
                            }
                            sh = cv.wait(sh).unwrap();
                        }
                    };
                    let Some((node, mut part, pending)) = job else {
                        cv.notify_all();
                        break;
                    };
                    let f0 = part.search.frames;
                    let mut fresh = Vec::new();
                    for (m, assumed) in pending {
                        if let std::collections::hash_map::Entry::Vacant(slot) =
                            part.seeds.entry(settings.key(&m))
                        {
                            slot.insert((m.clone(), assumed));
                            fresh.push(m);
                        }
                    }
                    part.search.explore(node.0, fresh, settings);
                    part.searched = true;
                    let exits: Vec<(search::Exit, Machine)> = part.search.exits.drain().collect();
                    let mut sh = shared.lock().unwrap();
                    sh.frames += part.search.frames - f0;
                    for (exit, m) in exits.into_iter().filter(|_| propagate) {
                        let (to, x, y) = exit;
                        if to >= 512 {
                            continue;
                        }
                        let p = part_at(planet, to, x, y);
                        if p == 0 || to == CORE_ROOM {
                            continue;
                        }
                        let (pk, _) = part.search.exit_path[&exit].clone();
                        let (seed, _) = part.search.path(pk);
                        let assumed = part.seeds.get(&seed).is_none_or(|s| s.1);
                        sh.parts
                            .entry((to, p))
                            .or_default()
                            .pending
                            .push((m, assumed));
                    }
                    sh.parts.insert(node, part);
                    sh.busy.remove(&node);
                    cv.notify_all();
                }
            });
        }
    });
}

fn main() {
    let args = Args::parse("proof <assets-dir> [threads] [--half]");
    let threads: usize = args.get(0, 10);
    let t0 = Instant::now();
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let start = sk_lab::room(&base);
    let (bx, by) = sk_lab::blob(&base);
    let start_node = (
        start,
        planet.rooms[usize::from(start)]
            .shut
            .at(top_row(by), bx >> 3),
    );

    // The graph from the start.
    let mut graph: BTreeMap<Node, Vec<Edge>> = BTreeMap::new();
    let mut queue = VecDeque::from([start_node]);
    while let Some(n) = queue.pop_front() {
        if graph.contains_key(&n) {
            continue;
        }
        let es = if n.0 == CORE_ROOM {
            vec![]
        } else {
            edges(&planet, n)
        };
        for e in &es {
            if !graph.contains_key(&e.to) {
                queue.push_back(e.to);
            }
        }
        graph.insert(n, es);
    }
    let edge_count: usize = graph.values().map(Vec::len).sum();
    let rooms: HashSet<u16> = graph.keys().map(|n| n.0).collect();
    eprintln!(
        "graph from the start, doors shut: {} parts in {} rooms, {edge_count} edges",
        graph.len(),
        rooms.len()
    );

    let shared = Mutex::new(Shared::default());
    let cv = Condvar::new();
    let coarse = Settings::default();
    shared
        .lock()
        .unwrap()
        .parts
        .entry(start_node)
        .or_default()
        .pending
        .push((base.clone(), false));

    // Play first; then parts the graph has that play never reached, entered
    // where the graph says, until every part has been searched.
    let mut rounds = 0;
    loop {
        run(&shared, &cv, &coarse, threads, &planet, true);
        rounds += 1;
        let mut sh = shared.lock().unwrap();
        let unsearched: Vec<Node> = graph
            .keys()
            .filter(|n| sh.parts.get(n).is_none_or(|p| !p.searched))
            .copied()
            .collect();
        eprintln!(
            "round {rounds}: {} parts searched, {} not yet; {:.0?}",
            sh.parts.values().filter(|p| p.searched).count(),
            unsearched.len(),
            t0.elapsed()
        );
        if unsearched.is_empty() {
            break;
        }
        let mut added = 0;
        for (from, es) in &graph {
            for e in es.iter().filter(|e| unsearched.contains(&e.to)) {
                let _ = from;
                for (x, y) in entry_places(&planet, e) {
                    if let Some(m) = stand(&base, e.to.0, x, y)
                        && part_at(&planet, e.to.0, sk_lab::blob(&m).0, sk_lab::blob(&m).1)
                            == e.to.1
                    {
                        sh.parts.entry(e.to).or_default().pending.push((m, true));
                        added += 1;
                    }
                }
            }
        }
        // A part nothing could be stood in is marked searched, so the loop ends.
        for n in &unsearched {
            let p = sh.parts.entry(*n).or_default();
            if p.pending.is_empty() {
                p.searched = true;
            }
        }
        if added == 0 {
            break;
        }
    }

    // What the searches crossed, by part.
    let classify = |sh: &Shared, planet: &Planet| {
        let mut crossed: HashMap<(Node, Node), bool> = HashMap::new(); // (from, to) -> assumed only
        for (node, part) in &sh.parts {
            for exit in &part.search.exit_keys {
                let (to, x, y) = *exit;
                if to >= 512 {
                    continue;
                }
                let p = part_at(planet, to, x, y);
                let (pk, _) = part.search.exit_path[exit].clone();
                let (seed, _) = part.search.path(pk);
                let assumed = part.seeds.get(&seed).is_none_or(|s| s.1);
                let e = crossed.entry((*node, (to, p))).or_insert(true);
                *e = *e && assumed;
            }
        }
        crossed
    };

    // Second pass: the parts with an edge not crossed, searched again with
    // the full key from every seed they had.
    let full = Settings {
        key: FULL_KEY.to_vec(),
        ..Settings::default()
    };
    {
        let mut sh = shared.lock().unwrap();
        let crossed = classify(&sh, &planet);
        let redo: Vec<Node> = graph
            .iter()
            .filter(|(n, es)| es.iter().any(|e| !crossed.contains_key(&(**n, e.to))))
            .map(|(n, _)| *n)
            .collect();
        eprintln!(
            "second pass, full key: {} parts with an edge not crossed",
            redo.len()
        );
        for n in redo {
            if let Some(p) = sh.parts.get_mut(&n) {
                let seeds: Vec<(Machine, bool)> = p.seeds.values().cloned().collect();
                p.search = Room::default();
                p.seeds.clear();
                p.pending = seeds;
            }
        }
    }
    run(&shared, &cv, &full, threads, &planet, false);
    let sh = shared.into_inner().unwrap();
    let crossed = classify(&sh, &planet);

    // The verdicts.
    let mut out = String::new();
    let mut counts: BTreeMap<(Class, bool), usize> = BTreeMap::new();
    let mut marks: Vec<(u16, Edge, Class)> = Vec::new();
    for (from, es) in &graph {
        for e in es {
            let class = match crossed.get(&(*from, e.to)) {
                Some(false) => Class::Proven,
                Some(true) => Class::FromMapEntry,
                None => Class::NotProven,
            };
            let is_lift = lift_edge(&planet, from.0, e);
            *counts.entry((class, is_lift)).or_default() += 1;
            if class != Class::Proven {
                let _ = writeln!(
                    out,
                    "{class:?}: {}:{} -> {}:{} {:?} at {:?}{}",
                    from.0,
                    from.1,
                    e.to.0,
                    e.to.1,
                    e.dir,
                    e.places,
                    if is_lift { " [lift]" } else { "" }
                );
            }
            marks.push((from.0, e.clone(), class));
        }
    }
    // The same verdicts for level 5's own graph (sidekick::map::Graph, #10):
    // each of its ways, a climb or not.
    let level5 = sidekick::map::Graph::new(&sidekick::starquake::all_rooms(&base), CORE_ROOM);
    let mut l5: BTreeMap<(Class, bool), usize> = BTreeMap::new();
    let mut l5_unproven = Vec::new();
    let mut l5_outside = 0;
    for from in level5.places() {
        for w in level5.ways(from) {
            if !graph.contains_key(&from) {
                l5_outside += 1;
                continue;
            }
            let class = match crossed.get(&(from, w.to)) {
                Some(false) => Class::Proven,
                Some(true) => Class::FromMapEntry,
                None => Class::NotProven,
            };
            *l5.entry((class, w.climb)).or_default() += 1;
            if class == Class::NotProven && !w.climb {
                l5_unproven.push(format!("{}:{} -> {}:{}", from.0, from.1, w.to.0, w.to.1));
            }
        }
    }
    let n = |c, climb| l5.get(&(c, climb)).copied().unwrap_or(0);
    let l5_line = format!(
        "LEVEL 5 GRAPH: ways that are not climbs: proven {} , from a map entry {}, not proven {} {:?}; climbs: proven {}, from a map entry {}, not proven {}; ways out of places the search's graph never reached: {l5_outside}",
        n(Class::Proven, false),
        n(Class::FromMapEntry, false),
        n(Class::NotProven, false),
        l5_unproven,
        n(Class::Proven, true),
        n(Class::FromMapEntry, true),
        n(Class::NotProven, true)
    );
    println!("{l5_line}");
    let _ = writeln!(out, "{l5_line}");
    let mut gaps = Vec::new();
    for ((from, to), assumed) in &crossed {
        let in_graph = graph
            .get(from)
            .is_some_and(|es| es.iter().any(|e| e.to == *to));
        if !in_graph {
            let _ = writeln!(
                out,
                "Gap: {}:{} -> {}:{} crossed by play{}, not in the graph",
                from.0,
                from.1,
                to.0,
                to.1,
                if *assumed { " (from a map entry)" } else { "" }
            );
            gaps.push((*from, *to));
        }
    }
    let assumed_parts = sh
        .parts
        .iter()
        .filter(|(_, p)| p.searched && !p.seeds.is_empty() && p.seeds.values().all(|s| s.1))
        .count();
    let summary = format!(
        "graph: {} parts in {} rooms, {edge_count} edges; proven {} (lift {}), proven from a map entry {} (lift {}), not proven {} (lift {}); gaps {}; parts entered only from a map entry {assumed_parts}; {} frames, {:.0?} on {threads} threads",
        graph.len(),
        rooms.len(),
        counts.get(&(Class::Proven, false)).unwrap_or(&0)
            + counts.get(&(Class::Proven, true)).unwrap_or(&0),
        counts.get(&(Class::Proven, true)).unwrap_or(&0),
        counts.get(&(Class::FromMapEntry, false)).unwrap_or(&0)
            + counts.get(&(Class::FromMapEntry, true)).unwrap_or(&0),
        counts.get(&(Class::FromMapEntry, true)).unwrap_or(&0),
        counts.get(&(Class::NotProven, false)).unwrap_or(&0)
            + counts.get(&(Class::NotProven, true)).unwrap_or(&0),
        counts.get(&(Class::NotProven, true)).unwrap_or(&0),
        gaps.len(),
        sh.frames,
        t0.elapsed()
    );
    println!("{summary}");
    print!("{out}");
    std::fs::write(args.path("proof.txt"), format!("{summary}\n{out}")).expect("write proof.txt");

    // The picture: the planet, a mark on the source side of every edge.
    let mut img = picture::picture(&base, &planet, start, &rooms);
    for (from, e, class) in &marks {
        let colour = match class {
            Class::Proven => 0x20E060,
            Class::FromMapEntry => 0x40C8FF,
            Class::NotProven => 0xFF2040,
        };
        let (x0, y0) = picture::origin(*from);
        let mid = |v: &[u8]| i64::from(v[v.len() / 2]);
        let (x, y) = match e.dir {
            Dir::Left => (x0 + 4, y0 + (mid(&e.places) - 6) * 8 + 4),
            Dir::Right => (x0 + ROOM_W as i64 - 16, y0 + (mid(&e.places) - 6) * 8 + 4),
            Dir::Up => (x0 + mid(&e.places) * 8 + 4, y0 + 4),
            Dir::Down => (x0 + mid(&e.places) * 8 + 4, y0 + ROOM_H as i64 - 16),
            Dir::Passage => (
                x0 + ROOM_W as i64 / 2 - 6 + if e.to.0 > *from { 20 } else { -20 },
                y0 + ROOM_H as i64 / 2,
            ),
        };
        let size = if *class == Class::Proven { 10 } else { 14 };
        img.fill(x - 1, y - 1, size + 2, size + 2, 0x000000);
        img.fill(x, y, size, size, colour);
    }
    for (from, to) in &gaps {
        let (x0, y0) = picture::origin(from.0);
        let _ = to;
        img.outline(
            x0 + 2,
            y0 + 2,
            ROOM_W as i64 - 4,
            ROOM_H as i64 - 4,
            3,
            0xFF40FF,
        );
    }
    let out_img = if args.flag("half") {
        picture::half(&img)
    } else {
        img
    };
    std::fs::write(args.path("proof.png"), out_img.png()).expect("write proof.png");
    eprintln!(
        "wrote proof.txt and proof.png: green proven, blue proven from a map entry, red not proven; magenta outline a room with a crossing the graph lacks"
    );
}
