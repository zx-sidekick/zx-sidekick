//! Scratch: the attribute bytes of a room's cells in a column range, by row.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use sidekick::starquake::read_room;
fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let base = app::into_play(&tape);
    for spec in std::env::args().skip(2) {
        let v: Vec<usize> = spec.split(',').map(|s| s.parse().unwrap()).collect();
        let (room, c0, c1) = (v[0] as u16, v[1], v[2]);
        let mut m = base.clone();
        read_room(&mut m, room);
        println!("room {room} cols {c0}..={c1}:");
        for row in 6..24 { let line: Vec<String> = (c0..=c1).map(|c| format!("{:02x}", m.zx.mem[0x5800 + row * 32 + c])).collect(); println!("  row {row:2}: {}", line.join(" ")); }
    }
}
