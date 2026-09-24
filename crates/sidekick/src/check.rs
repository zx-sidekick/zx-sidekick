//! What the games' check tools share (the 2026-09-24 review): reading the assets
//! folder, naming keys, and loading a tape through a real ROM's own loader
//! to find where the game starts. Only the check tools use these; they need
//! the player's tape and ROM, which are never committed.

use std::path::Path;

use zx_spectrum::{CF, Key};

use crate::Machine;

/// Reads `name` from the assets folder `dir`, or exits with why not.
#[must_use]
pub fn read_asset(dir: &Path, name: &str) -> Vec<u8> {
    std::fs::read(dir.join(name)).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {e}", dir.join(name).display());
        std::process::exit(2);
    })
}

/// The key a name stands for, for the checks' scripts.
///
/// # Panics
///
/// If `name` is not a key's: a check script's own mistake.
#[must_use]
pub fn key(name: &str) -> Key {
    Key::by_name(name).unwrap_or_else(|| panic!("{name} is not a key"))
}

/// The ROM's tape loader, LD-BYTES.
const LD_BYTES: u16 = 0x0556;
/// Where LD-BYTES sends its own return, pushed before it loads.
const SA_LD_RET: u16 = 0x053F;

/// Keys typed at a frame: all let go, then these pressed.
pub type Typing<'a> = &'a [(u64, &'a [&'a str])];

/// Dismissing the copyright message, then `LOAD ""` and ENTER.
const LOAD: Typing<'static> = &[
    (150, &["enter"]),
    (160, &[]),
    (200, &["j"]),
    (210, &[]),
    (230, &["symbol", "p"]),
    (240, &[]),
    (260, &["symbol", "p"]),
    (270, &[]),
    (290, &["enter"]),
    (300, &[]),
];

/// Where a real ROM's loader left the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub pc: u16,
    pub sp: u16,
}

/// Boots a Spectrum with `rom`, types `LOAD ""` and then `more` (keys a
/// game's loader waits for), and gives the ROM's loader the tape's blocks
/// in turn, each where it asks, as a Spectrum's tape would. Returns where
/// the processor is once the last block is in, or, with `run_on`, once it
/// has gone on to that address.
///
/// # Errors
///
/// If the tape cannot be split into blocks, if the loader never takes them
/// all within 3,000 frames, or if it never reaches `run_on`.
pub fn through_rom(
    tape: &[u8],
    rom: &[u8],
    more: Typing,
    run_on: Option<u16>,
) -> Result<Loaded, String> {
    let blocks = zx_core::tape::blocks(tape)?;
    let mut m = Machine::<()>::blank(0, 0).with_rom(rom);
    let z = &mut m.zx;
    let typing: Vec<_> = LOAD.iter().chain(more).collect();
    let mut typed = 0;
    let mut next = 0;
    while z.frame < 3000 {
        while typed < typing.len() && typing[typed].0 <= z.frame {
            z.release_all_keys();
            for k in typing[typed].1 {
                z.set_key(key(k), true);
            }
            typed += 1;
        }
        if z.pc() == LD_BYTES && next < blocks.len() {
            let block = blocks[next];
            next += 1;
            let (len, dest) = (z.de() as usize, z.ix());
            z.set_sp(z.sp().wrapping_sub(2));
            z.write16(z.sp(), SA_LD_RET);
            let n = len.min(block.len() - 2);
            // A block of the wrong kind loads nothing, as on a Spectrum;
            // what would land in the ROM is lost.
            if block[0] == z.a() {
                for k in 0..n {
                    let at = dest.wrapping_add(k as u16);
                    if at >= 0x4000 {
                        z.mem[at as usize] = block[1 + k];
                    }
                }
            }
            z.set_ix(dest.wrapping_add(n as u16));
            z.set_de(0);
            z.set_f(z.f() | CF);
            let pc = z.pop();
            z.set_pc(pc);
            if next == blocks.len() {
                if let Some(to) = run_on
                    && !z.run_until_any(&[to], 500)
                {
                    return Err(format!(
                        "the loader stopped at {:04x}, never reaching {to:04x}",
                        z.pc()
                    ));
                }
                return Ok(Loaded {
                    pc: z.pc(),
                    sp: z.sp(),
                });
            }
        }
        let _ = z.run_until_any(&[LD_BYTES], 1);
    }
    Err(format!(
        "the tape never finished loading ({next} of {} blocks)",
        blocks.len()
    ))
}
