# ZX Sidekick for Manic Miner

Manic Miner (Matthew Smith / Bug-Byte, 1983), played from your own copy of the game.

**Not affiliated with or endorsed by the rights holders of Manic Miner or the ZX Spectrum.** You need your own copy of the game, the original Bug-Byte release. ZX Sidekick contains no part of it: the original program you supply runs in an emulated Spectrum inside the app, unchanged.

> Plain play, training mode and guidance levels 1 to 4 so far, from #140. Levels 5 and 6 come as their own sub-issues of #140. See [`GOAL.md`](../../GOAL.md) for the aim.

## The ROM

Not needed. Manic Miner calls no ROM routine and never enables interrupts. The one thing it takes from a ROM is its character set, the 768 bytes at `0x3D00`, for its text. Those bytes are included in the program (`manicminer/src/font.rs`), under Amstrad's permission for emulators to include the Spectrum ROM (#140, decision 1). Amstrad have kindly given their permission for the redistribution of their copyrighted material but retain that copyright. No ROM file and no ROM code are included, and the font check proves the bytes are the 48K ROM's own.

## What works

- [x] The game runs from your tape with no ROM, from the title and its tune to game over
- [x] Its text in the Spectrum's own letters, from the ROM's character set
- [x] Keyboard, and the Kempston joystick the game finds by itself: the arrows with Left Control, or a gamepad with A or X to jump (built; not yet checked by hand)
- [x] Pausing freezes the emulation, with the game's own pause keys (A to G) or Start, and the window says so (built; not yet checked by hand)
- [x] Sound (built; not yet checked by ear)
- [x] Tape prompt: find or drop the tape or its `.zip`, kept in the user data directory (built; not yet checked by hand)
- [x] Training mode (#148): a picker over the picture (Esc, or Select on a pad) with five switches (endless lives, air stays full, safe falls, no harm from guardians, no harm from nasties), Go to cavern by the game's own cheat, End this game and Exit. Each switch steers the game at the one instruction it decides the thing with, and writes nothing into it (checked by hand by the maintainer, 2026-09-23)
- [x] Guidance levels 1 to 3 (#153), chosen in the same picker, in a panel beside the picture: 1 the cavern, the air in seconds, the items left and whether the portal is open; 2 the cavern drawn from the game's own cells, with Willy, the items ringed (one in the Solar Power Generator cannot be seen in the picture) and the portal outlined; 3 the nasty tiles, crumbling floor, conveyors and each guardian's path. Eugene, the Kong Beast and the Skylabs have routines of their own and get no path (checked by hand by the maintainer, 2026-09-23)
- [x] Guidance level 4 (#155), the jump preview: while Willy stands, where a jump left, straight up and right would land, or that it would kill him, drawn on the panel's cavern. The game itself answers, run on a copy of the machine with the switches in force, on a thread of its own so play never waits for it (built; not yet checked by hand)

## Playing

```
cargo run --release -p zx-sidekick-manicminer
```

The game starts fullscreen, and F11 leaves it for a window. `--headless FRAMES [DIR [LEVEL]]` runs without a window, ENTER held on the title screen to start a game, and writes a PNG of the window every 250 frames: the picture, and the panel at guidance level `LEVEL` (0 unless given).

## How it is checked

Against your own tape, `SK_ASSETS=<folder with manic.tap and 48.rom> scripts/check.sh` runs the checks against the game itself (`games/manicminer/check`):

- **entry**: boots a real ROM, types `LOAD ""`, and feeds its loader the tape; its BASIC goes on to `RANDOMIZE USR 33792` and arrives at `0x8400` with the stack at `0x7519`, exactly where ZX Sidekick starts the game.
- **keys**: the game finds the Kempston joystick; with the keyboard and with the joystick alike Willy walks and jumps; a pause key is reported while the game goes on running its loop, never waiting in its own pause; and Start starts a game from the title screen.
- **facts**: a game played by nobody arrives at the title, a new game, the main loop, a life lost and game over in that order, and CAPS SHIFT with SPACE goes back to the title. For guidance, in all 20 caverns: the tiles, conveyor, portal and items read as the tape defines them; the empty cavern's cells as its layout, and the conveyor only on its own tiles; every guardian's path kept by the guardian through 1,000 frames of play; Willy's cell at his height; an item taken leaving one fewer; the portal opening once none is left; and the air's passes falling by one each pass of the main loop, which the seconds shown are counted from. For the jump preview, in seven staged scenes, every jump it finds is made again by a player holding the keys and ends the same way, landing at the same height and at most a step on (21 of 21, 8 of them deaths); and a jump it finds fatal by a fall too long lands with safe falls on.
- **font**: the character set included is the 48K ROM's own, byte for byte, and the cavern's name is drawn letter for letter from it.
- **training**: Go to cavern reaches all 20 caverns by the game's own cheat; a fall, a nasty, a guardian, Eugene and the Kong Beast each kill Willy in play without their switch and not with it; a death takes no life with endless lives on; the air is where it started after a minute with air stays full on, and the end-of-cavern bonus still counts it down; and with every switch off, the whole of memory after play is as a machine with no rules at all leaves it.
