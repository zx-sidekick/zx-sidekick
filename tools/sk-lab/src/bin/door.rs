//! Walks Blob into a security door and runs its screen on a copy: which
//! items the door asks for (the code the screen computes, read from memory),
//! whether it opens with what Blob carries, and where he ends up.
//!
//! `door <assets-dir> <room> <x> <y> [input] [--carry=G,G,...]`: Blob stands
//! at (x, y) and holds the input (1 right, 2 left) until the door screen
//! opens; up to four items by graphic go in his inventory first.

use sidekick::starquake::routine;
use sk_lab::search::Platforms;
use sk_lab::{Args, CODE, SEED, carry, into_play, stand};

fn main() {
    let args = Args::parse("door <assets-dir> <room> <x> <y> [input] [--carry=G,G,...]");
    let mut base = into_play(&args.tape());
    let room: u16 = args.get(0, 0);
    let (x, y): (u8, u8) = (args.get(1, 0), args.get(2, 63));
    let input: u8 = args.get(3, 1);
    let carried: String = args.value("carry", String::new());
    for g in carried.split(',').filter_map(|g| g.parse::<u8>().ok()) {
        if !carry(&mut base, g) {
            println!("no item is drawn with graphic {g}");
            return;
        }
    }
    let seed = base.zx.read16(SEED);
    let Some(mut m) = stand(&base, room, x, y) else {
        println!("room {room}: Blob at ({x},{y}) left the room or it did not settle");
        return;
    };
    let mut opened = None;
    for f in 0..300 {
        let hits = sk_lab::frame(&mut m, input, Platforms::Unlimited);
        if hits.contains(&routine::MODAL) {
            opened = Some(f);
            break;
        }
        if sk_lab::room(&m) != room {
            println!(
                "room {room}: left for {} before any screen",
                sk_lab::room(&m)
            );
            return;
        }
    }
    let Some(at_frame) = opened else {
        println!("room {room}: no screen opened in 300 frames from ({x},{y}) holding {input}");
        return;
    };
    let (bx, by) = sk_lab::blob(&m);
    // The screen runs on: let it, with nothing held, until play resumes.
    m.watch = vec![routine::MAIN_LOOP, routine::DEATH];
    let mut resumed = None;
    for f in 0..3000 {
        let hits = sk_lab::frame(&mut m, 0, Platforms::Unlimited);
        if hits.contains(&routine::MAIN_LOOP) {
            resumed = Some(f);
            break;
        }
    }
    let code = usize::from(CODE);
    let mem = &m.zx.mem;
    let wanted: Vec<u8> = (0..usize::from(mem[code + 2]))
        .map(|i| mem[code + 3 + i * 2])
        .collect();
    let matched: Vec<u8> = (0..usize::from(mem[code + 2]))
        .map(|i| mem[code + 3 + i * 2 + 1])
        .collect();
    let (nx, ny) = sk_lab::blob(&m);
    let inventory: Vec<u8> = (0..4)
        .map(|s| mem[usize::from(sk_lab::INVENTORY) + s * 2])
        .collect();
    println!("inventory after: {inventory:?}");
    println!(
        "room {room}, seed {seed:#06x}: the screen opened at frame {at_frame} with Blob at ({bx},{by}); code length {} at column {} row {}; wanted graphics {wanted:?}, matched flags {matched:?}; play resumed {}; Blob now at ({nx},{ny}) in room {}{}",
        mem[code + 2],
        mem[code],
        mem[code + 1],
        resumed.map_or("never".to_string(), |f| format!("after {f} frames")),
        sk_lab::room(&m),
        if nx != bx {
            ", moved through"
        } else {
            ", not moved"
        }
    );
}
