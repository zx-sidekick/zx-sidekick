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
| South | Down: build a platform, or fly down | `south = "build"` |
| East | Up: pick up or swap an item, or board a hover platform | `east = "pick-up"` |
| North | Switch level 5's route | `north = "next-route"` |
| Start | Pause in play; start a game on the title screen | `[pad.default] start`, `[pad.title] start` |
| Select | The guidance menu | always |

**Up and down.** In the original, *up* both picks up or swaps and boards a hover platform, and *down* both builds and flies down; the game decides which from where BLOB is. The east button sends *up* and the south one *down* (`[[actions]] pick-up` and `build`), each with both of the game's meanings, as the D-pad's up and down do. Splitting them, so that east only picks up and south only builds, was looked at and not taken up (#20).

**The pause notice:** in the original, any move or fire continues once the pause key is let go. On a pad, A or B dismisses it.

**The loading screen:** the game's loader shows its screen until a key is pressed, and the rest of the tape loads after it; a card at the screen's foot says so, with Start on the pad (it taps SPACE).

**The title:** the game lists its control options, but the program chooses the Kempston joystick for the player on the title (the arrows and a pad drive it), so a card over the foot of the title says "Ignore the list: play with the arrow keys or a gamepad", with the keys to move, fire and start.

## Guidance levels

Each level adds to the ones below it (`[[guidance.levels]]`, with these names and descriptions):

| Level | Name | Adds |
|:-:|---|---|
| 0 | Off | Nothing: "No guidance. Press Esc or Select to choose a level." |
| 1 | Codes and the core | **TELEPORTS:** the codes of the teleports entered, in the game's own letters. **DOORS:** each security door's three key code cards, drawn in the game's own graphics, each dim until what BLOB carries answers it as the door would: a 15 answers all three, a 14 the first card that no card of its own picture answers, and each carried card one card. **CORE:** the core's nine slots in the game's own pictures, once BLOB has been in the core's room, where the game shows them ("Not seen yet" before); pieces delivered are bright, those missing are dim, and a piece being carried is outlined. |
| 2 | The map you have walked | A map of the rooms visited: walls drawn along each edge without an opening, so an opening is a gap, and inside a room the walls that keep apart two ways through it. Teleports are diamonds, BLOB's room is marked, and a door is drawn dotted, with its number. |
| 3 | What you have seen | The items lying in rooms walked through, in the game's own pictures, coloured by what they do: once the core has been seen, the core pieces still needed in the first colour (pink), unless BLOB carries one with the same picture; cards violet (a door's, or one that answers any card); the key yellow; any other core piece, which a pyramid takes, white. Packs aren't marked: they're a room's bonus rather than items, and act as BLOB touches them. |
| 4 | What you have not | The same for rooms not reached; a room not reached that holds a needed piece is outlined with dashes. From this level up the core and the pieces it needs show from a new game. |
| 5 | Routes | The shortest route to the nearest missing piece, and to the core while a needed piece is carried, in two colours, teleports included. Arrows at the picture's edge show which way out of the room each route goes. Tab, or the north pad button, switches between the nearest pieces. The first door each route passes is outlined in the rail. |
| 6 | Everything | Every teleport's code, every door's cards, and the whole planet. |

Levels 0 to 3 show only what the player could have written down; 4 and up tell things they couldn't have known. So below level 4 the core waits until BLOB has been in its room (199) this game, which the game's own set of rooms visited records: the game shows what the core wants only there. A door's cards appear in the rail in the frame the game draws them, a teleport's code once the booth shows it.

**Widgets:** CORE is an `icon-grid` (3 columns); TELEPORTS a `chip-list` in the game's font, from `persist:teleport-codes` up to level 5 and from the script at level 6; DOORS a `card-set`; the map a `map-grid`. All of them take their data from `scripts/starquake.rhai`, except TELEPORTS below level 6. DOORS lists the doors met this game, numbered as met; at level 6, every door on the planet: those met first, keeping their numbers, then the rest in room order. A door's cards are worked out when BLOB meets it, from the game's seed and the room (`docs/re/starquake.md`). So the script notes each door, in the `door` state, in the kept list `doors-met` (`seed:room:cards`), where a game continued from a save finds it again. A card is answered as the door's screen would answer it with what BLOB carries (`docs/re/starquake.md`, what items do).

**The map you have walked** (level 2) comes from the game's own set of rooms not yet visited, which it keeps for its first-visit bonus: a room whose bit is clear has been walked. A saved game carries the set, so a continued game shows its map again. Each room is drawn with its walls as decoded from the room data (below), and what's inside it (`docs/mockups/2026-10-13-inner-walls.png`). The game decides what blocks BLOB by each cell's attribute, and the real-tape tests check the decoded cells against the screen's, cell for cell. BLOB is two cells by two, so a room is read as the places he fits, joined up, down, left and right into parts. An edge is an opening where he fits against it and against the facing edge of the room beside, at the same place; a pair of wall passages opens the edge between their rooms; the core's room, shut in rock, is open where BLOB fits against the room beside it. Inside a room, the walls drawn are the blocking cells on the seams between the parts that lead out: a part leads out by an opening or a passage, or through a security door or a locked door's post to a part that does. A seam's cells in a door's entry tiles, or on a post, are drawn dotted. The doors listed in the DOORS rail carry their numbers on the map, in the middle of the door's tile.

**Items** (levels 3 and 4) come from the item table: each item's room and picture, while it isn't picked up. Those kept in the core's own room are never placed, so they aren't shown. Level 3 marks those in rooms walked, level 4 all of them.

**The whole planet** (level 6) is decoded from the room data in the script's `on_load`, the way the game draws a room: large tiles of medium tiles, each medium tile's picture saying which cells it draws, and each cell's attribute whether it blocks. The grid work, places, parts and seams, is done by the program's grid helpers. Only each room's map cell (its edges and what's inside), its blocking cells, and its door are kept. Rooms never entered are drawn dimmer. Every door is found from its tile type, and its cards worked out from the game's seed and the room, as the game does on meeting it.

Screenshots of the program running the game: [level 1](../media/starquake-level1.png), [level 2](../media/starquake-level2.png), [level 3](../media/starquake-level3.png), [level 4](../media/starquake-level4.png), [level 5](../media/starquake-level5.png), [level 6](../media/starquake-level6.png).

**Routes** (level 5) are a breadth-first search over a graph of places, not just rooms. A place is a part of a room, with its doors and posts taken as open, since a door is a thing to fetch cards for rather than a wall; gravity is ignored, as if BLOB could fly anywhere. The script builds the graph in `on_load`: one step joins two rooms' parts wherever BLOB fits against both at the same place on their shared edge, both ways, except down where a vacuum tube stands at that edge, since a tube carries him up only; a pair of wall passages joins the parts beside them; the core's room is a step from the room on its left only, one way; and every teleport booth shown on the map is a step from every other. In `on_frame` it searches from BLOB's place for the fewest steps: to the three nearest rooms with a missing piece (the part of the room the piece lies in once it's laid down, any part before), in pink, and while BLOB carries a piece the core needs, to the core, in gold, its end ringed. Tab or the north button picks the next of the nearest three ("Item 2/3"). A route is drawn through the rooms' middles, dashed into or out of a room not walked, with no line for a teleport's jump, and the two routes side by side where they share a step; a pin at the picture's edge shows the way each leaves BLOB's room. The first security door each route passes, where the part of a room it enters by isn't the part it leaves by, is outlined in the DOORS rail in its colour (once the door is listed there), and the items that would answer its cards not yet answered are ringed in that colour, with any card that answers any card. A route through a locked door's post rings the key in its colour, unless BLOB carries it.

**The game's own letters:** codes and door numbers are drawn with the game's font, read from the tape, at a whole number of screen pixels per game pixel, so every letter is the same size.

## What the definition reads

Each of these is a fact to find (below) and then a `[values]` entry:

- the current room, and BLOB's position;
- the set of rooms visited (the game keeps one for its first-visit bonus): a `bitset`;
- the room data (`rooms`, `large_tiles`, `tile_types`, `tile_pictures`), and from it each room's four openings and its doors. Worked out once, in the script's `on_load`; they never change;
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
| Full energy | `set` energy to `max`, and `patch` out the timer's call that takes from it, so its bar on screen stays full too |
| Full bridging platforms | `set` platforms to `max`, and `patch` out the call that takes 2 for each platform built |
| Full laser | `set` laser to `max`, and `patch` out the two calls that take 1 for each shot |
| Endless lives | `hold` lives |
| No harm from enemies | a `patch` where the game decides an enemy has hurt BLOB, or a write that undoes the hit before it counts; find which by tracing |

## Teleport booths

A booth asks for its five-letter code on the keyboard. `[text_entry]` describes it: state `booth`, letters A–Z, length 5, the card under the booth's own code line so the letters the booth echoes stay visible. The booth waits for every key to be let go before reading the next letter, so each letter is typed down for a frame and released the next.

The card opens on the codes seen from guidance level 1 (`choices_from_level = 1`), and the panel's list and map outline the chosen teleport.

**Codes kept between games:** every code discovered (a booth showing its own code, or a code typed correctly) is kept in `teleporter-codes.txt` (`[persist.teleport-codes]`). Level 1 lists them from the start of every later game. The menu's *Forget the teleport codes* clears them.

## The high-score table

The game's **CORE OF HEROES** holds eight entries, best first: a three-letter name, a six-digit score, and the share of the planet explored in that game. `[high_scores]` describes it (`docs/re/starquake.md`).

- **Kept between runs** in `high-scores.txt`: the entries of games played here, and no others. The kept table starts empty; until a game here gets in, the game shows its own entries, as it always has. They stay in its memory and are never kept.
- **Put back when the game starts:** each kept entry goes in whole, its share explored too, ranked by score among the game's own, which fill the rest. The table looks as it did when the last run ended.
- **Read back after each game**, as soon as the name is in: the game then starts showing the table (the `scores` state), so a save loaded before the title doesn't lose the score. A new entry is this game's, marked with the highest guidance level it had; training games are shown, then left out; loaded games are marked "loaded".

**The name.** After a high score the game asks for three letters (the `name` state). The keyboard types them as on a Spectrum. On a pad, the first press opens the booth's card under the game's three marks for the letters (`[name_entry]`, `card_row = 18`), with the titles "Choose a name" and "Spell a name". It offers the names entered before first, the most recent first; the west button switches to spelling a new one, from A to Z, 0 to 9 and a space, shown as an open box (␣). While spelling, A takes the letter into its place and moves on, and enters the name at the third, so the game always gets all three. The game makes a sound after each letter before it looks for the next, so each is typed down for 2 frames and let go for 9. Every name a game here puts in the table is kept in `high-score-names.txt` (`[persist.names]`), whether the card or the keyboard typed it; the menu's *Forget the kept lists* clears them with the other lists.

## Facts to find

Under `CLAUDE.md`'s rules, written up in `docs/re/starquake.md`, each marked confirmed, read or guess. None is left to find: every fact the definition holds has been seen running (#173), and `sidekick check` passes on `games/starquake` with the supported dump.
