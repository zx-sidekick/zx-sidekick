//! Starquake's side of ZX Sidekick: the facts about the game (which tape it
//! is, where it starts, where it keeps what the guidance panel shows), the
//! planet's map as the game itself draws it, and its [`play::Play`] rules
//! for the shared machine (#141).
//!
//! Nothing in this crate is the game's program. What it knows about the game
//! is facts, and what it does is steer the machine at the game's own points.

pub mod facts;
pub mod map;
pub mod play;

pub use sidekick::machine::{Input, JOY_DOWN, JOY_FIRE, JOY_LEFT, JOY_RIGHT, JOY_UP};

/// The machine running Starquake.
pub type Machine = sidekick::Machine<play::Play>;
