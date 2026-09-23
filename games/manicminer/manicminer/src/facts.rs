//! Facts about Manic Miner (Matthew Smith, Bug-Byte, 1983), as the tape
//! ZX Sidekick supports has it: where things are, never what the game does
//! with them. Each address was read from the tape itself (#140), with
//! Richard Dymond's SkoolKit disassembly of the Bug-Byte release as the map
//! to it.

/// The SHA-1 of the one tape this version supports: the Bug-Byte release.
pub const TAPE_SHA1: &str = "84808c20566aa65e9308c3f8910a16bacfa1b982";

/// The SHA-1 of the 48K ROM the character set is from, which the font check
/// compares it with.
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
    /// A cavern is done: what is left of the air is counted into the score,
    /// calling the air routine from `0x90B4` until it runs out.
    pub const BONUS: u16 = 0x90AD;
    /// The next cavern is set up, once the bonus is counted.
    pub const NEXT_CAVERN: u16 = 0x8691;
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

/// Where training mode steers the game (#148), each at the one instruction
/// the game decides the thing with.
pub mod steer {
    /// `DEC (HL)` on the lives: going on at the next instruction takes none.
    pub const LOSE_LIFE: u16 = 0x8940;
    /// The air routine, which lowers the clock and, when it wraps, the air.
    pub const AIR: u16 = 0x8A3C;
    /// In the air routine, where the bar is drawn from the air and the clock
    /// as they stand, returning "air left".
    pub const AIR_DRAW: u16 = 0x8A52;
    /// Where the air routine returns to from the main loop, and from the
    /// light beam's four calls. The end-of-cavern bonus's call, from
    /// `0x90B4`, is not among them: it counts the air down.
    pub const AIR_FROM: [u16; 5] = [0x87EE, 0x8D8C, 0x8D8F, 0x8D92, 0x8D95];
    /// `JP NC,$8D06` after the fall's `CP $0C`: a fall too long kills.
    pub const FALL_KILL: u16 = 0x8BE2;
    /// Each guardian's call to the sprite routine in its collision mode
    /// (`C` = 1), which returns NZ on touching Willy, and the `JP NZ` after
    /// kills: the horizontal guardians, Eugene, the vertical ones (the
    /// Skylabs among them) and the Kong Beast.
    pub const GUARDIAN_DRAWS: [u16; 4] = [0x8DEB, 0x8E39, 0x8F3B, 0x9206];
    /// The nasty-tile checks' `JP Z,$8D05`.
    pub const NASTY_KILLS: [u16; 2] = [0x9274, 0x927B];
}

/// The game's own cheat (#148): typing 6031769 in play, one digit at a
/// time, then 6 held with keys 1 to 5 spelling a cavern in binary goes to
/// that cavern and starts it again.
pub mod cheat {
    /// The code, as it is typed.
    pub const CODE: [&str; 7] = ["6", "0", "3", "1", "7", "6", "9"];
    /// Where the main loop reads the row 1 to 5, then 6 to 0, to follow the
    /// code; and where, with it entered, it reads 6, then the row 1 to 5,
    /// to teleport.
    pub const READ_LOW: u16 = 0x88BD;
    pub const READ_HIGH: u16 = 0x88DA;
    pub const TELEPORT_SIX: u16 = 0x8887;
    pub const TELEPORT_CAVERN: u16 = 0x8898;
    /// The keyboard half-rows of 1 to 5 and of 6 to 0.
    pub const LOW_ROW: usize = 3;
    pub const HIGH_ROW: usize = 4;
}

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
    /// How much of the cheat has been typed: 7 once it is in.
    pub const CHEAT_COUNT: u16 = 0x845D;
    /// Not zero while the demo plays the caverns (it counts down each
    /// cavern's time), zero in a game.
    pub const DEMO: u16 = 0x845A;
    /// The twenty caverns, 1K each from here, each with its name at offset
    /// 512.
    pub const CAVERNS: u16 = 0xB000;
    /// The cavern being played, copied from the second half of its 1K; the
    /// facts below are in this copy (#153).
    pub const CAVERN_COPY: u16 = 0x8000;
    /// The eight tile kinds, 9 bytes each (attribute, then the graphic), in
    /// the order of [`crate::guide::Tile`].
    pub const TILES: u16 = 0x8020;
    /// The conveyor: its direction (0 left, 1 right), its address in the
    /// screen buffer (two bytes) and its length in cells.
    pub const CONVEYOR: u16 = 0x806F;
    /// The items, 5 bytes each (attribute, address in the attribute buffer,
    /// the screen buffer's high byte, a spare), ending at `0xFF`. An item's
    /// attribute is 0 once it is collected.
    pub const ITEMS: u16 = 0x8075;
    /// The portal's attribute, flashing once every item is taken.
    pub const PORTAL: u16 = 0x808F;
    /// The portal's top left cell, as an address in the attribute buffer.
    pub const PORTAL_CELL: u16 = 0x80B0;
    /// The air's clock, which the air routine lowers by
    /// [`crate::facts::AIR_CLOCK_STEP`] each pass of the main loop; the air
    /// loses a unit when it wraps.
    pub const CLOCK: u16 = 0x80BD;
    /// The horizontal guardians, up to four of 7 bytes, ending at `0xFF`:
    /// attribute (0 for an empty slot), cell address in the attribute buffer
    /// (two bytes), the screen buffer's high byte, frame, then the leftmost
    /// and rightmost cell's low address byte.
    pub const HORIZONTAL: u16 = 0x80BE;
    /// The vertical guardians, up to four of 7 bytes, ending at `0xFF`:
    /// attribute, frame, y in pixels, column, step, then the lowest and
    /// highest y.
    pub const VERTICAL: u16 = 0x80DD;
    /// The attribute buffer the game draws each pass into, 512 cells: the
    /// addresses above are into it.
    pub const ATTRIBUTES: u16 = 0x5C00;
    /// The empty cavern's cells: its attributes without Willy, the
    /// guardians or the items, 512 cells.
    pub const EMPTY_CELLS: u16 = 0x5E00;
    /// The screen buffer the game draws into, laid out as the display file.
    pub const SCREEN_BUFFER: u16 = 0x7000;
}

/// The caverns the main loop treats apart, by its compares of the cavern's
/// number from `0x876F` (#153).
pub mod caverns {
    /// Eugene's own routine (`CP $04` at `0x8772`).
    pub const EUGENE: u8 = 4;
    /// The Kong Beast's own routine (`CP $07` at `0x878A`, `CP $0B` at
    /// `0x8792`).
    pub const KONG_BEAST: [u8; 2] = [7, 11];
    /// The vertical guardians' table is run from this cavern on (`CP $08`
    /// at `0x8782`, `CALL NC,$8EF1`); before it, what is there is not a
    /// guardian table.
    pub const VERTICAL_FROM: u8 = 8;
    /// The Skylabs' own routine instead of the vertical guardians' (`CP $0D`
    /// at `0x877A`): they fall down one column and come back in another.
    pub const SKYLABS: u8 = 13;
    /// The light beam's own routine (`CP $12` at `0x879A`).
    pub const SOLAR: u8 = 18;
}

/// The air when none is left: the air routine reports "no air" when the
/// clock wraps at this (`CP $24` at `0x8A4B`).
pub const AIR_EMPTY: u8 = 0x24;
/// What the air routine takes from the clock each pass (`SUB $04` at
/// `0x8A3F`); the clock wraps to `0xFC`.
pub const AIR_CLOCK_STEP: u8 = 4;

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
