//! From hover pads whose flight leaves through the top, keep holding up:
//! does Blob keep flying in the room above, and how far up does he get?
//! `flyon <assets-dir>`
use sk_lab::search::Platforms;
use sk_lab::{Args, LEFT, RIGHT, UP, into_play, stand};
fn main() {
    let args = Args::parse("flyon <assets-dir>");
    let base = into_play(&args.tape());
    for (room, x, y) in [
        (393u16, 104u8, 47u8),
        (428, 40, 47),
        (434, 168, 47),
        (506, 72, 47),
        (461, 104, 71),
    ] {
        let Some(mut m) = stand(&base, room, x, y) else {
            println!("{room}: no stand");
            continue;
        };
        let mut rooms = vec![room];
        let mut trail = Vec::new();
        for f in 0..900u32 {
            // Up, with a sideways wiggle now and then, as a player steering.
            let input = match (f / 60) % 4 {
                1 => UP | LEFT,
                3 => UP | RIGHT,
                _ => UP,
            };
            sk_lab::frame(&mut m, input, Platforms::None);
            let now = sk_lab::room(&m);
            if *rooms.last().unwrap() != now {
                rooms.push(now);
                trail.push((f, sk_lab::blob(&m)));
            }
        }
        // Then let go in whatever room he is in: does he fall?
        let before = sk_lab::blob(&m);
        for _ in 0..50 {
            sk_lab::frame(&mut m, 0, Platforms::None);
        }
        println!(
            "pad in {room}: rooms {rooms:?} at {trail:?}; let go at {before:?}, 50 frames later {:?} in {}",
            sk_lab::blob(&m),
            sk_lab::room(&m)
        );
    }
}
