//! The planet as a grid of rooms, for the guidance map: its size, which
//! edges of a room lead on, the walls inside a room, and a set of rooms.
//!
//! What is read here is a room's cells as the game drew them (see
//! `starquake::read_room`, which has the game build each room); the
//! reading itself is ours: which edges Blob fits through, and a flood fill of
//! the places he fits to find where a room is divided inside.

/// The map's width and height, in rooms.
pub const COLS: u16 = 16;
pub const ROWS: u16 = 32;

/// The first and last character rows a room occupies on screen, and its
/// last column.
const FIRST_ROW: u8 = 6;
const LAST_ROW: u8 = 23;
const LAST_COL: u8 = 31;

/// Where Blob's top-left cell can be: he is two cells by two, so one row and
/// one column short of the room.
const SPOTS_DOWN: usize = (LAST_ROW - FIRST_ROW) as usize;
const SPOTS_ACROSS: usize = LAST_COL as usize;

/// The length of each edge in cells, clockwise from the top-left corner:
/// top, right, bottom, left. A place on the edge is measured in the same
/// cells, from 0 up to [`AROUND`].
const TOP: u8 = LAST_COL + 1;
const SIDE: u8 = LAST_ROW - FIRST_ROW + 1;
pub const AROUND: u8 = 2 * (TOP + SIDE);

/// Which edges of a room have an opening, and the walls inside it between
/// openings that do not reach each other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Openings {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Walls inside the room, where they stand: the solid cells between
    /// openings that do not reach each other.
    pub divides: Divides,
}

/// The solid cells that keep two of a room's openings apart, a bit per
/// cell (bit `col` of `cells[row]`, rows from the top of the play area), and
/// which of them are a door's or a pad's: the two parts they keep apart
/// become one when doors and pads are open (#43).
///
/// They are found by growing every part that has an opening outwards
/// through the room, free cells and solid alike, one cell a step: a solid
/// cell reached from two different parts at the same time, or next to a
/// cell reached from another part, lies between them. A part with no
/// opening of its own, a sealed pocket, grows nothing and draws nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Divides {
    pub cells: [u32; 18],
    pub doors: [u32; 18],
}

impl Divides {
    /// Whether the cell at (`row`, `col`) of the play area is a wall.
    #[must_use]
    pub fn wall(&self, row: usize, col: usize) -> bool {
        row < 18 && col < 32 && self.cells[row] & (1 << col) != 0
    }

    /// Whether the cell at (`row`, `col`) is a door's or a pad's.
    #[must_use]
    pub fn door(&self, row: usize, col: usize) -> bool {
        row < 18 && col < 32 && self.doors[row] & (1 << col) != 0
    }

    /// Finds the walls of `room` between the parts in `ported`, those with
    /// an opening.
    fn find(room: &Room, ported: &[u8]) -> Divides {
        // Every cell Blob stands on in a ported part is a source; each cell
        // then remembers the nearest part, by breadth-first growth.
        let mut nearest = [[0u8; 32]; 18];
        let mut queue = std::collections::VecDeque::new();
        for r in 0..SPOTS_DOWN {
            for c in 0..SPOTS_ACROSS {
                let p = room.shut.at(FIRST_ROW + r as u8, c as u8);
                if p == 0 || !ported.contains(&p) {
                    continue;
                }
                for (rr, cc) in [(r, c), (r, c + 1), (r + 1, c), (r + 1, c + 1)] {
                    if nearest[rr][cc] == 0 {
                        nearest[rr][cc] = p;
                        queue.push_back((rr, cc));
                    }
                }
            }
        }
        // A cell on the seam, reached by two parts in the same step, is
        // recorded as a wall as it is claimed.
        let mut cells = [0u32; 18];
        let mut doors = [0u32; 18];
        let open_of = |p: u8| {
            (0..SPOTS_DOWN)
                .flat_map(|r| (0..SPOTS_ACROSS).map(move |c| (r, c)))
                .find(|&(r, c)| room.shut.at(FIRST_ROW + r as u8, c as u8) == p)
                .map(|(r, c)| room.open.at(FIRST_ROW + r as u8, c as u8))
                .unwrap_or(0)
        };
        let mut mark = |r: usize, c: usize, a: u8, b: u8| {
            if room.solid[r] & (1 << c) == 0 {
                return;
            }
            cells[r] |= 1 << c;
            if open_of(a) == open_of(b) {
                doors[r] |= 1 << c;
            }
        };
        while let Some((r, c)) = queue.pop_front() {
            let here = nearest[r][c];
            let beside = [
                (r, c + 1),
                (r + 1, c),
                (r, c.wrapping_sub(1)),
                (r.wrapping_sub(1), c),
            ];
            for (rr, cc) in beside {
                if rr >= 18 || cc >= 32 {
                    continue;
                }
                let there = nearest[rr][cc];
                if there == 0 {
                    nearest[rr][cc] = here;
                    queue.push_back((rr, cc));
                } else if there != here {
                    mark(r, c, here, there);
                    mark(rr, cc, here, there);
                }
            }
        }
        Divides { cells, doors }
    }
}

/// Whether a cell's attribute lets Blob through: below `0x40` (not bright) is
/// solid, as the game's collision test has it.
pub fn free(attr: u8) -> bool {
    attr >= 0x40
}

/// The parts of a room Blob can move between, ignoring gravity: every place
/// his top-left cell fits is numbered by the part it is in, and 0 where he
/// does not fit. Two places in the same part are joined by free cells, so a
/// solid wall between them gives them different numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parts([[u8; SPOTS_ACROSS]; SPOTS_DOWN]);

impl Parts {
    /// Numbers the parts of a room from `free(row, col)`.
    fn find(free: impl Fn(u8, u8) -> bool) -> Parts {
        let fits = |r: usize, c: usize| {
            let (row, col) = (FIRST_ROW + r as u8, c as u8);
            free(row, col) && free(row, col + 1) && free(row + 1, col) && free(row + 1, col + 1)
        };
        let mut parts = [[0u8; SPOTS_ACROSS]; SPOTS_DOWN];
        let mut next = 0;
        for r in 0..SPOTS_DOWN {
            for c in 0..SPOTS_ACROSS {
                if parts[r][c] != 0 || !fits(r, c) {
                    continue;
                }
                next += 1;
                parts[r][c] = next;
                let mut todo = vec![(r, c)];
                while let Some((r, c)) = todo.pop() {
                    let beside = [
                        (r, c + 1),
                        (r + 1, c),
                        (r, c.wrapping_sub(1)),
                        (r.wrapping_sub(1), c),
                    ];
                    for (r, c) in beside {
                        if r < SPOTS_DOWN && c < SPOTS_ACROSS && parts[r][c] == 0 && fits(r, c) {
                            parts[r][c] = next;
                            todo.push((r, c));
                        }
                    }
                }
            }
        }
        Parts(parts)
    }

    /// Joins the parts on either side of a tile `width` cells wide and
    /// three tall from its cell at screen (`row`, `col`): what a door does
    /// when it opens, moving Blob 48 pixels past the tile rather than
    /// through its cells, and what a teleporter pad does when it is blanked.
    /// So on each side the nearest place Blob fits within six cells counts,
    /// and whatever lies between two doors' tiles, the second tile included,
    /// does not keep the sides apart.
    fn join_across(&mut self, row: u8, col: u8, width: u8) {
        const REACH: u8 = 6;
        let rows = row.saturating_sub(1)..row + 3;
        // Blob's top-left cell beside the tile: two cells to its left, so
        // his right cell touches it, or just past its right side; then
        // further out.
        let left = (0..REACH).map(|d| col.wrapping_sub(2 + d));
        let right = (0..REACH).map(|d| col + width + d);
        let mut parts: Vec<u8> = Vec::new();
        for side in [left.collect::<Vec<u8>>(), right.collect()] {
            let nearest = side
                .into_iter()
                .find_map(|c| rows.clone().map(|r| self.at(r, c)).find(|&p| p != 0));
            if let Some(p) = nearest
                && !parts.contains(&p)
            {
                parts.push(p);
            }
        }
        if let Some((&first, rest)) = parts.split_first() {
            for line in &mut self.0 {
                for cell in line.iter_mut() {
                    if rest.contains(cell) {
                        *cell = first;
                    }
                }
            }
        }
    }

    /// The part of the room Blob is in with his top-left cell at screen
    /// (`row`, `col`), or 0 where he does not fit.
    pub fn at(&self, row: u8, col: u8) -> u8 {
        let r = row.wrapping_sub(FIRST_ROW) as usize;
        self.0
            .get(r)
            .and_then(|cols| cols.get(col as usize))
            .copied()
            .unwrap_or(0)
    }
}

/// One room as the map reads it.
#[derive(Clone)]
pub struct Room {
    pub openings: Openings,
    /// Its parts with security doors shut, and with them open.
    pub shut: Parts,
    pub open: Parts,
    /// The cell of its wall passage marker, if it has one.
    pub passage: Option<(u8, u8)>,
    /// Which way its passage leads: right when Blob can stand beside the
    /// tile on its left and walk into it, left when he can on its right. On
    /// the tape every passage has one such side (#10).
    pub passage_right: bool,
    pub passage_left: bool,
    /// Its solid cells, a bit per cell: bit `col` of `solid[row]`, rows from
    /// the top of the play area.
    pub solid: [u32; 18],
    /// Its lift cells, the same way: a lift only ever carries Blob up.
    pub lift: [u32; 18],
    /// The part (doors shut) Blob can reach its wall passage from, and the
    /// part its teleporter booth is in.
    pub passage_part: u8,
    pub booth_part: u8,
}

/// The character cell of a marker's position (the way the game's tiles place them).
pub fn marker_cell(x: u8, y: u8) -> (u8, u8) {
    (0x18 - ((y as u16 + 1) >> 3) as u8, x >> 3)
}

/// The marker a wall passage's tile leaves: touching it while walking left or
/// right takes Blob into the room beside, to that room's own passage marker.
const PASSAGE: u8 = 0x0F;

/// The marker a security door's tile leaves.
const DOOR: u8 = 0;

/// The attributes of a lift's cells, bright with green paper and used for
/// nothing else on the planet: standing in one carries Blob up (#10).
pub const LIFT_ATTRS: [u8; 2] = [0x60, 0x64];

/// The marker a teleporter booth's tile leaves.
const BOOTH: u8 = 0x0D;

/// The marker a teleporter pad's tile leaves, one on each side of the pad's
/// own column, which stands one cell wide and three tall in a gap and is
/// blanked, once, when Blob touches it carrying item `0x10`. Found on
/// 2026-09-15 by walking Blob into the pads on the player's tape (#40).
const PAD: u8 = 0x0B;

impl Room {
    /// Reads a room from its attribute cells as drawn, `attr(row, col)`, and
    /// the markers its tiles left, as (x, y, kind).
    pub fn read(attr: impl Fn(u8, u8) -> u8, markers: &[(u8, u8, u8)]) -> Room {
        let cell = |&(x, y, _): &(u8, u8, u8)| marker_cell(x, y);
        let passage = markers.iter().find(|m| m.2 == PASSAGE).map(cell);
        // A door's tile is four cells by three, from its marker's cell; open,
        // it joins the parts on either side of it. A pad's column is one
        // cell wide, at the pair's cell rounded down to four plus one, as
        // the game blanks it.
        let shut = Parts::find(|row, col| free(attr(row, col)));
        let mut open = shut.clone();
        for &(x, y, kind) in markers {
            let (r, c) = marker_cell(x, y);
            match kind {
                DOOR => open.join_across(r, c, 4),
                PAD => open.join_across(r, (c & !3) | 1, 1),
                _ => {}
            }
        }
        // Where Blob can stand beside the passage's tile, four cells wide and
        // three tall: with his top-left cell two columns left of it, or just
        // past its right side, on a row that overlaps it. Walking into it
        // from there takes him through to the room on that side.
        let beside = |col: i16| {
            passage.is_some_and(|(row, _)| {
                (0..SPOTS_ACROSS as i16).contains(&col)
                    && (row.saturating_sub(1)..=row + 1).any(|r| shut.at(r, col as u8) != 0)
            })
        };
        let at = passage.map_or(0, |(_, col)| i16::from(col));
        let (passage_right, passage_left) = (beside(at - 2), beside(at + 4));
        // The part on the side Blob walks into the passage from, where he
        // also arrives through it from the room beside.
        let passage_part = passage
            .map(|(row, _)| {
                let side = if passage_right { at - 2 } else { at + 4 };
                (row.saturating_sub(1)..=row + 1)
                    .map(|r| shut.at(r, side.clamp(0, 30) as u8))
                    .find(|&p| p != 0)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        // The part a tile's marker stands in: Blob on it, his top cell on the
        // marker's row or up to two above, at its column or beside.
        let part_at_marker = |(row, col): (u8, u8)| {
            (0..=2u8)
                .flat_map(|up| [0, 1, -1i16].map(move |d| (row.saturating_sub(up), d)))
                .map(|(r, d)| shut.at(r, (i16::from(col) + d).clamp(0, 30) as u8))
                .find(|&p| p != 0)
                .unwrap_or(0)
        };
        let mut solid = [0u32; 18];
        let mut lift = [0u32; 18];
        for r in 0..18 {
            for col in 0..32u8 {
                let a = attr(FIRST_ROW + r as u8, col);
                if !free(a) {
                    solid[r] |= 1 << col;
                }
                if LIFT_ATTRS.contains(&a) {
                    lift[r] |= 1 << col;
                }
            }
        }
        let booth_part = markers
            .iter()
            .find(|m| m.2 == BOOTH)
            .map_or(0, |m| part_at_marker(cell(m)));
        Room {
            openings: scan(|row, col| free(attr(row, col))),
            shut,
            open,
            passage,
            passage_right,
            passage_left,
            solid,
            lift,
            passage_part,
            booth_part,
        }
    }
}

/// The openings of every room, from each room read in number order.
///
/// An edge is open where there is a gap in it, or a passage through the
/// wall: two rooms side by side whose passages lead to each other, the left
/// one's walked into going right and the right one's going left (#10). The core
/// room is not played in its tiles (walking in from the left runs its own
/// screen, which puts Blob back in the room he came from), so its one way in
/// and out is its left edge.
#[must_use]
pub fn openings(rooms: &[Room], core_room: u16) -> Vec<Openings> {
    let leads = |room: usize, right: bool| {
        rooms.get(room).is_some_and(|r| {
            if right {
                r.passage_right
            } else {
                r.passage_left
            }
        })
    };
    let mut openings = Vec::with_capacity(rooms.len());
    for (i, room) in rooms.iter().enumerate() {
        let col = i as u16 % COLS;
        let right = col != COLS - 1 && leads(i, true) && leads(i + 1, false);
        let left = col != 0 && leads(i, false) && leads(i - 1, true);
        let mut o = room.openings;
        o.right |= right;
        o.left |= left;
        let ported: Vec<u8> = ports(room, left, right).iter().map(|p| p.shut).collect();
        o.divides = Divides::find(room, &ported);
        openings.push(o);
    }
    if let Some(core) = openings.get_mut(usize::from(core_room)) {
        *core = Openings {
            left: true,
            ..Openings::default()
        };
    }
    openings
}

/// Reads the four edges of a room from `free(row, col)`.
///
/// Blob is two cells by two. The game sends him through the left edge
/// from columns 0–1, the right from 30–31, the top once he rises past the
/// top two rows and the bottom from the bottom two. So an edge is open where
/// a two-by-two window of free cells touches it.
fn scan(free: impl Fn(u8, u8) -> bool) -> Openings {
    let window = |row: u8, col: u8| {
        free(row, col) && free(row, col + 1) && free(row + 1, col) && free(row + 1, col + 1)
    };
    Openings {
        left: (FIRST_ROW..LAST_ROW).any(|row| window(row, 0)),
        right: (FIRST_ROW..LAST_ROW).any(|row| window(row, LAST_COL - 1)),
        up: (0..LAST_COL).any(|col| window(FIRST_ROW, col)),
        down: (0..LAST_COL).any(|col| window(LAST_ROW - 1, col)),
        divides: Divides::default(),
    }
}

/// A stretch of a room's edge Blob can leave through, from `start` to `end`
/// clockwise, and the part of the room it belongs to with doors shut and
/// with them open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Port {
    start: u8,
    end: u8,
    shut: u8,
    open: u8,
}

/// The stretches of a room's edge Blob can leave through, in clockwise
/// order: every place he fits against an edge, joined where they touch and
/// belong to the same part, and the wall passages on the sides given.
fn ports(room: &Room, passage_left: bool, passage_right: bool) -> Vec<Port> {
    let last_down = (SPOTS_DOWN - 1) as u8;
    let last_across = (SPOTS_ACROSS - 1) as u8;
    let port = |start: u8, row: u8, col: u8| {
        let (shut, open) = (room.shut.at(row, col), room.open.at(row, col));
        (shut != 0).then_some(Port {
            start,
            end: start + 2,
            shut,
            open,
        })
    };
    let mut pieces: Vec<Port> = Vec::new();
    for c in 0..=last_across {
        pieces.extend(port(c, FIRST_ROW, c));
    }
    for r in 0..=last_down {
        pieces.extend(port(TOP + r, FIRST_ROW + r, last_across));
    }
    for c in (0..=last_across).rev() {
        pieces.extend(port(
            TOP + SIDE + (last_across - c),
            FIRST_ROW + last_down,
            c,
        ));
    }
    for r in (0..=last_down).rev() {
        pieces.extend(port(2 * TOP + SIDE + (last_down - r), FIRST_ROW + r, 0));
    }
    if let Some((row, col)) = room.passage {
        // The part the passage is in: the first place Blob fits next to its
        // tile, which is four cells by three.
        let near = (row.saturating_sub(1)..row + 3)
            .flat_map(|r| (col.saturating_sub(2)..col + 4).map(move |c| (r, c)))
            .find(|&(r, c)| room.shut.at(r, c) != 0);
        if let Some((r, c)) = near {
            let at = row.saturating_sub(FIRST_ROW).min(SIDE - 3);
            let (shut, open) = (room.shut.at(r, c), room.open.at(r, c));
            if passage_right {
                pieces.push(Port {
                    start: TOP + at,
                    end: TOP + at + 3,
                    shut,
                    open,
                });
            }
            if passage_left {
                let start = 2 * TOP + SIDE + (SIDE - 3 - at);
                pieces.push(Port {
                    start,
                    end: start + 3,
                    shut,
                    open,
                });
            }
        }
    }
    pieces.sort_by_key(|p| p.start);
    let mut joined: Vec<Port> = Vec::new();
    for p in pieces {
        match joined.last_mut() {
            Some(last) if last.shut == p.shut && p.start <= last.end => {
                last.end = last.end.max(p.end);
            }
            _ => joined.push(p),
        }
    }
    joined
}

/// A set of the 512 rooms, a bit each, most significant bit first: the way
/// Starquake keeps its own sets of rooms in memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomSet(pub [u8; 64]);

impl Default for RoomSet {
    /// No rooms.
    fn default() -> Self {
        RoomSet([0; 64])
    }
}

impl RoomSet {
    #[must_use]
    pub fn contains(&self, room: u16) -> bool {
        let room = room & 0x1FF;
        self.0[(room >> 3) as usize] & (0x80 >> (room & 7)) != 0
    }

    pub fn set(&mut self, room: u16, on: bool) {
        let room = room & 0x1FF;
        let bit = 0x80 >> (room & 7);
        let byte = &mut self.0[(room >> 3) as usize];
        *byte = if on { *byte | bit } else { *byte & !bit };
    }
}

/// The connections between rooms a player has proven this game (#9), and
/// the way to the nearest target over them.
///
/// Every walked step counts both ways, up and down included: a drop that
/// cannot be climbed can be flown back up on the hover platform (decision 9
/// on #9).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Known {
    /// `from → to` for each proven step.
    steps: std::collections::BTreeSet<(u16, u16)>,
}

/// One step of a route: into `room`, walking or by teleport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub room: u16,
    pub teleport: bool,
}

impl Known {
    /// Records walking from `from` into the neighbouring room `to`. A step
    /// that is not to a neighbour records nothing.
    pub fn walked(&mut self, from: u16, to: u16) {
        if [1, 0xFFFF, 16, 0xFFF0].contains(&to.wrapping_sub(from)) {
            self.steps.insert((from, to));
            self.steps.insert((to, from));
        }
    }

    /// Whether any step is known.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The fewest steps from `start` to a room in `targets`, a teleport
    /// counting as one step: walking over known steps, and from a booth in
    /// `booths` to any other of them. The core room, whose screen a player
    /// walks into from its left and is put back from, counts as reached
    /// from the room to its left once that room is reached. `None` when no
    /// target can be reached; empty when `start` is one.
    #[must_use]
    pub fn route(
        &self,
        start: u16,
        booths: &[u16],
        targets: &RoomSet,
        core_room: u16,
    ) -> Option<Vec<Step>> {
        let rooms = usize::from(COLS * ROWS);
        if usize::from(start) >= rooms {
            return None;
        }
        let mut came: Vec<Option<Step>> = vec![None; rooms];
        let mut seen = vec![false; rooms];
        seen[usize::from(start)] = true;
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(room) = queue.pop_front() {
            if targets.contains(room) {
                let mut path = vec![];
                let mut at = room;
                // Every room reached but the start knows where it came from.
                while let Some(step) = came[usize::from(at)] {
                    path.push(Step {
                        room: at,
                        teleport: step.teleport,
                    });
                    at = step.room;
                }
                path.reverse();
                return Some(path);
            }
            let walks = self
                .steps
                .range((room, 0)..(room, u16::MAX))
                .map(|&(_, to)| (to, false))
                .chain((room + 1 == core_room).then_some((core_room, false)));
            let jumps = booths
                .contains(&room)
                .then(|| {
                    booths
                        .iter()
                        .filter(move |&&b| b != room)
                        .map(|&b| (b, true))
                })
                .into_iter()
                .flatten();
            for (next, teleport) in walks.chain(jumps).collect::<Vec<_>>() {
                if usize::from(next) < rooms && !seen[usize::from(next)] {
                    seen[usize::from(next)] = true;
                    // Remember where each room was reached from.
                    came[usize::from(next)] = Some(Step { room, teleport });
                    queue.push_back(next);
                }
            }
        }
        None
    }
}

/// The part with doors open that holds the part `shut` of a room with doors
/// shut.
fn open_part(room: &Room, shut: u8) -> u8 {
    (0..SPOTS_DOWN)
        .flat_map(|r| (0..SPOTS_ACROSS).map(move |c| (r, c)))
        .map(|(r, c)| (FIRST_ROW + r as u8, c as u8))
        .find(|&(r, c)| room.shut.at(r, c) == shut && shut != 0)
        .map_or(0, |(r, c)| room.open.at(r, c))
}

/// Where Blob can be on the planet: a room, and the part of it he is in,
/// with its doors and teleporter pads open (#10).
pub type Place = (u16, u8);

/// The planet as the map reads it, for level 5 (#10): every place, and the
/// places a step leads to from each. Two rooms join where both have a place
/// for Blob at the same spot on their shared edge, both ways and in every
/// direction, since level 5 assumes Blob can fly everywhere and leaves the
/// logistics to the player (decision 5), except down through a lift, which
/// only ever goes up; and where their wall passages lead to each other.
/// Doors and teleporter pads count as open, and the core room is reached
/// from the room to its left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    ways: std::collections::BTreeMap<Place, Vec<Place>>,
    booths: Vec<Place>,
    /// Every room's parts with doors open, to find where Blob is.
    parts: Vec<Parts>,
}

impl Graph {
    /// Builds the graph from every room, read in number order.
    #[must_use]
    pub fn new(rooms: &[Room], core_room: u16) -> Graph {
        let mut ways = std::collections::BTreeMap::<Place, Vec<Place>>::new();
        let mut join = |a: Place, b: Place, both: bool| {
            if a.1 == 0 || b.1 == 0 {
                return;
            }
            for (from, to) in [(a, b), (b, a)].into_iter().take(if both { 2 } else { 1 }) {
                let list = ways.entry(from).or_default();
                if !list.contains(&to) {
                    list.push(to);
                }
            }
        };
        for (i, r) in rooms.iter().enumerate() {
            let room = i as u16;
            if room == core_room {
                continue;
            }
            if room % COLS + 1 < COLS
                && let Some(o) = rooms.get(i + 1)
            {
                if room + 1 == core_room {
                    for row in FIRST_ROW..LAST_ROW {
                        join((room, r.open.at(row, LAST_COL - 1)), (core_room, 1), false);
                    }
                } else {
                    for row in FIRST_ROW..LAST_ROW {
                        join(
                            (room, r.open.at(row, LAST_COL - 1)),
                            (room + 1, o.open.at(row, 0)),
                            true,
                        );
                    }
                    if r.passage_right && o.passage_left {
                        join(
                            (room, open_part(r, r.passage_part)),
                            (room + 1, open_part(o, o.passage_part)),
                            true,
                        );
                    }
                }
            }
            if let Some(below) = rooms.get(i + usize::from(COLS))
                && room + COLS != core_room
            {
                let lift = |room: &Room, row: usize, c: u8| room.lift[row] & (0b11 << c) != 0;
                for c in 0..LAST_COL {
                    let (up, down) = (
                        (room + COLS, below.open.at(FIRST_ROW, c)),
                        (room, r.open.at(LAST_ROW - 1, c)),
                    );
                    // Up always; down only where no lift stands at the edge.
                    join(up, down, false);
                    if !(lift(r, 16, c) || lift(r, 17, c) || lift(below, 0, c) || lift(below, 1, c))
                    {
                        join(down, up, false);
                    }
                }
            }
        }
        let booths = rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| r.booth_part != 0)
            .map(|(i, r)| (i as u16, open_part(r, r.booth_part)))
            .collect();
        Graph {
            ways,
            booths,
            parts: rooms.iter().map(|r| r.open.clone()).collect(),
        }
    }

    /// The place Blob is in, standing at (`x`, `y`) in `room`: the part
    /// under his top-left cell, or one beside it while he is between cells;
    /// part 0 when none is found.
    #[must_use]
    pub fn place(&self, room: u16, x: u8, y: u8) -> Place {
        (room, self.part_at(room, (0xBF - y.min(0xBF)) >> 3, x >> 3))
    }

    /// The part of `room` at the screen cell (`row`, `col`), such as an
    /// item's spot (#50), or the part beside it when the cell is one Blob
    /// does not fit at; 0 when none is found.
    #[must_use]
    pub fn part_at(&self, room: u16, row: u8, col: u8) -> u8 {
        let Some(parts) = self.parts.get(usize::from(room)) else {
            return 0;
        };
        let (row, col) = (i16::from(row), i16::from(col));
        [
            (0, 0),
            (0, 1),
            (1, 0),
            (1, 1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (1, -1),
        ]
        .into_iter()
        .map(|(dr, dc)| (row + dr, col + dc))
        .filter(|&(r, c)| r >= 0 && c >= 0)
        .map(|(r, c)| parts.at(r as u8, c as u8))
        .find(|&p| p != 0)
        .unwrap_or(0)
    }

    /// The places one step from `place`.
    #[must_use]
    pub fn ways(&self, place: Place) -> &[Place] {
        self.ways.get(&place).map_or(&[], Vec::as_slice)
    }

    /// Every place the graph has a way out of.
    pub fn places(&self) -> impl Iterator<Item = Place> + '_ {
        self.ways.keys().copied()
    }

    /// The fewest steps from `start` to a room in `targets` or a place in
    /// `places` over the whole map (level 5, #10), a teleport counting as
    /// one: the graph's ways, and a jump from a booth in a room in `booths`
    /// to another such booth. `start` with part 0 starts from every place
    /// in its room. `None` when no target can be reached; empty when
    /// `start` is at one.
    #[must_use]
    pub fn route(
        &self,
        start: Place,
        booths: &[u16],
        targets: &RoomSet,
        places: &[Place],
    ) -> Option<Vec<Step>> {
        use std::collections::{BTreeMap, BTreeSet, VecDeque};
        let reached = |p: Place| targets.contains(p.0) || places.contains(&p);
        if targets.contains(start.0) || places.contains(&start) {
            return Some(vec![]);
        }
        let starts: Vec<Place> = if start.1 == 0 {
            self.ways
                .range((start.0, 0)..=(start.0, u8::MAX))
                .map(|(p, _)| *p)
                .collect()
        } else {
            vec![start]
        };
        let seen_booths: Vec<Place> = self
            .booths
            .iter()
            .copied()
            .filter(|b| booths.contains(&b.0))
            .collect();
        let mut came: BTreeMap<Place, (Place, bool)> = BTreeMap::new();
        let mut queue: VecDeque<Place> = starts.iter().copied().collect();
        let mut seen: BTreeSet<Place> = starts.iter().copied().collect();
        while let Some(here) = queue.pop_front() {
            if reached(here) {
                let mut path = vec![];
                let mut at = here;
                while let Some(&(from, teleport)) = came.get(&at) {
                    path.push(Step {
                        room: at.0,
                        teleport,
                    });
                    at = from;
                }
                path.reverse();
                return Some(path);
            }
            let walks = self.ways(here).iter().map(|&to| (to, false));
            let jumps = seen_booths
                .contains(&here)
                .then(|| {
                    seen_booths
                        .iter()
                        .filter(move |&&b| b != here)
                        .map(|&b| (b, true))
                })
                .into_iter()
                .flatten();
            for (to, teleport) in walks.chain(jumps).collect::<Vec<_>>() {
                if seen.insert(to) {
                    came.insert(to, (here, teleport));
                    queue.push_back(to);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room drawn as text, top row first: `#` solid, anything else free.
    fn room(lines: &[&str]) -> impl Fn(u8, u8) -> bool {
        let grid: Vec<Vec<bool>> = lines
            .iter()
            .map(|l| l.chars().map(|c| c != '#').collect())
            .collect();
        move |row, col| grid[(row - FIRST_ROW) as usize][col as usize]
    }

    /// The solid cells of a room drawn as text.
    fn solid_of(lines: &[&str]) -> [u32; 18] {
        let mut solid = [0u32; 18];
        for (r, line) in lines.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                if ch == '#' {
                    solid[r] |= 1 << c;
                }
            }
        }
        solid
    }

    fn walled() -> Vec<String> {
        (0..18)
            .map(|r| {
                if r == 0 || r == 17 {
                    "#".repeat(32)
                } else {
                    format!("#{}#", ".".repeat(30))
                }
            })
            .collect()
    }

    fn targets(rooms: &[u16]) -> RoomSet {
        let mut set = RoomSet::default();
        for &r in rooms {
            set.set(r, true);
        }
        set
    }

    #[test]
    fn a_route_follows_walked_steps_both_ways() {
        let mut k = Known::default();
        // 100 → 101 → 117 (down), and 117 → 118.
        k.walked(100, 101);
        k.walked(101, 117);
        k.walked(117, 118);
        let route = k.route(100, &[], &targets(&[118]), 199).unwrap();
        let rooms: Vec<u16> = route.iter().map(|s| s.room).collect();
        assert_eq!(rooms, [101, 117, 118]);
        // Back again, up the drop too.
        let back = k.route(118, &[], &targets(&[100]), 199).unwrap();
        let rooms: Vec<u16> = back.iter().map(|s| s.room).collect();
        assert_eq!(rooms, [117, 101, 100]);
    }

    #[test]
    fn a_route_takes_the_fewest_steps_and_a_teleport_is_one() {
        let mut k = Known::default();
        for r in 40..47 {
            k.walked(r, r + 1); // a long corridor 40 … 47
        }
        k.walked(300, 301);
        let route = k.route(40, &[40, 300], &targets(&[47, 301]), 199).unwrap();
        assert_eq!(
            route,
            [
                Step {
                    room: 300,
                    teleport: true
                },
                Step {
                    room: 301,
                    teleport: false
                }
            ],
            "two steps by teleport beat seven walking"
        );
    }

    #[test]
    fn standing_on_a_target_is_an_empty_route_and_the_core_is_reached_from_its_left() {
        let k = Known::default();
        assert_eq!(k.route(5, &[], &targets(&[5]), 199), Some(vec![]));
        assert_eq!(k.route(5, &[], &targets(&[6]), 199), None);
        let mut k = Known::default();
        k.walked(197, 198);
        let route = k.route(197, &[], &targets(&[199]), 199).unwrap();
        assert_eq!(route.last().unwrap().room, 199);
    }

    #[test]
    fn a_step_that_is_not_to_a_neighbour_records_nothing() {
        let mut k = Known::default();
        k.walked(10, 40);
        assert!(k.is_empty());
    }

    #[test]
    fn a_closed_room_has_no_openings() {
        let lines = walled();
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(scan(room(&refs)), Openings::default());
    }

    #[test]
    fn a_gap_two_cells_wide_is_an_opening() {
        let mut lines = walled();
        // Two rows free in the left wall, and two columns in the floor.
        for r in [8, 9] {
            lines[r].replace_range(0..1, ".");
        }
        lines[17].replace_range(10..12, "..");
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let o = scan(room(&refs));
        assert!(o.left && o.down);
        assert!(!o.right && !o.up);
    }

    #[test]
    fn a_gap_one_cell_wide_is_not() {
        let mut lines = walled();
        lines[8].replace_range(31..32, ".");
        lines[0].replace_range(5..6, ".");
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        assert_eq!(scan(room(&refs)), Openings::default());
    }

    /// A room open on the left and right, rows 8 and 9 of the room.
    fn open_both_sides() -> Vec<String> {
        let mut lines = walled();
        for r in [8, 9] {
            lines[r].replace_range(0..1, ".");
            lines[r].replace_range(31..32, ".");
        }
        lines
    }

    #[test]
    fn a_room_in_one_part_has_no_walls_inside() {
        let d = divides_of(&open_both_sides());
        assert!(d.cells.iter().all(|&bits| bits == 0));
    }

    /// Room 210: two doors either side of a solid pillar four cells wide,
    /// their own cells free as the game draws them. A door moves Blob 48
    /// pixels past it, so the divide is a door, pillar or not.
    #[test]
    fn two_doors_with_solid_cells_between_them_still_make_a_door() {
        let mut lines = open_both_sides();
        for line in &mut lines {
            line.replace_range(14..18, "####");
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let grid = room(&refs);
        // Doors at cells (13, 12) and (13, 18), as markers (x, y, kind 0):
        // `marker_cell(96, 87)` is (13, 12).
        assert_eq!(marker_cell(96, 87), (13, 12));
        let r = Room::read(
            |row, col| if grid(row, col) { 0x47 } else { 0x07 },
            &[(96, 87, DOOR), (144, 87, DOOR)],
        );
        let d = openings(&[r], 999)[0].divides;
        let drawn = draw_divides(&d);
        assert!(drawn.iter().any(|l| l.contains('D')), "{drawn:?}");
        assert!(
            drawn.iter().all(|l| !l.contains('#')),
            "a door's wall, nothing plain: {drawn:?}"
        );
    }

    /// Room 190: a teleporter pad, one cell wide and three tall, closing the
    /// only gap between the halves; its two markers sit either side of it.
    #[test]
    fn a_teleporter_pad_in_the_only_gap_makes_a_door() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            line.replace_range(15..18, if (7..10).contains(&r) { "#.#" } else { "###" });
            if (7..10).contains(&r) {
                line.replace_range(17..18, "#");
                line.replace_range(15..17, "..");
            }
        }
        // Rows 7 to 9: free up to column 16, the pad's column 17 solid, free
        // from 18; walls above and below. Markers at (13, 16) and (13, 17).
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let grid = room(&refs);
        assert_eq!(marker_cell(128, 87), (13, 16));
        let r = Room::read(
            |row, col| if grid(row, col) { 0x47 } else { 0x07 },
            &[(128, 87, PAD), (136, 87, PAD)],
        );
        let d = openings(&[r], 999)[0].divides;
        let drawn = draw_divides(&d);
        assert!(drawn.iter().any(|l| l.contains('D')), "{drawn:?}");
        assert!(
            drawn.iter().all(|l| !l.contains('#')),
            "a door's wall, nothing plain: {drawn:?}"
        );
    }

    /// A room's divides from its text, with `D` a door's cells (solid while
    /// shut) and the doors' tiles as markers.
    fn divides_of(lines: &[String]) -> Divides {
        let shut_lines: Vec<String> = lines.iter().map(|l| l.replace('D', "#")).collect();
        let shut_refs: Vec<&str> = shut_lines.iter().map(String::as_str).collect();
        let open_lines: Vec<String> = lines.iter().map(|l| l.replace('D', ".")).collect();
        let open_refs: Vec<&str> = open_lines.iter().map(String::as_str).collect();
        let r = Room {
            openings: scan(room(&shut_refs)),
            shut: Parts::find(room(&shut_refs)),
            open: Parts::find(room(&open_refs)),
            passage: None,
            passage_right: false,
            passage_left: false,
            solid: solid_of(&shut_refs),
            lift: [0; 18],
            passage_part: 0,
            booth_part: 0,
        };
        openings(&[r], 999)[0].divides
    }

    /// The wall cells of a divides map as text, `#` a wall, `D` a door's.
    fn draw_divides(d: &Divides) -> Vec<String> {
        (0..18)
            .map(|r| {
                (0..32)
                    .map(|c| {
                        if d.door(r, c) {
                            'D'
                        } else if d.wall(r, c) {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_wall_down_the_middle_stands_where_it_is() {
        let mut lines = open_both_sides();
        for line in &mut lines {
            line.replace_range(15..17, "##");
        }
        let d = divides_of(&lines);
        let drawn = draw_divides(&d);
        // The two solid columns between the halves, and nothing else: not
        // the outer walls, which keep no two openings apart.
        for (r, line) in drawn.iter().enumerate() {
            let expected: String = (0..32)
                .map(|c| if (15..17).contains(&c) { '#' } else { '.' })
                .collect();
            assert_eq!(line, &expected, "row {r}");
        }
        assert!(d.doors.iter().all(|&bits| bits == 0));
    }

    #[test]
    fn a_pocket_with_no_opening_draws_no_wall() {
        // A sealed pocket in the top-right corner, beside a room open both sides.
        let mut lines = open_both_sides();
        for line in &mut lines[1..5] {
            line.replace_range(24..25, "#");
        }
        lines[5].replace_range(24..31, "#######");
        let d = divides_of(&lines);
        assert!(
            d.cells.iter().all(|&bits| bits == 0),
            "{:?}",
            draw_divides(&d)
        );
    }

    #[test]
    fn a_door_s_wall_is_marked_as_a_door_s() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            let cells = if (7..10).contains(&r) { "DD" } else { "##" };
            line.replace_range(15..17, cells);
        }
        let d = divides_of(&lines);
        let drawn = draw_divides(&d);
        assert!(
            drawn.iter().all(|l| l.chars().all(|ch| ch != '#')),
            "{drawn:?}"
        );
        assert!(drawn.iter().any(|l| l.contains('D')), "{drawn:?}");
    }

    /// A room walled all round with a wall passage's tile at screen row 13,
    /// column `col`, and solid cells from `solid_from` to `solid_to` on rows
    /// 12 to 15, so Blob can stand beside the tile on one side only.
    fn passage_room(col: u8, solid_from: usize, solid_to: usize) -> Room {
        let mut lines = walled();
        for line in &mut lines[6..10] {
            line.replace_range(solid_from..solid_to, &"#".repeat(solid_to - solid_from));
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let grid = room(&refs);
        assert_eq!(marker_cell(col * 8, 87), (13, col));
        Room::read(
            |row, c| if grid(row, c) { 0x47 } else { 0x07 },
            &[(col * 8, 87, PASSAGE)],
        )
    }

    /// Rooms 192 to 195 on the tape: each passage is walked into from its one
    /// free side and leads to the room on that side, so 192 joins 193 and 194
    /// joins 195, but 193 and 194, whose passages face away from each other,
    /// do not join (#10, decision 6).
    #[test]
    fn a_passage_joins_only_the_room_on_the_side_it_is_walked_into_from() {
        // Free on its left, a wall on its right: walked into going right.
        let right = || passage_room(25, 27, 31);
        // A wall on its left, free on its right: walked into going left.
        let left = || passage_room(5, 1, 5);
        assert!(right().passage_right && !right().passage_left);
        assert!(left().passage_left && !left().passage_right);
        let o = openings(&[right(), left(), right(), left()], 999);
        assert!(o[0].right && o[1].left, "192 and 193 join");
        assert!(!o[1].right && !o[2].left, "193 and 194 do not");
        assert!(o[2].right && o[3].left, "194 and 195 join");
        assert!(!o[0].left && !o[3].right);
    }

    /// A room read from text, `#` solid, `L` a lift's cell and anything else
    /// free, with the markers given.
    fn read_text(lines: &[String], markers: &[(u8, u8, u8)]) -> Room {
        let grid: Vec<Vec<char>> = lines.iter().map(|l| l.chars().collect()).collect();
        Room::read(
            |row, col| match grid[usize::from(row - FIRST_ROW)][usize::from(col)] {
                '#' => 0x07,
                'L' => LIFT_ATTRS[0],
                _ => 0x47,
            },
            markers,
        )
    }

    /// Seventeen closed rooms, so rooms 0, 1 and 16 can be drawn.
    fn planet_of(drawn: &[(usize, Room)]) -> Vec<Room> {
        let mut rooms: Vec<Room> = (0..17).map(|_| read_text(&walled(), &[])).collect();
        for (i, r) in drawn {
            rooms[*i] = r.clone();
        }
        rooms
    }

    /// A walled room with gaps: `left`/`right` rows 8 and 9, `down`/`up`
    /// columns 10 and 11.
    fn gapped(left: bool, right: bool, up: bool, down: bool) -> Vec<String> {
        let mut lines = walled();
        for r in [8, 9] {
            if left {
                lines[r].replace_range(0..1, ".");
            }
            if right {
                lines[r].replace_range(31..32, ".");
            }
        }
        if up {
            lines[0].replace_range(10..12, "..");
        }
        if down {
            lines[17].replace_range(10..12, "..");
        }
        lines
    }

    fn ways_between(g: &Graph, from: u16, to: u16) -> Vec<Place> {
        g.places()
            .filter(|p| p.0 == from)
            .flat_map(|p| g.ways(p).to_vec())
            .filter(|w| w.0 == to)
            .collect()
    }

    #[test]
    fn rooms_open_to_each_other_join_both_ways() {
        let rooms = planet_of(&[
            (0, read_text(&gapped(false, true, false, false), &[])),
            (1, read_text(&gapped(true, false, false, false), &[])),
        ]);
        let g = Graph::new(&rooms, 999);
        assert_eq!(ways_between(&g, 0, 1), [(1, 1)]);
        assert_eq!(ways_between(&g, 1, 0), [(0, 1)]);
    }

    #[test]
    fn only_the_part_of_a_divided_room_that_touches_the_edge_joins() {
        // Room 0 open on its right, but a wall down its middle.
        let mut lines = gapped(false, true, false, false);
        for line in &mut lines {
            line.replace_range(15..17, "##");
        }
        let rooms = planet_of(&[
            (0, read_text(&lines, &[])),
            (1, read_text(&gapped(true, false, false, false), &[])),
        ]);
        let g = Graph::new(&rooms, 999);
        let from: Vec<Place> = g.places().filter(|p| p.0 == 0).collect();
        assert_eq!(from.len(), 1, "one part of room 0 has a way out: {from:?}");
        assert_eq!(from[0].1, rooms[0].shut.at(12, 20));
    }

    /// Blob can fly everywhere at level 5 (#10, decision 5): a drop is a way back
    /// up as well as down.
    #[test]
    fn a_drop_is_a_way_down_and_back_up() {
        let rooms = planet_of(&[
            (0, read_text(&gapped(false, false, false, true), &[])),
            (16, read_text(&gapped(false, false, true, false), &[])),
        ]);
        let g = Graph::new(&rooms, 999);
        assert_eq!(ways_between(&g, 0, 16), [(16, 1)]);
        assert_eq!(ways_between(&g, 16, 0), [(0, 1)]);
    }

    #[test]
    fn a_lift_is_a_way_up_and_never_down() {
        let mut below = gapped(false, false, true, false);
        for line in &mut below[0..17] {
            line.replace_range(10..12, "LL");
        }
        let rooms = planet_of(&[
            (0, read_text(&gapped(false, false, false, true), &[])),
            (16, read_text(&below, &[])),
        ]);
        let g = Graph::new(&rooms, 999);
        assert_eq!(ways_between(&g, 16, 0), [(0, 1)]);
        assert!(
            ways_between(&g, 0, 16).is_empty(),
            "a lift only ever goes up"
        );
    }

    #[test]
    fn passages_join_the_parts_blob_uses_them_from() {
        let rooms = planet_of(&[(0, passage_room(25, 27, 31)), (1, passage_room(5, 1, 5))]);
        let g = Graph::new(&rooms, 999);
        assert_eq!(ways_between(&g, 0, 1), [(1, rooms[1].passage_part)]);
        assert_eq!(ways_between(&g, 1, 0).len(), 1);
    }

    #[test]
    fn the_core_is_reached_from_the_room_to_its_left_and_leads_nowhere() {
        let rooms = planet_of(&[
            (0, read_text(&gapped(false, true, false, false), &[])),
            (1, read_text(&gapped(true, true, false, false), &[])),
        ]);
        let g = Graph::new(&rooms, 1);
        assert_eq!(ways_between(&g, 0, 1), [(1, 1)]);
        assert!(g.places().all(|p| p.0 != 1), "nothing out of the core");
    }

    #[test]
    fn level_5_routes_through_rooms_never_walked() {
        // 0 → 1 sideways, 1 → 17 down, and back up.
        let mut rooms = planet_of(&[
            (0, read_text(&gapped(false, true, false, false), &[])),
            (1, read_text(&gapped(true, false, false, true), &[])),
        ]);
        rooms.push(read_text(&gapped(false, false, true, false), &[]));
        let g = Graph::new(&rooms, 999);
        let there = g.route((0, 1), &[], &targets(&[17]), &[]).unwrap();
        assert_eq!(there.iter().map(|s| s.room).collect::<Vec<_>>(), [1, 17]);
        let back = g.route((17, 1), &[], &targets(&[0]), &[]).unwrap();
        assert_eq!(back.iter().map(|s| s.room).collect::<Vec<_>>(), [1, 0]);
    }

    #[test]
    fn level_5_jumps_between_booths_entered_only() {
        // Room 0 and room 16, closed all round, each with a booth: the only
        // way into 16 is the jump.
        let booth = (160, 15, BOOTH);
        let rooms = planet_of(&[
            (0, read_text(&gapped(false, true, false, false), &[booth])),
            (16, read_text(&walled(), &[booth])),
            (1, read_text(&gapped(true, false, false, false), &[])),
        ]);
        let g = Graph::new(&rooms, 999);
        assert_eq!(
            g.route((0, 1), &[0], &targets(&[16]), &[]),
            None,
            "16's booth not entered"
        );
        let route = g.route((0, 1), &[0, 16], &targets(&[16]), &[]).unwrap();
        assert_eq!(
            route,
            [Step {
                room: 16,
                teleport: true
            }]
        );
    }

    #[test]
    fn level_5_goes_through_a_door() {
        // Room 0 open on its right, divided by a solid pillar with a door's
        // tile against it, as in room 210: level 5 counts the door open and
        // leaves the items to the player.
        let mut lines = gapped(false, true, false, false);
        for line in &mut lines {
            line.replace_range(15..17, "##");
        }
        let door = read_text(&lines, &[(112, 87, DOOR)]);
        let (left, right) = (door.shut.at(12, 5), door.shut.at(12, 25));
        assert!(left != 0 && right != 0 && left != right, "two halves shut");
        let rooms = planet_of(&[
            (0, door),
            (1, read_text(&gapped(true, false, false, false), &[])),
        ]);
        let g = Graph::new(&rooms, 999);
        let from_left = g.place(0, 5 * 8, 143 - 8 * 6);
        assert_eq!(
            g.route(from_left, &[], &targets(&[1]), &[]),
            Some(vec![Step {
                room: 1,
                teleport: false
            }])
        );
    }

    #[test]
    fn level_5_goes_round_to_the_part_a_piece_is_in() {
        // Room 1 split by a pillar, open on both sides; the way from its
        // left half to its right half runs through 0, the row below and 2.
        let mut split = gapped(true, true, false, false);
        for line in &mut split {
            line.replace_range(15..17, "##");
        }
        let mut rooms = planet_of(&[
            (0, read_text(&gapped(false, true, false, true), &[])),
            (1, read_text(&split, &[])),
            (2, read_text(&gapped(true, false, false, true), &[])),
            (16, read_text(&gapped(false, true, true, false), &[])),
        ]);
        rooms.push(read_text(&gapped(true, true, false, false), &[]));
        rooms.push(read_text(&gapped(true, false, true, false), &[]));
        let g = Graph::new(&rooms, 999);
        let (left, right) = ((1, g.part_at(1, 12, 5)), (1, g.part_at(1, 12, 25)));
        assert!(left.1 != 0 && right.1 != 0 && left != right, "two halves");
        let route = g.route(left, &[], &RoomSet::default(), &[right]).unwrap();
        assert_eq!(
            route.iter().map(|s| s.room).collect::<Vec<_>>(),
            [0, 16, 17, 18, 2, 1]
        );
        assert_eq!(
            g.route(left, &[], &RoomSet::default(), &[left]),
            Some(vec![]),
            "already in the piece's part"
        );
        assert_eq!(
            g.route(left, &[], &targets(&[1]), &[]),
            Some(vec![]),
            "the whole room, before the piece's spot is known"
        );
    }

    #[test]
    fn a_gap_that_goes_round_the_wall_joins_the_parts() {
        let mut lines = open_both_sides();
        for (r, line) in lines.iter_mut().enumerate() {
            if r != 1 && r != 2 {
                line.replace_range(15..17, "##");
            }
        }
        assert!(divides_of(&lines).cells.iter().all(|&bits| bits == 0));
    }
}
