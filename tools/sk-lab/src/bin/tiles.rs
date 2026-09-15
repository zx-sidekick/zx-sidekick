//! Rooms as text, side by side: `#` solid, `.` free, `=` a lift; and with
//! `--png`, each room as the game draws it, `room-N.png` in the assets
//! folder, or with `--stack` all of them in one picture top to bottom,
//! `rooms-N-M.png`.
//!
//! `tiles <assets-dir> <room>[,<room>...] [--png] [--stack]`

use sidekick::starquake::read_room;
use sk_lab::raster::Image;
use sk_lab::rooms::{Planet, draw};
use sk_lab::{Args, into_play};
use zx_core::screen;

fn main() {
    let args = Args::parse("tiles <assets-dir> <room>[,<room>...] [--png] [--stack]");
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let rooms = args.rooms(0);
    let grids: Vec<Vec<String>> = rooms
        .iter()
        .map(|&r| {
            let o = planet.openings[usize::from(r)];
            let mut g = vec![format!(
                "room {r}: L{} R{} U{} D{} passage {:?}",
                u8::from(o.left),
                u8::from(o.right),
                u8::from(o.up),
                u8::from(o.down),
                planet.rooms[usize::from(r)].passage
            )];
            g.extend(draw(&planet.cells[usize::from(r)]));
            g
        })
        .collect();
    if let Some(first) = grids.first() {
        for i in 0..first.len() {
            let line: Vec<String> = grids.iter().map(|g| format!("{:<40}", g[i])).collect();
            println!("{}", line.join(" ").trim_end());
        }
    }
    if args.flag("png") || args.flag("stack") {
        let stack = args.flag("stack");
        let mut px = vec![0u32; screen::WIDTH * screen::HEIGHT];
        let (w, h) = (screen::WIDTH * 2, 144 * 2);
        let gap = if stack { 8 } else { 0 };
        let mut img = Image::new(
            w,
            if stack {
                rooms.len() * (h + gap) - gap
            } else {
                h
            },
            0x202020,
        );
        for (i, &r) in rooms.iter().enumerate() {
            let mut m = base.clone();
            read_room(&mut m, r);
            let mem = &m.zx.mem;
            screen::render(
                &mem[0x4000..0x5800],
                &mem[0x5800..0x5B00],
                false,
                &mut px,
                screen::WIDTH,
                0,
                |p| p,
            );
            let top = if stack { (i * (h + gap)) as i64 } else { 0 };
            for y in 0..h {
                for x in 0..w {
                    img.set(
                        x as i64,
                        top + y as i64,
                        px[(y / 2 + 48) * screen::WIDTH + x / 2],
                    );
                }
            }
            if !stack {
                write(&args, &format!("room-{r}.png"), &img);
            }
        }
        if stack && let (Some(first), Some(last)) = (rooms.first(), rooms.last()) {
            write(&args, &format!("rooms-{first}-{last}.png"), &img);
        }
    }
}

fn write(args: &Args, name: &str, img: &Image) {
    let path = args.path(name);
    std::fs::write(&path, img.png()).unwrap_or_else(|e| {
        eprintln!("cannot write {}: {e}", path.display());
        std::process::exit(2);
    });
    eprintln!("wrote {}", path.display());
}
