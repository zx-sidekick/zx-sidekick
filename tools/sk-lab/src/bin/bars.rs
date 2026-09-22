//! The energy, platform and gun bars (#104): what each holds as play starts,
//! the panel's loop that draws them and sets one above
//! [`sidekick::starquake::BAR_FULL`] back to it, and what the platforms and
//! the gun fall to when Blob lays platforms or fires for a while.
//!
//! `bars <assets-dir> [frames]`

use sidekick::machine::Machine;
use sidekick::starquake::at::{BRIDGES, ENERGY, LASER};
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
        bar(&base, BRIDGES),
        bar(&base, LASER)
    );
    // LD B,3; LD HL,LASER; then for each bar: LD A,(HL); CP 7F; JR C,+2;
    // LD (HL),7F; ... draw ...; DEC HL; DJNZ.
    let bytes: Vec<String> = DRAW.map(|a| format!("{:02X}", base.zx.mem[a])).collect();
    println!(
        "the panel's bar loop at {:#06x}: {}",
        DRAW.start,
        bytes.join(" ")
    );

    // Held down lays platforms, and fire shoots: each for `frames` frames,
    // pressed every other frame, with energy kept up so Blob lives through it.
    for (name, input, a) in [
        ("bridging platforms", 0x04, BRIDGES),
        ("laser", 0x10, LASER),
    ] {
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
}
