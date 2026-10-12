# Starquake

**Starquake** (Stephen Crow / Bubble Bus Software, 1985) is ZX Sidekick's reference game: its definition, `games/starquake/`, uses every feature in `ZX-SIDEKICK.md`, and the mockups in `../mockups/starquake/` show it. This file holds what's particular to Starquake: what its guidance levels show, its training switches, its pad, its teleport booths and its high-score table. It also says how each maps onto the definition format.

Not affiliated with or endorsed by the rights holders; you need your own copy.

## The supported dump

One dump: the original Bubble Bus release, as World of Spectrum distributes it (`Starquake.tap.zip`, worldofspectrum.net/item/0004873). Its SHA-1 goes in `[[media]]` once it's taken from the downloaded file. Anything else is refused, with a message naming this dump. The game loads with the ROM's standard loader.

## Keys and pad

**Keyboard:** exactly as the original: its five control methods, defined keys, **SPACE** to pause, **A S D F G** together to abandon a game. **Tab** switches level 5's route between pieces (`[[actions]] next-route`, `host_key = "Tab"`).

**Pad**, in play, on the Kempston joystick (`[run] joystick = "kempston"`), whichever control method the game's menu chose:

| Button | Does | In the definition |
|---|---|---|
| D-pad, left stick | Move | `dpad`, `stick = "joystick"` |
| West | Fire | `west = "fire"` |
| South | Build a platform | `south = "build"` |
| East | Pick up or swap an item | `east = "pick-up"` |
| North | Switch level 5's route | `north = "next-route"` |
| Start | Pause in play; start a game on the title screen | `[pad.default] start`, `[pad.title] start` |
| Select | The guidance menu | always |

**Split meanings.** In the original, *up* both picks up and boards the hover platform, and *down* both builds and flies down. The pad separates them: the D-pad's up and down board and fly; the east button only picks up, and the south one only builds. The game decides in its own code what *up* means at that moment, so `pick-up` and `build` are `[[actions]]` with a `patch_for_frame` at the point where it reads the direction for each purpose, or a write that suppresses one meaning for that frame. Find the decision points first (below). If it proves fragile, ship without the split.

**The pause notice:** in the original, any move or fire continues once the pause key is let go. On a pad, A or B dismisses it.

## Guidance levels

Each level adds to the ones below it (`[[guidance.levels]]`, with these names and descriptions):

| Level | Name | Adds |
|:-:|---|---|
| 0 | Off | Nothing: "No guidance. Press Esc or Select to choose a level." |
| 1 | Codes and the core | **TELEPORTS:** the codes of the teleports entered, in the game's own letters. **DOORS:** each security door's three key code cards, drawn in the game's own graphics, each dim until something carried answers it. **CORE:** the core's nine slots in the game's own pictures; pieces delivered are bright, those missing are dim, and a piece being carried is outlined. |
| 2 | The map you have walked | A map of the rooms visited: walls drawn along each edge without an opening, so an opening is a gap. Teleports are diamonds, BLOB's room is marked, and doors are numbered. |
| 3 | What you have seen | The items lying in rooms walked through, in the game's own pictures, coloured by what they do (pad key, things to trade or packs, door cards), with the core pieces still needed marked. |
| 4 | What you have not | The same for rooms not reached. |
| 5 | Routes | The shortest route to the nearest missing piece, and to the core while a needed piece is carried, in two colours, teleports included. Arrows at the picture's edge show which way out of the room each route goes. Tab, or the north pad button, switches between the nearest pieces. The first door each route passes is outlined in the rail. |
| 6 | Everything | Every teleport's code, every door's cards, and the whole planet. |

Levels 0 to 3 show only what the player could have written down; 4 and up tell things they couldn't have known.

**Widgets:** CORE is an `icon-grid` (3 columns); TELEPORTS a `chip-list` in the game's font, from `persist:teleport-codes` up to level 5 and from the script at level 6; DOORS a `card-set`; the map a `map-grid`. All of them take their data from `scripts/starquake.rhai`, except TELEPORTS below level 6. DOORS lists the doors met this game, numbered as met. A door's cards are worked out when BLOB meets it, from the game's seed and the room (`docs/re/starquake.md`). So the script notes each door, in the `door` state, in the kept list `doors-met` (`seed:room:cards`), where a game continued from a save finds it again. A card is answered when BLOB carries it.

**The map you have walked** (level 2) comes from the screen. The game decides what blocks BLOB by each cell's attribute, so while BLOB is in a room the script reads its four edges from the play area. An edge with two or more open cells in a row is an opening; the rest are walls. Each room walked is kept once a game in `rooms-walked` (`seed:room:walls`), the first time its walls read the same two frames running, so a continued game shows its map again; the list keeps the last 2,048 rooms, some games' worth. Doors met are marked on the map, numbered only in the DOORS rail, as the mockups have them.

Screenshots of the program running the game: [level 1](../media/starquake-level1.png), [level 2](../media/starquake-level2.png). Rooms not reached (levels 4 and 6) need the room data decoded, which the items and everything (#101) bring.

**Routes** are a breadth-first search over a graph of places, not just rooms. A room can be divided by walls, so a place is the part of a room that can be walked between. Edges are the openings, the teleports (any seen code can be typed), and lifts. The script builds the graph in `on_load` and searches it in `on_frame`; `next-route` steps through the nearest pieces.

**The game's own letters:** codes and door numbers are drawn with the game's font, read from the tape, at a whole number of screen pixels per game pixel, so every letter is the same size.

## What the definition reads

Each of these is a fact to find (below) and then a `[values]` entry:

- the current room, and BLOB's position;
- the set of rooms visited (the game keeps one for its first-visit bonus): a `bitset`;
- the room map, and from it each room's four openings. Worked out once, in the script's `on_load`; they never change;
- the teleport table: each teleport's room and code;
- each security door's room and the three cards it asks for;
- the item table: each item's room, position, colour and graphic, and whether it's carried;
- the core's nine slots: the graphic each wants, and which are filled;
- the inventory;
- lives, and the three bars (energy, bridging platforms, laser);
- the game's font and item graphics (`font`, `sprites`), for drawing codes and pictures in its own style.

## Training switches

Five `[[cheats]]`:

| Switch | How |
|---|---|
| Full energy | `set` energy to `max` |
| Full bridging platforms | `set` platforms to `max` |
| Full laser | `set` laser to `max` |
| Endless lives | `hold` lives |
| No harm from enemies | a `patch` where the game decides an enemy has hurt BLOB, or a write that undoes the hit before it counts; find which by tracing |

## Teleport booths

A booth asks for its five-letter code on the keyboard. `[text_entry]` describes it: state `booth`, letters A–Z, length 5, the card under the booth's own code line so the letters the booth echoes stay visible. The booth waits for every key to be let go before reading the next letter, so each letter is typed down for a frame and released the next.

The card opens on the codes seen from guidance level 1 (`choices_from_level = 1`), and the panel's list and map outline the chosen teleport.

**Codes kept between games:** every code discovered (a booth showing its own code, or a code typed correctly) is kept in `teleporter-codes.txt` (`[persist.teleport-codes]`). Level 1 lists them from the start of every later game. The menu's *Forget the teleport codes* clears them.

## The high-score table

The game's **CORE OF HEROES** is described by `[high_scores]`: kept in `high-scores.txt`, written into memory when the game starts, read back after each game. Each entry carries the highest guidance level its game had; training games are shown, then left out; loaded games are marked "loaded".

## Facts to find

Under `CLAUDE.md`'s rules, written up in `docs/re/starquake.md`, each marked confirmed, read or guess:

- the program's entry point and the stack at handover;
- **the main loop's top** (`[run] sync`);
- the routine that starts play in a room (`[situations] enter`);
- the states: title screen, play, paused, booth, death, game over;
- every value listed above;
- where the game checks pause, death and enemy collisions;
- where it decides what *up* and *down* mean (split meanings);
- the booth's code reading;
- the high-score table.

`games/starquake/game.toml` has the definition's shape, with each of these marked `TODO` until it's found and confirmed.
