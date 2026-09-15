//! Where the program names an address: every place the two bytes of `addr`
//! (little-endian) appear, with the byte before (the opcode).
//! `refs <assets-dir> <addr-hex>...`
use sk_lab::{Args, into_play};
fn main() {
    let args = Args::parse("refs <assets-dir> <addr>...");
    let m = into_play(&args.tape());
    let mem = &m.zx.mem;
    for a in &args.rest {
        let v = u16::from_str_radix(a, 16).unwrap();
        let [lo, hi] = v.to_le_bytes();
        let hits: Vec<String> = (0x5B01..0xFFFF)
            .filter(|&i| mem[i] == lo && mem[i + 1] == hi)
            .map(|i| format!("{:04x}(op {:02x})", i - 1, mem[i - 1]))
            .collect();
        println!("{a}: {}", hits.join(" "));
    }
}
