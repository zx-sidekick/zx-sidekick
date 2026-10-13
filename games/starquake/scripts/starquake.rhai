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

// A room's size in cells, across and down.
fn across() {
    32
}

fn down() {
    18
}

// The ways a part of a room leads out, a bit each.
fn north() {
    1
}

fn east() {
    2
}

fn south() {
    4
}

fn west() {
    8
}

// What a medium tile's type says it is, in its top four bits (docs/re/starquake.md, "Medium
// tile types"): a security door's entry, a locked door's post, a wall passage.
fn door_entry() {
    0
}

fn post() {
    11
}

fn passage() {
    15
}

// A row's blocking cells after a tile draws over it: the cells it draws take its say.
fn cover(row, drawn, blocks) {
    (row & (-1 ^ drawn)) | blocks
}

// A medium tile's picture as its six rows of eight cells, each `[drawn, blocks]`: the cells it
// draws, and those that block (an attribute without BRIGHT), column c at bit 7 - c. The
// picture's first six bytes say which cells it draws, row by row, the leftmost highest; their
// attributes are stored before it, the first drawn cell's last.
fn medium(mem, at) {
    let attr = at - 1;
    let rows = [];
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
        }
        rows.push([drawn, blocks]);
    }
    rows
}

// Where a large tile's four medium tiles are drawn within it, in turn.
fn offsets() {
    [[4, 3], [0, 3], [4, 0], [0, 0]]
}

// A large tile as its medium tiles `m` draw it, in turn, later cells over earlier: nine rows
// (its own six and three below) of twelve columns (its own eight and four beyond, column c at
// bit 11 - c), each `[drawn, blocks]`.
fn large(m) {
    let at = offsets();
    let rows = [];
    for row in 0..9 {
        let d = 0;
        let b = 0;
        for k in 0..4 {
            let r = row - at[k][1];
            if r >= 0 && r < 6 {
                let shift = 4 - at[k][0];
                let drawn = m[k][r][0] << shift;
                d = d | drawn;
                b = cover(b, drawn, m[k][r][1] << shift);
            }
        }
        rows.push([d, b]);
    }
    rows
}

// The doors, posts and passages a large tile's medium tiles of types `kinds` put in it:
// `#{ kind, x, y }`, the type's top four bits and its marker's cell. A type is 0 for a plain
// tile.
fn marks(kinds) {
    let at = offsets();
    let found = [];
    for k in 0..4 {
        let kind = kinds[k];
        let what = kind >> 4;
        if kind != 0 && (what == door_entry() || what == post() || what == passage()) {
            found.push(#{ kind: what, x: at[k][0] + (kind & 3), y: at[k][1] + ((kind >> 2) & 3) });
        }
    }
    found
}

// Whether the cell at column `x` blocks in a row.
fn blocks_at(row, x) {
    x < 0 || x >= across() || ((row >> (31 - x)) & 1) != 0
}

// A room's cells for `grid`, row by row, from its rows: `bytes` holds each byte's eight cells.
fn cells(rows, bytes) {
    let all = [];
    for row in rows {
        for k in 0..4 {
            all += bytes[(row >> (24 - 8 * k)) & 0xFF];
        }
    }
    all
}

// A row's cells from `x` for `width`, as bits of a row.
fn span(x, width) {
    let bits = (1 << width) - 1;
    let shift = 32 - x - width;
    if shift >= 0 {
        (bits << shift) & 0xFFFFFFFF
    } else {
        bits >> -shift
    }
}

// The group a part is in, once doors join parts.
fn group(joined, part) {
    while joined[part] != part {
        part = joined[part];
    }
    part
}

// Which way an entry or a passage leads: east when the rock is on its right, west otherwise.
fn way(rows, mark) {
    if blocks_at(rows[mark.y], mark.x + 2) && blocks_at(rows[mark.y + 1], mark.x + 2) {
        1
    } else {
        -1
    }
}

// What's inside a room, as `#{ walls, doors }` of cells `[x, y]`: the blocking cells on the
// seams between the parts that lead out (`()` when there are none). A part leads out by a way
// in `ways`, or through a door or a locked door's post to a part that does. A seam's cells in a
// door's two entry tiles or on a post are the door's (docs/re/starquake.md).
fn inside(g, shut, rows, ways, room_marks, bytes) {
    if shut.count < 2 {
        return ();
    }
    let doors = room_marks.filter(|m| m.kind == door_entry());
    let posts = room_marks.filter(|m| m.kind == post());
    let door_rows = [];
    door_rows.pad(down(), 0);
    // Doors open: a post's cells free, and each door's entry joined to where it puts BLOB.
    let open = shut;
    if posts.len() > 0 {
        let cleared = rows;
        for p in posts {
            let col = span((p.x & 0xFC) + 1, 1);
            for y in p.y..min(p.y + 3, down()) {
                cleared[y] = cleared[y] & (-1 ^ col);
                door_rows[y] = door_rows[y] | col;
            }
        }
        open = grid(across(), down(), cells(cleared, bytes)).parts(2, 2);
    }
    let joined = [];
    for part in 0..open.count {
        joined.push(part);
    }
    for d in doors {
        let other = d.x + 6 * way(rows, d);
        let a = open.at(d.x, d.y);
        let b = open.at(other, d.y);
        if a != () && b != () {
            joined[group(joined, a)] = group(joined, b);
        }
        let tiles = span(min(d.x, other), 8);
        for y in max(d.y - 1, 0)..min(d.y + 2, down()) {
            door_rows[y] = door_rows[y] | tiles;
        }
    }
    let groups = [];
    groups.pad(shut.count, ());
    if doors.len() + posts.len() == 0 {
        for part in 0..shut.count {
            groups[part] = part;
        }
    } else {
        for y in 0..shut.height {
            for x in 0..shut.width {
                let part = shut.at(x, y);
                if part != () && groups[part] == () {
                    groups[part] = group(joined, open.at(x, y));
                }
            }
        }
    }
    let leading = [];
    for part in 0..shut.count {
        if ways[part] != 0 {
            leading.push(groups[part]);
        }
    }
    let sources = [];
    for part in 0..shut.count {
        if ways[part] != 0 || leading.contains(groups[part]) {
            sources.push(part);
        }
    }
    if sources.len() < 2 {
        return ();
    }
    let seam = g.seams(shut, sources);
    let walls = [];
    let door_cells = [];
    for i in 0..seam.len() {
        if seam[i] {
            let x = i % across();
            let y = i / across();
            if blocks_at(door_rows[y], x) {
                door_cells.push([x, y]);
            } else {
                walls.push([x, y]);
            }
        }
    }
    #{ walls: walls, doors: door_cells }
}

// Once, after loading: every room decoded from the room data the way the game draws it
// (docs/re/starquake.md, "The rooms"), then where BLOB fits in it, the parts those places make,
// the ways each part leads out, and what's inside each room: its walls and its door.
fn on_load(mem) {
    let rooms = mem.value("rooms");
    let bigs = mem.value("large_tiles");
    let types = mem.value("tile_types");
    let pictures = mem.value("tile_pictures");
    if rooms == () || bigs == () || types == () || pictures == () {
        return #{ cells: [], doors: [], blocks: [] };
    }
    // Only the medium tiles the large tiles use.
    let mediums = [];
    mediums.pad(256, ());
    for id in bigs {
        if mediums[id] == () {
            mediums[id] = medium(mem, pictures[id * 2] + pictures[id * 2 + 1] * 256);
        }
    }
    // Each large tile's nine rows, `[drawn, blocks]` in turn, 18 numbers a tile in one list.
    let larges = [];
    let tile_marks = [];
    for big in 0..256 {
        let m = [];
        let kinds = [];
        for id in bigs.extract(big * 4, 4) {
            m.push(mediums[id]);
            kinds.push(types[id]);
        }
        for row in large(m) {
            larges += row;
        }
        tile_marks.push(marks(kinds));
    }
    let marked = [];
    for found in tile_marks {
        marked.push(found.len() > 0);
    }
    let bytes = [];
    for byte in 0..256 {
        let eight = [];
        for col in 0..8 {
            eight.push(((byte >> (7 - col)) & 1) != 0);
        }
        bytes.push(eight);
    }

    // Each room's blocking cells, a row a number, column c at bit 31 - c: its 4 x 3 large tiles
    // of 8 x 6 cells, drawn left to right, then down, later cells over earlier ones. Then its
    // places and parts, and its grid where there are two parts or more; the parts of the
    // places along its edges, top, bottom, left and right; and the marks of its doors, posts
    // and passages. Done here rather than in functions, which would be handed copies of these
    // lists.
    let blocks = [];
    let grids = [];
    let parts = [];
    let edges = [];
    let room_marks = [];
    let last_x = across() - 2;
    let last_y = down() - 2;
    for room in 0..512 {
        let tiles = rooms.extract(room * 12, 12);
        let rows = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        for i in 0..12 {
            let at = tiles[i] * 18;
            let top = (i / 4) * 6;
            let shift = 20 - (i % 4) * 8;
            for r in 0..min(9, 18 - top) {
                let d = larges[at + r * 2];
                if d != 0 {
                    let b = larges[at + r * 2 + 1];
                    if shift >= 0 {
                        d = d << shift;
                        b = b << shift;
                    } else {
                        d = d >> -shift;
                        b = b >> -shift;
                    }
                    rows[top + r] = ((rows[top + r] & (-1 ^ d)) | b) & 0xFFFFFFFF;
                }
            }
        }
        let all = [];
        for row in rows {
            for k in 0..4 {
                all += bytes[(row >> (24 - 8 * k)) & 0xFF];
            }
        }
        let g = grid(across(), down(), all);
        let p = g.parts(2, 2);
        let top = [];
        let bottom = [];
        for x in 0..=last_x {
            top.push(p.at(x, 0));
            bottom.push(p.at(x, last_y));
        }
        let left = [];
        let right = [];
        for y in 0..=last_y {
            left.push(p.at(0, y));
            right.push(p.at(last_x, y));
        }
        edges.push([top, bottom, left, right]);
        blocks.push(rows);
        parts.push(p);
        grids.push(if p.count >= 2 { g } else { () });
        let here = [];
        for i in 0..12 {
            if !marked[tiles[i]] {
                continue;
            }
            for mark in tile_marks[tiles[i]] {
                mark.x += (i % 4) * 8;
                mark.y += (i / 4) * 6;
                if mark.x < across() - 1 && mark.y < down() - 1 {
                    here.push(mark);
                }
            }
        }
        room_marks.push(here);
    }

    // The ways out of each part: an edge where BLOB fits against it and against the facing
    // edge of the room beside, at the same place; and a wall passage, toward its rock, into a
    // room beside that has one too. The core's room is shut in rock all round, and entering it
    // is the game's own doing: an edge into it is open where BLOB fits against the room beside,
    // and `core` holds the ways that leaves it open.
    let ways = [];
    for room in 0..512 {
        let none = [];
        none.pad(parts[room].count, 0);
        ways.push(none);
    }
    let core = 0;
    let core_at = core_room();
    for room in 16..512 {
        let above = room - 16;
        let tops = edges[room][0];
        let bottoms = edges[above][1];
        for x in 0..=last_x {
            let a = tops[x];
            let b = bottoms[x];
            if a != () && (b != () || above == core_at) {
                ways[room][a] = ways[room][a] | north();
                core = core | if above == core_at { south() } else { 0 };
            }
            if b != () && (a != () || room == core_at) {
                ways[above][b] = ways[above][b] | south();
                core = core | if room == core_at { north() } else { 0 };
            }
        }
    }
    for room in 0..512 {
        if room % 16 == 0 {
            continue;
        }
        let left = room - 1;
        let lefts = edges[room][2];
        let rights = edges[left][3];
        for y in 0..=last_y {
            let a = lefts[y];
            let b = rights[y];
            if a != () && (b != () || left == core_at) {
                ways[room][a] = ways[room][a] | west();
                core = core | if left == core_at { east() } else { 0 };
            }
            if b != () && (a != () || room == core_at) {
                ways[left][b] = ways[left][b] | east();
                core = core | if room == core_at { west() } else { 0 };
            }
        }
    }
    for room in 0..512 {
        for mark in room_marks[room] {
            if mark.kind == passage() {
                let part = parts[room].at(mark.x, mark.y);
                let side = way(blocks[room], mark);
                let beside = room + side;
                if part != () && beside / 16 == room / 16 && beside >= 0 && beside < 512 {
                    if room_marks[beside].some(|m| m.kind == passage()) {
                        let bit = if side > 0 { east() } else { west() };
                        ways[room][part] = ways[room][part] | bit;
                    }
                }
            }
        }
    }

    let cells = [];
    let doors = [];
    for room in 0..512 {
        let out = if room == core_at { core } else { 0 };
        for w in ways[room] {
            out = out | w;
        }
        let shut = "";
        if (out & north()) == 0 {
            shut += "N";
        }
        if (out & east()) == 0 {
            shut += "E";
        }
        if (out & south()) == 0 {
            shut += "S";
        }
        if (out & west()) == 0 {
            shut += "W";
        }
        // Not entered, until the map is drawn for a game.
        let cell = #{ x: room % 16, y: room / 16, walls: shut, seen: true, dim: true };
        if grids[room] != () {
            let inner = inside(
                grids[room],
                parts[room],
                blocks[room],
                ways[room],
                room_marks[room],
                bytes,
            );
            if inner != () {
                cell.inside = inner;
            }
        }
        cells.push(cell);
        let entries = room_marks[room].filter(|m| m.kind == door_entry());
        if entries.len() > 0 {
            // The door's number sits on its right-hand entry, beside its wall.
            let d = entries[0];
            let lo = min(d.x, d.x + 6 * way(blocks[room], d));
            doors.push(#{ room: room, at: [lo + 7, d.y] });
        }
    }
    // The whole planet's map cells, as level 6 shows them, made once; and every room's blocking
    // rows in one list, two rows a number, the first in its top 32 bits: 9 a room.
    let packed = [];
    for rows in blocks {
        for k in 0..9 {
            packed.push((rows[2 * k] << 32) | rows[2 * k + 1]);
        }
    }
    #{ cells: cells, doors: doors, blocks: packed }
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

// The doors listed, numbered in order: each door's room and number.
fn numbered(listed) {
    let doors = [];
    let number = 1;
    for door in listed {
        doors.push(#{ room: door.room, label: number, cards: door.cards });
        number += 1;
    }
    doors
}

// The doors for the panel, by number, each card answered when BLOB carries it.
fn doors(listed, carrying) {
    let rows = [];
    for door in listed {
        let cards = [];
        for card in door.cards {
            cards.push(#{ picture: card, answered: carrying.contains(card) });
        }
        rows.push(#{ label: door.label, cards: cards });
    }
    rows
}

// The room on screen's blocking cells as decoded from the room data, a row a number, column c
// at bit 31 - c: to check against the cells the game draws.
fn decoded(mem, loaded) {
    let room = mem.value("room");
    if room == () || room < 0 || (room + 1) * 9 > loaded.blocks.len() {
        return [];
    }
    let rows = [];
    for pair in loaded.blocks.extract(room * 9, 9) {
        rows.push((pair >> 32) & 0xFFFFFFFF);
        rows.push(pair & 0xFFFFFFFF);
    }
    rows
}

// The rooms walked this game, by room number as text, each with its room number: those whose
// bit is clear in the game's own set of rooms not yet visited, a bit a room, the first room the
// top bit of the first byte (docs/re/starquake.md).
fn rooms_walked(mem, loaded) {
    let unvisited = mem.value("visited");
    let rooms = #{};
    if unvisited == () || loaded.cells.len() != 512 {
        return rooms;
    }
    for byte in 0..64 {
        let bits = unvisited[byte];
        if bits != 0xFF {
            for bit in 0..8 {
                if (bits & (0x80 >> bit)) == 0 {
                    let room = byte * 8 + bit;
                    rooms[`${room}`] = room;
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
            mark.color = "#A066FF";
        } else if picture == 16 {
            mark.color = 6;
        } else {
            mark.color = 7;
        }
        lying.push(mark);
    }
    lying
}

// The map: the rooms walked with their walls and what's inside them, or at level 6 every room,
// those not entered dimmer; the teleports in them, and the doors listed, each with its number;
// from level 3 the items in rooms walked, from level 4 in every room, a room not walked that
// holds a needed piece outlined; BLOB's room.
fn map(mem, loaded, rooms, level, carrying, listed) {
    let everything = level >= 6 && loaded.cells.len() == 512;
    let cells = [];
    if everything {
        cells = loaded.cells;
        for room in rooms.values() {
            cells[room].dim = false;
        }
    } else {
        for room in rooms.values() {
            let cell = loaded.cells[room];
            cell.dim = false;
            cells.push(cell);
        }
    }
    let markers = [];
    for teleport in mem.table("teleports") {
        if everything || `${teleport.room}` in rooms {
            markers.push(#{ kind: "teleport", x: teleport.room % 16, y: teleport.room / 16 });
        }
    }
    for door in listed {
        let mark = #{ kind: "door", x: door.room % 16, y: door.room / 16, label: door.label };
        for found in loaded.doors {
            if found.room == door.room {
                mark.at = found.at;
            }
        }
        markers.push(mark);
    }
    if level >= 3 {
        let outlined = [];
        for item in items(mem, carrying) {
            let key = `${item.room}`;
            if level >= 4 || key in rooms {
                let away = !everything && !(key in rooms);
                if item.kind == "piece" && away && !outlined.contains(item.room) {
                    outlined.push(item.room);
                    cells.push(#{ x: item.x, y: item.y, seen: false, dashed: true });
                }
                item.remove("room");
                markers.push(item);
            }
        }
    }
    let here = mem.value("room");
    markers.push(#{ kind: "player", x: here % 16, y: here / 16 });
    #{ width: 16, height: 32, inside: [32, 18], cells: cells, markers: markers, routes: [] }
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
    let listed = numbered(if level >= 6 && "doors" in loaded {
        every_door(mem, loaded, met)
    } else {
        met
    });
    #{
        core: core(mem, carrying),
        teleports: teleports(mem, kept, level),
        doors: doors(listed, carrying),
        map: map(mem, loaded, rooms, level, carrying, listed),
        kept: keep,
        room: mem.value("room"),
        blocks: decoded(mem, loaded),
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces.
fn next_route(mem, loaded, prev, kept) {
    prev
}
