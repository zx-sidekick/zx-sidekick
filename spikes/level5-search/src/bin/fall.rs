//! Scratch: stand Blob at (x, y) in a room and watch whether he falls, with one input held.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use app::search::{self, Platforms};
use sidekick::starquake::{at, routine};

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let base = app::into_play(&tape);
    let e = usize::from(at::ENTITIES);
    for spec in std::env::args().skip(2) {
        let v: Vec<u8> = spec.split(',').map(|s| s.parse::<u16>().unwrap() as u8).collect();
        let room: u16 = spec.split(',').next().unwrap().parse().unwrap();
        let (x, y, inp) = (v[1], v[2], v[3]);
        let mut m = base.clone();
        let z = &mut m.zx;
        z.write16(at::ROOM, room);
        z.mem[usize::from(at::ENTRY_REASON)] = 0;
        assert!(m.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000));
        let z = &mut m.zx;
        z.t = 0; z.set_interrupts(true);
        z.mem[e + 5] = x; z.mem[e + 6] = y;
        m.watch = vec![routine::MODAL, routine::DEATH, sidekick::starquake::PLAY_INPUT];
        let mut trail = Vec::new();
        let mut out = None;
        for f in 0..200 {
            let hits = search::frame(&mut m, inp, Platforms::Unlimited);
            let (nx, ny) = (m.zx.mem[e + 5], m.zx.mem[e + 6]);
            if f % 10 == 0 { trail.push(format!("{nx},{ny}")); }
            if hits.contains(&routine::DEATH) { out = Some(format!("death at frame {f}")); break; }
            if m.zx.read16(at::ROOM) != room { out = Some(format!("room {} at frame {f}", m.zx.read16(at::ROOM))); break; }
        }
        println!("room {room} from ({x},{y}) input {inp}: {} ; trail {}", out.unwrap_or("stays".into()), trail.join(" "));
    }
}
