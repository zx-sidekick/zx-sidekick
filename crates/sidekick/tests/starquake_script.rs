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

#[test]
fn a_room_walked_is_mapped_with_the_edges_the_screen_shuts() {
    let mut s = session();
    let (seed, room, area) = (addr(&s, "seed"), addr(&s, "room"), addr(&s, "play_area"));
    poke(&mut s, seed, &[0x34, 0x12]);
    poke(&mut s, room, &[8, 0]);
    // Open everywhere (BRIGHT), but the top row and the right column, which block.
    let mut cells = [0x47u8; 576];
    cells[..32].fill(0x07);
    for row in 0..18 {
        cells[row * 32 + 31] = 0x07;
    }
    // A one-cell gap on the left is too small to count as an opening.
    for row in 0..18 {
        cells[row * 32] = if row == 9 { 0x47 } else { 0x07 };
    }
    poke(&mut s, area, &cells);
    enter(&mut s, "play");
    s.run_frame().expect("a frame");
    assert_eq!(kept(&s, "rooms-walked"), ["4660:8:NEW"]);
    // The map is drawn from the kept rooms as the frame began: it shows the room a frame on.
    s.run_frame().expect("a frame");

    let map = s.script_results()["map"].clone();
    let Fact::List(rooms) = field(&map, "cells").clone() else {
        panic!("cells");
    };
    assert_eq!(rooms.len(), 1);
    assert_eq!(field(&rooms[0], "x"), &Fact::Number(8));
    assert_eq!(field(&rooms[0], "walls"), &Fact::Text("NEW".to_owned()));
    let Fact::List(markers) = field(&map, "markers").clone() else {
        panic!("markers");
    };
    assert!(markers
        .iter()
        .any(|m| field(m, "kind") == &Fact::Text("player".to_owned())));

    poke(&mut s, seed, &[0x35, 0x12]);
    poke(&mut s, room, &[9, 0]);
    s.run_frame().expect("a frame");
    let map = s.script_results()["map"].clone();
    assert_eq!(
        field(&map, "cells"),
        &Fact::List(Vec::new()),
        "another game: none walked yet"
    );
}

/// A room's attributes: open (BRIGHT) everywhere but its bottom row, where only the cells
/// `gap` let BLOB through.
fn room_with_floor(gap: std::ops::Range<usize>) -> [u8; 576] {
    let mut cells = [0x47u8; 576];
    for column in 0..32 {
        cells[17 * 32 + column] = if gap.contains(&column) { 0x47 } else { 0x07 };
    }
    cells
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
fn a_room_is_kept_once_a_game_from_two_readings_in_play() {
    let mut s = session();
    let (seed, room, area) = (addr(&s, "seed"), addr(&s, "room"), addr(&s, "play_area"));
    poke(&mut s, seed, &[0x34, 0x12]);
    poke(&mut s, room, &[8, 0]);
    // Two open cells at the bottom are a way down: about BLOB's width.
    poke(&mut s, area, &room_with_floor(5..7));
    enter(&mut s, "booth");
    s.run_frame().expect("a frame");
    assert!(kept(&s, "rooms-walked").is_empty(), "not in play");
    enter(&mut s, "play");
    s.run_frame().expect("a frame");
    assert_eq!(kept(&s, "rooms-walked"), ["4660:8:"], "open all round");

    // The floor shut later (a platform across the gap): the first reading stands.
    poke(&mut s, area, &room_with_floor(5..6));
    for _ in 0..3 {
        s.run_frame().expect("a frame");
    }
    assert_eq!(kept(&s, "rooms-walked"), ["4660:8:"], "kept once a game");

    // A new room isn't kept on its first frame, only once read twice the same.
    poke(&mut s, room, &[9, 0]);
    s.run_frame().expect("a frame");
    assert_eq!(kept(&s, "rooms-walked").len(), 1, "not on its first frame");
    s.run_frame().expect("a frame");
    assert_eq!(
        kept(&s, "rooms-walked"),
        ["4660:8:", "4660:9:S"],
        "one open cell isn't a way down"
    );
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
