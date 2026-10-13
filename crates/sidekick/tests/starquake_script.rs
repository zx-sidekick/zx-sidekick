//! Starquake's script, on stand-ins: its definition and script from `games/starquake`, a 48K
//! with no tape, and stand-in values written where the definition says. Runs everywhere.

use sidekick_core::{Fact, Machine, Model, Script, Session};
use sidekick_def::{Definition, Package};
use std::path::{Path, PathBuf};

/// Frames for the 48K ROM to start up and settle into its editor.
const BOOT_FRAMES: usize = 100;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A session of Starquake's definition and script on a bare machine.
fn session() -> Session {
    let package = Package::load(&repo().join("games/starquake")).expect("its definition");
    let source = std::fs::read_to_string(repo().join("games/starquake/scripts/starquake.rhai"))
        .expect("its script");
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for _ in 0..BOOT_FRAMES {
        machine.run_frame().expect("a frame");
    }
    let mut s = Session::new(machine, Some(package.definition));
    s.set_script(Script::compile(&source).map_err(|e| e.to_string()));
    s
}

const fn definition(s: &Session) -> &Definition {
    s.definition().expect("a definition")
}

/// Where a value starts, from the definition.
fn addr(s: &Session, name: &str) -> u16 {
    definition(s).values[name].addr.expect("an address").0
}

/// The address a state is decided by, from the definition.
fn state_pc(s: &Session, name: &str) -> u16 {
    let state = definition(s)
        .states
        .iter()
        .find(|st| st.name == name)
        .expect("the state");
    state
        .pc
        .as_ref()
        .expect("a pc")
        .iter()
        .next()
        .expect("an address")
        .0
}

fn poke(s: &mut Session, at: u16, bytes: &[u8]) {
    for (a, &b) in (at..).zip(bytes) {
        s.machine_mut().write(a, b);
    }
}

/// Holds the processor in a loop at the state's address, so every frame passes it.
fn enter(s: &mut Session, state: &str) {
    let pc = state_pc(s, state);
    poke(s, pc, &[0x18, 0xFE]);
    s.machine_mut().registers_mut().set_pc(pc);
    for _ in 0..2 {
        s.run_frame().expect("a frame");
    }
    assert_eq!(s.state(), Some(state));
}

fn result(s: &Session, name: &str) -> Vec<Fact> {
    match s.script_results().get(name) {
        Some(Fact::List(items)) => items.clone(),
        other => panic!("{name}: {other:?} ({:?})", s.script_off()),
    }
}

fn kept(s: &Session, list: &str) -> Vec<String> {
    s.kept().get(list).cloned().unwrap_or_default()
}

#[test]
fn a_code_is_kept_only_while_a_booth_shows_it() {
    let mut s = session();
    let shown = addr(&s, "booth_code");
    poke(&mut s, shown, b"QWERT");
    enter(&mut s, "play");
    assert!(kept(&s, "teleport-codes").is_empty(), "not in play");
    enter(&mut s, "booth");
    assert_eq!(kept(&s, "teleport-codes"), ["QWERT"]);
    poke(&mut s, shown, b"QW?RT");
    enter(&mut s, "play");
    enter(&mut s, "booth");
    assert_eq!(kept(&s, "teleport-codes"), ["QWERT"], "letters only");
    assert_eq!(result(&s, "teleports"), [Fact::Text("QWERT".to_owned())]);
}

#[test]
fn a_door_is_kept_with_its_game_and_listed_for_that_game_only() {
    let mut s = session();
    let (cards, count) = (addr(&s, "door_cards"), addr(&s, "door_card_count"));
    let (seed, room) = (addr(&s, "seed"), addr(&s, "room"));
    poke(&mut s, seed, &[0x34, 0x12]);
    poke(&mut s, room, &[40, 0]);
    poke(&mut s, count, &[3]);
    poke(&mut s, cards, &[3, 0, 3, 0, 3]);
    enter(&mut s, "door");
    assert!(kept(&s, "doors-met").is_empty(), "cards not worked out yet");

    poke(&mut s, cards, &[9, 0, 12, 0, 13]);
    enter(&mut s, "play");
    enter(&mut s, "door");
    assert_eq!(kept(&s, "doors-met"), ["4660:40:9-12-13"]);
    s.run_frame().expect("a frame");
    let doors = result(&s, "doors");
    assert_eq!(doors.len(), 1);

    poke(&mut s, count, &[2]);
    poke(&mut s, room, &[41, 0]);
    enter(&mut s, "play");
    enter(&mut s, "door");
    assert_eq!(
        kept(&s, "doors-met").len(),
        1,
        "two cards: not a security door"
    );

    poke(&mut s, seed, &[0x35, 0x12]);
    s.run_frame().expect("a frame");
    assert!(
        result(&s, "doors").is_empty(),
        "another game's doors aren't this one's"
    );
}

#[test]
fn level_6_lists_every_teleport_s_code() {
    let mut s = session();
    let table = addr(&s, "teleports");
    for (k, letter) in (b'A'..).take(15).enumerate() {
        let at = table + u16::try_from(k).expect("a few") * 7;
        poke(&mut s, at, &[letter; 5]);
        poke(&mut s, at + 5, &[u8::try_from(k).expect("a few"), 0]);
    }
    enter(&mut s, "play");
    assert!(result(&s, "teleports").is_empty(), "level 0: none kept");
    s.set_level(6);
    s.run_frame().expect("a frame");
    let codes = result(&s, "teleports");
    assert_eq!(codes.len(), 15);
    assert_eq!(codes[0], Fact::Text("AAAAA".to_owned()));
    assert_eq!(codes[14], Fact::Text("OOOOO".to_owned()));
}

fn field<'a>(fact: &'a Fact, key: &str) -> &'a Fact {
    match fact {
        Fact::Map(map) => &map[key],
        other => panic!("{key} of {other:?}"),
    }
}

/// The game's set of rooms not yet visited, with `rooms` visited: a bit a room, the first
/// room the top bit of the first byte, set while not visited.
fn visit(s: &mut Session, rooms: &[usize]) {
    let mut bits = [0xFFu8; 64];
    for &room in rooms {
        bits[room / 8] &= !(0x80 >> (room % 8));
    }
    let at = addr(s, "visited");
    poke(s, at, &bits);
}

#[test]
fn rooms_visited_are_mapped_with_their_decoded_walls() {
    let mut s = session();
    blank_planet(&mut s);
    // Room 300's top row is large tile 1, whose medium tile 2 draws a blocking row.
    poke(&mut s, ROW, &[0xFF, 0, 0, 0, 0, 0]);
    poke(&mut s, ROW - 8, &[0x07; 8]);
    let pictures = addr(&s, "tile_pictures");
    poke(&mut s, pictures + 4, &ROW.to_le_bytes());
    let large = addr(&s, "large_tiles");
    poke(&mut s, large + 4, &[0, 0, 2, 2]);
    let rooms = addr(&s, "rooms");
    poke(&mut s, rooms + 300 * 12, &[1, 1, 1, 1]);
    // Before a game, the game's set has every room unvisited.
    enter(&mut s, "play");
    s.set_level(2);
    s.run_frame().expect("a frame");
    let map = s.script_results()["map"].clone();
    assert_eq!(
        field(&map, "cells"),
        &Fact::List(Vec::new()),
        "none walked yet"
    );

    visit(&mut s, &[8, 300]);
    s.run_frame().expect("a frame");
    let map = s.script_results()["map"].clone();
    let Fact::List(cells) = field(&map, "cells").clone() else {
        panic!("cells");
    };
    let mut walked: Vec<(i64, Fact)> = cells
        .iter()
        .map(|c| (number(field(c, "y")), field(c, "walls").clone()))
        .collect();
    walked.sort_by_key(|&(y, _)| y);
    assert_eq!(
        walked,
        [
            (0, Fact::Text("N".to_owned())),
            (18, Fact::Text("N".to_owned())),
        ],
        "rooms 8 and 300, with their walls as decoded: no room above the top row"
    );
}

fn markers(s: &Session) -> Vec<String> {
    let map = s.script_results()["map"].clone();
    let Fact::List(markers) = field(&map, "markers").clone() else {
        panic!("markers");
    };
    markers
        .iter()
        .map(|m| match field(m, "kind") {
            Fact::Text(kind) => kind.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn teleports_and_doors_in_rooms_walked_are_marked() {
    let mut s = session();
    let (seed, room, area) = (addr(&s, "seed"), addr(&s, "room"), addr(&s, "play_area"));
    let (teleports, cards, count) = (
        addr(&s, "teleports"),
        addr(&s, "door_cards"),
        addr(&s, "door_card_count"),
    );
    poke(&mut s, seed, &[0x34, 0x12]);
    poke(&mut s, room, &[8, 0]);
    poke(&mut s, area, &[0x47; 576]);
    visit(&mut s, &[8]);
    // A teleport in room 8 and one in room 300, which isn't walked.
    poke(&mut s, teleports, b"AAAAA");
    poke(&mut s, teleports + 5, &[8, 0]);
    poke(&mut s, teleports + 7, b"BBBBB");
    poke(&mut s, teleports + 12, &[0x2C, 1]);
    // A door met in room 8.
    poke(&mut s, count, &[3]);
    poke(&mut s, cards, &[9, 0, 10, 0, 11]);
    enter(&mut s, "door");
    enter(&mut s, "play");
    for _ in 0..2 {
        s.run_frame().expect("a frame");
    }
    assert_eq!(markers(&s), ["teleport", "door", "player"]);
}

/// Where stand-in medium tile pictures go: one that draws nothing, and one that draws a row.
const BLANK: u16 = 0x8100;
const ROW: u16 = 0x8010;

/// Writes bytes into a value, `offset` bytes from its start.
fn poke_into(s: &mut Session, name: &str, offset: u16, bytes: &[u8]) {
    let at = addr(s, name) + offset;
    poke(s, at, bytes);
}

/// A planet of rooms that draw nothing, so open all round, none visited, and no items.
fn blank_planet(s: &mut Session) {
    visit(s, &[]);
    let pictures = addr(s, "tile_pictures");
    for id in 0..256 {
        poke(s, pictures + id * 2, &BLANK.to_le_bytes());
    }
    let items = addr(s, "items");
    for k in 0..45 {
        poke(s, items + k * 4, &[0, 0, 0, 0xFF]);
    }
}

fn number(fact: &Fact) -> i64 {
    match fact {
        Fact::Number(n) => *n,
        other => panic!("a number: {other:?}"),
    }
}

fn list(fact: &Fact) -> Vec<Fact> {
    match fact {
        Fact::List(items) => items.clone(),
        other => panic!("a list: {other:?}"),
    }
}

#[test]
fn level_6_maps_every_room_with_the_walls_its_tiles_draw() {
    let mut s = session();
    blank_planet(&mut s);
    // Medium tile 2 draws its top row, eight cells without BRIGHT: they block.
    poke(&mut s, ROW, &[0xFF, 0, 0, 0, 0, 0]);
    poke(&mut s, ROW - 8, &[0x07; 8]);
    poke_into(&mut s, "tile_pictures", 4, &ROW.to_le_bytes());
    // Large tile 1: medium tile 2 at the top, left and right; room 300's top row is all large
    // tile 1, so its top edge is shut.
    poke_into(&mut s, "large_tiles", 4, &[0, 0, 2, 2]);
    poke_into(&mut s, "rooms", 300 * 12, &[1, 1, 1, 1]);
    enter(&mut s, "play");
    s.run_frame().expect("a frame");
    let map = s.script_results()["map"].clone();
    assert!(
        list(field(&map, "cells")).len() < 512,
        "level 0: rooms walked only"
    );

    s.set_level(6);
    s.run_frame().expect("a frame");
    let map = s.script_results()["map"].clone();
    let cells = list(field(&map, "cells"));
    assert_eq!(cells.len(), 512);
    assert_eq!(field(&cells[300], "walls"), &Fact::Text("N".to_owned()));
    assert_eq!(
        field(&cells[284], "walls"),
        &Fact::Text("S".to_owned()),
        "the room above it is shut against it too"
    );
    assert_eq!(field(&cells[17], "walls"), &Fact::Text(String::new()));
    assert_eq!(
        field(&cells[0], "walls"),
        &Fact::Text("NW".to_owned()),
        "the planet's edge"
    );
}

#[test]
fn level_6_lists_every_security_door_with_the_cards_the_game_works_out() {
    let mut s = session();
    blank_planet(&mut s);
    // Medium tile 1 is a security door, one cell in and one down; large tile 3 has it at its
    // top left, and room 176 has large tile 3 at its top left.
    poke_into(&mut s, "tile_types", 1, &[0x05]);
    poke_into(&mut s, "large_tiles", 12, &[0, 0, 0, 1]);
    poke_into(&mut s, "rooms", 176 * 12, &[3]);
    // The seed of a game where the door in room 176 was met with cards 12, 13 and 13.
    poke_into(&mut s, "seed", 0, &[0xF5, 0x07]);
    enter(&mut s, "play");
    s.run_frame().expect("a frame");
    assert!(result(&s, "doors").is_empty(), "level 0: doors met only");

    s.set_level(6);
    s.run_frame().expect("a frame");
    let doors = result(&s, "doors");
    assert_eq!(doors.len(), 1);
    let cards: Vec<i64> = list(field(&doors[0], "cards"))
        .iter()
        .map(|card| number(field(card, "picture")))
        .collect();
    assert_eq!(cards, [12, 13, 13]);
    let map = s.script_results()["map"].clone();
    assert!(list(field(&map, "markers")).iter().any(|m| {
        field(m, "kind") == &Fact::Text("door".to_owned())
            && number(field(m, "x")) == 0
            && number(field(m, "y")) == 11
    }));
}

/// The item markers on the map, as (picture, color, room).
fn item_marks(s: &Session) -> Vec<(i64, Fact, i64)> {
    let map = s.script_results()["map"].clone();
    list(field(&map, "markers"))
        .iter()
        .filter(|m| matches!(field(m, "kind"), Fact::Text(k) if k == "piece" || k == "item"))
        .map(|m| {
            let room = number(field(m, "y")) * 16 + number(field(m, "x"));
            (number(field(m, "picture")), field(m, "color").clone(), room)
        })
        .collect()
}

#[test]
fn items_are_marked_by_kind_from_level_3_in_rooms_walked_and_level_4_everywhere() {
    let mut s = session();
    blank_planet(&mut s);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    // The core still needs picture 5; picture 7 is delivered.
    let slots = addr(&s, "core_slots");
    poke(&mut s, slots, &[0x85, 0x07]);
    let items = addr(&s, "items");
    // Placed in room 8: a needed piece and the key.
    poke(&mut s, items, &[3, 20, 8, 5]);
    poke(&mut s, items + 4, &[3, 20, 8, 16]);
    // A card in room 300, not yet placed: room 300's ninth bit is the second byte's top bit.
    poke(&mut s, items + 8, &[3, 0x80, 0x2C, 10]);
    // Kept in the core's own room, and one picked up: neither is on the map.
    poke(&mut s, items + 12, &[0, 10, 199, 0]);
    poke(&mut s, items + 16, &[0, 1, 8, 7]);
    // Room 8 walked, and the core's room, so what the core needs is known.
    visit(&mut s, &[8, 199]);
    enter(&mut s, "play");
    s.set_level(2);
    s.run_frame().expect("a frame");
    assert!(item_marks(&s).is_empty(), "level 2: no items");

    s.set_level(3);
    s.run_frame().expect("a frame");
    assert_eq!(
        item_marks(&s),
        [
            (5, Fact::Text("first".to_owned()), 8),
            (16, Fact::Number(6), 8)
        ],
        "level 3: in rooms walked, the needed piece in the first color, the key yellow"
    );

    s.set_level(4);
    s.run_frame().expect("a frame");
    assert_eq!(
        item_marks(&s).last(),
        Some(&(10, Fact::Text("#A066FF".to_owned()), 300)),
        "level 4: in rooms not reached too, a card violet"
    );
}

#[test]
fn no_item_off_the_floor_is_marked() {
    let mut s = session();
    blank_planet(&mut s);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    // Rows 1 to 5 are off the floor: picked up, or otherwise not lying in the room.
    for (k, row) in (0..).zip(1..=5u8) {
        poke_into(&mut s, "items", k * 4, &[3, row, 8, 20 + row]);
    }
    // Row 6 lies on the floor; row 0 isn't placed yet, but its room is known.
    poke_into(&mut s, "items", 20, &[3, 6, 8, 30]);
    poke_into(&mut s, "items", 24, &[3, 0, 9, 31]);
    enter(&mut s, "play");
    s.set_level(4);
    s.run_frame().expect("a frame");
    let pictures: Vec<i64> = item_marks(&s).iter().map(|&(p, _, _)| p).collect();
    assert_eq!(pictures, [30, 31]);
}

#[test]
fn a_door_met_keeps_its_number_at_level_6() {
    let mut s = session();
    blank_planet(&mut s);
    // Security doors in rooms 30 and 40, as in the level 6 test above.
    poke_into(&mut s, "tile_types", 1, &[0x05]);
    poke_into(&mut s, "large_tiles", 12, &[0, 0, 0, 1]);
    poke_into(&mut s, "rooms", 30 * 12, &[3]);
    poke_into(&mut s, "rooms", 40 * 12, &[3]);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    // The door in room 40 met this game.
    let mut kept = s.kept().clone();
    kept.insert("doors-met".to_owned(), vec!["4660:40:9-12-13".to_owned()]);
    s.set_kept(kept);
    enter(&mut s, "play");
    s.set_level(2);
    s.run_frame().expect("a frame");
    let cards = |door: &Fact| -> Vec<i64> {
        list(field(door, "cards"))
            .iter()
            .map(|card| number(field(card, "picture")))
            .collect()
    };
    let doors = result(&s, "doors");
    assert_eq!(doors.len(), 1);
    assert_eq!(field(&doors[0], "label"), &Fact::Number(1));

    s.set_level(6);
    s.run_frame().expect("a frame");
    let doors = result(&s, "doors");
    assert_eq!(doors.len(), 2);
    assert_eq!(field(&doors[0], "label"), &Fact::Number(1));
    assert_eq!(
        cards(&doors[0]),
        [9, 12, 13],
        "the door met is still door 1"
    );
    assert_eq!(
        field(&doors[1], "label"),
        &Fact::Number(2),
        "then room 30's"
    );
}

#[test]
fn items_are_marked_by_what_the_game_does_with_them() {
    let mut s = session();
    blank_planet(&mut s);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    // The core still needs pictures 5 and 6; BLOB carries a 6.
    poke_into(&mut s, "core_slots", 0, &[0x85, 0x86]);
    poke_into(&mut s, "carried", 0, &[6, 3]);
    // In room 9: the twin of the 6 carried, a card that answers any (14), a pack (20), and a
    // core piece not needed (27).
    for (k, picture) in (0..).zip([5u8, 6, 14, 20, 27]) {
        poke_into(&mut s, "items", k * 4, &[3, 20, 9, picture]);
    }
    enter(&mut s, "play");
    s.set_level(4);
    s.run_frame().expect("a frame");
    assert_eq!(
        item_marks(&s),
        [
            (5, Fact::Text("first".to_owned()), 9),
            (6, Fact::Number(7), 9),
            (14, Fact::Text("#A066FF".to_owned()), 9),
            (27, Fact::Number(7), 9),
        ],
        "a needed piece, the carried one's twin as any piece, a card, no pack, a piece"
    );
}

#[test]
fn below_level_4_the_core_and_its_pieces_show_once_the_core_is_seen() {
    let mut s = session();
    blank_planet(&mut s);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    // The core still needs a 5, and one lies in room 9, walked.
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    poke_into(&mut s, "items", 0, &[3, 20, 9, 5]);
    visit(&mut s, &[9]);
    enter(&mut s, "play");
    let at = |s: &mut Session, level: u8| {
        s.set_level(level);
        s.run_frame().expect("a frame");
        (result(s, "core").len(), item_marks(s))
    };
    let white = vec![(5, Fact::Number(7), 9)];
    let pink = vec![(5, Fact::Text("first".to_owned()), 9)];
    assert_eq!(at(&mut s, 1).0, 0, "level 1: the core not seen yet");
    assert_eq!(
        at(&mut s, 3),
        (0, white),
        "level 3: a piece, not known to be needed"
    );
    assert_eq!(
        at(&mut s, 4),
        (9, pink.clone()),
        "level 4: known regardless"
    );
    // Been in the core's room.
    visit(&mut s, &[9, 199]);
    assert_eq!(at(&mut s, 1).0, 9, "level 1: the core seen");
    assert_eq!(at(&mut s, 3), (9, pink), "level 3: needed, in pink");
}

/// Where painted rooms' stand-in medium tile pictures go, 64 bytes apart: each picture's six
/// rows, its cells' attributes in the bytes before it.
const PAINTED: u16 = 0xB000;

/// Attributes: a cell without BRIGHT blocks; a vacuum tube's cells are bright
/// (docs/re/starquake.md).
const BLOCKS: u8 = 0x07;
const TUBE: u8 = 0x64;

/// Rooms painted cell by cell from stand-in tiles: each 8 × 6 block of a room is a large tile
/// whose last medium tile, drawn at its top left, draws the block's blocking cells.
struct Painter {
    /// The next free medium and large tile; 0 to 7 are left to the tests above.
    next: u8,
    /// Each painted room's large tiles.
    tiles: std::collections::BTreeMap<usize, [u8; 12]>,
}

impl Painter {
    const fn new() -> Self {
        Self {
            next: 8,
            tiles: std::collections::BTreeMap::new(),
        }
    }

    fn take(&mut self) -> u8 {
        let id = self.next;
        self.next += 1;
        assert!(id < 64, "room for 56 stand-in tiles");
        id
    }

    /// Paints `room` from 18 rows of 32 cells, where `blocks(x, y)`.
    fn paint(&mut self, s: &mut Session, room: usize, blocks: impl Fn(usize, usize) -> bool) {
        self.paint_in(s, room, |x, y| blocks(x, y).then_some(BLOCKS));
    }

    /// Paints `room` from 18 rows of 32 cells, each drawn in an attribute or not drawn.
    fn paint_in(
        &mut self,
        s: &mut Session,
        room: usize,
        drawn: impl Fn(usize, usize) -> Option<u8>,
    ) {
        let mut tiles = [0u8; 12];
        for (i, tile) in tiles.iter_mut().enumerate() {
            let id = self.take();
            *tile = id;
            let picture = PAINTED + u16::from(id) * 64;
            let (left, top) = ((i % 4) * 8, (i / 4) * 6);
            // Each drawn cell's attribute, the first just before the picture, going down.
            let mut attribute = picture;
            for r in 0..6 {
                let mut row = 0u8;
                for c in 0..8 {
                    if let Some(a) = drawn(left + c, top + r) {
                        row |= 0x80 >> c;
                        attribute -= 1;
                        poke(s, attribute, &[a]);
                    }
                }
                poke(s, picture + u16::try_from(r).expect("a row"), &[row]);
            }
            poke_into(
                s,
                "tile_pictures",
                u16::from(id) * 2,
                &picture.to_le_bytes(),
            );
            poke_into(s, "large_tiles", u16::from(id) * 4, &[0, 0, 0, id]);
        }
        poke_into(
            s,
            "rooms",
            u16::try_from(room * 12).expect("a room"),
            &tiles,
        );
        self.tiles.insert(room, tiles);
    }

    /// Marks a painted room's cell (`x`, `y`) with a medium tile type's top four bits, `kind`:
    /// on a medium tile of the large tile there whose marker can reach it.
    fn mark(&mut self, s: &mut Session, room: usize, kind: u8, (x, y): (usize, usize)) {
        let large = self.tiles[&room][(y / 6) * 4 + x / 8];
        let (dx, dy) = (x % 8, y % 6);
        // The large tile's medium tiles are drawn at (4, 3), (0, 3), (4, 0) and (0, 0).
        let place = |(ox, oy): (usize, usize)| {
            (dx >= ox && dx - ox <= 3 && dy >= oy && dy - oy <= 3)
                .then(|| (dx - ox) | (dy - oy) << 2)
        };
        let byte = |at: usize| kind << 4 | u8::try_from(at).expect("a place in a tile");
        if let Some(at) = place((0, 0)) {
            poke_into(s, "tile_types", u16::from(large), &[byte(at)]);
            return;
        }
        let (slot, at) = [(4, 3), (0, 3), (4, 0)]
            .into_iter()
            .enumerate()
            .find_map(|(k, offset)| place(offset).map(|at| (k, at)))
            .expect("a medium tile reaches the cell");
        let id = self.take();
        poke_into(s, "tile_types", u16::from(id), &[byte(at)]);
        let slot = u16::try_from(slot).expect("a slot");
        poke_into(s, "large_tiles", u16::from(large) * 4 + slot, &[id]);
    }
}

/// The map's cells at level 6, every room's.
fn every_cell(s: &mut Session) -> Vec<Fact> {
    s.set_level(6);
    s.run_frame().expect("a frame");
    assert_eq!(s.script_off(), None);
    let map = s.script_results()["map"].clone();
    let cells = list(field(&map, "cells"));
    assert_eq!(cells.len(), 512);
    cells
}

fn walls(cell: &Fact) -> String {
    match field(cell, "walls") {
        Fact::Text(walls) => walls.clone(),
        other => panic!("walls: {other:?}"),
    }
}

/// Cells of a map cell's inside, as `[x, y]` pairs.
type Spots = Vec<(i64, i64)>;

/// A cell's inside walls and doors; both empty when it has no inside.
fn inside(cell: &Fact) -> (Spots, Spots) {
    let Fact::Map(map) = cell else {
        panic!("a cell: {cell:?}")
    };
    let Some(inside) = map.get("inside") else {
        return (Vec::new(), Vec::new());
    };
    let pairs = |key| {
        list(field(inside, key))
            .iter()
            .map(|xy| {
                let xy = list(xy);
                (number(&xy[0]), number(&xy[1]))
            })
            .collect()
    };
    (pairs("walls"), pairs("doors"))
}

/// The kinds of medium tile the script looks for, by their type's top four bits
/// (docs/re/starquake.md, "Medium tile types").
const DOOR_ENTRY: u8 = 0;
const POST: u8 = 11;
const BOOTH: u8 = 13;
const PASSAGE: u8 = 15;

#[test]
fn an_edge_is_open_only_where_blob_fits_against_both_rooms() {
    let mut s = session();
    blank_planet(&mut s);
    let mut painter = Painter::new();
    // Room 116's top two rows are rock: BLOB fits against room 100's bottom edge, not
    // against 116's. Room 117 has a gap two cells wide through them.
    painter.paint(&mut s, 116, |_, y| y < 2);
    painter.paint(&mut s, 117, |x, y| y < 2 && !(10..12).contains(&x));
    enter(&mut s, "play");
    let cells = every_cell(&mut s);
    assert_eq!(walls(&cells[100]), "S");
    assert_eq!(walls(&cells[116]), "N");
    assert_eq!(walls(&cells[101]), "");
    assert_eq!(walls(&cells[117]), "");
}

#[test]
fn a_wall_across_a_room_is_drawn_inside_it_and_a_ledge_is_not() {
    let mut s = session();
    blank_planet(&mut s);
    let mut painter = Painter::new();
    // Rows 8 and 9 of room 100 are rock all the way across: two parts, one leading up and
    // one down. Room 101 has the same rock from its left wall to column 20 only: one part.
    painter.paint(&mut s, 100, |_, y| (8..10).contains(&y));
    painter.paint(&mut s, 101, |x, y| (8..10).contains(&y) && x < 20);
    enter(&mut s, "play");
    let cells = every_cell(&mut s);
    let (walls_100, doors_100) = inside(&cells[100]);
    let across: Vec<(i64, i64)> = (8..10).flat_map(|y| (0..32).map(move |x| (x, y))).collect();
    assert_eq!(walls_100, across, "the seam between the two parts");
    assert!(doors_100.is_empty());
    assert_eq!(walls(&cells[100]), "", "each part leads out");
    assert_eq!(inside(&cells[101]), (Vec::new(), Vec::new()), "a ledge");
}

#[test]
fn a_sealed_pocket_draws_no_wall() {
    let mut s = session();
    blank_planet(&mut s);
    let mut painter = Painter::new();
    // A ring of rock two cells thick, with room for BLOB inside it.
    painter.paint(&mut s, 100, |x, y| {
        (5..13).contains(&y)
            && (10..22).contains(&x)
            && !((7..11).contains(&y) && (12..20).contains(&x))
    });
    enter(&mut s, "play");
    let cells = every_cell(&mut s);
    assert_eq!(inside(&cells[100]), (Vec::new(), Vec::new()));
}

/// A room split by a wall four cells thick, columns 18 to 21, with a pocket on its right that
/// has no way out but through the wall.
fn door_room(x: usize, y: usize) -> bool {
    let wall = (18..22).contains(&x);
    let pocket = x >= 22 && (!(2..16).contains(&y) || x >= 30);
    wall || pocket
}

#[test]
fn a_door_leads_into_the_pocket_behind_it_and_is_drawn_dotted_with_its_number() {
    let mut s = session();
    blank_planet(&mut s);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    let mut painter = Painter::new();
    // Room 100's wall is a security door, its entries at (16, 8) and (22, 8); room 101's is
    // rock.
    painter.paint(&mut s, 100, door_room);
    painter.mark(&mut s, 100, DOOR_ENTRY, (16, 8));
    painter.mark(&mut s, 100, DOOR_ENTRY, (22, 8));
    painter.paint(&mut s, 101, door_room);
    enter(&mut s, "play");
    let cells = every_cell(&mut s);
    let (walls_100, doors_100) = inside(&cells[100]);
    assert!(!walls_100.is_empty(), "the wall above and below the door");
    assert!(!doors_100.is_empty(), "the door");
    assert!(
        doors_100
            .iter()
            .all(|&(x, y)| (16..24).contains(&x) && (7..10).contains(&y)),
        "a door's cells are in its two entries' tiles: {doors_100:?}"
    );
    assert!(
        walls_100.iter().all(|&(x, _)| (18..22).contains(&x)),
        "{walls_100:?}"
    );
    assert_eq!(
        inside(&cells[101]),
        (Vec::new(), Vec::new()),
        "with no door, the pocket is sealed"
    );
    let map = s.script_results()["map"].clone();
    let door = list(field(&map, "markers"))
        .into_iter()
        .find(|m| field(m, "kind") == &Fact::Text("door".to_owned()))
        .expect("the door's marker");
    assert_eq!(
        field(&door, "label"),
        &Fact::Number(1),
        "door 1 in the rail"
    );
    assert_eq!(
        (number(field(&door, "x")), number(field(&door, "y"))),
        (4, 6),
        "in room 100"
    );
    assert_eq!(
        list(field(&door, "at")),
        [Fact::Number(19), Fact::Number(8)],
        "in the middle of its tile"
    );
}

#[test]
fn a_pair_of_wall_passages_opens_the_edge_between_their_rooms() {
    let mut s = session();
    blank_planet(&mut s);
    let mut painter = Painter::new();
    // Rooms 100 and 101 meet rock to rock, a passage on each side; rooms 102 and 103 too,
    // with a passage on 102's side only.
    let left = |x: usize, _| x >= 26;
    let right = |x: usize, _| x < 6;
    painter.paint(&mut s, 100, left);
    painter.paint(&mut s, 101, right);
    painter.paint(&mut s, 102, left);
    painter.paint(&mut s, 103, right);
    painter.mark(&mut s, 100, PASSAGE, (24, 6));
    painter.mark(&mut s, 101, PASSAGE, (6, 6));
    painter.mark(&mut s, 102, PASSAGE, (24, 6));
    enter(&mut s, "play");
    let cells = every_cell(&mut s);
    assert_eq!(walls(&cells[100]), "");
    assert_eq!(walls(&cells[101]), "");
    assert_eq!(walls(&cells[102]), "E", "a passage alone leads nowhere");
    assert_eq!(walls(&cells[103]), "W");
}

#[test]
fn the_core_s_room_is_open_where_blob_fits_against_the_room_beside() {
    let mut s = session();
    blank_planet(&mut s);
    let mut painter = Painter::new();
    // The core's room is shut in rock all round, with room for BLOB in its middle. Rooms 183,
    // 200 and 215 are rock against it too; room 198, on its left, is open.
    painter.paint(&mut s, 199, |x, y| {
        !((4..14).contains(&y) && (10..22).contains(&x))
    });
    painter.paint(&mut s, 183, |_, y| y >= 16);
    painter.paint(&mut s, 200, |x, _| x < 2);
    painter.paint(&mut s, 215, |_, y| y < 2);
    enter(&mut s, "play");
    let cells = every_cell(&mut s);
    assert_eq!(walls(&cells[199]), "NES", "entered from its left");
    assert_eq!(walls(&cells[198]), "");
    assert_eq!(walls(&cells[200]), "W");
    assert_eq!(walls(&cells[183]), "S");
    assert_eq!(inside(&cells[199]), (Vec::new(), Vec::new()));
}

/// The map's cells as (room, dim, dashed).
fn looks(s: &Session) -> Vec<(i64, bool, bool)> {
    let map = s.script_results()["map"].clone();
    let flag = |cell: &Fact, key| {
        let Fact::Map(map) = cell else {
            panic!("{cell:?}")
        };
        map.get(key) == Some(&Fact::Flag(true))
    };
    let mut cells: Vec<(i64, bool, bool)> = list(field(&map, "cells"))
        .iter()
        .map(|c| {
            let room = number(field(c, "y")) * 16 + number(field(c, "x"));
            (room, flag(c, "dim"), flag(c, "dashed"))
        })
        .collect();
    cells.sort_unstable();
    cells
}

#[test]
fn rooms_not_entered_are_dimmer_and_a_needed_piece_s_room_is_outlined() {
    let mut s = session();
    blank_planet(&mut s);
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    // The core still needs picture 5, lying in room 300 (its ninth bit is the second byte's
    // top bit), which isn't walked.
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    poke_into(&mut s, "items", 0, &[3, 0x94, 0x2C, 5]);
    visit(&mut s, &[8]);
    enter(&mut s, "play");
    s.set_level(3);
    s.run_frame().expect("a frame");
    assert_eq!(looks(&s), [(8, false, false)], "level 3: rooms walked");
    s.set_level(4);
    s.run_frame().expect("a frame");
    assert_eq!(
        looks(&s),
        [(8, false, false), (300, false, true)],
        "level 4: the piece's room outlined"
    );
    s.set_level(6);
    s.run_frame().expect("a frame");
    let cells = looks(&s);
    assert_eq!(cells.len(), 512);
    assert_eq!(cells[8], (8, false, false), "walked");
    assert_eq!(cells[300], (300, true, false), "never entered");
}

/// Puts BLOB in `room` with his top left on cell (`x`, `y`): his left edge in pixels, and his
/// height up from the screen's bottom as entering a room lines it up (docs/re/starquake.md).
fn stand(s: &mut Session, room: u16, (x, y): (u8, u8)) {
    poke_into(s, "room", 0, &room.to_le_bytes());
    poke_into(s, "blob_x", 0, &[x * 8]);
    poke_into(s, "blob_y", 0, &[(17 - y) * 8 + 7]);
}

/// Item record `k`: a `picture` in `room`, laid down with its top left on a cell, or not yet
/// placed (docs/re/starquake.md, "The items").
fn lay(s: &mut Session, k: u16, room: u16, picture: u8, cell: Option<(u8, u8)>) {
    let ninth = u8::try_from(room >> 8).expect("a bit") << 7;
    let [low, _] = room.to_le_bytes();
    let (x, row) = cell.map_or((0, 0), |(x, y)| (x, y + 6));
    poke_into(s, "items", k * 4, &[x, row | ninth, low, picture]);
}

/// A route on the map: its colour, its label, the edge it leaves BLOB's room by, and its
/// steps as `"x,y"`, with how it comes to a step where it isn't a solid line.
#[derive(Debug, PartialEq, Eq)]
struct Shown {
    color: String,
    label: String,
    exit: Option<String>,
    steps: Vec<String>,
}

fn text(fact: &Fact) -> String {
    match fact {
        Fact::Text(t) => t.clone(),
        other => panic!("text: {other:?}"),
    }
}

fn routes(s: &Session) -> Vec<Shown> {
    assert_eq!(s.script_off(), None);
    let map = s.script_results()["map"].clone();
    list(field(&map, "routes"))
        .iter()
        .map(|route| {
            let Fact::Map(fields) = route else {
                panic!("a route: {route:?}")
            };
            Shown {
                color: text(&fields["color"]),
                label: text(&fields["label"]),
                exit: fields.get("exit").map(text),
                steps: list(&fields["steps"])
                    .iter()
                    .map(|step| {
                        let step = list(step);
                        let xy = format!("{},{}", number(&step[0]), number(&step[1]));
                        step.get(2)
                            .map_or_else(|| xy.clone(), |how| format!("{xy} {}", text(how)))
                    })
                    .collect(),
            }
        })
        .collect()
}

fn steps(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|&c| c.to_owned()).collect()
}

/// The ring markers on the map, as (room, color).
fn rings(s: &Session) -> Vec<(i64, String)> {
    let map = s.script_results()["map"].clone();
    list(field(&map, "markers"))
        .iter()
        .filter(|m| field(m, "kind") == &Fact::Text("target".to_owned()))
        .map(|m| {
            let room = number(field(m, "y")) * 16 + number(field(m, "x"));
            (room, text(field(m, "color")))
        })
        .collect()
}

/// A new game's planet with nothing in its way: seeded, BLOB at the top left of room 8.
fn routed_planet(s: &mut Session) {
    blank_planet(s);
    poke_into(s, "seed", 0, &[0x34, 0x12]);
    stand(s, 8, (4, 8));
}

#[test]
fn level_5_routes_to_the_nearest_missing_piece_and_tab_picks_the_next() {
    let mut s = session();
    routed_planet(&mut s);
    // The core needs pictures 5, 6 and 7: lying two, three and four rooms from BLOB.
    poke_into(&mut s, "core_slots", 0, &[0x85, 0x86, 0x87]);
    lay(&mut s, 0, 72, 7, None);
    lay(&mut s, 1, 10, 5, Some((20, 10)));
    lay(&mut s, 2, 56, 6, None);
    visit(&mut s, &[8, 9]);
    enter(&mut s, "play");
    s.set_level(4);
    s.run_frame().expect("a frame");
    assert!(routes(&s).is_empty(), "level 4: no routes");

    s.set_level(5);
    s.run_frame().expect("a frame");
    assert_eq!(
        routes(&s),
        [Shown {
            color: "first".to_owned(),
            label: "Item 1/3".to_owned(),
            exit: Some("E".to_owned()),
            steps: steps(&["8,0", "9,0", "10,0 dashed"]),
        }],
        "through rooms walked, dashed into one not"
    );

    let label = |s: &mut Session| {
        s.script_action("next_route").expect("Tab");
        s.run_frame().expect("a frame");
        let shown = routes(s);
        (shown[0].label.clone(), shown[0].steps.last().cloned())
    };
    assert_eq!(
        label(&mut s),
        ("Item 2/3".to_owned(), Some("8,3 dashed".to_owned()))
    );
    assert_eq!(
        label(&mut s),
        ("Item 3/3".to_owned(), Some("8,4 dashed".to_owned()))
    );
    assert_eq!(label(&mut s).0, "Item 1/3", "and round again");
}

#[test]
fn a_vacuum_tube_at_an_edge_carries_blob_up_it_only() {
    let mut s = session();
    routed_planet(&mut s);
    let mut painter = Painter::new();
    // Room 116's top row is a vacuum tube all the way across: up into room 100, never down
    // from it.
    painter.paint_in(&mut s, 116, |_, y| (y == 0).then_some(TUBE));
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    lay(&mut s, 0, 116, 5, None);
    stand(&mut s, 100, (4, 8));
    enter(&mut s, "play");
    s.set_level(5);
    s.run_frame().expect("a frame");
    let down = &routes(&s)[0].steps;
    assert_eq!(down.len(), 4, "round by a room beside: {down:?}");
    assert_ne!(down[1], "4,7 dashed");

    lay(&mut s, 0, 100, 5, None);
    stand(&mut s, 116, (4, 8));
    s.run_frame().expect("a frame");
    assert_eq!(routes(&s)[0].steps, steps(&["4,7", "4,6 dashed"]), "up it");
    assert_eq!(routes(&s)[0].exit.as_deref(), Some("N"));
}

#[test]
fn a_pair_of_wall_passages_is_a_step_between_their_rooms() {
    let mut s = session();
    routed_planet(&mut s);
    let mut painter = Painter::new();
    painter.paint(&mut s, 100, |x, _| x >= 26);
    painter.paint(&mut s, 101, |x, _| x < 6);
    painter.mark(&mut s, 100, PASSAGE, (24, 6));
    painter.mark(&mut s, 101, PASSAGE, (6, 6));
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    lay(&mut s, 0, 101, 5, Some((10, 8)));
    stand(&mut s, 100, (4, 8));
    enter(&mut s, "play");
    s.set_level(5);
    s.run_frame().expect("a frame");
    let shown = &routes(&s)[0];
    assert_eq!(shown.steps, steps(&["4,6", "5,6 dashed"]));
    assert_eq!(shown.exit.as_deref(), Some("E"));
}

#[test]
fn a_door_on_the_way_is_outlined_and_the_cards_that_answer_it_ringed() {
    let mut s = session();
    routed_planet(&mut s);
    let mut painter = Painter::new();
    // Room 100's wall is a security door, the pocket behind it shut but for the door.
    painter.paint(&mut s, 100, door_room);
    painter.mark(&mut s, 100, DOOR_ENTRY, (16, 8));
    painter.mark(&mut s, 100, DOOR_ENTRY, (22, 8));
    // The door met, its cards 9, 12 and 13; BLOB carries a 12.
    let mut kept = s.kept().clone();
    kept.insert("doors-met".to_owned(), vec!["4660:100:9-12-13".to_owned()]);
    s.set_kept(kept);
    poke_into(&mut s, "carried", 0, &[12, 3]);
    // A piece the core needs lies in the pocket; cards lie in other rooms: a 9 and a 13 the
    // door asks for, a 14 that answers any card, the 12's twin, and a 10 it doesn't ask for.
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    lay(&mut s, 0, 100, 5, Some((24, 8)));
    for (k, (room, picture)) in (1..).zip([(120, 9), (121, 13), (122, 14), (123, 12), (124, 10)]) {
        lay(&mut s, k, room, picture, None);
    }
    stand(&mut s, 100, (4, 8));
    enter(&mut s, "play");
    s.set_level(5);
    s.run_frame().expect("a frame");
    let shown = &routes(&s)[0];
    assert_eq!(shown.steps, steps(&["4,6"]), "within BLOB's room");
    assert_eq!(shown.exit, None);
    let doors = result(&s, "doors");
    assert_eq!(
        field(&doors[0], "outlined"),
        &Fact::Text("first".to_owned())
    );
    assert_eq!(
        rings(&s),
        [
            (120, "first".to_owned()),
            (121, "first".to_owned()),
            (122, "first".to_owned())
        ]
    );

    // With the piece outside the pocket, no door is on the way.
    lay(&mut s, 0, 100, 5, Some((4, 12)));
    s.run_frame().expect("a frame");
    let Fact::Map(door) = &result(&s, "doors")[0] else {
        panic!("a door")
    };
    assert!(!door.contains_key("outlined"));
    assert!(rings(&s).is_empty());
}

#[test]
fn a_locked_door_s_post_on_the_way_rings_the_key() {
    let mut s = session();
    routed_planet(&mut s);
    let mut painter = Painter::new();
    // Room 100's wall is one cell thick, at column 17, with a post in it whose cells, rows 8
    // to 10, open the way into the pocket behind.
    painter.paint(&mut s, 100, |x, y| {
        x == 17 || (x >= 18 && (!(2..16).contains(&y) || x >= 30))
    });
    painter.mark(&mut s, 100, POST, (16, 8));
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    lay(&mut s, 0, 100, 5, Some((24, 8)));
    lay(&mut s, 1, 120, 16, None);
    lay(&mut s, 2, 121, 9, None);
    stand(&mut s, 100, (4, 8));
    enter(&mut s, "play");
    s.set_level(5);
    s.run_frame().expect("a frame");
    assert_eq!(routes(&s)[0].steps, steps(&["4,6"]), "within BLOB's room");
    assert_eq!(
        rings(&s),
        [(120, "first".to_owned())],
        "the key, not a card"
    );

    poke_into(&mut s, "carried", 0, &[16, 3]);
    s.run_frame().expect("a frame");
    assert!(rings(&s).is_empty(), "carried: nothing to fetch");
}

#[test]
fn teleport_booths_shown_on_the_map_are_a_step_from_each_other() {
    let mut s = session();
    routed_planet(&mut s);
    let mut painter = Painter::new();
    // Booths in rooms 8 and 300, rooms that draw nothing else.
    for room in [8, 300] {
        painter.paint(&mut s, room, |_, _| false);
        painter.mark(&mut s, room, BOOTH, (12, 10));
    }
    poke_into(&mut s, "teleports", 0, b"AAAAA");
    poke_into(&mut s, "teleports", 5, &[8, 0]);
    poke_into(&mut s, "teleports", 7, b"BBBBB");
    poke_into(&mut s, "teleports", 12, &300u16.to_le_bytes());
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    lay(&mut s, 0, 301, 5, None);
    visit(&mut s, &[8]);
    enter(&mut s, "play");
    s.set_level(5);
    s.run_frame().expect("a frame");
    assert!(
        routes(&s)[0].steps.len() > 20,
        "room 300's booth not on the map: walked all the way"
    );

    visit(&mut s, &[8, 300]);
    s.run_frame().expect("a frame");
    let shown = &routes(&s)[0];
    assert_eq!(shown.steps, steps(&["8,0", "12,18 jump", "13,18 dashed"]));
    assert_eq!(shown.exit, None, "no edge: by the booth");
}

#[test]
fn carrying_a_piece_the_core_needs_routes_to_the_core_from_its_left_only() {
    let mut s = session();
    routed_planet(&mut s);
    // The core needs a 5, which BLOB carries, in room 183 above the core's room. On this
    // planet every room is open, the core's room too, but it's entered from its left only.
    poke_into(&mut s, "core_slots", 0, &[0x85]);
    poke_into(&mut s, "carried", 0, &[5, 3]);
    stand(&mut s, 183, (4, 8));
    enter(&mut s, "play");
    s.set_level(5);
    s.run_frame().expect("a frame");
    assert_eq!(
        routes(&s),
        [Shown {
            color: "second".to_owned(),
            label: "Core".to_owned(),
            exit: Some("W".to_owned()),
            steps: steps(&["7,11", "6,11 dashed", "6,12 dashed", "7,12 dashed"]),
        }]
    );
    assert_eq!(rings(&s), [(199, "second".to_owned())], "the core's end");
}

#[test]
fn routes_and_the_core_s_route_are_shown_together() {
    let mut s = session();
    routed_planet(&mut s);
    poke_into(&mut s, "core_slots", 0, &[0x85, 0x86]);
    poke_into(&mut s, "carried", 0, &[5, 3]);
    lay(&mut s, 0, 9, 6, None);
    enter(&mut s, "play");
    s.set_level(6);
    s.run_frame().expect("a frame");
    let shown = routes(&s);
    let colors: Vec<&str> = shown.iter().map(|r| r.color.as_str()).collect();
    assert_eq!(colors, ["first", "second"], "level 6 too");
    assert_eq!(shown[0].label, "Item 1/1");
}
