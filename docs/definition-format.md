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
situations/*.txt     crafted situations, for testing
icons/*.png          your own icons, never the game's graphics
README.md            optional: credits, sources, notes for players
```

ZX Sidekick finds definitions in `games/` beside the program and in `games/` in its data folder. When two have the same `id`, the data folder's wins, so a player can try a newer version.

## Conventions

- **TOML or JSON.** Both describe the same structure; every example here is TOML. In JSON, an address may be a number or a string such as `"0x8F21"`, since JSON has no hex numbers. `sidekick convert FILE` turns one into the other. The JSON Schema in `schema/definition.schema.json` lets editors check both as you type.
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
| `hold_loading_screen` | `true`: the loading screen stays until a key is pressed. |
| `joystick` | What the pad's directions drive in play: `"kempston"` (default), `"sinclair1"`, `"sinclair2"`, `"cursor"`, or `"keys"` (then `[pad]` names the keys). |

## `[[states]]`: moments of the game

Each state names a moment the program needs to recognise: the title screen, play, pause, a code being asked for, game over. They're checked in order at the end of each frame, and **the first that matches is the game's state** for that frame. Unknown is a state too: nothing matched.

```toml
[[states]]
name = "paused"
when = { value = "paused_flag", equals = 1 }

[[states]]
name = "play"
pc = 0x6A00            # the frame stopped at, or passed through, this address

[[states]]
name = "title"
pc = [0x5F10, 0x5F42]  # any of these
```

| Key | Meaning |
|---|---|
| `name` | The state's name. `title`, `play`, `paused` and `game_over` have meanings to the program (the pause notice, Start on the title screen, the high-score table); others are yours. |
| `pc` | One address or a list: matches if the processor executed it during the frame. |
| `when` | A test of a value: `equals`, `not`, `in = [..]`, `at_least`, `at_most`, or `bits = 0x80` (all set). `all = [..]` and `any = [..]` combine tests. |

## `[values]`: named facts

Each entry turns bytes into a named fact. Everything else (cheats, the panel, scripts, crafted situations) uses these names.

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
| `type` | `u8`, `i8`, `u16` (little-endian), `u16be`, `bcd` (`len` bytes, two digits each), `digits` (`len` bytes, one digit each, with `zero` the code of the digit 0, default `0x30`), `bitset` (`len` bytes; bit 0 of the first byte is item 0), `bytes` (`len` raw bytes), `text` (`len` bytes in `charset`). |
| `len` | For the multi-byte types. |
| `max` | The value's full amount, for meters and for cheats that fill it. |
| `writable` | `true` if cheats, scripts or crafted situations may write it. Values are read-only otherwise. |
| `charset` | For `text`: `"ascii"` (default), or a table `{ first = 0x20, chars = " ABC..." }` mapping codes to letters, for games with their own. |
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

`count` may be a value's name instead of a number, for tables whose length the game keeps. A field may be any type above, at its `offset` within the record, with `bits` to pick out part of a byte.

### The game's font and graphics

The panel draws in the game's own style by reading its font and graphics from memory, never from the definition:

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
colours = { value = "item_colours" }   # optional: a table or bytes giving each picture's ink
```

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
| `set` | One or a list of `{ value, to }`: `to` is a number or `"max"`. Written at each sync point while the switch is on. |
| `hold` | One or a list of value names that may not go down while the switch is on. |
| `patch` | A list of `{ addr, bytes, expect }`. Applied when the switch goes on, and the original bytes put back when it goes off. If memory doesn't hold `expect` when applying, the switch is turned off with a notice: the dump or the moment is not what the definition expects. |
| `marks_score` | `true` (the default): a game played with this switch on is a training game (`ZX-SIDEKICK.md` §13). |
| `note` | Why and how it works, in your own words. |

`sidekick import-pok FILE.pok` writes `[[cheats]]` entries from a `.pok` file, each as a `patch` or a `set`.

## `[pad]`: the controller

Per state, what each part of a pad does. `[pad.default]` applies in every state; `[pad.<state>]` overrides it, entry by entry, while that state is the game's.

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
- `{ tap = ["P"], frames = 2 }`: those keys for a fixed number of frames, however long the button is held.
- `{ action = "build" }`, or just `"build"`: an `[[actions]]` entry.
- `"menu"`: the guidance menu. `select` is always `"menu"`, so every game's pad can reach it.
- `"none"`.

A button held when a state changes is held back from the game until it's let go (`ZX-SIDEKICK.md` §9).

## `[[actions]]`

Named things a button or key can do, beyond a key or joystick bit.

```toml
[[actions]]
id = "end-game"
label = "End this game"
tap = ["A", "S", "D", "F", "G"]
frames = 5

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
| `id`, `label` | Its name, and its text where shown. `end-game` is the guidance menu's "End this game". |
| `keys`, `tap`, `frames`, `joystick` | What reaches the game, as in `[pad]`. |
| `patch_for_frame` | Patches applied for the frames the action lasts, then undone; `expect` as in `[[cheats]]`. This is how a pad splits a direction's two meanings. |
| `script` | A function of the script to call (see Scripts). |
| `host_key` | A key the Spectrum doesn't have (`"Tab"`, `"F6"`…) that also does this. Esc, F5, F9 and F11 are the program's own. |

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

The card opens on the first pad press while the game is in `state`. A word entered is typed on the keys, one letter held for `key_down_frames` and released for `key_up_frames`.

## `[persist]`: lists kept between games

```toml
[persist.teleport-codes]
label = "teleport codes"       # the menu says "Forget the teleport codes"
file = "teleporter-codes.txt"  # in the game's data folder, one entry per line
add = { value = "booth_code", in_state = "booth" }   # added when the game shows one
```

`add` names a value read while the game is in a state; or `add = "script"`, and the script returns new entries as `kept.<list>` (see Scripts). Entries are kept in the order found, without repeats.

## `[high_scores]`

```toml
[high_scores]
addr = 0xC100
count = 8
stride = 9
name  = { offset = 0, len = 3, type = "text" }
score = { offset = 3, len = 6, type = "digits" }
```

The table is written into memory when the game starts, and read back when the game leaves `game_over`.

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

Widgets, drawn in the panel in the order listed, each from its `from_level` up:

```toml
[[panel]]
kind = "icon-grid"
title = "CORE"
from_level = 1
columns = 3
pictures = "item_pictures"
source = "script:core"

[[panel]]
kind = "chip-list"
title = "TELEPORTS"
from_level = 1
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
| `chip-list` | Short words in the game's font; one may be outlined | a list of `{ text, outlined }` |
| `icon-grid` | Pictures from `pictures`, each bright, dim or outlined | a list of `{ picture, look = "bright" / "dim" / "outlined" }` |
| `card-set` | Numbered groups of pictures, each dim until answered | a list of `{ label, cards = [{ picture, answered }] }` |
| `map-grid` | Places on a grid, walls, markers, routes, and exit arrows at the picture's edge | `{ width, height, cells = [{ x, y, walls = "NESW", seen }], markers = [{ x, y, kind, picture?, colour? }], routes = [{ colour, steps = [[x, y], …], exit = "N" }] }` |

`map-grid` `kind`s for markers are `player`, `teleport`, `door`, `item` and `target`; route colours are `first` and `second`.

## `[situations]`

```toml
[situations]
enter = 0x6400      # the routine that starts play in the current room
```

A crafted situation is a text file in `situations/`, of `name = value` lines naming only what differs from a new game, using `[values]` names (which must be `writable`), `cheats = [..]` and `kept.<list> = [..]`. The program starts a new game, writes those, and calls `enter`.

## Scripts

For what plain data can't say: maps worked out from the game's room data, routes, which core piece is nearest. A script is a [Rhai](https://rhai.rs) file named in `[script]`:

```toml
[script]
file = "scripts/starquake.rhai"
```

It may provide:

- **`on_load(mem)`**, run once when the tape has loaded. Whatever it returns is passed to every later call as `loaded`.
- **`on_frame(mem, loaded, prev, kept)`**, run at each sync point. It returns a map of results: each key is a `script:<result>` a widget can show, plus `kept` to add entries to `[persist]` lists.
- **Action functions** named by `[[actions]] script`, called as `fn(mem, loaded, prev, kept)`, returning the same as `on_frame`.

What a script can use:

| | |
|---|---|
| `mem.peek(addr)`, `mem.peek16(addr)` | Bytes and words |
| `mem.value(name)` | A named value |
| `mem.table(name)` | A table, as a list of maps of its fields |
| `mem.state` | The game's state's name |
| `mem.write(name, v)` | Only for `writable` values, and only in action functions |
| `prev` | What it returned last frame |
| `kept` | The `[persist]` lists |
| `level` | The guidance level |

A script can't touch files, the network or the clock, and each call has an operation limit. A script that goes over it, or fails, has its widgets turned off for the rest of the session, with the reason in the panel.

## Checking a definition

`sidekick check DEF [TAPE]` reports, without starting the game:

- anything the schema doesn't allow, with the line;
- names used but not defined, and addresses out of range;
- given the tape: that its SHA-1 is one of `[[media]]`'s, and, once the game has loaded, that each patch's `expect` bytes are there.

`sidekick author TAPE [DEF]` runs the game with the definition's values live on screen, a cheat finder, a program-counter sampler, watchpoints and breakpoints, and copies what you find as TOML (`ZX-SIDEKICK.md` §15). Edit the definition while the game runs and it's reloaded at the next frame.

## Contributing a definition

Open a pull request adding `games/<id>.toml`, or `games/<id>/`, with:

- facts only: no code, listings, graphics, maps or text of the game's;
- a `README.md` or a `docs/re/<id>.md` saying where each fact came from and how sure it is (confirmed, read, guess);
- `sidekick check` passing against the dump named in `[[media]]`.
