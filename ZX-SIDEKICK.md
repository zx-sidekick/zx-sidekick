# ZX Sidekick

A design and build plan for one program that plays original **ZX Spectrum games** from the player's own copy, with optional help built around them: training switches, gamepad controls, a guidance panel, save games and kept high scores. Each game's original program runs, unchanged, inside an emulated Spectrum. Everything this program adds lives outside that machine. It reads, and occasionally writes, the emulated machine's memory, and never changes a game's code on disk.

What the program does for a particular game comes from that game's **definition**: a TOML or JSON file, or a zip holding one with a few extra files, that names the game's tape, its memory addresses and what they mean, its training switches, how a pad drives it, and what its guidance panel shows. **Anyone can add a game by adding a definition**, with no change to the program. A tape no definition knows still plays, without the extras.

**Starquake** (Stephen Crow / Bubble Bus Software, 1985) is the reference game. Its definition exercises every feature in this plan, and its own notes are in [`docs/games/starquake.md`](docs/games/starquake.md). The screenshots in `starquake-emulated-mockups/` come from an earlier Starquake-only implementation. Treat them as the reference design: the layout, wording and behaviour described here match them, for every game.

The definition format is specified in [`docs/definition-format.md`](docs/definition-format.md). The rules for working with game code are in [`CLAUDE.md`](CLAUDE.md).

---

## 1. Ground rules

1. **No game data in the repository, ever.** No tapes, snapshots, extracted graphics, maps or text, and no disassembly listings. `assets/` is ignored except its README, and CI fails if a `.tap`, `.tzx`, `.z80`, `.sna`, `.szx` or `.rom` is committed. What the repository may hold are *facts*: a supported tape's SHA-1, where a program starts, memory addresses, the meaning of the bytes there, and short patches with the bytes they replace. **Definitions hold only facts**, in their authors' own words, plus their authors' own icons.
2. **The player brings the tape.** Each definition supports the dumps it names by SHA-1. Anything else is refused for that game, with a message naming the dump that's expected; it can still be played without the game's extras (§5).
3. **The ROM is bundled.** The emulated Spectrum needs Amstrad's ROMs to load tapes and to run the games that call them. Amstrad has long allowed emulators to distribute the Spectrum ROMs, provided the copyright notice is kept and nothing is charged. They come inside `rustzx-core`, and `THIRD-PARTY.md` carries the notice.
4. **A game's code never changes on disk.** Anything that alters behaviour (training switches, a pad's split button meanings) happens at run time: a write to RAM between frames, or a patch applied to emulated memory after loading. A definition lists each one in one place, with what it changes and why. A patch names the bytes it expects to replace and isn't applied if they differ.
5. **Plain naming and a notice.** Game titles in plain text, no logos or original title lettering, and "not affiliated with or endorsed by the rights holders; you need your own copy" in the README, the player's guide, the release notes and each game's tape prompt.
6. **No money on any game.** No donations, tiers or paid builds tied to a game, which makes a takedown more likely and conflicts with the ROM permission. If funding is ever wanted, ask the rights holders first.
7. **Disassembly stays private.** Finding a game's facts may involve disassembling the player's copy, to understand how it works and nothing else, never to recreate it. The disassembly never leaves `.scratch/`. See `CLAUDE.md`.

## 2. Why an emulator

| | Emulator (this plan) | Rewriting each game |
|---|---|---|
| **Faithful to the original** | By construction: the original's code runs. Timing, sound and contention are right as far as the emulated machine is. | Has to be earned and proven, routine by routine, for every game. |
| **Adding a game** | A definition file: addresses and meanings. | A port. |
| **Legal exposure** | Lower: no translation of any program ships; an emulator imitates hardware. | Higher: a close translation can be argued to be a derivative work. |
| **Needs the ROM at run time** | Yes; it's bundled. | No. |
| **Save games** | A snapshot of the whole machine, at any moment. | The game's state written back, with its own checks. |
| **Changing behaviour** | Memory writes and run-time patches, at addresses the definition names. | Ordinary code. |
| **CPU at 50 frames a second** | A few percent of one core on a desktop; worth measuring on small handhelds (§17). | Negligible. |

## 3. How it's built

A Rust workspace. The emulated machine comes from a fork; everything else is crates of this repository.

| Crate | What it is |
|---|---|
| `rustzx-core`, `rustzx-z80` | The machine, from the [zx-sidekick fork of RustZX](https://github.com/zx-sidekick/rustzx): a git dependency pinned to one commit. `aym` (the AY chip) comes in through `rustzx-core`. The fork's frontend, `rustzx-utils`, `rustzx-test` and `vtx` aren't used. |
| `sidekick-core` | No frontend dependencies, so CI can build and test it anywhere: the frame runner, states, named values, writes, holds and patches, input mapping, saves and records. |
| `sidekick-def` | Loads and checks definitions: serde structs read from TOML or JSON, alone or zipped; validation; a JSON Schema generated with `schemars` and committed, for editors. |
| `sidekick-script` | The script runtime for what plain data can't say (§6.3): [Rhai](https://rhai.rs), pure Rust and sandboxed, with an operation limit per call. No C toolchain, which keeps handheld and libretro builds simple. |
| `sidekick-ui` | Draws the panel, menus, cards and notices in software, into an RGBA buffer (`tiny-skia` and a font rasteriser), so the same code renders headless golden images for tests. |
| `sidekick` | The program: window, sound, gamepads (SDL2, as the fork's own frontend uses, bundled and statically linked; its GameController API reports vendor IDs and buttons by position, and it has a software path for handhelds), sound output through cpal, and the command line. |

## 4. The emulated machine

**`rustzx-core`** from the fork, with the fork's **`rustzx-z80`** processor. The processor matches Fuse's own core tests in 1,346 of 1,356 cases exactly (the other ten are differences in Fuse's log, not in the processor), and passes zexall and the z80test suites, including 1.2a's `z80memptr`; the fork's `ZX-SIDEKICK.md` has the detail. `rustzx-core` already provides:

- **Machines:** the 48K and the 128K, with **memory and I/O contention**. Games like Starquake depend on contention for their speed.
- **The ROMs:** 48K and 128K, embedded (`embedded-roms`).
- **Sound:** the beeper and the AY, mixed into stereo samples.
- **Input:** the keyboard's eight half-rows, the Kempston joystick on port `0x1F`, Sinclair joysticks.
- **Tape:** `.tap`, fast-loaded by trapping the ROM's loader and copying each block straight into memory.
- **Snapshots:** SNA load and save, SZX load.

What ZX Sidekick needs from it that isn't there yet. Each is a small patch to the fork, made there by pull request with its own tests, and recorded in the fork's `ZX-SIDEKICK.md`:

1. **A frame that stops where the game says.** *(Done: fork patches 11 and 12.)* `Machine::run` stops at the definition's sync address, at most once a frame, or at the frame's end (§7), through the core's `DebugInterface`; the core now steps with `Z80::step`, so an interrupt handler's address is seen too, and a frame's end isn't lost over a breakpoint stop.
2. **Writing and inspecting the machine.** *(Done: fork patch 13.)* `Machine::write` leaves the ROM alone and reaches the picture; `registers`/`registers_mut` and `ram_bank` read and set the rest.
3. **`Emulator: Clone`**, for in-memory states and for reading facts on copies of the machine.
4. **Saving SZX.** SZX loads already; saving a 48K SNA pushes the program counter onto the game's own stack, so SZX is the save format.
5. **A 320×256 picture** (the mockups' border) as a setting, beside the core's 320×240.
6. **Sound that follows emulated time.** Some games' sound effects hold the game while they play, so a frame can last much longer than 1/50 s. Check the mixer keeps every sample of such a frame, so effects aren't cut short.
7. **TZX** later (upstream TODO #56); `.tap` covers the first games.

When the first `rustzx-core` patch lands, the fork's `ZX-SIDEKICK.md` is updated. It currently says only `rustzx-z80` is used, and that ZX Sidekick runs without the ROM.

**Checked against outside references, never against itself.** The fork's CI runs the processor's tests, zexall and z80test on every pull request there; core patches add their tests there too. This repository tests what it builds on top.

### Working on the engine

`Cargo.toml` pins both crates to a commit of the fork. To build against a local checkout of the fork while changing it, put a `.cargo/config.toml` in this repository (ignored by git):

```toml
[patch."https://github.com/zx-sidekick/rustzx"]
rustzx-core = { path = "../rustzx/rustzx-core" }
rustzx-z80 = { path = "../rustzx/rustzx-z80" }
```

Once the change is merged in the fork, delete the override and bump the `rev` in `Cargo.toml` to the merge commit.

## 5. Games and definitions

- **Where definitions are found:** `games/` beside the program (the ones shipped with it), and `games/` in the player's data folder (ones the player adds). A definition is a `.toml` or `.json` file, a `.zip`, or a folder in the zip's layout. Dropping a definition onto the window copies it into the data folder.
- **Which game a tape is:** a tape (or the zip it came in), located or dropped onto the window, is matched by SHA-1 against every definition's dumps. The match decides the game. The file name doesn't matter.
- **The library:** when more than one game is installed, the program opens on a library screen of the games it has tapes for, and of those it has definitions for but no tape yet. Choosing one of the latter opens its tape prompt (§8).
- **Plain play:** a tape no definition knows still runs, with the keyboard, and the pad on the Kempston joystick. There's no panel, no training and no kept scores; saves still work.
- **Data folders:** each game keeps its own files in `<data>/<game-id>/`: its tape, kept codes, high scores, saves.
- **The command line:** `sidekick` opens the library; `sidekick TAPE` or `sidekick GAME-ID` starts one game; `sidekick check`, `sidekick author` and `sidekick import-pok` are the authoring tools (§15).

## 6. The definition

The full reference is [`docs/definition-format.md`](docs/definition-format.md). In brief:

### 6.1 Its form

- **One file**, `<game-id>.toml` or `<game-id>.json`: enough for a game that needs no script, icons or crafted situations.
- **A zip**, `<game-id>.zip`, holding `game.toml` or `game.json` at its top, with `scripts/`, `situations/`, `icons/` (the author's own art) and an optional `README.md`. In the repository the same layout is kept as a folder, `games/<game-id>/`, and zipped for releases.
- TOML and JSON describe exactly the same thing. TOML is easier to write by hand: it allows comments, and hex addresses like `0x5E3A`. JSON suits tools; the committed JSON Schema lets editors check either.
- `schema = 1` at the top, so the format can change without breaking old definitions.

### 6.2 What it says

| Section | Holds |
|---|---|
| `[game]` | id, title, year, publisher, machine (`48k`, `128k`), notice |
| `[[media]]` | the dumps supported: name, SHA-1, where to download it, the file names it's distributed under |
| `[run]` | the sync address (the top of the main loop), the loading screen hold, which joystick the game expects |
| `[[states]]` | named moments of the game (`title`, `play`, `paused`, `booth`, `game_over`…), each told by the program counter or a memory test |
| `[values.*]` | named facts: an address and a type, or a table of records; the game's font and graphics, read from memory at run time |
| `[[cheats]]` | training switches: a value set each frame, a value held from going down, or a patch |
| `[pad]` | how a pad drives the game, per state: the D-pad and stick, buttons by position, Start and Select |
| `[[actions]]` | named things a button or key can do: keys, joystick bits, a patch for one frame, a script function |
| `[text_entry]` | typing a word the game asks for with a pad (§12) |
| `[persist.*]` | lists kept between games, such as codes discovered |
| `[high_scores]` | where the game keeps its table, and how its entries are written |
| `[guidance]` | the guidance levels, with their names and descriptions |
| `[[panel]]` | the panel's widgets: what each shows, from which level, and from which value or script result |
| `[situations]` | how a crafted situation enters play (§13) |
| `[script]` | the script and the functions it provides |

Everything but `[game]` and `[[media]]` is optional. A definition with only those, a `[[cheats]]` entry for endless lives and a `[pad]` section is a complete, useful definition.

### 6.3 Scripts

Most of what a definition says is plain data. A few things are computations no data format can describe well: Starquake's map, worked out from its room data, or the shortest route to the nearest missing piece. For those, a definition can include a [Rhai](https://rhai.rs) script.

- `on_load(mem)` runs once, after the tape has loaded: for work done once, such as each room's openings.
- `on_frame(mem, prev, kept)` runs at each sync point and returns facts the panel shows (`routes`, `map`…).
- A script reads memory (`peek`, `peek16`, `value(name)`, `table(name)`), the facts it returned last frame, and the kept lists. It can't write memory, except through values the definition marks `writable`, so every write is still named in one place.
- It has no file, network or clock access, and an operation limit per call; going over the limit turns the script's widgets off for that game with a notice, rather than stalling the frame.

## 7. The program around the game

- **One frame at a time.** The program runs the emulated machine to the next frame boundary, then reads its memory, draws, plays sound and feeds input. Everything in this plan happens between frames.
- **A consistent point between frames.** Most games' main loops have a top: the address they return to once per frame of play. When a definition names it (`[run] sync`), each frame runs *to that address*, not to an arbitrary T-state. Every readout, save and injected write then sees the game between two of its own frames. Without it, frames end at the interrupt. Outside play (title screen, menus, death, game over) games wait in their own loops; the definition's `[[states]]` tell those apart.
- **Reading the game.** `[values]` turn bytes at known addresses into named facts. The panel, the training switches, crafted situations and scripts all use those names, never raw addresses.
- **Writing to the game.** Every write (a training switch, a typed key, a restored save, a crafted situation) goes through one place in `sidekick-core`, by name.

## 8. The window

**Mockup:** every screenshot below shows the window: the picture on the left (the Spectrum's 320×256 with its border, three times over), the panel on the right. A game with no panel widgets gets the whole width.

- **The picture:** scaled by a whole number, never blurred. A game **starts fullscreen**, at the largest whole scale the screen allows. **F11** switches between fullscreen and a window; on a Mac **Control-Command-F** does, since F11 never reaches a program there.
- **The window's title bar** follows the system's light or dark appearance.
- **The tape prompt** appears when a game's tape isn't found. It shows the game's title, a short explanation ("This needs your copy of the game. Nothing from the original is included; it runs from your own tape"), a **Locate the tape…** button, drag-and-drop onto the window, and a link to where the tape can be downloaded, all from the game's `[[media]]`. It accepts the tape, or the `.zip` it's distributed in, in any upper or lower case. A located tape is kept in the game's data folder, so it's found next time. A different dump is refused with a clear message naming the dump that's needed.

  ![The tape prompt](starquake-emulated-mockups/tape-prompt.png)
  ![The tape prompt refusing a different dump](starquake-emulated-mockups/tape-prompt-wrong-dump.png)

  The mockup's text says "the graphics, maps and sound are read from your own tape". The emulator's should say the game *runs* from the player's own tape.
- **The loading screen:** the picture the tape draws as it loads stays up until a key is pressed, as on a real Spectrum, when the definition asks for it (`[run] hold_loading_screen`).
- **A frame's picture during blocking sound effects:** while an effect holds the game, the screen as it was when the effect started stays up. The emulator gets this right as long as it presents the screen at the moment each frame ends.

## 9. Controls

**Keyboard:** exactly as the original: every key reaches the game, which reads it as it always did. The program's own keys are ones the Spectrum doesn't have: **Esc** opens the guidance menu, **F5** saves, **F9** loads, **F11** toggles fullscreen. A definition may bind other keys the Spectrum doesn't have to its `[[actions]]` (Starquake's **Tab** switches between routes).

**Gamepad** (any controller the operating system knows, by USB or Bluetooth, plugged in at any time):
- **Moving:** the D-pad and left stick move. By default, in play, the pad drives the joystick the definition names (`[run] joystick`: Kempston, Sinclair, cursor keys, or the game's own keys), whichever control method the game's menu chose if the game reads them all.
- **Buttons, by position, not by letter.** A definition maps the four face buttons (`west`, `south`, `east`, `north`), the shoulders, **Start** and **Select**, per state. By default the west button fires, Start pauses in play and starts a game from the title screen, and **Select opens the guidance menu**, always.
- **Letters shown in hints follow what the pad reports itself as.** Microsoft's vendor ID gets Xbox letters, Nintendo's gets Nintendo letters (A on the right; A confirms), and Sony's gets the cross, circle, square and triangle. A pad reporting as an Xbox pad shows Xbox letters, whatever is printed on it.
- **Split meanings.** Some games give one direction two meanings, decided in their own code (in Starquake, *up* both picks up and boards the hover platform). A pad can separate them, but only through the game: the definition names the point where the game decides, and an `[[actions]]` entry applies a patch there for one frame, or a write that suppresses one meaning. If that proves fragile for a game, it ships without the split.
- **No accidental actions.** A press is told apart from a hold. A button still held when a menu closes, or when a game starts, is held back from the game until it's let go, so the press that closed the menu doesn't also pause, fire or jump. The pad never types into what the game reads as letters (high-score initials, codes) except through the text-entry card (§12).

## 10. The pause notice

When a definition names its game's paused state, the picture dims there and a card says how to go on: in the definition's own words for the keyboard (for Starquake, any move or fire continues once the pause key is let go), and A or B on a pad.

![Paused](starquake-emulated-mockups/paused.png)

## 11. Guidance and training

### 11.1 The panel

Beside the picture, the panel offers optional help, drawn by the program and never by the game. Nothing in it changes what the game does. A definition defines **guidance levels** from 0 (the original, no help) upward, each adding to the ones below, with a name and a one-line description. It also defines the **widgets** each level adds:

| Widget | Shows |
|---|---|
| `text`, `counter`, `meter` | a value, a number, a bar |
| `chip-list` | short words in the game's own font, at a whole number of screen pixels per game pixel, so every letter is the same size (codes, names) |
| `icon-grid` | pictures in the game's own graphics, each bright, dim or outlined (items needed, delivered, carried) |
| `card-set` | groups of pictures, each dim until something answers it (a door's key cards) |
| `map-grid` | a grid of places: walls along each edge without an opening, so an opening is a gap; markers for the player, items and features; routes in two colours, and arrows at the picture's edge for the way out |

A widget takes its data from a value, a table, or what the script returned. The game's font and graphics are always read from the player's tape at run time; definitions never carry them.

**Levels should be honest about what they tell.** As a convention: the lower levels show only what the player could have written down; the upper ones tell what they couldn't have known. The level's description says which.

Starquake's seven levels are the reference: [`docs/games/starquake.md`](docs/games/starquake.md).

| | |
|---|---|
| ![Level 0](starquake-emulated-mockups/play-level0.png) | ![Level 1](starquake-emulated-mockups/play-level1.png) |
| ![Level 2](starquake-emulated-mockups/play-level2.png) | ![Level 3](starquake-emulated-mockups/play-level3.png) |
| ![Level 4](starquake-emulated-mockups/play-level4.png) | ![Level 5](starquake-emulated-mockups/play-level5.png) |
| ![Level 6](starquake-emulated-mockups/play-level6.png) | |

### 11.2 The guidance menu

Esc, or Select on a pad, opens the menu and pauses the game. It's built from the definition.

![The guidance menu](starquake-emulated-mockups/menu.png)

- **Guidance level:** left and right, or Enter on it to step up and wrap round. Changes take effect at once in the panel. Shown when the definition has levels.
- **Training mode:** one **On/Off** switch per `[[cheats]]` entry, with its label. Each is a write between frames, a value held from going down, or a patch applied while it's on.
- **Use these settings / Back to the game:** applies and returns. If applying would raise what the game's score will show (a higher level than this game has had, or a training switch it hasn't used), a question asks first: "Your score will show this."
- **Save, Load, Open a save…:** three buttons in one row (§13). Left and right move between them.
- **Forget the kept lists:** shown while a `[persist]` list holds something, with the definition's wording (Starquake: "Forget the teleport codes"); asks for a second press.
- **End this game:** shown in play when the definition says how (`[[actions]]` entry `end_game`, such as Starquake's A S D F G); asks for a second press.
- **Exit:** asks for a second press. "Exit Starquake" names the game.

  An action pressed once turns red and says "Press Enter or A again to …"; moving away cancels it.
- **Esc or B leaves** without applying: changes made in the menu are undone.
- **The record:** a game remembers the highest guidance level it used, and which training switches. After a game, that's shown beside the game-over and high-score screens (§13).

## 12. Typing a word with a pad

Some games ask for a word on the keyboard: a code, a password, a name. A definition's `[text_entry]` describes it: the state in which it's asked for, the letters allowed, the length, and where to show the card. On a pad, the first press there opens a card that leaves the game's own echo of the letters visible.

- **Choosing**, when a `[persist]` list holds candidates (and the guidance level allows it): the card opens on them. Up or left goes to the previous one, right or down to the next, round the ends. The title counts them ("2 of 4"), and the panel outlines the chosen one where it shows it.
- **Spelling:** the west button toggles to spelling, and back. Up and down step a slot's letter through the allowed letters, left and right move between slots, and B clears them.
- **A enters** the word either way. It's then typed on the keys the game reads, one letter down for a frame and released the next (timings from the definition), because games often wait for every key to be let go before reading the next letter.
- **While the card is open, the pad drives only the card.** The game sees none of it. Otherwise a held A could keep the game waiting for a release and swallow letters.
- **Keyboard players** never see the card.

| Choosing | Spelling |
|---|---|
| ![Choosing a code](starquake-emulated-mockups/booth-choose.png) | ![Spelling a code](starquake-emulated-mockups/booth-spell.png) |

**Kept lists.** A definition's `[persist]` lists are kept between games in the game's data folder, as plain text (Starquake: every teleport code discovered, in `teleporter-codes.txt`). The panel and the card use them from the start of every later game.

## 13. Scores and saves

**The high-score table**, when a definition describes it (`[high_scores]`), is kept between runs in `high-scores.txt` in the game's data folder, put back into the game's memory when it starts, and read back from it after each game. Each entry carries the highest guidance level its game had.
- **A training game** is shown in the table, then left out of it.
- **A game a save was loaded into** stays in the table, marked "loaded".

![Game over: the record and the table](starquake-emulated-mockups/game-over.png)

**Save games** work for every game, defined or not: a save is a snapshot of the whole machine, stored as a standard `.szx`, with the program's own state alongside (game id, training switches, guidance record, kept-list entries found this game).
- **F5** saves to a quick slot (`quick.sav` in the game's data folder); **F9** loads it, on the title screen too. The menu's row offers the same, plus **Open a save…** for any file, through the system's own dialog. On a Mac that dialog must run on the main thread.
- A short notice confirms: "Game saved", "Game loaded", or why not.

  ![Game saved](starquake-emulated-mockups/saved-note.png)
- **Saved on quitting:** closing the window during a game saves it to its own slot. So does **Cmd-Q** or Quit in the Mac menu, which never arrives as the window closing, only as the program shutting down; handle both. The next start offers it ("Continue where you left off?"): Enter or A continues, anything else starts afresh, and either way it's removed.

  ![The offer to continue](starquake-emulated-mockups/offer.png)
- **Wherever the game is** (paused, typing a code, the menu open), a snapshot can be taken. Take it at the sync point wherever possible, so loading resumes cleanly; otherwise at the end of a frame.
- **A loaded game is marked** beside its score.

**Crafted situations, for testing:** a short text file of `name = value` lines naming only what differs from a new game, using the definition's `[values]` names: the room, the player's position, lives, energy, items carried, codes seen, training switches. The program starts a new game, writes those into memory, then enters play through the routine the definition names (`[situations] enter`). Crafted files hold nothing of the original's, so they can be committed, in the definition's `situations/`. `sidekick TAPE --load FILE` starts from a save or a crafted file, in a window or headless.

## 14. What the player sees for a game without a definition

The same window, menus, saves and pad, minus what needs facts: no panel, no training switches, no kept scores, no pause card, no text-entry card. The menu offers "Find a definition for this game", which opens the project's page of known definitions, and shows the tape's SHA-1 so one can be written for it.

## 15. Authoring tools

What makes "anyone can add a game" real:

- **`sidekick check DEF [TAPE]`:** checks a definition against the schema, that addresses are in range, that names it uses are defined, and, given the tape, that each patch's expected bytes are there once the game has loaded. Every definition in `games/` passes it in CI.
- **`sidekick author TAPE [DEF]`:** runs the game with an overlay for authors:
  - every named value, live;
  - a **cheat finder**: narrow the whole memory down, frame by frame, to the bytes that went down, went up, changed, stayed or equal a number (lose a life, search "went down", repeat);
  - a program-counter sampler, which shows the addresses the game spends its frames at, to find the main loop's top and the states;
  - memory watchpoints, and breakpoints that pause the game;
  - **copy as TOML** for anything found.
- **`sidekick import-pok FILE.pok`:** turns a `.pok` file, the POKE format Fuse and ZXDB use, into `[[cheats]]` entries. Many games' endless lives and the like are already known, so this gives them training switches on day one.
- **Hot reload:** a definition edited while its game runs is reloaded at the next frame, with any error shown in the panel instead of the widgets.
- **The authors' guide**, `docs/definition-format.md`, walks through writing a definition from nothing: find the dump's SHA-1, add a cheat, map the pad, add a value and a widget.

## 16. Finding facts

The emulator needs no translation of any game, but everything a definition says is **facts**: addresses and what the bytes there mean. Find them with whatever is quickest and most accurate (disassembly of the player's copy, tracing, memory search, watchpoints), under the rules in `CLAUDE.md`. Write them down, in your own words, in `docs/re/<game>.md`. Disassemblies stay in `.scratch/`, are used only to understand the game, and are never committed or published.

1. **Trace:** run the game from the tape under scripted input, and record which code runs and in what order. Disassemble what runs, for reading locally.
2. **Diff memory:** compare snapshots before and after an action (pick up an item, enter a room, lose a life), or use the cheat finder.
3. **Watch:** set breakpoints and memory watchpoints on the candidates, to find the main loop's top, the routine that starts play in a room, the pause and death checks, keyboard reads, and the high-score table.
4. **Confirm:** mark each fact **confirmed** (seen running), **read** (understood from the disassembly) or **guess**. A definition's addresses should all be confirmed before it's shipped.

Never circumvent a protected loader to find facts; a protected game loads in the emulator as it would on a Spectrum.

## 17. How it's checked

The games' logic is the originals', so there's no rewrite to prove. What needs proving is the machine, the definitions, and everything read from or written to the machine.

- **The machine** against the outside references, in the fork's CI (§4).
- **Definitions:** every one in `games/` passes `sidekick check`; each round-trips TOML → structs → JSON → structs unchanged; the committed schema matches the structs.
- **The readouts:** from recorded play (input logs, or RZX recordings), assert that what the panel says matches what the game shows: the score, lives and room, the codes it displays, the items picked up.
- **Snapshots:** save, load into a fresh machine, and check the memory and registers are identical and the next 1,000 frames run the same.
- **Crafted situations:** each committed file starts its game, plays scripted input, and must reach its expected event.
- **The panel's look:** each widget, and each Starquake level, rendered headless into PNGs and compared with committed golden images. The panel is the program's own drawing, so its images may be committed; the game's picture behind it is a plain stand-in, and widgets that draw the game's graphics use stand-in graphics in tests.
- **Scripts:** the sandbox holds (no file access, the operation limit stops a runaway loop), and a script over its limit turns off its widgets rather than stalling.
- **Soak:** long runs of random input, looking for crashes, stuck states and readouts that stop making sense.
- **Writes and patches:** for each training switch and split pad meaning, a test that it does what it says and nothing else, for example that endless lives keeps the count and changes nothing else in the frame.

`scripts/check.sh` runs all of it, gates on its exit code, and names whatever it skipped for want of a tape in `assets/`.

## 18. Performance and platforms

- **Desktop** (macOS on Apple silicon and Intel, Windows, Linux): an accuracy-first Spectrum emulator runs a game 50 times or more faster than real time on a current desktop. At real speed that's a few percent of one core. Drawing the scaled picture costs more than emulating. Measure the script's share per frame; Starquake's routes are the heaviest expected.
- **Small handhelds** (for example a Miyoo Mini: Cortex-A7 at about 1.2 GHz, no 3D GPU, 640×480): measure early. An accuracy-first emulator may need a large share of one core there, and the window needs SDL2's software path. The panel won't fit 640×480 beside the picture and would become an overlay.
- **A libretro core** is worth considering once the desktop program works. RetroArch then provides the screen, sound, input and save states on most handhelds and consoles, and Rust produces such a core naturally. Definitions would travel with it unchanged.

## 19. Build order

Each step ends green, committed, and on a reviewed pull request.

1. **The workspace:** this repository, the pinned fork dependency, the repo guard and CI, the ROM smoke test. *(Done.)*
2. **Fork patches** (§4), each a pull request on the fork, the `rev` bumped here after each: the frame loop on `run_until` with a sync address, public writes, registers and banks, `Clone`, SZX saving, the 320×256 picture, sound through blocking effects.
3. **Definitions:** `sidekick-def`, the schema, `sidekick check`; matching tapes by SHA-1; plain play.
4. **The window:** integer scaling, fullscreen and F11, title bar, sound, headless mode with screenshots; the tape prompt from `[[media]]`; the library.
5. **Controls:** keyboard, then the pad (joystick mapping, per-state `[pad]`, Start and Select, maker letters, hold-back).
6. **States, values and the authoring tools:** `sidekick author`, the cheat finder, `import-pok`, hot reload.
7. **Training switches**, the guidance menu, the record, the pause notice.
8. **The panel's widgets** and the **Rhai** runtime, each widget with its render test.
9. **Starquake:** its facts (§16), `games/starquake/`, levels 1 to 6 in order, each with its render test.
10. **The text-entry card** and kept lists.
11. **Kept high scores** with levels and marks.
12. **Saves:** quick slot, Open, saved on quitting, the offer, the loaded mark, crafted situations and `--load`.
13. **Split pad meanings**, if they hold up.
14. **The 128K and the AY** in definitions, and a second, simpler reference definition.
15. **Releases:** archives for each platform, with the player's guide, the authors' guide, `THIRD-PARTY.md` and the font's licence.

## Mockups

All the files in `starquake-emulated-mockups/`, at the window's size of 1368×768. They show Starquake, but the layout, wording and behaviour are the same for every game:

| File | Shows |
|---|---|
| `tape-prompt.png`, `tape-prompt-wrong-dump.png` | The tape prompt, and its refusal of a different dump |
| `play-level0.png` … `play-level6.png` | A game in play at each of Starquake's guidance levels |
| `menu.png` | The guidance menu over the game |
| `paused.png` | The pause notice |
| `booth-choose.png`, `booth-spell.png` | Typing a word with a pad: choosing, and spelling |
| `saved-note.png` | The notice after F5 |
| `offer.png` | The offer to continue the game saved on quitting |
| `game-over.png` | The record and the kept high-score table beside the game over |

Still to be drawn: the library screen, and the menu for a game without a definition (§14).

They contain the original's picture, so like the tape they stay out of the repository.
