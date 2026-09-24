# Assets

This project contains no part of the original game and no Spectrum ROM. Everything in this directory except this README is ignored by git.

**To play**, nothing needs to go here: each game's program asks for your own copy of the game (`starquake.tap`, `manicminer.tap`, or the `.zip` it was downloaded in) the first time it runs, and keeps it in your user data directory.

**For development only**, each game's local checks (`games/<game>/check`) can use, when given this folder:

| File | What | Used by |
|---|---|---|
| `starquake.tap` | Starquake (Bubble Bus, 1985), SHA-1 `65450d6f33692c2c2868c0b497037f2cfd0ef3bd` | every local check |
| `manic.tap` or `manicminer.tap` | Manic Miner (Bug-Byte, 1983), SHA-1 `84808c20566aa65e9308c3f8910a16bacfa1b982` | every Manic Miner check |
| `48.rom` | ZX Spectrum 48K ROM, SHA-1 `5ea7c2b824672e914525d1d5c419d71b84a426a2` | Starquake's checks that answering the three ROM calls matches the real ROM (`games/starquake/docs/rom.md`), each game's `entry` check, and Manic Miner's `font` check, which compares the character set included with the ROM's |
| `tests.in`, `tests.expected` | The Fuse project's Z80 test corpus (GPL, fetched, never committed); `scripts/check.sh` also finds it in `SK_ASSETS` | the processor conformance test |
| `zexall.com` | Frank Cringle's Z80 instruction exerciser (GPL, never committed), SHA-1 `1dbdf5c84c262e83cafacd26db787fba68b9605d`, as RustZX ships it in `rustzx-z80/tests/integration/assets/` | the processor's zexall tests, `cargo test --release -p rustzx-z80 -- --include-ignored` (also found in `SK_ASSETS`) |
