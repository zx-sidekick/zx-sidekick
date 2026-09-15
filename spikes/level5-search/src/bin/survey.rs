//! Scratch: every attribute value used in the room area across all 512 rooms, and the rooms holding the lift's values.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use sidekick::starquake::read_room;
use std::collections::BTreeMap;
fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let base = app::into_play(&tape);
    let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
    let mut lift_rooms: BTreeMap<u16, usize> = BTreeMap::new();
    for room in 0..512u16 {
        let mut m = base.clone();
        read_room(&mut m, room);
        for row in 6..24 { for col in 0..32 {
            let a = m.zx.mem[0x5800 + row * 32 + col];
            *hist.entry(a).or_default() += 1;
            if a == 0x60 || a == 0x64 { *lift_rooms.entry(room).or_default() += 1; }
        } }
    }
    println!("attributes: {hist:?}");
    println!("rooms with 0x60/0x64 cells ({}): {lift_rooms:?}", lift_rooms.len());
}
