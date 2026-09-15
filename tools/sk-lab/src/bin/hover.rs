//! Every marker 0x0C on the planet, Blob stood on it with up held, on
//! copies of the machine: whether he rises, and where he goes.
//! `hover <assets-dir>`
use sidekick::map::marker_cell;
use sidekick::starquake::{at, read_room};
use sk_lab::search::Platforms;
use sk_lab::{Args, UP, into_play, stand};
fn main() {
    let args = Args::parse("hover <assets-dir>");
    let base = into_play(&args.tape());
    let mut kinds = std::collections::BTreeMap::<u8, usize>::new();
    for room in 0..512u16 {
        let mut m = base.clone();
        read_room(&mut m, room);
        let z = &m.zx;
        let end = z.read16(at::MARKERS_END).max(at::MARKERS);
        let markers: Vec<(u8, u8, u8)> = (at::MARKERS..end)
            .step_by(3)
            .map(|a| {
                let a = usize::from(a);
                (z.mem[a], z.mem[a + 1], z.mem[a + 2])
            })
            .collect();
        for &(x, y, kind) in &markers {
            *kinds.entry(kind).or_default() += 1;
            if kind != 0x0C || ![380u16, 393, 453, 484, 438, 461].contains(&room) {
                continue;
            }
            let (row, col) = marker_cell(x, y);
            // Stand on the pad: Blob's feet on its row, from a little above.
            let Some(s) = stand(&base, room, x, y.saturating_add(8)) else {
                println!("room {room} pad at ({row},{col}) x{x} y{y}: did not settle");
                continue;
            };
            let start = sk_lab::blob(&s);
            // Three tries on copies: nothing held; up held; up for 20 frames then right held; up for 20 then nothing.
            let run = |plan: &dyn Fn(u32) -> u8| {
                let mut c = s.clone();
                let mut out = Vec::new();
                for f in 0..120u32 {
                    sk_lab::frame(&mut c, plan(f), Platforms::None);
                    if f % 20 == 19 {
                        out.push(sk_lab::blob(&c));
                    }
                    if sk_lab::room(&c) != room {
                        out.push((0, 0));
                        break;
                    }
                }
                out
            };
            let still = run(&|_| 0);
            let rise_right = run(&|f| if f < 20 { UP } else { UP | 1 });
            let rise_let_go = run(&|f| if f < 20 { UP } else { 0 });
            let left: Option<(u16, u32)> = None;
            let trail = format!(
                "still {still:?}; up then up+right {rise_right:?}; up then let go {rise_let_go:?}"
            );
            println!(
                "room {room} pad at ({row},{col}) marker ({x},{y}): from {start:?} holding up: {trail:?}; left for {left:?}"
            );
        }
    }
    println!("marker kinds: {kinds:?}");
}
