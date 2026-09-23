//! One room searched under different settings, to compare keys and inputs:
//! the coarse and the full key, with and without diagonals, and any hold.
//!
//! `variant <assets-dir> [room] [--hold=N] [--no-prune] [--no-platforms]`; the
//! room is entered as walking in, from where the game puts Blob, or the start
//! room from play when none is given.

use starquake_lab::search::{COARSE_KEY, FULL_KEY, Room, Settings};
use starquake_lab::{Args, into_play, stand};

fn main() {
    let args = Args::parse("variant <assets-dir> [room] [--hold=N] [--no-prune] [--no-platforms]");
    let base = into_play(&args.tape());
    let entry = match args.rest.first().and_then(|r| r.parse::<u16>().ok()) {
        Some(room) => {
            let (x, y) = starquake_lab::blob(&base);
            stand(&base, room, x, y).unwrap_or_else(|| {
                eprintln!("room {room} did not settle with Blob at ({x},{y})");
                std::process::exit(2);
            })
        }
        None => base.clone(),
    };
    let room = starquake_lab::room(&entry);
    for (name, key) in [("coarse", COARSE_KEY), ("full", FULL_KEY)] {
        for diagonals in [false, true] {
            let settings = Settings {
                key: key.to_vec(),
                diagonals,
                ..Settings::from_args(&args)
            };
            let mut r = Room::default();
            let t = std::time::Instant::now();
            r.explore(room, vec![entry.clone()], &settings);
            let mut exits: Vec<_> = r.exit_keys.iter().copied().collect();
            exits.sort_unstable();
            println!(
                "room {room}, {name} key, diagonals {diagonals}, hold {}: {} states, {} frames, {:.1?}; {} exits {exits:?}",
                settings.hold,
                r.seen.len(),
                r.frames,
                t.elapsed(),
                exits.len()
            );
        }
    }
}
