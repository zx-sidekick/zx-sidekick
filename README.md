# ZX Sidekick for Starquake

Starquake (Stephen Crow / Bubble Bus, 1985), played from your own copy of the game.

**Not affiliated with or endorsed by the rights holders of Starquake or the ZX Spectrum.** You need your own copy of the game. ZX Sidekick contains no part of it: the original program you supply runs in an emulated Spectrum inside the app, unchanged.

> The plain game runs from your tape. See `GOAL.md` for the aim and `PLAN.md` for the steps and what comes later.

## The legal model

- **What is in this repository:** an emulated Spectrum (the screen, sound and keyboard around a Z80 processor), the window, and *facts* about Starquake: the tape's checksum, where the game starts, and memory addresses.
- **What is not:** any part of the game (no tapes, snapshots, graphics, maps or text extracted into files), any translation of its program into another language, and the Spectrum ROM. Continuous integration fails if a game or ROM file is ever committed.
- **The ROM:** not needed. Starquake calls only three ROM routines, and ZX Sidekick answers those calls itself (`docs/rom.md`).
- **The processor:** `rustzx-z80` (MIT, [RustZX](https://github.com/rustzx/rustzx)), from our fork [zx-sidekick/rustzx](https://github.com/zx-sidekick/rustzx).
- **Reused code:** our own generic code from starquake-recompiled and the earlier ZX Sidekick build, listed in `REUSED.md`.

## What works

- [x] The game runs from your tape with no ROM
- [x] Window with the Spectrum picture and its border
- [ ] Sound (built; awaiting a check by ear)
- [ ] Keyboard, and a gamepad as the Kempston joystick with Start to pause (built; awaiting a check by hand)
- [ ] Tape prompt: find or drop `starquake.tap` or its `.zip`, a link to World of Spectrum, the tape kept in the user data directory (built; awaiting a check by hand)

## Playing

```
cargo run --release -p zx-sidekick-starquake
```

The first time, the window asks for your copy of Starquake (`starquake.tap`, or the `.zip` it came in) and keeps it in your user data directory. A tape named on the command line is used as it is. The keys are the Spectrum's; a gamepad is the Kempston joystick (choose `1` on the title screen), and Start pauses.

`--headless FRAMES [DIR]` runs without a window and writes PNGs of the picture, the tape's loading picture first.

## How it is checked

CI builds and tests everything that needs no game data, on Linux, macOS and Windows, and fails if a game or ROM file is ever committed. `scripts/check.sh` runs that and, with `SK_ASSETS` pointing at a folder holding your `starquake.tap` and a `48.rom`, the checks against the game itself (`tools/sk-check`). Measured on 14 September 2026 on `rustzx-z80` at the fork's commit `a73772d`:

- **The processor**, against the Fuse project's Z80 test corpus rather than our own work: 1,329 of 1,335 cases match exactly, and the other 6 (`37_1`, `3f`, `cb4e`, `cb5e`, `cb6e`, `cb76`) differ only in the undocumented bits 3 and 5 of F after `SCF`, `CCF` and `BIT n,(HL)`, where `rustzx-z80` follows later research into real chips; bus activity matches in all 1,335.
- **entry**: boots a real ROM, types `LOAD ""`, and feeds its loader the tape; the loader returns to `0x5E24` with the stack at `0x5E20`, exactly where ZX Sidekick starts the game.
- **rom**: through the menu, a new game and 6,000 frames of play under random joystick input, every call the game makes to the three ROM routines ZX Sidekick answers is repeated from the same state by the answer, and memory, stack and registers agree: 20,326 of 20,326 calls (226 more not compared because an interrupt landed inside them). The time each takes agrees too, on average: 895 T-states for the interrupt, 1,610 for printing a character, 541 for a control code, and 960 against the ROM's 969 for the multiply.
- **By eye**: headless screenshots of the loading picture, the title screen and play.

## Development

`scripts/check.sh` is the gate; gate on its exit code. The processor conformance test needs the Fuse corpus in `assets/` (see `assets/README.md`).

## Licence

MIT OR Apache-2.0, at your option. It covers only this program and grants no rights in Starquake.
