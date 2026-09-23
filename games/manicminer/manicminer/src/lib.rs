//! Manic Miner's side of ZX Sidekick (#140, #142): the facts about the game
//! (which tape it is, where it starts, where it reads its keys), its
//! [`play::Play`] rules for the shared machine, and the font it prints with
//! when the player has no ROM.
//!
//! Nothing in this crate is the game's program. The game runs no ROM code
//! at all; the one thing it takes from a ROM is its character set, which the
//! player's own ROM supplies, or else our [`font::OURS`].

pub mod facts;
pub mod font;
pub mod play;

pub use sidekick::machine::{Input, JOY_DOWN, JOY_FIRE, JOY_LEFT, JOY_RIGHT, JOY_UP};

/// The machine running Manic Miner.
pub type Machine = sidekick::Machine<play::Play>;

/// The machine as the game starts, from `tape`, with `font` where the ROM's
/// character set would be.
///
/// # Errors
///
/// If the tape cannot be read.
pub fn start(tape: &[u8], font: &[u8; 768]) -> Result<Machine, String> {
    let mut m = Machine::from_tape(tape, facts::ENTRY_PC, facts::ENTRY_SP)?;
    let at = usize::from(facts::FONT);
    m.zx.mem[at..at + 768].copy_from_slice(font);
    Ok(m)
}
