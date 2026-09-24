//! The machine ZX Sidekick runs a game on: the Spectrum from `zx-spectrum`
//! (around the `rustzx-z80` processor), started from the player's own tape,
//! with the few Spectrum ROM routines a game calls answered here instead of by
//! a ROM image.
//!
//! Nothing in this crate is any game's program, or any game's facts: each
//! game's own crate says which tape it is, where it starts, and what its
//! [`Rules`] do as it runs (#141).

pub mod check;
pub mod machine;
pub mod print;
pub mod rom;

pub use machine::{Input, Machine, Rules};
