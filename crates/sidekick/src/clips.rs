//! How to play's clips: small copies of the game the player started, each entered into a
//! crafted situation and played by scripted input in a loop. Nothing of the game is stored:
//! each copy is the player's own game, from the moment a game first started. No SDL here.

use crate::{
    pad::{self, Mapping, Part, Play, Way},
    picture::{color, SCREEN},
    situations,
};
use sidekick_core::{Machine, Session};
use sidekick_def::{Clip as Shown, Definition, Situation};

/// The clips of a game's How to play.
#[derive(Default)]
pub(crate) struct Clips {
    each: Vec<Clip>,
    definition: Option<Definition>,
    /// The machine as it was when a game first started, which each clip is entered from.
    base: Option<Box<Machine>>,
}

/// One clip: what it shows, its situation, and the game running in it.
struct Clip {
    shown: Shown,
    /// Its situation, or why it couldn't be read or entered, and whether that's been said.
    situation: Result<Situation, String>,
    said: bool,
    /// The game entered into its situation, where each loop starts; none until entered.
    start: Option<Box<Session>>,
    /// The loop under way, its frame, the input's step and frames into it, and the parts held
    /// in the frame before.
    now: Option<Box<Session>>,
    frame: u32,
    step: usize,
    in_step: u32,
    previous: Vec<Part>,
    play: Play,
    mapping: Mapping,
}

impl Clips {
    /// The clips `definition`'s How to play shows, each with its situation as read (in the
    /// same order).
    pub(crate) fn new(
        definition: Option<&Definition>,
        situations: Vec<Result<Situation, String>>,
    ) -> Self {
        let mapping = Mapping::new(definition);
        let shown = definition
            .and_then(|d| d.about.as_ref())
            .map(|a| a.clips.clone())
            .unwrap_or_default();
        let mut situations = situations.into_iter();
        let each = shown
            .into_iter()
            .map(|shown| Clip {
                situation: situations
                    .next()
                    .unwrap_or_else(|| Err(format!("{}: not read", shown.situation))),
                shown,
                said: false,
                start: None,
                now: None,
                frame: 0,
                step: 0,
                in_step: 0,
                previous: Vec::new(),
                play: Play::new(mapping.clone()),
                mapping: mapping.clone(),
            })
            .collect();
        Self {
            each,
            definition: definition.cloned(),
            base: None,
        }
    }

    /// A game started: the first keeps its machine, which the clips are entered from.
    pub(crate) fn game_started(&mut self, game: &Session) {
        if self.base.is_none() && !self.each.is_empty() {
            self.base = Some(Box::new(game.machine().clone()));
        }
    }

    /// Each clip from its loop's start: entered into its situation first, if it hasn't been
    /// and a game has started. A clip whose situation can't be entered says why, once, and
    /// stays dark.
    #[allow(clippy::print_stderr, reason = "a clip that can't run says why")]
    pub(crate) fn restart(&mut self) {
        let (Some(base), Some(definition)) = (&self.base, &self.definition) else {
            return;
        };
        for clip in &mut self.each {
            if clip.start.is_none() && !clip.said {
                if let Ok(situation) = &clip.situation {
                    let mut session = Session::new((**base).clone(), Some(definition.clone()));
                    // A frame of play in the room once entered, so the loop starts with all of
                    // it drawn: the game draws BLOB in its first frame there.
                    let entered = situations::enter(&mut session, situation)
                        .and_then(|()| session.run_frame().map_err(|e| e.to_string()));
                    match entered {
                        Ok(()) => clip.start = Some(Box::new(session)),
                        Err(why) => clip.situation = Err(why),
                    }
                }
                if let Err(why) = &clip.situation {
                    eprintln!("How to play's clip {}: {why}", clip.shown.situation);
                    clip.said = true;
                }
            }
            clip.rewind();
        }
    }

    /// Runs a frame of each clip that runs, and starts its loop again at its end.
    ///
    /// # Errors
    /// If a clip's machine fails.
    pub(crate) fn run_frame(&mut self) -> Result<(), String> {
        for clip in &mut self.each {
            clip.run_frame()?;
        }
        Ok(())
    }

    /// Whether clip `i` runs.
    pub(crate) fn running(&self, i: usize) -> bool {
        self.each.get(i).is_some_and(|c| c.now.is_some())
    }

    /// Whether clip `i` holds what it shows pressed, in the frame to come.
    pub(crate) fn lit(&self, i: usize) -> bool {
        self.each.get(i).is_some_and(Clip::lit)
    }

    /// Clip `i`'s part of the screen, as RGBA into `rgba`; its size, when it runs.
    pub(crate) fn picture(&self, i: usize, rgba: &mut Vec<u8>) -> Option<(u32, u32)> {
        let clip = self.each.get(i)?;
        let now = clip.now.as_ref()?;
        let [left, top, width, height] = clip.shown.area;
        let picture = now.machine().picture();
        let at = |p: u32| usize::try_from(p).unwrap_or(usize::MAX);
        rgba.clear();
        for row in SCREEN.1 + top..SCREEN.1 + top + height {
            for column in SCREEN.0 + left..SCREEN.0 + left + width {
                let inside = at(column) < picture.width() && at(row) < picture.height();
                let index = if inside {
                    picture.color(at(column), at(row))
                } else {
                    0
                };
                rgba.extend(color(index));
            }
        }
        Some((width, height))
    }
}

/// A clip's input part by the name its definition gives it.
fn part(name: &str) -> Option<Part> {
    Some(match name {
        "up" => Part::DPad(Way::Up),
        "down" => Part::DPad(Way::Down),
        "left" => Part::DPad(Way::Left),
        "right" => Part::DPad(Way::Right),
        "west" => Part::West,
        "south" => Part::South,
        "east" => Part::East,
        "north" => Part::North,
        _ => return None,
    })
}

impl Clip {
    /// Back to its loop's start, nothing held.
    fn rewind(&mut self) {
        self.now = self.start.clone();
        self.frame = 0;
        self.step = 0;
        self.in_step = 0;
        self.previous = Vec::new();
        self.play = Play::new(self.mapping.clone());
    }

    /// The parts its input holds now: its step's, or none after the last.
    fn held(&self) -> Vec<Part> {
        self.shown
            .input
            .get(self.step)
            .map(|step| step.hold.iter().filter_map(|n| part(n)).collect())
            .unwrap_or_default()
    }

    fn lit(&self) -> bool {
        self.now.is_some() && self.lights(&self.held())
    }

    /// Whether holding `held` shows what the clip shows pressed: its part itself, or a part that
    /// does the same in play (the D-pad's up, for a button that pushes the joystick up).
    fn lights(&self, held: &[Part]) -> bool {
        let Some(shown) = self.shown.pad.as_deref() else {
            return false;
        };
        held.iter().any(|&p| match (shown, p) {
            ("dpad" | "stick", Part::DPad(_)) => true,
            (name, p) => part(name).is_some_and(|s| s == p || self.play.alike(s, p)),
        })
    }

    /// On to the next step once this one's frames are up, or its `until` holds.
    fn step_on(&mut self) {
        let Some(step) = self.shown.input.get(self.step) else {
            return;
        };
        self.in_step += 1;
        let done = self.in_step >= step.frames
            || step
                .until
                .as_ref()
                .is_some_and(|test| self.now.as_ref().is_some_and(|session| session.holds(test)));
        if done {
            self.step += 1;
            self.in_step = 0;
        }
    }

    fn run_frame(&mut self) -> Result<(), String> {
        let held = self.held();
        let before = std::mem::replace(&mut self.previous, held.clone());
        let Some(session) = &mut self.now else {
            return Ok(());
        };
        self.play.enter(session.state(), &|t| session.holds(t));
        for &p in held.iter().filter(|p| !before.contains(p)) {
            self.play.pressed(p);
        }
        let mut changes = self.play.update(held.into_iter());
        let patches = std::mem::take(&mut changes.patches);
        pad::send(session.machine_mut(), changes);
        let applied: Vec<_> = patches.into_iter().filter(|p| session.patch(p)).collect();
        let ran = session.run_frame();
        for patch in &applied {
            session.unpatch(patch);
        }
        ran.map_err(|e| e.to_string())?;
        self.step_on();
        self.play.frame_ran();
        self.frame += 1;
        if self.frame >= self.shown.frames {
            self.rewind();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick_core::Model;

    /// Where the stand-in game sits in play, the sync point: a loop the processor is parked in.
    const PLAY: u16 = 0x8000;
    /// The stand-in's room entry: a return.
    const ENTER: u16 = 0x8100;

    fn definition() -> Definition {
        Definition::from_toml(&format!(
            r#"
            schema = 1
            [game]
            id = "test"
            title = "Test"
            machine = "48k"
            [[media]]
            name = "the tape"
            [run]
            sync = {PLAY}
            [[states]]
            name = "play"
            role = "play"
            pc = {PLAY}
            [values.room]
            addr = 0x9000
            type = "u8"
            writable = true
            [pad.default]
            east = "jump"
            north = "jump"
            [[actions]]
            id = "jump"
            label = "Jump"
            keys = ["Q"]
            [situations]
            enter = {ENTER}
            start = ["SPACE"]
            [about]
            goal = "Win."
            [[about.clips]]
            does = "jump"
            pad = "east"
            situation = "clips/jump.toml"
            area = [8, 16, 32, 24]
            frames = 10
            input = [{{ frames = 3 }}, {{ frames = 4, hold = ["east"] }}]
            "#
        ))
        .expect("the test definition parses")
    }

    /// A game in play on a bare 48K: parked at the play state's address, the screen's first
    /// cell inked.
    fn in_play() -> Session {
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
        for _ in 0..100 {
            machine.run_frame().expect("a frame");
        }
        machine.write(PLAY, 0x18);
        machine.write(PLAY + 1, 0xFE);
        machine.write(ENTER, 0xC9);
        let mut s = Session::new(machine, Some(definition()));
        s.machine_mut().registers_mut().set_pc(PLAY);
        for _ in 0..2 {
            s.run_frame().expect("a frame");
        }
        assert_eq!(s.state(), Some("play"));
        s
    }

    fn clips() -> Clips {
        let situation = Situation::from_toml("room = 7\n").expect("it parses");
        Clips::new(Some(&definition()), vec![Ok(situation)])
    }

    #[test]
    fn a_clip_runs_once_a_game_has_started_entered_into_its_situation() {
        let mut clips = clips();
        clips.restart();
        assert!(!clips.running(0), "no game yet");
        let mut game = in_play();
        clips.game_started(&game);
        clips.restart();
        assert!(clips.running(0));
        let now = clips.each[0].now.as_ref().expect("running");
        assert_eq!(
            now.reading()["room"].number(),
            Some(7),
            "the situation's values"
        );
        game.run_frame().expect("a frame");
        assert_eq!(
            game.reading()["room"].number(),
            Some(0),
            "the player's game untouched"
        );
    }

    #[test]
    fn its_button_lights_while_its_input_holds_it_and_the_loop_starts_again() {
        let mut clips = clips();
        clips.game_started(&in_play());
        clips.restart();
        let mut lit = Vec::new();
        for _ in 0..12 {
            lit.push(clips.lit(0));
            clips.run_frame().expect("a frame");
        }
        assert_eq!(
            lit,
            [false, false, false, true, true, true, true, false, false, false, false, false],
            "frames 3 to 6 of 10, then from the start again"
        );
        assert_eq!(clips.each[0].frame, 2, "round again");
    }

    #[test]
    fn its_button_lights_for_a_part_that_does_the_same() {
        let mut definition = definition();
        let about = definition.about.as_mut().expect("its How to play");
        about.clips[0].input[1].hold = vec!["north".to_owned()];
        let situation = Situation::from_toml("room = 7\n").expect("it parses");
        let mut clips = Clips::new(Some(&definition), vec![Ok(situation)]);
        clips.game_started(&in_play());
        clips.restart();
        let mut lit = Vec::new();
        for _ in 0..8 {
            lit.push(clips.lit(0));
            clips.run_frame().expect("a frame");
        }
        assert_eq!(
            lit,
            [false, false, false, true, true, true, true, false],
            "north jumps as east does: east lights"
        );
    }

    #[test]
    fn a_step_ends_early_once_its_test_holds() {
        let mut definition = definition();
        let about = definition.about.as_mut().expect("its How to play");
        about.clips[0].input = vec![
            sidekick_def::ClipStep {
                frames: 5,
                hold: vec!["east".to_owned()],
                until: Some(sidekick_def::Test {
                    value: Some("room".to_owned()),
                    equals: Some(7),
                    ..sidekick_def::Test::default()
                }),
            },
            sidekick_def::ClipStep {
                frames: 3,
                hold: Vec::new(),
                until: None,
            },
        ];
        let situation = Situation::from_toml("room = 7\n").expect("it parses");
        let mut clips = Clips::new(Some(&definition), vec![Ok(situation)]);
        clips.game_started(&in_play());
        clips.restart();
        let mut lit = Vec::new();
        for _ in 0..4 {
            lit.push(clips.lit(0));
            clips.run_frame().expect("a frame");
        }
        assert_eq!(
            lit,
            [true, false, false, false],
            "the room is 7 after the first frame: on to the wait"
        );
    }

    #[test]
    fn a_clip_whose_situation_can_t_be_entered_stays_dark() {
        let bad = Situation::from_toml("lives = 3\n").expect("it parses");
        let mut clips = Clips::new(Some(&definition()), vec![Ok(bad)]);
        clips.game_started(&in_play());
        clips.restart();
        assert!(!clips.running(0), "no such value");
        let mut unread = Clips::new(Some(&definition()), vec![Err("missing".to_owned())]);
        unread.game_started(&in_play());
        unread.restart();
        assert!(!unread.running(0));
        assert!(unread.picture(0, &mut Vec::new()).is_none());
    }

    /// Starquake's clips, entered from a game started on the real tape; `None` without it.
    #[allow(
        clippy::print_stderr,
        reason = "a skipped test says why; how long entering took is reported"
    )]
    fn starquake() -> Option<(Clips, Session)> {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zip = repo.join("assets/Starquake.tap.zip");
        if !zip.exists() {
            eprintln!("skipped: no {} (the player's own tape)", zip.display());
            return None;
        }
        let tapes = sidekick_def::tapes_in(&zip).expect("the zip holds a tape");
        let package =
            sidekick_def::Package::load(&repo.join("games/starquake")).expect("its definition");
        let data = tempfile::TempDir::new().expect("a folder");
        let game = crate::games::Game::new(package.clone(), data.path());
        let mut playing = game.playing(&tapes[0]).expect("the tape plays");
        situations::start(&mut playing.session, &Situation::default()).expect("a game starts");
        let mut clips = crate::games::clips(&package);
        clips.game_started(&playing.session);
        let started = std::time::Instant::now();
        clips.restart();
        eprintln!("entering both clips' situations: {:?}", started.elapsed());
        Some((clips, playing.session))
    }

    /// A clip's reading of a value, now.
    fn number(clips: &Clips, i: usize, name: &str) -> i64 {
        let now = clips.each[i].now.as_ref().expect("running");
        now.reading()[name].number().expect(name)
    }

    /// The first carried slot's color, 0 while it's empty.
    fn carried(clips: &Clips, i: usize) -> u8 {
        let now = clips.each[i].now.as_ref().expect("running");
        let at = now.definition().expect("its definition").values["carried"]
            .addr
            .expect("an address")
            .0;
        now.machine().peek(at + 1)
    }

    /// Clip `i`'s steps that hold what it shows pressed: how many, and their frames in all.
    fn presses(clips: &Clips, i: usize) -> (usize, u32) {
        let clip = &clips.each[i];
        let steps: Vec<u32> = clip
            .shown
            .input
            .iter()
            .filter(|step| {
                let held: Vec<Part> = step.hold.iter().filter_map(|n| part(n)).collect();
                clip.lights(&held)
            })
            .map(|step| step.frames)
            .collect();
        (steps.len(), steps.iter().sum())
    }

    /// A reading of clip `i`'s game, to see it change.
    type Reads = dyn Fn(&Clips, usize) -> i64;

    /// What happens in one pass of each clip's loop, by the loop's frame: the frames lit, and
    /// those after which `changed` saw a change in the clip.
    struct Pass {
        lit: Vec<u32>,
        changed: Vec<u32>,
        /// The frames after which BLOB stood where he was, in his last place before the first
        /// change.
        still: Vec<u32>,
        /// The loop has come round to its start, and what `changed` read then.
        done: bool,
        again: Option<i64>,
    }

    /// Runs every clip through one pass of its loop, noting when `changed` (of the clip's
    /// game before and after a frame) says something changed.
    fn one_pass(clips: &mut Clips, changed: &[&Reads]) -> Vec<Pass> {
        let longest = clips.each.iter().map(|c| c.shown.frames).max().unwrap_or(0);
        let mut passes: Vec<Pass> = (0..clips.each.len())
            .map(|_| Pass {
                lit: Vec::new(),
                changed: Vec::new(),
                still: Vec::new(),
                done: false,
                again: None,
            })
            .collect();
        for _ in 0..longest {
            let frames: Vec<u32> = clips.each.iter().map(|c| c.frame).collect();
            let before: Vec<i64> = (0..passes.len()).map(|i| changed[i](clips, i)).collect();
            let lit: Vec<bool> = (0..passes.len()).map(|i| clips.lit(i)).collect();
            let at: Vec<i64> = (0..passes.len())
                .map(|i| number(clips, i, "blob_x"))
                .collect();
            clips.run_frame().expect("a frame");
            for (i, pass) in passes.iter_mut().enumerate() {
                if pass.done {
                    continue;
                }
                if frames[i] + 1 != clips.each[i].frame {
                    pass.done = true;
                    pass.again = Some(changed[i](clips, i));
                    continue;
                }
                if lit[i] {
                    pass.lit.push(frames[i]);
                }
                if number(clips, i, "blob_x") == at[i] {
                    pass.still.push(frames[i]);
                } else {
                    pass.still.clear();
                }
                if changed[i](clips, i) != before[i] {
                    pass.changed.push(frames[i]);
                }
            }
        }
        passes
    }

    #[test]
    fn starquake_s_clips_pick_an_item_up_and_build_two_platforms_each_loop() {
        let Some((mut clips, _game)) = starquake() else {
            return;
        };
        assert!(clips.running(0) && clips.running(1), "both entered");
        assert_eq!(carried(&clips, 0), 0, "nothing carried at the start");
        let platforms = number(&clips, 1, "platforms");
        let passes = one_pass(
            &mut clips,
            &[&|c, i| i64::from(carried(c, i)), &|c, i| {
                number(c, i, "platforms")
            }],
        );
        let ((_, east), (south_presses, south)) = (presses(&clips, 0), presses(&clips, 1));
        assert_eq!(
            passes[0].lit.len(),
            usize::try_from(east).unwrap(),
            "east lit while held"
        );
        assert_eq!(
            passes[1].lit.len(),
            usize::try_from(south).unwrap(),
            "south lit while held"
        );

        let [picked] = passes[0].changed[..] else {
            panic!("picked up once: {:?}", passes[0].changed);
        };
        assert!(
            passes[0].lit.contains(&picked),
            "while east is held: {picked}"
        );
        let length = clips.each[0].shown.frames;
        assert!(
            picked + 100 <= length,
            "two seconds at least to see it carried: frame {picked} of {length}"
        );
        let first_lit = passes[0].lit[0];
        let stood = passes[0].still.iter().filter(|&&f| f < first_lit).count();
        assert!(
            stood >= 45,
            "BLOB stands on the item a second, unlit, before east: {stood} frames"
        );

        assert_eq!(passes[1].changed.len(), south_presses, "a platform a press");
        for built in &passes[1].changed {
            assert!(
                passes[1].lit.contains(built),
                "while south is held: {built}"
            );
        }
        assert_eq!(
            passes[0].again,
            Some(0),
            "from the start again: nothing carried"
        );
        assert_eq!(passes[1].again, Some(platforms), "and none built");
    }

    #[test]
    fn blob_stands_on_each_platform_he_builds() {
        let Some((mut clips, _game)) = starquake() else {
            return;
        };
        let height = number(&clips, 1, "blob_y");
        let (built, _) = presses(&clips, 1);
        let held: u32 = clips.each[1].shown.input.iter().map(|s| s.frames).sum();
        for _ in 0..held + 5 {
            clips.run_frame().expect("a frame");
        }
        let built = i64::try_from(built).expect("a few presses");
        assert_eq!(
            number(&clips, 1, "blob_y"),
            height + 8 * built,
            "8 pixels higher on each"
        );
    }

    #[test]
    #[allow(clippy::print_stderr, reason = "it reports what it measured")]
    fn two_clips_cost_a_little_of_each_frame() {
        let Some((mut clips, mut game)) = starquake() else {
            return;
        };
        let frames = 500;
        let started = std::time::Instant::now();
        for _ in 0..frames {
            clips.run_frame().expect("a frame");
        }
        let clips_took = started.elapsed() / frames;
        let started = std::time::Instant::now();
        for _ in 0..frames {
            game.run_frame().expect("a frame");
        }
        let game_took = started.elapsed() / frames;
        eprintln!("a frame of both clips: {clips_took:?}; of the game: {game_took:?}");
        assert!(
            clips_took < std::time::Duration::from_millis(20),
            "both clips fit in a frame's 20 ms: {clips_took:?}"
        );
    }

    #[test]
    fn its_picture_is_its_part_of_the_screen() {
        let mut clips = clips();
        let mut game = in_play();
        // The screen's top-left cell, inked: its pixel at (8, 16) is in the clip's corner.
        for line in 0..8 {
            game.machine_mut()
                .write(0x4000 + 0x40 + line * 0x100 + 1, 0xFF);
        }
        game.machine_mut().draw_screen_from_memory();
        clips.game_started(&game);
        clips.restart();
        let mut rgba = Vec::new();
        assert_eq!(clips.picture(0, &mut rgba), Some((32, 24)));
        assert_eq!(rgba.len(), 32 * 24 * 4);
        assert_eq!(rgba[..4], color(0), "the inked cell's ink, in its corner");
        assert_eq!(rgba[8 * 4..9 * 4], color(7), "paper past it");
    }
}
