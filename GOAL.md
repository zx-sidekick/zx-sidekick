# Goal: zx-sidekick-starquake

Starquake (Stephen Crow / Bubble Bus, 1985), the plain game, running as the **original program** in an emulated ZX Spectrum the player never sees. No emulator menus, file loaders, snapshots or settings: the player gives it their tape and plays Starquake in a window, with sound, keyboard and gamepad.

**Why the emulator route:** this is the first of a series of games, and running the original program unchanged is the legally easier model to keep clean over the long run. Every decision favours what is simplest to keep legally clean.

## Hard rules (legal)

1. **No game or ROM data in the repository, ever.** That means no tapes, snapshots, ROMs, or graphics, maps or text extracted into files. The player supplies their own tape at runtime, and CI guards this.
2. **No translated game logic.** Anything specific to Starquake is limited to facts: the tape's checksum, where it starts, memory addresses. The game runs as its own program; none of it is reimplemented.
3. **Generic code is fine:** the emulated machine, the window, sound, input and tape prompt.
4. **Reuse our own generic work** from [starquake-recompiled](https://github.com/starquake/starquake-recompiled) and the earlier ZX Sidekick build, never starquake-recompiled's `games/starquake` game-logic modules. `REUSED.md` records every file's origin and commit.
5. **No ROM file.** Starquake enters the ROM at only three addresses, and the program answers those calls itself. `games/starquake/docs/rom.md` holds the evidence and the decision. Manic Miner calls none, and reads only the ROM's character set for its text: those 768 bytes are included, under Amstrad's permission for emulators to include the Spectrum ROM, acknowledged as it asks (#140, decision 1). No game ever needs a ROM file, and no ROM code is ever included.
6. **Brand first.** "ZX Sidekick" is the brand and the game title appears in plain text only, with no logos or title lettering. The app and README carry a "not affiliated with or endorsed by the rights holders; you need your own copy" notice. Third-party licences are listed and respected.

## The processor

`rustzx-z80` (MIT, [RustZX](https://github.com/rustzx/rustzx)) with our patches, in `crates/rustzx-z80` under its own licence, brought in from our fork [zx-sidekick/rustzx](https://github.com/zx-sidekick/rustzx) (#193). It is checked in our bus against the Fuse corpus, never taken on its own word.

## Done

- Every step of `games/starquake/PLAN.md` committed and pushed.
- `scripts/check.sh` exits 0 with `SK_ASSETS` pointing at a folder holding the tape and a ROM: the Fuse corpus at 1,329 exact with 6 listed undocumented-flag cases and bus activity 1,335 of 1,335; `starquake-check entry` passes; `starquake-check rom` matches every compared call.
- Starquake plays in a window from the player's tape, with sound, keyboard and gamepad.
