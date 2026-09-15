//! Scratch: search one room again from the entries the whole-map run found
//! into it, with the FULL state key, and report which neighbours it reaches.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use app::search::{self, Platforms};
use sidekick::starquake::{at, routine};
use std::collections::BTreeMap;

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let exits = std::fs::read_to_string(std::env::args().nth(2).unwrap()).unwrap();
    let rooms: Vec<u16> = std::env::args().nth(3).unwrap().split(',').map(|r| r.parse().unwrap()).collect();
    let max_entries: usize = std::env::args().nth(4).and_then(|n| n.parse().ok()).unwrap_or(4);
    let base = app::into_play(&tape);
    std::thread::scope(|s| {
        for &room in &rooms {
            let base = &base; let exits = &exits;
            s.spawn(move || {
                let mut entries: Vec<(u8, u8)> = Vec::new();
                for line in exits.lines() {
                    let Some(rest) = line.trim().strip_prefix("exit ") else { continue };
                    let mut it = rest.split_whitespace();
                    let _from = it.next(); it.next();
                    let to: u16 = it.next().unwrap().parse().unwrap(); it.next();
                    if to != room { continue; }
                    let xy = it.next().unwrap().trim_matches(|c| c == '(' || c == ')');
                    let (x, y) = xy.split_once(',').unwrap();
                    let e = (x.parse().unwrap(), y.parse().unwrap());
                    if !entries.contains(&e) { entries.push(e); }
                }
                // Spread the entries out.
                let step = (entries.len() / max_entries).max(1);
                let chosen: Vec<(u8, u8)> = entries.iter().step_by(step).take(max_entries).copied().collect();
                let mut seeds = Vec::new();
                for &(x, y) in &chosen {
                    let mut m = base.clone();
                    let z = &mut m.zx;
                    z.write16(at::ROOM, room);
                    z.mem[usize::from(at::ENTRY_REASON)] = 0;
                    if !m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000) { println!("room {room}: enter failed"); return; }
                    let z = &mut m.zx;
                    z.t = 0; z.set_interrupts(true);
                    let e = usize::from(at::ENTITIES);
                    z.mem[e + 5] = x; z.mem[e + 6] = y;
                    search::settle(&mut m, Platforms::Unlimited);
                    if m.zx.read16(at::ROOM) != room { println!("room {room}: entry ({x},{y}) left the room while settling"); continue; }
                    seeds.push(m);
                }
                let t = std::time::Instant::now();
                let mut r = search::Room::default();
                r.explore(room, seeds, Platforms::Unlimited);
                let mut by: BTreeMap<u16, usize> = BTreeMap::new();
                for k in &r.exit_keys { *by.entry(k.0).or_default() += 1; }
                println!("room {room}: {} entries {:?}; {} states, {} frames, {:.0?}; exits by room {:?}; modal {}, deaths {}, pruned {}", chosen.len(), chosen, r.seen.len(), r.frames, t.elapsed(), by, r.modal.len(), r.deaths, r.pruned);
            });
        }
    });
}
