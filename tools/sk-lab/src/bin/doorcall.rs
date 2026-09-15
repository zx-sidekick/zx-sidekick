//! Which of the calls to the modal routine a security door makes, and what
//! the code at 0xD5F4 holds when play resumes. `doorcall <assets-dir>`
use sidekick::starquake::routine;
use sk_lab::search::Platforms;
use sk_lab::{Args, into_play, stand};
fn main() {
    let args = Args::parse("doorcall <assets-dir>");
    let base = into_play(&args.tape());
    let sites = [0xCBEAu16, 0xCCF9, 0xCD27, 0xCED1, 0xD5F8, 0xD5FA];
    for (label, room, x, y, input) in [("door 210", 210u16, 128u8, 63u8, 1u8)] {
        let Some(mut m) = stand(&base, room, x, y) else {
            println!("no stand");
            continue;
        };
        m.watch = sites
            .iter()
            .copied()
            .chain([routine::MODAL, routine::MAIN_LOOP, routine::ENTER_ROOM])
            .collect();
        let mut seen = Vec::new();
        for f in 0..2000 {
            let hits = sk_lab::frame(&mut m, if f < 300 { input } else { 0 }, Platforms::None);
            for h in hits {
                if seen.last() != Some(&h) {
                    seen.push(h);
                }
            }
            if seen.contains(&routine::ENTER_ROOM) && f > 300 {
                break;
            }
        }
        let c = &m.zx.mem[0xD5F4..0xD5FE];
        println!(
            "{label}: hits in order {:04x?}; code bytes {:02x?}; entry reason {}",
            seen.iter()
                .filter(|&&h| h != routine::MAIN_LOOP)
                .collect::<Vec<_>>(),
            c,
            m.zx.mem[usize::from(sidekick::starquake::at::ENTRY_REASON)]
        );
    }
}
