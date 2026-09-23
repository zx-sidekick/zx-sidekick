//! The attribute bytes of a room's cells, a column range at a time, by row
//! from the top of the play area.
//!
//! `attrs <assets-dir> <room> [first-col] [last-col]`

use sk_lab::rooms::Planet;
use sk_lab::{Args, into_play};

fn main() {
    let args = Args::parse("attrs <assets-dir> <room> [first-col] [last-col]");
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let room: u16 = args.get(0, 0);
    let (c0, c1): (usize, usize) = (args.get(1, 0), args.get(2, 31));
    println!("room {room}, columns {c0} to {c1}:");
    for (row, line) in planet.cells[usize::from(room)].iter().enumerate() {
        let bytes: Vec<String> = line[c0..=c1.min(31)]
            .iter()
            .map(|a| format!("{a:02x}"))
            .collect();
        println!("  row {row:2}: {}", bytes.join(" "));
    }
}
