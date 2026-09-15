//! Stands Blob at (x, y) in a room, holds one input, and prints where he
//! goes: whether he stays, dies, or leaves for another room, with his
//! position every ten frames. `y` is pixels from the bottom; standing with
//! his top cell in room row R reads `y = 143 - 8R`.
//!
//! `fall <assets-dir> <room> <x> <y> [input] [frames]`, the input as Kempston
//! bits: 1 right, 2 left, 4 down (builds a platform), 8 up.

use sidekick::starquake::routine;
use sk_lab::search::Platforms;
use sk_lab::{Args, into_play, stand};

fn main() {
    let args = Args::parse("fall <assets-dir> <room> <x> <y> [input] [frames]");
    let base = into_play(&args.tape());
    let room: u16 = args.get(0, 0);
    let (x, y): (u8, u8) = (args.get(1, 0), args.get(2, 39));
    let input: u8 = args.get(3, 0);
    let frames: u64 = args.get(4, 200);
    let Some(mut m) = stand(&base, room, x, y) else {
        println!("room {room}: Blob at ({x},{y}) left the room or it did not settle");
        return;
    };
    let mut trail = Vec::new();
    let mut outcome = None;
    for f in 0..frames {
        let hits = sk_lab::frame(&mut m, input, Platforms::Unlimited);
        let (nx, ny) = sk_lab::blob(&m);
        if f % 10 == 0 {
            trail.push(format!("({nx},{ny})"));
        }
        if hits.contains(&routine::DEATH) {
            outcome = Some(format!("dies at frame {f}"));
            break;
        }
        if hits.contains(&routine::MODAL) {
            outcome = Some(format!("a door, booth or pyramid screen at frame {f}"));
            break;
        }
        if sk_lab::room(&m) != room {
            outcome = Some(format!("room {} at frame {f}", sk_lab::room(&m)));
            break;
        }
    }
    println!(
        "room {room} from ({x},{y}) holding {input}: {}; every ten frames {}",
        outcome.unwrap_or_else(|| "stays".to_string()),
        trail.join(" ")
    );
}
