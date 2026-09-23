//! The guidance panel beside the picture (#153), as Starquake's: the level
//! in its corner, and what the game shows made readable. Level 1 names the
//! cavern and says the air in seconds, the items left and whether the
//! portal is open; level 2 draws the cavern's cells with Willy, the items
//! ringed and the portal outlined; level 3 adds what can hurt him. It is
//! drawn from [`guide::read`], what the game keeps in memory, and nothing
//! else.

use std::collections::VecDeque;

use manicminer::guide::{self, COLUMNS, Cavern, Patrol, ROWS, Tile};
use sidekick_frontend::overlay::{self, HEIGHT, PICTURE_W};
use sidekick_frontend::text::{Canvas, Fonts, Rgb, Span, Weight};

use crate::frontend::PANEL_W;
use crate::picker::LEVELS;

const PANEL: Rgb = [0x0f, 0x11, 0x17];
const RULE: Rgb = [0x22, 0x26, 0x2f];
const LABEL: Rgb = [0x6d, 0x73, 0x85];
const BRIGHT: Rgb = [0xe6, 0xe8, 0xee];
const SOFT: Rgb = [0xaa, 0xb0, 0xbf];
const QUIET: Rgb = [0x5a, 0x60, 0x72];
const CELLS: Rgb = [0x14, 0x17, 0x1f];
const FLOOR: Rgb = [0x2c, 0x38, 0x58];
const WALL: Rgb = [0x4a, 0x50, 0x62];
const DANGER: Rgb = [0xe0, 0x67, 0x6f];
const CRUMBLE: Rgb = [0xf5, 0xb8, 0x4b];
const CONVEYOR: Rgb = [0x7f, 0xd1, 0xc7];
const PATROL: Rgb = [0xc9, 0x8b, 0xff];
const OPEN: Rgb = [0x6f, 0xd0, 0x8f];
const RIM: Rgb = [0x05, 0x06, 0x08];

/// A cell of the cavern on the panel, in layout units: 32 of them fill the
/// panel's width less its margins.
const CELL: f32 = 12.0;

/// What the panel shows: the cavern being played, and how fast its main
/// loop is running, or nothing outside a game.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub cavern: Option<Cavern>,
    /// Passes of the main loop a second, once enough are counted.
    pub rate: Option<f32>,
}

impl View {
    /// The air left in whole seconds, when the rate is known.
    #[must_use]
    pub fn air_seconds(&self) -> Option<u32> {
        let cavern = self.cavern.as_ref()?;
        let rate = self.rate?;
        Some((cavern.air_passes as f32 / rate).floor() as u32)
    }
}

/// How many frames the main loop's rate is counted over: ten seconds.
const RATE_FRAMES: usize = 500;
/// How many before a rate is given at all: two seconds.
const RATE_FIRST: usize = 100;

/// Follows the game frame by frame for the panel: how often the main loop
/// comes round in the cavern being played, and what [`guide::read`] reads.
#[derive(Default)]
pub struct Follow {
    /// The main loop's passes in each of the last frames, in this cavern.
    passes: VecDeque<u8>,
    cavern: Option<u8>,
}

impl Follow {
    /// After a frame: whether the main loop ran in it (`passed`), and
    /// whether a game is being played (`in_game`).
    pub fn frame(&mut self, mem: &[u8], passed: bool, in_game: bool) -> View {
        if !in_game {
            self.passes.clear();
            self.cavern = None;
            return View::default();
        }
        let cavern = guide::read(mem);
        if self.cavern != Some(cavern.number) {
            self.passes.clear();
            self.cavern = Some(cavern.number);
        }
        self.passes.push_back(u8::from(passed));
        if self.passes.len() > RATE_FRAMES {
            self.passes.pop_front();
        }
        let rate = (self.passes.len() >= RATE_FIRST).then(|| {
            let passes: u32 = self.passes.iter().map(|&p| u32::from(p)).sum();
            passes as f32 * 50.0 / self.passes.len() as f32
        });
        View {
            cavern: Some(cavern),
            rate: rate.filter(|&r| r > 0.0),
        }
    }
}

fn span(text: &str, size: f32, weight: Weight, colour: Rgb) -> Span<'_> {
    Span {
        text,
        size,
        weight,
        colour,
    }
}

/// A small label with its letters spread out, as Starquake's panel has.
fn spaced(fonts: &mut Fonts, canvas: &mut Canvas, mut x: f32, y: f32, text: &str, colour: Rgb) {
    let mut buf = [0u8; 4];
    for c in text.chars() {
        let s = span(c.encode_utf8(&mut buf), 11.0, Weight::SemiBold, colour);
        fonts.text(Some(canvas), x, y, None, 1.0, std::slice::from_ref(&s));
        x += fonts.advance(c, 11.0, Weight::SemiBold) + 11.0 * 0.14;
    }
}

fn spaced_width(fonts: &Fonts, text: &str) -> f32 {
    text.chars()
        .map(|c| fonts.advance(c, 11.0, Weight::SemiBold) + 11.0 * 0.14)
        .sum::<f32>()
        - 11.0 * 0.14
}

/// Draws the panel at guidance `level` beside the picture.
pub fn draw(fonts: &mut Fonts, canvas: &mut Canvas, view: &View, level: u8) {
    let window_w = overlay::width(PANEL_W);
    let left = PICTURE_W + 24.0;
    let right = window_w - 24.0;
    canvas.round_rect(PICTURE_W, 0.0, window_w - PICTURE_W, HEIGHT, 0.0, PANEL);
    canvas.round_rect(PICTURE_W, 0.0, 1.0, HEIGHT, 0.0, RULE);

    // The level in the corner opposite the label, with its name under it.
    spaced(fonts, canvas, left, 26.0, "GUIDANCE", LABEL);
    let title = if level == 0 {
        "OFF".to_string()
    } else {
        format!("LEVEL {level}")
    };
    let tw = spaced_width(fonts, &title);
    spaced(fonts, canvas, right - tw, 26.0, &title, BRIGHT);
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
    spaced(
        fonts,
        canvas,
        left,
        top,
        &format!("CAVERN {} OF 20", cavern.number + 1),
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
        .air_seconds()
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
        spaced(fonts, canvas, x, fy, label, LABEL);
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
    spaced(fonts, canvas, left, my, "THE CAVERN", LABEL);
    let cy = my + 22.0;
    cells(canvas, cavern, level, left, cy);
    if level >= 3 {
        hazards(canvas, cavern, left, cy);
    }
    marks(canvas, cavern, left, cy);
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

/// What each mark means, under the cavern: a line from level 2, and a
/// second at level 3.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cavern(air_passes: u32) -> Cavern {
        Cavern {
            air_passes,
            ..Cavern::default()
        }
    }

    #[test]
    fn the_air_is_whole_seconds_once_the_rate_is_known() {
        let mut view = View {
            cavern: Some(cavern(1150)),
            rate: None,
        };
        assert_eq!(view.air_seconds(), None);
        view.rate = Some(11.5);
        assert_eq!(view.air_seconds(), Some(100));
        view.rate = Some(11.6);
        assert_eq!(view.air_seconds(), Some(99), "rounded down");
    }

    #[test]
    fn nothing_is_followed_outside_a_game() {
        let mem = vec![0u8; 0x10000];
        let mut follow = Follow::default();
        assert_eq!(follow.frame(&mem, true, false), View::default());
        for i in 0..RATE_FIRST {
            let view = follow.frame(&mem, i % 4 == 0, true);
            assert!(view.cavern.is_some());
            assert_eq!(view.rate.is_some(), i + 1 >= RATE_FIRST);
        }
        let rate = follow.frame(&mem, false, true).rate.unwrap();
        assert!((rate - 12.4).abs() < 0.2, "{rate}");
        assert_eq!(
            follow.frame(&mem, true, false),
            View::default(),
            "and it starts over"
        );
    }
}
