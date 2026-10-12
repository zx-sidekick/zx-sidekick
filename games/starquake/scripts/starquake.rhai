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

// The doors met this game: when the last door's cards change, the room BLOB is in has a door
// with those cards. Kept in the results, and started afresh with each game's seed.
fn doors_met(mem, prev) {
    let seed = mem.value("seed");
    let met = if prev != () && "doors_met" in prev && prev.seed == seed {
        prev.doors_met
    } else {
        []
    };
    let raw = mem.value("door_cards");
    let cards = [raw[0], raw[2], raw[4]];
    let last = if prev != () && "last_cards" in prev { prev.last_cards } else { cards };
    let valid = cards.all(|c| c >= 9 && c <= 13);
    if valid && cards != last {
        let room = mem.value("room");
        if !met.some(|door| door.room == room) {
            met.push(#{ room: room, cards: cards });
        }
    }
    #{ seed: seed, met: met, cards: cards }
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
    let met = doors_met(mem, prev);
    let keep = #{};
    let shown = mem.value("booth_code");
    if mem.state == "booth" && is_code(shown) {
        keep["teleport-codes"] = [shown];
    }
    #{
        core: core(mem, carrying),
        teleports: teleports(mem, kept, level),
        doors: doors(met.met, carrying),
        map: (),
        kept: keep,
        seed: met.seed,
        doors_met: met.met,
        last_cards: met.cards,
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
