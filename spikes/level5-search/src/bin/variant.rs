#[path = "../main.rs"] #[allow(dead_code)] mod app;
use app::search::{self, Platforms};
use sidekick::starquake::at;
fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let base = app::into_play(&tape);
    let mut r = search::Room::default();
    let t = std::time::Instant::now();
    r.explore(base.zx.read16(at::ROOM), vec![base.clone()], Platforms::Unlimited);
    let mut ex: Vec<_> = r.exit_keys.iter().copied().collect(); ex.sort();
    println!("HOLD={:?} NO_DIAG={}: {} states, {} frames, {:.1?}; {} exits {:?}", std::env::var("HOLD").ok(), std::env::var("NO_DIAG").is_ok(), r.seen.len(), r.frames, t.elapsed(), ex.len(), ex);
}
