//! The guidance panel beside the picture (#153), as Starquake's: the level
//! in its corner, and what the game shows made readable. Level 1 names the
//! cavern and says the air in seconds, the items left and whether the
//! portal is open; level 2 draws the cavern's cells with Willy, the items
//! ringed and the portal outlined; level 3 adds what can hurt him; level 4
//! where each jump from where he stands would land, or that it kills
//! (#155). It is drawn from [`guide::read`], what the game keeps in memory,
//! and from [`manicminer::preview`], the game itself run on a copy, and
//! nothing else.

use manicminer::guide::{self, COLUMNS, Cavern, Patrol, ROWS, Tile};
use manicminer::preview::{End, Jump};
use sidekick_frontend::overlay::{self, HEIGHT, PICTURE_W};
use sidekick_frontend::text::palette::{BRIGHT, DANGER, LABEL, PANEL, QUIET, RULE, SOFT};
use sidekick_frontend::text::{Canvas, Fonts, Rgb, Weight, span};

use crate::frontend::PANEL_W;
use crate::picker::LEVELS;

const CELLS: Rgb = [0x14, 0x17, 0x1f];
const FLOOR: Rgb = [0x2c, 0x38, 0x58];
const WALL: Rgb = [0x4a, 0x50, 0x62];
const CRUMBLE: Rgb = [0xf5, 0xb8, 0x4b];
const CONVEYOR: Rgb = [0x7f, 0xd1, 0xc7];
const PATROL: Rgb = [0xc9, 0x8b, 0xff];
const OPEN: Rgb = [0x6f, 0xd0, 0x8f];
const RIM: Rgb = [0x05, 0x06, 0x08];
const ARC: Rgb = [0x8f, 0xb4, 0xff];
const DIES: Rgb = [0xff, 0x5a, 0x64];

/// A cell of the cavern on the panel, in layout units: 32 of them fill the
/// panel's width less its margins.
const CELL: f32 = 12.0;

/// What the panel shows: the cavern being played, and the air left in
/// seconds once the main loop's rate is known, or nothing outside a game.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub cavern: Option<Cavern>,
    pub air_seconds: Option<u32>,
    /// The jumps from where Willy stands, at level 4 while he stands.
    pub jumps: Vec<Jump>,
}

/// How many frames of the main loop running before a rate is given at all:
/// two seconds.
const RATE_FIRST: u32 = 100;

/// Follows the game frame by frame for the panel: how often the main loop
/// comes round in the cavern being played, and what [`guide::read`] reads.
///
/// The rate is counted over everything since the cavern began, so it
/// settles rather than wobbling by a pass as a moving window would; and the
/// seconds shown never go back up while the air has not, so the figure
/// only ever counts down (#154).
#[derive(Default)]
pub struct Follow {
    /// Frames with the main loop running, and its passes in them, in this
    /// cavern.
    frames: u32,
    passes: u32,
    cavern: Option<u8>,
    /// The air's passes and the seconds last shown for them.
    shown: Option<(u32, u32)>,
}

impl Follow {
    /// After a frame: whether the main loop ran in it (`passed`), whether
    /// it is running at all (`looping`: not while Willy dies or the air is
    /// counted into the score, which would slow the rate), and whether a
    /// game is being played (`playing`: from its first pass to the title,
    /// so the panel stays through a death).
    pub fn frame(&mut self, mem: &[u8], passed: bool, looping: bool, playing: bool) -> View {
        if !playing {
            *self = Follow::default();
            return View::default();
        }
        let cavern = guide::read(mem);
        if self.cavern != Some(cavern.number) {
            *self = Follow {
                cavern: Some(cavern.number),
                ..Follow::default()
            };
        }
        if looping {
            self.frames += 1;
            self.passes += u32::from(passed);
        }
        let seconds = (self.frames >= RATE_FIRST && self.passes > 0).then(|| {
            let rate = self.passes as f32 * 50.0 / self.frames as f32;
            let seconds = (cavern.air_passes as f32 / rate).floor() as u32;
            match self.shown {
                Some((passes, shown)) if cavern.air_passes <= passes => seconds.min(shown),
                _ => seconds,
            }
        });
        if let Some(s) = seconds {
            self.shown = Some((cavern.air_passes, s));
        }
        View {
            cavern: Some(cavern),
            air_seconds: seconds,
            jumps: Vec::new(),
        }
    }
}

/// Draws the panel at guidance `level` beside the picture.
pub fn draw(fonts: &mut Fonts, canvas: &mut Canvas, view: &View, level: u8) {
    let window_w = overlay::width(PANEL_W);
    let left = PICTURE_W + 24.0;
    let right = window_w - 24.0;
    canvas.round_rect(PICTURE_W, 0.0, window_w - PICTURE_W, HEIGHT, 0.0, PANEL);
    canvas.round_rect(PICTURE_W, 0.0, 1.0, HEIGHT, 0.0, RULE);

    // The level in the corner opposite the label, with its name under it.
    fonts.spaced(canvas, left, 26.0, "GUIDANCE", 11.0, LABEL);
    let title = if level == 0 {
        "OFF".to_string()
    } else {
        format!("LEVEL {level}")
    };
    let tw = fonts.spaced_width(&title, 11.0);
    fonts.spaced(canvas, right - tw, 26.0, &title, 11.0, BRIGHT);
    if level == 0 {
        return;
    }
    let name = [span(
        LEVELS[usize::from(level)],
        12.0,
        Weight::Regular,
        SOFT,
    )];
    let w = fonts.measure(&name);
    fonts.text(Some(canvas), right - w, 42.0, None, 1.0, &name);

    let Some(cavern) = &view.cavern else {
        fonts.text(
            Some(canvas),
            left,
            86.0,
            None,
            1.0,
            &[span(
                "Guidance begins with a game.",
                13.0,
                Weight::Regular,
                QUIET,
            )],
        );
        return;
    };

    // The cavern: its number and name.
    let top = 86.0;
    fonts.spaced(
        canvas,
        left,
        top,
        &format!("CAVERN {} OF 20", cavern.number + 1),
        11.0,
        LABEL,
    );
    fonts.text(
        Some(canvas),
        left,
        top + 18.0,
        Some(right - left),
        1.0,
        &[span(&cavern.name, 22.0, Weight::SemiBold, BRIGHT)],
    );

    // Air, items, portal: three figures in a row.
    let fy = top + 76.0;
    let col = (right - left) / 3.0;
    let air = view
        .air_seconds
        .map_or_else(|| "\u{2013}".to_string(), |s| format!("{s} s"));
    let (portal, portal_colour) = if cavern.portal_open {
        ("open", OPEN)
    } else {
        ("shut", BRIGHT)
    };
    let figures = [
        ("AIR", air, BRIGHT),
        (
            "ITEMS LEFT",
            format!("{} of {}", cavern.items_left(), cavern.items.len()),
            BRIGHT,
        ),
        ("PORTAL", portal.to_string(), portal_colour),
    ];
    for (i, (label, value, colour)) in figures.iter().enumerate() {
        let x = left + col * i as f32;
        fonts.spaced(canvas, x, fy, label, 11.0, LABEL);
        fonts.text(
            Some(canvas),
            x,
            fy + 18.0,
            None,
            1.0,
            &[span(value, 26.0, Weight::SemiBold, *colour)],
        );
    }

    // The cavern's cells, from level 2.
    let my = fy + 92.0;
    if level < 2 {
        fonts.text(
            Some(canvas),
            left,
            my,
            None,
            1.0,
            &[span(
                "The cavern appears at level 2.",
                13.0,
                Weight::Regular,
                QUIET,
            )],
        );
        return;
    }
    fonts.spaced(canvas, left, my, "THE CAVERN", 11.0, LABEL);
    let cy = my + 22.0;
    cells(canvas, cavern, level, left, cy);
    if level >= 3 {
        hazards(canvas, cavern, left, cy);
    }
    marks(canvas, cavern, left, cy);
    if level >= 4 {
        jumps(canvas, &view.jumps, left, cy);
    }
    key(fonts, canvas, level, left, cy + ROWS as f32 * CELL + 18.0);
}

/// Where the cell at `row`, `col` starts on the panel.
fn at(left: f32, top: f32, row: impl Into<f32>, col: impl Into<f32>) -> (f32, f32) {
    (left + col.into() * CELL, top + row.into() * CELL)
}

/// The cavern's cells: walls and floors, and from level 3 the nasty tiles
/// and the crumbling floor.
fn cells(canvas: &mut Canvas, cavern: &Cavern, level: u8, left: f32, top: f32) {
    canvas.round_rect(
        left - 2.0,
        top - 2.0,
        COLUMNS as f32 * CELL + 4.0,
        ROWS as f32 * CELL + 4.0,
        3.0,
        CELLS,
    );
    for row in 0..ROWS {
        for col in 0..COLUMNS {
            let tile = cavern.tile(row, col);
            let colour = match tile {
                Tile::Wall => WALL,
                Tile::Nasty if level >= 3 => DANGER,
                Tile::Floor | Tile::Crumbling | Tile::Conveyor => FLOOR,
                Tile::Background | Tile::Nasty | Tile::Extra => continue,
            };
            let (x, y) = at(left, top, row as f32, col as f32);
            canvas.round_rect(x + 0.5, y + 0.5, CELL - 1.0, CELL - 1.0, 1.5, colour);
            if level >= 3 && tile == Tile::Crumbling {
                canvas.round_rect(x + 0.5, y + 0.5, CELL - 1.0, 4.0, 1.0, CRUMBLE);
            }
        }
    }
}

/// Level 3: the conveyor's arrows, and each guardian's path with a mark at
/// either end. A guardian is two cells square; its path runs through its
/// middle.
fn hazards(canvas: &mut Canvas, cavern: &Cavern, left: f32, top: f32) {
    if let Some(v) = cavern.conveyor {
        for k in 0..v.length {
            let (x, y) = at(left, top, v.cell.row, v.cell.col + k);
            let (x, y) = (x + CELL / 2.0, y + CELL / 2.0);
            let d = if v.rightwards { 3.0 } else { -3.0 };
            canvas.triangle([(x + d, y), (x - d, y - 3.5), (x - d, y + 3.5)], CONVEYOR);
        }
    }
    for patrol in &cavern.patrols {
        match *patrol {
            Patrol::Across { row, from, to } => {
                let y = top + (f32::from(row) + 1.0) * CELL;
                let x0 = left + (f32::from(from) + 1.0) * CELL;
                let x1 = left + (f32::from(to) + 1.0) * CELL;
                canvas.line((x0, y), (x1, y), 2.0, None, PATROL);
                for x in [x0, x1] {
                    canvas.round_rect(x - 1.5, y - 7.0, 3.0, 14.0, 1.0, PATROL);
                }
            }
            Patrol::Down { col, from, to } => {
                let x = left + (f32::from(col) + 1.0) * CELL;
                let y0 = top + (f32::from(from) + 1.0) * CELL;
                let y1 = top + (f32::from(to) + 1.0) * CELL;
                canvas.line((x, y0), (x, y1), 2.0, None, PATROL);
                for y in [y0, y1] {
                    canvas.round_rect(x - 7.0, y - 1.5, 14.0, 3.0, 1.0, PATROL);
                }
            }
        }
    }
}

/// Willy, the items left ringed, and the portal outlined: in green once it
/// is open.
fn marks(canvas: &mut Canvas, cavern: &Cavern, left: f32, top: f32) {
    if let Some(w) = cavern.willy {
        let (x, y) = at(left, top, w.row, w.col);
        canvas.round_rect(
            x + 1.0,
            y + 1.0,
            2.0 * CELL - 2.0,
            2.0 * CELL - 2.0,
            3.0,
            SOFT,
        );
    }
    for item in cavern.items.iter().filter(|i| i.left) {
        let (x, y) = at(left, top, item.cell.row, item.cell.col);
        let (x, y) = (x + CELL / 2.0, y + CELL / 2.0);
        canvas.outline(x - 10.0, y - 10.0, 20.0, 20.0, 10.0, 4.5, None, RIM);
        canvas.outline(x - 9.0, y - 9.0, 18.0, 18.0, 9.0, 2.5, None, BRIGHT);
        canvas.round_rect(x - 2.5, y - 2.5, 5.0, 5.0, 2.5, BRIGHT);
    }
    if let Some(p) = cavern.portal {
        let (x, y) = at(left, top, p.row, p.col);
        let colour = if cavern.portal_open { OPEN } else { BRIGHT };
        let side = 2.0 * CELL;
        canvas.outline(
            x - 3.0,
            y - 3.0,
            side + 6.0,
            side + 6.0,
            4.0,
            4.5,
            None,
            RIM,
        );
        canvas.outline(
            x - 2.0,
            y - 2.0,
            side + 4.0,
            side + 4.0,
            3.0,
            2.5,
            None,
            colour,
        );
    }
}

/// Level 4: each jump as dots along Willy's path, every other step, ending
/// in a ring where he lands or a cross where he dies (#155 decision 1).
fn jumps(canvas: &mut Canvas, jumps: &[Jump], left: f32, top: f32) {
    // A cavern pixel is an eighth of a cell.
    let at = |(x, y): (u8, u8)| {
        (
            left + f32::from(x) * CELL / 8.0,
            top + f32::from(y) * CELL / 8.0,
        )
    };
    for jump in jumps {
        for (i, &step) in jump.path.iter().enumerate().skip(1) {
            if i % 2 == 0 {
                let (x, y) = at(step);
                canvas.round_rect(x - 2.0, y - 2.0, 4.0, 4.0, 2.0, ARC);
            }
        }
        let Some(&last) = jump.path.last() else {
            continue;
        };
        let (x, y) = at(last);
        match jump.end {
            End::Lands => {
                canvas.outline(x - 8.0, y - 8.0, 16.0, 16.0, 8.0, 4.0, None, RIM);
                canvas.outline(x - 7.0, y - 7.0, 14.0, 14.0, 7.0, 2.5, None, BRIGHT);
            }
            End::Dies => {
                for (width, colour) in [(5.0, RIM), (3.0, DIES)] {
                    canvas.line((x - 7.0, y - 7.0), (x + 7.0, y + 7.0), width, None, colour);
                    canvas.line((x - 7.0, y + 7.0), (x + 7.0, y - 7.0), width, None, colour);
                }
            }
        }
    }
}

/// What each mark means, under the cavern: a line from level 2, a second
/// at level 3 and a third at level 4.
fn key(fonts: &mut Fonts, canvas: &mut Canvas, level: u8, left: f32, y: f32) {
    type Swatch = fn(&mut Canvas, f32, f32);
    let row = |fonts: &mut Fonts, canvas: &mut Canvas, y: f32, entries: &[(Swatch, &str)]| {
        let mut x = left;
        for (swatch, word) in entries {
            swatch(canvas, x, y);
            let s = [span(word, 12.0, Weight::Regular, SOFT)];
            fonts.text(Some(canvas), x + 18.0, y - 2.0, None, 1.0, &s);
            x += 18.0 + fonts.measure(&s) + 16.0;
        }
    };
    row(
        fonts,
        canvas,
        y,
        &[
            (
                |c, x, y| c.outline(x, y, 12.0, 12.0, 6.0, 2.0, None, BRIGHT),
                "item",
            ),
            (
                |c, x, y| c.outline(x, y, 12.0, 12.0, 2.0, 2.0, None, BRIGHT),
                "portal",
            ),
            (|c, x, y| c.round_rect(x, y, 12.0, 12.0, 2.0, SOFT), "Willy"),
        ],
    );
    if level >= 3 {
        row(
            fonts,
            canvas,
            y + 24.0,
            &[
                (
                    |c, x, y| c.round_rect(x, y, 12.0, 12.0, 2.0, DANGER),
                    "kills",
                ),
                (
                    |c, x, y| {
                        c.round_rect(x, y, 12.0, 12.0, 2.0, FLOOR);
                        c.round_rect(x, y, 12.0, 4.0, 1.0, CRUMBLE);
                    },
                    "crumbles",
                ),
                (
                    |c, x, y| {
                        c.triangle(
                            [(x + 11.0, y + 6.0), (x + 3.0, y + 1.0), (x + 3.0, y + 11.0)],
                            CONVEYOR,
                        );
                    },
                    "conveyor",
                ),
                (
                    |c, x, y| {
                        c.line((x, y + 6.0), (x + 12.0, y + 6.0), 2.0, None, PATROL);
                        c.round_rect(x, y, 2.0, 12.0, 1.0, PATROL);
                        c.round_rect(x + 10.0, y, 2.0, 12.0, 1.0, PATROL);
                    },
                    "guardian's path",
                ),
            ],
        );
    }
    if level >= 4 {
        row(
            fonts,
            canvas,
            y + 48.0,
            &[
                (
                    |c, x, y| {
                        c.round_rect(x, y + 4.0, 4.0, 4.0, 2.0, ARC);
                        c.round_rect(x + 8.0, y + 4.0, 4.0, 4.0, 2.0, ARC);
                    },
                    "a jump",
                ),
                (
                    |c, x, y| c.outline(x, y, 12.0, 12.0, 6.0, 2.5, None, BRIGHT),
                    "lands",
                ),
                (
                    |c, x, y| {
                        c.line((x, y), (x + 12.0, y + 12.0), 3.0, None, DIES);
                        c.line((x, y + 12.0), (x + 12.0, y), 3.0, None, DIES);
                    },
                    "dies",
                ),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A memory whose air has `passes` passes left.
    fn with_air(passes: u32) -> Vec<u8> {
        use manicminer::facts::{AIR_EMPTY, at};
        let mut mem = vec![0u8; 0x10000];
        // The last pass is the clock's wrap at no air left.
        let q = passes - 1;
        mem[usize::from(at::AIR)] = AIR_EMPTY + (q / 64) as u8;
        mem[usize::from(at::CLOCK)] = (q % 64) as u8 * 4;
        mem
    }

    #[test]
    fn the_air_is_whole_seconds_once_the_rate_is_known_and_only_counts_down() {
        let mut follow = Follow::default();
        let mut mem = with_air(1150);
        assert_eq!(follow.frame(&mem, true, true, false), View::default());
        // A pass every four frames: 12.5 a second.
        let mut last = None;
        for i in 0..2000u32 {
            let passed = i % 4 == 0;
            if passed && i > 0 {
                mem = with_air(guide::read(&mem).air_passes - 1);
            }
            let view = follow.frame(&mem, passed, true, true);
            assert!(view.cavern.is_some());
            assert_eq!(view.air_seconds.is_some(), i + 1 >= RATE_FIRST, "frame {i}");
            if let (Some(before), Some(now)) = (last, view.air_seconds) {
                assert!(now <= before, "frame {i}: {before} then {now}");
            }
            last = view.air_seconds.or(last);
        }
        // 1150 passes less 500 at 12.5 a second.
        assert_eq!(last, Some(52));
        // A death: the loop stops, the panel and the figure stay.
        for _ in 0..200 {
            let view = follow.frame(&mem, false, false, true);
            assert_eq!(view.air_seconds, last);
        }
        // The air back up: the figure with it.
        let view = follow.frame(&with_air(1150), false, true, true);
        assert_eq!(view.air_seconds, Some(92));
        assert_eq!(
            follow.frame(&mem, true, true, false),
            View::default(),
            "and at the title it starts over"
        );
    }
}
