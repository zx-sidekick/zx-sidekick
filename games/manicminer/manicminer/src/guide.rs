//! What guidance reads of the cavern being played (#153): its cells and
//! what each is, the items, the portal, the conveyor, the guardians' paths,
//! Willy and the air. Everything is read from memory the game keeps, laid
//! out as [`facts::at`] says; nothing is written and none of the game's
//! rules is reimplemented.

use crate::facts::{AIR_CLOCK_STEP, AIR_EMPTY, at, caverns};

/// The cavern is 32 cells across and 16 down.
pub const COLUMNS: usize = 32;
pub const ROWS: usize = 16;

/// What a cell is: the cavern's tile whose attribute it holds, looked for
/// in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Background,
    Floor,
    Crumbling,
    Wall,
    Conveyor,
    Nasty,
    Extra,
}

/// The tile kinds in the order the cavern lists them: two of them nasty.
const KINDS: [Tile; 8] = [
    Tile::Background,
    Tile::Floor,
    Tile::Crumbling,
    Tile::Wall,
    Tile::Conveyor,
    Tile::Nasty,
    Tile::Nasty,
    Tile::Extra,
];

/// A cell: its row and column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub row: u8,
    pub col: u8,
}

impl Cell {
    /// The cell an address in the attribute buffer names.
    fn at(address: u16) -> Cell {
        let offset = address.wrapping_sub(at::ATTRIBUTES) & 0x1FF;
        Cell {
            row: (offset / 32) as u8,
            col: (offset % 32) as u8,
        }
    }
}

/// An item and whether it is still there to take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub cell: Cell,
    pub left: bool,
}

/// The conveyor: its leftmost cell, how many cells long, and which way it
/// carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conveyor {
    pub cell: Cell,
    pub length: u8,
    pub rightwards: bool,
}

/// A guardian's patrol, in cells of its top left corner: along a row from
/// `from` to `to` columns, or down a column from `from` to `to` rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Patrol {
    Across { row: u8, from: u8, to: u8 },
    Down { col: u8, from: u8, to: u8 },
}

/// What guidance reads of the cavern being played.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cavern {
    /// 0 to 19.
    pub number: u8,
    pub name: String,
    /// The empty cavern's cells, row by row.
    pub cells: Vec<Tile>,
    pub willy: Option<Cell>,
    pub items: Vec<Item>,
    pub portal: Option<Cell>,
    pub portal_open: bool,
    pub conveyor: Option<Conveyor>,
    pub patrols: Vec<Patrol>,
    /// Passes of the main loop until the air runs out: the air and its
    /// clock as they stand, one unit of air every wrap of the clock.
    pub air_passes: u32,
}

impl Cavern {
    /// How many items are left to take.
    #[must_use]
    pub fn items_left(&self) -> usize {
        self.items.iter().filter(|i| i.left).count()
    }

    /// What the cell at `row`, `col` is.
    #[must_use]
    pub fn tile(&self, row: usize, col: usize) -> Tile {
        self.cells
            .get(row * COLUMNS + col)
            .copied()
            .unwrap_or(Tile::Background)
    }
}

fn byte(mem: &[u8], a: u16) -> u8 {
    mem[usize::from(a)]
}

fn word(mem: &[u8], a: u16) -> u16 {
    u16::from_le_bytes([byte(mem, a), byte(mem, a.wrapping_add(1))])
}

/// Reads the cavern being played from the machine's memory, `mem` being all
/// 64K of it.
#[must_use]
pub fn read(mem: &[u8]) -> Cavern {
    let tiles: Vec<u8> = (0..8u16).map(|t| byte(mem, at::TILES + 9 * t)).collect();
    let kind = |attr: u8| {
        tiles
            .iter()
            .position(|&t| t == attr)
            .map_or(Tile::Background, |i| KINDS[i])
    };
    let empty = usize::from(at::EMPTY_CELLS);
    let cells: Vec<Tile> = mem[empty..empty + COLUMNS * ROWS]
        .iter()
        .map(|&a| kind(a))
        .collect();
    let name_at = usize::from(at::CAVERN_NAME);
    let name = String::from_utf8_lossy(&mem[name_at..name_at + 32])
        .trim()
        .to_string();

    let mut items = Vec::new();
    let mut a = at::ITEMS;
    while byte(mem, a) != 0xFF && items.len() < 5 {
        items.push(Item {
            cell: Cell::at(word(mem, a + 1)),
            left: byte(mem, a) != 0,
        });
        a += 5;
    }

    // The conveyor's address is in the screen buffer, laid out as the
    // display file: a third, a character row, a pixel line, a column. Two
    // caverns keep a record where no conveyor is laid (The Endorian Forest,
    // Amoebatrons' Revenge): a conveyor is one only on its own tiles.
    let conveyor = {
        let off = word(mem, at::CONVEYOR + 1).wrapping_sub(at::SCREEN_BUFFER);
        let length = byte(mem, at::CONVEYOR + 3);
        (off < 0x1000 && length > 0)
            .then(|| Conveyor {
                cell: Cell {
                    row: (((off >> 11) << 3) | ((off >> 5) & 7)) as u8,
                    col: (off & 31) as u8,
                },
                length,
                rightwards: byte(mem, at::CONVEYOR) == 1,
            })
            .filter(|v| {
                (0..v.length).all(|k| {
                    let col = usize::from(v.cell.col + k);
                    col < COLUMNS
                        && cells[usize::from(v.cell.row) * COLUMNS + col] == Tile::Conveyor
                })
            })
    };

    let mut patrols = Vec::new();
    for g in 0..4u16 {
        let r = at::HORIZONTAL + 7 * g;
        if byte(mem, r) == 0xFF {
            break;
        }
        if byte(mem, r) != 0 {
            let cell = Cell::at(word(mem, r + 1));
            patrols.push(Patrol::Across {
                row: cell.row,
                from: byte(mem, r + 5) & 31,
                to: byte(mem, r + 6) & 31,
            });
        }
    }
    // The vertical table patrols only where the game runs it: not before
    // its first cavern, and not where the Skylabs fall instead. Eugene and
    // the Kong Beast have routines of their own and no patrol here.
    let number = byte(mem, at::CAVERN);
    let vertical = number >= caverns::VERTICAL_FROM && number != caverns::SKYLABS;
    for g in 0..4u16 {
        let r = at::VERTICAL + 7 * g;
        if !vertical || byte(mem, r) == 0xFF {
            break;
        }
        patrols.push(Patrol::Down {
            col: byte(mem, r + 3),
            from: byte(mem, r + 5) / 8,
            to: byte(mem, r + 6) / 8,
        });
    }

    let air = byte(mem, at::AIR);
    let clock = byte(mem, at::CLOCK);
    let air_passes = u32::from(air.saturating_sub(AIR_EMPTY)) * (256 / u32::from(AIR_CLOCK_STEP))
        + u32::from(clock / AIR_CLOCK_STEP)
        + 1;

    Cavern {
        number,
        name,
        cells,
        willy: Some(Cell::at(word(mem, at::WILLY_CELL))),
        items,
        portal: Some(Cell::at(word(mem, at::PORTAL_CELL))),
        portal_open: byte(mem, at::PORTAL) & 0x80 != 0,
        conveyor,
        patrols,
        air_passes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A memory with one of each thing where the facts say.
    fn staged() -> Vec<u8> {
        let mut mem = vec![0u8; 0x10000];
        let set = |mem: &mut Vec<u8>, a: u16, bytes: &[u8]| {
            mem[usize::from(a)..usize::from(a) + bytes.len()].copy_from_slice(bytes);
        };
        set(&mut mem, at::CAVERN, &[9]);
        set(
            &mut mem,
            at::CAVERN_NAME,
            b"   Test Cavern                  ",
        );
        for (t, attr) in [0x00, 0x42, 0x02, 0x16, 0x04, 0x44, 0x05, 0x00]
            .iter()
            .enumerate()
        {
            mem[usize::from(at::TILES) + 9 * t] = *attr;
        }
        let empty = usize::from(at::EMPTY_CELLS);
        mem[empty + 32 * 15 + 3] = 0x16; // a wall
        mem[empty + 32 * 14] = 0x44; // a nasty
        mem[empty + 32 * 14 + 1] = 0x05; // the other nasty
        mem[empty + 32 * 5 + 7] = 0x02; // crumbling
        mem[empty + 32 * 9 + 8..empty + 32 * 9 + 28].fill(0x04); // the conveyor
        // Two items, the second taken, then the end.
        set(
            &mut mem,
            at::ITEMS,
            &[
                0x06, 0x09, 0x5C, 0x60, 0xFF, 0x00, 0x1D, 0x5C, 0x60, 0xFF, 0xFF,
            ],
        );
        set(&mut mem, at::PORTAL, &[0x8E]);
        set(&mut mem, at::PORTAL_CELL, &[0xBD, 0x5D]);
        // The conveyor at row 9, column 8, 20 long, carrying left.
        set(&mut mem, at::CONVEYOR, &[0x00, 0x28, 0x78, 0x14]);
        set(&mut mem, at::WILLY_CELL, &[0xA2, 0x5D]);
        set(
            &mut mem,
            at::HORIZONTAL,
            &[
                0x46, 0xEB, 0x5C, 0x60, 0x06, 0xE8, 0xEF, 0x00, 0, 0, 0, 0, 0, 0, 0xFF,
            ],
        );
        set(
            &mut mem,
            at::VERTICAL,
            &[0x26, 0x00, 0x40, 0x05, 0x03, 0x02, 0x66, 0xFF],
        );
        set(&mut mem, at::AIR, &[AIR_EMPTY + 2, 0x08]);
        mem
    }

    #[test]
    fn a_cavern_is_read_where_the_facts_say() {
        let c = read(&staged());
        assert_eq!(c.number, 9);
        assert_eq!(c.name, "Test Cavern");
        assert_eq!(c.tile(15, 3), Tile::Wall);
        assert_eq!(c.tile(14, 0), Tile::Nasty);
        assert_eq!(c.tile(14, 1), Tile::Nasty);
        assert_eq!(c.tile(5, 7), Tile::Crumbling);
        assert_eq!(c.tile(0, 0), Tile::Background);
        assert_eq!(c.items.len(), 2);
        assert_eq!(c.items[0].cell, Cell { row: 0, col: 9 });
        assert_eq!(c.items_left(), 1);
        assert_eq!(c.portal, Some(Cell { row: 13, col: 29 }));
        assert!(c.portal_open);
        assert_eq!(c.willy, Some(Cell { row: 13, col: 2 }));
        assert_eq!(
            c.conveyor,
            Some(Conveyor {
                cell: Cell { row: 9, col: 8 },
                length: 20,
                rightwards: false
            })
        );
        assert_eq!(
            c.patrols,
            vec![
                Patrol::Across {
                    row: 7,
                    from: 8,
                    to: 15
                },
                Patrol::Down {
                    col: 5,
                    from: 0,
                    to: 12
                },
            ]
        );
    }

    #[test]
    fn a_conveyor_is_one_only_on_its_own_tiles() {
        let mut mem = staged();
        mem[usize::from(at::EMPTY_CELLS) + 32 * 9 + 10] = 0x42;
        assert_eq!(read(&mem).conveyor, None);
    }

    #[test]
    fn the_vertical_table_patrols_only_where_the_game_runs_it() {
        let mut mem = staged();
        for number in [0, caverns::VERTICAL_FROM - 1, caverns::SKYLABS] {
            mem[usize::from(at::CAVERN)] = number;
            let c = read(&mem);
            assert!(
                c.patrols.iter().all(|p| matches!(p, Patrol::Across { .. })),
                "cavern {number}"
            );
        }
    }

    #[test]
    fn the_air_counts_passes_to_the_last_wrap() {
        // Two units and a clock of 8: 2 passes to the wrap, then 64 each.
        assert_eq!(read(&staged()).air_passes, 2 * 64 + 2 + 1);
        let mut mem = staged();
        mem[usize::from(at::AIR)] = AIR_EMPTY;
        mem[usize::from(at::CLOCK)] = 0;
        assert_eq!(read(&mem).air_passes, 1, "one pass: the wrap that ends it");
    }
}
