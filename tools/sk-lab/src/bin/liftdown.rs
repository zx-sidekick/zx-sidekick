//! From room 438's hover pad: take off, fly up and over the lift shaft,
//! then down into it. Does a flying Blob go down a lift? `liftdown <assets-dir>`
use sk_lab::search::Platforms;
use sk_lab::{Args, DOWN, LEFT, UP, into_play, stand};
fn main() {
    let args = Args::parse("liftdown <assets-dir>");
    let base = into_play(&args.tape());
    let Some(mut m) = stand(&base, 438, 168, 95) else {
        println!("no stand");
        return;
    };
    let state = |m: &sidekick::Machine| m.zx.mem[sk_lab::BLOB + 0x0A];
    for _ in 0..120 {
        if state(&m) == 2 {
            break;
        }
        sk_lab::frame(&mut m, UP, Platforms::None);
    }
    for _ in 0..40 {
        if sk_lab::blob(&m).1 >= 135 {
            break;
        }
        sk_lab::frame(&mut m, UP, Platforms::None);
    }
    println!(
        "up: {:?} state {} room {}",
        sk_lab::blob(&m),
        state(&m),
        sk_lab::room(&m)
    );
    for _ in 0..80 {
        if sk_lab::blob(&m).0 <= 72 {
            break;
        }
        sk_lab::frame(&mut m, LEFT, Platforms::None);
    }
    println!(
        "over the shaft: {:?} state {} room {}",
        sk_lab::blob(&m),
        state(&m),
        sk_lab::room(&m)
    );
    let mut last = sk_lab::blob(&m);
    for f in 0..200 {
        sk_lab::frame(&mut m, DOWN, Platforms::None);
        let now = sk_lab::blob(&m);
        if sk_lab::room(&m) != 438 {
            println!(
                "down: left for room {} at {:?} after {f}, state {}",
                sk_lab::room(&m),
                now,
                state(&m)
            );
            return;
        }
        if f % 20 == 19 {
            println!("  down {f}: {now:?} state {}", state(&m));
        }
        last = now;
    }
    println!("down: stayed at {last:?} state {}", state(&m));
}
