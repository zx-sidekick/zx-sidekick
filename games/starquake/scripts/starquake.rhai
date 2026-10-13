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

// Whether an edge has a way out for BLOB, who is two cells by two: two cells side by side, open
// in both the edge's own row (or column) and the one next to it. `blocks` holds the cells that
// block in either, one bit a cell, `width` of them.
fn opening(blocks, width) {
    let all = (1 << width) - 1;
    let open = all ^ (blocks & all);
    (open & (open >> 1)) != 0
}

// A medium tile's picture, as far as a room's edges need it: along each of its rows and down
// its columns 0 to 3, 6 and 7, the cells it draws (`d`) and those that block (`b`, an attribute
// without BRIGHT). Along a row, column c is bit 7 - c; down a column, row r is bit r. The
// picture's first six bytes say which of its 8 x 6 cells it draws, row by row, the leftmost
// highest; their attributes are stored before it, the first drawn cell's last.
fn medium(mem, at) {
    let attr = at - 1;
    let rd = [];
    let rb = [];
    let cd = [0, 0, 0, 0, 0, 0, 0, 0];
    let cb = [0, 0, 0, 0, 0, 0, 0, 0];
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
            for col in [0, 1, 2, 3, 6, 7] {
                cd[col] = cd[col] | (((drawn >> (7 - col)) & 1) << row);
                cb[col] = cb[col] | (((blocks >> (7 - col)) & 1) << row);
            }
        }
        rd.push(drawn);
        rb.push(blocks);
    }
    #{ rd: rd, rb: rb, cd: cd, cb: cb }
}

// One row of a large tile, twelve columns (its own eight and four beyond, column c at bit
// 15 - c), as its medium tiles `m` draw it: those three rows down at their row `low` (-1: they
// don't reach it), then those at the top at their row `high`. As `[drawn, blocks]`.
fn row_of(m, low, high) {
    let d = 0;
    let b = 0;
    if low >= 0 {
        d = (m[0].rd[low] << 4) | (m[1].rd[low] << 8);
        b = cover(m[0].rb[low] << 4, m[1].rd[low] << 8, m[1].rb[low] << 8);
    }
    d = d | (m[2].rd[high] << 4) | (m[3].rd[high] << 8);
    b = cover(b, m[2].rd[high] << 4, m[2].rb[high] << 4);
    b = cover(b, m[3].rd[high] << 8, m[3].rb[high] << 8);
    [d, b]
}

// One column of a large tile, nine rows (its own six and three below, row r at bit r): the
// medium tiles four across at their column `right` (-1: they don't reach it), those at the left
// at their column `left`, drawn in turn. As `[drawn, blocks]`.
fn col_of(m, right, left) {
    let d = 0;
    let b = 0;
    if right >= 0 {
        d = m[0].cd[right] << 3;
        b = m[0].cb[right] << 3;
    }
    d = d | (m[1].cd[left] << 3);
    b = cover(b, m[1].cd[left] << 3, m[1].cb[left] << 3);
    if right >= 0 {
        d = d | m[2].cd[right];
        b = cover(b, m[2].cd[right], m[2].cb[right]);
    }
    d = d | m[3].cd[left];
    b = cover(b, m[3].cd[left], m[3].cb[left]);
    [d, b]
}

// An edge's two rows across the room, the edge's own in the top 32 bits, the next in the low:
// a large tile's twelve columns from `row_of`, placed at large tile column `i`, column c of the
// room at bit 31 - c of its half; what falls past the room is dropped.
fn across(pair, i) {
    let shift = 16 - 8 * i;
    let halves = [0, 0];
    for k in 0..2 {
        let bits = if shift >= 0 { pair[k] << shift } else { pair[k] >> -shift };
        halves[k] = bits & 0xFFFFFFFF;
    }
    (halves[0] << 32) | halves[1]
}

// An edge's two columns down the room, likewise: a large tile's nine rows from `col_of`, placed
// at large tile row `i`, row r of the room at bit r of its half.
fn down(pair, i) {
    ((pair[0] << (6 * i)) & 0x3FFFF) << 32 | ((pair[1] << (6 * i)) & 0x3FFFF)
}

// Two rows or columns, each `[drawn, blocks]`, as `[drawn pairs, blocks pairs]`.
fn pairs(first, second) {
    [[first[0], second[0]], [first[1], second[1]]]
}

// A large tile's say on a room's edges, at each place it could take on them: its top two rows,
// its bottom two, its left two columns and its right two, the edge's own first, packed by
// `across` and `down` (`nd` and `nb`: north, drawn and blocking, by column; and so on); and the
// places of its doors. Its four medium tiles `m`, of types `kinds`, are drawn at (4, 3), (0, 3),
// (4, 0) and (0, 0) in turn.
fn large(m, kinds) {
    let n = pairs(row_of(m, -1, 0), row_of(m, -1, 1));
    let s = pairs(row_of(m, 2, 5), row_of(m, 1, 4));
    let w = pairs(col_of(m, -1, 0), col_of(m, -1, 1));
    let e = pairs(col_of(m, 3, 7), col_of(m, 2, 6));
    let tile = #{ nd: [], nb: [], sd: [], sb: [], wd: [], wb: [], ed: [], eb: [], doors: [] };
    for i in 0..4 {
        tile.nd.push(across(n[0], i));
        tile.nb.push(across(n[1], i));
        tile.sd.push(across(s[0], i));
        tile.sb.push(across(s[1], i));
    }
    for i in 0..3 {
        tile.wd.push(down(w[0], i));
        tile.wb.push(down(w[1], i));
        tile.ed.push(down(e[0], i));
        tile.eb.push(down(e[1], i));
    }
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

// The cells of an edge's two rows or columns that block in either, from a packed pair.
fn either(packed) {
    ((packed >> 32) & 0xFFFFFFFF) | (packed & 0xFFFFFFFF)
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
        // Each edge's two rows or columns, packed as `across` and `down` place them.
        let north = 0;
        let south = 0;
        let west = 0;
        let east = 0;
        for i in 0..4 {
            let top = larges[rooms[at + i]];
            north = (north & (-1 ^ top.nd[i])) | top.nb[i];
            let bottom = larges[rooms[at + 8 + i]];
            south = (south & (-1 ^ bottom.sd[i])) | bottom.sb[i];
        }
        for i in 0..3 {
            let left = larges[rooms[at + i * 4]];
            west = (west & (-1 ^ left.wd[i])) | left.wb[i];
            let right = larges[rooms[at + i * 4 + 3]];
            east = (east & (-1 ^ right.ed[i])) | right.eb[i];
        }
        let shut = "";
        if !opening(either(north), 32) {
            shut += "N";
        }
        if !opening(either(east), 18) {
            shut += "E";
        }
        if !opening(either(south), 32) {
            shut += "S";
        }
        if !opening(either(west), 18) {
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

// Every door on the planet with its cards for this game: those met first, keeping the numbers
// they had, then the rest by room.
fn every_door(mem, loaded, met) {
    let seed = mem.value("seed");
    let all = met;
    let rooms = met.map(|door| door.room);
    for door in loaded.doors {
        if !rooms.contains(door.room) {
            all.push(#{ room: door.room, cards: door_cards(seed, door.room) });
        }
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

// The edges of the room on screen without a way out for BLOB, by the same rule as the decoded
// rooms': some of N, E, S and W. A cell blocks when its attribute lacks BRIGHT.
fn walls(mem) {
    let area = mem.value("play_area");
    let across = [0, 0, 0, 0];
    let rows = [0, 1, 17, 16];
    for k in 0..4 {
        for col in 0..32 {
            if (area[rows[k] * 32 + col] & 0x40) == 0 {
                across[k] = across[k] | (1 << (31 - col));
            }
        }
    }
    let down = [0, 0, 0, 0];
    let cols = [0, 1, 31, 30];
    for k in 0..4 {
        for row in 0..18 {
            if (area[row * 32 + cols[k]] & 0x40) == 0 {
                down[k] = down[k] | (1 << row);
            }
        }
    }
    let shut = "";
    if !opening(across[0] | across[1], 32) {
        shut += "N";
    }
    if !opening(down[2] | down[3], 18) {
        shut += "E";
    }
    if !opening(across[2] | across[3], 32) {
        shut += "S";
    }
    if !opening(down[0] | down[1], 18) {
        shut += "W";
    }
    shut
}

// The rooms walked this game, by room number as text, each with its walls as decoded: those
// whose bit is clear in the game's own set of rooms not yet visited, a bit a room, the first
// room the top bit of the first byte (docs/re/starquake.md).
fn rooms_walked(mem, loaded) {
    let unvisited = mem.value("visited");
    let rooms = #{};
    if unvisited == () || loaded.walls.len() != 512 {
        return rooms;
    }
    for byte in 0..64 {
        let bits = unvisited[byte];
        if bits != 0xFF {
            for bit in 0..8 {
                if (bits & (0x80 >> bit)) == 0 {
                    let room = byte * 8 + bit;
                    rooms[`${room}`] = loaded.walls[room];
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

// The items lying in rooms, each with its room and kind, by what the game does with its
// picture (docs/re/starquake.md): a core piece still needed, but not the twin of one BLOB
// carries; a card, 9 to 13 for doors and 14 and 15 that answer any card; the key, 16; anything
// else is a core piece a pyramid takes. Packs (17 to 24) act when picked up: nothing to fetch.
// Not those off the floor (rows 1 to 5: the game lays an item down at row 6 or more, and a
// carried one has a row below that), nor those kept in the core's room.
fn items(mem, carrying) {
    let pieces = needed(mem);
    let lying = [];
    for item in mem.table("items") {
        let picture = item.picture;
        let row = item.second & 0x7F;
        let pack = picture >= 17 && picture <= 24;
        if picture >= picture_count() || pack || (row >= 1 && row <= 5) {
            continue;
        }
        let room = item.room_low + (item.second >> 7) * 256;
        if room == core_room() {
            continue;
        }
        let mark = #{ kind: "item", x: room % 16, y: room / 16, room: room, picture: picture };
        if pieces.contains(picture) && !carrying.contains(picture) {
            mark.kind = "piece";
            mark.color = "first";
        } else if picture >= 9 && picture <= 15 {
            mark.color = "#C8A2E8";
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
fn map(mem, loaded, kept, rooms, level, carrying) {
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
        for item in items(mem, carrying) {
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
    let rooms = rooms_walked(mem, loaded);
    let met = doors_met(mem, kept);
    let listed = if level >= 6 && "doors" in loaded {
        every_door(mem, loaded, met)
    } else {
        met
    };
    #{
        core: core(mem, carrying),
        teleports: teleports(mem, kept, level),
        doors: doors(listed, carrying),
        map: map(mem, loaded, kept, rooms, level, carrying),
        kept: keep,
        room: mem.value("room"),
        walls: walls(mem),
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
