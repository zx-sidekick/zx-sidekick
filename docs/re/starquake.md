# Starquake: facts

The facts `games/starquake/game.toml` rests on, found under `CLAUDE.md`'s rules and written in our own words. Each is marked **confirmed** (seen running), **read** (understood from the disassembly) or **guess**. Disassembly stays in `.scratch/`; nothing of the game's code belongs here.

Dump: Starquake (Bubble Bus Software), as World of Spectrum distributes it: `STARQUAK.TAP` (56,203 bytes) in `Starquake.tap.zip`.

| Fact | Address | Meaning | Status | How found |
|---|---|---|---|---|
| The dump's SHA-1 | — | `65450d6f33692c2c2868c0b497037f2cfd0ef3bd`, of `STARQUAK.TAP` itself | confirmed | Computed from the downloaded tape (#51) |
| The loader | — | The ROM's standard loader: `LOAD ""` loads it, fast-loading works | confirmed | Loaded in the emulator; the game's title menu comes up (#43, #44) |
| The loading screen waits for a key | — | After loading, the program waits in the ROM's `PAUSE` until a key is pressed | confirmed | Seen running: the program counter in the ROM's PAUSE routine until SPACE (#43) |
| The main loop's wait for a frame | `0xA523` | Where the main loop calls its wait for the next frame, which then redraws; passed once a frame in play, at about 25 game cycles a second. `[run] sync`, and the `play` state | confirmed, read | Counted per frame in play by watching addresses (#78) |
| The title menu's loop | `0x5FF4` | The menu's loop reading its keys, passed every four frames once the title tune has played (while it plays, the tune routine runs alone). The `title` state | confirmed, read | Watched at the title (#78) |
| Pause | `0xC5AF`, `0xC5B5` | SPACE pauses: the game waits for SPACE to be let go, then until any move or fire. The `paused` state | confirmed, read | Paused and read in play (#78) |
| Game over | `0x64A0` | Where play ends: when the last life is lost, or A S D F G are held together. The game-over screen then waits for a key in the same routine as the title's tune | confirmed, read | Played until the last life was lost (#78) |
| Abandoning a game | — | A, S, D, F and G held together end the game | read | (#78) |
| Entering a room | `0xA426` | The routine that starts play in the current room: it lines BLOB up on the room's grid, draws the room and sets up its creatures. For `[situations] enter` | read | (#78) |
| The room | `0xD2C8` (word) | The room BLOB is in: 16 rooms to a row, 0 at the top left; a game starts in room 8 | confirmed | Walked down a room: 8 became 24 (#78) |
| BLOB's position | `0xDD1D`, `0xDD1E` | BLOB's x in pixels, and his y | confirmed (x), read (y) | x rose 8 a step moving right (#78) |
| Lives | `0xD2CC` | Lives left: 4 at the start; the status line shows it as two digits | confirmed | Fell by one with each life lost (#78) |
| Energy | `0xD2CD` | Energy, full at 127: drains on touching creatures; at 0 a life is lost and it refills | confirmed | Watched draining and refilling (#78) |
| Platforms | `0xD2CE` | Bridging platforms left, at most 127; 50 at the start; building one takes 2 | confirmed | Built one: 58 became 56 (#78) |
| Laser | `0xD2CF` | Laser charge, full at 127; each shot takes some | confirmed | Fired: 127 became 124 (#78) |
| The bars' most | — | Picking something up adds to lives, energy, platforms or laser in turn, and the three bars are capped at 127 | read | (#78) |
| Creatures touching BLOB | `0xA53A` | The main loop checks BLOB against each creature on the screen, and one close enough harms him; at no energy, a life goes | read | (#78), for the "no harm" switch (#79) |
| The control method | `0x5E58` | The menu's choice: 1 Kempston, 4 the keyboard (the default), and so on | confirmed | Chose Kempston: 4 became 1 (#78) |

`docs/games/starquake.md`, "Facts to find", lists what's still needed.
