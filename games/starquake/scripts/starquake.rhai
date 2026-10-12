// Starquake's script: the map, routes, and the panel's core, teleports and doors.
// See docs/games/starquake.md and the Scripts section of docs/definition-format.md.
// The map and routes are still to come: their facts aren't found yet.

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
    #{
        core: core(mem, carrying),
        teleports: teleports(mem, kept, level),
        doors: doors(doors_met(mem, kept), carrying),
        map: (),
        kept: keep,
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
