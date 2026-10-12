# Starquake: facts

The facts `games/starquake/game.toml` rests on, found under `CLAUDE.md`'s rules and written in our own words. Each is marked **confirmed** (seen running), **read** (understood from the disassembly) or **guess**. Disassembly stays in `.scratch/`; nothing of the game's code belongs here.

Dump: Starquake (Bubble Bus Software), as World of Spectrum distributes it: `STARQUAK.TAP` (56,203 bytes) in `Starquake.tap.zip`.

| Fact | Address | Meaning | Status | How found |
|---|---|---|---|---|
| The dump's SHA-1 | — | `65450d6f33692c2c2868c0b497037f2cfd0ef3bd`, of `STARQUAK.TAP` itself | confirmed | Computed from the downloaded tape (#51) |
| The loader | — | The ROM's standard loader: `LOAD ""` loads it, fast-loading works | confirmed | Loaded in the emulator; the game's title menu comes up (#43, #44) |
| The loading screen waits for a key | — | After loading, the program waits in the ROM's `PAUSE` until a key is pressed | confirmed | Seen running: the program counter in the ROM's PAUSE routine until SPACE (#43) |

Nothing more found yet. `docs/games/starquake.md`, "Facts to find", lists what's needed.
