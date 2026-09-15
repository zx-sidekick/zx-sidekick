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
- [ ] Tape prompt: find or drop `starquake.tap` or its `.zip`, a link to World of Spectrum, the tape kept in the user data directory (built; awaiting a check by hand)
- [x] A notice while the game is paused, saying how to go on
- [x] The guidance panel beside the game, the picker for the guidance level and training mode (Esc, or Select on a gamepad), End this game and Exit, the note of how much help a game had beside the game-over screens, and pausing with Start or the pause key.
- [x] Guidance level 1: the codes of the teleporters whose booths you have entered, listed in the panel.
- [x] Guidance level 2: a map of the rooms you have visited, with every edge shown open or closed and walls inside divided rooms.
- [x] Guidance level 3: the missing core pieces marked on the map, in rooms visited or not.
- [x] Guidance level 3's core column: the nine holes beside the map, missing pieces in their own graphic, in white, delivered ones dimmed, a carried one outlined.
- [ ] Guidance level 4: a route to the nearest missing piece over the ways you have walked, drawn on the map with an arrow in the border and the teleport code to select (built; awaiting a check by hand). Level 5 and training mode are still to come.

## Playing

```
cargo run --release -p zx-sidekick-starquake
```

The first time, the window asks for your copy of Starquake (`starquake.tap`, or the `.zip` it came in) and keeps it in your user data directory. A tape named on the command line is used as it is. The keys are the Spectrum's, so the title screen's own choices all work. On top of that the arrow keys with Left Control (or Alt, comma or full stop) to fire, and a gamepad (the d-pad or left stick to move, A for down and X to fire, as on an Xbox pad), are a joystick that works whichever control method you choose there: the machine presses the keys the game is listening for. Start or fire also starts a game from the title screen and goes past the intro text, so a controller alone gets you playing. Start, or the game's own pause key (Space, or the key you defined), pauses by freezing the emulation: the game never pauses itself, and the window says so over the picture; any key, a direction, fire or Start goes on. Right Control is Symbol Shift.

Beside the picture is the guidance panel. Esc, or Select on a gamepad, opens the picker and holds the game while it is open: up and down choose a row, left and right change it, Enter or A keeps the changes, and Esc, B or Select cancel them. It sets the guidance level (0 to 5) and training mode, and asks before a change that would show on the game's score. At level 1 the panel lists the codes of the teleporters whose booths you have entered this game; at level 2 it adds a map of the rooms you have visited, with their openings, the room you are in and the teleporters you have seen; at level 3 it marks the rooms holding a core piece still needed, visited or not, and shows the core's nine holes in a column beside the map, drawn with the game's own graphics from memory; at level 4 it draws a route to the nearest missing piece over the ways you have walked, or to the core while you carry a piece it needs, with an arrow in the picture's border and the code to select when a teleporter is next; level 5 and training mode are still to be built. End this game abandons the game in play the game's own way, by holding A S D F G, and Exit closes the program; both need a second press. Beside the game-over and high-score screens the panel says how much help the game had.

`--headless FRAMES [DIR [LEVEL]]` runs without a window, the joystick wandering at random, and writes PNGs of the window, picture and panel at that guidance level, the tape's loading picture first.

## How it is checked

CI builds and tests everything that needs no game data, on Linux, macOS and Windows, and fails if a game or ROM file is ever committed. `scripts/check.sh` runs that and, with `SK_ASSETS` pointing at a folder holding your `starquake.tap` and a `48.rom`, the checks against the game itself (`tools/sk-check`). Measured on 14 September 2026 on `rustzx-z80` at the fork's commit `a73772d`:

- **The processor**, against the Fuse project's Z80 test corpus rather than our own work: 1,329 of 1,335 cases match exactly, and the other 6 (`37_1`, `3f`, `cb4e`, `cb5e`, `cb6e`, `cb76`) differ only in the undocumented bits 3 and 5 of F after `SCF`, `CCF` and `BIT n,(HL)`, where `rustzx-z80` follows later research into real chips; bus activity matches in all 1,335.
- **entry**: boots a real ROM, types `LOAD ""`, and feeds its loader the tape; the loader returns to `0x5E24` with the stack at `0x5E20`, exactly where ZX Sidekick starts the game.
- **rom**: through the menu, a new game and 6,000 frames of play under random joystick input, every call the game makes to the three ROM routines ZX Sidekick answers is repeated from the same state by the answer, and memory, stack and registers agree: 20,326 of 20,326 calls (226 more not compared because an interrupt landed inside them). The time each takes agrees too, on average: 895 T-states for the interrupt, 1,610 for printing a character, 541 for a control code, and 960 against the ROM's 969 for the multiply.
- **keys**: chooses each of the five control methods on the title screen in turn, starts a game, and drives it with the joystick alone: in every method each direction and fire move the picture where Blob is, the pause key is taken from the game while it plays on (also with the pause key redefined, where the Kempston method still pauses with Space and the others with the defined key), Start or fire alone gets from the title screen into play, and the control method, its key tables and the pause key read as recorded.
- **facts**: the entry points the guidance panel follows arrive in order, the menu and then play, and holding A S D F G from the top of the play loop, as End this game does, reaches the game-over screens and comes back to the menu, in all five control methods; the teleporter table holds fifteen five-letter codes in fifteen rooms, and walking into each booth has the game print the code the table gives for its room, 15 of 15. In 6,000 frames of play under random input, every room walked into is marked visited in the game's own set within 12 frames: 18 of 18. Of the rooms the map marks as holding a missing core piece as a game starts, the game places a wanted piece in each when it enters the room: 16 of 16. The game draws each of those pieces from the graphics table the core column reads, 16 of 16, and walking into the core draws each hole with the graphic and colour the column uses.
- **map** also walks each sideways crossing seen in its walks back by holding the opposite direction, which the level 4 route relies on to count a walked step left or right both ways: 55 of 55. **facts** types another booth's code and finds the game entering the next room by teleport, and every room entered while walking about entered as walked.
- **map**: has the game draw each of the 512 rooms on a copy of the machine and reads the map from them, then walks Blob at random through play from 60 rooms. He never leaves a room through an edge the map shows closed, nor crosses a wall it shows inside a room: 1,126 of 2,048 edges open, 27 rooms divided inside, 243 room crossings and 48,372 positions walked, none against the map.
- **By eye**: headless screenshots of the loading picture, the title screen and play.

## Development

Work lands through tickets on the [ZX Sidekick Starquake board](https://github.com/orgs/zx-sidekick/projects/1) and reviewed pull requests; `CLAUDE.md` describes the flow.

`scripts/check.sh` is the gate; gate on its exit code. Besides what CI runs, it needs the Fuse corpus in `assets/` for the processor conformance test (see `assets/README.md`), `SK_ASSETS` for the checks against the game, and `cargo-deny` and `cargo-about` installed for the dependency policy and `THIRD-PARTY.md`.

CI also checks that the machine and the checks have no frontend dependencies, holds the dependency policy in `deny.toml`, keeps `THIRD-PARTY.md` current, and builds for Intel Macs. `release.yml` builds archives for Linux, macOS and Windows on a version tag, with `docs/player/README.txt` as the player's guide.

## Licence

MIT OR Apache-2.0, at your option. It covers only this program and grants no rights in Starquake.
