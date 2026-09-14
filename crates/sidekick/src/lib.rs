//! The machine ZX Sidekick runs a game on: the Spectrum from `zx-spectrum`
//! (around the `rustzx-z80` processor), started from the player's own tape,
//! with the few Spectrum ROM routines a game calls answered here instead of by
//! a ROM image.
//!
//! Nothing in this crate is any game's program. What it knows about a game
//! is facts: which tape it is, where it starts, which ROM routines it calls,
//! and where it keeps what the guidance panel shows. `map` reads a room's
//! openings from the cells the game itself drew.

pub mod machine;
pub mod map;
pub mod print;
pub mod rom;
pub mod starquake;

pub use machine::{Input, Machine};
