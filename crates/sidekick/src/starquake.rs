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
/// whatever the define-keys screen was given after that.
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
pub const CONTROLS_INPUT: u16 = 0xC566;

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
    fn only_the_known_tape_is_supported() {
        assert!(!is_supported_tape(&[]));
        assert!(!is_supported_tape(b"not a tape"));
        assert_eq!(TAPE_SHA1.len(), 40);
    }
}
