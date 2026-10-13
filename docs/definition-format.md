# Writing a game definition

A **definition** tells ZX Sidekick about one game: which tape it is, where the game keeps its lives and energy, what the training switches do, how a pad drives it, and what the guidance panel shows. The program has no game built in; everything game-specific is here. Add a definition and the game gets those extras, with no change to the program.

This is the reference for definition format **1**. `ZX-SIDEKICK.md` explains the features a definition switches on; this file says how to write one.

A definition holds **facts only**: SHA-1s, addresses, what the bytes there mean, short patches, and your own words and icons. Never put the game's code, graphics, maps, text or sound in it; the program reads those from the player's tape at run time. How to find the facts, and the rules for doing so, are in `ZX-SIDEKICK.md` §16 and `CLAUDE.md`.

## A first definition

The smallest useful definition: one tape, endless lives, and a pad. (The game, addresses and SHA-1 here are made up.)

```toml
schema = 1

[game]
id = "example-game"
title = "Example Game"
year = 1984
publisher = "Example Software"
machine = "48k"

[[media]]
name = "Example Game (Example Software)"
sha1 = "0123456789abcdef0123456789abcdef01234567"
files = ["example game.tap", "ExampleGame.tap.zip"]
download = "https://worldofspectrum.net/item/0000000"

[run]
joystick = "kempston"

[values.lives]
addr = 0x8F21
type = "u8"
max = 5
writable = true

[[cheats]]
id = "endless-lives"
label = "Endless lives"
hold = "lives"

[pad.default]
west = "fire"
start = { keys = ["H"] }   # this game pauses on H
```

Save it as `example-game.toml` in the `games/` folder of ZX Sidekick's data folder (or drop it onto the window), then open the game's tape. Check it with:

```
sidekick check example-game.toml path/to/example-game.tap
```

## Files and packaging

| Form | When |
|---|---|
| `<id>.toml` or `<id>.json` | Everything fits in one file. |
| `<id>.zip` holding `game.toml` or `game.json` at its top | The definition needs a script, icons or crafted situations. |
| `<id>/` folder with the zip's layout | While writing it, and in this repository. Releases zip it. |

Inside a zip or folder:

```
game.toml            or game.json
scripts/*.rhai       the script, if any (see Scripts)
tests/*.toml         crafted situations, the author's tests: left out of the zip players get
icons/*.png          your own icons, never the game's graphics
README.md            optional: credits, sources, notes for players
```

ZX Sidekick finds definitions in `games/` beside the program and in `games/` in its data folder. When two have the same `id`, the data folder's wins, so a player can try a newer version.

## Conventions

- **TOML or JSON.** Both describe the same structure; every example here is TOML. In JSON, an address may be a number or a string such as `"0x8F21"`, since JSON has no hex numbers; TOML takes either too. A converted definition writes addresses as such strings, so they stay readable. `sidekick convert FILE` turns one into the other. The JSON Schema in `schema/definition.schema.json` lets editors check both as you type.
- **`schema = 1`** comes first in every definition.
- **Addresses** are 16-bit, as the processor sees them. On a 128K, an address at `0xC000` or above may add `bank = n` (0–7) to mean that bank, wherever it's paged.
- **Names** (`id`s, value names, state names) are lowercase letters, digits, `-` and `_`. Everything else refers to them by name, never by address.
- **Bytes** in patches are written as hex strings: `"C9"`, `"00 00 00"`.
- **Spectrum keys** are named as on the keyboard: `"1"`…`"0"`, `"Q"`…`"P"`, `"A"`…`"L"`, `"ENTER"`, `"CAPS SHIFT"`, `"Z"`…`"M"`, `"SYMBOL SHIFT"`, `"SPACE"`.
- **Addresses in this file's examples are made up.** They show the form, not any real game.
- **Order matters** only where this reference says so (`[[states]]`, `[[guidance.levels]]`, `[[panel]]`).

## `[game]`

| Key | Required | Meaning |
|---|---|---|
| `id` | yes | Unique, lowercase: names the data folder (`<data>/<id>/`) and the file. |
| `title` | yes | In plain text, as the game is known. |
| `year`, `publisher`, `authors` | | Shown on the tape prompt and in the library. |
| `machine` | yes | `"48k"` or `"128k"`. |
| `notice` | | Replaces the standard "Not affiliated with or endorsed by the rights holders; you need your own copy." Keep its meaning. |

## `[[media]]`: the supported dumps

One entry per dump the definition supports. A tape that matches none is refused for this game.

| Key | Required | Meaning |
|---|---|---|
| `name` | yes | How the tape prompt names the dump. |
| `sha1` | yes | Of the tape file itself (not a zip around it). |
| `format` | | `"tap"` (default), later `"tzx"`. |
| `files` | | File names it's distributed under, matched in any case, for the tape prompt's hint. A zip is searched for a tape with the right SHA-1. |
| `download`, `download_label` | | Where the player can get it legally, and the link's text ("World of Spectrum"). |

## `[run]`

| Key | Meaning |
|---|---|
| `sync` | The address at the top of the game's main loop, reached once per frame of play. When given, each frame runs to it, so every readout and write sees the game between two of its own frames. Without it, frames end at the interrupt. |
| `hold_loading_screen` | `true`: the loading screen stays until a key or pad button is pressed, for a game that otherwise starts the moment its tape has loaded. The machine stops the moment the tape's last block has gone in through the ROM's loader, so the screen is as the tape left it and the game hasn't started; the press doesn't reach it. A game that waits by itself doesn't need this, and a tape with a loader of its own (a turbo loader) isn't held. |
| `joystick` | What the pad's directions drive in play: `"kempston"` (default), `"sinclair1"`, `"sinclair2"`, `"cursor"`, or `"keys"`. |
| `keys` | With `joystick = "keys"`: the game's own keys, `{ up = "Q", down = "A", left = "O", right = "P", fire = "SPACE" }`. |

## `[[states]]`: moments of the game

Each state names a moment the program needs to recognise: the title screen, play, pause, a code being asked for, game over. They're checked in order at the end of each frame, and **the first that matches is the game's state** for that frame. Unknown is a state too: nothing matched. A state with both `pc` and `when` needs both, and one with neither (a draft's) never matches; a `when` on a value that isn't a number (text, bytes, a table) never holds.

```toml
[[states]]
name = "paused"
when = { value = "paused_flag", equals = 1 }

[[states]]
name = "play"
role = "play"          # what it means to the program
pc = 0x6A00            # the frame stopped at, or passed through, this address

[[states]]
name = "title"
role = "title"
pc = [0x5F10, 0x5F42]  # any of these
```

| Key | Meaning |
|---|---|
| `name` | The state's name: yours, meaning nothing to the program by itself. Other sections name it (`[pad.<state>]`, `text_entry.state`, `menu_in`), and a script sees it as `mem.state`. |
| `role` | What it means to the program, if anything: `title` (the kept high scores go in when it's first reached; coming back to it ends a game), `play` (training switches apply; coming to it from the title, game over or nothing starts a game's record; a crafted situation enters it), `paused` (the picture dims and the pause notice shows) or `game_over` (no game is under way, so nothing is kept on quitting). At most one state has each. `sidekick check` points out a state named after a role without it. |
| `pc` | One address or a list: matches if the processor executed it during the frame. |
| `when` | A test of a value: `equals`, `not`, `in = [..]`, `at_least`, `at_most`, or `bits = 0x80` (all set). `all = [..]` and `any = [..]` combine tests. |
| `notice` | For the state with `role = "paused"` only: what the pause notice says while the game is paused. `text` is its line; `hints` are what to press, each with `keys` (the keyboard's, as shown: `"Alt"`, or arrows such as `"← ↑ → ↓"`), `pad` (a pad part by position: `dpad`, `stick`, `west`, `south`, `east` or `north`, shown as the pad's maker marks it) or both, and `does`. Without it, the notice says "Press a key or a button to go on." |

```toml
[[states]]
name = "paused"
when = { value = "paused_flag", equals = 1 }
notice = { text = "Move or fire to go on.", hints = [
  { keys = "← ↑ → ↓", does = "move" },
  { keys = "Alt", pad = "west", does = "fire" },
] }
```

## `[values]`: named facts

Each entry turns bytes into a named fact. Everything else (cheats, the panel, scripts, crafted situations) uses these names.

Values are read once a frame: when the frame reaches `[run] sync`, at that moment, or else at the frame's end. Everything in that frame (its state, the panel, cheats) sees the same reading. Only numbers (`u8`, `i8`, `u16`, `u16be`, `bcd`, `digits`) marked `writable` can be written, and only in RAM.

`sidekick run TAPE --definition DEF --values` prints the state and every value after the last frame, to check a definition against the game without a window.

```toml
[values.lives]
addr = 0x8F21
type = "u8"
max = 5
writable = true
description = "Lives left, shown in the status bar"
```

| Key | Meaning |
|---|---|
| `addr` | Where it starts. |
| `type` | `u8`, `i8`, `u16` (little-endian), `u16be`, `bcd` (`len` bytes, two digits each, the first byte the most significant), `digits` (`len` bytes, one digit each, the first the most significant, with `zero` the code of the digit 0, default `0x30`), `bitset` (`len` bytes; bit 0 of the first byte is item 0), `bytes` (`len` raw bytes), `text` (`len` bytes in `charset`). |
| `len` | For the multi-byte types. |
| `max` | The value's full amount, for meters and for cheats that fill it. |
| `writable` | `true` if cheats, scripts or crafted situations may write it. Values are read-only otherwise. |
| `charset` | For `text`: `"ascii"` (default), or a table `{ first = 0x20, chars = " ABC..." }` mapping codes to letters, for games with their own. A code outside it reads as `?`. |
| `description` | For authors and `sidekick author`'s overlay. |

### Tables

Many games keep records: items, enemies, doors. A table names them once:

```toml
[values.items]
type = "table"
addr = 0x9A00
count = 35
stride = 4
fields.room   = { offset = 0, type = "u8" }
fields.x      = { offset = 1, type = "u8" }
fields.sprite = { offset = 2, type = "u8" }
fields.carried = { offset = 3, type = "u8", bits = 0x80 }   # true when the bit is set
```

`count` may be a value's name instead of a number, for tables whose length the game keeps; at most 1,024 records are read, whatever it says. A field may be any type above, at its `offset` within the record, with `bits` to pick out part of a byte.

### The game's font and graphics

The panel draws in the game's own style by reading its font and graphics from memory, never from the definition. They're read each time the panel is drawn, so a game that changes them in play shows the change. Characters are one bit a pixel, the high bit leftmost, `width` / 8 bytes a row; the panel draws the font at two of its pixels to a game pixel, and each picture as large as fits its tile at a whole number of its pixels to a game pixel (an 8 × 8 picture at four, a 16 × 16 one at two). Their `mask` is skipped: only ink is drawn.

```toml
[values.font]
type = "font"
addr = 0xE000
first = 0x20       # the code of the first character
count = 64
width = 8          # pixels
height = 8

[values.item_pictures]
type = "sprites"
addr = 0xB000
count = 40
width = 16         # pixels, a multiple of 8
height = 16
stride = 32        # bytes from one picture to the next (default width/8 × height)
mask = "none"      # "none", "interleaved" (mask byte before each byte), or "after" (mask after the picture)
layout = "rows"    # "rows", or "cells": 8 × 8 character cells, left to right then top to bottom
colors = { value = "item_colors" }   # optional: a table or bytes giving each picture's ink
```

Pictures are usually stored row by row. Some games keep them as the screen's characters are, in 8 × 8 cells, each cell's eight rows together. `layout = "cells"` reads those, and wants a height that's a multiple of 8, as the width always is.

## `[[cheats]]`: training switches

Each entry is one On/Off switch in the guidance menu's training section.

```toml
[[cheats]]
id = "full-energy"
label = "Full energy"
set = { value = "energy", to = "max" }   # written at each sync point while on

[[cheats]]
id = "endless-lives"
label = "Endless lives"
hold = "lives"                           # if it went down since the last frame, put it back

[[cheats]]
id = "no-harm"
label = "No harm from enemies"
patch = [{ addr = 0x7123, bytes = "00 00 00", expect = "CD 40 75" }]
note = "Skips the call that takes energy when an enemy touches the player."
```

| Key | Meaning |
|---|---|
| `id`, `label` | Its name, and its text in the menu. |
| `set` | One or a list of `{ value, to }`: `to` is a number or `"max"`. Written each frame while the switch is on, right after the values are read (at the sync point, or the frame's end without one), so the game sees it at the start of its next frame. When a state has role `play`, only in that state: elsewhere a game may use the same memory for other things. |
| `hold` | One or a list of value names that may not go down while the switch is on: one that went down since the last frame is put back, at the same moment as `set`. |
| `patch` | A list of `{ addr, bytes, expect }`. Applied when the switch goes on, and the original bytes put back when it goes off. If memory doesn't hold every patch's `expect` when the switch goes on, it stays off and none of them is applied, with a notice: the dump or the moment is not what the definition expects. |
| `call` | A routine of the game's, called once at the next sync point in play after the switch goes on, with the sync address as its return, so the main loop carries on after it: one that redraws what `set` changed, so it shows at once. Needs `[run] sync`. |
| `marks_score` | `true` (the default): a game played with this switch on is a training game (`ZX-SIDEKICK.md` §13). |
| `note` | Why and how it works, in your own words. |

A game remembers the highest guidance level it was played at and every switch with `marks_score` that was on. A game starts when the state becomes the one with role `play` after the ones with `title` or `game_over`, or none; without those roles, everything from loading is one game. `sidekick run TAPE --switch ID@FRAME --values` turns a switch on at a frame and prints the record.

`sidekick import-pok FILE.pok` writes `[[cheats]]` entries from a `.pok` file, the POKE format Fuse and ZXDB use, to paste into a definition (`-o FILE` writes them to a file). Each trainer becomes a switch with a `patch` for each of its pokes, the byte that was there as its `expect`, and the bank on a 128K. A trainer that asks the player for a value is left out, with a comment saying so.

While a game runs, the program looks at its definition's files about once a second, the definition and the files in its folder and the folders in that, and reads it again when one has changed: its values, states and switches, its script, and what the pad does. A notice says it reloaded, naming any switch it had to leave off (gone from the definition, or whose patch no longer finds its bytes), or why it couldn't, in which case the game goes on as it was.

## `[pad]`: the controller

Per state, what each part of a pad does. `[pad.default]` applies in every state; `[pad.<state>]` overrides it, entry by entry, while that state is the game's (as of the last frame's end). Without a `[pad]`, the D-pad and stick drive the joystick, west fires, and the other buttons do nothing in the game; `[pad.default]` changes that entry by entry.

Parts go by position, whoever made the pad. The stick counts once it's past half its travel, and so do the triggers.

```toml
[pad.default]
dpad  = "joystick"    # drives the joystick named in [run]
stick = "joystick"
west  = "fire"
start = { keys = ["H"] }

[pad.title]
start = { keys = ["1"] }       # 1 starts a game on the title screen
south = { keys = ["2"] }
```

| Part | Names |
|---|---|
| Directions | `dpad`, `stick` (left stick) |
| Face buttons, by position | `west`, `south`, `east`, `north` |
| Others | `l1`, `r1`, `l2`, `r2`, `start`, `select` |

What a part can do:

- `"joystick"` (directions only): the joystick named in `[run]`.
- `"up"`, `"down"`, `"left"`, `"right"`, `"fire"`: one joystick direction or fire.
- `{ keys = ["SYMBOL SHIFT", "A"] }`: those keys, held as long as the button is.
- `{ tap = ["P"], frames = 2 }`: those keys for a fixed number of frames, however long the button is held. `frames` is 2 when not given.
- `{ sequence = ["1", "0"], frames = 5 }`: those keys tapped one after another, each for `frames` frames with as many between; for a menu that wants several keys, such as choosing the controls and then starting.
- `{ action = "build" }`, or just `"build"`: an `[[actions]]` entry.
- `"menu"`: the guidance menu. `select` is always `"menu"`, so every game's pad can reach it.
- `"none"`.

A button held when a state changes what it does is held back from the game until it's let go (`ZX-SIDEKICK.md` §9); one that does the same in both states carries on, so a held direction isn't cut short.

## `[[actions]]`

Named things a button or key can do, beyond a key or joystick bit.

```toml
[[actions]]
id = "end-game"
label = "End this game"
tap = ["A", "S", "D", "F", "G"]
frames = 5
menu = "End this game"   # a row in the guidance menu
menu_in = ["play"]       # offered only in these states

[[actions]]
id = "pick-up"
joystick = ["up"]
patch_for_frame = [{ addr = 0x6C10, bytes = "18", expect = "20" }]
note = "Up, with the hover-platform check skipped for this frame, so it only picks up."

[[actions]]
id = "next-route"
script = "next_route"
host_key = "Tab"
```

| Key | Meaning |
|---|---|
| `id`, `label` | Its name, and its text where shown. |
| `menu`, `menu_in` | A row in the guidance menu, in `menu`'s words, offered in the states `menu_in` names (every state when left out). Like Exit, it wants a second press, and then the menu closes and the action is done. |
| `keys`, `tap`, `sequence`, `frames`, `joystick` | What reaches the game, as in `[pad]`. |
| `patch_for_frame` | Patches applied for the frames the action lasts, then undone; `expect` as in `[[cheats]]`. This is how a pad splits a direction's two meanings. |
| `script` | A function of the script to call (see Scripts). |
| `host_key` | A key the Spectrum doesn't have that also does this, as a pad button would: `Tab`, `F1` to `F12`, `Insert`, `Delete`, `Home`, `End`, `PageUp` or `PageDown`. Esc, F5, F9 and F11 are the program's own. |

An action lasts while its button (or key) is held, and at least `frames` frames from the press when it gives `frames`: its `keys` and `joystick` are held, and its `patch_for_frame` applied before each of those frames and undone after. A patch whose `expect` bytes aren't there that frame is skipped, so it never lands on code that isn't the game's; the rest of the action still happens. A `script` does nothing until scripts arrive.

## `[text_entry]`: typing a word with a pad

```toml
[text_entry]
state = "booth"
letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
length = 5
card_row = 20              # the character row the card's top sits at
title_choose = "Choose a code"
title_spell = "Spell a code"
choices = "teleport-codes" # a [persist] list
choices_from_level = 1     # offer them from this guidance level up
key_down_frames = 1
key_up_frames = 1
```

The card opens on the first pad press while the game is in `state`, its top at the character row `card_row` so the game's own echo of the letters stays in sight. It offers the words of the `choices` list from `choices_from_level` up, and otherwise opens on spelling; the west button switches between the two, and the back button clears what's spelled. A word entered is typed on the keys, one letter held for `key_down_frames` and released for `key_up_frames` (a frame each when not given). While the card is open the pad drives only the card; the keyboard never opens it.

## `[persist]`: lists kept between games

```toml
[persist.teleport-codes]
label = "teleport codes"       # the menu says "Forget the teleport codes"
file = "teleporter-codes.txt"  # in the game's data folder, one entry per line
add = { value = "booth_code", in_state = "booth" }   # added when the game shows one
```

`most = N` keeps at most `N` entries: past that, the oldest go first. A list a game adds to every play, such as rooms or doors met in each game, uses it so the list doesn't grow for ever. `add` names a value read while the game is in a state; or `add = "script"`, and the script returns new entries as `kept.<list>` (see Scripts). Entries are kept in the order found, without repeats. An entry is the value's text (trimmed; an empty one is never kept). A list changed in play is saved to its file at once; `sidekick run` reads the lists but never writes them. A chip-list's `source = "persist:<list>"` shows one, and the guidance menu's "Forget the …" row (with a second press) empties every list.

## `[high_scores]`

```toml
[high_scores]
title = "CORE OF HEROES"   # its heading in the panel after a game
addr = 0xC100
count = 8
stride = 9
name  = { offset = 0, len = 3, type = "text" }
score = { offset = 3, len = 6, type = "digits" }
```

The kept table (`high-scores.txt` in the game's data folder) is written into memory the first time the game reaches its title, and read back when the title comes round after a game, by when the game has put in the new score and its name. An entry that wasn't there before is this game's, marked with how it was played: its highest guidance level, or "no guidance"; one from a training game is shown for the rest of the session but not kept. Entries no game here made are marked "from the tape". After a game, until the next starts, the panel shows what it was played with and the table under `title`.

`name` is `text` (in its `charset`); `score` is `digits`, `bcd`, `u8` or `u16`.

## `[guidance]` and `[[panel]]`

Levels, in order from 0 (no help):

```toml
[[guidance.levels]]
name = "Off"
description = "No guidance. Press Esc or Select to choose a level."

[[guidance.levels]]
name = "Codes and the core"
description = "The codes you've seen, and the core's pieces."
```

Widgets, drawn in the panel in the order listed, each from its `from_level` up. `column` puts a widget in the `left` column or the `right` one (right-aligned), which stack down side by side, or across the panel's `full` width (the default), below everything in the columns above it:

```toml
[[panel]]
kind = "icon-grid"
title = "CORE"
from_level = 1
column = "left"
columns = 3
pictures = "item_pictures"
source = "script:core"

[[panel]]
kind = "chip-list"
title = "TELEPORTS"
from_level = 1
column = "right"
font = "font"
source = "persist:teleport-codes"
empty = "None yet"

[[panel]]
kind = "meter"
title = "ENERGY"
from_level = 1
source = "value:energy"
```

`source` is `value:<name>`, `persist:<list>` or `script:<result>`. What each kind shows, and the data it takes from a script:

| Kind | Shows | Script data |
|---|---|---|
| `text` | A line of text | a string |
| `counter` | A number | a number |
| `meter` | A bar, against the value's `max` | a number, and optionally `max` |
| `chip-list` | Short words in the game's font; one may be outlined | a list of `{ text, outlined }`, or of words; `outlined` is `true` (the accent), or `"first"` or `"second"` for a route's colour |
| `icon-grid` | Pictures from `pictures`, each bright, dim or outlined | a list of `{ picture, look = "bright" / "dim" / "outlined" }` |
| `card-set` | Numbered groups of pictures, each dim until answered | a list of `{ label, cards = [{ picture, answered }] }` |
| `map-grid` | Places on a grid, walls, what's inside each place, markers, routes, and exit arrows at the picture's edge | `{ width, height, inside? = [w, h], cells = [{ x, y, walls = "NESW", seen, dim?, dashed?, inside? = { walls = [[x, y], …], doors = [[x, y], …] } }], markers = [{ x, y, at? = [x, y], kind, picture?, label?, color? }], routes = [{ color, steps = [[x, y], …], label?, exit? }] }` |

A `map-grid`'s `markers` say how each kind of marker its script uses is drawn: `markers = { you = "player", teleport = "diamond", door = "label", target = "ring", item = "picture" }`. The shapes are `player` (a box in its cell), `diamond`, `square`, `ring`, `picture` (the marker's `picture`, from the widget's `pictures`) and `label` (the marker's `label`, a few characters such as a door's number; a marker without one isn't drawn). A kind the widget doesn't list isn't drawn. A marker's `color` is `"first"`, `"second"`, a Spectrum colour 0 to 15, or any colour as `"#RRGGBB"`; a `square`, `diamond` or `label` without one is teal. Route `color`s are `first` (pink) and `second` (gold). A cell is drawn with its walls once it's `seen`, and as a dot before; a `dim` one (seen on the map, never entered) is drawn dimmer, and a `dashed` one not yet seen is outlined with dashes instead of its dot, so a marker in it has somewhere to sit. Cells are as large as fits, up to 15 pixels. Floors are drawn first, then every wall over them, each edge's line overhanging its corners by a pixel. A route's `label` is shown underlined in its colour just above the map; its `exit` (`N`, `E`, `S` or `W`) puts a pin at that edge of the game's picture, saying the label's first word.

A cell's inside is drawn when the map gives `inside = [w, h]`: each cell is divided into `w` columns and `h` rows, stretched over its square. A cell's `inside.walls` are drawn as small blocks of that grid, and its `inside.doors` as a checkerboard, in teal. A marker's `at` puts it on the middle of that inside cell rather than its cell's middle.

## `[situations]`

```toml
[situations]
enter = 0x6400          # the routine that starts play in the current room
start = ["SPACE", "0"]  # the keys from the loaded tape to play
```

A crafted situation is a TOML file in `tests/`, of `name = value` lines naming only what differs from a new game: `[values]` names (which must be `writable`), `cheats = [..]`, `kept.<list> = [..]`, and `level = N`, the guidance level to play at (one of `[guidance]`'s levels; without it, the level is left as it is).

```
lives = 9
cheats = ["no-harm"]
level = 6
```

The program starts a new game by tapping the `start` keys in turn. Each is tapped a second after the state last changed, and again every forty seconds while it stays the same. Once the game is in the state with role `play` (which a situation needs), at the sync address, the program writes the values, turns the switches on, sets the guidance level and adds the kept entries. Then it calls `enter`, with the sync address as its return, so the main loop carries on after it. The game is marked loaded. `sidekick TAPE --load FILE.toml`, or `sidekick run TAPE --load FILE.toml`, starts from one.

## Scripts

For what plain data can't say: maps worked out from the game's room data, routes, which core piece is nearest. A script is a [Rhai](https://rhai.rs) file named in `[script]`:

```toml
[script]
file = "scripts/starquake.rhai"
```

It may provide:

- **`on_load`**, run once: when the tape has loaded through the ROM's loader, or when the game's state is first known. Whatever it returns is passed to every later call as `loaded`.
- **`on_frame`**, run each frame, after the values are read. It returns a map of results: each key is a `script:<result>` a widget can show, plus `kept` to add entries to `[persist]` lists.
- **Action functions** named by `[[actions]] script`, called when the action's button or key is pressed, returning the same as `on_frame`.

Each function is given as many of `mem`, `loaded`, `prev`, `kept` and `level` as it takes, in that order: `fn on_load(mem)` and `fn on_frame(mem, loaded, prev)` are both fine.

What a script can use:

| | |
|---|---|
| `mem.peek(addr)`, `mem.peek16(addr)` | Bytes and words |
| `mem.value(name)` | A named value (`()` when it couldn't be read this frame; a name that isn't a value stops the script, so a typo shows) |
| `mem.table(name)` | A table, as a list of maps of its fields |
| `mem.state` | The game's state's name |
| `mem.write(name, v)` | Only for `writable` values, and only in action functions |
| `grid(width, height, cells)` | A grid of cells, row by row, each blocking when true or non-zero. Done by the program, each call one operation: |
| `g.parts(w, h)` | Where a body `w` x `h` cells fits, all its cells free, the places numbered by part: places side by side or one above the other are one part. `.at(x, y)` is the part of the place with its top-left cell there (`()` where it doesn't fit), `.count` how many parts, `.width` and `.height` the places across and down |
| `g.seams(parts, sources)` | The blocking cells where parts meet, row by row, as flags: each part in `sources` grows out from the cells its places cover, a cell a step through every cell, all at once, and where two parts' growths touch, the blocking cells on either side are marked |
| `prev` | What it returned last frame |
| `kept` | The `[persist]` lists |
| `level` | The guidance level |

A script can't touch files, the network or the clock, and can't `eval`. Each call may take 1,000,000 operations, and `on_load`, which runs once, 20,000,000; calls go 32 deep; strings are at most 4,096 characters, arrays and maps 65,536 entries. A script that goes over a limit, or fails, is turned off for the rest of the session, and its widgets show the reason instead. `mem.write` in `on_frame` is a failure: only actions write.

`sidekick check` compiles the script, reporting a syntax error with its line, and checks that every action's `script` names one of its functions.

## Checking a definition

`sidekick check DEF [TAPE]` reports, without starting the game, one problem a line as `path: message`:

- anything the schema doesn't allow (an unknown key is an error, so a typo can't hide), with the line;
- what's still missing (an address not found yet), names used but not defined, and values used in ways their type doesn't allow;
- given a tape, or a zip of tapes: whether it's one of `[[media]]`'s dumps, by SHA-1.

It also says what works but may not be what was meant, as `path: warning: message`: a state named `play`, `title`, `paused` or `game_over` without that `role`, an action named `end-game` without a `menu` row, and a `map-grid` without `markers`. Those names meant something to the program once; now only `role`, `menu` and `markers` do.

It exits 0 with no problems (warnings or not), 1 with problems, and 2 when something couldn't be read. Checking each patch's `expect` bytes in the loaded game comes with `sidekick author`.

`sidekick convert FILE [-o OUT]` turns a TOML definition into JSON, or a JSON one into TOML.

`sidekick author TAPE [DEF]` runs the game with the definition's values live on screen, a cheat finder, a program-counter sampler, watchpoints and breakpoints, and copies what you find as TOML (`ZX-SIDEKICK.md` §15). Edit the definition while the game runs and it's reloaded at the next frame.

## Contributing a definition

Open a pull request adding `games/<id>.toml`, or `games/<id>/`, with:

- facts only: no code, listings, graphics, maps or text of the game's;
- a `README.md` or a `docs/re/<id>.md` saying where each fact came from and how sure it is (confirmed, read, guess);
- `sidekick check` passing against the dump named in `[[media]]`.
