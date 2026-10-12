// Starquake's script: the map, routes, and the panel's core, teleports and doors.
// See docs/games/starquake.md and the Scripts section of docs/definition-format.md.
// Routes are still to come.

// The core's own room: entering it delivers what BLOB carries and puts him back next door. The
// items kept there are never placed (docs/re/starquake.md).
fn core_room() {
    199
}

// How many item pictures there are: an item whose picture is past them is an empty record.
fn picture_count() {
    35
}

// An edge's blocking cells after a tile draws over it: the cells it draws take its say.
fn cover(edge, drawn, blocks) {
    (edge & (-1 ^ drawn)) | blocks
}

// Whether an edge of `width` cells has two open cells side by side: a way out for BLOB.
fn opening(blocks, width) {
    let all = (1 << width) - 1;
    let open = all ^ (blocks & all);
    (open & (open >> 1)) != 0
}

// A medium tile's picture, as far as a room's edges need it: along its rows 0, 2 and 5 and
// down its columns 0, 3 and 7, the cells it draws (`d`) and those that block (`b`, an attribute
// without BRIGHT). Along a row, column c is bit 7 - c; down a column, row r is bit r. The
// picture's first six bytes say which of its 8 x 6 cells it draws, row by row, the leftmost
// highest; their attributes are stored before it, the first drawn cell's last.
fn medium(mem, at) {
    let attr = at - 1;
    let rd = [];
    let rb = [];
    let cd0 = 0;
    let cb0 = 0;
    let cd3 = 0;
    let cb3 = 0;
    let cd7 = 0;
    let cb7 = 0;
    for row in 0..6 {
        let drawn = mem.peek(at + row);
        let blocks = 0;
        if drawn != 0 {
            for col in 0..8 {
                let bit = 0x80 >> col;
                if (drawn & bit) != 0 {
                    if (mem.peek(attr) & 0x40) == 0 {
                        blocks = blocks | bit;
                    }
                    attr -= 1;
                }
            }
            cd0 = cd0 | (((drawn >> 7) & 1) << row);
            cb0 = cb0 | (((blocks >> 7) & 1) << row);
            cd3 = cd3 | (((drawn >> 4) & 1) << row);
            cb3 = cb3 | (((blocks >> 4) & 1) << row);
            cd7 = cd7 | ((drawn & 1) << row);
            cb7 = cb7 | ((blocks & 1) << row);
        }
        rd.push(drawn);
        rb.push(blocks);
    }
    #{
        rd0: rd[0], rb0: rb[0], rd2: rd[2], rb2: rb[2], rd5: rd[5], rb5: rb[5],
        cd0: cd0, cb0: cb0, cd3: cd3, cb3: cb3, cd7: cd7, cb7: cb7,
    }
}

// A large tile's say on a room's edges, were it on them: along its top row and its bottom one
// (twelve columns, its own eight and four beyond, column c at bit 15 - c), down its left column
// and its right one (nine rows, its own six and three below, row r at bit r); and the places
// of its doors. Its four medium tiles `m`, of types `kinds`, are drawn at (4, 3), (0, 3), (4, 0)
// and (0, 0) in turn.
fn large(m, kinds) {
    let tile = #{ doors: [] };
    tile.nd = (m[2].rd0 << 4) | (m[3].rd0 << 8);
    tile.nb = cover(m[2].rb0 << 4, m[3].rd0 << 8, m[3].rb0 << 8);
    tile.sd = (m[0].rd2 << 4) | (m[1].rd2 << 8) | (m[2].rd5 << 4) | (m[3].rd5 << 8);
    let sb = cover(m[0].rb2 << 4, m[1].rd2 << 8, m[1].rb2 << 8);
    sb = cover(sb, m[2].rd5 << 4, m[2].rb5 << 4);
    tile.sb = cover(sb, m[3].rd5 << 8, m[3].rb5 << 8);
    tile.wd = (m[1].cd0 << 3) | m[3].cd0;
    tile.wb = cover(m[1].cb0 << 3, m[3].cd0, m[3].cb0);
    tile.ed = (m[0].cd3 << 3) | (m[1].cd7 << 3) | m[2].cd3 | m[3].cd7;
    let eb = cover(m[0].cb3 << 3, m[1].cd7 << 3, m[1].cb7 << 3);
    eb = cover(eb, m[2].cd3, m[2].cb3);
    tile.eb = cover(eb, m[3].cd7, m[3].cb7);
    // A security door: a medium tile whose type has its top four bits clear, the rest its place.
    let offsets = [[4, 3], [0, 3], [4, 0], [0, 0]];
    for k in 0..4 {
        let kind = kinds[k];
        if kind != 0 && (kind & 0xF0) == 0 {
            tile.doors.push([offsets[k][0] + (kind & 3), offsets[k][1] + ((kind >> 2) & 3)]);
        }
    }
    tile
}

// Once, after loading: each room's walls and security doors, decoded from the room data the way
// the game draws a room (docs/re/starquake.md, "The rooms"): 4 x 3 large tiles of 8 x 6 cells,
// drawn in turn, later cells over earlier ones.
fn on_load(mem) {
    let rooms = mem.value("rooms");
    let bigs = mem.value("large_tiles");
    let types = mem.value("tile_types");
    let pictures = mem.value("tile_pictures");
    if rooms == () || bigs == () || types == () || pictures == () {
        return #{ walls: [], cells: [], doors: [] };
    }
    // Only the medium tiles the large tiles use.
    let mediums = [];
    mediums.pad(256, ());
    for id in bigs {
        if mediums[id] == () {
            mediums[id] = medium(mem, pictures[id * 2] + pictures[id * 2 + 1] * 256);
        }
    }
    let larges = [];
    let with_doors = [];
    for big in 0..256 {
        let m = [];
        let kinds = [];
        for id in bigs.extract(big * 4, 4) {
            m.push(mediums[id]);
            kinds.push(types[id]);
        }
        let tile = large(m, kinds);
        if tile.doors.len() > 0 {
            with_doors.push(big);
        }
        larges.push(tile);
    }

    let walls = [];
    let cells = [];
    let doors = [];
    for room in 0..512 {
        let at = room * 12;
        // Along the top and bottom, column c at bit 39 - c; down the sides, row r at bit r.
        let north = 0;
        let south = 0;
        let west = 0;
        let east = 0;
        for i in 0..4 {
            let shift = 24 - i * 8;
            let top = larges[rooms[at + i]];
            north = (north & (-1 ^ (top.nd << shift))) | (top.nb << shift);
            let bottom = larges[rooms[at + 8 + i]];
            south = (south & (-1 ^ (bottom.sd << shift))) | (bottom.sb << shift);
        }
        for i in 0..3 {
            let shift = i * 6;
            let left = larges[rooms[at + i * 4]];
            west = (west & (-1 ^ (left.wd << shift))) | (left.wb << shift);
            let right = larges[rooms[at + i * 4 + 3]];
            east = (east & (-1 ^ (right.ed << shift))) | (right.eb << shift);
        }
        let shut = "";
        if !opening(north >> 8, 32) {
            shut += "N";
        }
        if !opening(east, 18) {
            shut += "E";
        }
        if !opening(south >> 8, 32) {
            shut += "S";
        }
        if !opening(west, 18) {
            shut += "W";
        }
        walls.push(shut);
        cells.push(#{ x: room % 16, y: room / 16, walls: shut, seen: true });
        let places = [];
        for i in 0..12 {
            let big = rooms[at + i];
            if with_doors.contains(big) {
                for place in larges[big].doors {
                    places.push([(i % 4) * 8 + place[0], (i / 4) * 6 + place[1]]);
                }
            }
        }
        if places.len() > 0 {
            doors.push(#{ room: room, places: places });
        }
    }
    // The whole planet's map cells, as level 6 shows them, made once.
    #{ walls: walls, cells: cells, doors: doors }
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

// A security door's three cards, worked out as the game does on meeting it: from the game's
// seed, the room's low byte, and the two numbers the door hands the card routine, each brought
// into pictures 9 to 13 (docs/re/starquake.md, "Security doors").
fn door_cards(seed, room) {
    let high = seed >> 8;
    let first = 0x11 ^ high ^ (room & 0xFF);
    let second = first ^ (seed & 0xFF) ^ 0x0B;
    let third = second ^ high ^ 0x11;
    [first, second, third].map(|card| (card & 0x3F) % 5 + 9)
}

// Every door on the planet, by room, with its cards for this game.
fn every_door(mem, loaded) {
    let seed = mem.value("seed");
    let all = [];
    for door in loaded.doors {
        all.push(#{ room: door.room, cards: door_cards(seed, door.room) });
    }
    all
}

// The doors for the panel, numbered in order, each card answered when BLOB carries it.
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

// The room BLOB is in, as a kept entry: the game's seed, the room and its walls. Kept once a
// game, the first time its walls read the same two frames running in play, so a room drawn
// part-way, or a platform built across an opening later, doesn't change its walls.
fn room_walked(mem, prev, shut, walked) {
    let room = mem.value("room");
    if mem.state != "play" || `${room}` in walked || prev == () || !("room" in prev) {
        return ();
    }
    if prev.room != room || prev.walls != shut {
        return ();
    }
    `${mem.value("seed")}:${room}:${shut}`
}

// The rooms walked this game, by room number as text, each its walls.
fn rooms_walked(mem, kept) {
    let seed = `${mem.value("seed")}:`;
    let rooms = #{};
    if "rooms-walked" in kept {
        for entry in kept["rooms-walked"] {
            if entry.starts_with(seed) {
                let parts = entry.split(":");
                if !(parts[1] in rooms) {
                    rooms[parts[1]] = if parts.len() > 2 { parts[2] } else { "" };
                }
            }
        }
    }
    rooms
}

// The pictures of the core pieces still to deliver.
fn needed(mem) {
    let pictures = [];
    for slot in mem.table("core_slots") {
        if slot.missing {
            pictures.push(slot.slot & 0x7F);
        }
    }
    pictures
}

// The items lying in rooms, each with its room and kind: a core piece still needed, or another
// item colored by what it does (a door's card, the key to the locked doors, anything else).
// Not those picked up, nor those kept in the core's room.
fn items(mem) {
    let pieces = needed(mem);
    let lying = [];
    for item in mem.table("items") {
        let picture = item.picture;
        if picture >= picture_count() || (item.second & 0x7F) == 1 {
            continue;
        }
        let room = item.room_low + (item.second >> 7) * 256;
        if room == core_room() {
            continue;
        }
        let mark = #{ kind: "item", x: room % 16, y: room / 16, room: room, picture: picture };
        if pieces.contains(picture) {
            mark.kind = "piece";
            mark.color = "first";
        } else if picture >= 9 && picture <= 13 {
            mark.color = 5;
        } else if picture == 16 {
            mark.color = 6;
        } else {
            mark.color = 7;
        }
        lying.push(mark);
    }
    lying
}

// The map: the rooms walked, or at level 6 every room, with their walls; the teleports and
// doors in them; from level 3 the items in rooms walked, from level 4 in every room; BLOB's room.
fn map(mem, loaded, kept, rooms, level) {
    let everything = level >= 6 && loaded.cells.len() == 512;
    let cells = [];
    if everything {
        cells = loaded.cells;
    } else {
        for key in rooms.keys() {
            let room = parse_int(key);
            cells.push(#{ x: room % 16, y: room / 16, walls: rooms[key], seen: true });
        }
    }
    let markers = [];
    for teleport in mem.table("teleports") {
        if everything || `${teleport.room}` in rooms {
            markers.push(#{ kind: "teleport", x: teleport.room % 16, y: teleport.room / 16 });
        }
    }
    let doors = if everything { loaded.doors } else { doors_met(mem, kept) };
    for door in doors {
        markers.push(#{ kind: "door", x: door.room % 16, y: door.room / 16 });
    }
    if level >= 3 {
        for item in items(mem) {
            if level >= 4 || `${item.room}` in rooms {
                item.remove("room");
                markers.push(item);
            }
        }
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
    let shut = walls(mem);
    let rooms = rooms_walked(mem, kept);
    let walked = room_walked(mem, prev, shut, rooms);
    if walked != () {
        keep["rooms-walked"] = [walked];
    }
    let listed = if level >= 6 && "doors" in loaded {
        every_door(mem, loaded)
    } else {
        doors_met(mem, kept)
    };
    #{
        core: core(mem, carrying),
        teleports: teleports(mem, kept, level),
        doors: doors(listed, carrying),
        map: map(mem, loaded, kept, rooms, level),
        kept: keep,
        room: mem.value("room"),
        walls: shut,
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
