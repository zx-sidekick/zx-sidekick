// Starquake's script: the map, routes, and the panel's core, teleports and doors.
// See docs/games/starquake.md and the Scripts section of docs/definition-format.md.
// A stub until the facts it reads are found: every result is empty.

// Once, after loading: each room's openings and the graph of places, from the room map.
fn on_load(mem) {
    #{}
}

// At each sync point: the panel's data, and codes to keep.
fn on_frame(mem, loaded, prev, kept) {
    #{ core: [], teleports: [], doors: [], map: (), kept: #{} }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
