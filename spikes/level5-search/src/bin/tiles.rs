#[path = "../main.rs"] #[allow(dead_code)] mod app;
use sidekick::starquake::read_room;
fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let base = app::into_play(&tape);
    for pair in std::env::args().skip(2) {
        let rooms: Vec<u16> = pair.split(',').map(|r| r.parse().unwrap()).collect();
        let grids: Vec<Vec<String>> = rooms.iter().map(|&r| {
            let mut m = base.clone();
            let room = read_room(&mut m, r);
            let o = room.openings;
            let mut g = vec![format!("room {r}: L{} R{} U{} D{} passage {:?}          ", o.left as u8, o.right as u8, o.up as u8, o.down as u8, room.passage)];
            for row in 6..24 { g.push((0..32).map(|c| if sidekick::map::free(m.zx.mem[0x5800 + row * 32 + c]) { '.' } else { '#' }).collect()); }
            g
        }).collect();
        for i in 0..grids[0].len() { println!("{}", grids.iter().map(|g| format!("{:<34}", &g[i])).collect::<Vec<_>>().join(" ")); }
        println!();
    }
}
