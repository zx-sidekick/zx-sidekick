//! The item table as play starts: each item's graphic and room, counted by graphic.
//!
//! `items <assets-dir>`
use sidekick::starquake::{CORE_ROOM, items_and_core};
use sk_lab::{Args, into_play};
use std::collections::BTreeMap;
fn main() {
    let args = Args::parse("items <assets-dir>");
    let m = into_play(&args.tape());
    let (items, core) = items_and_core(&m.zx.mem[..]);
    let mut by: BTreeMap<u8, Vec<u16>> = BTreeMap::new();
    for it in &items {
        by.entry(it.graphic()).or_default().push(it.room());
    }
    for (g, rooms) in &by {
        let out: Vec<u16> = rooms.iter().copied().filter(|&r| r != CORE_ROOM).collect();
        println!(
            "graphic {g:2}: {} items, {} in the core room, out on the planet in rooms {out:?}",
            rooms.len(),
            rooms.len() - out.len()
        );
    }
    println!("core slots {core:02x?}");
}
