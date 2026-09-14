//! Following the game from what it does: which part of the program is
//! running, from the entry points it arrives at, passed on to the guidance
//! panel so it knows when a game starts and ends.

use sidekick::starquake::routine;

use super::guidance::Guidance;

/// Which part of the program is running, for the panel beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scene {
    /// Before the game has reached its title screen.
    #[default]
    Loading,
    /// The title screen and its menu, and a new game's intro text.
    Menu,
    /// A game being played, with the deaths along the way.
    Play,
    /// The end of a game: the scores, entering initials, the high-score
    /// table.
    GameOver,
}

/// The routines whose arrival says which scene the program is in.
pub const WATCH: [u16; 3] = [routine::MENU, routine::MAIN_LOOP, routine::GAME_OVER];

#[derive(Default)]
pub struct Tracker {
    pub scene: Scene,
}

impl Tracker {
    /// Takes in one watched routine the program arrived at, and tells
    /// `guidance` when a game starts, which begins its record, and whether
    /// one is being played. Returns the new scene if it changed.
    pub fn follow(&mut self, hit: u16, guidance: &mut Guidance) -> Option<Scene> {
        let next = match hit {
            routine::MENU => Scene::Menu,
            routine::MAIN_LOOP => Scene::Play,
            routine::GAME_OVER => Scene::GameOver,
            _ => self.scene,
        };
        if next == self.scene {
            return None;
        }
        if next == Scene::Play {
            guidance.new_game();
        }
        guidance.set_playing(next == Scene::Play);
        self.scene = next;
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::guidance::{Record, Setting};

    #[test]
    fn the_scene_follows_the_routines_the_program_arrives_at() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        assert_eq!(t.scene, Scene::Loading);
        assert_eq!(t.follow(routine::MENU, &mut g), Some(Scene::Menu));
        assert_eq!(t.follow(routine::MENU, &mut g), None, "no change");
        assert_eq!(t.follow(routine::MAIN_LOOP, &mut g), Some(Scene::Play));
        assert_eq!(t.follow(routine::MAIN_LOOP, &mut g), None, "every frame");
        assert_eq!(t.follow(routine::GAME_OVER, &mut g), Some(Scene::GameOver));
        assert_eq!(t.follow(routine::MENU, &mut g), Some(Scene::Menu));
        assert_eq!(t.follow(0x1234, &mut g), None, "not a watched routine");
    }

    #[test]
    fn a_game_starting_begins_its_record_and_offers_to_end_it() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        g.set_level(3);
        g.set_training(true);
        g.set_level(1);
        t.follow(routine::MENU, &mut g);
        assert!(!g.rows().contains(&Setting::EndGame), "nothing to end yet");
        t.follow(routine::MAIN_LOOP, &mut g);
        assert_eq!(
            g.record(),
            Record {
                highest: 1,
                training: true
            },
            "what is in use as the game starts"
        );
        assert!(g.rows().contains(&Setting::EndGame));
        g.set_level(2);
        t.follow(routine::GAME_OVER, &mut g);
        assert!(!g.rows().contains(&Setting::EndGame), "over");
        assert_eq!(g.record().highest, 2, "kept for the score note");
    }
}
