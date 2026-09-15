//! Following the game from what it does: which part of the program is
//! running, from the entry points it arrives at, passed on to the guidance
//! panel so it knows when a game starts and ends; and the teleporter booths
//! entered and the security door codes seen, for level 1 (#4, #49).

use sidekick::map::{Graph, Known, Place, RoomSet, Step};
use sidekick::starquake::{
    CORE_ROOM, Item, SeenTeleporter, at, door_code, entry, font, graphic, hole, items_and_core,
    kind, missing_pieces, routine, teleporter_code,
};

use super::guidance::{DoorCode, Found, Guidance, Hole};

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
/// program is in, a new game, a teleporter booth entered, and a security
/// door's screen opened.
pub const WATCH: [u16; 8] = [
    routine::MENU,
    routine::HEROES,
    routine::MAIN_LOOP,
    routine::GAME_OVER,
    routine::NEW_GAME,
    routine::TELEPORT_BOOTH,
    routine::ENTER_ROOM,
    routine::DOOR_SCREEN,
];

#[derive(Default)]
pub struct Tracker {
    pub scene: Scene,
    /// The booths entered this game, in the order they were entered.
    seen: Vec<SeenTeleporter>,
    /// The connections walked this game (#9).
    known: Known,
    /// The room last entered this game, which a walk into the next starts
    /// from.
    entered: Option<u16>,
    /// The door codes seen this game, in the order their screens opened.
    doors: Vec<DoorCode>,
    /// The room whose door screen has opened, until the room is entered
    /// again when the screen is done, which is when its code is read.
    door_opened: Option<u16>,
    /// The planet as the map reads it, for level 5's routes (#10); empty
    /// until the program has read the rooms.
    pub graph: Graph,
}

impl Tracker {
    /// Takes in one watched routine the program arrived at, with the
    /// machine's memory `mem` as it arrived, and tells `guidance` when a game
    /// starts, which begins its record, whether one is being played, and
    /// which teleporters' booths have been entered and which security doors'
    /// codes have been seen. The codes are forgotten
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
            routine::DOOR_SCREEN => {
                self.door_opened = Some(u16::from_le_bytes([
                    mem[usize::from(at::ROOM)],
                    mem[usize::from(at::ROOM) + 1],
                ]));
            }
            routine::NEW_GAME | routine::MENU => {
                self.seen.clear();
                self.doors.clear();
                self.door_opened = None;
                self.known = Known::default();
                self.entered = None;
            }
            // The room and why it was entered are both set as the game
            // enters it; the room number alone changes earlier for a teleport.
            routine::ENTER_ROOM => {
                let room = u16::from_le_bytes([
                    mem[usize::from(at::ROOM)],
                    mem[usize::from(at::ROOM) + 1],
                ]);
                if mem[usize::from(at::ENTRY_REASON)] == entry::WALKED
                    && let Some(from) = self.entered
                {
                    self.known.walked(from, room);
                }
                self.entered = Some(room);
                // Back from a door's screen: its code is the one just shown.
                if self.door_opened.take() == Some(room)
                    && let Some(chips) = door_code(mem)
                    && !self.doors.iter().any(|d| d.room == room)
                {
                    self.doors.push(DoorCode {
                        room,
                        chips,
                        graphics: chips.map(|g| graphic(mem, g)),
                    });
                }
            }
            _ => {}
        }
        guidance.set_teleporters(&self.seen);
        guidance.set_door_codes(&self.doors);
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
            if let Some(font) = font(mem) {
                guidance.set_font(font);
            }
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
                let pieces = missing_pieces(&core, &items);
                guidance.set_core(holes(mem, &core, &items));
                let found = found(mem, &items, &core, &unvisited);
                let here = u16::from_le_bytes([mem[room], mem[room + 1]]);
                let booths: Vec<u16> = self.seen.iter().map(|t| t.room).collect();
                // Level 5 routes over the whole map from the place Blob is in
                // (#10); below it, over the ways walked (#9).
                let blob = usize::from(at::ENTITIES);
                let place = (guidance.level() >= 5)
                    .then(|| self.graph.place(here, mem[blob + 5], mem[blob + 6]));
                let whole = place.map(|p| (&self.graph, p));
                let (nearest, core) =
                    routes(&self.known, whole, here, &booths, &pieces, guidance.core());
                // Tab or Y switches between the three nearest (#51).
                let ends: Vec<u16> = nearest
                    .iter()
                    .map(|r| r.last().map_or(here, |s| s.room))
                    .collect();
                let (which, chosen) =
                    choose(&ends, guidance.chosen_piece(), guidance.take_switch());
                let count = u8::try_from(ends.len()).unwrap_or(u8::MAX);
                guidance.set_piece_choice(
                    chosen,
                    (u8::try_from(which).map_or(0, |w| w + 1).min(count), count),
                );
                guidance.set_route(nearest.get(which).cloned());
                guidance.set_core_route(core);
                let mut rooms = RoomSet::default();
                for item in &pieces {
                    rooms.set(item.room(), true);
                }
                guidance.set_pieces(&rooms);
                guidance.set_items(&found);
            }
            Scene::GameOver => guidance.set_room(None),
            Scene::Loading | Scene::Menu => guidance.forget_map(),
        }
    }
}

/// How many of the nearest missing pieces the piece route can switch
/// between (#51).
const NEAREST: usize = 3;

/// The routes from `here` (#44): to the missing pieces of `pieces`, the
/// nearest [`NEAREST`] rooms of them nearest first (#51); and to the core
/// room while a piece it needs is carried (#44,
/// decision 5), `None` otherwise or when there is no way. Over the
/// connections `known`, to a piece's room, or with `whole`, over the whole
/// map from that place (#10), to the part of the room a piece is in once
/// its room has been entered and it has a spot (#50). A teleport between
/// two of `booths` counts as one step.
fn routes(
    known: &Known,
    whole: Option<(&Graph, Place)>,
    here: u16,
    booths: &[u16],
    pieces: &[Item],
    core: &[Hole],
) -> (Vec<Vec<Step>>, Option<Vec<Step>>) {
    let search = |rooms: &RoomSet, places: &[Place]| match whole {
        Some((graph, place)) => graph.route(place, booths, rooms, places),
        None => known.route(here, booths, rooms, CORE_ROOM),
    };
    // A route to each missing piece: to the part of its room it lies in
    // once its room has been entered (#50), or to the whole room until
    // then. Nearest first, one to a room, and at most [`NEAREST`] of them.
    let mut found: Vec<(usize, u16, Vec<Step>)> = Vec::new();
    for item in pieces {
        let mut rooms = RoomSet::default();
        let mut places = Vec::new();
        // Row 0 is a piece whose room has not been entered yet: no spot.
        let spot = whole
            .filter(|_| item.row() != 0)
            .map(|(graph, _)| {
                (
                    item.room(),
                    graph.part_at(item.room(), item.row(), item.column()),
                )
            })
            .filter(|&(_, part)| part != 0);
        match spot {
            Some(place) => places.push(place),
            None => rooms.set(item.room(), true),
        }
        if let Some(route) = search(&rooms, &places) {
            found.push((route.len(), item.room(), route));
        }
    }
    found.sort_by_key(|(steps, room, _)| (*steps, *room));
    let mut piece: Vec<Vec<Step>> = Vec::new();
    let mut rooms: Vec<u16> = Vec::new();
    for (_, room, route) in found {
        if piece.len() == NEAREST {
            break;
        }
        if !rooms.contains(&room) {
            rooms.push(room);
            piece.push(route);
        }
    }
    let core = core.iter().any(|h| h.carried).then(|| {
        let mut room = RoomSet::default();
        room.set(CORE_ROOM, true);
        search(&room, &[])
    });
    (piece, core.flatten())
}

/// The items found, for the map's graphics (#36): those lying in a room
/// that has been visited, outside the core room, not being carried, each
/// with its own graphic and whether the core wants it, which is the
/// game's own choice each game, kept in `core`.
fn found(mem: &[u8], items: &[Item], core: &[u8; 9], unvisited: &RoomSet) -> Vec<Found> {
    items
        .iter()
        .filter(|item| {
            item.room() != CORE_ROOM
                && item.row() != 0
                && !(1..=5).contains(&item.row())
                && !unvisited.contains(item.room())
        })
        .map(|item| Found {
            room: item.room(),
            kind: kind(item.graphic()),
            piece: core
                .iter()
                .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == item.graphic()),
            graphic: graphic(mem, item.graphic()),
        })
        .collect()
}

/// Which of the nearest pieces' rooms `ends` the piece route leads to, and
/// the room to remember as chosen (#51). The room `chosen` stays chosen
/// while it is among them; gone, the route goes back to the nearest. A
/// `switch` moves to the next, and from the last back to the nearest,
/// which is no choice: the route then follows whichever piece is nearest.
fn choose(ends: &[u16], chosen: Option<u16>, switch: bool) -> (usize, Option<u16>) {
    let found = chosen.and_then(|c| ends.iter().position(|&e| e == c));
    if switch && ends.len() > 1 {
        let next = (found.unwrap_or(0) + 1) % ends.len();
        (next, (next != 0).then(|| ends[next]))
    } else {
        (found.unwrap_or(0), found.and(chosen))
    }
}

/// The core's nine holes as the column draws them: each one's graphic from
/// the game's memory `mem`, whether it is open, and whether an item with its
/// graphic is carried, which is while its row is 1 to 5.
fn holes(mem: &[u8], core: &[u8; 9], items: &[Item]) -> Vec<Hole> {
    core.iter()
        .enumerate()
        .map(|(i, &slot)| {
            let (number, open) = hole(i, slot);
            Hole {
                graphic: graphic(mem, number),
                open,
                carried: open
                    && items
                        .iter()
                        .any(|item| item.graphic() == number && (1..=5).contains(&item.row())),
            }
        })
        .collect()
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
    fn a_door_s_code_is_kept_once_its_screen_has_shown_it() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        let mut mem = memory(0, b"ABCDE", 210);
        let code = usize::from(at::CODE);
        mem[code..code + 9].copy_from_slice(&[0x0B, 0x11, 3, 11, 3, 12, 3, 11, 3]);
        t.follow(&mem, routine::MAIN_LOOP, &mut g);
        t.follow(&mem, routine::ENTER_ROOM, &mut g);
        assert!(g.door_codes().is_empty(), "walking in shows no code");
        t.follow(&mem, routine::DOOR_SCREEN, &mut g);
        assert!(g.door_codes().is_empty(), "not until the screen is done");
        t.follow(&mem, routine::ENTER_ROOM, &mut g);
        assert_eq!(g.door_codes().len(), 1);
        assert_eq!(
            (g.door_codes()[0].room, g.door_codes()[0].chips),
            (210, [11, 12, 11])
        );
        // Opened again: still one.
        t.follow(&mem, routine::DOOR_SCREEN, &mut g);
        t.follow(&mem, routine::ENTER_ROOM, &mut g);
        assert_eq!(g.door_codes().len(), 1);
        // The pyramid's screen leaves a code of two: not a door's.
        let mut pyramid = memory(0, b"ABCDE", 300);
        pyramid[code..code + 7].copy_from_slice(&[0x0F, 0x0D, 2, 26, 3, 29, 3]);
        t.follow(&pyramid, routine::DOOR_SCREEN, &mut g);
        t.follow(&pyramid, routine::ENTER_ROOM, &mut g);
        assert_eq!(g.door_codes().len(), 1, "no code kept for 300");
        t.follow(&mem, routine::NEW_GAME, &mut g);
        assert!(g.door_codes().is_empty(), "a new game forgets them");
    }

    #[test]
    fn the_game_s_font_is_read_once_play_starts() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        let mut mem = vec![0u8; 0x10000];
        mem[usize::from(at::FONT)] = 0x7E;
        t.follow(&mem, routine::MENU, &mut g);
        assert_eq!(g.font(), None, "not on the title screen");
        t.follow(&mem, routine::MAIN_LOOP, &mut g);
        assert_eq!(g.font().map(|f| (f.len(), f[0])), Some((768, 0x7E)));
    }

    #[test]
    fn walked_entries_become_connections_and_a_teleport_does_not() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        let mut mem = vec![0u8; 0x10000];
        let mut enter = |t: &mut Tracker, g: &mut Guidance, room: u16, why: u8| {
            mem[usize::from(at::ROOM)..usize::from(at::ROOM) + 2]
                .copy_from_slice(&room.to_le_bytes());
            mem[usize::from(at::ENTRY_REASON)] = why;
            t.follow(&mem, routine::ENTER_ROOM, g);
        };
        enter(&mut t, &mut g, 40, entry::WALKED);
        enter(&mut t, &mut g, 41, entry::WALKED);
        enter(&mut t, &mut g, 300, entry::TELEPORTED);
        enter(&mut t, &mut g, 316, entry::WALKED);
        let mut want = Known::default();
        want.walked(40, 41);
        want.walked(300, 316);
        assert_eq!(
            t.known, want,
            "no step recorded for the teleport from 41 to 300"
        );
        t.follow(&[0u8; 0x10000], routine::NEW_GAME, &mut g);
        assert!(
            t.known.is_empty() && t.entered.is_none(),
            "a new game forgets them"
        );
    }

    /// A missing piece in `room` at the screen spot (`row`, `col`); row 0
    /// before its room has been entered.
    fn piece_in(room: u16, row: u8, col: u8) -> Item {
        Item([
            col,
            ((room >> 8) as u8).rotate_right(1) | row,
            room as u8,
            30,
        ])
    }

    #[test]
    fn the_core_route_shows_only_while_a_piece_it_needs_is_carried() {
        // Walked from 197: left to a piece in 196, right to 198 beside the core.
        let mut known = Known::default();
        known.walked(197, 196);
        known.walked(197, 198);
        let pieces = [piece_in(196, 12, 5)];
        let hole = |carried| Hole {
            graphic: [0; 32],
            open: true,
            carried,
        };
        let step = |room| Step {
            room,
            teleport: false,
        };
        let piece = vec![vec![step(196)]];
        let core = Some(vec![step(198), step(CORE_ROOM)]);

        let none_carried = routes(&known, None, 197, &[], &pieces, &[hole(false), hole(false)]);
        assert_eq!(none_carried, (piece.clone(), None), "no core route");

        let one_carried = routes(&known, None, 197, &[], &pieces, &[hole(false), hole(true)]);
        assert_eq!(one_carried, (piece, core.clone()), "both routes");

        let far = [piece_in(100, 12, 5)];
        let no_way = routes(&known, None, 197, &[], &far, &[hole(true)]);
        assert_eq!(
            no_way,
            (vec![], core),
            "no way to a piece leaves the core route"
        );
    }

    /// Rooms 0 and 1 open to each other on screen rows 12 and 13, every
    /// other room closed; with `split`, a wall down room 1's middle.
    fn two_rooms(split: bool) -> Graph {
        let room = |open_left: bool, open_right: bool, wall: bool| {
            sidekick::map::Room::read(
                |row, col| {
                    let edge = row == 6 || row == 23 || col == 0 || col == 31;
                    let gap = (12..14).contains(&row)
                        && ((col == 0 && open_left) || (col == 31 && open_right));
                    if (edge && !gap) || (wall && (15..17).contains(&col)) {
                        0x07
                    } else {
                        0x47
                    }
                },
                &[],
            )
        };
        let mut rooms = vec![room(false, true, false), room(true, false, split)];
        rooms.extend((2..512).map(|_| room(false, false, false)));
        Graph::new(&rooms, CORE_ROOM)
    }

    #[test]
    fn level_5_aims_at_the_part_a_piece_is_in_once_its_spot_is_known() {
        // Room 1's right half can't be reached from room 0.
        let graph = two_rooms(true);
        let place = graph.place(0, 64, 63);
        let route = |piece| {
            routes(
                &Known::default(),
                Some((&graph, place)),
                0,
                &[],
                &[piece],
                &[],
            )
            .0
        };
        let step = [vec![Step {
            room: 1,
            teleport: false,
        }]];
        assert_eq!(route(piece_in(1, 0, 0)), step, "not placed yet: its room");
        assert_eq!(route(piece_in(1, 12, 5)), step, "placed in the left half");
        assert!(
            route(piece_in(1, 12, 25)).is_empty(),
            "placed in the half cut off"
        );
    }

    #[test]
    fn level_5_routes_through_the_whole_map() {
        // Nothing walked.
        let graph = two_rooms(false);
        let pieces = [piece_in(1, 0, 0)];
        let place = graph.place(0, 64, 63);
        assert_eq!(place, (0, 1), "Blob in room 0's one part");
        let (piece, _) = routes(
            &Known::default(),
            Some((&graph, place)),
            0,
            &[],
            &pieces,
            &[],
        );
        assert_eq!(
            piece,
            [vec![Step {
                room: 1,
                teleport: false
            }]]
        );
        let (walked_only, _) = routes(&Known::default(), None, 0, &[], &pieces, &[]);
        assert!(walked_only.is_empty(), "level 4 knows no way");
    }

    #[test]
    fn the_three_nearest_pieces_nearest_first() {
        // Walked 100 → 101 → 102 → 103 → 104, a piece in each room past 100.
        let mut known = Known::default();
        for r in 100..104 {
            known.walked(r, r + 1);
        }
        let pieces: Vec<Item> = (101..=104).map(|r| piece_in(r, 0, 0)).collect();
        let (nearest, _) = routes(&known, None, 100, &[], &pieces, &[]);
        let ends: Vec<u16> = nearest.iter().map(|r| r.last().unwrap().room).collect();
        assert_eq!(ends, [101, 102, 103], "three, the fourth left out");
    }

    #[test]
    fn a_switch_goes_through_the_three_and_back_to_the_nearest() {
        let ends = [101, 102, 103];
        assert_eq!(choose(&ends, None, false), (0, None), "the nearest");
        assert_eq!(choose(&ends, None, true), (1, Some(102)));
        assert_eq!(choose(&ends, Some(102), false), (1, Some(102)), "kept");
        assert_eq!(choose(&ends, Some(102), true), (2, Some(103)));
        assert_eq!(choose(&ends, Some(103), true), (0, None), "round again");
        // Blob walked on: the chosen piece is nearest now, and still chosen.
        assert_eq!(choose(&[102, 101, 103], Some(102), false), (0, Some(102)));
    }

    #[test]
    fn a_chosen_piece_gone_from_the_three_goes_back_to_the_nearest() {
        // Picked up, or no longer among the three nearest.
        assert_eq!(choose(&[101, 103, 104], Some(102), false), (0, None));
        assert_eq!(choose(&[101, 103, 104], Some(102), true), (1, Some(103)));
        assert_eq!(
            choose(&[101], None, true),
            (0, None),
            "one: nothing to switch to"
        );
        assert_eq!(choose(&[], Some(102), true), (0, None), "none");
    }

    #[test]
    fn the_piece_route_follows_the_choice_and_a_new_game_forgets_it() {
        let mut t = Tracker::default();
        let mut g = Guidance::default();
        g.set_level(4);
        for r in 100..104 {
            t.known.walked(r, r + 1);
        }
        let mut mem = vec![0u8; 0x10000];
        mem[usize::from(at::ROOM)..usize::from(at::ROOM) + 2]
            .copy_from_slice(&100u16.to_le_bytes());
        // Three open holes wanting graphic 30, and a piece of it in 101, 102 and 103.
        let core = usize::from(at::CORE_SLOTS);
        mem[core..core + 9].fill(0x80 | 30);
        for (i, room) in [101u16, 102, 103].into_iter().enumerate() {
            let a = usize::from(at::ITEMS) + i * 4;
            mem[a..a + 4].copy_from_slice(&[
                0,
                12 | ((room >> 8) as u8).rotate_right(1),
                room as u8,
                30,
            ]);
        }
        t.follow(&mem, routine::MAIN_LOOP, &mut g);
        let end = |g: &Guidance| g.route().and_then(|r| r.last()).map(|s| s.room);
        t.publish(&mem, &mut g);
        assert_eq!((end(&g), g.piece_choice()), (Some(101), (1, 3)));
        g.switch_piece();
        t.publish(&mem, &mut g);
        assert_eq!((end(&g), g.piece_choice()), (Some(102), (2, 3)));
        t.publish(&mem, &mut g);
        assert_eq!(end(&g), Some(102), "a switch is taken once");
        g.new_game();
        t.publish(&mem, &mut g);
        assert_eq!(
            (end(&g), g.piece_choice()),
            (Some(101), (1, 3)),
            "a new game forgets"
        );
    }

    #[test]
    fn the_items_found_are_those_lying_in_a_room_visited() {
        use sidekick::starquake::Kind;
        // The core wants graphic 30; every room visited but 500.
        let core = [0x80 | 30; 9];
        let mut unvisited = RoomSet::default();
        unvisited.set(500, true);
        let item = |room: u16, row: u8, graphic: u8| {
            Item([
                0,
                ((room >> 8) as u8).rotate_right(1) | row,
                room as u8,
                graphic,
            ])
        };
        let items = [
            item(40, 12, 15),        // the door card, lying in a room seen
            item(41, 12, 16),        // the pad key
            item(42, 12, 26),        // a trade object
            item(43, 12, 9),         // a chip
            item(44, 12, 30),        // a core piece: level 3 marks its room
            item(45, 3, 15),         // carried
            item(46, 0, 15),         // its room never entered
            item(500, 12, 15),       // in a room not visited
            item(CORE_ROOM, 12, 15), // parked in the core
        ];
        let mem = vec![0u8; 0x10000];
        let found = found(&mem, &items, &core, &unvisited);
        assert_eq!(
            found
                .iter()
                .map(|f| (f.room, f.kind, f.piece))
                .collect::<Vec<_>>(),
            [
                (40, Kind::DoorCard, false),
                (41, Kind::PadKey, false),
                (42, Kind::Trade, false),
                (43, Kind::Chip(b'0'), false),
                (44, Kind::Trade, true),
            ],
            "the core's own piece comes too, marked as one"
        );
    }

    #[test]
    fn the_holes_show_their_piece_or_placeholder_and_what_is_carried() {
        let mut mem = vec![0u8; 0x10000];
        for g in 0u8..40 {
            let at = usize::from(at::GRAPHICS) + usize::from(g) * 32;
            mem[at] = g;
        }
        let mut core = [0x80 | 30; 9];
        core[0] = 0x80 | 33; // open, wanting graphic 33
        core[1] = 1; // filled: its own number
        core[2] = 0x80 | 34; // open, its piece carried
        let items = [
            Item([0x95, 12, 16, 33]), // placed
            Item([0x60, 3, 16, 34]),  // carried
        ];
        let h = holes(&mem, &core, &items);
        assert_eq!(h.len(), 9);
        assert_eq!(
            (h[0].graphic[0], h[0].open, h[0].carried),
            (33, true, false)
        );
        assert_eq!(
            (h[1].graphic[0], h[1].open, h[1].carried),
            (1, false, false)
        );
        assert_eq!((h[2].open, h[2].carried), (true, true));
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
