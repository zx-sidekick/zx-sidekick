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
- [ ] Keyboard, and a joystick in every control method: the arrows with Left Control, or a gamepad with A for down and X for fire, and Start or the game's pause key to pause (built; awaiting a check by hand)
- [x] The high-score table kept between runs in `high-scores.txt` beside the kept tape, with each entry's guidance level, listed beside the game's CORE OF HEROES screen; games with training mode are not kept.
- [ ] Tape prompt: find or drop `starquake.tap` or its `.zip`, a link to World of Spectrum, the tape kept in the user data directory (built; awaiting a check by hand)
- [x] A notice while the game is paused, saying how to go on
- [x] The guidance panel beside the game, the picker for the guidance level and training mode (Esc, or Select on a gamepad), End this game and Exit, the note of how much help a game had beside the game-over screens, and pausing with Start or the pause key.
- [x] Guidance level 1: the codes you have been shown, in a column down the panel's right in the game's own letters: every teleporter booth you have entered, and the three chips each security door asked for once its screen has shown them.
- [x] The items found marked on the map from level 2, drawn with the game's own graphics at a whole number of screen pixels a game pixel, outlined, and a missing core piece in a room you have seen drawn as itself.
- [x] Guidance level 2: a map of the rooms you have visited, with every edge shown open or closed and walls inside divided rooms.
- [x] Guidance level 3: the missing core pieces marked on the map, in rooms visited or not.
- [x] Guidance level 3's core column: the nine holes beside the map, missing pieces in their own graphic, in white, delivered ones dimmed, a carried one outlined.
- [ ] Guidance level 4: a route to the nearest missing piece, and one to the core while you carry a piece it needs, over the ways you have walked, each drawn on the map in its colour with an arrow in the border marked item or core, a legend, and the teleport code to select (built; awaiting a check by hand).
- [x] Guidance level 5: the same two routes through the whole map, as the map reads the rooms, dashed through rooms you have not visited. It assumes Blob can fly everywhere and that doors and wall passages work, with lifts only ever going up, and leaves the logistics to the player (built; checked by hand). Training mode is still to come.

## Playing

```
cargo run --release -p zx-sidekick-starquake
```

The game starts fullscreen, and F11 leaves it for a window as large as the screen allows: the picture is scaled by a whole number, so fullscreen is often a step larger than any window fits. The first time, the window asks for your copy of Starquake (`starquake.tap`, or the `.zip` it came in) and keeps it in your user data directory. A tape named on the command line is used as it is. The keys are the Spectrum's, so the title screen's own choices all work. On top of that the arrow keys with Left Control (or Alt, comma or full stop) to fire, and a gamepad (the d-pad or left stick to move, A for down and X to fire, as on an Xbox pad), are a joystick that works whichever control method you choose there: the machine presses the keys the game is listening for. Start or fire also starts a game from the title screen and goes past the intro text, so a controller alone gets you playing. Start, or the game's own pause key (Space, or the key you defined), pauses by freezing the emulation: the game never pauses itself, and the window says so over the picture; any key, a direction, fire or Start goes on. Right Control is Symbol Shift.

Beside the picture is the guidance panel. Esc, or Select on a gamepad, opens the picker and holds the game while it is open: up and down choose a row, left and right change it, Enter or A keeps the changes, and Esc, B or Select cancel them. It sets the guidance level and training mode, and asks before a change that would show on the game's score. End this game abandons the game in play the game's own way, by holding A S D F G, and Exit closes the program; both need a second press. Beside the game-over and high-score screens the panel says how much help the game had, and lists the high scores kept between runs with the guidance each was played with.

Each level includes everything below it:

| level | what the panel shows |
|---|---|
| 0 | Nothing. The game as it came on the tape. |
| 1 | The codes you have been shown this game, in a column at the panel's right in the game's own letters: every teleporter booth you have entered, and the three chips each security door asked you for. |
| 2 | A map of the rooms you have visited: every edge open or closed, the walls inside a room, where you are, the teleporters you have seen, and the items you have found lying about, each drawn in the game's own graphics — a chip, the card that opens any door, the pad key, or something a pyramid trades. |
| 3 | The rooms holding a core piece still needed, marked whether you have been there or not, and the core's nine holes as a square under the map: what is still wanted, what is delivered, and what you are carrying. |
| 4 | Two routes over the ways you have walked: pink to the nearest missing piece, orange to the core while you carry a piece it needs. Each has an arrow in the picture's border marked "item" or "core", so they are told apart without colour, and the panel says which code to select when a teleporter is next. Tab, or Y on a gamepad, switches the pink route between the three nearest pieces. |
| 5 | The same two routes through the whole map rather than only where you have walked, dashed through rooms you have not visited, and ending at the side of a room a piece is actually on. It assumes you can fly everywhere, that doors and wall passages work, and that lifts only go up: getting the items and the battery for that is your problem. |
| training | Changes the rules rather than showing more: the energy drain frozen, and more besides. Still to be built. |

`--headless FRAMES [DIR [LEVEL]]` runs without a window, the joystick wandering at random, and writes PNGs of the window, picture and panel at that guidance level, the tape's loading picture first.

## How it is checked

CI builds and tests everything that needs no game data, on Linux, macOS and Windows, and fails if a game or ROM file is ever committed. `scripts/check.sh` runs that and, with `SK_ASSETS` pointing at a folder holding your `starquake.tap` and a `48.rom`, the checks against the game itself (`tools/sk-check`). `tools/sk-lab` holds tools that look rather than prove: a picture of the whole planet with every room's openings, probes, and the search behind guidance level 5's design. Measured on 14 September 2026 on `rustzx-z80` at the fork's commit `a73772d`:

- **The processor**, against the Fuse project's Z80 test corpus rather than our own work: 1,329 of 1,335 cases match exactly, and the other 6 (`37_1`, `3f`, `cb4e`, `cb5e`, `cb6e`, `cb76`) differ only in the undocumented bits 3 and 5 of F after `SCF`, `CCF` and `BIT n,(HL)`, where `rustzx-z80` follows later research into real chips; bus activity matches in all 1,335.
- **entry**: boots a real ROM, types `LOAD ""`, and feeds its loader the tape; the loader returns to `0x5E24` with the stack at `0x5E20`, exactly where ZX Sidekick starts the game.
- **rom**: through the menu, a new game and 6,000 frames of play under random joystick input, every call the game makes to the three ROM routines ZX Sidekick answers is repeated from the same state by the answer, and memory, stack and registers agree: 20,326 of 20,326 calls (226 more not compared because an interrupt landed inside them). The time each takes agrees too, on average: 895 T-states for the interrupt, 1,610 for printing a character, 541 for a control code, and 960 against the ROM's 969 for the multiply.
- **keys**: chooses each of the five control methods on the title screen in turn, starts a game, and drives it with the joystick alone: in every method each direction and fire move the picture where Blob is, the pause key is taken from the game while it plays on (also with the pause key redefined, where the Kempston method still pauses with Space and the others with the defined key), Start or fire alone gets from the title screen into play, and the control method, its key tables and the pause key read as recorded.
- **facts**: the entry points the guidance panel follows arrive in order, the menu and then play, and holding A S D F G from the top of the play loop, as End this game does, reaches the game-over screens and comes back to the menu, in all five control methods; the teleporter table holds fifteen five-letter codes in fifteen rooms, and walking into each booth has the game print the code the table gives for its room, 15 of 15. In 6,000 frames of play under random input, every room walked into is marked visited in the game's own set within 12 frames: 18 of 18. Of the rooms the map marks as holding a missing core piece as a game starts, the game places a wanted piece in each when it enters the room: 16 of 16, each at a spot inside a part of the room as the map reads it, where a level 5 route ends. The game draws each of those pieces from the graphics table the core column reads, 16 of 16, and walking into the core draws each hole with the graphic and colour the column uses. The high-score table is the tape's STARQUAKES, and a table written into memory once the tape is loaded, as the app does, is the one a game's score is ranked against: End this game's score lands fourth in a table made for it, the rest moved down, all in place when the CORE OF HEROES screen comes up.
- **facts** also types another booth's code and finds the game entering the next room by teleport, and every room entered while walking about entered as walked, which the level 4 route relies on to tell a walked connection from a teleport.
- **map**: has the game draw each of the 512 rooms on a copy of the machine and reads the map from them, then walks Blob at random through play from 60 rooms. He never leaves a room through an edge the map shows closed, nor crosses a wall it shows inside a room: 1,124 of 2,048 edges open, 27 rooms divided inside, 243 room crossings and 48,372 positions walked, none against the map. It also walks Blob into every wall passage from the side he can stand beside it: all 22 take him to the room on that side, and the map joins exactly those 11 pairs of rooms, since a passage leads only one way. The same walks are held against level 5's graph of the planet, 532 places and 1,154 ways: every one of the 243 crossings between rooms beside each other is a way in it, and from where play starts it reaches 500 rooms without a teleport.
- **By eye**: headless screenshots of the loading picture, the title screen and play.

## Development

Work lands through tickets on the [ZX Sidekick Starquake board](https://github.com/orgs/zx-sidekick/projects/1) and reviewed pull requests; `CLAUDE.md` describes the flow.

`scripts/check.sh` is the gate; gate on its exit code. Besides what CI runs, it needs the Fuse corpus in `assets/` for the processor conformance test (see `assets/README.md`), `SK_ASSETS` for the checks against the game, and `cargo-deny` and `cargo-about` installed for the dependency policy and `THIRD-PARTY.md`.

CI also checks that the machine and the checks have no frontend dependencies, holds the dependency policy in `deny.toml`, keeps `THIRD-PARTY.md` current, and builds for Intel Macs. `release.yml` builds archives for Linux, macOS and Windows on a version tag, with `docs/player/README.txt` as the player's guide.

## Licence

MIT OR Apache-2.0, at your option. It covers only this program and grants no rights in Starquake.
