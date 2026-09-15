//! Following the game from what it does: which part of the program is
//! running, from the entry points it arrives at, passed on to the guidance
//! panel so it knows when a game starts and ends; and the teleporter booths
//! entered, for level 1 (#4).

use sidekick::map::RoomSet;
use sidekick::starquake::{
    SeenTeleporter, at, items_and_core, missing_piece_rooms, routine, teleporter_code,
};

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

/// The routines whose arrival tells the tracker something: which scene the
/// program is in, a new game, and a teleporter booth entered.
pub const WATCH: [u16; 5] = [
    routine::MENU,
    routine::MAIN_LOOP,
    routine::GAME_OVER,
    routine::NEW_GAME,
    routine::TELEPORT_BOOTH,
];

#[derive(Default)]
pub struct Tracker {
    pub scene: Scene,
    /// The booths entered this game, in the order they were entered.
    seen: Vec<SeenTeleporter>,
}

impl Tracker {
    /// Takes in one watched routine the program arrived at, with the
    /// machine's memory `mem` as it arrived, and tells `guidance` when a game
    /// starts, which begins its record, whether one is being played, and
    /// which teleporters' booths have been entered. The codes are forgotten
    /// when a new game is set up and on the title screen, which shows none.
    /// Returns the new scene if it changed.
    pub fn follow(&mut self, mem: &[u8], hit: u16, guidance: &mut Guidance) -> Option<Scene> {
        match hit {
            routine::TELEPORT_BOOTH => {
                let room = u16::from_le_bytes([
                    mem[usize::from(at::ROOM)],
                    mem[usize::from(at::ROOM) + 1],
                ]);
                if let Some(code) = teleporter_code(mem, room)
                    && !self.seen.iter().any(|t| t.code == code)
                {
                    self.seen.push(SeenTeleporter { room, code });
                }
            }
            routine::NEW_GAME | routine::MENU => self.seen.clear(),
            _ => {}
        }
        guidance.set_teleporters(&self.seen);
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

    /// Passes on the map as the game has it now, in `mem`, after a frame:
    /// the room Blob is in, the rooms visited and the rooms holding a
    /// missing core piece while a game is played (#5, #6), no room at the
    /// game's end, and nothing on the title screen.
    pub fn publish(&self, mem: &[u8], guidance: &mut Guidance) {
        match self.scene {
            Scene::Play => {
                let room = usize::from(at::ROOM);
                guidance.set_room(Some(u16::from_le_bytes([mem[room], mem[room + 1]])));
                let start = usize::from(at::UNVISITED_ROOMS);
                let unvisited = RoomSet(mem[start..start + 64].try_into().expect("64 bytes"));
                guidance.set_unvisited(&unvisited);
                let (items, core) = items_and_core(mem);
                guidance.set_pieces(&missing_piece_rooms(&core, &items));
            }
            Scene::GameOver => guidance.set_room(None),
            Scene::Loading | Scene::Menu => guidance.forget_map(),
        }
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
        assert_eq!(t.follow(&[], routine::MENU, &mut g), Some(Scene::Menu));
        assert_eq!(t.follow(&[], routine::MENU, &mut g), None, "no change");
        assert_eq!(t.follow(&[], routine::MAIN_LOOP, &mut g), Some(Scene::Play));
        assert_eq!(
            t.follow(&[], routine::MAIN_LOOP, &mut g),
            None,
            "every frame"
        );
        assert_eq!(
            t.follow(&[], routine::GAME_OVER, &mut g),
            Some(Scene::GameOver)
        );
        assert_eq!(t.follow(&[], routine::MENU, &mut g), Some(Scene::Menu));
        assert_eq!(t.follow(&[], 0x1234, &mut g), None, "not a watched routine");
    }

    /// Memory with the teleporter table holding `code` for `room`, and Blob
    /// in `here`.
    fn memory(room: u16, code: &[u8; 5], here: u16) -> Vec<u8> {
        let mut mem = vec![0u8; 0x10000];
        let entry = usize::from(at::TELEPORTER_NAMES);
        mem[entry..entry + 5].copy_from_slice(code);
        mem[entry + 5..entry + 7].copy_from_slice(&room.to_le_bytes());
        mem[usize::from(at::ROOM)..usize::from(at::ROOM) + 2].copy_from_slice(&here.to_le_bytes());
        mem
    }

    #[test]
    fn a_booth_entered_is_seen_once_until_a_new_game() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        let mem = memory(300, b"ABCDE", 300);
        t.follow(&mem, routine::MAIN_LOOP, &mut g);
        assert!(g.teleporters().is_empty(), "walking about sees nothing");
        t.follow(&mem, routine::TELEPORT_BOOTH, &mut g);
        t.follow(&mem, routine::TELEPORT_BOOTH, &mut g);
        assert_eq!(
            g.teleporters(),
            [SeenTeleporter {
                room: 300,
                code: *b"ABCDE"
            }],
            "entered twice, seen once"
        );
        // A booth in a room the table has no teleporter for adds nothing.
        t.follow(&memory(300, b"ABCDE", 301), routine::TELEPORT_BOOTH, &mut g);
        assert_eq!(g.teleporters().len(), 1);
        t.follow(&mem, routine::GAME_OVER, &mut g);
        assert_eq!(g.teleporters().len(), 1, "kept through the game over");
        t.follow(&mem, routine::NEW_GAME, &mut g);
        assert!(g.teleporters().is_empty(), "a new game");
        t.follow(&mem, routine::TELEPORT_BOOTH, &mut g);
        t.follow(&mem, routine::MENU, &mut g);
        assert!(g.teleporters().is_empty(), "the title screen shows none");
    }

    #[test]
    fn the_map_is_published_in_play_and_forgotten_on_the_title_screen() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        let mut mem = memory(300, b"ABCDE", 40);
        let unvisited = usize::from(at::UNVISITED_ROOMS);
        mem[unvisited..unvisited + 64].fill(0xFF);
        mem[unvisited + 5] = 0x7F; // room 40 visited
        t.follow(&mem, routine::MAIN_LOOP, &mut g);
        t.publish(&mem, &mut g);
        assert_eq!((g.room(), g.explored()), (Some(40), 1));
        assert!(g.visited(40));
        t.follow(&mem, routine::GAME_OVER, &mut g);
        t.publish(&mem, &mut g);
        assert_eq!(
            (g.room(), g.explored()),
            (None, 1),
            "kept for the game over"
        );
        t.follow(&mem, routine::MENU, &mut g);
        t.publish(&mem, &mut g);
        assert_eq!(g.explored(), 0);
    }

    #[test]
    fn a_game_starting_begins_its_record_and_offers_to_end_it() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        g.set_level(3);
        g.set_training(true);
        g.set_level(1);
        t.follow(&[], routine::MENU, &mut g);
        assert!(!g.rows().contains(&Setting::EndGame), "nothing to end yet");
        t.follow(&[], routine::MAIN_LOOP, &mut g);
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
        t.follow(&[], routine::GAME_OVER, &mut g);
        assert!(!g.rows().contains(&Setting::EndGame), "over");
        assert_eq!(g.record().highest, 2, "kept for the score note");
    }
}
