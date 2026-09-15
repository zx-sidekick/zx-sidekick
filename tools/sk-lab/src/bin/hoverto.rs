//! From the hover pad in room 438, fly up and steer toward a column, and
//! report where Blob comes out. `hoverto <assets-dir>`
use sidekick::map::Graph;
use sidekick::starquake::{CORE_ROOM, all_rooms};
use sk_lab::search::Platforms;
use sk_lab::{Args, LEFT, RIGHT, UP, into_play, stand};
fn main() {
    let args = Args::parse("hoverto <assets-dir>");
    let base = into_play(&args.tape());
    let graph = Graph::new(&all_rooms(&base), CORE_ROOM);
    for (label, steer) in [("up then left", LEFT), ("up then right", RIGHT)] {
        for hold in [10u32, 25, 40, 60] {
            let Some(mut m) = stand(&base, 438, 168, 95) else {
                println!("no stand");
                return;
            };
            let mut out = None;
            for f in 0..400u32 {
                let input = if f < hold { UP } else { UP | steer };
                sk_lab::frame(&mut m, input, Platforms::None);
                if sk_lab::room(&m) != 438 {
                    let (x, y) = sk_lab::blob(&m);
                    out = Some((sk_lab::room(&m), x, y, f));
                    break;
                }
            }
            match out {
                Some((room, x, y, f)) => println!(
                    "{label}, up {hold} frames first: into room {room} at ({x},{y}) after {f}, place {:?}",
                    graph.place(room, x, y)
                ),
                None => println!(
                    "{label}, up {hold} frames first: stayed, at {:?}",
                    sk_lab::blob(&m)
                ),
            }
        }
    }
    for w in graph.ways((438, 1)) {
        println!("graph way 438:1 -> {:?} climb {}", w.to, w.climb);
    }
    println!("438 place of the pad: {:?}", graph.place(438, 168, 95));
}
