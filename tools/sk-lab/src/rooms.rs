//! Every room as the game draws it, read by `sidekick::starquake::read_room`
//! on copies of the machine, with its cells kept; and the planet's graph of
//! (room, part) nodes over them.

use std::collections::{HashSet, VecDeque};

use sidekick::Machine;
use sidekick::map::{COLS, Openings, ROWS, Room, free};
use sidekick::starquake::{CORE_ROOM, all_openings, read_room};

use crate::LIFT_ATTRS;

/// A room's attribute bytes, 18 rows of 32 from the top of the play area.
pub type Cells = [[u8; 32]; 18];

/// The whole planet, read once.
pub struct Planet {
    pub rooms: Vec<Room>,
    pub cells: Vec<Cells>,
    pub openings: Vec<Openings>,
}

impl Planet {
    /// Reads every room on copies of `base`.
    ///
    /// # Panics
    ///
    /// If a room does not finish drawing, which means `base` does not hold
    /// the game.
    #[must_use]
    pub fn read(base: &Machine) -> Planet {
        let mut rooms = Vec::new();
        let mut cells = Vec::new();
        for room in 0..COLS * ROWS {
            let mut m = base.clone();
            rooms.push(read_room(&mut m, room));
            let mut c = [[0u8; 32]; 18];
            for (row, line) in c.iter_mut().enumerate() {
                for (col, cell) in line.iter_mut().enumerate() {
                    *cell = m.zx.mem[0x5800 + (row + 6) * 32 + col];
                }
            }
            cells.push(c);
        }
        let openings = all_openings(base);
        Planet {
            rooms,
            cells,
            openings,
        }
    }

    /// Whether Blob fits with his top-left cell at (`row`, `col`) of `room`,
    /// rows and columns from the top left of the play area.
    #[must_use]
    pub fn fits(&self, room: u16, row: usize, col: usize) -> bool {
        let c = &self.cells[usize::from(room)];
        row + 1 < 18
            && col + 1 < 32
            && free(c[row][col])
            && free(c[row][col + 1])
            && free(c[row + 1][col])
            && free(c[row + 1][col + 1])
    }

    /// The part of `room` a passage's tile is in, with doors shut: the first
    /// place Blob fits beside its four-by-three tile.
    #[must_use]
    pub fn passage_part(&self, room: u16) -> Option<u8> {
        let r = &self.rooms[usize::from(room)];
        let (row, col) = r.passage?;
        (row.saturating_sub(1)..row + 3)
            .flat_map(|rr| (col.saturating_sub(2)..col + 4).map(move |c| (rr, c)))
            .map(|(rr, c)| r.shut.at(rr, c))
            .find(|&p| p != 0)
    }

    /// The rooms reachable from `start` (with Blob's top cell at screen
    /// `row`, `col`) over (room, part) nodes: two rooms join where both have
    /// Blob-sized free cells at the same place on their shared edge, or
    /// through wall passages. Doors shut, no teleporter, and the core room
    /// a dead end.
    #[must_use]
    pub fn reach(&self, start: u16, row: u8, col: u8) -> HashSet<u16> {
        let part = self.rooms[usize::from(start)].shut.at(row, col);
        let mut seen: HashSet<(u16, u8)> = HashSet::from([(start, part)]);
        let mut queue = VecDeque::from([(start, part)]);
        let mut reached = HashSet::new();
        while let Some((room, part)) = queue.pop_front() {
            reached.insert(room);
            if room == CORE_ROOM {
                continue;
            }
            for (next, next_part) in self.neighbours(room, part) {
                if seen.insert((next, next_part)) {
                    queue.push_back((next, next_part));
                }
            }
        }
        reached
    }

    /// The (room, part) nodes one step from `room`'s `part`.
    fn neighbours(&self, room: u16, part: u8) -> Vec<(u16, u8)> {
        let r = &self.rooms[usize::from(room)];
        let col = room % COLS;
        let mut out = Vec::new();
        let mut join = |to: u16, p: u8| {
            if p != 0 {
                out.push((to, p));
            }
        };
        if col > 0 {
            let o = &self.rooms[usize::from(room) - 1];
            for row in 6..23 {
                if r.shut.at(row, 0) == part {
                    join(room - 1, o.shut.at(row, 30));
                }
            }
        }
        if col < COLS - 1 {
            let o = &self.rooms[usize::from(room) + 1];
            for row in 6..23 {
                if r.shut.at(row, 30) == part {
                    join(room + 1, o.shut.at(row, 0));
                }
            }
        }
        if room >= COLS {
            let o = &self.rooms[usize::from(room - COLS)];
            for c in 0..31 {
                if r.shut.at(6, c) == part {
                    join(room - COLS, o.shut.at(22, c));
                }
            }
        }
        if room + COLS < COLS * ROWS {
            let o = &self.rooms[usize::from(room + COLS)];
            for c in 0..31 {
                if r.shut.at(22, c) == part {
                    join(room + COLS, o.shut.at(6, c));
                }
            }
        }
        if self.passage_part(room) == Some(part) {
            if col > 0
                && let Some(p) = self.passage_part(room - 1)
            {
                join(room - 1, p);
            }
            if col < COLS - 1
                && let Some(p) = self.passage_part(room + 1)
            {
                join(room + 1, p);
            }
        }
        out
    }
}

/// Whether an attribute is a lift's.
#[must_use]
pub fn lift(attr: u8) -> bool {
    LIFT_ATTRS.contains(&attr)
}

/// A room's cells as text, `#` solid and `.` free, `=` a lift, one line a
/// row.
#[must_use]
pub fn draw(cells: &Cells) -> Vec<String> {
    cells
        .iter()
        .map(|line| {
            line.iter()
                .map(|&a| {
                    if lift(a) {
                        '='
                    } else if free(a) {
                        '.'
                    } else {
                        '#'
                    }
                })
                .collect()
        })
        .collect()
}
