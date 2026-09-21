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
- [ ] Keyboard, and a joystick in every control method: the arrows with Left Control, or a gamepad with the bottom face button for down and the left one for fire, whatever their letters, and Start or the game's pause key to pause (built; awaiting a check by hand)
- [x] The high-score table kept between runs in `high-scores.txt` beside the kept tape, with each entry's guidance level, listed beside the game's CORE OF HEROES screen; games with training mode are not kept.
- [ ] Tape prompt: find or drop `starquake.tap` or its `.zip`, a link to World of Spectrum, the tape kept in the user data directory (built; awaiting a check by hand)
- [x] A notice while the game is paused, saying how to go on
- [x] The guidance panel beside the game, the picker for the guidance level and training mode's four switches (Esc, or Select on a gamepad), End this game and Exit, the note of how much help a game had beside the game-over screens, and pausing with Start or the pause key.
- [x] Guidance level 1: the codes you have been shown, in a rail down the panel's right in the game's own letters — every teleporter booth you have entered, and under them the three chips each security door asked for once its screen has shown them — and the core's nine slots as a square of three by three at the panel's top left, each in the same place at every level, in the game's own graphics: what each still wants, what is delivered, and what you carry.
- [x] Guidance level 2: a map of the rooms you have walked through, with every edge shown open or closed and walls inside divided rooms.
- [x] Guidance level 3: what you have seen lying in those rooms, each item drawn with the game's own graphic at a whole number of screen pixels a game pixel, outlined, in the colour of what it does, and a missing core piece drawn as itself.
- [x] Guidance level 4: the same for the rooms you have never walked through, since the game knows where every item is from the moment a game starts.
- [x] Guidance level 5: a route to a missing piece and one to the core while you carry a piece it needs, over the whole map as the map reads the rooms, dashed through rooms you have not visited, each drawn in its colour with an arrow in the border marked item or core, a legend of the two colours beside the core's slots, and the code of the teleporter and of the security door the route needs next outlined in the rail in the route's colour. Tab, or the top face button on a gamepad, switches between the three nearest pieces. It assumes Blob can fly everywhere and that doors and wall passages work, with lifts only ever going up (built; checked by hand).
- [x] Guidance level 6: every teleporter and door code whether you have been shown it or not, read on a copy of the machine at every new game, and the whole planet's map, the rooms you have never entered drawn dimmer (built; checked by hand).
- [x] Training mode's four switches: time stands still, full gun and platforms, endless lives, no harm from enemies; each written back between frames while it is on, and named on the game's score (built; checked by hand). No harm from enemies also stops the kinds of enemy that kill on touch, the deadly patches and the zappers, by steering a register at the instruction the game decides each with, so nothing drawn changes (built; checked against the game, not yet played).

## Playing

```
cargo run --release -p zx-sidekick-starquake
```

The game starts fullscreen, and F11 leaves it for a window as large as the screen allows: the picture is scaled by a whole number, so fullscreen is often a step larger than any window fits. The first time, the window asks for your copy of Starquake (`starquake.tap`, or the `.zip` it came in) and keeps it in your user data directory. A tape named on the command line is used as it is. The keys are the Spectrum's, so the title screen's own choices all work, Q for quit included: answer Y and the program closes once the game has said its goodbye. On top of that the arrow keys with Left Control (or Alt, comma or full stop) to fire, and a gamepad (the d-pad or left stick to move, the bottom face button for down and the left one to fire — A and X on an Xbox pad, B and Y on a Switch one, the cross and the square on a PlayStation one), are a joystick that works whichever control method you choose there: the machine presses the keys the game is listening for. Start or fire also starts a game from the title screen and goes past the intro text, so a controller alone gets you playing. Start, or the game's own pause key (Space, or the key you defined), pauses by freezing the emulation: the game never pauses itself, and the window says so over the picture; any key, a direction, fire or Start goes on. Right Control is Symbol Shift.

Beside the picture is the guidance panel. Esc, or Select on a gamepad, opens the picker and holds the game while it is open: up and down choose a row, left and right change it, Enter or A keeps the changes, and Esc, B or Select cancel them. The pad's letters are its own: A keeps and B cancels on an Xbox pad and on a Switch one alike, which is the bottom button on the first and the right one on the second, since that is where each has its A; a PlayStation pad keeps with the cross and cancels with the circle. The letters shown follow the maker the pad reports, so a controller with a mode switch shows what its own mode says. It sets the guidance level and training mode's switches, and asks before a change that would show on the game's score. End this game abandons the game in play the game's own way, by holding A S D F G, and Exit closes the program; both need a second press. Beside the game-over and high-score screens the panel says how much help the game had, and lists the high scores kept between runs with the guidance each was played with.

Each level adds to the ones below it. **Levels 0 to 3 show only what you could have written down yourself; 4 and up tell you things you could not have known.**

| level | what the panel shows |
|---|---|
| 0 | Nothing. The game as it came on the tape. |
| 1 | The codes you have been shown this game, in a rail at the panel's right in the game's own letters: every teleporter booth you have entered, and under them the three chips each security door asked you for. At the top left, the core's nine slots: what each still wants, what is delivered, and what you are carrying. |
| 2 | A map of the rooms you have walked through: every edge open or closed, the walls inside a room, where you are, and the teleporters you have seen. |
| 3 | What you have seen lying in those rooms, drawn in the game's own graphics and coloured by what it does — lilac for a chip or the card that opens any door, yellow for the pad key, white for something a pyramid trades, pink for a piece the core still wants. |
| 4 | The same for the rooms you have never walked through. The game places every item at the start of a game, so it knows where they all are; this tells you, ringed to say you have not been there. |
| 5 | A route to a missing piece and, while you carry a piece the core needs, one to the core. Each is a line on the map and an arrow in the picture's border marked "item" or "core", so they are told apart without colour; pink leads to the piece and orange to the core, and the code a route needs next, a teleporter's or a security door's, is outlined in the rail in that colour. Tab, or the top face button on a gamepad, switches between the three nearest pieces. The routes run over the whole map, dashed through rooms you have not visited, and assume you can fly everywhere, that doors and wall passages work, and that lifts only go up: getting the items and the battery for that is your problem. |
| 6 | Everything the program knows: every teleporter and door code whether you have been shown it or not, and the whole planet's map, the rooms you have never entered drawn dimmer. |

Training mode is four switches beside the levels rather than a level of its own: time stands still, so energy drains only on contact; the gun and platform bars stay full; losing a life does not cost one; and nothing harms you: enemies drain no energy, and the kinds that kill on touch, the deadly patches and the zappers cannot kill. The first three are held by writing to the game's memory between frames, never by changing the game; the last also steers a register at the three instructions the game decides an outright death with, and writes nothing, so everything is drawn and erased as the game meant. Every switch used shows on that game's score.

`--headless FRAMES [DIR [LEVEL]]` runs without a window, the joystick wandering at random, and writes PNGs of the window, picture and panel at that guidance level, the tape's loading picture first.

## How it is checked

CI builds and tests everything that needs no game data, on Linux, macOS and Windows, and fails if a game or ROM file is ever committed. `scripts/check.sh` runs that and, with `SK_ASSETS` pointing at a folder holding your `starquake.tap` and a `48.rom`, the checks against the game itself (`tools/sk-check`). `tools/sk-lab` holds tools that look rather than prove: a picture of the whole planet with every room's openings, probes, and the search behind guidance level 5's design. Measured on 14 September 2026 on `rustzx-z80` at the fork's commit `a73772d`:

- **The processor**, against the Fuse project's Z80 test corpus rather than our own work: 1,329 of 1,335 cases match exactly, and the other 6 (`37_1`, `3f`, `cb4e`, `cb5e`, `cb6e`, `cb76`) differ only in the undocumented bits 3 and 5 of F after `SCF`, `CCF` and `BIT n,(HL)`, where `rustzx-z80` follows later research into real chips; bus activity matches in all 1,335.
- **entry**: boots a real ROM, types `LOAD ""`, and feeds its loader the tape; the loader returns to `0x5E24` with the stack at `0x5E20`, exactly where ZX Sidekick starts the game.
- **rom**: through the menu, a new game and 6,000 frames of play under random joystick input, every call the game makes to the three ROM routines ZX Sidekick answers is repeated from the same state by the answer, and memory, stack and registers agree: 20,326 of 20,326 calls (226 more not compared because an interrupt landed inside them). The time each takes agrees too, on average: 895 T-states for the interrupt, 1,610 for printing a character, 541 for a control code, and 960 against the ROM's 969 for the multiply.
- **keys**: chooses each of the five control methods on the title screen in turn, starts a game, and drives it with the joystick alone: in every method each direction and fire move the picture where Blob is, the pause key is taken from the game while it plays on (also with the pause key redefined, where the Kempston method still pauses with Space and the others with the defined key), Start or fire alone gets from the title screen into play, and the control method, its key tables and the pause key read as recorded.
- **facts**: the entry points the guidance panel follows arrive in order, the menu and then play, and holding A S D F G from the top of the play loop, as End this game does, reaches the game-over screens and comes back to the menu, in all five control methods; the teleporter table holds fifteen five-letter codes in fifteen rooms, and walking into each booth has the game print the code the table gives for its room, 15 of 15. In 6,000 frames of play under random input, every room walked into is marked visited in the game's own set within 12 frames: 18 of 18. Of the rooms the map marks as holding a missing core piece as a game starts, the game places a wanted piece in each when it enters the room: 16 of 16, each at a spot inside a part of the room as the map reads it, where a level 5 route ends. The game draws each of those pieces from the graphics table the core column reads, 16 of 16, and walking into the core draws each hole with the graphic and colour the column uses. The high-score table is the tape's STARQUAKES, and a table written into memory once the tape is loaded, as the app does, is the one a game's score is ranked against: End this game's score lands fourth in a table made for it, the rest moved down, all in place when the CORE OF HEROES screen comes up. On the title screen Q then Y arrives, after the game's own 255 frames of goodbye, where it begins wiping itself, which is where the window closes the program; Q then N goes back to the menu and never does.
- **facts** also types another booth's code and finds the game entering the next room by teleport, and every room entered while walking about entered as walked, which the level 5 route relies on to tell a walked connection from a teleport.
- **map**: has the game draw each of the 512 rooms on a copy of the machine and reads the map from them, then walks Blob at random through play from 60 rooms. He never leaves a room through an edge the map shows closed, nor crosses a wall it shows inside a room: 1,124 of 2,048 edges open, 27 rooms divided inside, 243 room crossings and 48,372 positions walked, none against the map. It also walks Blob into every wall passage from the side he can stand beside it: all 22 take him to the room on that side, and the map joins exactly those 11 pairs of rooms, since a passage leads only one way. The same walks are held against level 5's graph of the planet, 532 places and 1,154 ways: every one of the 243 crossings between rooms beside each other is a way in it, and from where play starts it reaches 500 rooms without a teleport.
- **training**: 1,200 frames of the same scripted play from one saved start, under each switch alone. With every switch off, energy drains to 3 of 123 and a life is lost; time standing still leaves it at 27 or better; the gun and platform bars end no lower than they started where a plain run spends them; endless lives never falls below where it started, where a plain run does; and time standing still with no harm from enemies together leaves energy untouched at 127. With every switch off nothing is written, so the play ends exactly as the game left it. Then the outright deaths, on a copy of the machine in rooms holding no other danger: Blob stood on a thing that kills on touch in 10 rooms, on a deadly patch in 6 and in a zapper in 6 dies in all 22 with no harm from enemies off and in none with it on, and the three instructions the switch steers hold the bytes recorded.
- **By eye**: headless screenshots of the loading picture, the title screen and play.

## Development

Work lands through tickets on the [ZX Sidekick Starquake board](https://github.com/orgs/zx-sidekick/projects/1) and reviewed pull requests; `CLAUDE.md` describes the flow.

`scripts/check.sh` is the gate; gate on its exit code. Besides what CI runs, it needs the Fuse corpus in `assets/` for the processor conformance test (see `assets/README.md`), `SK_ASSETS` for the checks against the game, and `cargo-deny` and `cargo-about` installed for the dependency policy and `THIRD-PARTY.md`.

CI also checks that the machine and the checks have no frontend dependencies, holds the dependency policy in `deny.toml`, keeps `THIRD-PARTY.md` current, and builds for Intel Macs. `release.yml` builds archives for Linux, macOS and Windows on a version tag, with `docs/player/README.txt` as the player's guide.

## Licence

MIT OR Apache-2.0, at your option. It covers only this program and grants no rights in Starquake.
