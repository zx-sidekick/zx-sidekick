//! A room's markers, the three bytes each its tiles leave (x, y, kind), with
//! the cell each is at: `0x00` a security door, `0x0B` a space lock,
//! `0x0C` a flying platform, `0x0D` a teleport, `0x0E` a platform pack, `0x0F` a
//! secret passage.
//!
//! `markers <assets-dir> <room>[,<room>...]`

use sidekick::map::marker_cell;
use sidekick::starquake::{at, read_room};
use sk_lab::{Args, into_play};

fn main() {
    let args = Args::parse("markers <assets-dir> <room>[,<room>...]");
    let base = into_play(&args.tape());
    for room in args.rooms(0) {
        let mut m = base.clone();
        let r = read_room(&mut m, room);
        let z = &m.zx;
        let end = z.read16(at::MARKERS_END).max(at::MARKERS);
        let list: Vec<String> = (at::MARKERS..end)
            .step_by(3)
            .map(|a| {
                let a = usize::from(a);
                let (x, y, kind) = (z.mem[a], z.mem[a + 1], z.mem[a + 2]);
                let (row, col) = marker_cell(x, y);
                format!("kind {kind:#04x} at ({x},{y}) cell ({row},{col})")
            })
            .collect();
        println!(
            "room {room}: passage {:?}; markers: {}",
            r.passage,
            if list.is_empty() {
                "none".to_string()
            } else {
                list.join("; ")
            }
        );
    }
}
