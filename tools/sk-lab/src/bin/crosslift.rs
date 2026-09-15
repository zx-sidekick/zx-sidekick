//! From room 438's hover pad: take off, fly down to the corridor across the
//! lift, fly left past it, then up. Where does Blob come out?
//! `crosslift <assets-dir>`
use sk_lab::search::Platforms;
use sk_lab::{Args, DOWN, LEFT, UP, into_play, stand};
fn main() {
    let args = Args::parse("crosslift <assets-dir>");
    let base = into_play(&args.tape());
    let Some(mut m) = stand(&base, 438, 168, 95) else {
        println!("no stand");
        return;
    };
    let state = |m: &sidekick::Machine| m.zx.mem[sk_lab::BLOB + 0x0A];
    let mut log = Vec::new();
    for _ in 0..120 {
        if state(&m) == 2 {
            break;
        }
        sk_lab::frame(&mut m, UP, Platforms::None);
    }
    for _ in 0..10 {
        sk_lab::frame(&mut m, UP, Platforms::None);
    }
    log.push(("took off", sk_lab::blob(&m), state(&m)));
    for _ in 0..30 {
        if sk_lab::blob(&m).0 <= 140 {
            break;
        }
        sk_lab::frame(&mut m, LEFT, Platforms::None);
    }
    log.push(("left off the pad", sk_lab::blob(&m), state(&m)));
    for _ in 0..60 {
        if sk_lab::blob(&m).1 <= 63 {
            break;
        }
        sk_lab::frame(&mut m, DOWN, Platforms::None);
    }
    log.push(("down to the corridor", sk_lab::blob(&m), state(&m)));
    for _ in 0..120 {
        if sk_lab::blob(&m).0 <= 40 {
            break;
        }
        sk_lab::frame(&mut m, LEFT, Platforms::None);
    }
    log.push(("left across the lift", sk_lab::blob(&m), state(&m)));
    let mut out = None;
    for f in 0..200 {
        sk_lab::frame(&mut m, UP, Platforms::None);
        if sk_lab::room(&m) != 438 {
            out = Some((sk_lab::room(&m), sk_lab::blob(&m), f));
            break;
        }
    }
    for (what, at, st) in log {
        println!("{what}: at {at:?}, state {st}");
    }
    println!("then up: {out:?}, state {}", state(&m));
}
