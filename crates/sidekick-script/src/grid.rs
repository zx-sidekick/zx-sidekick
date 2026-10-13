//! A grid of cells for scripts: where a body fits, the parts those places join into, and the
//! blocking cells where parts meet. Done here, in Rust, since a script would take millions of
//! steps for a game's rooms; each call counts as one of the script's operations. Nothing here
//! knows a game: a script hands over which cells block.

use rhai::{Array, Dynamic, Engine, EvalAltResult};
use std::collections::VecDeque;

/// The most cells a grid may have, as a script's arrays may.
const MOST_CELLS: usize = 65_536;

/// Cells, row by row, each blocking or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    width: usize,
    height: usize,
    blocks: Vec<bool>,
}

/// The places a body fits in a grid, row by row, numbered by the part each joins: places side
/// by side or one above the other are one part. `None` where the body doesn't fit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parts {
    /// Places across and down: the grid's size less the body's, plus one.
    width: usize,
    height: usize,
    /// The body's size.
    body: (usize, usize),
    part: Vec<Option<usize>>,
    count: usize,
}

impl Grid {
    /// A grid `width` cells across and `height` down, from `blocks`, row by row.
    ///
    /// # Errors
    /// When `blocks` isn't `width` × `height` long, or the grid is empty or too large.
    pub fn new(width: usize, height: usize, blocks: Vec<bool>) -> Result<Self, String> {
        let cells = width.saturating_mul(height);
        if width == 0 || height == 0 || cells > MOST_CELLS {
            return Err(format!(
                "a grid is 1 to {MOST_CELLS} cells, not {width} x {height}"
            ));
        }
        if blocks.len() != cells {
            return Err(format!(
                "a {width} x {height} grid has {cells} cells, not {}",
                blocks.len()
            ));
        }
        Ok(Self {
            width,
            height,
            blocks,
        })
    }

    fn blocks(&self, x: usize, y: usize) -> bool {
        self.blocks[y * self.width + x]
    }

    /// Whether a body `w` × `h` cells fits with its top-left cell at (`x`, `y`): every cell it
    /// covers free.
    fn fits_at(&self, x: usize, y: usize, (w, h): (usize, usize)) -> bool {
        (y..y + h).all(|cy| (x..x + w).all(|cx| !self.blocks(cx, cy)))
    }

    /// The places a body `w` × `h` cells large fits, numbered by part.
    #[must_use]
    pub fn parts(&self, w: usize, h: usize) -> Parts {
        let w = w.clamp(1, self.width);
        let h = h.clamp(1, self.height);
        let (pw, ph) = (self.width - w + 1, self.height - h + 1);
        let fits: Vec<bool> = (0..ph)
            .flat_map(|y| (0..pw).map(move |x| (x, y)))
            .map(|(x, y)| self.fits_at(x, y, (w, h)))
            .collect();
        let mut part = vec![None; pw * ph];
        let mut count = 0;
        for start in 0..part.len() {
            if !fits[start] || part[start].is_some() {
                continue;
            }
            part[start] = Some(count);
            let mut queue = VecDeque::from([start]);
            while let Some(at) = queue.pop_front() {
                for next in neighbours(at, pw, ph) {
                    if fits[next] && part[next].is_none() {
                        part[next] = Some(count);
                        queue.push_back(next);
                    }
                }
            }
            count += 1;
        }
        Parts {
            width: pw,
            height: ph,
            body: (w, h),
            part,
            count,
        }
    }

    /// The blocking cells where parts meet: each part in `sources` grows out from the cells its
    /// places cover, one cell a step through every cell, free or blocking, all at once; where
    /// two parts' growths touch, the blocking cells on either side are marked. Row by row.
    #[must_use]
    pub fn seams(&self, parts: &Parts, sources: &[usize]) -> Vec<bool> {
        let cells = self.width * self.height;
        let mut owner: Vec<Option<usize>> = vec![None; cells];
        let mut queue = VecDeque::new();
        for py in 0..parts.height {
            for px in 0..parts.width {
                let Some(part) = parts.part[py * parts.width + px] else {
                    continue;
                };
                if !sources.contains(&part) {
                    continue;
                }
                for y in py..py + parts.body.1 {
                    for x in px..px + parts.body.0 {
                        let at = y * self.width + x;
                        if owner[at].is_none() {
                            owner[at] = Some(part);
                            queue.push_back(at);
                        }
                    }
                }
            }
        }
        let mut seam = vec![false; cells];
        while let Some(at) = queue.pop_front() {
            for next in neighbours(at, self.width, self.height) {
                match owner[next] {
                    None => {
                        owner[next] = owner[at];
                        queue.push_back(next);
                    }
                    Some(other) if Some(other) != owner[at] => {
                        seam[at] |= self.blocks[at];
                        seam[next] |= self.blocks[next];
                    }
                    Some(_) => {}
                }
            }
        }
        seam
    }
}

impl Parts {
    /// The part of the place with its top-left cell at (`x`, `y`), if the body fits there.
    #[must_use]
    pub fn at(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| self.part[y * self.width + x])
            .flatten()
    }

    /// How many parts there are.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
}

/// The cells beside `at`, up, down, left and right, in a grid `width` × `height`.
fn neighbours(at: usize, width: usize, height: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (at % width, at / width);
    [
        (x > 0).then(|| at - 1),
        (x + 1 < width).then(|| at + 1),
        (y > 0).then(|| at - width),
        (y + 1 < height).then(|| at + width),
    ]
    .into_iter()
    .flatten()
}

fn size(n: i64) -> Result<usize, Box<EvalAltResult>> {
    usize::try_from(n).map_err(|_| format!("{n} isn't a size").into())
}

fn index(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `grid`, `parts`, `seams` and the rest, for scripts.
pub(crate) fn register(engine: &mut Engine) {
    engine
        .register_type_with_name::<Grid>("Grid")
        .register_type_with_name::<Parts>("Parts")
        .register_fn(
            "grid",
            |width: i64, height: i64, cells: Array| -> Result<Grid, Box<EvalAltResult>> {
                let blocks = cells
                    .iter()
                    .map(|cell| {
                        cell.as_bool()
                            .or_else(|_| cell.as_int().map(|n| n != 0))
                            .map_err(|_| "a grid's cells are numbers or flags".into())
                    })
                    .collect::<Result<Vec<bool>, Box<EvalAltResult>>>()?;
                Grid::new(size(width)?, size(height)?, blocks).map_err(Into::into)
            },
        )
        .register_fn(
            "parts",
            |grid: &mut Grid, w: i64, h: i64| -> Result<Parts, Box<EvalAltResult>> {
                Ok(grid.parts(size(w)?, size(h)?))
            },
        )
        .register_fn(
            "seams",
            |grid: &mut Grid, parts: Parts, sources: Array| -> Result<Array, Box<EvalAltResult>> {
                let sources = sources
                    .iter()
                    .map(|s| {
                        s.as_int()
                            .map_err(|_| "a part is a number".into())
                            .and_then(size)
                    })
                    .collect::<Result<Vec<usize>, Box<EvalAltResult>>>()?;
                Ok(grid
                    .seams(&parts, &sources)
                    .into_iter()
                    .map(Dynamic::from_bool)
                    .collect())
            },
        )
        .register_fn("at", |parts: &mut Parts, x: i64, y: i64| -> Dynamic {
            match (usize::try_from(x), usize::try_from(y)) {
                (Ok(x), Ok(y)) => parts.at(x, y).map_or(Dynamic::UNIT, |p| index(p).into()),
                _ => Dynamic::UNIT,
            }
        })
        .register_get("count", |parts: &mut Parts| index(parts.count()))
        .register_get("width", |parts: &mut Parts| index(parts.width))
        .register_get("height", |parts: &mut Parts| index(parts.height));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grid from rows of `#` (blocks) and `.` (free).
    fn grid(rows: &[&str]) -> Grid {
        let blocks = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        Grid::new(rows[0].len(), rows.len(), blocks).unwrap()
    }

    #[test]
    fn a_body_fits_where_all_its_cells_are_free() {
        let g = grid(&["....", ".#..", "...."]);
        let parts = g.parts(2, 2);
        assert_eq!((parts.width, parts.height), (3, 2));
        assert_eq!(parts.at(0, 0), None, "over the block");
        assert_eq!(parts.at(2, 0), Some(0));
        assert_eq!(parts.at(2, 1), Some(0));
        assert_eq!(parts.at(3, 0), None, "past the edge");
    }

    #[test]
    fn a_wall_parts_the_places() {
        let g = grid(&["...#...", "...#...", "...#..."]);
        let parts = g.parts(2, 2);
        assert_eq!(parts.count(), 2);
        assert_ne!(parts.at(0, 0), parts.at(4, 0));
        let one_wide = grid(&["...#.##", "...#.##"]);
        assert_eq!(
            one_wide.parts(2, 2).count(),
            1,
            "a one-cell gap fits no body"
        );
    }

    #[test]
    fn a_seam_runs_through_the_wall_between_two_parts() {
        let g = grid(&["#########", "...###...", "...###...", "#########"]);
        let parts = g.parts(2, 2);
        assert_eq!(parts.count(), 2);
        let sources = [parts.at(0, 1).unwrap(), parts.at(6, 1).unwrap()];
        let seam = g.seams(&parts, &sources);
        let marked: Vec<String> = (0..4)
            .map(|y| {
                (0..9)
                    .map(|x| if seam[y * 9 + x] { '#' } else { '.' })
                    .collect()
            })
            .collect();
        // The two growths meet in the middle of the wall, through the top and bottom rows too.
        assert_eq!(marked, ["....##...", "....##...", "....##...", "....##..."]);
    }

    #[test]
    fn a_part_that_isn_t_a_source_draws_no_seam() {
        let g = grid(&["...#...", "...#..."]);
        let parts = g.parts(2, 2);
        let only = [parts.at(0, 0).unwrap()];
        assert!(g.seams(&parts, &only).iter().all(|&s| !s));
    }

    #[test]
    fn a_grid_s_cells_must_be_its_size() {
        assert!(Grid::new(3, 2, vec![false; 5]).is_err());
        assert!(Grid::new(0, 2, Vec::new()).is_err());
    }
}
