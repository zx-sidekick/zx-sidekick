//! Dumps memory as hex and text: `hidump <assets-dir> <from-hex> <to-hex>`.
use sk_lab::{Args, into_play};
fn main() {
    let args = Args::parse("hidump <assets-dir> <from> <to>");
    let from = usize::from_str_radix(&args.rest[0], 16).unwrap();
    let to = usize::from_str_radix(&args.rest[1], 16).unwrap();
    let m = into_play(&args.tape());
    for a in (from..to).step_by(16) {
        let row = &m.zx.mem[a..a + 16];
        let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
        let txt: String = row
            .iter()
            .map(|&b| {
                let c = b & 0x7F;
                if (0x20..0x7F).contains(&c) {
                    c as char
                } else {
                    '.'
                }
            })
            .collect();
        println!("{a:04x}  {}  {txt}", hex.join(" "));
    }
}
