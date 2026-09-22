//! The energy, platform and gun bars (#104): what each holds as play starts,
//! the panel's loop that draws them and sets one above
//! [`sidekick::starquake::BAR_FULL`] back to it, and what the platforms and
//! the gun fall to when Blob lays platforms or fires for a while.
//!
//! `bars <assets-dir> [frames]`

use sidekick::machine::Machine;
use sidekick::starquake::at::{ENERGY, GUN, PLATFORMS};
use sk_lab::{Args, into_play};

/// Where the panel's bar loop starts, and the byte after its `DJNZ`.
const DRAW: std::ops::Range<usize> = 0xD463..0xD4A4;

fn main() {
    let args = Args::parse("bars <assets-dir> [frames]");
    let frames: u64 = args.get(0, 600);
    let base = into_play(&args.tape());
    let bar = |m: &Machine, a: u16| m.zx.mem[usize::from(a)];
    println!(
        "as play starts: energy {}, platforms {}, gun {}",
        bar(&base, ENERGY),
        bar(&base, PLATFORMS),
        bar(&base, GUN)
    );
    // LD B,3; LD HL,GUN; then for each bar: LD A,(HL); CP 7F; JR C,+2;
    // LD (HL),7F; ... draw ...; DEC HL; DJNZ.
    let bytes: Vec<String> = DRAW.map(|a| format!("{:02X}", base.zx.mem[a])).collect();
    println!(
        "the panel's bar loop at {:#06x}: {}",
        DRAW.start,
        bytes.join(" ")
    );

    // Held down lays platforms, and fire shoots: each for `frames` frames,
    // pressed every other frame, with energy kept up so Blob lives through it.
    for (name, input, a) in [("platforms", 0x04, PLATFORMS), ("gun", 0x10, GUN)] {
        let mut m = base.clone();
        let mut seen = vec![bar(&m, a)];
        for f in 0..frames {
            m.zx.release_all_keys();
            m.zx.kempston = if f % 2 == 0 { input } else { 0 };
            m.zx.mem[usize::from(ENERGY)] = sidekick::starquake::BAR_FULL;
            m.run_frame();
            if seen.last() != Some(&bar(&m, a)) {
                seen.push(bar(&m, a));
            }
        }
        println!("{name} over {frames} frames: {seen:?}");
    }

    // SCRATCH: run both bars down, switch full on, and see whether the
    // panel's pixels follow.
    let mut m = base.clone();
    m.training = sidekick::machine::Training {
        time: true,
        unharmed: true,
        ..Default::default()
    };
    for f in 0..4000u32 {
        if m.zx.mem[usize::from(PLATFORMS)] == 0 && m.zx.mem[usize::from(GUN)] == 0 {
            break;
        }
        m.zx.release_all_keys();
        m.zx.kempston = if f % 2 == 0 { 0x14 } else { 0 };
        m.run_frame();
    }
    m.zx.kempston = 0;
    for _ in 0..30 {
        m.run_frame();
    }
    let shot = |m: &Machine, name: &str| {
        let mut px = vec![0u32; zx_core::screen::WIDTH * zx_core::screen::HEIGHT];
        zx_core::screen::render(
            &m.zx.mem[0x4000..0x5800],
            &m.zx.mem[0x5800..0x5B00],
            false,
            &mut px,
            zx_core::screen::WIDTH,
            0,
            |p| p,
        );
        std::fs::write(
            args.path(name),
            zx_core::png::encode(&px, zx_core::screen::WIDTH, zx_core::screen::HEIGHT),
        )
        .unwrap();
        px
    };
    let empty = shot(&m, "bars-1-empty.png");
    m.training = sidekick::machine::Training {
        full: true,
        ..Default::default()
    };
    for _ in 0..5 {
        m.run_frame();
    }
    let after5 = shot(&m, "bars-2-on-5-frames.png");
    for _ in 0..300 {
        m.run_frame();
    }
    let after300 = shot(&m, "bars-3-on-305-frames.png");
    let top = 48 * zx_core::screen::WIDTH;
    let diff = |a: &[u32], b: &[u32]| {
        a[..top]
            .iter()
            .zip(&b[..top])
            .filter(|(x, y)| x != y)
            .count()
    };
    println!(
        "panel pixels changed: 5 frames on {}, 305 frames on {}; bars now {} {}",
        diff(&empty, &after5),
        diff(&empty, &after300),
        m.zx.mem[usize::from(PLATFORMS)],
        m.zx.mem[usize::from(GUN)]
    );
    // One platform laid and one shot fired with it on.
    for f in 0..4 {
        m.zx.kempston = if f % 2 == 0 { 0x14 } else { 0 };
        m.run_frame();
    }
    m.zx.kempston = 0;
    for _ in 0..5 {
        m.run_frame();
    }
    let used = shot(&m, "bars-4-after-use.png");
    println!(
        "after using both: panel pixels changed vs empty {}",
        diff(&empty, &used)
    );
}

#[allow(dead_code)]
fn scratch() {}
