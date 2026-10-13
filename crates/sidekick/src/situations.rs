//! Crafted situations: a new game started, set up as a file says, and entered through the
//! definition's routine. And `--load`, which starts from one or from a save.

use crate::{keys::key_named, saves};
use sidekick_core::Session;
use sidekick_def::{Role, Situation};
use std::{fs, path::Path};

/// Frames before a start key's first tap: a second, for the game to settle.
const SETTLE: u32 = 50;
/// Frames before a start key is tapped again, the state unchanged: forty seconds, since
/// tapping often can keep a game from where it's going (a title tune starting over).
const TAP_AGAIN: u32 = 2000;
/// Frames a tap holds its key.
const TAP_FRAMES: u32 = 4;
/// The most frames spent reaching play: two emulated minutes.
const MOST_FRAMES: u32 = 6000;
/// The most frames to wait for the game to leave play to change room, once entered.
const LEAVE_FRAMES: u32 = 50;

/// The most runs spent reaching the sync address once in play.
const SYNC_TRIES: u32 = 100;

/// Starts a crafted situation from a session whose tape has gone in: the definition's start
/// keys take it to play; there, at the sync point, the values are written, the switches
/// turned on, the guidance level set and the kept lists added to, and `[situations] enter` is
/// called. The game is marked loaded.
///
/// # Errors
/// What the situation or definition lacks, or the game never reached play.
pub(crate) fn start(session: &mut Session, situation: &Situation) -> Result<(), String> {
    let definition = session
        .definition()
        .ok_or("a crafted situation needs the game's definition")?;
    let problems = situation.check(definition);
    if !problems.is_empty() {
        let list: Vec<String> = problems.iter().map(ToString::to_string).collect();
        return Err(list.join("; "));
    }
    let sync = definition
        .run
        .as_ref()
        .and_then(|r| r.sync)
        .ok_or("run.sync: a situation enters play at the sync point, and there's none")?;
    let situations = definition.situations.clone().unwrap_or_default();
    let enter = situations.enter.ok_or("situations.enter: missing")?;
    reach_play(session, &situations.start)?;
    let mut at_sync = false;
    for _ in 0..SYNC_TRIES {
        if session.run_to_sync().map_err(|e| e.to_string())? {
            at_sync = true;
            break;
        }
    }
    if !at_sync {
        return Err("the game never reached its sync address in play".to_owned());
    }
    for (name, &number) in &situation.values {
        session
            .write(name, number)
            .map_err(|e| format!("{name}: {e}"))?;
    }
    for id in &situation.cheats {
        session
            .switch(id, true)
            .map_err(|e| format!("cheats.{id}: {e}"))?;
    }
    if let Some(level) = situation.level {
        session.set_level(level);
    }
    let mut kept = session.kept().clone();
    for (list, entries) in &situation.kept {
        let have = kept.entry(list.clone()).or_default();
        for entry in entries {
            if !have.contains(entry) {
                have.push(entry.clone());
            }
        }
    }
    session.set_kept(kept);
    // Told against what was written, so a hold switch keeps the situation's value.
    session.reading_anew();
    session.machine_mut().call(enter.0, sync.0);
    session.mark_loaded();
    through_the_change_of_room(session)
}

/// Runs through the game's change of room after entering one: out of play, and back. So the
/// game starts, and F9 returns, with the room set up drawn, not the one the game started in.
/// A game that doesn't leave play to change room goes on at once.
fn through_the_change_of_room(session: &mut Session) -> Result<(), String> {
    let play = session
        .definition()
        .and_then(|d| d.state_for(Role::Play))
        .map(str::to_owned);
    let in_play = |session: &Session| session.state() == play.as_deref();
    let mut left = false;
    for _ in 0..LEAVE_FRAMES {
        session.run_frame().map_err(|e| e.to_string())?;
        if !in_play(session) {
            left = true;
            break;
        }
    }
    if left {
        for _ in 0..MOST_FRAMES {
            if in_play(session) {
                return Ok(());
            }
            session.run_frame().map_err(|e| e.to_string())?;
        }
        return Err("the game didn't come back to play after entering the room".to_owned());
    }
    Ok(())
}

/// Taps each start key in turn, a second after the last state change, and again every forty
/// seconds while the state stays; then waits for play.
fn reach_play(session: &mut Session, keys: &[String]) -> Result<(), String> {
    let play = session
        .definition()
        .and_then(|d| d.state_for(Role::Play))
        .map(str::to_owned)
        .ok_or("a crafted situation needs a state with role = \"play\"")?;
    let mut frames = 0;
    let mut frame = |session: &mut Session| -> Result<(), String> {
        frames += 1;
        if frames > MOST_FRAMES {
            return Err(format!(
                "the game didn't reach play in {} seconds",
                MOST_FRAMES / 50
            ));
        }
        if session.holding() {
            session.release();
        }
        session.run_frame().map_err(|e| e.to_string())
    };
    for name in keys {
        let key = key_named(name).ok_or_else(|| format!("situations.start: no key {name}"))?;
        // The last state known, not this frame's: a state may match only some frames.
        let before = session.progress().last_known.clone();
        let mut since = TAP_AGAIN - SETTLE;
        while session.progress().last_known == before {
            if since == TAP_AGAIN {
                since = 0;
                session.machine_mut().key(key, true);
                for _ in 0..TAP_FRAMES {
                    frame(session)?;
                }
                session.machine_mut().key(key, false);
            }
            frame(session)?;
            since += 1;
        }
    }
    while session.progress().last_known.as_deref() != Some(play.as_str()) {
        frame(session)?;
    }
    Ok(())
}

/// What `--load` starts from: a crafted situation (`.toml`) or a save (`.sav`, `.szx`).
///
/// # Errors
/// Why it couldn't be: unreadable, refused, or the situation couldn't be set up.
pub(crate) fn load(session: &mut Session, saves: &saves::Saves, file: &Path) -> Result<(), String> {
    let name = file.display();
    let extension = file
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    match extension.as_deref() {
        Some("toml") => {
            let text = fs::read_to_string(file).map_err(|e| format!("{name}: {e}"))?;
            let situation = Situation::from_toml(&text).map_err(|e| format!("{name}: {e}"))?;
            start(session, &situation).map_err(|e| format!("{name}: {e}"))
        }
        Some("sav" | "szx") => saves::read(file, session, saves)
            .map_err(|refused| format!("{name}: {}", refused.words())),
        _ => Err(format!(
            "{name}: --load takes a crafted situation (.toml) or a save (.sav, .szx)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::Game;
    use sidekick_def::{tapes_in, Package};
    use std::path::PathBuf;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn starquake_s_situations_reach_play_as_written() {
        let zip = repo().join("assets/Starquake.tap.zip");
        if !zip.exists() {
            eprintln!("skipped: no {} (the player's own tape)", zip.display());
            return;
        }
        let tapes = tapes_in(&zip).expect("the zip holds a tape");
        let package = Package::load(&repo().join("games/starquake")).expect("its definition");
        let data = tempfile::TempDir::new().unwrap();
        let game = Game::new(package, data.path());
        let folder = repo().join("games/starquake/tests");
        let mut files: Vec<PathBuf> = fs::read_dir(&folder)
            .expect("its situations")
            .map(|e| e.expect("an entry").path())
            .collect();
        files.sort();
        assert!(!files.is_empty());
        // Lower than a new game's, under a switch that holds it from going down; at a level.
        let held = data.path().join("held.toml");
        fs::write(
            &held,
            "lives = 1\ncheats = [\"endless-lives\"]\nlevel = 6\n",
        )
        .unwrap();
        files.push(held);
        for file in files {
            let mut playing = game.playing(&tapes[0]).expect("the tape plays");
            let s = &mut playing.session;
            load(s, &playing.saves, &file).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                s.state(),
                Some("play"),
                "{}: through the change of room, in play at once",
                file.display()
            );
            for _ in 0..100 {
                s.run_frame().expect("a frame");
            }
            let text = fs::read_to_string(&file).unwrap();
            let situation = Situation::from_toml(&text).unwrap();
            assert_eq!(s.state(), Some("play"), "{}", file.display());
            // Entering a room lines BLOB up on its grid, lower than written.
            for (name, &number) in situation.values.iter().filter(|(n, _)| *n != "blob_y") {
                assert_eq!(
                    s.reading()[name].number(),
                    Some(number),
                    "{}: {name}",
                    file.display()
                );
            }
            assert!(situation.cheats.iter().all(|id| s.switched_on(id)));
            if let Some(level) = situation.level {
                assert_eq!(s.level(), level, "{}: its guidance level", file.display());
            }
            assert!(s.record().loaded, "marked");
        }
    }

    #[test]
    fn load_takes_saves_and_situations_and_refuses_the_rest() {
        let data = tempfile::TempDir::new().unwrap();
        let machine = sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).unwrap();
        let mut session = Session::new(machine, None);
        let saves = saves::Saves::of_tape("t.tap", "ABC123", data.path());
        let other = data.path().join("notes.txt");
        fs::write(&other, "x").unwrap();
        assert!(load(&mut session, &saves, &other)
            .unwrap_err()
            .contains("--load takes"));
        let missing = data.path().join("none.sav");
        assert!(load(&mut session, &saves, &missing)
            .unwrap_err()
            .contains("No save yet"));
        let crafted = data.path().join("c.toml");
        fs::write(&crafted, "lives = 3\n").unwrap();
        assert!(
            load(&mut session, &saves, &crafted)
                .unwrap_err()
                .contains("needs the game's definition"),
            "a tape played plainly has no names"
        );
    }
}
