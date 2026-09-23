//! Manic Miner's side of ZX Sidekick (#140, #142): the facts about the game
//! (which tape it is, where it starts, where it reads its keys), its
//! [`play::Play`] rules for the shared machine, and the Spectrum ROM's
//! character set it prints with.
//!
//! Nothing in this crate is the game's program. The game runs no ROM code
//! at all; the one thing it takes from the ROM is its character set,
//! [`font::CHARACTER_SET`], placed where the ROM would have it.

pub mod facts;
pub mod font;
pub mod play;

pub use sidekick::machine::{Input, JOY_DOWN, JOY_FIRE, JOY_LEFT, JOY_RIGHT, JOY_UP};

/// The machine running Manic Miner.
pub type Machine = sidekick::Machine<play::Play>;

/// The machine as the game starts, from `tape`, with the ROM's character
/// set where the ROM has it.
///
/// # Errors
///
/// If the tape cannot be read.
pub fn start(tape: &[u8]) -> Result<Machine, String> {
    let mut m = Machine::from_tape(tape, facts::ENTRY_PC, facts::ENTRY_SP)?;
    let at = usize::from(facts::FONT);
    m.zx.mem[at..at + 768].copy_from_slice(&font::CHARACTER_SET);
    Ok(m)
}
