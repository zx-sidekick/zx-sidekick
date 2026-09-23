# Reused code

Everything listed here is our own work, under the same licence (MIT OR Apache-2.0), from:

- **starquake-recompiled**, [starquake/starquake-recompiled](https://github.com/starquake/starquake-recompiled), at commit `15d43902f06aafc8ff09b91659fda2ce6e176015` (marked *recompiled* below);
- **the earlier ZX Sidekick build**, `starquake/zx-sidekick-starquake` (private), at commit `5b61d71`, which itself took the files marked below from starquake-recompiled at that commit.

It is generic Spectrum, ROM-behaviour or frontend code, and the way of working and CI around it. **None of starquake-recompiled's Starquake game logic is reused** (`games/starquake/src` outside the frontend files below); see `GOAL.md`, rule 2. The processor is not our work: it is `rustzx-z80`, a dependency.

| Here | From | Changes |
|---|---|---|
| `LICENSE-MIT`, `LICENSE-APACHE`, `rust-toolchain.toml`, `deny.toml`, `.gitignore`, `assets/README.md` | the earlier build (the first five from starquake-recompiled) | `deny.toml` allows our rustzx fork as a git source |
| `crates/zx-core` | the earlier build (from starquake-recompiled's `crates/zx-core`) | none here; the earlier build removed the Z80 decoder and machine-cycle model |
| `crates/zx-spectrum/src/lib.rs` | the earlier build | none |
| `crates/zx-spectrum/src/keys.rs` | the earlier build (from starquake-recompiled's `crates/zx-runtime/src/keys.rs`) | none |
| `crates/zx-spectrum/tests/fuse.rs` | the earlier build (from starquake-recompiled's `crates/zx-runtime/tests/fuse.rs`) | none here; the earlier build made it run the corpus against `rustzx-z80` in our bus |
| `games/starquake/sidekick/src/machine.rs` | the earlier build | without what only the guidance used: the key hold, the watched addresses and running a routine on a copy |
| `games/starquake/sidekick/src/rom.rs` | the earlier build | none |
| `games/starquake/sidekick/src/print.rs` | the earlier build (from starquake-recompiled's `games/starquake/src/printer.rs`) | none here; the Spectrum ROM's print behaviour, not Starquake's, which the earlier build rewrote to work on the emulated machine's memory |
| `games/starquake/sidekick/src/starquake.rs` | the earlier build | only the tape's checksum and the start state |
| `games/starquake/sk-check/src/main.rs` | the earlier build | only the `rom`, `entry` and `shot` commands |
| `games/starquake/docs/rom.md` | the earlier build | says where the first measurement was made |
| `games/starquake/app/src/frontend/{audio,input,prompt,tape}.rs` | the earlier build (from starquake-recompiled's `games/starquake/src/frontend/`) | none |
| `games/starquake/app/src/frontend/text.rs` | the earlier build (from starquake-recompiled's `games/starquake/src/frontend/`) | without the drawing only the guidance panel used |
| `games/starquake/app/src/frontend/{mod,video,headless,gamepad}.rs`, `games/starquake/app/src/main.rs` | the earlier build (the frontend ones from starquake-recompiled's `games/starquake/src/frontend/`) | without the guidance panel, its picker and the game tracking behind it at first; #25 brought the panel, picker and tracking back (see the row below) |
| `games/starquake/app/src/frontend/{guidance,track,overlay,panel}.rs`, `games/starquake/app/src/frontend/overlay.wgsl`, the picker's parts of `mod.rs`, `video.rs` and `gamepad.rs`, the drawing added to `text.rs`, and the key hold and watched addresses in `games/starquake/sidekick/src/machine.rs` | the earlier build (the frontend ones from starquake-recompiled's `games/starquake/src/frontend/`) | level 1's codes brought back by #4, level 2's map by #5 and level 3's pieces by #6; legends drawn to the rule on #3; the pause notice drawn in the overlay |
| the menu, play, game-over, hand-over and death entry points and the End this game keys in `games/starquake/sidekick/src/starquake.rs`, and `sk-check facts` | the earlier build | only what the framework follows; checked in every control method |
| the teleporter table, room, new-game, enter-room and booth facts and `teleporter_code` in `games/starquake/sidekick/src/starquake.rs`, `Machine::call`, and the booth walk in `sk-check facts` | the earlier build | the booth walk reports inside `facts` |
| `games/starquake/sidekick/src/map.rs`, the room reading and map facts in `games/starquake/sidekick/src/starquake.rs`, and `sk-check map` | the earlier build | none beyond naming Blob; the check runs 60 walks in the gate |
| the core and item facts, `Item`, `items_and_core` and `missing_piece_rooms` in `games/starquake/sidekick/src/starquake.rs`, and the pieces check in `sk-check facts` | the earlier build | none |
| `stroke` in `games/starquake/app/src/frontend/panel.rs`, used for the route | the earlier build | none |
| `games/starquake/app/fonts/` | the earlier build (from starquake-recompiled's `games/starquake/fonts/`) | none (Inter, under the SIL Open Font License) |
| `.github/workflows/ci.yml`, `scripts/check.sh` | the earlier build | without the guidance's local checks (`facts`, `map`) |
| `Cargo.toml`, each crate's `Cargo.toml` | the earlier build | the processor from our fork pinned to a commit, and descriptions without the guidance |
| `CLAUDE.md`, `.claude/skills/*`, `.claude/scripts/board.sh`, `.github/ISSUE_TEMPLATE/spec.md` | recompiled | adapted: this project's board (an org Project), its checks and hard rules in place of the differential suites, the gate at `scripts/check.sh`, and qualified references to recompiled's issues |
| the `dependencies` and `intel-mac` jobs and the documentation step in `.github/workflows/ci.yml` | recompiled | the binary's name; the frontend-free check reads the dependency tree (`scripts/no-frontend.sh`), since this project has no frontend feature |
| `.github/rulesets/main.json` | recompiled's `main: require CI` ruleset, read from its API | the checks named for this repository; applied to it as `main: require CI` |
| `.github/workflows/release.yml` | recompiled | the binary's name, the font's path, and the release notes |
| `about.toml`, `about.hbs` | recompiled | the program's name and how it uses the game; Windows added to the attributed targets |
| `games/starquake/docs/player/README.txt` | recompiled | rewritten for a program that runs the original game |
