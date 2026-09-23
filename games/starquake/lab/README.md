# starquake-lab

Tools for studying the game on the player's own tape: pictures, probes and the level 5 search (#10, #39). They are not checks: `starquake-check` proves things, these look at them. Nothing here runs in CI or in the gate, which only builds them.

Every tool takes the assets folder first, reads `starquake.tap` from it and writes whatever it produces into it. That folder is ignored by git, so nothing derived from the game is ever committed (`GOAL.md`, rule 1). Run one with `cargo run --release -p starquake-lab --bin <tool> -- "$SK_ASSETS" …`.

## Pictures

- `planet <dir> [--half]`: the whole planet as one picture, `planet.png`: every room as the game draws it, with its openings (green bars), walls inside a divided room (orange; a door's dashed yellow), wall passages (purple), lift cells (green boxes), the start room (white) and the core (pink), and rooms not reachable from the start with doors shut dimmed.
- `tiles <dir> <room>[,<room>…] [--png] [--stack]`: rooms as text side by side (`#` solid, `.` free, `=` a lift); `--png` writes each as the game draws it (`room-N.png`), `--stack` all of them in one picture top to bottom.

## Probes

- `bars <dir> [frames]`: the energy, bridging platform and laser bars as play starts, the panel's loop that draws them and caps each at 127, and what laying bridging platforms and firing take from them (#104).
- `markers <dir> <room>[,<room>…]`: a room's markers, the three bytes each of its tiles leaves, with the cell each is at.
- `attrs <dir> <room> [first-col] [last-col]`: the attribute bytes of a room's cells.
- `survey <dir>`: every attribute value used across all rooms, and the rooms holding lift cells.
- `door <dir> <room> <x> <y> [input] [--carry=G,G,...]`: walks Blob into a security door and runs its screen on a copy: the three items it asks for, whether it opens with what he carries, and where he ends up (#33).
- `fall <dir> <room> <x> <y> [input] [frames] [--carry=G]`: stands Blob somewhere with one input held and prints where he goes; `--carry=G` first puts the item drawn with graphic G in his inventory (16 opens a teleporter pad). Standing with his top cell in room row R reads `y = 143 - 8R`.

## The search

The level 5 spike's search, on copies of the machine: from a way into a room, every input on every frame play reads it, each state of Blob kept once, until he leaves the room, dies, or a door, booth or pyramid screen opens. Its positive findings replay; its negative findings are not to be trusted, as the report on #10 says. Flags: `--full` (the full state key; the coarse one is the default), `--diagonals`, `--no-prune`, `--no-platforms`, `--hold=N`.

- `whole <dir> [threads] [minutes]`: the whole map from the start, writing `exits.txt`, which the three comparisons read.
- `validate <dir> [walks] [entries]`: replays every exit found and looks for exits random walks take that the search missed.
- `variant <dir> [room]`: one room under the coarse and full keys, with and without diagonals.
- `probe <dir> <room>[,<room>…] [entries] [--coarse]`: one room again, from the entries `exits.txt` records into it, with the full key.
- `residual <dir>`: every open edge the search never crossed, and whether the map's parts explain it; the (room, part) graph's reach against the search's.
- `gaps <dir>`: how wide the gaps the search crossed are, against the map's.
- `route <dir> [cold|warm] [threads] [seconds]`: the "verify only the route" idea, measured and found wanting.
