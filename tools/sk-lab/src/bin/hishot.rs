//! Puts a changed high-score table into memory and runs the game's own
//! "CORE OF HEROES" screen (0x654B) on a copy, to see whether the game
//! draws it as its own. Writes `heroes.png`. `hishot <assets-dir>`
use sk_lab::raster::Image;
use sk_lab::{Args, into_play};
use zx_core::screen;
fn main() {
    let args = Args::parse("hishot <assets-dir>");
    let mut m = into_play(&args.tape());
    m.zx.mem[0x64FA..0x6503].copy_from_slice(b"ZZZ123456");
    m.zx.mem[0x6522..0x652B].copy_from_slice(b"QQQ047000");
    let mut shots: Vec<Vec<u8>> = Vec::new();
    let mut n = 0u64;
    m.call_observing(0x654B, 0x5E81, 40_000_000, |z| {
        n += 1;
        if n.is_multiple_of(500_000) && shots.len() < 12 {
            shots.push(z.mem[0x4000..0x5B00].to_vec());
        }
    });
    let (w, h) = (screen::WIDTH, screen::HEIGHT);
    let mut img = Image::new(4 * (w + 4), 3 * (h + 4), 0x404040);
    let mut px = vec![0u32; w * h];
    for (i, mem) in shots.iter().enumerate() {
        screen::render(&mem[..0x1800], &mem[0x1800..], false, &mut px, w, 0, |p| p);
        let (ox, oy) = ((i % 4) * (w + 4), (i / 4) * (h + 4));
        for y in 0..h {
            for x in 0..w {
                img.set((ox + x) as i64, (oy + y) as i64, px[y * w + x]);
            }
        }
    }
    std::fs::write(args.path("heroes.png"), img.png()).unwrap();
    println!("ran {n} instructions");
}
