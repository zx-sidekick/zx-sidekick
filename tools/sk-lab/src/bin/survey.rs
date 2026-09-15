//! Every attribute value used in the play area across all 512 rooms, how
//! many cells hold each, and the rooms holding lift cells.
//!
//! `survey <assets-dir>`

use std::collections::BTreeMap;

use sk_lab::rooms::{Planet, lift};
use sk_lab::{Args, into_play};

fn main() {
    let args = Args::parse("survey <assets-dir>");
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    let mut lifts: BTreeMap<u16, usize> = BTreeMap::new();
    for (room, cells) in planet.cells.iter().enumerate() {
        for &a in cells.iter().flatten() {
            *counts.entry(a).or_default() += 1;
            if lift(a) {
                *lifts.entry(room as u16).or_default() += 1;
            }
        }
    }
    println!("attribute: cells");
    for (a, n) in &counts {
        println!("  {a:#04x}: {n}");
    }
    println!("rooms with lift cells ({}): {lifts:?}", lifts.len());
}
