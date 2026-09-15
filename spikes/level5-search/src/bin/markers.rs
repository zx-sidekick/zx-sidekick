//! Scratch: a room's markers (x, y, kind) and their cells, and the attributes of given cells.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use sidekick::map::marker_cell;
use sidekick::starquake::{at, read_room};
fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let base = app::into_play(&tape);
    for spec in std::env::args().skip(2) {
        let room: u16 = spec.parse().unwrap();
        let mut m = base.clone();
        let r = read_room(&mut m, room);
        let z = &m.zx;
        let end = z.read16(at::MARKERS_END).max(at::MARKERS);
        let mut out = Vec::new();
        for a in (at::MARKERS..end).step_by(3) {
            let a = usize::from(a);
            let (x, y, k) = (z.mem[a], z.mem[a + 1], z.mem[a + 2]);
            let (row, col) = marker_cell(x, y);
            out.push(format!("kind {k:#04x} at ({x},{y}) cell ({row},{col}) attr {:#04x}", z.mem[0x5800 + usize::from(row) * 32 + usize::from(col)]));
        }
        println!("room {room}: passage {:?}; markers: {}", r.passage, out.join("; "));
    }
}
