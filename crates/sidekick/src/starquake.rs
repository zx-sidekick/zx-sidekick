//! Facts about Starquake (Stephen Crow / Bubble Bus, 1985). Addresses and
//! values only: nothing here is the game's program.

use zx_spectrum::Key;

/// SHA-1 of the one tape this version knows the facts of.
pub const TAPE_SHA1: &str = "65450d6f33692c2c2868c0b497037f2cfd0ef3bd";

/// Where the game starts once its code block has loaded. The block covers all
/// of RAM, the stack included, and the ROM's loader returns through the
/// address the block leaves on its stack: here. Found by running the real
/// loader with a ROM, once, in development (`docs/rom.md`).
pub const ENTRY_PC: u16 = 0x5E24;
/// The stack pointer at that moment.
pub const ENTRY_SP: u16 = 0x5E20;
/// `IY` as the ROM keeps it, pointing into its system variables.
pub const ENTRY_IY: u16 = 0x5C3A;
/// The interrupt vector register the ROM sets at start-up.
pub const ENTRY_I: u8 = 0x3F;

/// The control method chosen on the title screen, 1 to 5 as the screen
/// numbers them: Kempston joystick, cursor joystick, Sinclair joystick,
/// keyboard, user-defined keys. Found on 2026-09-14 by choosing each option
/// on the player's tape and looking at what changed.
pub const CONTROL_METHOD: u16 = 0x5E58;

/// The four key tables the game reads in methods 2 to 5, five bytes each in
/// the order left, right, down, up, fire, each byte a key as [`key`] reads
/// it. The tape ships them as `5 8 6 7 0`, `1 2 3 4 5`, `O P A Q M` and
/// `Q W E R T`; the define-keys screen rewrites the last. Method 1 reads the
/// Kempston port instead.
pub const KEY_TABLES: u16 = 0x5E5C;

/// The pause key, as [`key`] reads it: Space as the tape ships it, and
/// whatever the define-keys screen was given after that. Methods 2 to 5
/// pause with it; the Kempston method pauses with Space whatever it holds.
/// Checked by hand and on the tape on 2026-09-14, in every method.
pub const PAUSE_KEY: u16 = 0x5E70;

/// The game's play-time key reader: the one routine that consults
/// [`CONTROL_METHOD`] and the tables, run once a frame during play and not
/// at all on the title screen (which reads the keyboard at `0xDA2B`) or the
/// define-keys screen (`0xD5D4`). It starts by reading the pause key. A key
/// pressed as the program counter arrives here reaches the game in play and
/// nowhere else. Found on 2026-09-14 by tracing every `IN` the game runs on
/// each screen.
pub const PLAY_INPUT: u16 = 0xC55D;

/// Where the same routine goes on to read the directions and fire, after
/// the pause key; and where a paused game waits, looping back here without
/// the pause read until a direction or fire resumes it. A key pressed only
/// at [`PLAY_INPUT`] never reaches a paused game. Found on 2026-09-14 by
/// tracing a paused game on the player's tape, in every control method.
/// The machine keeps the pause key from the game between the two, so the
/// game never pauses itself, and gives it back here.
pub const CONTROLS_INPUT: u16 = 0xC566;

/// The key read the title screen waits on until its first key, and the
/// intro text waits on for its any-key: every half-row, and on the title
/// screen the digits choose while `0` starts a game. Once a key has been
/// pressed the title screen reads through [`MENU_KEY`] instead, with its
/// highlight cycling between reads. Found on 2026-09-14 by tracing every
/// `IN` the game runs on each screen.
pub const MENU_INPUT: u16 = 0xDA2B;

/// The keyboard-reading routine the title screen calls, once a key has been
/// pressed, to take the next choice, and calls again a few frames after a
/// choice to confirm the key is still down. The define-keys screen reads its
/// keys through the same routine, from a different place: the two are told
/// apart by the return address on the stack as the routine is entered,
/// [`MENU_KEY_FROM_TITLE`] for the title screen. Found on 2026-09-14 by
/// tracing the calls leading to each key read on the player's tape.
pub const MENU_KEY: u16 = 0xD5C8;

/// The return address on the stack when the title screen calls
/// [`MENU_KEY`]: the instruction after its `CALL` at `0x6021`. The
/// define-keys screen's call returns to `0x625A`.
pub const MENU_KEY_FROM_TITLE: u16 = 0x6024;

/// Entry points of the game's routines that say what the program is doing,
/// for the panel beside it: which part of the program is running, and when
/// End this game may hold its keys. Found by studying the program in the
/// earlier ZX Sidekick build, and checked on the player's tape by
/// `sk-check facts`.
pub mod routine {
    /// The title screen's menu.
    pub const MENU: u16 = 0x5E81;
    /// The top of the play loop, once a frame while Blob is being played.
    pub const MAIN_LOOP: u16 = 0xA523;
    /// Where the play loop hands over to a security door, a teleporter booth
    /// or the pyramid, each of which then runs as a screen of its own.
    pub const MODAL: u16 = 0xA412;
    /// Blob losing a life.
    pub const DEATH: u16 = 0xC350;
    /// The end of a game: the scores, entering initials, the high-score
    /// table.
    pub const GAME_OVER: u16 = 0x6730;
    /// Setting up a new game.
    pub const NEW_GAME: u16 = 0x629D;
    /// Entering a room, up to the play loop.
    pub const ENTER_ROOM: u16 = 0xA426;
    /// A teleporter booth, one of the screens play hands over to. It prints
    /// the code of the teleporter Blob is standing in.
    pub const TELEPORT_BOOTH: u16 = 0xCED4;
}

/// Where the game keeps what the guidance panel shows. Addresses in the
/// original program, found in the earlier ZX Sidekick build and checked on
/// the player's tape by `sk-check facts`.
pub mod at {
    /// The room Blob is in, a word from 0 to 511.
    pub const ROOM: u16 = 0xD2C8;
    /// The teleporters: fifteen entries of five letters, the code, then the
    /// room the teleporter is in as a word.
    pub const TELEPORTER_NAMES: u16 = 0xD036;
    pub const TELEPORTER_COUNT: usize = 15;
    /// The six entity slots, 32 bytes each; slot 0 is Blob, whose position
    /// is at offsets 5 (pixels from the left) and 6 (from the bottom).
    pub const ENTITIES: u16 = 0xDD18;
    /// The markers the current room's tiles left: from here to the address
    /// held at [`MARKERS_END`], three bytes each (x, y, kind).
    pub const MARKERS: u16 = 0x96FC;
    pub const MARKERS_END: u16 = 0x96FA;
    /// Why the room was entered (0 walking in).
    pub const ENTRY_REASON: u16 = 0xD2C4;
}

/// The marker a teleporter booth's tile leaves in its room.
pub const BOOTH_MARKER: u8 = 0x0D;

/// A teleporter whose booth has been entered: the room it is in and its
/// code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeenTeleporter {
    pub room: u16,
    pub code: [u8; 5],
}

/// The code of the teleporter in `room`, from the game's table in `mem`
/// (the machine's whole 64K), if there is one there.
#[must_use]
pub fn teleporter_code(mem: &[u8], room: u16) -> Option<[u8; 5]> {
    (0..at::TELEPORTER_COUNT).find_map(|i| {
        let entry = mem.get(usize::from(at::TELEPORTER_NAMES) + i * 7..)?;
        let (code, here) = (entry.get(..5)?, entry.get(5..7)?);
        (u16::from_le_bytes([here[0], here[1]]) == room).then(|| code.try_into().ok())?
    })
}

/// The keys that abandon a game in play when held together, the game's own
/// way: A, S, D, F and G, the whole of the keyboard's half-row 1, as its
/// half-row and bits. Found in the earlier ZX Sidekick build and checked on
/// the player's tape by `sk-check facts`.
pub const END_GAME_KEYS: (usize, u8) = (1, 0x1F);

/// What End this game holds: [`END_GAME_KEYS`], from the top of the play
/// loop until play hands over to a death or to a door, booth or pyramid
/// screen, which would read them as letters.
#[must_use]
pub fn end_game_hold() -> crate::machine::Hold {
    crate::machine::Hold {
        from: routine::MAIN_LOOP,
        until: vec![routine::MODAL, routine::DEATH],
        row: END_GAME_KEYS.0,
        bits: END_GAME_KEYS.1,
    }
}

/// The key a byte of the game's tables names. Letters and digits are their
/// ASCII; the four keys with no character of their own are the codes the
/// define-keys screen writes for them, seen on the player's tape: `*` for
/// Space, `\` for Enter, `[` for Caps Shift and `]` for Symbol Shift.
#[must_use]
pub fn key(code: u8) -> Option<Key> {
    let name = match code {
        b'*' => "space",
        b'\\' => "enter",
        b'[' => "caps",
        b']' => "symbol",
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => {
            return Key::by_name(&(code as char).to_string());
        }
        _ => return None,
    };
    Key::by_name(name)
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
    fn every_key_of_the_matrix_has_a_code_and_nothing_else_does() {
        let mut seen = std::collections::HashSet::new();
        let codes = (b'0'..=b'9').chain(b'A'..=b'Z').chain(*b"*\\[]");
        for code in codes {
            let Some(Key::Matrix(row, bit)) = key(code) else {
                panic!("{} should name a key", code as char);
            };
            assert!(
                seen.insert((row, bit)),
                "{} names a key twice",
                code as char
            );
        }
        assert_eq!(seen.len(), 40);
        assert_eq!(key(b'o'), key(b'O'));
        assert_eq!(key(b'*'), Key::by_name("space"));
        assert_eq!(key(b'\\'), Key::by_name("enter"));
        assert_eq!(key(b'['), Key::by_name("caps"));
        assert_eq!(key(b']'), Key::by_name("symbol"));
        for code in [0, b' ', b'\r', b'!', 0x7F, 0xFF] {
            assert_eq!(key(code), None, "{code:#04x}");
        }
    }

    #[test]
    fn a_teleporter_code_comes_from_its_room_s_entry() {
        let mut mem = vec![0u8; 0x10000];
        let entry = usize::from(at::TELEPORTER_NAMES) + 7 * 3;
        mem[entry..entry + 5].copy_from_slice(b"ABCDE");
        mem[entry + 5..entry + 7].copy_from_slice(&300u16.to_le_bytes());
        assert_eq!(teleporter_code(&mem, 300), Some(*b"ABCDE"));
        assert_eq!(teleporter_code(&mem, 301), None);
        assert_eq!(teleporter_code(&mem[..0xD040], 300), None, "cut short");
    }

    #[test]
    fn only_the_known_tape_is_supported() {
        assert!(!is_supported_tape(&[]));
        assert!(!is_supported_tape(b"not a tape"));
        assert_eq!(TAPE_SHA1.len(), 40);
    }
}
