//! The security door screen as the game draws it, beside the graphics table:
//! `doorshot <assets-dir> <room> <x> <y> [input] [--carry=G,...]` writes
//! `door-<room>.png` (the screen some frames after it opens) and
//! `graphics.png` (graphics 0 to 47, numbered).
use sidekick::starquake::{graphic, routine};
use sk_lab::raster::Image;
use sk_lab::search::Platforms;
use sk_lab::{Args, CODE, carry, into_play, stand};
use zx_core::screen;

fn shot(m: &sidekick::Machine) -> Image {
    let mut px = vec![0u32; screen::WIDTH * screen::HEIGHT];
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
    let mut img = Image::new(screen::WIDTH * 3, screen::HEIGHT * 3, 0);
    for y in 0..screen::HEIGHT {
        for x in 0..screen::WIDTH {
            img.fill(x as i64 * 3, y as i64 * 3, 3, 3, px[y * screen::WIDTH + x]);
        }
    }
    img
}

fn main() {
    let args = Args::parse("doorshot <assets-dir> <room> <x> <y> [input] [--carry=G,...]");
    let mut base = into_play(&args.tape());
    let room: u16 = args.get(0, 210);
    let (x, y): (u8, u8) = (args.get(1, 128), args.get(2, 63));
    let input: u8 = args.get(3, 1);
    let carried: String = args.value("carry", String::new());
    for g in carried.split(',').filter_map(|g| g.parse::<u8>().ok()) {
        carry(&mut base, g);
    }
    let mut sheet = Image::new(8 * 60 + 10, 6 * 70 + 10, 0x202020);
    for n in 0..48u8 {
        let g = graphic(&base.zx.mem[..], n);
        let (ox, oy) = (10 + i64::from(n % 8) * 60, 10 + i64::from(n / 8) * 70);
        for (cell, (cy, cx)) in [(0, 0), (0, 8), (8, 0), (8, 8)].into_iter().enumerate() {
            for row in 0..8 {
                let b = g[cell * 8 + row];
                for bit in 0..8 {
                    if b & (0x80 >> bit) != 0 {
                        sheet.fill(
                            ox + (cx + bit) as i64 * 3,
                            oy + (cy + row) as i64 * 3,
                            3,
                            3,
                            0xFFFFFF,
                        );
                    }
                }
            }
        }
        sheet.number(ox, oy + 52, u16::from(n), 2, 0xFFD060);
    }
    std::fs::write(args.path("graphics.png"), sheet.png()).unwrap();
    let Some(mut m) = stand(&base, room, x, y) else {
        println!("did not settle");
        return;
    };
    let mut open = false;
    for _ in 0..300 {
        if sk_lab::frame(&mut m, input, Platforms::Unlimited).contains(&routine::MODAL) {
            open = true;
            break;
        }
    }
    if !open {
        println!("no screen");
        return;
    }
    for f in 0..400 {
        sk_lab::frame(&mut m, 0, Platforms::Unlimited);
        if f == 60 || f == 150 {
            std::fs::write(args.path(&format!("door-{room}-{f}.png")), shot(&m).png()).unwrap();
        }
    }
    let code = usize::from(CODE);
    let mem = &m.zx.mem;
    println!(
        "wanted {:?}",
        (0..usize::from(mem[code + 2]))
            .map(|i| mem[code + 3 + i * 2])
            .collect::<Vec<_>>()
    );
}
