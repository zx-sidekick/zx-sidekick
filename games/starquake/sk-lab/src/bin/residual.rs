//! The search against the map's own reading, from `exits.txt`: every open
//! edge out of a room the search reached that it never crossed, and whether
//! the map's parts explain it (the edge is in a part no way in reaches); and
//! the planet's (room, part) graph's reach from the start against the
//! search's.
//!
//! `residual <assets-dir>`

use std::collections::{BTreeSet, HashSet};

use sidekick::map::{COLS, Room};
use sidekick::starquake::CORE_ROOM;
use sk_lab::exits;
use sk_lab::rooms::Planet;
use sk_lab::{Args, into_play, top_row};

/// The parts, with doors shut, touching each edge: left, right, up, down.
fn edge_parts(r: &Room) -> [BTreeSet<u8>; 4] {
    let mut parts: [BTreeSet<u8>; 4] = Default::default();
    for row in 6..23 {
        for (i, col) in [(0, 0), (1, 30)] {
            let p = r.shut.at(row, col);
            if p != 0 {
                parts[i].insert(p);
            }
        }
    }
    for col in 0..31 {
        for (i, row) in [(2, 6), (3, 22)] {
            let p = r.shut.at(row, col);
            if p != 0 {
                parts[i].insert(p);
            }
        }
    }
    parts
}

fn main() {
    let args = Args::parse("residual <assets-dir>");
    let text = std::fs::read_to_string(args.path(exits::FILE)).unwrap_or_else(|e| {
        eprintln!(
            "cannot read {}: {e} (run whole first)",
            args.path(exits::FILE).display()
        );
        std::process::exit(2);
    });
    let dump = exits::read(&text);
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let start = sk_lab::room(&base);
    let (bx, by) = sk_lab::blob(&base);

    let dirs = [
        (-1i32, 0usize, "left"),
        (1, 1, "right"),
        (-16, 2, "up"),
        (16, 3, "down"),
    ];
    let (mut open, mut crossed, mut explained, mut under) = (0, 0, 0, 0);
    let mut residual = Vec::new();
    for (&room, record) in &dump {
        if room == CORE_ROOM {
            continue;
        }
        let r = &planet.rooms[usize::from(room)];
        let o = planet.openings[usize::from(room)];
        let parts = edge_parts(r);
        let mut entry_parts: BTreeSet<u8> = exits::entries(&dump, room)
            .iter()
            .map(|&(x, y)| r.shut.at(top_row(y), x >> 3))
            .filter(|&p| p != 0)
            .collect();
        if room == start {
            entry_parts.insert(r.shut.at(top_row(by), bx >> 3));
        }
        let to: HashSet<u16> = record.exits.iter().map(|e| e.0).collect();
        for (d, i, name) in dirs {
            let col = room % COLS;
            if (d == -1 && col == 0) || (d == 1 && col == COLS - 1) {
                continue;
            }
            let t = i32::from(room) + d;
            if !(0..512).contains(&t) {
                continue;
            }
            let t = t as u16;
            let is_open = [o.left, o.right, o.up, o.down][i];
            let was_crossed = to.contains(&t);
            if is_open {
                open += 1;
            }
            if was_crossed {
                crossed += 1;
                if !is_open {
                    under += 1;
                    println!("crossed but shown closed: {room} -> {t} ({name})");
                }
            }
            if is_open && !was_crossed {
                let mut edge = parts[i].clone();
                if i < 2
                    && planet.rooms[usize::from(t)].passage.is_some()
                    && let Some(p) = planet.passage_part(room)
                {
                    edge.insert(p);
                }
                if edge.iter().any(|p| entry_parts.contains(p)) {
                    residual.push(format!(
                        "{room} -> {t} ({name}); entry parts {entry_parts:?}, edge parts {edge:?}{}{}",
                        if t == CORE_ROOM { " [the core room]" } else { "" },
                        if record.modal { " [a door or booth screen in the room]" } else { "" }
                    ));
                } else {
                    explained += 1;
                }
            }
        }
    }
    println!(
        "rooms reached {}; open edges out of them {open}, crossed {crossed}, crossed but shown closed {under}; not crossed: {explained} in another part than every way in, {} in a way in's part:",
        dump.len(),
        residual.len()
    );
    for line in &residual {
        println!("  {line}");
    }
    let reach = planet.reach(start, top_row(by), bx >> 3);
    let reached: BTreeSet<u16> = dump.keys().copied().collect();
    let graph: BTreeSet<u16> = reach.iter().copied().collect();
    let only_graph: Vec<u16> = graph.difference(&reached).copied().collect();
    let only_search: Vec<u16> = reached.difference(&graph).copied().collect();
    println!(
        "the (room, part) graph reaches {} rooms from the start with doors shut; the search reached {}; graph only {} {only_graph:?}; search only {} {only_search:?}",
        graph.len(),
        reached.len(),
        only_graph.len(),
        only_search.len()
    );
}
