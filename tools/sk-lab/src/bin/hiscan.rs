//! Looks for the high-score table: printable text runs in memory once the
//! game is on its title screen, with the bytes around them.
//!
//! `hiscan <assets-dir>`
use sidekick::starquake::routine;
use sk_lab::{Args, into_play};
fn main() {
    let args = Args::parse("hiscan <assets-dir>");
    let m = into_play(&args.tape());
    let mem = &m.zx.mem;
    let printable = |b: u8| (0x20..0x7F).contains(&(b & 0x7F));
    let mut a = 0x5B00usize;
    while a < 0x10000 {
        let start = a;
        while a < 0x10000 && printable(mem[a]) {
            a += 1;
        }
        if a - start >= 6 {
            let text: String = mem[start..a].iter().map(|&b| (b & 0x7F) as char).collect();
            if text.chars().filter(char::is_ascii_alphabetic).count() >= 4 {
                println!("{start:04x}..{a:04x} {text:?}");
            }
        }
        a += 1;
    }
    let _ = routine::MENU;
}
