// Starquake's script: the map, routes, and the panel's core, teleports and doors.
// See docs/games/starquake.md and the Scripts section of docs/definition-format.md.
// Routes, and the rooms not reached, are still to come.

// Once, after loading: each room's openings and the graph of places, from the room map.
fn on_load(mem) {
    #{}
}

// The pictures BLOB carries.
fn carried(mem) {
    let pictures = [];
    for slot in mem.table("carried") {
        if slot.color != 0 {
            pictures.push(slot.picture);
        }
    }
    pictures
}

// The core's nine slots: delivered bright, carried outlined, the rest dim.
fn core(mem, carrying) {
    let slots = [];
    for slot in mem.table("core_slots") {
        let picture = slot.slot & 0x7F;
        let look = if !slot.missing {
            "bright"
        } else if carrying.contains(picture) {
            "outlined"
        } else {
            "dim"
        };
        slots.push(#{ picture: picture, look: look });
    }
    slots
}

// The codes to list: those kept, or at level 6 every teleport's.
fn teleports(mem, kept, level) {
    let codes = [];
    if level >= 6 {
        for teleport in mem.table("teleports") {
            codes.push(teleport.code);
        }
    } else if "teleport-codes" in kept {
        codes = kept["teleport-codes"];
    }
    codes
}

// A code as the booth shows it: five letters, A to Z.
fn is_code(text) {
    if text.len() != 5 {
        return false;
    }
    for c in text.chars() {
        if c < 'A' || c > 'Z' {
            return false;
        }
    }
    true
}

// The door BLOB is meeting, as a kept entry: the game's seed, the room and the three cards.
// `()` unless the game is in its door routine with cards worked out.
fn door_met(mem) {
    if mem.state != "door" || mem.value("door_card_count") != 3 {
        return ();
    }
    let raw = mem.value("door_cards");
    let cards = [raw[0], raw[2], raw[4]];
    if !cards.all(|c| c >= 9 && c <= 13) {
        return ();
    }
    `${mem.value("seed")}:${mem.value("room")}:${cards[0]}-${cards[1]}-${cards[2]}`
}

// The doors met this game, from the kept entries with this game's seed, in the order met.
fn doors_met(mem, kept) {
    let seed = `${mem.value("seed")}:`;
    let met = [];
    if "doors-met" in kept {
        for entry in kept["doors-met"] {
            if entry.starts_with(seed) {
                let parts = entry.split(":");
                let cards = [];
                for card in parts[2].split("-") {
                    cards.push(parse_int(card));
                }
                met.push(#{ room: parse_int(parts[1]), cards: cards });
            }
        }
    }
    met
}

// The doors for the panel, numbered as met, each card answered when BLOB carries it.
fn doors(met, carrying) {
    let rows = [];
    let number = 1;
    for door in met {
        let cards = [];
        for card in door.cards {
            cards.push(#{ picture: card, answered: carrying.contains(card) });
        }
        rows.push(#{ label: number, cards: cards });
        number += 1;
    }
    rows
}

// Whether two or more cells in a row let BLOB through: cells with BRIGHT in their attribute.
fn open(cells) {
    let run = 0;
    for attr in cells {
        if (attr & 0x40) != 0 {
            run += 1;
            if run >= 2 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

// The edges of the room on screen without an opening: some of N, E, S and W.
fn walls(mem) {
    let area = mem.value("play_area");
    let west = [];
    let east = [];
    for row in 0..18 {
        west.push(area[row * 32]);
        east.push(area[row * 32 + 31]);
    }
    let shut = "";
    if !open(area.extract(0, 32)) {
        shut += "N";
    }
    if !open(east) {
        shut += "E";
    }
    if !open(area.extract(17 * 32, 32)) {
        shut += "S";
    }
    if !open(west) {
        shut += "W";
    }
    shut
}

// The room BLOB is in, as a kept entry, once he's been in it since the last frame (so it's
// drawn whole): the game's seed, the room and its walls.
fn room_walked(mem, prev) {
    let room = mem.value("room");
    if mem.state != "play" || prev == () || !("room" in prev) || prev.room != room {
        return ();
    }
    `${mem.value("seed")}:${room}:${walls(mem)}`
}

// The rooms walked this game, by room number as text, each its walls; the latest entry wins.
fn rooms_walked(mem, kept) {
    let seed = `${mem.value("seed")}:`;
    let rooms = #{};
    if "rooms-walked" in kept {
        for entry in kept["rooms-walked"] {
            if entry.starts_with(seed) {
                let parts = entry.split(":");
                rooms[parts[1]] = if parts.len() > 2 { parts[2] } else { "" };
            }
        }
    }
    rooms
}

// The map of rooms walked: their walls, the teleports and doors in them, and BLOB's room.
fn map(mem, kept) {
    let rooms = rooms_walked(mem, kept);
    let cells = [];
    for key in rooms.keys() {
        let room = parse_int(key);
        cells.push(#{ x: room % 16, y: room / 16, walls: rooms[key], seen: true });
    }
    let markers = [];
    for teleport in mem.table("teleports") {
        if `${teleport.room}` in rooms {
            markers.push(#{ kind: "teleport", x: teleport.room % 16, y: teleport.room / 16 });
        }
    }
    for door in doors_met(mem, kept) {
        markers.push(#{ kind: "door", x: door.room % 16, y: door.room / 16 });
    }
    let here = mem.value("room");
    markers.push(#{ kind: "player", x: here % 16, y: here / 16 });
    #{ width: 16, height: 32, cells: cells, markers: markers, routes: [] }
}

// At each sync point: the panel's data, and codes to keep.
fn on_frame(mem, loaded, prev, kept, level) {
    let carrying = carried(mem);
    let keep = #{};
    let shown = mem.value("booth_code");
    if mem.state == "booth" && is_code(shown) {
        keep["teleport-codes"] = [shown];
    }
    let door = door_met(mem);
    if door != () {
        keep["doors-met"] = [door];
    }
    let walked = room_walked(mem, prev);
    if walked != () {
        keep["rooms-walked"] = [walked];
    }
    #{
        core: core(mem, carrying),
        teleports: teleports(mem, kept, level),
        doors: doors(doors_met(mem, kept), carrying),
        map: map(mem, kept),
        kept: keep,
        room: mem.value("room"),
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
