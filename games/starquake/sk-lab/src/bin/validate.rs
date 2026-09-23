//! Whether the search is right where it says yes: every exit it finds in a
//! room is replayed from the entry with the recorded inputs, and random
//! walks from the same entry, diagonals included, look for exits it missed.
//!
//! `validate <assets-dir> [walks] [entries] [--full] [--diagonals] [--no-prune] [--no-platforms]`

use std::collections::{HashMap, HashSet};

use sk_lab::search::{Exit, INPUTS, Room, Settings, replay};
use sk_lab::{Args, Rng, into_play};
use starquake::Machine;
use starquake::facts::{PLAY_INPUT, routine};

fn main() {
    let args = Args::parse(
        "validate <assets-dir> [walks] [entries] [--full] [--diagonals] [--no-prune] [--no-platforms]",
    );
    let walks: usize = args.get(0, 80);
    let only: usize = args.get(1, 6);
    let settings = Settings::from_args(&args);
    let base = into_play(&args.tape());
    // Entries: the start, and every exit the start room's search found.
    let mut entries: Vec<Machine> = vec![base.clone()];
    let mut first = Room::default();
    first.explore(sk_lab::room(&base), vec![base.clone()], &settings);
    entries.extend(first.exits.values().cloned());
    let mut rng = Rng(0xC0FFEE);
    let (mut replayed, mut replay_ok, mut walk_exits, mut walk_missed) = (0, 0, 0, 0);
    for entry in entries.iter().take(only) {
        let room = sk_lab::room(entry);
        let mut r = Room::default();
        let t = std::time::Instant::now();
        r.explore(room, vec![entry.clone()], &settings);
        let took = t.elapsed();
        let mut bad = Vec::new();
        for (exit, (key, input)) in &r.exit_path {
            let (_, mut inputs) = r.path(key.clone());
            inputs.push(*input);
            replayed += 1;
            if replay(entry, &inputs, settings.platforms) == Some(*exit) {
                replay_ok += 1;
            } else {
                bad.push((*exit, inputs.len()));
            }
        }
        let found: HashSet<Exit> = r.exit_keys.iter().copied().collect();
        let mut taken: HashMap<Exit, usize> = HashMap::new();
        for _ in 0..walks {
            let mut m = entry.clone();
            m.watch = vec![routine::MODAL, routine::DEATH, PLAY_INPUT];
            let mut input = 0u8;
            for f in 0..1500 {
                if f % (5 + rng.below(20) as usize) == 0 {
                    input = INPUTS[rng.below(9) as usize];
                }
                let hits = sk_lab::frame(&mut m, input, settings.platforms);
                if hits.contains(&routine::DEATH) || hits.contains(&routine::MODAL) {
                    break;
                }
                let now = sk_lab::room(&m);
                if now != room {
                    sk_lab::settle(&mut m, settings.platforms);
                    let (x, y) = sk_lab::blob(&m);
                    *taken.entry((now, x, y)).or_default() += 1;
                    break;
                }
            }
        }
        let missed: Vec<&Exit> = taken.keys().filter(|k| !found.contains(k)).collect();
        walk_exits += taken.len();
        walk_missed += missed.len();
        let (x, y) = sk_lab::blob(entry);
        println!(
            "room {room:3} from ({x},{y}): {} states, {took:.1?}; exits {}, replayed ok {}/{} {bad:?}; walks took {} distinct exits, missed by the search {missed:?}",
            r.seen.len(),
            found.len(),
            r.exit_path.len() - bad.len(),
            r.exit_path.len(),
            taken.len()
        );
    }
    println!("replayed {replay_ok}/{replayed}; walk exits missed {walk_missed}/{walk_exits}");
}
