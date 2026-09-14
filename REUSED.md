# Reused code

Everything listed here is our own work, under the same licence (MIT OR Apache-2.0), from:

- **starquake-recompiled**, [starquake/starquake-recompiled](https://github.com/starquake/starquake-recompiled), at commit `15d43902f06aafc8ff09b91659fda2ce6e176015`;
- **the earlier ZX Sidekick build**, `starquake/zx-sidekick-starquake` (private), at commit `5b61d71`, which itself took the files marked below from starquake-recompiled at that commit.

It is generic Spectrum, ROM-behaviour or frontend code. **None of starquake-recompiled's Starquake game logic is reused** (`games/starquake/src` outside the frontend files below); see `GOAL.md`, rule 2. The processor is not our work: it is `rustzx-z80`, a dependency.

| Here | From | Changes |
|---|---|---|
| `LICENSE-MIT`, `LICENSE-APACHE`, `rust-toolchain.toml`, `deny.toml`, `.gitignore`, `assets/README.md` | the earlier build (the first five from starquake-recompiled) | `deny.toml` allows our rustzx fork as a git source |
| `crates/zx-core` | the earlier build (from starquake-recompiled's `crates/zx-core`) | none here; the earlier build removed the Z80 decoder and machine-cycle model |
| `crates/zx-spectrum/src/lib.rs` | the earlier build | none |
| `crates/zx-spectrum/src/keys.rs` | the earlier build (from starquake-recompiled's `crates/zx-runtime/src/keys.rs`) | none |
| `crates/zx-spectrum/tests/fuse.rs` | the earlier build (from starquake-recompiled's `crates/zx-runtime/tests/fuse.rs`) | none here; the earlier build made it run the corpus against `rustzx-z80` in our bus |
| `crates/sidekick/src/machine.rs` | the earlier build | without what only the guidance used: the key hold, the watched addresses and running a routine on a copy |
| `crates/sidekick/src/rom.rs` | the earlier build | none |
| `crates/sidekick/src/print.rs` | the earlier build (from starquake-recompiled's `games/starquake/src/printer.rs`) | none here; the Spectrum ROM's print behaviour, not Starquake's, which the earlier build rewrote to work on the emulated machine's memory |
| `crates/sidekick/src/starquake.rs` | the earlier build | only the tape's checksum and the start state |
| `tools/sk-check/src/main.rs` | the earlier build | only the `rom`, `entry` and `shot` commands |
| `docs/rom.md` | the earlier build | says where the first measurement was made |
| `app/src/frontend/{audio,input,prompt,tape}.rs` | the earlier build (from starquake-recompiled's `games/starquake/src/frontend/`) | none |
| `app/src/frontend/text.rs` | the earlier build (from starquake-recompiled's `games/starquake/src/frontend/`) | without the drawing only the guidance panel used |
| `app/src/frontend/{mod,video,headless,gamepad}.rs`, `app/src/main.rs` | the earlier build (the frontend ones from starquake-recompiled's `games/starquake/src/frontend/`) | without the guidance panel, its picker and the game tracking behind it: the window is the picture alone, and the gamepad drives the joystick and pause |
| `app/fonts/` | the earlier build (from starquake-recompiled's `games/starquake/fonts/`) | none (Inter, under the SIL Open Font License) |
