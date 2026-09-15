//! The whole planet as one picture, `planet.png` in the assets folder:
//! every room as the game draws it, with its openings, inner walls where they stand, doors,
//! wall passages and lifts drawn over it, the start room and the core room
//! outlined, and rooms not reachable from the start with doors shut dimmed.
//!
//! `planet <assets-dir> [--half]`

use sk_lab::picture::{half, picture};
use sk_lab::rooms::Planet;
use sk_lab::{Args, into_play};

fn main() {
    let args = Args::parse("planet <assets-dir> [--half]");
    let base = into_play(&args.tape());
    let start = sk_lab::room(&base);
    let planet = Planet::read(&base);
    let (bx, by) = sk_lab::blob(&base);
    let reach = planet.reach(start, sk_lab::top_row(by), bx >> 3);
    eprintln!(
        "start room {start}; {} rooms reachable from it with doors shut",
        reach.len()
    );
    let img = picture(&base, &planet, start, &reach);
    let out = if args.flag("half") { half(&img) } else { img };
    let path = args.path("planet.png");
    std::fs::write(&path, out.png()).unwrap_or_else(|e| {
        eprintln!("cannot write {}: {e}", path.display());
        std::process::exit(2);
    });
    eprintln!(
        "wrote {} ({} x {}): green bars are openings, orange boxes the cells of a wall inside a room, yellow ones a door's or a pad's, purple a wall passage, green boxes lift cells; white outline the start, pink the core; dimmed rooms are not reachable from the start with doors shut",
        path.display(),
        out.width,
        out.height
    );
}
