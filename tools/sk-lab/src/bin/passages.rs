//! Every wall passage on the planet walked into from each side Blob fits
//! beside it, on copies of the machine: where he ends up.
//! `passages <assets-dir>`
use sk_lab::rooms::Planet;
use sk_lab::search::Platforms;
use sk_lab::{Args, into_play, stand};
fn main() {
    let args = Args::parse("passages <assets-dir>");
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let mut rows_out = Vec::new();
    for room in 0..512u16 {
        let r = &planet.rooms[usize::from(room)];
        let Some((row, col)) = r.passage else {
            continue;
        };
        let mut sides = Vec::new();
        // (name, Blob's top-left column beside the tile, input held)
        for (name, c, input) in [
            ("from the left", i16::from(col) - 2, 1u8),
            ("from the right", i16::from(col) + 4, 2u8),
        ] {
            if !(0..31).contains(&c) {
                sides.push(format!("{name}: off the room"));
                continue;
            }
            let c = c as u8;
            let fit = (row.saturating_sub(1)..=row + 1).find(|&rr| r.shut.at(rr, c) != 0);
            let Some(rr) = fit else {
                sides.push(format!("{name}: no room to stand"));
                continue;
            };
            let (x, y) = (c * 8, 143 - 8 * (rr - 6));
            let Some(mut m) = stand(&base, room, x, y) else {
                sides.push(format!("{name}: did not settle"));
                continue;
            };
            let mut went = None;
            for f in 0..80 {
                sk_lab::frame(&mut m, input, Platforms::None);
                if sk_lab::room(&m) != room {
                    went = Some((sk_lab::room(&m), f));
                    break;
                }
            }
            sides.push(match went {
                Some((to, f)) => format!("{name}: to {to} after {f}"),
                None => format!("{name}: stays"),
            });
        }
        rows_out.push(format!(
            "room {room:3} passage at ({row},{col}): {}",
            sides.join("; ")
        ));
    }
    for l in rows_out {
        println!("{l}");
    }
}
