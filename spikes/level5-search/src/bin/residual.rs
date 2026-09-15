//! Scratch analysis: which open edges the search could not cross, and whether
//! the map's own parts explain them; and rooms the parts graph reaches that
//! the search did not (and the other way round).
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use sidekick::map::{Room, COLS, ROWS};
use sidekick::starquake::{read_room, all_openings, at, CORE_ROOM};
use std::collections::{HashMap, HashSet, VecDeque, BTreeSet};

const FIRST_ROW: u8 = 6; const LAST_ROW: u8 = 23; const LAST_COL: u8 = 31;

fn cell(x: u8, y: u8) -> (u8, u8) { (0x18u8.wrapping_sub((y as u16 + 1 >> 3) as u8), x >> 3) }

/// The parts (with doors shut) touching each edge: (left, right, up, down).
fn edge_parts(r: &Room) -> [BTreeSet<u8>; 4] {
    let mut e: [BTreeSet<u8>; 4] = Default::default();
    for row in FIRST_ROW..LAST_ROW {
        let p = r.shut.at(row, 0); if p != 0 { e[0].insert(p); }
        let p = r.shut.at(row, LAST_COL - 1); if p != 0 { e[1].insert(p); }
    }
    for col in 0..LAST_COL {
        let p = r.shut.at(FIRST_ROW, col); if p != 0 { e[2].insert(p); }
        let p = r.shut.at(LAST_ROW - 1, col); if p != 0 { e[3].insert(p); }
    }
    e
}

fn passage_part(r: &Room) -> Option<u8> {
    let (row, col) = r.passage?;
    (row.saturating_sub(1)..row + 3).flat_map(|rr| (col.saturating_sub(2)..col + 4).map(move |c| (rr, c))).map(|(rr, c)| r.shut.at(rr, c)).find(|&p| p != 0)
}

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let exits_file = std::env::args().nth(2).unwrap_or("exits.txt".into());
    let base = app::into_play(&tape);
    let openings = all_openings(&base);
    let rooms: Vec<Room> = (0..COLS * ROWS).map(|r| read_room(&mut base.clone(), r)).collect();
    let text = std::fs::read_to_string(&exits_file).unwrap();
    let mut reached: BTreeSet<u16> = BTreeSet::new();
    let mut exits: HashMap<u16, Vec<(u16, u8, u8)>> = HashMap::new();
    let mut modal: HashSet<u16> = HashSet::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("room ") {
            let r: u16 = rest.split(' ').next().unwrap().parse().unwrap();
            reached.insert(r);
            if !rest.contains("modal []") && !rest.contains("modal {}") { modal.insert(r); }
        } else if let Some(rest) = line.trim().strip_prefix("exit ") {
            let mut it = rest.split_whitespace();
            let from: u16 = it.next().unwrap().parse().unwrap(); it.next();
            let to: u16 = it.next().unwrap().parse().unwrap(); it.next();
            let xy = it.next().unwrap().trim_matches(|c| c == '(' || c == ')');
            let (x, y) = xy.split_once(',').unwrap();
            exits.entry(from).or_default().push((to, x.parse().unwrap(), y.parse().unwrap()));
        }
    }
    let start = base.zx.read16(at::ROOM);
    let e = usize::from(at::ENTITIES);
    let mut entries: HashMap<u16, Vec<(u8, u8)>> = HashMap::new();
    entries.entry(start).or_default().push((base.zx.mem[e + 5], base.zx.mem[e + 6]));
    for (_, v) in &exits { for &(to, x, y) in v { entries.entry(to).or_default().push((x, y)); } }
    // Entry cells: how many land in a part.
    let (mut fit, mut nofit) = (0, 0);
    for (&room, v) in &entries { for &(x, y) in v { let (r, c) = cell(x, y); if rooms[usize::from(room)].shut.at(r, c) != 0 { fit += 1 } else { nofit += 1; if nofit < 8 { println!("entry into {room} at ({x},{y}) cell ({r},{c}) fits no part"); } } } }
    println!("entries in a part: {fit}, in no part: {nofit}");

    let dirs = [(-1i32, 0usize, "left"), (1, 1, "right"), (-16, 2, "up"), (16, 3, "down")];
    let (mut open_n, mut crossed_n, mut explained, mut residual, mut under) = (0, 0, 0, 0, 0);
    let mut residual_list = Vec::new();
    for &room in &reached {
        if room == CORE_ROOM { continue; }
        let r = &rooms[usize::from(room)];
        let o = openings[usize::from(room)];
        let ep = edge_parts(r);
        let entry_parts: BTreeSet<u8> = entries.get(&room).map(|v| v.iter().map(|&(x, y)| { let (rr, c) = cell(x, y); r.shut.at(rr, c) }).filter(|&p| p != 0).collect()).unwrap_or_default();
        let to: HashSet<u16> = exits.get(&room).map(|v| v.iter().map(|k| k.0).collect()).unwrap_or_default();
        for (d, i, name) in dirs {
            let t = i32::from(room) + d;
            if t < 0 || t >= 512 { continue; }
            let col = room % COLS;
            if (d == -1 && col == 0) || (d == 1 && col == COLS - 1) { continue; }
            let t = t as u16;
            let open = [o.left, o.right, o.up, o.down][i];
            let crossed = to.contains(&t);
            if open { open_n += 1; }
            if crossed { crossed_n += 1; if !open { under += 1; println!("UNDER: {room} -> {t} ({name}) crossed but shown closed"); } }
            if open && !crossed {
                let mut parts = ep[i].clone();
                if i < 2 { if let Some(p) = passage_part(r) { let other = &rooms[usize::from(t)]; if other.passage.is_some() && r.passage.is_some() { parts.insert(p); } } }
                let joined = parts.iter().any(|p| entry_parts.contains(p));
                if joined { residual += 1; residual_list.push((room, t, name, entry_parts.clone(), parts.clone(), t == CORE_ROOM, modal.contains(&room))); } else { explained += 1; }
            }
        }
    }
    println!("reached rooms {}; open edges out of them {open_n}, crossed {crossed_n}, under-claimed {under}; not crossed: {explained} in another part than every entry, {residual} in an entry's part (residual)", reached.len());
    for (room, t, name, ep, parts, core, m) in &residual_list {
        println!("  residual: {room} -> {t} ({name}); entry parts {ep:?}, edge parts {parts:?}{}{}", if *core { " [core room]" } else { "" }, if *m { " [room has a door/booth screen]" } else { "" });
    }

    // Parts graph BFS from the start: (room, part) over matching edge positions, doors shut, no booths.
    let mut seen: HashSet<(u16, u8)> = HashSet::new();
    let (sx, sy) = entries[&start][0];
    let (sr, sc) = cell(sx, sy);
    let sp = rooms[usize::from(start)].shut.at(sr, sc);
    let mut q = VecDeque::from([(start, sp)]);
    seen.insert((start, sp));
    let mut graph_rooms: BTreeSet<u16> = BTreeSet::new();
    while let Some((room, part)) = q.pop_front() {
        graph_rooms.insert(room);
        if room == CORE_ROOM { continue; }
        let r = &rooms[usize::from(room)];
        let col = room % COLS;
        let mut push = |t: u16, p: u8, seen: &mut HashSet<(u16, u8)>, q: &mut VecDeque<(u16, u8)>| { if p != 0 && seen.insert((t, p)) { q.push_back((t, p)); } };
        // left / right: same row.
        if col > 0 { let t = room - 1; let o = &rooms[usize::from(t)]; for row in FIRST_ROW..LAST_ROW { if r.shut.at(row, 0) == part { push(t, o.shut.at(row, LAST_COL - 1), &mut seen, &mut q); } } }
        if col < COLS - 1 { let t = room + 1; let o = &rooms[usize::from(t)]; for row in FIRST_ROW..LAST_ROW { if r.shut.at(row, LAST_COL - 1) == part { push(t, o.shut.at(row, 0), &mut seen, &mut q); } } }
        if room >= COLS { let t = room - COLS; let o = &rooms[usize::from(t)]; for c in 0..LAST_COL { if r.shut.at(FIRST_ROW, c) == part { push(t, o.shut.at(LAST_ROW - 1, c), &mut seen, &mut q); } } }
        if room + COLS < 512 { let t = room + COLS; let o = &rooms[usize::from(t)]; for c in 0..LAST_COL { if r.shut.at(LAST_ROW - 1, c) == part { push(t, o.shut.at(FIRST_ROW, c), &mut seen, &mut q); } } }
        if let Some(p) = passage_part(r) { if p == part {
            if col > 0 { let t = room - 1; let o = &rooms[usize::from(t)]; if let Some(op) = passage_part(o) { push(t, op, &mut seen, &mut q); } }
            if col < COLS - 1 { let t = room + 1; let o = &rooms[usize::from(t)]; if let Some(op) = passage_part(o) { push(t, op, &mut seen, &mut q); } }
        } }
    }
    let only_graph: Vec<u16> = graph_rooms.difference(&reached).copied().collect();
    let only_search: Vec<u16> = reached.difference(&graph_rooms).copied().collect();
    println!("parts graph from the start reaches {} rooms; search reached {}; graph-only {} {:?}; search-only {} {:?}", graph_rooms.len(), reached.len(), only_graph.len(), only_graph, only_search.len(), only_search);
    // Which graph-only rooms are behind a room with a door/booth screen? (Their entry from a modal room.)
    let modal_reached: Vec<u16> = modal.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
    println!("reached rooms with a door/booth screen: {modal_reached:?}");
}
