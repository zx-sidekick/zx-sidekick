//! Facts about Manic Miner (Matthew Smith, Bug-Byte, 1983), as the tape
//! ZX Sidekick supports has it: where things are, never what the game does
//! with them. Each address was read from the tape itself (#140), with
//! Richard Dymond's SkoolKit disassembly of the Bug-Byte release as the map
//! to it.

/// The SHA-1 of the one tape this version supports: the Bug-Byte release.
pub const TAPE_SHA1: &str = "84808c20566aa65e9308c3f8910a16bacfa1b982";

/// The SHA-1 of the 48K ROM whose character set may be used for the text.
pub const ROM_SHA1: &str = "5ea7c2b824672e914525d1d5c419d71b84a426a2";

/// Where the game starts: its BASIC loader ends with `RANDOMIZE USR 33792`.
/// The game sets its own stack at once, and never enables interrupts.
pub const ENTRY_PC: u16 = 0x8400;
/// The stack as `USR` leaves it, which `manicminer-check entry` measures
/// against a real ROM's loader.
pub const ENTRY_SP: u16 = 0x7519;

/// Where the ROM's character set is, which the game prints its text from,
/// eight bytes a letter from code 32: the only ROM bytes it ever touches.
pub const FONT: u16 = 0x3D00;

/// The game's own routines, as the checks follow them.
pub mod routine {
    /// The title screen, with its tune; quitting a game comes back here.
    pub const TITLE: u16 = 0x85CC;
    /// A new game (or the demo) starts: the score is cleared.
    pub const NEW_GAME: u16 = 0x8684;
    /// The top of the main loop, once a cavern is in play.
    pub const MAIN_LOOP: u16 = 0x870E;
    /// A life is taken, when one is left.
    pub const LOSE_LIFE: u16 = 0x8940;
    /// The game-over sequence, when none is left.
    pub const GAME_OVER: u16 = 0x8944;
}

/// Where the game reads keys, each at the instruction that reads the port.
pub mod reads {
    /// The title screen reads ENTER, which starts a game.
    pub const TITLE_ENTER: u16 = 0x866C;
    /// The tune reads ENTER, which ends it (fire does too, with a Kempston
    /// joystick).
    pub const TUNE_ENTER: u16 = 0x9345;
    /// The main loop reads the row A to G, which pauses the game; it then
    /// waits in its own loop until another key.
    pub const PAUSE: u16 = 0x8803;
}

/// The keyboard half-row the pause keys are on (A, S, D, F, G), and its
/// bits.
pub const PAUSE_KEYS: (usize, u8) = (1, 0x1F);

/// Where the game keeps what the checks read.
pub mod at {
    /// The cavern being played, 0 to 19.
    pub const CAVERN: u16 = 0x8407;
    /// Lives left besides the one being played.
    pub const LIVES: u16 = 0x8457;
    /// 1 when a Kempston joystick answered on port 0x1F at the title.
    pub const KEMPSTON: u16 = 0x8459;
    /// Willy's cell in the attribute buffer (two bytes).
    pub const WILLY_CELL: u16 = 0x806C;
    /// Willy's airborne state: 0 standing, 1 jumping, 2 to 11 falling
    /// safely, 12 and up too far, 255 killed.
    pub const AIRBORNE: u16 = 0x806B;
    /// The air left, 36 to 63.
    pub const AIR: u16 = 0x80BC;
    /// The cavern's name in the working buffer, 32 characters.
    pub const CAVERN_NAME: u16 = 0x8000;
}

/// Whether `bytes` are the tape these facts are about.
#[must_use]
pub fn is_supported_tape(bytes: &[u8]) -> bool {
    zx_core::sha1::sha1_hex(bytes) == TAPE_SHA1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_known_tape_is_supported() {
        assert!(!is_supported_tape(b"another tape"));
        assert_eq!(TAPE_SHA1.len(), 40);
        assert_eq!(ROM_SHA1.len(), 40);
    }
}
