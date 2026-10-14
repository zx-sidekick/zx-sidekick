// Starquake's script: the map, routes, and the panel's core, teleports and doors.
// See docs/games/starquake.md and the Scripts section of docs/definition-format.md.

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

fn booth() {
    13
}

fn passage() {
    15
}

// A row's blocking cells after a tile draws over it: the cells it draws take its say.
fn cover(row, drawn, blocks) {
    (row & (-1 ^ drawn)) | blocks
}

// A vacuum tube's cells' attribute (docs/re/starquake.md).
fn tube() {
    0x64
}

// A medium tile's picture as its six rows of eight cells, each `[drawn, blocks, tubes]`: the
// cells it draws, those that block (an attribute without BRIGHT), and those of a vacuum tube,
// column c at bit 7 - c. The picture's first six bytes say which cells it draws, row by row,
// the leftmost highest; their attributes are stored before it, the first drawn cell's last.
fn medium(mem, at) {
    let attr = at - 1;
    let rows = [];
    for row in 0..6 {
        let drawn = mem.peek(at + row);
        let blocks = 0;
        let tubes = 0;
        if drawn != 0 {
            for col in 0..8 {
                let bit = 0x80 >> col;
                if (drawn & bit) != 0 {
                    let a = mem.peek(attr);
                    if (a & 0x40) == 0 {
                        blocks = blocks | bit;
                    } else if a == tube() {
                        tubes = tubes | bit;
                    }
                    attr -= 1;
                }
            }
        }
        rows.push([drawn, blocks, tubes]);
    }
    rows
}

// Where a large tile's four medium tiles are drawn within it, in turn.
fn offsets() {
    [[4, 3], [0, 3], [4, 0], [0, 0]]
}

// A large tile as its medium tiles `m` draw it, in turn, later cells over earlier: nine rows
// (its own six and three below) of twelve columns (its own eight and four beyond, column c at
// bit 11 - c), each `[drawn, blocks, tubes]`.
fn large(m) {
    let at = offsets();
    let rows = [];
    for row in 0..9 {
        let d = 0;
        let b = 0;
        let t = 0;
        for k in 0..4 {
            let r = row - at[k][1];
            if r >= 0 && r < 6 {
                let shift = 4 - at[k][0];
                let drawn = m[k][r][0] << shift;
                d = d | drawn;
                b = cover(b, drawn, m[k][r][1] << shift);
                t = cover(t, drawn, m[k][r][2] << shift);
            }
        }
        rows.push([d, b, t]);
    }
    rows
}

// The doors, posts, passages and booths a large tile's medium tiles of types `kinds` put in it:
// `#{ kind, x, y }`, the type's top four bits and its marker's cell. A type is 0 for a plain
// tile.
fn marks(kinds) {
    let at = offsets();
    let found = [];
    for k in 0..4 {
        let kind = kinds[k];
        let what = kind >> 4;
        let wanted = what == door_entry() || what == post() || what == passage() || what == booth();
        if kind != 0 && wanted {
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

// A room's parts with its doors and posts open, as `#{ groups, door_rows }`: the group each
// part of `shut` is in once doors and posts join parts, and the cells of its doors' entry tiles
// and posts, a row a number (docs/re/starquake.md).
fn opened(shut, rows, room_marks, bytes) {
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
    #{ groups: groups, door_rows: door_rows }
}

// What's inside a room, as `#{ walls, doors }` of cells `[x, y]`: the blocking cells on the
// seams between the parts that lead out (`()` when there are none). A part leads out by a way
// in `ways`, or through a door or a locked door's post to a part that does, as `open` groups
// them. A seam's cells in a door's two entry tiles or on a post are the door's.
fn inside(g, shut, ways, open) {
    let groups = open.groups;
    let door_rows = open.door_rows;
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

// The place BLOB fits nearest a cell `[x, y]` of a room's places `p`, as the part it's in: BLOB's
// top left on the cell, or a cell left of it or above it. `()` when he fits at none.
fn part_near(p, x, y) {
    for dy in [0, -1, 1] {
        for dx in [0, -1, 1] {
            let part = p.at(x + dx, y + dy);
            if part != () {
                return part;
            }
        }
    }
    ()
}

// Once, after loading: every room decoded from the room data the way the game draws it
// (docs/re/starquake.md, "The rooms"), then where BLOB fits in it, the parts those places make,
// the ways each part leads out, and what's inside each room: its walls and its door. Then the
// graph routes are found on: its places, each a part of a room, and the steps between them.
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
    // Each large tile's nine rows, `[drawn, blocks]` in turn, 18 numbers a tile in one list; and
    // the vacuum tubes in its own top and bottom rows, two numbers a tile.
    let larges = [];
    let tile_tubes = [];
    let tile_marks = [];
    for big in 0..256 {
        let m = [];
        let kinds = [];
        for id in bigs.extract(big * 4, 4) {
            m.push(mediums[id]);
            kinds.push(types[id]);
        }
        let drawn = large(m);
        for row in drawn {
            larges.push(row[0]);
            larges.push(row[1]);
        }
        tile_tubes.push(drawn[0][2]);
        tile_tubes.push(drawn[5][2]);
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
    // places along its edges, top, bottom, left and right; the vacuum tubes in its top and
    // bottom rows; and the marks of its doors, posts, passages and booths. Done here rather
    // than in functions, which would be handed copies of these lists.
    let blocks = [];
    let grids = [];
    let parts = [];
    let edges = [];
    let tubes = [];
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
        // The top row is the first row of the top tiles; the bottom row the last of the bottom
        // ones, which no tile above reaches.
        let edge_tubes = [0, 0];
        for k in 0..2 {
            let r = k * 5;
            for i in (k * 8)..(k * 8 + 4) {
                let shift = 20 - (i % 4) * 8;
                let d = larges[tiles[i] * 18 + r * 2];
                let t = tile_tubes[tiles[i] * 2 + k];
                if shift >= 0 {
                    d = d << shift;
                    t = t << shift;
                } else {
                    d = d >> -shift;
                    t = t >> -shift;
                }
                edge_tubes[k] = ((edge_tubes[k] & (-1 ^ d)) | t) & 0xFFFFFFFF;
            }
        }
        tubes.push(edge_tubes);
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

    // The graph's places: each room's parts, numbered room by room from `base[room]`. A room's
    // parts joined by its doors and posts are one place: each part's `ring` is the next part
    // of its place, round to itself. `steps[i]` are the parts one step from part i.
    let base = [];
    let room_of = [];
    let count = 0;
    for room in 0..512 {
        base.push(count);
        for k in 0..parts[room].count {
            room_of.push(room);
        }
        count += parts[room].count;
    }
    base.push(count);
    let ring = [];
    for i in 0..count {
        ring.push(i);
    }
    let steps = [];
    steps.pad(count, []);
    let opens = [];
    opens.pad(512, ());
    for room in 0..512 {
        if parts[room].count < 2 {
            continue;
        }
        let shut = room_marks[room].filter(|m| m.kind == door_entry() || m.kind == post());
        if shut.len() == 0 {
            continue;
        }
        let open = opened(parts[room], blocks[room], room_marks[room], bytes);
        opens[room] = open;
        let groups = open.groups;
        for a in 0..groups.len() {
            for b in (a + 1)..groups.len() {
                if groups[a] == groups[b] && ring[base[room] + b] == base[room] + b {
                    // b joins a's ring, just after a.
                    ring[base[room] + b] = ring[base[room] + a];
                    ring[base[room] + a] = base[room] + b;
                }
            }
        }
    }

    // The ways out of each part: an edge where BLOB fits against it and against the facing
    // edge of the room beside, at the same place; and a wall passage, toward its rock, into a
    // room beside that has one too. The core's room is shut in rock all round, and entering it
    // is the game's own doing: an edge into it is open where BLOB fits against the room beside,
    // and `core` holds the ways that leaves it open. On the graph, each is a step both ways,
    // but down where a vacuum tube stands at the edge, which carries BLOB up only; and the core's
    // room is a step from the room on its left only, one way.
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
        let tubed = tubes[above][1] | tubes[room][0];
        let near_core = room == core_at || above == core_at;
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
            if a != () && b != () && !near_core {
                let up = base[room] + a;
                let to = base[above] + b;
                if !steps[up].contains(to) {
                    steps[up].push(to);
                }
                if (tubed & span(x, 2)) == 0 && !steps[to].contains(up) {
                    steps[to].push(up);
                }
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
        let near_core = room == core_at || left == core_at;
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
            if b != () && room == core_at && parts[core_at].count > 0 {
                let from = base[left] + b;
                if !steps[from].contains(base[core_at]) {
                    steps[from].push(base[core_at]);
                }
            }
            if a != () && b != () && !near_core {
                let i = base[room] + a;
                let j = base[left] + b;
                if !steps[i].contains(j) {
                    steps[i].push(j);
                    steps[j].push(i);
                }
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
                    let far = room_marks[beside].filter(|m| m.kind == passage());
                    if far.len() > 0 {
                        let bit = if side > 0 { east() } else { west() };
                        ways[room][part] = ways[room][part] | bit;
                        let there = parts[beside].at(far[0].x, far[0].y);
                        let i = base[room] + part;
                        if there != () && !steps[i].contains(base[beside] + there) {
                            steps[i].push(base[beside] + there);
                        }
                    }
                }
            }
        }
    }
    // The teleport booths: the part of each room with one where BLOB stands in it.
    let booths = [];
    for room in 0..512 {
        for mark in room_marks[room] {
            if mark.kind == booth() {
                let part = part_near(parts[room], mark.x, mark.y);
                if part != () {
                    booths.push([room, base[room] + part]);
                    break;
                }
            }
        }
    }

    let cells = [];
    let doors = [];
    let posts = [];
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
            let open = opens[room];
            if open == () {
                let groups = [];
                for part in 0..parts[room].count {
                    groups.push(part);
                }
                let none = [];
                none.pad(down(), 0);
                open = #{ groups: groups, door_rows: none };
            }
            let inner = inside(grids[room], parts[room], ways[room], open);
            if inner != () {
                cell.inside = inner;
            }
        }
        cells.push(cell);
        let entries = room_marks[room].filter(|m| m.kind == door_entry());
        if entries.len() > 0 {
            // The door's number sits in the middle of its two entries' tiles, over the door.
            let d = entries[0];
            let lo = min(d.x, d.x + 6 * way(blocks[room], d));
            doors.push(#{ room: room, at: [lo + 3, d.y] });
        }
        if room_marks[room].some(|m| m.kind == post()) {
            posts.push(room);
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
    #{
        cells: cells,
        doors: doors,
        blocks: packed,
        bytes: bytes,
        graph: #{
            base: base,
            room_of: room_of,
            ring: ring,
            steps: steps,
            booths: booths,
            posts: posts,
        },
    }
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

// Which of a door's cards what BLOB carries answers, as the door's screen decides it
// (docs/re/starquake.md): a 15 answers them all; otherwise each card in turn is answered by a
// carried card of its own picture, or failing that by a 14, each carried card answering one.
fn answered(cards, carrying) {
    if carrying.contains(15) {
        return cards.map(|card| true);
    }
    let left = carrying;
    let marks = [];
    for card in cards {
        let k = left.index_of(card);
        if k < 0 {
            k = left.index_of(14);
        }
        if k >= 0 {
            left.remove(k);
        }
        marks.push(k >= 0);
    }
    marks
}

// The doors for the panel, by number, each card answered as the door would answer it with what
// BLOB carries; a door a route passes first outlined in its colour, as `outlined` has it by room.
fn doors(listed, carrying, outlined) {
    let rows = [];
    for door in listed {
        let marks = answered(door.cards, carrying);
        let cards = [];
        for k in 0..door.cards.len() {
            cards.push(#{ picture: door.cards[k], answered: marks[k] });
        }
        let row = #{ label: door.label, cards: cards };
        let room = `${door.room}`;
        if room in outlined {
            row.outlined = outlined[room];
        }
        rows.push(row);
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

// Whether BLOB has been in the core's room this game, where the game shows what the core wants:
// its bit is clear in the game's own set of rooms not yet visited (docs/re/starquake.md).
fn core_seen(mem) {
    let unvisited = mem.value("visited");
    if unvisited == () {
        return false;
    }
    let room = core_room();
    (unvisited[room / 8] & (0x80 >> (room % 8))) == 0
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
// carries; a card, 9 to 13 for doors, 14 answering any one card and 15 every card; the key, 16;
// anything else is a core piece a pyramid takes. Packs (17 to 24) are a room's bonus, never in
// the item table, and act as they're touched: nothing to fetch.
// Not those off the floor (rows 1 to 5: the game lays an item down at row 6 or more, and a
// carried one has a row below that), nor those kept in the core's room. Which pieces are needed
// is only `known` once the core has been seen, below level 4.
fn items(mem, carrying, known) {
    let pieces = if known { needed(mem) } else { [] };
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
        if row >= 6 {
            // Laid down: its column, and its screen row less the six rows above the room.
            mark.place = [item.first & 0x1F, row - 6];
        }
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

// The places of a room's parts, from its blocking rows as `on_load` decoded them: numbered as
// `on_load` numbered them. Called on `loaded`, which isn't copied then.
fn room_parts(room) {
    let rows = [];
    for pair in this.blocks.extract(room * 9, 9) {
        rows.push((pair >> 32) & 0xFFFFFFFF);
        rows.push(pair & 0xFFFFFFFF);
    }
    grid(across(), down(), cells(rows, this.bytes)).parts(2, 2)
}

// The way from one room to the next, beside it.
fn heading(from, to) {
    let step = to - from;
    if step == -16 {
        "N"
    } else if step == 16 {
        "S"
    } else if step == 1 {
        "E"
    } else if step == -1 {
        "W"
    } else {
        ()
    }
}

// A route as the map draws it, from the graph's places in `nodes`, in order, each reached as
// `how` says (0 a step, 1 within a place, 2 a teleport): `#{ steps, exit, door }`. A step is
// its room's cell, dashed into or out of a room not walked, a jump to a booth drawn as none.
// `exit` is the edge it leaves BLOB's room by; `door` the room of the first security door it
// passes: where the part it enters a room by isn't the part it leaves by (docs/games/starquake.md).
fn route_of(nodes, how, room_of, rooms, door_rooms, post_rooms) {
    let steps = [];
    let exit = ();
    let door = ();
    let post = ();
    let first = nodes[0];
    let start = room_of[first];
    let left = false;
    steps.push([start % 16, start / 16]);
    for k in 1..=nodes.len() {
        let last = room_of[nodes[k - 1]];
        let room = if k < nodes.len() { room_of[nodes[k]] } else { -1 };
        if room == last {
            continue;
        }
        if door == () && nodes[k - 1] != first && door_rooms.contains(last) {
            door = last;
        }
        if post == () && nodes[k - 1] != first && post_rooms.contains(last) {
            post = last;
        }
        if room < 0 {
            break;
        }
        first = nodes[k];
        let step = [room % 16, room / 16];
        if how[nodes[k]] == 2 {
            step.push("jump");
        } else if !(`${room}` in rooms) || !(`${last}` in rooms) {
            step.push("dashed");
        }
        steps.push(step);
        if !left {
            left = true;
            if how[nodes[k]] != 2 {
                exit = heading(last, room);
            }
        }
    }
    #{ steps: steps, exit: exit, door: door, post: post }
}

// The graph's places from the search's `start` to `end`, through each place's `parent`.
fn path(parent, start, end) {
    let nodes = [end];
    let at = end;
    while at != start {
        at = parent[at];
        nodes.push(at);
    }
    nodes.reverse();
    nodes
}

// Level 5's routes, called on `loaded`: breadth-first from BLOB's place over the graph of
// places, fewest steps, to the nearest three rooms with a missing piece and, while BLOB carries
// a piece the core needs, to the core (docs/games/starquake.md). `#{ routes, rings, outlined,
// route }`: the map's routes, the first to the piece `prev.route` picks of the nearest found;
// the ring markers, on the core, on the items that answer the first door each route passes,
// which `outlined` gives each route's colour by its room, and on the key where a route passes a
// locked door's post; and the piece picked.
fn guide(mem, prev, level, rooms, carrying, lying, listed) {
    let none = #{ routes: [], rings: [], outlined: #{}, route: 0 };
    let room = mem.value("room");
    if !("graph" in this) || room == () || room < 0 || room >= 512 {
        return none;
    }
    let base = this.graph.base;
    let count = base[512];
    // BLOB's place: his left edge's column, his top's row; a cell on when he's between them
    // (docs/re/starquake.md).
    let here = this.room_parts(room);
    let x = mem.value("blob_x") >> 3;
    let y = down() - 1 - (mem.value("blob_y") >> 3);
    let part = ();
    for c in [[0, 0], [1, 0], [0, 1], [1, 1]] {
        if part == () {
            part = here.at(x + c[0], y + c[1]);
        }
    }
    if part == () {
        return none;
    }
    let start = base[room] + part;
    // What's sought: each missing piece's place, the part it lies in, or its room's every part
    // until it's laid down; and the core's room.
    let goal = [];
    goal.pad(count, -1);
    for item in lying {
        if item.kind != "piece" {
            continue;
        }
        let r = item.room;
        let first = base[r];
        let n = base[r + 1] - first;
        let at = ();
        if "place" in item && n > 1 {
            at = part_near(this.room_parts(r), item.place[0], item.place[1]);
        }
        if at != () {
            goal[first + at] = r;
        } else {
            for k in 0..n {
                goal[first + k] = r;
            }
        }
    }
    let pieces = needed(mem);
    let core_at = core_room();
    let core_node = -1;
    if carrying.some(|c| pieces.contains(c)) && base[core_at + 1] > base[core_at] {
        core_node = base[core_at];
    }
    let booths = [];
    for b in this.graph.booths {
        if level >= 6 || `${b[0]}` in rooms {
            booths.push(b[1]);
        }
    }
    let ring = this.graph.ring;
    let parent = [];
    parent.pad(count, -1);
    let how = [];
    how.pad(count, 0);
    let queue = [start];
    parent[start] = start;
    let k = ring[start];
    let before = start;
    while k != start {
        parent[k] = before;
        how[k] = 1;
        queue.push(k);
        before = k;
        k = ring[k];
    }
    let head = 0;
    let found = [];
    let found_rooms = [];
    let core_found = false;
    let jumped = false;
    while head < queue.len() {
        let i = queue[head];
        head += 1;
        let r = goal[i];
        if r >= 0 && found.len() < 3 && !found_rooms.contains(r) {
            found_rooms.push(r);
            found.push(i);
        }
        if i == core_node {
            core_found = true;
        }
        if found.len() >= 3 && (core_found || core_node < 0) {
            break;
        }
        let next = [];
        for j in this.graph.steps[i] {
            if parent[j] < 0 {
                next.push([j, 0]);
            }
        }
        if !jumped && booths.contains(i) {
            jumped = true;
            for b in booths {
                if parent[b] < 0 {
                    next.push([b, 2]);
                }
            }
        }
        for step in next {
            let j = step[0];
            if parent[j] >= 0 {
                continue;
            }
            parent[j] = i;
            how[j] = step[1];
            queue.push(j);
            // The rest of its place, joined by a door or a post: reached with it.
            let k = ring[j];
            let before = j;
            while k != j {
                if parent[k] < 0 {
                    parent[k] = before;
                    how[k] = 1;
                    queue.push(k);
                }
                before = k;
                k = ring[k];
            }
        }
    }
    let room_of = this.graph.room_of;
    let door_rooms = this.doors.map(|d| d.room);
    let post_rooms = this.graph.posts;
    let chosen = 0;
    if type_of(prev) == "map" && "route" in prev && type_of(prev.route) == "i64" {
        chosen = prev.route;
    }
    let ways = [];
    if found.len() > 0 {
        chosen = chosen % found.len();
        let way = route_of(
            path(parent, start, found[chosen]),
            how,
            room_of,
            rooms,
            door_rooms,
            post_rooms,
        );
        way.color = "first";
        way.label = `Item ${chosen + 1}/${found.len()}`;
        ways.push(way);
    } else {
        chosen = 0;
    }
    let rings = [];
    if core_found {
        let way = route_of(
            path(parent, start, core_node),
            how,
            room_of,
            rooms,
            door_rooms,
            post_rooms,
        );
        way.color = "second";
        way.label = "Core";
        ways.push(way);
        rings.push(#{ kind: "target", x: core_at % 16, y: core_at / 16, color: "second" });
    }
    // The first door each route passes, outlined in its colour, the first route's where both
    // pass one; and the items that would answer its cards not yet answered ringed, with any
    // card that answers any card. A route through a locked door's post rings the key, 16,
    // unless BLOB carries it.
    let outlined = #{};
    let routes = [];
    for way in ways {
        let door = way.door;
        if door != () {
            if !(`${door}` in outlined) {
                outlined[`${door}`] = way.color;
            }
            let cards = door_cards(mem.value("seed"), door);
            for met in listed {
                if met.room == door {
                    cards = met.cards;
                }
            }
            let marks = answered(cards, carrying);
            let open = [];
            for k in 0..cards.len() {
                if !marks[k] {
                    open.push(cards[k]);
                }
            }
            if open.len() > 0 {
                for item in lying {
                    let p = item.picture;
                    if open.contains(p) || p == 14 || p == 15 {
                        rings.push(#{ kind: "target", x: item.x, y: item.y, color: way.color });
                    }
                }
            }
        }
        if way.post != () && !carrying.contains(16) {
            for item in lying {
                if item.picture == 16 {
                    rings.push(#{ kind: "target", x: item.x, y: item.y, color: way.color });
                }
            }
        }
        let shown = #{ color: way.color, steps: way.steps, label: way.label };
        if way.exit != () {
            shown.exit = way.exit;
        }
        routes.push(shown);
    }
    #{ routes: routes, rings: rings, outlined: outlined, route: chosen }
}

// The map: the rooms walked with their walls and what's inside them, or at level 6 every room,
// those not entered dimmer; the teleports in them, and the doors listed, each with its number;
// from level 3 the items `lying` in rooms walked, from level 4 in every room, a room not walked
// that holds a needed piece outlined; BLOB's room; and from level 5 the routes `guided`.
fn map(mem, loaded, rooms, level, lying, listed, guided) {
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
    let outlined = [];
    for item in lying {
        let key = `${item.room}`;
        if level >= 4 || key in rooms {
            let away = !everything && !(key in rooms);
            if item.kind == "piece" && away && !outlined.contains(item.room) {
                outlined.push(item.room);
                cells.push(#{ x: item.x, y: item.y, seen: false, dashed: true });
            }
            item.remove("room");
            item.remove("place");
            markers.push(item);
        }
    }
    markers += guided.rings;
    let here = mem.value("room");
    markers.push(#{ kind: "player", x: here % 16, y: here / 16 });
    #{
        width: 16,
        height: 32,
        inside: [32, 18],
        cells: cells,
        markers: markers,
        routes: guided.routes,
    }
}

// At each sync point: the panel's data, and codes to keep.
// The door's cards the cards BLOB carries leave unanswered: each card answers itself, and each
// of pictures 14 and 15 answers any one card (docs/re/starquake.md). Empty when all are.
fn cards_missing(cards, carrying) {
    let wild = carrying.filter(|p| p == 14 || p == 15).len();
    let missing = cards.filter(|card| !carrying.contains(card));
    if missing.len() > wild { missing } else { [] }
}

// Whether an item that would answer one of `missing` lies on the planet, where the map shows
// it: that card, or one that answers any card; not carried, nor kept in the core's room.
fn card_lying(mem, missing) {
    for item in mem.table("items") {
        let picture = item.picture;
        let row = item.second & 0x7F;
        let room = item.room_low + (item.second >> 7) * 256;
        let answers = missing.contains(picture) || picture == 14 || picture == 15;
        if answers && !(row >= 1 && row <= 5) && room != core_room() {
            return true;
        }
    }
    false
}

// A table field read as a flag, whether it comes as one or as a number.
fn set(field) {
    if type_of(field) == "bool" { field } else { field != 0 }
}

// Whether the locked door's post in `room` is still locked, by the game's table of locked
// doors (docs/re/starquake.md).
fn post_locked(mem, room) {
    for door in mem.table("locked_doors") {
        let ninth = if set(door.room_high) { 256 } else { 0 };
        if door.room_low + ninth == room {
            return set(door.locked);
        }
    }
    false
}

// How many rooms walked before the hint that the core shows what it wants only in its room.
fn rooms_before_core_hint() {
    20
}

// The definition's `[[hints]]` script conditions that hold now (docs/games/starquake.md): BLOB
// in a security door's room without its cards, and whether one that would answer lies on the
// map; at a locked door's post without the key; with four items carried; or a good way into the
// planet without having seen the core.
fn hint_conditions(mem, loaded, carrying, rooms) {
    let holding = [];
    let room = mem.value("room");
    if room != () && "doors" in loaded && loaded.doors.some(|d| d.room == room) {
        let missing = cards_missing(door_cards(mem.value("seed"), room), carrying);
        if missing.len() > 0 {
            holding.push("door-cards-missing");
            if card_lying(mem, missing) {
                holding.push("door-cards-on-map");
            }
        }
    }
    if room != () && "graph" in loaded && loaded.graph.posts.contains(room) {
        if !carrying.contains(16) && post_locked(mem, room) {
            holding.push("post-locked");
        }
    }
    if carrying.len() == 4 {
        holding.push("hands-full");
    }
    if !core_seen(mem) && rooms.len() >= rooms_before_core_hint() {
        holding.push("core-not-seen");
    }
    holding
}

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
    let lying = if level >= 3 { items(mem, carrying, level >= 4 || core_seen(mem)) } else { [] };
    let guided = if level >= 5 {
        loaded.guide(mem, prev, level, rooms, carrying, lying, listed)
    } else {
        #{ routes: [], rings: [], outlined: #{}, route: 0 }
    };
    #{
        core: if level >= 4 || core_seen(mem) { core(mem, carrying) } else { [] },
        teleports: teleports(mem, kept, level),
        doors: doors(listed, carrying, guided.outlined),
        map: map(mem, loaded, rooms, level, lying, listed, guided),
        kept: keep,
        room: mem.value("room"),
        blocks: decoded(mem, loaded),
        route: guided.route,
        hints: hint_conditions(mem, loaded, carrying, rooms),
    }
}

// Tab or the north button: the route to the next of the nearest missing pieces, from the next
// frame.
fn next_route(mem, loaded, prev, kept) {
    let at = 0;
    if type_of(prev) == "map" && "route" in prev && type_of(prev.route) == "i64" {
        at = prev.route;
    }
    #{ route: at + 1 }
}
