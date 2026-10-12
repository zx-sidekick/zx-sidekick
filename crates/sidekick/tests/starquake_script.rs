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
