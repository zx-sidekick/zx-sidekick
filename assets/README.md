# Assets

This project contains no part of the original game and no Spectrum ROM. Everything in this directory except this README is ignored by git.

**To play**, nothing needs to go here: the program asks for your own copy of Starquake (`starquake.tap`, or the `.zip` it was downloaded in) the first time it runs, and keeps it in your user data directory.

**For development only**, the local checks under `tools/` can use, when given their paths:

| File | What | Used by |
|---|---|---|
| `starquake.tap` | Starquake (Bubble Bus, 1985), SHA-1 `65450d6f33692c2c2868c0b497037f2cfd0ef3bd` | every local check |
| `48.rom` | ZX Spectrum 48K ROM, SHA-1 `5ea7c2b824672e914525d1d5c419d71b84a426a2` | only the check that answering the three ROM calls matches the real ROM (`docs/rom.md`) |
| `tests.in`, `tests.expected` | The Fuse project's Z80 test corpus (GPL, fetched, never committed) | the processor conformance test |
