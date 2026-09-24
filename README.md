# ZX Sidekick

Classic ZX Spectrum games played from your own copy: the original program runs, unchanged, in an emulated Spectrum inside the app, with guidance added around it. One program per game.

- [Starquake](games/starquake/README.md) (Stephen Crow / Bubble Bus, 1985)
- [Manic Miner](games/manicminer/README.md) (Matthew Smith / Bug-Byte, 1983): plain play so far

**Not affiliated with or endorsed by the rights holders of the games or of the ZX Spectrum.** You need your own copy of each game; ZX Sidekick contains no part of any of them.

## The legal model

- **What is in this repository:** an emulated Spectrum (the screen, sound and keyboard around a Z80 processor), the window, and *facts* about Starquake: the tape's checksum, where the game starts, and memory addresses.
- **What is not:** any part of any game (no tapes, snapshots, graphics, maps or text extracted into files), any translation of its program into another language, and the Spectrum ROM, with one exception: the ROM's character set, 768 bytes that Manic Miner prints its text with, included under Amstrad's permission for emulators (#142). Amstrad have kindly given their permission for the redistribution of their copyrighted material but retain that copyright. Continuous integration fails if a game or ROM file is ever committed.
- **The processor:** `rustzx-z80` (MIT, [RustZX](https://github.com/rustzx/rustzx)), from our fork [zx-sidekick/rustzx](https://github.com/zx-sidekick/rustzx).
- **Reused code:** our own generic code from starquake-recompiled and the earlier ZX Sidekick build, listed in `REUSED.md`.

## How it is checked

CI builds and tests everything that needs no game data, on Linux, macOS and Windows, and fails if a game or ROM file is ever committed. `scripts/check.sh` runs that and, with `SK_ASSETS` pointing at a folder holding your `starquake.tap` and a `48.rom`, the checks against each game itself, which its README lists ([Starquake](games/starquake/README.md#how-it-is-checked)). Measured on 14 September 2026 on `rustzx-z80` at the fork's commit `a73772d`, and again on 24 September 2026 at `eac068f` and at `2992611`, with the same results:

- **The processor**, against the Fuse project's Z80 test corpus rather than our own work: 1,329 of 1,335 cases match exactly, and the other 6 (`37_1`, `3f`, `cb4e`, `cb5e`, `cb6e`, `cb76`) differ only in the undocumented bits 3 and 5 of F after `SCF`, `CCF` and `BIT n,(HL)`, where `rustzx-z80` follows later research into real chips; bus activity matches in all 1,335.

## Development

Work lands through tickets on the [ZX Sidekick board](https://github.com/orgs/zx-sidekick/projects/1) and reviewed pull requests; `CLAUDE.md` describes the flow.

`scripts/check.sh` is the gate; gate on its exit code. Besides what CI runs, it needs the Fuse corpus in `assets/` or in `SK_ASSETS` for the processor conformance test (see `assets/README.md`), `SK_ASSETS` for the checks against the game, and `cargo-deny`, `cargo-machete` (0.9.2, as CI) and `cargo-about` installed for the dependency policy, unused dependencies and `THIRD-PARTY.md`. Without the corpus it fails, unless `SK_NO_FUSE=1` says to skip it; whatever did not run, it names at the end.

CI also checks that the machine and the checks have no frontend dependencies, holds the dependency policy in `deny.toml`, finds dependencies a package names and never uses (`cargo machete`), keeps `THIRD-PARTY.md` current, and builds for Intel Macs. Outside dependencies are declared once, in the workspace's `Cargo.toml`, with `rust-version` matching the pinned toolchain; release builds use thin LTO, one codegen unit and stripped symbols. `release.yml` builds archives for Linux, macOS and Windows on a game's tag (`starquake-v0.3.0`, or `starquake-v0.3.0-rc.1` for a candidate), with `games/starquake/docs/player/README.txt` as the player's guide, and opens each release's notes with what changed since the last one, from the commit messages. [`games/starquake/CHANGELOG.md`](games/starquake/CHANGELOG.md) is the same list for every release; `scripts/changelog.sh starquake` writes it, and is run again after each release is tagged.

## Licence

MIT OR Apache-2.0, at your option. It covers only this program and grants no rights in any of the games.
