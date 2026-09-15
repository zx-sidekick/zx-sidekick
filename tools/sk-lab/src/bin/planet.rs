//! The whole planet as one picture, `planet.png` in the assets folder:
//! every room as the game draws it, with its openings, inner walls, doors,
//! wall passages and lifts drawn over it, the start room and the core room
//! outlined, and rooms not reachable from the start with doors shut dimmed.
//!
//! `planet <assets-dir> [--half]`

use sidekick::map::{COLS, ROWS};
use sidekick::starquake::{CORE_ROOM, read_room};
use sk_lab::raster::Image;
use sk_lab::rooms::{Planet, lift};
use sk_lab::{Args, into_play};
use zx_core::screen;

const ROOM_W: usize = 256;
const ROOM_H: usize = 144;
const GAP: usize = 12;
const MARGIN: usize = 32;

const BG: u32 = 0x0B0D12;
const OPEN: u32 = 0x2EE6A6;
const WALL: u32 = 0xFF9F43;
const DOOR: u32 = 0xFFE066;
const PASSAGE: u32 = 0xD66BFF;
const LIFT: u32 = 0x3CB043;
const NUMBER: u32 = 0xE0E4EC;
const START: u32 = 0xFFFFFF;
const CORE: u32 = 0xFF5C8A;

/// A point on a room's edge, measured clockwise from its top-left corner in
/// cells, as `map::Wall::to` has it.
fn edge_point(x: f64, y: f64, to: u8) -> (f64, f64) {
    let (w, h) = (ROOM_W as f64, ROOM_H as f64);
    let (top, side) = (32.0, 18.0);
    let t = f64::from(to);
    if t < top {
        (x + w * t / top, y)
    } else if t < top + side {
        (x + w, y + h * (t - top) / side)
    } else if t < 2.0 * top + side {
        (x + w - w * (t - top - side) / top, y + h)
    } else {
        (x, y + h - h * (t - 2.0 * top - side) / side)
    }
}

fn dim(colour: u32) -> u32 {
    let (r, g, b) = (colour >> 16 & 0xFF, colour >> 8 & 0xFF, colour & 0xFF);
    (r * 2 / 5) << 16 | (g * 2 / 5) << 8 | (b * 2 / 5)
}

fn main() {
    let args = Args::parse("planet <assets-dir> [--half]");
    let base = into_play(&args.tape());
    let start = sk_lab::room(&base);
    let planet = Planet::read(&base);
    let (bx, by) = sk_lab::blob(&base);
    let reach = planet.reach(start, sk_lab::top_row(by), bx >> 3);
    eprintln!(
        "start room {start}; {} rooms reachable from it with doors shut",
        reach.len()
    );

    let width = MARGIN * 2 + 16 * (ROOM_W + GAP) - GAP;
    let height = MARGIN * 2 + 32 * (ROOM_H + GAP) - GAP;
    let mut img = Image::new(width, height, BG);
    let mut screen_px = vec![0u32; screen::WIDTH * screen::HEIGHT];
    for room in 0..COLS * ROWS {
        let (c, r) = (usize::from(room % COLS), usize::from(room / COLS));
        let (x0, y0) = (
            (MARGIN + c * (ROOM_W + GAP)) as i64,
            (MARGIN + r * (ROOM_H + GAP)) as i64,
        );
        let near = reach.contains(&room);
        // The room's own pixels, from the display memory the room builder
        // filled on a copy: the play area is screen rows 48 to 191.
        let mut m = base.clone();
        read_room(&mut m, room);
        let mem = &m.zx.mem;
        screen::render(
            &mem[0x4000..0x5800],
            &mem[0x5800..0x5B00],
            false,
            &mut screen_px,
            screen::WIDTH,
            0,
            |p| p,
        );
        for y in 0..ROOM_H {
            for x in 0..ROOM_W {
                let p = screen_px[(y + 48) * screen::WIDTH + x];
                img.set(x0 + x as i64, y0 + y as i64, if near { p } else { dim(p) });
            }
        }
        let cells = &planet.cells[usize::from(room)];
        for (row, line) in cells.iter().enumerate() {
            for (col, &a) in line.iter().enumerate() {
                if lift(a) {
                    let (x, y) = (x0 + col as i64 * 8, y0 + row as i64 * 8);
                    img.outline(x, y, 8, 8, 1, LIFT);
                }
            }
        }
        // Openings: a bar along the edge wherever a Blob-sized window of free
        // cells touches it.
        let fits = |row: usize, col: usize| planet.fits(room, row, col);
        for col in 0..31 {
            if fits(0, col) {
                img.fill(x0 + col as i64 * 8, y0 - 5, 16, 4, OPEN);
            }
            if fits(16, col) {
                img.fill(x0 + col as i64 * 8, y0 + ROOM_H as i64 + 1, 16, 4, OPEN);
            }
        }
        for row in 0..17 {
            if fits(row, 0) {
                img.fill(x0 - 5, y0 + row as i64 * 8, 4, 16, OPEN);
            }
            if fits(row, 30) {
                img.fill(x0 + ROOM_W as i64 + 1, y0 + row as i64 * 8, 4, 16, OPEN);
            }
        }
        // A wall passage: its tile outlined, and a bar on each side whose
        // neighbour has a passage too.
        let has_passage = |r: u16| planet.rooms[usize::from(r)].passage.is_some();
        if let Some((prow, pcol)) = planet.rooms[usize::from(room)].passage {
            let (px, py) = (x0 + i64::from(pcol) * 8, y0 + (i64::from(prow) - 6) * 8);
            img.outline(px, py, 32, 24, 2, PASSAGE);
            if room % COLS > 0 && has_passage(room - 1) {
                img.fill(x0 - 5, py, 4, 24, PASSAGE);
            }
            if room % COLS < COLS - 1 && has_passage(room + 1) {
                img.fill(x0 + ROOM_W as i64 + 1, py, 4, 24, PASSAGE);
            }
        }
        // Walls inside a divided room, from the centre out; a door's dashed.
        let (cx, cy) = (
            x0 as f64 + ROOM_W as f64 / 2.0,
            y0 as f64 + ROOM_H as f64 / 2.0,
        );
        for wall in planet.openings[usize::from(room)]
            .walls
            .into_iter()
            .flatten()
        {
            let (ex, ey) = edge_point(x0 as f64, y0 as f64, wall.to);
            let colour = if wall.door { DOOR } else { WALL };
            img.line(cx, cy, ex, ey, 3, wall.door, colour);
        }
        img.fill(x0 + 3, y0 + 3, 4 * 2 * 3 + 2, 5 * 2 + 4, 0x000000);
        img.number(x0 + 5, y0 + 5, room, 2, NUMBER);
        if room == start || room == CORE_ROOM {
            let colour = if room == start { START } else { CORE };
            img.outline(
                x0 - 8,
                y0 - 8,
                ROOM_W as i64 + 16,
                ROOM_H as i64 + 16,
                3,
                colour,
            );
        }
    }
    let out = if args.flag("half") {
        let mut half = Image::new(width / 2, height / 2, BG);
        for y in 0..height / 2 {
            for x in 0..width / 2 {
                // The brightest of the four, so thin lines survive.
                let px = |dx: usize, dy: usize| img.pixels[(2 * y + dy) * width + 2 * x + dx];
                let best = [px(0, 0), px(1, 0), px(0, 1), px(1, 1)]
                    .into_iter()
                    .max_by_key(|&p| (p >> 16 & 0xFF) + (p >> 8 & 0xFF) + (p & 0xFF))
                    .unwrap_or(BG);
                half.set(x as i64, y as i64, best);
            }
        }
        half
    } else {
        img
    };
    let path = args.path("planet.png");
    std::fs::write(&path, out.png()).unwrap_or_else(|e| {
        eprintln!("cannot write {}: {e}", path.display());
        std::process::exit(2);
    });
    eprintln!(
        "wrote {} ({} x {}): green bars are openings, orange lines walls inside a room, yellow dashed a door, purple a wall passage, green boxes lift cells; white outline the start, pink the core; dimmed rooms are not reachable from the start with doors shut",
        path.display(),
        out.width,
        out.height
    );
}
