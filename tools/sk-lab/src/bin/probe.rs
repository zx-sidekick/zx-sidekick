//! One room searched again, from the entries the whole-map run found into it
//! (`exits.txt`), each room on its own thread, reporting which neighbours it
//! reaches. The full key unless `--coarse`.
//!
//! `probe <assets-dir> <room>[,<room>...] [entries] [--coarse] [--diagonals] [--no-prune] [--no-platforms]`

use std::collections::BTreeMap;

use sk_lab::exits;
use sk_lab::search::{COARSE_KEY, FULL_KEY, Room, Settings};
use sk_lab::{Args, into_play, stand};

fn main() {
    let args = Args::parse(
        "probe <assets-dir> <room>[,<room>...] [entries] [--coarse] [--diagonals] [--no-prune] [--no-platforms]",
    );
    let max_entries: usize = args.get(1, 4);
    let settings = Settings {
        key: if args.flag("coarse") {
            COARSE_KEY.to_vec()
        } else {
            FULL_KEY.to_vec()
        },
        ..Settings::from_args(&args)
    };
    let text = std::fs::read_to_string(args.path(exits::FILE)).unwrap_or_else(|e| {
        eprintln!(
            "cannot read {}: {e} (run whole first)",
            args.path(exits::FILE).display()
        );
        std::process::exit(2);
    });
    let dump = exits::read(&text);
    let base = into_play(&args.tape());
    std::thread::scope(|s| {
        for room in args.rooms(0) {
            let (base, dump, settings) = (&base, &dump, &settings);
            s.spawn(move || {
                let entries = exits::entries(dump, room);
                let step = (entries.len() / max_entries).max(1);
                let chosen: Vec<(u8, u8)> = entries.iter().step_by(step).take(max_entries).copied().collect();
                let mut seeds = Vec::new();
                for &(x, y) in &chosen {
                    match stand(base, room, x, y) {
                        Some(m) => seeds.push(m),
                        None => println!("room {room}: entry ({x},{y}) left the room while settling"),
                    }
                }
                let t = std::time::Instant::now();
                let mut r = Room::default();
                r.explore(room, seeds, settings);
                let mut by: BTreeMap<u16, usize> = BTreeMap::new();
                for k in &r.exit_keys {
                    *by.entry(k.0).or_default() += 1;
                }
                println!(
                    "room {room}: {} entries {chosen:?}; {} states, {} frames, {:.0?}; exits by room {by:?}; door or booth screens {}, deaths {}, pruned {}",
                    chosen.len(),
                    r.seen.len(),
                    r.frames,
                    t.elapsed(),
                    r.modal.len(),
                    r.deaths,
                    r.pruned
                );
            });
        }
    });
}
