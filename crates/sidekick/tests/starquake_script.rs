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
            (0, Fact::Text(String::new())),
            (18, Fact::Text("N".to_owned())),
        ],
        "rooms 8 and 300, with their walls as decoded"
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
    assert_eq!(field(&cells[0], "walls"), &Fact::Text(String::new()));
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
    visit(&mut s, &[8]);
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
fn an_edge_is_open_only_where_blob_fits_against_it() {
    let mut s = session();
    poke_into(&mut s, "seed", 0, &[0x34, 0x12]);
    poke_into(&mut s, "room", 0, &[8, 0]);
    // Open everywhere but the bottom two rows and the left two columns, which block.
    let mut cells = [0x47u8; 576];
    for col in 0..32 {
        cells[16 * 32 + col] = 0x07;
        cells[17 * 32 + col] = 0x07;
    }
    for row in 0..18 {
        cells[row * 32] = 0x07;
        cells[row * 32 + 1] = 0x07;
    }
    // Two-cell gaps on the bottom edge and the left one, with rock just behind them.
    cells[17 * 32 + 5] = 0x47;
    cells[17 * 32 + 6] = 0x47;
    cells[9 * 32] = 0x47;
    cells[10 * 32] = 0x47;
    poke_into(&mut s, "play_area", 0, &cells);
    enter(&mut s, "play");
    s.run_frame().expect("a frame");
    assert_eq!(
        s.script_results()["walls"],
        Fact::Text("SW".to_owned()),
        "BLOB is two cells deep: a gap with rock behind it is no way out"
    );

    // Room behind the gaps: now BLOB fits against both edges.
    cells[16 * 32 + 5] = 0x47;
    cells[16 * 32 + 6] = 0x47;
    cells[9 * 32 + 1] = 0x47;
    cells[10 * 32 + 1] = 0x47;
    poke_into(&mut s, "play_area", 0, &cells);
    s.run_frame().expect("a frame");
    assert_eq!(s.script_results()["walls"], Fact::Text(String::new()));
}
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
