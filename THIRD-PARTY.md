# Third-party material

## RustZX

ZX Sidekick's emulated machine is `rustzx-core` and `rustzx-z80` (with `aym`, through `rustzx-core`) from the [zx-sidekick fork of RustZX](https://github.com/zx-sidekick/rustzx), taken as a Cargo dependency pinned to one commit. RustZX is under the MIT licence, copyright (c) 2016 Vladyslav Nikonov and the RustZX contributors; the fork's patches are under the same licence. The licence is in the fork's `LICENSE.md`, and is included in each release.

## The ZX Spectrum ROMs

The emulator contains the ZX Spectrum 48K and 128K ROMs, created by Sinclair Research Ltd (now owned by Amstrad plc). They come inside `rustzx-core` (`rustzx_core::zx::roms`), enabled by its `embedded-roms` feature.

Amstrad has [given permission](https://groups.google.com/forum/?hl=en#!msg/comp.sys.amstrad.8bit/HtpBU2Bzv_U/HhNDSU3MksAJ) to distribute their ROMs in conjunction with emulators, provided the copyright notice is kept and nothing is charged for them. ZX Sidekick is free, and charges nothing for anything it includes.

## Inter

The program's own text is set in [Inter](https://rsms.me/inter/) 4.1, by Rasmus Andersson and The Inter Project Authors, under the SIL Open Font License 1.1. Its Regular, Medium, SemiBold and Bold are embedded from `crates/sidekick-ui/fonts/`, with the licence beside them (`LICENSE.txt`), included in each release.

## Games

No game is included. Each game runs from the player's own copy. The definitions in `games/` hold facts about the games (checksums, memory addresses and what they mean) in their authors' own words, and nothing of the games themselves. ZX Sidekick is not affiliated with or endorsed by any game's rights holders.
