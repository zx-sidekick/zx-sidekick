# Plan

Steps, in order, ticked as they land. Each ends with the gate green, a commit, and a push.

1. [x] **Skeleton.** The workspace, licences, the gate script, CI that fails if any tape, snapshot or ROM is committed, and the documents: `GOAL.md`, this plan, `README.md`, `REUSED.md`, `assets/README.md`.
2. [x] **The machine.** `zx-core` (tape, screen, PNG, SHA-1, snapshot, timing with ULA contention) and `zx-spectrum` (a 48K bus around `rustzx-z80` from our fork). The Fuse corpus: 1,329 of 1,335 exact, the 6 undocumented-flag cases listed, bus activity 1,335 of 1,335; in the gate and in CI.
3. [x] **Starquake with no ROM.** `JR $` traps at the three ROM entries, the answers to MASK-INT, PRINT-A-2 and HL-HL×DE, the start state from the tape, `docs/rom.md`. `sk-check entry` (the real loader returns where the game is started) and `sk-check rom` (every ROM call answered as the real ROM does).
4. [ ] **Window, sound and controls.** The picture, the beeper, 50 Hz pacing, keyboard, gamepad as a Kempston joystick, the tape prompt and data directory, and headless screenshots.
5. [ ] **Documents.** `README.md` with what works and how it is checked, `REUSED.md` complete, screenshots checked by eye.

## Later

- Adopt the fork's features: a derived `Clone` for `Zx`, and `Z80::step` for the interrupt, removing the trap at `0x0038` and its time given back, `fetched_from`, and the ROM check's copy before every step. The `JR $` at `0x15F2` and `0x30A9` stay as safety stops, and every check must come out the same.
- Gamepad: press the chosen control method's keys (method at `0x5E58`), and have Start press its pause key (`UDK_PAUSE` at `0x5E70`).
- Packaging: `THIRD-PARTY.md` via cargo-about, a cargo-deny policy check, a player README.
- Release builds for Linux, Windows and macOS, only when asked.
- Hand checks by the maintainer: the tape prompt and sound.
