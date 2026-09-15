# Spike: where Blob can get to (#10)

Throwaway code for the level 5 spike, kept on this branch as the record behind the report on #10. It is not part of the workspace, not built by CI, and never merged as it is.

It runs the original game on copies of the machine: from a way into a room, every input on every frame play reads it, keeping each state of Blob once, until he leaves the room, dies, or opens a door or booth screen. Nothing in it moves Blob itself.

```bash
T="$SK_ASSETS/starquake.tap"
cargo run --release --bin validate -- "$T" 80 platforms 8   # replay every exit found; random walks look for exits missed
cargo run --release --bin variant -- "$T"                    # one room, to compare keys and inputs
PRUNE=1 MASK=5,6,17,20 NO_DIAG=1 cargo run --release --bin whole2 -- "$T" 11 40            # the whole map from the start
cargo run --release --bin tiles -- "$T" 226,227              # two rooms' cells side by side
```

- `MASK` picks Blob's slot bytes that make a state (default: all but counters). `NO_DIAG` leaves diagonals out. `PRUNE` drops states with Blob inside a solid cell.
- `whole2` writes every exit it found to `exits.txt`, which is derived from the game: never commit it.

## The re-evaluation (2026-09-15, later)

Scratch tools behind the second report on #10, which compares the search with the map's own reading:

```bash
cargo run --release --bin residual -- "$T" exits.txt          # open edges out of reached rooms the search never crossed, and whether the map's parts explain them; the parts graph's reach
PRUNE=1 NO_DIAG=1 cargo run --release --bin probe -- "$T" exits.txt 52,99,102,167,241,244,290 4   # one room again, from the entries the whole-map run found, with the FULL key
cargo run --release --bin gaps -- "$T" exits.txt              # gap widths on the map against the widths the search crossed
cargo run --release --bin fall -- "$T" 244,40,39,0 290,40,39,0 # stand Blob at (x, y) with an input held and watch whether he falls (y = 143 - 8 * room row of his top cell)
cargo run --release --bin markers -- "$T" 244 260             # a room's markers and their cells
cargo run --release --bin attrs -- "$T" 244,3,8               # attribute bytes of a room's cells in a column range
cargo run --release --bin survey -- "$T"                      # every attribute value used across all rooms; the rooms holding the lift's
```
