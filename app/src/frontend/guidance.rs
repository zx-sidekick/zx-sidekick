//! How much help the player has asked for: the guidance level, training
//! mode, the record of both for the game in progress, and the picker that
//! changes them (#25).
//!
//! Nothing here reaches the game. The window and the game thread share it:
//! the window changes it from the keyboard and draws it, and the game thread
//! changes it from a gamepad and holds the game while the picker is open.
//! What each level shows is its own ticket's (#3); level 1's teleporter
//! codes are carried here from the game thread to the panel (#4).

use sidekick::machine::Training;
use sidekick::map::{Openings, RoomSet, Step};
use sidekick::starquake::SeenTeleporter;

/// The number of rooms on the planet.
const ROOMS: usize = (sidekick::map::COLS * sidekick::map::ROWS) as usize;

/// The levels, each including the ones before it (#3).
pub const LEVELS: [&str; 7] = [
    "Off",
    "Codes and the core",
    "The map you have walked",
    "What you have seen",
    "What you have not",
    "Routes",
    "Everything",
];

/// An item found: one lying in a room that has been visited, with what it
/// does, which is the icon the map draws for it (#36).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub room: u16,
    pub kind: sidekick::starquake::Kind,
    /// Whether the core wants it: drawn in the piece's colour, and its
    /// room's dot left out, since the item itself says more (#36).
    pub piece: bool,
    /// The game's own graphic for it, 32 bytes, as the core column reads
    /// them.
    pub graphic: [u8; 32],
    /// Whether it has been seen lying in a room walked through, which is
    /// level 3's half of the map's marks; the rest are level 4's (#66).
    pub seen: bool,
}

/// A security door whose screen has shown its code this game (#49): the
/// room, the three chips it asks for by graphic, and their graphics read
/// from the game's memory to draw them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoorCode {
    pub room: u16,
    pub chips: [u8; 3],
    pub graphics: [[u8; 32]; 3],
}

/// One of the core's nine holes as the column draws it (#7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hole {
    /// The graphic it shows, read from the game's memory: the piece while
    /// the hole is open, its placeholder once filled.
    pub graphic: [u8; 32],
    pub open: bool,
    /// Whether that piece is being carried.
    pub carried: bool,
}

/// How much help one game has had: the highest level in use at any point,
/// and whether training mode was ever on. It only ever rises within a game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub highest: u8,
    pub training: Training,
}

/// The rows of the picker, top to bottom: the guidance level, training
/// mode's four switches (#8), then the actions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    #[default]
    Level,
    /// One of training mode's four switches (#8), in the order they show.
    Time,
    Full,
    Lives,
    Unharmed,
    EndGame,
    Exit,
}

/// Training mode's switches as the picker names them: the row, its label,
/// and what it does (#8).
pub const SWITCHES: [(Setting, &str, &str); 4] = [
    (
        Setting::Time,
        "Time stands still",
        "Energy stops draining as time passes.",
    ),
    (
        Setting::Full,
        "Full gun and platforms",
        "The gun and the platform bars stay full.",
    ),
    (
        Setting::Lives,
        "Endless lives",
        "Losing a life does not cost one.",
    ),
    (
        Setting::Unharmed,
        "No harm from enemies",
        "Enemies, deadly patches and zappers cost no energy and cannot kill.",
    ),
];

/// The switch a picker row stands for, if it is one.
fn switch(setting: Setting, of: &mut Training) -> Option<&mut bool> {
    match setting {
        Setting::Time => Some(&mut of.time),
        Setting::Full => Some(&mut of.full),
        Setting::Lives => Some(&mut of.lives),
        Setting::Unharmed => Some(&mut of.unharmed),
        _ => None,
    }
}

/// Every switch either of them holds.
fn merged(a: Training, b: Training) -> Training {
    Training {
        time: a.time || b.time,
        full: a.full || b.full,
        lives: a.lives || b.lives,
        unharmed: a.unharmed || b.unharmed,
    }
}

/// Whether `on` turns on anything `was` did not.
fn newly_on(on: Training, was: Training) -> bool {
    (on.time && !was.time)
        || (on.full && !was.full)
        || (on.lives && !was.lives)
        || (on.unharmed && !was.unharmed)
}

/// Whether one switch is on.
#[must_use]
pub fn is_on(row: Setting, on: Training) -> bool {
    switch(row, &mut { on }).copied().unwrap_or(false)
}

/// Every switch `on` holds, by the picker's names.
#[must_use]
pub fn switches_on(on: Training) -> Vec<&'static str> {
    SWITCHES
        .into_iter()
        .filter(|&(row, _, _)| is_on(row, on))
        .map(|(_, label, _)| label)
        .collect()
}

/// What the picker was asked to do, once confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Abandon the game in progress, as A S D F G does.
    EndGame,
    /// Close the program.
    Exit,
}

#[derive(Clone, Debug, Default)]
pub struct Guidance {
    level: u8,
    training: Training,
    record: Record,
    picker: bool,
    /// The level and the training switches the picker shows, which take
    /// effect only when kept with Enter or A.
    picked: (u8, Training),
    /// "This will show on your score", asked when leaving the picker would
    /// add to the record, and which answer is highlighted.
    asking: bool,
    /// The row the picker has highlighted.
    focus: Setting,
    /// An action pressed once, waiting for the second press.
    armed: Option<Setting>,
    /// Whether a game is being played, which is when it can be ended.
    playing: bool,
    /// An action confirmed and not yet carried out.
    requested: Option<Action>,
    /// The teleporters whose booths were entered this game, in the order
    /// they were entered.
    teleporters: Vec<SeenTeleporter>,
    /// The security doors whose codes were seen this game, in the order
    /// their screens were opened (#49).
    door_codes: Vec<DoorCode>,
    /// The game's font, read from memory once play starts, for drawing
    /// codes in its letters (#49); empty until then.
    font: Vec<u8>,
    /// Every teleporter and every security door's code, whether or not it
    /// has been shown: level 6 tells them all, read once a game has started
    /// (#66). Empty until then.
    all_teleporters: Vec<SeenTeleporter>,
    all_door_codes: Vec<DoorCode>,
    /// Which game those codes are being read for: a reading that finishes
    /// after another game has started is dropped rather than shown (#66).
    game: u64,
    /// Every room's openings, for the map (#5). Empty until they are read.
    openings: Vec<Openings>,
    /// The rooms visited in the game being played, or just ended; empty on
    /// the title screen.
    visited: Vec<bool>,
    /// The room Blob is in, while a game is being played.
    room: Option<u16>,
    /// The rooms holding a core piece still needed, for level 3 (#6), in
    /// the game being played or just ended.
    pieces: RoomSet,
    /// The items found and left lying in a room visited, for level 2
    /// (#36), each with what it does.
    items: Vec<Found>,
    /// The core's nine holes, in the game being played or just ended; empty
    /// on the title screen.
    core: Vec<Hole>,
    /// The route to the nearest missing piece over known connections, for
    /// level 4 (#9); `None` when there is none.
    route: Option<Vec<Step>>,
    /// The route to the core while a piece it needs is carried (#44);
    /// `None` otherwise or when there is none.
    core_route: Option<Vec<Step>>,
    /// The room of the first security door each route has to pass (#99):
    /// the piece route's, then the core route's.
    route_doors: [Option<u16>; 2],
    /// The high-score table kept between runs, with each entry's guidance
    /// (#47); `None` until the tape is loaded.
    high_scores: Option<super::scores::Kept>,
    /// Which entry of it this game put in, for the panel to mark.
    this_game: Option<u8>,
    /// The room of the missing piece the player switched the route to with
    /// Tab or Y (#51), or `None` while it leads to the nearest.
    chosen_piece: Option<u16>,
    /// A switch asked for and not yet taken by the tracker.
    switch_piece: bool,
    /// Which of the nearest missing pieces the route leads to, and of how
    /// many: (1, 3) for the nearest of three, (0, 0) with none.
    piece_choice: (u8, u8),
    /// The letters the connected pad carries, for the legends (#101).
    pad: crate::frontend::gamepad::Layout,
    /// Bumped on every change, so a watcher can tell something changed.
    version: u64,
}

impl Guidance {
    /// The guidance level in effect.
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Whether training mode is in effect.
    pub fn training(&self) -> Training {
        self.training
    }

    pub fn record(&self) -> Record {
        self.record
    }

    pub fn picker_open(&self) -> bool {
        self.picker
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// The letters the connected pad carries, which every legend follows
    /// (#101).
    pub fn pad(&self) -> crate::frontend::gamepad::Layout {
        self.pad
    }

    /// Takes the layout from the pad, if it has changed: the version moves
    /// with it, so the window redraws the badges.
    pub fn set_pad(&mut self, layout: crate::frontend::gamepad::Layout) {
        if self.pad != layout {
            self.pad = layout;
            self.version += 1;
        }
    }

    pub fn focus(&self) -> Setting {
        self.focus
    }

    pub fn armed(&self) -> Option<Setting> {
        self.armed
    }

    /// The level and training mode chosen in the picker, not yet in effect.
    pub fn picked(&self) -> (u8, Training) {
        self.picked
    }

    /// Whether the question about the score is up (#64).
    pub fn asking(&self) -> bool {
        self.asking
    }

    /// Whether keeping what is chosen in the picker would add to this
    /// game's record: a level above the highest used, or training mode for
    /// the first time. Lowering either never does.
    pub fn raises_record(&self) -> bool {
        let (level, training) = self.picked;
        level > self.record.highest || newly_on(training, self.record.training)
    }

    /// The teleporters seen this game.
    pub fn teleporters(&self) -> &[SeenTeleporter] {
        &self.teleporters
    }

    /// Takes the game's list of teleporters seen, if it has changed.
    pub fn set_teleporters(&mut self, seen: &[SeenTeleporter]) {
        if self.teleporters != seen {
            self.teleporters = seen.to_vec();
            self.version += 1;
        }
    }

    /// The door codes seen this game. The panel goes through
    /// [`Guidance::codes_at`], which knows about level 6 (#66).
    #[cfg(test)]
    pub fn door_codes(&self) -> &[DoorCode] {
        &self.door_codes
    }

    /// The codes the panel shows at `level`: the ones you have been shown,
    /// or at level 6 every one there is, once they have been read (#66).
    pub fn codes_at(&self, level: u8) -> (&[SeenTeleporter], &[DoorCode]) {
        if level >= 6 && !self.all_teleporters.is_empty() {
            (&self.all_teleporters, &self.all_door_codes)
        } else {
            (&self.teleporters, &self.door_codes)
        }
    }

    /// Takes every code there is, read on a copy of the machine once a game
    /// has started (#66).
    pub fn set_all_codes(&mut self, teleporters: &[SeenTeleporter], doors: &[DoorCode]) {
        if self.all_teleporters != teleporters || self.all_door_codes != doors {
            self.all_teleporters = teleporters.to_vec();
            self.all_door_codes = doors.to_vec();
            self.version += 1;
        }
    }

    /// A new game's codes are not this game's: forgotten until read again.
    /// Returns which game the reading that follows is for.
    pub fn forget_all_codes(&mut self) -> u64 {
        self.game = self.game.wrapping_add(1);
        if !self.all_teleporters.is_empty() || !self.all_door_codes.is_empty() {
            self.all_teleporters.clear();
            self.all_door_codes.clear();
            self.version += 1;
        }
        self.game
    }

    /// The game the codes are being read for.
    pub fn game(&self) -> u64 {
        self.game
    }

    /// The game's font, 96 letters of eight bytes from the space, once read.
    pub fn font(&self) -> Option<&[u8]> {
        (!self.font.is_empty()).then_some(&self.font[..])
    }

    /// Takes the game's font, if it has changed.
    pub fn set_font(&mut self, font: &[u8]) {
        if self.font != font {
            self.font = font.to_vec();
            self.version += 1;
        }
    }

    /// Takes the game's list of door codes seen, if it has changed.
    pub fn set_door_codes(&mut self, seen: &[DoorCode]) {
        if self.door_codes != seen {
            self.door_codes = seen.to_vec();
            self.version += 1;
        }
    }

    /// Every room's openings, by room number; empty until they are read.
    pub fn openings(&self) -> &[Openings] {
        &self.openings
    }

    /// Takes every room's openings, read once.
    pub fn set_openings(&mut self, openings: Vec<Openings>) {
        self.openings = openings;
        self.version += 1;
    }

    /// Whether `room` has been visited.
    pub fn visited(&self, room: u16) -> bool {
        self.visited
            .get(usize::from(room))
            .copied()
            .unwrap_or(false)
    }

    /// How many rooms have been visited.
    #[cfg(test)]
    pub fn explored(&self) -> usize {
        self.visited.iter().filter(|&&v| v).count()
    }

    /// The room Blob is in, while a game is being played.
    pub fn room(&self) -> Option<u16> {
        self.room
    }

    /// Takes the room Blob is in, or `None` outside a game, if it has
    /// changed. The number the game keeps after its end (512) is no room.
    pub fn set_room(&mut self, room: Option<u16>) {
        let room = room.filter(|&r| usize::from(r) < ROOMS);
        if self.room != room {
            self.room = room;
            self.version += 1;
        }
    }

    /// Takes the game's set of rooms not yet visited, if it has changed.
    pub fn set_unvisited(&mut self, unvisited: &RoomSet) {
        let same = self.visited.len() == ROOMS
            && (0..ROOMS).all(|r| self.visited[r] != unvisited.contains(r as u16));
        if !same {
            self.visited = (0..ROOMS).map(|r| !unvisited.contains(r as u16)).collect();
            self.version += 1;
        }
    }

    /// Whether `room` holds a core piece still needed.
    pub fn piece(&self, room: u16) -> bool {
        self.pieces.contains(room)
    }

    /// Takes the rooms holding a core piece still needed, if they have
    /// changed.
    /// The items found, each in the room it lies in (#36).
    pub fn items(&self) -> &[Found] {
        &self.items
    }

    /// Takes the items found, if they have changed.
    pub fn set_items(&mut self, found: &[Found]) {
        if self.items != found {
            self.items = found.to_vec();
            self.version += 1;
        }
    }

    pub fn set_pieces(&mut self, rooms: &RoomSet) {
        if self.pieces != *rooms {
            self.pieces = rooms.clone();
            self.version += 1;
        }
    }

    /// The route to the nearest missing piece, or `None` when none is known.
    pub fn route(&self) -> Option<&[Step]> {
        self.route.as_deref()
    }

    /// The route to the core, while a piece it needs is carried and a way
    /// is known.
    pub fn core_route(&self) -> Option<&[Step]> {
        self.core_route.as_deref()
    }

    /// Takes the route to the core, if it has changed.
    pub fn set_core_route(&mut self, route: Option<Vec<Step>>) {
        if self.core_route != route {
            self.core_route = route;
            self.version += 1;
        }
    }

    /// The room of the first security door the piece route has to pass,
    /// then the core route's.
    pub fn route_doors(&self) -> [Option<u16>; 2] {
        self.route_doors
    }

    /// Takes the routes' first doors, if they have changed.
    pub fn set_route_doors(&mut self, doors: [Option<u16>; 2]) {
        if self.route_doors != doors {
            self.route_doors = doors;
            self.version += 1;
        }
    }

    /// The high-score table kept between runs, with each entry's guidance.
    pub fn high_scores(&self) -> Option<&super::scores::Kept> {
        self.high_scores.as_ref()
    }

    /// Takes the kept high-score table, if it has changed.
    pub fn set_high_scores(&mut self, kept: super::scores::Kept, this_game: Option<usize>) {
        let this_game = this_game.and_then(|i| u8::try_from(i).ok());
        if self.high_scores != Some(kept) || self.this_game != this_game {
            self.high_scores = Some(kept);
            self.this_game = this_game;
            self.version += 1;
        }
    }

    /// Which entry of the table this game put in, if any (#47).
    pub fn this_game(&self) -> Option<usize> {
        self.this_game.map(usize::from)
    }

    /// Asks for the piece route to switch to the next of the nearest missing
    /// pieces (#51), which the tracker takes on its next frame.
    pub fn switch_piece(&mut self) {
        self.switch_piece = true;
    }

    /// Whether a switch was asked for since the last call.
    pub fn take_switch(&mut self) -> bool {
        std::mem::take(&mut self.switch_piece)
    }

    /// The room of the piece the route was switched to, if any.
    pub fn chosen_piece(&self) -> Option<u16> {
        self.chosen_piece
    }

    /// Which of how many nearest missing pieces the route leads to, 1-based.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the panel does not say which; the tracker keeps it (@starquake, 2026-09-16)"
        )
    )]
    pub fn piece_choice(&self) -> (u8, u8) {
        self.piece_choice
    }

    /// Takes the piece chosen, and which of how many, if they have changed.
    pub fn set_piece_choice(&mut self, chosen: Option<u16>, which: (u8, u8)) {
        if (self.chosen_piece, self.piece_choice) != (chosen, which) {
            self.chosen_piece = chosen;
            self.piece_choice = which;
            self.version += 1;
        }
    }

    /// Takes the route to the nearest missing piece, if it has changed.
    pub fn set_route(&mut self, route: Option<Vec<Step>>) {
        if self.route != route {
            self.route = route;
            self.version += 1;
        }
    }

    /// The core's nine holes, or none outside a game.
    pub fn core(&self) -> &[Hole] {
        &self.core
    }

    /// Takes the core's holes, if they have changed.
    pub fn set_core(&mut self, holes: Vec<Hole>) {
        if self.core != holes {
            self.core = holes;
            self.version += 1;
        }
    }

    /// Forgets the map, pieces and core of the game that has ended, once
    /// its game-over screens are done: the title screen shows none.
    pub fn forget_map(&mut self) {
        let empty = RoomSet::default();
        if !self.visited.is_empty()
            || self.room.is_some()
            || self.pieces != empty
            || !self.core.is_empty()
        {
            self.visited.clear();
            self.room = None;
            self.pieces = empty;
            self.core.clear();
            self.route = None;
            self.core_route = None;
            self.route_doors = [None; 2];
            self.version += 1;
        }
    }

    /// The rows the picker shows: ending a game only while one is played.
    pub fn rows(&self) -> Vec<Setting> {
        let mut rows = vec![Setting::Level];
        rows.extend(SWITCHES.map(|(row, _, _)| row));
        if self.playing {
            rows.push(Setting::EndGame);
        }
        rows.push(Setting::Exit);
        rows
    }

    /// Whether a game is being played, as the game thread sees it.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
        if !playing && self.focus == Setting::EndGame {
            self.focus = Setting::Exit;
        }
        if self.armed == Some(Setting::EndGame) {
            self.armed = None;
        }
        self.version += 1;
    }

    /// Opens the picker on its top row.
    pub fn open(&mut self) {
        self.picker = true;
        self.picked = (self.level, self.training);
        self.asking = false;
        self.focus = Setting::Level;
        self.armed = None;
        self.version += 1;
    }

    /// Esc, B or Select. With the question up, back to the picker.
    /// Otherwise close it, leaving what is in effect as it was.
    pub fn back(&mut self) {
        if self.asking {
            self.asking = false;
            self.version += 1;
        } else {
            self.close();
        }
    }

    /// Leaves the picker, asking first if that would add to the record,
    /// with Undo highlighted so a reflex press changes nothing.
    fn leave(&mut self) {
        if self.raises_record() {
            self.asking = true;
            self.armed = None;
            self.version += 1;
        } else {
            self.keep();
        }
    }

    /// Puts what is chosen in the picker into effect, and closes it.
    fn keep(&mut self) {
        (self.level, self.training) = self.picked;
        self.close();
    }

    /// Closes the picker, dropping anything chosen in it and not kept. The
    /// record takes the settings in effect, so passing through a level on
    /// the way to another does not count as having used it.
    pub fn close(&mut self) {
        self.picker = false;
        self.asking = false;
        self.armed = None;
        self.record.highest = self.record.highest.max(self.level);
        self.record.training = merged(self.record.training, self.training);
        self.version += 1;
    }

    /// Enter or A. With the question up, it takes the highlighted answer.
    /// On a setting it keeps the changes and leaves the picker, asking first
    /// if they would add to the record. On an action the
    /// first press asks for a second, and the second requests the action
    /// and closes the picker.
    pub fn enter(&mut self) {
        // The question takes Enter or A for yes, and nothing else (#64).
        if self.asking {
            self.keep();
            return;
        }
        let action = match self.focus {
            Setting::EndGame => Action::EndGame,
            Setting::Exit => Action::Exit,
            // A setting: keep what is chosen and leave the picker.
            _ => {
                self.leave();
                return;
            }
        };
        if self.armed == Some(self.focus) {
            self.requested = Some(action);
            // An action is not a decision about the settings: what was
            // chosen and not kept is dropped, so an ended game's score note
            // cannot pick it up by accident.
            self.close();
        } else {
            self.armed = Some(self.focus);
            self.version += 1;
        }
    }

    /// Up and down in the picker: which row is highlighted. Moving away
    /// from an action that was pressed once cancels it.
    pub fn focus_up(&mut self) {
        self.move_focus(-1);
    }

    pub fn focus_down(&mut self) {
        self.move_focus(1);
    }

    fn move_focus(&mut self, by: isize) {
        if self.asking {
            return;
        }
        let rows = self.rows();
        let at = rows.iter().position(|&r| r == self.focus).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, rows.len() as isize - 1) as usize;
        self.focus = rows[to];
        self.armed = None;
        self.version += 1;
    }

    /// Takes the confirmed action, if there is one and it is `which`.
    pub fn take(&mut self, which: Action) -> bool {
        if self.requested == Some(which) {
            self.requested = None;
            true
        } else {
            false
        }
    }

    /// Left and right in the picker: the highlighted setting down or up a
    /// step, in the picker only until it is kept.
    pub fn change(&mut self, up: bool) {
        // Nothing to move between while the question is up (#64).
        if self.asking {
            return;
        }
        let max = LEVELS.len() as u8 - 1;
        match (self.focus, up) {
            (Setting::Level, true) => self.picked.0 = (self.picked.0 + 1).min(max),
            (Setting::Level, false) => self.picked.0 = self.picked.0.saturating_sub(1),
            (Setting::EndGame | Setting::Exit, _) => return,
            // A switch: right turns it on, left turns it off (#8).
            (row, on) => match switch(row, &mut self.picked.1) {
                Some(it) => *it = on,
                None => return,
            },
        }
        self.version += 1;
    }

    /// Puts a level into effect and records it, outside the picker: for
    /// tests, and for screenshots taken without a window.
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(LEVELS.len() as u8 - 1);
        self.record.highest = self.record.highest.max(self.level);
        self.version += 1;
    }

    /// Puts training mode into effect or out of it and records it. For
    /// tests, which start from a setting without going through the picker.
    #[cfg(test)]
    pub fn set_training(&mut self, on: Training) {
        self.training = on;
        self.record.training = merged(self.record.training, on);
        self.version += 1;
    }

    /// A new game has started: its record begins with what is in use now.
    pub fn new_game(&mut self) {
        self.record = Record {
            highest: self.level,
            training: self.training,
        };
        self.chosen_piece = None;
        self.switch_piece = false;
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One training switch on, the rest off.
    fn only(row: Setting) -> Training {
        let mut it = Training::default();
        if let Some(b) = switch(row, &mut it) {
            *b = true;
        }
        it
    }

    #[test]
    fn starts_with_no_help() {
        let g = Guidance::default();
        assert_eq!(g.level(), 0);
        assert_eq!(g.training(), Training::default());
        assert_eq!(g.record(), Record::default());
        assert!(!g.picker_open());
    }

    #[test]
    fn the_record_only_rises() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.set_level(1);
        assert_eq!(g.record().highest, 3);
        g.set_training(only(Setting::Time));
        g.set_training(Training::default());
        assert!(g.record().training.time);
    }

    #[test]
    fn changes_take_effect_only_when_kept() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.open();
        g.change(false);
        g.change(false);
        g.focus_down();
        g.change(true);
        assert_eq!(g.picked(), (1, only(Setting::Time)), "chosen in the picker");
        assert_eq!(
            (g.level(), g.training()),
            (3, Training::default()),
            "not in effect yet"
        );
        g.enter();
        assert!(g.asking(), "training mode would show");
        g.change(false);
        g.enter();
        assert_eq!((g.level(), g.training()), (1, only(Setting::Time)), "kept");
    }

    #[test]
    fn only_what_is_kept_is_recorded() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..5 {
            g.change(true);
        }
        assert_eq!(g.record().highest, 0, "not while the picker is open");
        for _ in 0..4 {
            g.change(false);
        }
        g.focus_down();
        g.change(true);
        g.change(false);
        g.enter();
        g.change(false);
        g.enter();
        assert_eq!(
            g.record(),
            Record {
                highest: 1,
                training: Training::default()
            }
        );
    }

    #[test]
    fn lowering_or_browsing_never_asks() {
        let mut g = Guidance::default();
        g.set_level(3);
        g.open();
        g.change(false);
        g.enter();
        assert!(!g.picker_open(), "a lower level: no question");
        assert_eq!(g.level(), 2);

        g.open();
        g.change(true);
        g.change(true);
        g.change(false);
        g.enter();
        assert!(!g.picker_open(), "back to level 3, already recorded");
    }

    #[test]
    fn raising_asks_with_undo_highlighted() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.change(true);
        g.enter();
        assert!(g.picker_open());
        assert!(g.asking());
        g.back();
        assert!(g.picker_open(), "cancelling stays in the picker");
        g.back();
        assert!(!g.picker_open());
        assert_eq!(g.level(), 0, "not kept");
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn use_it_keeps_and_records() {
        let mut g = Guidance::default();
        g.open();
        g.focus_down();
        g.change(true);
        g.enter();
        assert!(g.asking(), "Enter on a setting asks too");
        g.change(false);
        g.focus_down();
        assert!(g.asking(), "and nothing moves while it asks");
        g.enter();
        assert_eq!(g.training(), only(Setting::Time));
        assert!(g.record().training.time);
    }

    #[test]
    fn back_from_the_question_returns_to_the_picker() {
        let mut g = Guidance::default();
        g.open();
        g.change(true);
        g.enter();
        g.back();
        assert!(g.picker_open());
        assert!(!g.asking());
        assert_eq!(g.picked().0, 1, "still chosen");
        assert_eq!(g.level(), 0, "and not in effect");
    }

    #[test]
    fn closing_leaves_what_is_in_effect_as_it_was() {
        let mut g = Guidance::default();
        g.set_level(2);
        g.open();
        g.change(true);
        g.focus_down();
        g.change(true);
        g.back();
        assert!(!g.picker_open(), "no question on the way out");
        assert_eq!(
            (g.level(), g.training()),
            (2, Training::default()),
            "put back"
        );
        assert_eq!(
            g.record(),
            Record {
                highest: 2,
                training: Training::default()
            }
        );
        g.open();
        g.change(false);
        g.back();
        assert_eq!(g.level(), 2, "a lower level is put back too");
    }

    #[test]
    fn ending_a_game_drops_unconfirmed_raises() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        g.change(true);
        for _ in 0..5 {
            g.focus_down();
        }
        g.enter();
        g.enter();
        assert!(g.take(Action::EndGame));
        assert_eq!(g.level(), 0);
        assert_eq!(g.record(), Record::default());
    }

    #[test]
    fn levels_stop_at_the_ends() {
        let mut g = Guidance::default();
        g.open();
        g.change(false);
        assert_eq!(g.picked().0, 0);
        for _ in 0..10 {
            g.change(true);
        }
        assert_eq!(g.picked().0, LEVELS.len() as u8 - 1);
    }

    #[test]
    fn an_action_needs_two_presses() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        for _ in 0..5 {
            g.focus_down();
        }
        assert_eq!(g.focus(), Setting::EndGame);
        g.enter();
        assert_eq!(g.armed(), Some(Setting::EndGame));
        assert!(!g.take(Action::EndGame), "one press does nothing yet");
        g.enter();
        assert!(g.take(Action::EndGame));
        assert!(!g.picker_open(), "the picker closes");
    }

    #[test]
    fn moving_away_cancels_a_first_press() {
        let mut g = Guidance::default();
        g.open();
        for _ in 0..5 {
            g.focus_down();
        }
        assert_eq!(g.focus(), Setting::Exit);
        g.enter();
        g.focus_up();
        g.focus_down();
        g.enter();
        assert!(!g.take(Action::Exit), "the first press was cancelled");
    }

    #[test]
    fn the_picker_opens_on_its_top_row() {
        let mut g = Guidance::default();
        g.set_playing(true);
        g.open();
        for _ in 0..5 {
            g.focus_down();
        }
        g.close();
        g.open();
        assert_eq!(g.focus(), Setting::Level);
    }

    #[test]
    fn ending_a_game_is_offered_only_while_playing() {
        let mut g = Guidance::default();
        assert_eq!(
            g.rows(),
            [
                Setting::Level,
                Setting::Time,
                Setting::Full,
                Setting::Lives,
                Setting::Unharmed,
                Setting::Exit
            ]
        );
        g.set_playing(true);
        assert_eq!(
            g.rows(),
            [
                Setting::Level,
                Setting::Time,
                Setting::Full,
                Setting::Lives,
                Setting::Unharmed,
                Setting::EndGame,
                Setting::Exit
            ]
        );
    }

    #[test]
    fn teleporters_are_taken_only_when_they_change() {
        let mut g = Guidance::default();
        let before = g.version();
        g.set_teleporters(&[]);
        assert_eq!(g.version(), before, "nothing new, nothing to redraw");
        let seen = SeenTeleporter {
            room: 40,
            code: *b"ABCDE",
        };
        g.set_teleporters(&[seen]);
        assert_eq!(g.teleporters(), [seen]);
        assert!(g.version() > before);
    }

    #[test]
    fn the_map_is_taken_only_when_it_changes() {
        let mut g = Guidance::default();
        assert_eq!(g.explored(), 0, "nothing explored before a game");
        let mut unvisited = RoomSet([0xFF; 64]);
        unvisited.set(97, false);
        unvisited.set(98, false);
        g.set_unvisited(&unvisited);
        g.set_room(Some(98));
        assert_eq!(g.explored(), 2);
        assert!(g.visited(97) && !g.visited(96));
        assert_eq!(g.room(), Some(98));

        let before = g.version();
        g.set_unvisited(&unvisited);
        g.set_room(Some(98));
        assert_eq!(g.version(), before, "nothing new, nothing to redraw");

        let mut pieces = RoomSet::default();
        pieces.set(300, true);
        g.set_pieces(&pieces);
        assert!(g.piece(300) && !g.piece(98));
        let after = g.version();
        g.set_pieces(&pieces);
        assert_eq!(g.version(), after, "the same pieces, nothing to redraw");

        g.set_room(Some(512));
        assert_eq!(
            g.room(),
            None,
            "512 is where the game leaves it, not a room"
        );
        let step = Step {
            room: 99,
            teleport: false,
        };
        g.set_route(Some(vec![step]));
        g.set_core_route(Some(vec![step]));
        assert_eq!(g.core_route(), Some(&[step][..]));
        let routed = g.version();
        g.set_core_route(Some(vec![step]));
        assert_eq!(
            g.version(),
            routed,
            "the same core route, nothing to redraw"
        );
        g.forget_map();
        assert_eq!(g.explored(), 0, "the title screen shows no map");
        assert!(!g.piece(300), "nor any pieces");
        assert!(
            g.route().is_none() && g.core_route().is_none(),
            "nor either route"
        );
        let forgotten = g.version();
        g.forget_map();
        assert_eq!(g.version(), forgotten, "nothing left to forget");
    }

    #[test]
    fn a_new_game_starts_its_record_from_what_is_in_use() {
        let mut g = Guidance::default();
        g.set_level(4);
        g.set_training(only(Setting::Lives));
        g.set_level(2);
        g.set_training(Training::default());
        g.new_game();
        assert_eq!(g.level(), 2, "the chosen level is kept");
        assert_eq!(
            g.record(),
            Record {
                highest: 2,
                training: Training::default()
            }
        );
    }
}
