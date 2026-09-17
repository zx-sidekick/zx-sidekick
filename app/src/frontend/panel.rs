//! The guidance panel beside the game, the picker, and the note of how much
//! help a game had (#25), drawn to the approved mockups; and, over the
//! picture, the pause notice.
//!
//! Everything is drawn in the overlay's layout units (`overlay.rs`): the
//! picture takes the left `PICTURE_W`, the panel the rest. Legends follow
//! the rule on #3: a keyboard key is a squarish badge, a pad button a round
//! one, and a direction a bare arrow.

use super::gamepad;
use super::guidance::{Guidance, LEVELS, SWITCHES, Setting, is_on, switches_on};
use super::notice;
use super::overlay::{HEIGHT as WINDOW_H, PICTURE_W, WIDTH as WINDOW_W};
use super::text::{Canvas, Fonts, PadMark, Rgb, Span, Weight, palette};
use super::track::Scene;
use sidekick::map::{COLS, ROWS, Step};
use sidekick::starquake::Kind;

const PANEL: Rgb = [0x0f, 0x11, 0x17];
const RULE: Rgb = [0x22, 0x26, 0x2f];
const LABEL: Rgb = [0x6d, 0x73, 0x85];
const BRIGHT: Rgb = [0xe6, 0xe8, 0xee];
const QUIET: Rgb = [0x5a, 0x60, 0x72];
const SOFT: Rgb = [0xaa, 0xb0, 0xbf];
const DIM: Rgb = [0x08, 0x09, 0x0c];
const DIALOG: Rgb = [0x10, 0x12, 0x18];
const SELECTED: Rgb = [0x1b, 0x20, 0x30];
const ACCENT: Rgb = [0x8f, 0xb4, 0xff];
const ARROW: Rgb = [0x4a, 0x51, 0x63];
const HINT_KEY: Rgb = [0xa9, 0xaf, 0xbe];
const SWITCH_ON: Rgb = [0x2f, 0x6f, 0x4f];
const SWITCH_OFF: Rgb = [0x2a, 0x2f, 0x3b];
const ON_TEXT: Rgb = [0xea, 0xff, 0xf2];
const TITLE: Rgb = [0xf2, 0xf3, 0xf7];
const VALUE_DIM: Rgb = [0xc9, 0xcd, 0xd8];
const ACCENT_DIM: Rgb = [0x5e, 0x7f, 0xb8];
const NOTCH: Rgb = [0x26, 0x2b, 0x37];
const LABEL_FOCUSED: Rgb = [0xa9, 0xc5, 0xff];
const BUTTON_LINE: Rgb = [0x3a, 0x3f, 0x4c];
const DANGER: Rgb = [0xe0, 0x67, 0x6f];
const DANGER_FILL: Rgb = [0x2a, 0x16, 0x18];
const DANGER_TITLE: Rgb = [0xf3, 0xc6, 0xca];
const DANGER_TEXT: Rgb = [0xe0, 0xa3, 0xa8];
const TRAINING: Rgb = [0xf5, 0xb8, 0x4b];
const PAUSED: Rgb = [0x5d, 0x63, 0x72];
const CODE: Rgb = [0x7f, 0xd1, 0xc7];
const FLOOR: Rgb = [0x22, 0x2c, 0x45];
/// A room level 6 draws that has never been walked through: the same shape,
/// dimmer, so where you have been still reads at a glance (#66).
const FLOOR_UNSEEN: Rgb = [0x1a, 0x20, 0x30];
const WALL_UNSEEN: Rgb = [0x4b, 0x53, 0x68];
const MAP_DOT: Rgb = [0x17, 0x1a, 0x22];
const WALL: Rgb = [0x9a, 0xaa, 0xd0];
const HERE: Rgb = [0xe8, 0xec, 0xf4];
const PIECE: Rgb = [0xf0, 0x7a, 0xb0];
/// The items found on the map (#36): the colour says what each one does.
const ITEM_DOOR: Rgb = [0x9b, 0x8a, 0xf0];
const ITEM_PAD: Rgb = [0xf5, 0xd0, 0x4b];
const ITEM_TRADE: Rgb = [0xe6, 0xea, 0xf2];
const ITEM_EDGE: Rgb = [0x00, 0x00, 0x00];
const PIECE_ROOM: Rgb = [0x15, 0x1a, 0x26];
const PIECE_ROOM_LINE: Rgb = [0x6b, 0x75, 0x94];
const TILE: Rgb = [0x1b, 0x1f, 0x29];
const ROUTE: Rgb = [0xf5, 0xb8, 0x4b];
/// The words on the two border arrows and in their legend (#44).
const PIECE_WORD: &str = "item";
const CORE_WORD: &str = "core";
/// How far apart the two border arrows stand when they leave the same way.
const ARROW_APART: f32 = 22.0;
/// Where the panel's block starts under its header (#99): the CORE and
/// TELEPORTERS headings, with the core's square and the codes 22 below.
const BLOCK_TOP: f32 = 70.0;
const DELIVERED: Rgb = [0x3a, 0x3f, 0x4b];

/// The core column (#7): its tiles' size and pitch, and how many layout
/// units a pixel of a piece's graphic is.
const CODE_FILL: Rgb = [0x14, 0x25, 0x2a];

/// A training switch's row in the picker: the heading above them, the pitch
/// from one row to the next, and the line under them all saying what the
/// focused one does (#8).
const SWITCH_HEAD: f32 = 14.0;
const SWITCH_PITCH: f32 = 34.0;
const SWITCH_SAYS: f32 = 18.0;

/// What each level adds, for the picker (#3, re-cut in #69).
const ADDS: [&str; 7] = [
    "The original game, no help.",
    "The codes you have been shown, and the core's nine slots.",
    "A map of the rooms you have walked through.",
    "Items seen: pink core piece, lilac door, yellow pad key, white trade.",
    "And what is lying in the rooms you have not.",
    "Routes: pink to a missing piece, orange to the core.",
    "Every code, the whole planet, and what each room holds.",
];

/// Where the CORE OF HEROES heading goes beside the game-over screens
/// (#91): a gap under the note of how much help the game had, whose
/// Training line is at `y` with `switches` names under it, 18 apart from 20
/// below it, and never higher than it stood before the switches could be
/// four.
fn heroes_top(y: f32, switches: usize) -> f32 {
    let end = if switches == 0 {
        y
    } else {
        y + 20.0 + switches as f32 * 18.0
    };
    (end + 14.0).max(126.0)
}

/// The letters or marks the connected pad carries, for a legend (#101).
/// A confirms and B cancels on both an Xbox and a Nintendo pad — what
/// changes is the button under each — so only a PlayStation pad's marks
/// differ here; the firing button is the west one on every pad.
fn pad_hints(layout: gamepad::Layout) -> (Hint, Hint, Hint) {
    match layout {
        gamepad::Layout::PlayStation => (
            Hint::Mark(PadMark::Cross),
            Hint::Mark(PadMark::Circle),
            Hint::Mark(PadMark::Square),
        ),
        gamepad::Layout::Nintendo => (Hint::Button("A"), Hint::Button("B"), Hint::Button("Y")),
        gamepad::Layout::Xbox => (Hint::Button("A"), Hint::Button("B"), Hint::Button("X")),
    }
}

/// A legend entry: a key, a pad button, its mark, or arrows.
#[derive(Clone, Copy)]
enum Hint {
    Key(&'static str),
    Button(&'static str),
    Mark(PadMark),
    Arrows(&'static [&'static str]),
}

/// The height of a legend's badges.
const HINT_H: f32 = 22.0;

pub struct Panel {
    fonts: Fonts,
}

/// One of the game's 2 × 2 graphics, 32 bytes as [`sidekick::starquake::graphic`]
/// reads them, drawn from (`x`, `y`) at `px` units a pixel (#7, #49).
fn cells(canvas: &mut Canvas, graphic: &[u8; 32], x: f32, y: f32, px: f32, colour: Rgb) {
    for (cell, (cy, cx)) in [(0, 0), (0, 8), (8, 0), (8, 8)].into_iter().enumerate() {
        for row in 0..8 {
            let byte = graphic[cell * 8 + row];
            for bit in 0..8 {
                if byte & (0x80 >> bit) != 0 {
                    canvas.cell(
                        x + (cx + bit) as f32 * px,
                        y + (cy + row) as f32 * px,
                        px,
                        colour,
                    );
                }
            }
        }
    }
}

impl Panel {
    pub fn new() -> Panel {
        Panel {
            fonts: Fonts::load(),
        }
    }

    /// Draws the overlay: the panel, the pause notice over the picture when
    /// the game is `paused`, and the picker over everything when it is open.
    pub fn draw(&mut self, canvas: &mut Canvas, guidance: &Guidance, scene: Scene, paused: bool) {
        let left = PICTURE_W + 24.0;
        let width = WINDOW_W - PICTURE_W;
        canvas.round_rect(PICTURE_W, 0.0, width, WINDOW_H, 0.0, PANEL);
        canvas.round_rect(PICTURE_W, 0.0, 1.0, WINDOW_H, 0.0, RULE);

        if scene == Scene::GameOver {
            self.score_note(canvas, left, guidance);
        } else {
            // The level in the corner opposite the label, with its name
            // under it (#99).
            self.spaced(canvas, left, 26.0, "GUIDANCE");
            let level = guidance.level();
            let title = if level == 0 {
                "OFF".to_string()
            } else {
                format!("LEVEL {level}")
            };
            let right = WINDOW_W - 24.0;
            let title_x = right - self.spaced_width(&title);
            self.spaced_colour(canvas, title_x, 26.0, &title, 11.0, BRIGHT);
            if level >= 1 {
                let name = [span(
                    LEVELS[usize::from(level)],
                    12.0,
                    Weight::Regular,
                    SOFT,
                )];
                let w = self.fonts.measure(&name);
                self.fonts
                    .text(Some(canvas), right - w, 42.0, None, 1.0, &name);
            }
            // Everything has one place, the same at every level it shows at
            // (#99): the core's square at the top left, the codes in a rail
            // at the right, the map under the square to the panel's bottom.
            if level >= 1 {
                // Level 5 (#9, #44, #99): the first teleport and the first
                // door on each route have their codes outlined in its colour.
                let marks = if level >= 5 {
                    marks(guidance)
                } else {
                    Vec::new()
                };
                let rail_w = self.codes_rail(canvas, right, BLOCK_TOP, guidance, level, &marks);
                let px = Self::code_pixel(canvas);
                let tile = 16.0 * px + 2.0;
                let square = 3.0 * tile + 2.0 * 4.0;
                // No heading over nothing: the title screen has no core.
                if !guidance.core().is_empty() {
                    self.spaced(canvas, left, BLOCK_TOP, "CORE");
                    self.core_grid(canvas, guidance, left, BLOCK_TOP + 22.0, tile);
                }
                let foot = BLOCK_TOP + 22.0 + square;
                if level >= 5 {
                    // Standing on the square's bottom edge, over the map.
                    self.route_legend(canvas, left + square + 18.0, foot - 4.0);
                }
                let map_w = right - rail_w - 16.0 - left;
                if level >= 2 {
                    self.map(canvas, guidance, level, foot + 16.0, WINDOW_H - 24.0, map_w);
                } else {
                    let spans = [span(
                        "The map appears at level 2.",
                        13.0,
                        Weight::Regular,
                        QUIET,
                    )];
                    let w = self.fonts.measure(&spans);
                    self.fonts.text(
                        Some(canvas),
                        left + (map_w - w) / 2.0,
                        420.0,
                        None,
                        1.0,
                        &spans,
                    );
                }
            }
            let lines: &[&str] = if level == 0 {
                &["No guidance.", "Press Esc or Select to choose a level."]
            } else {
                &[]
            };
            for (i, line) in lines.iter().enumerate() {
                let spans = [span(line, 14.0, Weight::Regular, QUIET)];
                let w = self.fonts.measure(&spans);
                self.fonts.text(
                    Some(canvas),
                    PICTURE_W + (width - w) / 2.0,
                    340.0 + i as f32 * 22.4,
                    None,
                    1.0,
                    &spans,
                );
            }
        }

        // Level 5 (#9, #44, #69): an arrow in the picture's border for each
        // route that walks out of the room next, side by side when both
        // leave the same way.
        if scene == Scene::Play
            && guidance.level() >= 5
            && let Some(here) = guidance.room()
        {
            let leaving = |route: Option<&[Step]>| {
                route
                    .and_then(|r| r.first())
                    .filter(|s| !s.teleport)
                    .map(|s| s.room.wrapping_sub(here))
            };
            let (piece, core) = (leaving(guidance.route()), leaving(guidance.core_route()));
            let apart = if piece.is_some() && piece == core {
                ARROW_APART
            } else {
                0.0
            };
            if let Some(step) = core {
                self.border_arrow(canvas, step, CORE_WORD, ROUTE, apart);
            }
            if let Some(step) = piece {
                self.border_arrow(canvas, step, PIECE_WORD, PIECE, -apart);
            }
        }
        if paused && !guidance.picker_open() {
            notice::draw(&mut self.fonts, canvas, guidance.pad());
        }
        if guidance.picker_open() {
            self.picker(canvas, guidance);
        }
    }

    /// Level 2 (#5): the planet between `top` and `bottom`, a room to a
    /// square. Every room is a faint dot; visited rooms join into floor, with
    /// a line along each edge that has no opening, so an opening is a gap in
    /// the wall, and walls inside a divided room, dashed where a door divides
    /// it. The teleporters seen are diamonds and the room Blob is in is
    /// marked. From level 3 (#6, #36), so is every room holding a core piece
    /// still needed and every item found, from level 4 (#66) in the rooms
    /// never walked through as well, one of those outlined so the mark has
    /// somewhere to sit; level 5 (#9, #44) draws the routes and level 6
    /// (#66) the whole planet.
    fn map(
        &mut self,
        canvas: &mut Canvas,
        guidance: &Guidance,
        level: u8,
        top: f32,
        bottom: f32,
        width: f32,
    ) {
        let (cols, rows) = (f32::from(COLS), f32::from(ROWS));
        // 18 units a room as in the mockup, smaller when the height or the
        // width the codes' rail leaves (#49) does not run to it.
        let pitch = ((bottom - top) / rows)
            .min(width / cols)
            .floor()
            .clamp(1.0, 18.0);
        let unit = pitch / 18.0;
        // The map sits at the panel's left margin, with the codes' rail
        // to its right (#49). What is lying in a room walked through shows
        // from level 3, and what is lying anywhere else from level 4 (#66).
        let seen_marks = level >= 3;
        let all_marks = level >= 4;
        // A room never walked through that holds something worth marking:
        // a piece the core still wants, or any other item lying in it.
        let holds = |g: &Guidance, room: u16| {
            g.piece(room) || g.items().iter().any(|f| f.room == room && !f.seen)
        };
        let x0 = PICTURE_W + 24.0;
        let rooms = COLS * ROWS;
        let at = |room: u16| {
            (
                x0 + f32::from(room % COLS) * pitch,
                top + f32::from(room / COLS) * pitch,
            )
        };

        // Level 6 draws the whole planet, room by room, whether or not it
        // has been walked through (#66).
        let whole = level >= 6;
        for room in 0..rooms {
            let (x, y) = at(room);
            if guidance.visited(room) {
                canvas.round_rect(x, y, pitch, pitch, 0.0, FLOOR);
            } else if whole {
                canvas.round_rect(x, y, pitch, pitch, 0.0, FLOOR_UNSEEN);
            } else if all_marks && holds(guidance, room) {
                let (inset, size) = (2.5 * unit, pitch - 5.0 * unit);
                let (x, y) = (x + inset, y + inset);
                canvas.round_rect(x, y, size, size, 2.0 * unit, PIECE_ROOM);
                let dash = Some(2.5 * unit);
                canvas.outline(x, y, size, size, 2.0 * unit, 1.0, dash, PIECE_ROOM_LINE);
            } else {
                let dot = 8.0 * unit;
                let inset = (pitch - dot) / 2.0;
                canvas.round_rect(x + inset, y + inset, dot, dot, 2.0 * unit, MAP_DOT);
            }
        }
        // Walls after all the floor, so no floor covers them.
        let (line, overhang) = (2.0, 1.0);
        for room in (0..rooms).filter(|&r| guidance.visited(r) || whole) {
            let (x, y) = at(room);
            let wall = if guidance.visited(room) {
                WALL
            } else {
                WALL_UNSEEN
            };
            let open = guidance
                .openings()
                .get(room as usize)
                .copied()
                .unwrap_or_default();
            let long = pitch + 2.0 * overhang;
            if !open.up {
                canvas.round_rect(x - overhang, y - overhang, long, line, 0.0, wall);
            }
            if !open.down {
                canvas.round_rect(x - overhang, y + pitch - overhang, long, line, 0.0, wall);
            }
            if !open.left {
                canvas.round_rect(x - overhang, y - overhang, line, long, 0.0, wall);
            }
            if !open.right {
                canvas.round_rect(x + pitch - overhang, y - overhang, line, long, 0.0, wall);
            }
            // Walls inside, where they stand: the solid cells between two
            // openings, a door's or a pad's every other cell (#43).
            let (cw, ch) = (pitch / 32.0, pitch / 18.0);
            for r in 0..18 {
                for c in 0..32 {
                    let d = open.divides;
                    if !d.wall(r, c) || (d.door(r, c) && (r + c) % 2 == 1) {
                        continue;
                    }
                    canvas.round_rect(
                        x + c as f32 * cw,
                        y + r as f32 * ch,
                        cw.max(1.0),
                        ch.max(1.0),
                        0.0,
                        wall,
                    );
                }
            }
        }
        // Level 5 (#9, #44, #69): the routes, through the centres of the
        // rooms walked, above the floor and walls and below the markers: to
        // the core in orange and to the nearest piece in pink. Where both take
        // the same step they run side by side, thinner, so neither hides
        // the other. A teleport step jumps, so no line joins it.
        if guidance.level() >= 5
            && let Some(here) = guidance.room()
        {
            let centre = |room: u16| {
                let (x, y) = at(room);
                (x + pitch / 2.0, y + pitch / 2.0)
            };
            let (piece, core) = (
                walked_steps(here, guidance.route()),
                walked_steps(here, guidance.core_route()),
            );
            for (steps, other, colour, side) in
                [(&core, &piece, ROUTE, 1.0), (&piece, &core, PIECE, -1.0)]
            {
                for &(a, b) in steps {
                    let shared = other.contains(&(a, b)) || other.contains(&(b, a));
                    let (width, off) = if shared {
                        (2.6 * unit, side * 1.8 * unit)
                    } else {
                        (3.0 * unit, 0.0)
                    };
                    // Across the step: down for a step sideways, right for one up or down.
                    let (ox, oy) = if a.abs_diff(b) == 1 {
                        (0.0, off)
                    } else {
                        (off, 0.0)
                    };
                    let ((ax, ay), (bx, by)) = (centre(a), centre(b));
                    // Level 5 (#10): dashed into or out of a room not yet
                    // visited, as the mockup on #25 has it.
                    let dash = (!guidance.visited(a) || !guidance.visited(b)).then_some(3.0 * unit);
                    stroke(
                        canvas,
                        (ax + ox, ay + oy),
                        (bx + ox, by + oy),
                        width,
                        dash,
                        colour,
                    );
                }
            }
        }
        for seen in guidance.teleporters() {
            let (x, y) = at(seen.room % rooms);
            let (cx, cy, r) = (x + pitch / 2.0, y + pitch / 2.0, 5.0 * unit);
            canvas.triangle([(cx - r, cy), (cx, cy - r), (cx + r, cy)], CODE);
            canvas.triangle([(cx - r, cy), (cx, cy + r), (cx + r, cy)], CODE);
        }
        if let Some(room) = guidance.room() {
            let (x, y) = at(room);
            let (outer, inner) = (3.0 * unit, 6.0 * unit);
            let size = |inset: f32| pitch - 2.0 * inset;
            canvas.round_rect(
                x + outer,
                y + outer,
                size(outer),
                size(outer),
                2.0 * unit,
                HERE,
            );
            canvas.round_rect(x + inner, y + inner, size(inner), size(inner), unit, FLOOR);
        }
        // Over the room Blob is in, so a piece there still shows.
        // A piece whose room has been seen is drawn as itself below, so
        // the dot is for the rooms still unseen (#36).
        let itself = |room: u16| {
            guidance
                .items()
                .iter()
                .any(|f| f.room == room && f.piece && (f.seen || all_marks))
        };
        let marked = |room: u16| {
            guidance.piece(room)
                && !itself(room)
                && if guidance.visited(room) {
                    seen_marks
                } else {
                    all_marks
                }
        };
        for room in (0..rooms).filter(|&r| marked(r)) {
            let (x, y) = at(room);
            let r = 4.5 * unit;
            let (cx, cy) = (x + pitch / 2.0, y + pitch / 2.0);
            canvas.round_rect(cx - r, cy - r, 2.0 * r, 2.0 * r, r, PIECE);
        }
        // Levels 3 and 4 (#36, #66): every item found, drawn with the
        // game's own graphic at one screen pixel a game pixel, in the
        // colour of what it does, with a pixel of black around it so it
        // stands off the floor and off a route running beneath.
        // A game pixel is as many whole screen pixels as the room can
        // hold, leaving a little air: one at the smallest window, two or
        // three on a big screen (#36, #57).
        let room = pitch * canvas.scale;
        let steps = (room / 16.0).floor().clamp(1.0, 4.0);
        let px = steps / canvas.scale;
        for found in guidance.items() {
            if !(if found.seen { seen_marks } else { all_marks }) {
                continue;
            }
            let (x, y) = at(found.room);
            let colour = if found.piece {
                PIECE
            } else {
                match found.kind {
                    Kind::PadKey => ITEM_PAD,
                    Kind::Trade => ITEM_TRADE,
                    _ => ITEM_DOOR,
                }
            };
            let size = 16.0 * px;
            let (ix, iy) = (x + (pitch - size) / 2.0, y + (pitch - size) / 2.0);
            let lit = |row: i32, col: i32| {
                if !(0..16).contains(&row) || !(0..16).contains(&col) {
                    return false;
                }
                let (row, col) = (row as usize, col as usize);
                let (a, b) = if row < 8 {
                    (row, 8 + row)
                } else {
                    (16 + row - 8, 24 + row - 8)
                };
                let byte = if col < 8 {
                    found.graphic[a]
                } else {
                    found.graphic[b]
                };
                byte & (0x80 >> (col % 8)) != 0
            };
            // The black first, a pixel outside the graphic's own box so an
            // edge touching it is outlined too, then the graphic over it.
            for edge in [true, false] {
                for row in -1..=16i32 {
                    for col in -1..=16i32 {
                        let here = lit(row, col);
                        let draw = if edge {
                            !here && (-1..=1).any(|dr| (-1..=1).any(|dc| lit(row + dr, col + dc)))
                        } else {
                            here
                        };
                        if draw {
                            let ink = if edge { ITEM_EDGE } else { colour };
                            canvas.cell(ix + col as f32 * px, iy + row as f32 * px, px, ink);
                        }
                    }
                }
            }
        }
        // The core's end of its route, ringed in the route's colour (#44):
        // the map marks no core room otherwise.
        if guidance.level() >= 5
            && let Some(end) = guidance.core_route().and_then(|r| r.last())
        {
            let (x, y) = at(end.room);
            let (outer, inner) = (2.0 * unit, 5.0 * unit);
            let size = |inset: f32| pitch - 2.0 * inset;
            canvas.round_rect(
                x + outer,
                y + outer,
                size(outer),
                size(outer),
                3.0 * unit,
                ROUTE,
            );
            canvas.round_rect(
                x + inner,
                y + inner,
                size(inner),
                size(inner),
                1.5 * unit,
                FLOOR,
            );
        }
    }

    /// How many layout units a Spectrum pixel of a code is (#49): half a
    /// pixel of the game's picture, rounded down to whole screen pixels so
    /// every one is the same size. The picture is 3 units a pixel.
    fn code_pixel(canvas: &Canvas) -> f32 {
        (canvas.scale * 1.5).floor().max(1.0) / canvas.scale
    }

    /// Level 1 (#7, #49, #99): the core's nine holes as a square of three
    /// by three at the panel's top left, in the order the core holds them, each its own graphic
    /// from the game, in white while it is still wanted and dimmed once
    /// delivered, outlined while it is carried.
    fn core_grid(
        &mut self,
        canvas: &mut Canvas,
        guidance: &Guidance,
        left: f32,
        top: f32,
        tile: f32,
    ) {
        let px = Self::code_pixel(canvas);
        for (i, hole) in guidance.core().iter().enumerate() {
            let x = left + (i % 3) as f32 * (tile + 4.0);
            let y = top + (i / 3) as f32 * (tile + 4.0);
            canvas.round_rect(x, y, tile, tile, 3.0, TILE);
            if hole.carried {
                canvas.outline(x, y, tile, tile, 3.0, 2.0, None, HERE);
            }
            let colour = if hole.open { HERE } else { DELIVERED };
            cells(canvas, &hole.graphic, x + 1.0, y + 1.0, px, colour);
        }
    }

    /// How wide a spaced label is (#49), so the codes' rail is never
    /// narrower than its own headings.
    fn spaced_width(&mut self, label: &str) -> f32 {
        label
            .chars()
            .map(|c| self.fonts.advance(c, 11.0, Weight::SemiBold) + 11.0 * 0.14)
            .sum::<f32>()
            - 11.0 * 0.14
    }

    /// A spaced label ending at `right`, for the codes' rail (#49).
    fn spaced_right(&mut self, canvas: &mut Canvas, right: f32, y: f32, label: &str) {
        let width = self.spaced_width(label);
        self.spaced(canvas, right - width, y, label);
    }

    /// Level 1 (#4, #49, #99): the codes seen this game in a rail at the
    /// panel's right, one to a line: the teleporters' codes in the game's
    /// own letters under TELEPORTERS, then each door's three chips under
    /// DOORS, so finding a door never moves a teleporter's code. Both
    /// headings stand from the start, with "None yet" under one that has
    /// nothing. The next teleport and the next door on a route are outlined
    /// in its colour. Returns the rail's width.
    fn codes_rail(
        &mut self,
        canvas: &mut Canvas,
        right: f32,
        top: f32,
        guidance: &Guidance,
        level: u8,
        marks: &[Mark],
    ) -> f32 {
        let px = Self::code_pixel(canvas);
        let door_w = 3.0 * 16.0 * px + 4.0 + 6.0;
        let (tile_w, tile_h) = (5.0 * 8.0 * px + 6.0, 8.0 * px + 6.0);
        // Never narrower than its headings, which are right-aligned to the
        // panel's margin and would otherwise reach over the map.
        let width = door_w.max(tile_w).max(self.spaced_width("TELEPORTERS"));
        // The ones you have been shown, or every one there is at level 6.
        let (seen, doors) = guidance.codes_at(level);
        let mut y = top;
        self.spaced_right(canvas, right, y, "TELEPORTERS");
        y += 22.0;
        if seen.is_empty() {
            self.none_yet(canvas, right, y);
            y += 22.0;
        }
        for teleporter in seen {
            let x = right - tile_w;
            canvas.round_rect(x, y, tile_w, tile_h, 4.0, CODE_FILL);
            let on = marks
                .iter()
                .filter(|m| m.code == Code::Teleporter(teleporter.code));
            outlines(canvas, x, y, tile_w, tile_h, on);
            match guidance.font() {
                // The game's own letters (#49, decision 5).
                Some(font) => {
                    for (k, letter) in teleporter.code.iter().enumerate() {
                        // The font starts at the space and holds 96 letters;
                        // a code the game has not filled in yet holds bytes
                        // outside that, and they draw as nothing.
                        let Some(glyph) = usize::from(*letter)
                            .checked_sub(0x20)
                            .and_then(|i| font.get(i * 8..i * 8 + 8))
                        else {
                            continue;
                        };
                        for (row, byte) in glyph.iter().enumerate() {
                            for bit in 0..8 {
                                if byte & (0x80 >> bit) != 0 {
                                    canvas.cell(
                                        x + 3.0 + (k * 8 + bit) as f32 * px,
                                        y + 3.0 + row as f32 * px,
                                        px,
                                        CODE,
                                    );
                                }
                            }
                        }
                    }
                }
                // Until they are read, the window's own font.
                None => {
                    let text = String::from_utf8_lossy(&teleporter.code).into_owned();
                    let spans = [span(&text, 12.0, Weight::SemiBold, CODE)];
                    self.fonts
                        .text(Some(canvas), x + 3.0, y + 1.0, None, 1.0, &spans);
                }
            }
            y += tile_h + 3.0;
        }
        y += 12.0;
        self.spaced_right(canvas, right, y, "DOORS");
        y += 22.0;
        if doors.is_empty() {
            self.none_yet(canvas, right, y);
        }
        let door_h = 16.0 * px + 6.0;
        for code in doors {
            let x = right - door_w;
            canvas.round_rect(x, y, door_w, door_h, 4.0, CODE_FILL);
            let on = marks.iter().filter(|m| m.code == Code::Door(code.room));
            outlines(canvas, x, y, door_w, door_h, on);
            for (k, graphic) in code.graphics.iter().enumerate() {
                let cx = x + 3.0 + k as f32 * (16.0 * px + 2.0);
                cells(canvas, graphic, cx, y + 3.0, px, CODE);
            }
            y += door_h + 3.0;
        }
        width
    }

    /// "None yet" under a heading of the rail that has nothing to list.
    fn none_yet(&mut self, canvas: &mut Canvas, right: f32, y: f32) {
        let spans = [span("None yet", 12.0, Weight::Regular, QUIET)];
        let w = self.fonts.measure(&spans);
        self.fonts
            .text(Some(canvas), right - w, y, None, 1.0, &spans);
    }

    /// Level 5 (#44, #99): what the two lines' colours mean, each word over
    /// a sample of its line, from `x` with the samples' bottom at `foot`:
    /// "Item" for the route to the nearest missing piece, "Core" for the one
    /// to the core.
    fn route_legend(&mut self, canvas: &mut Canvas, x: f32, foot: f32) {
        let mut at = x;
        for (text, colour) in [("Item", PIECE), ("Core", ROUTE)] {
            let spans = [span(text, 12.0, Weight::SemiBold, SOFT)];
            let w = self.fonts.measure(&spans).max(34.0);
            self.fonts
                .text(Some(canvas), at, foot - 21.0, None, 1.0, &spans);
            canvas.round_rect(at, foot - 3.0, w, 3.0, 0.0, colour);
            at += w + 18.0;
        }
    }

    /// An arrow in the picture's border pointing the way a route leaves the
    /// room (#9): `step` is the room number's change, 1 right, -1 left, 16
    /// down and -16 up. A box in `colour` with `word` in it, which stays
    /// level on every edge, and a head on the side it points to (#44), so
    /// the arrows are told apart without their colours; `shift` moves it
    /// along the edge.
    fn border_arrow(
        &mut self,
        canvas: &mut Canvas,
        step: u16,
        word: &str,
        colour: Rgb,
        shift: f32,
    ) {
        let border = (PICTURE_W - 768.0) / 2.0;
        let spans = [span(word, 15.0, Weight::SemiBold, PANEL)];
        let text_w = self.fonts.measure(&spans);
        let (w, h, head) = (text_w + 16.0, 26.0, 14.0);
        // The box's centre, and the way the head points.
        let (cx, cy, dx, dy) = match step {
            1 => (
                PICTURE_W - border / 2.0 - head / 2.0,
                WINDOW_H / 2.0 + shift,
                1.0,
                0.0,
            ),
            0xFFFF => (border / 2.0 + head / 2.0, WINDOW_H / 2.0 + shift, -1.0, 0.0),
            16 => (
                PICTURE_W / 2.0 + shift,
                WINDOW_H - border / 2.0 - head / 2.0,
                0.0,
                1.0,
            ),
            0xFFF0 => (
                PICTURE_W / 2.0 + shift,
                border / 2.0 + head / 2.0,
                0.0,
                -1.0,
            ),
            _ => return,
        };
        canvas.round_rect(cx - w / 2.0, cy - h / 2.0, w, h, 4.0, colour);
        // The head's base overlaps the box by a pixel, so no seam shows.
        let (bx, by) = (cx + dx * (w / 2.0 - 1.0), cy + dy * (h / 2.0 - 1.0));
        let spread = if dx == 0.0 { w / 2.0 } else { h / 2.0 + 6.0 };
        canvas.triangle(
            [
                (bx + dx * (head + 1.0), by + dy * (head + 1.0)),
                (bx - dy * spread, by + dx * spread),
                (bx + dy * spread, by - dx * spread),
            ],
            colour,
        );
        self.fonts.text(
            Some(canvas),
            cx - text_w / 2.0,
            cy - 10.0,
            None,
            1.0,
            &spans,
        );
    }

    fn score_note(&mut self, canvas: &mut Canvas, left: f32, guidance: &Guidance) {
        let record = guidance.record();
        self.spaced(canvas, left, 26.0, "THIS GAME");
        self.fonts.text(
            Some(canvas),
            left,
            52.0,
            None,
            1.0,
            &[span("Played with", 13.0, Weight::Regular, LABEL)],
        );
        let (headline, detail) = if record.highest == 0 {
            ("No guidance".to_string(), None)
        } else {
            (
                format!("Guidance up to level {}", record.highest),
                Some(LEVELS[record.highest as usize]),
            )
        };
        self.fonts.text(
            Some(canvas),
            left,
            70.0,
            None,
            1.0,
            &[span(&headline, 13.0, Weight::SemiBold, BRIGHT)],
        );
        let mut y = 90.0;
        if let Some(detail) = detail {
            self.fonts.text(
                Some(canvas),
                left,
                y,
                None,
                1.0,
                &[span(detail, 12.0, Weight::Regular, SOFT)],
            );
            y += 24.0;
        } else {
            y += 12.0;
        }
        let used = switches_on(record.training);
        if !used.is_empty() {
            canvas.round_rect(left, y + 5.0, 8.0, 8.0, 4.0, TRAINING);
            self.fonts.text(
                Some(canvas),
                left + 18.0,
                y,
                None,
                1.0,
                &[span("Training", 13.0, Weight::Regular, BRIGHT)],
            );
            for (i, label) in used.iter().enumerate() {
                self.fonts.text(
                    Some(canvas),
                    left + 18.0,
                    y + 20.0 + i as f32 * 18.0,
                    None,
                    1.0,
                    &[span(label, 12.0, Weight::Regular, SOFT)],
                );
            }
        }
        // The table kept between runs, with the guidance each game had (#47),
        // below whatever the note above took (#91).
        let Some(kept) = guidance.high_scores() else {
            return;
        };
        let top = heroes_top(y, used.len());
        self.spaced(canvas, left, top, "CORE OF HEROES");
        let width = WINDOW_W - PICTURE_W - 48.0;
        for (i, (entry, level)) in kept.entries.iter().zip(kept.levels).enumerate() {
            let y = top + 30.0 + i as f32 * 44.0;
            let mine = guidance.this_game() == Some(i);
            if mine {
                canvas.round_rect(left - 10.0, y - 1.0, width + 20.0, 40.0, 6.0, SELECTED);
                canvas.outline(
                    left - 10.0,
                    y - 1.0,
                    width + 20.0,
                    40.0,
                    6.0,
                    1.5,
                    None,
                    ACCENT,
                );
            }
            let rank = format!("{}.", i + 1);
            let name = String::from_utf8_lossy(&entry.name).into_owned();
            let tail = format!(
                "{}  {}%",
                String::from_utf8_lossy(&entry.score),
                entry.percent
            );
            self.fonts.text(
                Some(canvas),
                left,
                y,
                None,
                1.0,
                &[span(&rank, 14.0, Weight::SemiBold, LABEL)],
            );
            self.fonts.text(
                Some(canvas),
                left + 26.0,
                y,
                None,
                1.0,
                &[span(&name, 14.0, Weight::SemiBold, BRIGHT)],
            );
            self.fonts.text(
                Some(canvas),
                left + 70.0,
                y,
                None,
                1.0,
                &[span(&tail, 14.0, Weight::Regular, SOFT)],
            );
            let (text, colour) = match level {
                None => ("As the tape had it".to_string(), QUIET),
                Some(0) => ("No guidance".to_string(), SOFT),
                Some(l) => (format!("Level {l} \u{b7} {}", LEVELS[usize::from(l)]), SOFT),
            };
            self.fonts.text(
                Some(canvas),
                left + 26.0,
                y + 18.0,
                None,
                1.0,
                &[span(&text, 12.0, Weight::Regular, colour)],
            );
            if mine {
                let tag = [span("THIS GAME", 10.0, Weight::SemiBold, ACCENT)];
                let w = self.fonts.measure(&tag);
                self.fonts
                    .text(Some(canvas), left + width - w, y + 12.0, None, 1.0, &tag);
            }
        }
        // A game with training is not kept, so it is in no row above.
        let note = if record.training.any() {
            "This game is not kept: training mode was used."
        } else {
            "Games played with training mode are not kept."
        };
        self.fonts.text(
            Some(canvas),
            left,
            top + 30.0 + 8.0 * 44.0 + 8.0,
            None,
            1.0,
            &[span(note, 12.0, Weight::Regular, QUIET)],
        );
    }

    fn picker(&mut self, canvas: &mut Canvas, guidance: &Guidance) {
        canvas.shade(0.0, 0.0, WINDOW_W, WINDOW_H, DIM, 184);
        let focus = guidance.focus();
        // The level, the training switches, then the actions, each taller
        // while it waits for its second press.
        let actions: Vec<Setting> = guidance
            .rows()
            .into_iter()
            .filter(|r| matches!(r, Setting::EndGame | Setting::Exit))
            .collect();
        let action_h = |r: Setting| {
            if guidance.armed() == Some(r) {
                54.0
            } else {
                40.0
            }
        };
        let actions_h: f32 = actions.iter().map(|&r| action_h(r) + 4.0).sum::<f32>() - 4.0;
        // Where the switches end, and with them the rule above the actions:
        // the heading, the four rows, and the line saying what the focused
        // one does, which is always kept.
        let rule = 250.0 + SWITCH_HEAD + 4.0 * SWITCH_PITCH + SWITCH_SAYS + 6.0;
        let (w, h) = (520.0, rule + 8.0 + actions_h + 12.0 + 52.0);
        let x = (WINDOW_W - w) / 2.0;
        let y = (WINDOW_H - h) / 2.0;
        canvas.round_rect(x, y, w, h, 12.0, palette::LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0, DIALOG);

        self.fonts.text(
            Some(canvas),
            x + 28.0,
            y + 20.0,
            None,
            1.0,
            &[span("Guidance", 19.0, Weight::SemiBold, BRIGHT)],
        );
        let paused = [span("The game is paused", 12.0, Weight::Regular, LABEL)];
        let pw = self.fonts.measure(&paused);
        self.fonts.text(
            Some(canvas),
            x + w - 28.0 - pw,
            y + 27.0,
            None,
            1.0,
            &paused,
        );

        let (level, training) = guidance.picked();

        // The guidance level: a number and a name, the notches, and what it adds.
        let (rx, rw) = (x + 12.0, w - 24.0);
        let top = y + 56.0;
        let focused = focus == Setting::Level;
        self.setting_box(canvas, rx, top, rw, 184.0, focused, "GUIDANCE LEVEL");
        let top_level = LEVELS.len() as u8 - 1;
        self.arrows(
            canvas,
            rx,
            rw,
            top + 69.0,
            focused,
            level > 0,
            level < top_level,
        );
        let value = if focused { TITLE } else { VALUE_DIM };
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 30.0,
            &[span(&level.to_string(), 40.0, Weight::SemiBold, value)],
        );
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 80.0,
            &[span(LEVELS[level as usize], 17.0, Weight::SemiBold, value)],
        );
        // One notch a level above Off, so the bar says how many there are
        // rather than a number fixed when there were five (#66).
        let notches = LEVELS.len() as u8 - 1;
        let (nx, nw, gap) = (rx + 16.0, rw - 32.0, 6.0);
        let step = (nw - f32::from(notches - 1) * gap) / f32::from(notches);
        for i in 1..=notches {
            let colour = match (i <= level, focused) {
                (true, true) => ACCENT,
                (true, false) => ACCENT_DIM,
                (false, _) => NOTCH,
            };
            canvas.round_rect(
                nx + f32::from(i - 1) * (step + gap),
                top + 116.0,
                step,
                8.0,
                3.0,
                colour,
            );
        }
        self.fonts.text(
            Some(canvas),
            nx,
            top + 130.0,
            None,
            1.0,
            &[span("less help", 11.0, Weight::Regular, PAUSED)],
        );
        let more = [span("more help", 11.0, Weight::Regular, PAUSED)];
        let mw = self.fonts.measure(&more);
        self.fonts
            .text(Some(canvas), nx + nw - mw, top + 130.0, None, 1.0, &more);
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 152.0,
            &[span(ADDS[level as usize], 13.0, Weight::Regular, HINT_KEY)],
        );

        // Training mode: four switches, a row each (#8).
        let mut top = y + 250.0;
        self.spaced(canvas, rx + 14.0, top, "TRAINING");
        top += SWITCH_HEAD;
        let mut says = None;
        for (row, label, does) in SWITCHES {
            let focused = focus == row;
            let on = is_on(row, training);
            let rh = SWITCH_PITCH - 4.0;
            if focused {
                canvas.round_rect(rx, top, rw, rh, 8.0, ACCENT);
                canvas.round_rect(rx + 2.0, top + 2.0, rw - 4.0, rh - 4.0, 6.0, SELECTED);
            }
            self.fonts.text(
                Some(canvas),
                rx + 14.0,
                top + 7.0,
                None,
                1.0,
                &[span(
                    label,
                    14.0,
                    Weight::SemiBold,
                    if focused { TITLE } else { VALUE_DIM },
                )],
            );
            // Off and On at the row's right, the one in force filled.
            let mut bx = rx + rw - 14.0;
            for (text, chosen, fill, ink) in [
                ("On", on, SWITCH_ON, ON_TEXT),
                ("Off", !on, SWITCH_OFF, BRIGHT),
            ] {
                let spans = [span(
                    text,
                    13.0,
                    Weight::SemiBold,
                    if chosen { ink } else { PAUSED },
                )];
                let tw = self.fonts.measure(&spans);
                bx -= tw + 20.0;
                if chosen {
                    canvas.round_rect(bx - 8.0, top + 5.0, tw + 16.0, 20.0, 5.0, fill);
                }
                self.fonts
                    .text(Some(canvas), bx, top + 8.0, None, 1.0, &spans);
            }
            if focused {
                says = Some(does);
            }
            top += SWITCH_PITCH;
        }
        // What the focused switch does, on a line of its own kept under them
        // all, so choosing a row never moves the rest.
        if let Some(does) = says {
            self.centred_in(
                canvas,
                rx,
                rw,
                top + 2.0,
                &[span(does, 12.0, Weight::Regular, HINT_KEY)],
            );
        }

        // The actions: pressed once, a row turns red and asks again.
        let mut ay = y + rule + 8.0;
        canvas.round_rect(x + 1.0, y + rule, w - 2.0, 1.0, 0.0, RULE);
        for &row in &actions {
            let rh = action_h(row);
            let (label, again) = match row {
                Setting::EndGame => ("End this game", "Press Enter or A again to end it"),
                _ => ("Exit Starquake", "Press Enter or A again to exit"),
            };
            let armed = guidance.armed() == Some(row);
            let focused = guidance.focus() == row;
            if armed {
                canvas.round_rect(rx, ay, rw, rh, 10.0, DANGER);
                canvas.round_rect(rx + 2.0, ay + 2.0, rw - 4.0, rh - 4.0, 8.0, DANGER_FILL);
            } else if focused {
                canvas.round_rect(rx, ay, rw, rh, 10.0, ACCENT);
                canvas.round_rect(rx + 2.0, ay + 2.0, rw - 4.0, rh - 4.0, 8.0, SELECTED);
            }
            let colour = if armed {
                DANGER_TITLE
            } else if focused {
                TITLE
            } else {
                VALUE_DIM
            };
            self.fonts.text(
                Some(canvas),
                rx + 16.0,
                ay + 11.0,
                None,
                1.0,
                &[span(label, 15.0, Weight::SemiBold, colour)],
            );
            if armed {
                self.fonts.text(
                    Some(canvas),
                    rx + 16.0,
                    ay + 31.0,
                    None,
                    1.0,
                    &[span(again, 12.0, Weight::Regular, DANGER_TEXT)],
                );
            }
            ay += rh + 4.0;
        }

        // What the keys do.
        let (ok, back, _) = pad_hints(guidance.pad());
        let foot = y + h - 52.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        self.hints(
            canvas,
            x + 28.0,
            foot + 16.0,
            &[
                (&[Hint::Arrows(&["\u{2191}", "\u{2193}"])], "choose"),
                (&[Hint::Arrows(&["\u{2190}", "\u{2192}"])], "change"),
                (&[Hint::Key("Enter"), ok], "OK"),
                (&[Hint::Key("Esc"), back], "cancel"),
            ],
        );

        if guidance.asking() {
            canvas.shade(x, y, w, h, DIM, 150);
            self.score_question(canvas, guidance);
        }
    }

    /// The question over the picker when leaving it would add to the score
    /// note: exactly what changed since it opened, and what the score will
    /// say. Enter or A goes ahead, Esc, B or Select cancel (#64), so there
    /// is nothing to choose between.
    fn score_question(&mut self, canvas: &mut Canvas, guidance: &Guidance) {
        let (was_level, was_training) = (guidance.level(), guidance.training());
        let (level, training) = guidance.picked();
        let record = guidance.record();
        let on_off = |on: bool| if on { "on" } else { "off" };

        let mut changes = Vec::new();
        if level != was_level {
            changes.push(format!(
                "Guidance level {was_level} \u{2192} {level}  ({})",
                LEVELS[level as usize]
            ));
        }
        for (row, label, _) in SWITCHES {
            let (was, now) = (is_on(row, was_training), is_on(row, training));
            if was != now {
                changes.push(format!("{label} {} \u{2192} {}", on_off(was), on_off(now)));
            }
        }
        // The lines above already say what changed, and the title says where
        // it shows. All that is left to say is that it cannot be taken back
        // (#64), and whether that is one thing or several.
        let newly = SWITCHES
            .into_iter()
            .filter(|&(row, _, _)| is_on(row, training) && !is_on(row, record.training))
            .count()
            + usize::from(level > record.highest);
        let explanation = if newly > 1 {
            "They stay on this game's score, even if you change them back."
        } else {
            "It stays on this game's score, even if you change it back."
        }
        .to_string();

        let w = 440.0;
        let x = (WINDOW_W - w) / 2.0;
        let inner = w - 48.0;
        // Measure the wrapped explanation before placing anything.
        let (_, explanation_h) = self.fonts.text(
            None,
            0.0,
            0.0,
            Some(inner),
            1.5,
            &[span(&explanation, 13.0, Weight::Regular, HINT_KEY)],
        );
        // The title, the changes, the explanation, and the hints line:
        // no room for buttons, since Enter or A answers it (#64).
        let h = 58.0 + 22.0 * changes.len() as f32 + 10.0 + explanation_h + 20.0 + 44.0;
        let y = (WINDOW_H - h) / 2.0;
        canvas.round_rect(x, y, w, h, 12.0, BUTTON_LINE);
        canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0, DIALOG);
        self.fonts.text(
            Some(canvas),
            x + 24.0,
            y + 20.0,
            None,
            1.0,
            &[span(
                "This will show on your score",
                19.0,
                Weight::SemiBold,
                BRIGHT,
            )],
        );
        let mut ly = y + 58.0;
        for change in &changes {
            self.fonts.text(
                Some(canvas),
                x + 24.0,
                ly,
                None,
                1.0,
                &[span(change, 14.0, Weight::SemiBold, TITLE)],
            );
            ly += 22.0;
        }
        ly += 10.0;
        self.fonts.text(
            Some(canvas),
            x + 24.0,
            ly,
            Some(inner),
            1.5,
            &[span(&explanation, 13.0, Weight::Regular, HINT_KEY)],
        );
        let (ok, back, _) = pad_hints(guidance.pad());
        let foot = y + h - 44.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        self.hints(
            canvas,
            x + 24.0,
            foot + 12.0,
            &[
                (&[Hint::Key("Enter"), ok], "go ahead"),
                (&[Hint::Key("Esc"), back], "cancel"),
            ],
        );
    }

    /// The box of one setting in the picker, outlined when highlighted, and
    /// its label.
    #[allow(
        clippy::too_many_arguments,
        reason = "a box, whether it is highlighted, and its label"
    )]
    fn setting_box(
        &mut self,
        canvas: &mut Canvas,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        focused: bool,
        label: &str,
    ) {
        if focused {
            canvas.round_rect(x, y, w, h, 10.0, ACCENT);
            canvas.round_rect(x + 2.0, y + 2.0, w - 4.0, h - 4.0, 8.0, SELECTED);
        }
        let colour = if focused { LABEL_FOCUSED } else { LABEL };
        self.spaced_colour(canvas, x + 16.0, y + 14.0, label, 12.0, colour);
    }

    /// The arrows either side of a setting, bright when they would do
    /// something.
    #[allow(
        clippy::too_many_arguments,
        reason = "where they go and which way they work"
    )]
    fn arrows(
        &mut self,
        canvas: &mut Canvas,
        x: f32,
        w: f32,
        cy: f32,
        focused: bool,
        left: bool,
        right: bool,
    ) {
        let colour = |on: bool| match (on, focused) {
            (true, true) => ACCENT,
            (true, false) => ARROW,
            (false, _) => NOTCH,
        };
        let (l, r) = (x + 16.0, x + w - 16.0);
        canvas.triangle(
            [(l, cy), (l + 14.0, cy - 8.0), (l + 14.0, cy + 8.0)],
            colour(left),
        );
        canvas.triangle(
            [(r, cy), (r - 14.0, cy - 8.0), (r - 14.0, cy + 8.0)],
            colour(right),
        );
    }

    fn centred_in(&mut self, canvas: &mut Canvas, x: f32, w: f32, y: f32, spans: &[Span]) {
        let tw = self.fonts.measure(spans);
        self.fonts
            .text(Some(canvas), x + (w - tw) / 2.0, y, None, 1.0, spans);
    }

    /// A row of legend entries: each group's keys, buttons and arrows, a
    /// slash between a key and the pad button that does the same, then what
    /// they do.
    fn hints(&mut self, canvas: &mut Canvas, mut x: f32, y: f32, groups: &[(&[Hint], &str)]) {
        for (entries, what) in groups {
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 && !matches!(entry, Hint::Arrows(_)) {
                    x += self.fonts.word(canvas, x, y, HINT_H, "/") + 5.0;
                }
                x += match *entry {
                    Hint::Key(key) => self.fonts.key_badge(canvas, x, y, HINT_H, key),
                    Hint::Button(button) => self.fonts.button_badge(canvas, x, y, HINT_H, button),
                    Hint::Mark(mark) => self.fonts.mark_badge(canvas, x, y, HINT_H, mark),
                    Hint::Arrows(arrows) => self.fonts.arrows(canvas, x, y, HINT_H, arrows),
                } + 5.0;
            }
            x += self.fonts.word(canvas, x + 2.0, y, HINT_H, what) + 22.0;
        }
    }

    /// A small label with its letters spread out.
    fn spaced(&mut self, canvas: &mut Canvas, x: f32, y: f32, text: &str) {
        self.spaced_colour(canvas, x, y, text, 11.0, LABEL);
    }

    #[allow(clippy::too_many_arguments, reason = "where, what, and how it looks")]
    fn spaced_colour(
        &mut self,
        canvas: &mut Canvas,
        mut x: f32,
        y: f32,
        text: &str,
        size: f32,
        colour: Rgb,
    ) {
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let s = span(c.encode_utf8(&mut buf), size, Weight::SemiBold, colour);
            self.fonts
                .text(Some(canvas), x, y, None, 1.0, std::slice::from_ref(&s));
            x += self.fonts.advance(c, size, Weight::SemiBold) + size * 0.14;
        }
    }
}

/// A straight line `width` wide from `a` to `b`, with square ends, dashed
/// when `dash` gives the length of a dash and of a gap.
fn stroke(
    canvas: &mut Canvas,
    a: (f32, f32),
    b: (f32, f32),
    width: f32,
    dash: Option<f32>,
    colour: Rgb,
) {
    canvas.line(a, b, width, dash, colour);
}

/// A code in the rail: a teleporter's five letters, or a door's by its room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Code {
    Teleporter([u8; 5]),
    Door(u16),
}

/// A code a route needs next, outlined in the rail in the route's colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mark {
    code: Code,
    colour: Rgb,
}

/// The codes the rail outlines (#9, #44, #99): the first teleport and the
/// first security door on the piece route, then on the core route, each in
/// its route's colour; any may be missing. A door whose screen has not been
/// seen has no code in the rail below level 6, and so nothing to outline.
fn marks(guidance: &Guidance) -> Vec<Mark> {
    let teleport = |route: Option<&[Step]>| {
        let step = route?.iter().find(|s| s.teleport)?;
        let seen = guidance
            .teleporters()
            .iter()
            .find(|t| t.room == step.room)?;
        Some(Code::Teleporter(seen.code))
    };
    let [piece_door, core_door] = guidance.route_doors();
    [
        (teleport(guidance.route()), PIECE),
        (piece_door.map(Code::Door), PIECE),
        (teleport(guidance.core_route()), ROUTE),
        (core_door.map(Code::Door), ROUTE),
    ]
    .into_iter()
    .filter_map(|(code, colour)| {
        Some(Mark {
            code: code?,
            colour,
        })
    })
    .collect()
}

/// A route's outline on a code's chip at (`x`, `y`), the other route's
/// around it when both need the same one (#46).
fn outlines<'a>(
    canvas: &mut Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    marks: impl Iterator<Item = &'a Mark>,
) {
    for (i, mark) in marks.enumerate() {
        let out = i as f32 * 4.0;
        canvas.outline(
            x - out,
            y - out,
            w + 2.0 * out,
            h + 2.0 * out,
            4.0 + out,
            2.0,
            None,
            mark.colour,
        );
    }
}

/// The steps a route walks, from `here`, as pairs of rooms; teleports are
/// left out.
fn walked_steps(here: u16, route: Option<&[Step]>) -> Vec<(u16, u16)> {
    let mut steps = Vec::new();
    let mut from = here;
    for step in route.unwrap_or_default() {
        if !step.teleport {
            steps.push((from, step.room));
        }
        from = step.room;
    }
    steps
}

fn span(text: &str, size: f32, weight: Weight, colour: Rgb) -> Span<'_> {
    Span {
        text,
        size,
        weight,
        colour,
    }
}

#[cfg(test)]
mod heroes_layout {
    use super::*;

    /// The table starts under the note's last line, however many training
    /// switches it names, and still fits the panel's height (#91).
    #[test]
    fn the_table_starts_under_the_note_and_fits() {
        for y in [102.0, 114.0] {
            for switches in 0..=SWITCHES.len() {
                let top = heroes_top(y, switches);
                // The last line the note draws, and a line's height.
                let last = if switches == 0 {
                    y
                } else {
                    y + 20.0 + (switches - 1) as f32 * 18.0 + 14.0
                };
                assert!(
                    top >= last,
                    "{switches} switches from {y}: {top} under {last}"
                );
                assert!(top >= 126.0, "never higher than it stood");
                // The heading, eight entries of 44, and the closing note.
                let bottom = top + 30.0 + 8.0 * 44.0 + 8.0 + 16.0;
                assert!(
                    bottom <= WINDOW_H - 24.0,
                    "{switches} switches: ends at {bottom}"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick::machine::Training;
    use sidekick::map::{Divides, Openings, RoomSet};
    use sidekick::starquake::SeenTeleporter;

    /// Draws the overlay in `guidance`'s state over a stand-in picture, at
    /// twice the layout's size, as 0xRRGGBB pixels.
    fn render(guidance: &Guidance, scene: Scene, paused: bool) -> (Vec<u32>, usize, usize) {
        let scale = 2.0;
        let (w, h) = ((WINDOW_W * scale) as usize, (WINDOW_H * scale) as usize);
        let mut pixels = vec![0u8; w * h * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            width: w,
            height: h,
            scale,
        };
        canvas.clear_transparent();
        Panel::new().draw(&mut canvas, guidance, scene, paused);
        let picture_w = (PICTURE_W * scale) as usize;
        let rgb = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .map(|(i, p)| {
                // Composite over a stand-in picture, as the GPU would.
                let grey = if i % w < picture_w { 0x30 } else { 0 };
                let under = grey * (255 - u32::from(p[3])) / 255;
                let c = |v: u8| u32::from(v) + under;
                c(p[0]) << 16 | c(p[1]) << 8 | c(p[2])
            })
            .collect();
        (rgb, w, h)
    }

    #[test]
    fn the_panel_is_drawn_beside_the_picture_and_the_picker_over_both() {
        let (quiet, w, _) = render(&Guidance::default(), Scene::Play, false);
        let at =
            |pixels: &[u32], x: f32, y: f32| pixels[(y * 2.0) as usize * w + (x * 2.0) as usize];
        assert_eq!(at(&quiet, 100.0, 5.0), 0x30_30_30, "the picture untouched");
        assert_eq!(at(&quiet, 1300.0, 700.0), 0x0f_11_17, "the panel");
        let mut open = Guidance::default();
        open.open();
        let (picker, _, _) = render(&open, Scene::Play, false);
        assert!(at(&picker, 100.0, 5.0) < 0x30_30_30, "the picture dimmed");
        assert!(at(&picker, 1300.0, 700.0) < 0x0f_11_17, "the panel dimmed");
    }

    /// A level 4 guidance with Blob in room `here`, every room open and
    /// visited, and the given routes.
    fn routed(here: u16, piece: &[(u16, bool)], core: &[(u16, bool)]) -> Guidance {
        let mut g = Guidance::default();
        g.set_level(5);
        let open = Openings {
            left: true,
            right: true,
            up: true,
            down: true,
            ..Openings::default()
        };
        g.set_openings(vec![open; usize::from(COLS * ROWS)]);
        g.set_unvisited(&RoomSet::default());
        g.set_room(Some(here));
        let steps = |r: &[(u16, bool)]| {
            (!r.is_empty()).then(|| {
                r.iter()
                    .map(|&(room, teleport)| Step { room, teleport })
                    .collect()
            })
        };
        g.set_route(steps(piece));
        g.set_core_route(steps(core));
        g
    }

    /// How many pixels of `colour` lie in the box at layout `(x, y, w, h)`.
    fn count(pixels: &[u32], w: usize, (x, y, bw, bh): (f32, f32, f32, f32), colour: Rgb) -> usize {
        let want = u32::from(colour[0]) << 16 | u32::from(colour[1]) << 8 | u32::from(colour[2]);
        let (x0, y0) = ((x * 2.0) as usize, (y * 2.0) as usize);
        (y0..y0 + (bh * 2.0) as usize)
            .flat_map(|py| (x0..x0 + (bw * 2.0) as usize).map(move |px| py * w + px))
            .filter(|&i| pixels[i] == want)
            .count()
    }

    /// Where a room sits on the map at `level`, and the map's pitch, by the
    /// same arithmetic `draw` uses (#49: the codes' rail takes the right).
    fn map_at(_level: u8) -> (impl Fn(u16) -> (f32, f32), f32) {
        let left = PICTURE_W + 24.0;
        // A code's pixel at the tests' scale of 2, and the column's width.
        let px = 1.5;
        let col_w = ((3.0f32 * 16.0 * px + 4.0).max(5.0 * 8.0 * px) + 6.0).max(92.5);
        let map_w = WINDOW_H.mul_add(0.0, WINDOW_W - 24.0) - col_w - 16.0 - left;
        // The map starts under the core's square at every level (#99).
        let square = 3.0 * (16.0 * px + 2.0) + 8.0;
        let top = BLOCK_TOP + 22.0 + square + 16.0;
        let bottom = WINDOW_H - 24.0;
        let pitch = ((bottom - top) / f32::from(ROWS))
            .min(map_w / f32::from(COLS))
            .floor()
            .clamp(1.0, 18.0);
        (
            move |room: u16| {
                (
                    left + f32::from(room % COLS) * pitch,
                    top + f32::from(room / COLS) * pitch,
                )
            },
            pitch,
        )
    }

    #[test]
    fn a_step_both_routes_take_shows_both_colours() {
        // Blob in 200; both routes go right to 201, then apart.
        let g = routed(
            200,
            &[(201, false), (202, false)],
            &[(201, false), (217, false)],
        );
        let (pixels, w, _) = render(&g, Scene::Play, false);
        let (at, pitch) = map_at(5);
        let (x, y) = at(200);
        let between = (x + pitch * 0.7, y, pitch * 0.6, pitch);
        assert!(
            count(&pixels, w, between, PIECE) > 0,
            "the piece route on the shared step"
        );
        assert!(
            count(&pixels, w, between, ROUTE) > 0,
            "and the core route beside it"
        );
        let (x, y) = at(201);
        let apart = (x + pitch * 1.2, y + pitch * 0.3, pitch * 0.6, pitch * 0.4);
        assert!(
            count(&pixels, w, apart, PIECE) > 0,
            "only the piece route goes on to 202"
        );
        assert_eq!(count(&pixels, w, apart, ROUTE), 0);
    }

    #[test]
    fn a_step_into_a_room_not_visited_is_dashed() {
        let pink = |visited: bool| {
            let mut g = routed(200, &[(201, false)], &[]);
            g.set_level(5);
            let mut unvisited = RoomSet::default();
            unvisited.set(201, !visited);
            g.set_unvisited(&unvisited);
            let (pixels, w, _) = render(&g, Scene::Play, false);
            // The whole panel: all else in it is the same either way.
            count(
                &pixels,
                w,
                (PICTURE_W, 0.0, WINDOW_W - PICTURE_W, WINDOW_H),
                PIECE,
            )
        };
        let (solid, dashed) = (pink(true), pink(false));
        assert!(dashed < solid, "the step with gaps: {dashed} of {solid}");
        assert!(solid - dashed < 200, "and still drawn: {dashed} of {solid}");
    }

    #[test]
    fn each_route_leaving_the_room_has_its_own_border_arrow() {
        let border = (PICTURE_W - 768.0) / 2.0;
        let right_edge = (PICTURE_W - border, 0.0, border, WINDOW_H);
        let bottom_edge = (0.0, WINDOW_H - border, PICTURE_W, border);
        // Both leave to the right: two arrows on that edge, none below.
        let g = routed(200, &[(201, false)], &[(201, false)]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert!(count(&pixels, w, right_edge, PIECE) > 0);
        assert!(count(&pixels, w, right_edge, ROUTE) > 0);
        assert_eq!(
            count(&pixels, w, bottom_edge, PIECE) + count(&pixels, w, bottom_edge, ROUTE),
            0
        );
        // The core route down, the piece route right: an arrow on each edge.
        let g = routed(200, &[(201, false)], &[(216, false)]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert!(count(&pixels, w, right_edge, PIECE) > 0);
        assert_eq!(count(&pixels, w, right_edge, ROUTE), 0);
        assert!(count(&pixels, w, bottom_edge, ROUTE) > 0);
        // No core route: only the piece's arrow.
        let g = routed(200, &[(201, false)], &[]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert_eq!(count(&pixels, w, (0.0, 0.0, PICTURE_W, WINDOW_H), ROUTE), 0);
    }

    #[test]
    fn each_route_names_its_first_teleport_in_its_colour() {
        let seen = |g: &mut Guidance| {
            g.set_teleporters(&[
                SeenTeleporter {
                    room: 300,
                    code: *b"AAAAA",
                },
                SeenTeleporter {
                    room: 400,
                    code: *b"BBBBB",
                },
            ]);
        };
        let jump = |code: &[u8; 5], colour| Mark {
            code: Code::Teleporter(*code),
            colour,
        };
        let mut g = routed(200, &[(201, false), (300, true)], &[(400, true)]);
        seen(&mut g);
        assert_eq!(
            marks(&g),
            [jump(b"AAAAA", PIECE), jump(b"BBBBB", ROUTE)],
            "both routes' teleports, the piece route's first"
        );
        let mut g = routed(200, &[(300, true)], &[(300, true)]);
        seen(&mut g);
        assert_eq!(
            marks(&g),
            [jump(b"AAAAA", PIECE), jump(b"AAAAA", ROUTE)],
            "the same teleport for both: named twice, one outline each"
        );
        let mut g = routed(200, &[(201, false)], &[(400, true)]);
        seen(&mut g);
        assert_eq!(
            marks(&g),
            [jump(b"BBBBB", ROUTE)],
            "only the core route teleports"
        );
        let mut g = routed(200, &[(201, false)], &[]);
        seen(&mut g);
        assert!(marks(&g).is_empty(), "no teleport, nothing named");
    }

    #[test]
    fn each_route_names_the_first_door_it_has_to_pass() {
        let door = |room| Mark {
            code: Code::Door(room),
            colour: ROUTE,
        };
        let mut g = routed(200, &[(201, false)], &[(216, false)]);
        g.set_route_doors([None, Some(210)]);
        assert_eq!(marks(&g), [door(210)], "the core route's door");
        g.set_route_doors([Some(210), Some(210)]);
        assert_eq!(
            marks(&g),
            [
                Mark {
                    code: Code::Door(210),
                    colour: PIECE
                },
                door(210)
            ],
            "the same door for both: named twice, one outline each"
        );
        g.set_route_doors([None, None]);
        assert!(marks(&g).is_empty(), "no door, nothing named");
    }

    #[test]
    fn a_code_the_game_has_not_filled_in_draws_nothing() {
        // The game leaves zero bytes in a code until it names it, and the
        // font starts at the space: they must not be looked up (#49).
        let mut g = Guidance::default();
        g.set_level(1);
        g.set_font(&[0xFF; 96 * 8]);
        g.set_teleporters(&[SeenTeleporter {
            room: 0,
            code: [0, 0x1F, b'A', 0x7F, 0xFF],
        }]);
        let (pixels, w, h) = render(&g, Scene::Play, false);
        assert_eq!(pixels.len(), w * h, "it drew without panicking");
    }

    #[test]
    fn a_chip_both_routes_use_has_both_outlines() {
        let mut g = routed(200, &[(300, true)], &[(300, true)]);
        g.set_teleporters(&[SeenTeleporter {
            room: 300,
            code: *b"AAAAA",
        }]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        // The codes' rail, down the panel's right (#49).
        let chips = (WINDOW_W - 130.0, 60.0, 130.0, WINDOW_H - 60.0);
        assert!(
            count(&pixels, w, chips, PIECE) > 0,
            "the piece route's outline"
        );
        assert!(
            count(&pixels, w, chips, ROUTE) > 0,
            "and the core route's around it"
        );
        let mut g = routed(200, &[(300, true)], &[]);
        g.set_teleporters(&[SeenTeleporter {
            room: 300,
            code: *b"AAAAA",
        }]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert_eq!(count(&pixels, w, chips, ROUTE), 0, "one route, one outline");
    }

    /// The pixels of the box at layout `(x, y, w, h)`.
    fn cut(pixels: &[u32], w: usize, (x, y, bw, bh): (f32, f32, f32, f32)) -> Vec<u32> {
        let (x0, y0) = ((x * 2.0) as usize, (y * 2.0) as usize);
        (y0..y0 + (bh * 2.0) as usize)
            .flat_map(|py| (x0..x0 + (bw * 2.0) as usize).map(move |px| pixels[py * w + px]))
            .collect()
    }

    #[test]
    fn what_each_level_adds_fits_the_pickers_box() {
        // The picker is 520 wide and the level's box 24 narrower, with 14
        // of padding either side.
        let mut fonts = Fonts::load();
        for line in ADDS {
            let w = fonts.measure(&[span(line, 13.0, Weight::Regular, HINT_KEY)]);
            assert!(w <= 520.0 - 24.0 - 28.0, "{line}: {w}");
        }
    }

    fn door_code(room: u16) -> crate::frontend::guidance::DoorCode {
        crate::frontend::guidance::DoorCode {
            room,
            chips: [9, 10, 11],
            graphics: [[0x5A; 32]; 3],
        }
    }

    #[test]
    fn the_core_and_the_teleporters_stand_still_whatever_the_level_and_the_doors() {
        use crate::frontend::guidance::Hole;
        let game = |level: u8, doors: usize| {
            let mut g = routed(200, &[(201, false)], &[(216, false)]);
            g.set_level(level);
            g.set_core(
                (0..9)
                    .map(|i| Hole {
                        graphic: [0xA5; 32],
                        open: i % 2 == 0,
                        carried: i == 0,
                    })
                    .collect(),
            );
            g.set_teleporters(&[SeenTeleporter {
                room: 300,
                code: *b"AAAAA",
            }]);
            let codes: Vec<_> = (0..doors as u16).map(|d| door_code(176 + d)).collect();
            g.set_door_codes(&codes);
            render(&g, Scene::Play, false)
        };
        let left = PICTURE_W + 24.0;
        let core = (left, BLOCK_TOP, 90.0, 22.0 + 88.0);
        let chip = (WINDOW_W - 24.0 - 96.0, BLOCK_TOP, 96.0, 22.0 + 20.0);
        let (first, w, _) = game(1, 0);
        assert!(
            count(&first, w, core, HERE) > 0,
            "the core's square is there"
        );
        assert!(count(&first, w, chip, CODE_FILL) > 0, "and the code's chip");
        for level in 1..=5 {
            for doors in [0, 1, 5] {
                let (pixels, w, _) = game(level, doors);
                assert_eq!(
                    cut(&pixels, w, core),
                    cut(&first, w, core),
                    "{level} {doors}"
                );
                assert_eq!(
                    cut(&pixels, w, chip),
                    cut(&first, w, chip),
                    "{level} {doors}"
                );
            }
        }
        // Level 6 lists every code instead, under the same heading.
        let (pixels, w, _) = game(6, 0);
        assert_eq!(cut(&pixels, w, core), cut(&first, w, core));
    }

    #[test]
    fn both_headings_stand_from_the_start() {
        let mut g = Guidance::default();
        g.set_level(1);
        let (none, w, _) = render(&g, Scene::Play, false);
        g.set_door_codes(&[door_code(210)]);
        let (one, _, _) = render(&g, Scene::Play, false);
        // TELEPORTERS, "None yet", DOORS: the same down to the door's chip.
        let head = (
            WINDOW_W - 24.0 - 96.0,
            BLOCK_TOP,
            96.0,
            22.0 + 22.0 + 12.0 + 20.0,
        );
        assert_eq!(cut(&none, w, head), cut(&one, w, head));
        assert!(count(&none, w, head, QUIET) > 0, "\"None yet\" is written");
        let chips = (WINDOW_W - 24.0 - 96.0, BLOCK_TOP + 76.0, 96.0, 40.0);
        assert_eq!(count(&none, w, chips, CODE_FILL), 0);
        assert!(count(&one, w, chips, CODE_FILL) > 0, "the door under DOORS");
    }

    #[test]
    fn a_door_a_route_has_to_pass_is_outlined_in_the_routes_colour() {
        let mut g = routed(200, &[(201, false)], &[(216, false)]);
        g.set_door_codes(&[door_code(176), door_code(210)]);
        let rail = (WINDOW_W - 24.0 - 100.0, BLOCK_TOP, 104.0, 300.0);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert_eq!(count(&pixels, w, rail, ROUTE), 0, "no door on the way");
        g.set_route_doors([None, Some(210)]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert!(count(&pixels, w, rail, ROUTE) > 0, "the core route's door");
        assert_eq!(count(&pixels, w, rail, PIECE), 0);
        // A door whose screen has not been seen has no code to outline.
        g.set_route_doors([Some(429), None]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert_eq!(count(&pixels, w, rail, PIECE), 0);
        // Below level 5 no route shows, so nothing is outlined.
        g.set_route_doors([None, Some(210)]);
        g.set_level(4);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        assert_eq!(count(&pixels, w, rail, ROUTE), 0);
    }

    /// A game just over at level 3 with the kept table beside it (#47),
    /// this game's entry sixth: made-up names and scores.
    fn heroes() -> Guidance {
        use crate::frontend::scores::Kept;
        use sidekick::starquake::HighScore;
        type Row = (&'static [u8; 3], &'static [u8; 6], u8, Option<u8>);
        let mut g = Guidance::default();
        g.set_level(3);
        g.new_game();
        let rows: [Row; 8] = [
            (b"STA", b"109825", 31, None),
            (b"TAR", b"093900", 23, None),
            (b"ARQ", b"082975", 19, None),
            (b"BOB", b"071540", 21, Some(0)),
            (b"RQU", b"062050", 17, None),
            (b"SQK", b"051230", 14, Some(3)),
            (b"JEN", b"048000", 12, Some(2)),
            (b"QUA", b"046125", 13, None),
        ];
        let kept = Kept {
            entries: std::array::from_fn(|i| HighScore {
                name: *rows[i].0,
                score: *rows[i].1,
                percent: rows[i].2,
            }),
            levels: std::array::from_fn(|i| rows[i].3),
        };
        g.set_high_scores(kept, Some(5));
        g
    }

    /// Level 4 with every code a game can show (#49): eight door codes,
    /// the codes `sk-check facts` reads on one game, and fifteen
    /// teleporters. Their chips come from the tape, so it needs `SQ_TAPE`.
    fn doors_seen(level: u8) -> Guidance {
        let mut g = Guidance::default();
        g.set_level(level);
        explore(&mut g, 3);
        // At level 5, everything the panel can hold at once: the core's
        // holes, the missing pieces, the items found, both routes and the
        // choice among the nearest (#49, to see the whole panel).
        if level >= 5 {
            let here = g.room().expect("a room");
            let mut pieces = RoomSet::default();
            for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
                pieces.set(row * COLS + col, true);
            }
            g.set_pieces(&pieces);
            g.set_core(made_up_core());
            g.set_piece_choice(Some(here), (2, 3));
            let step = |room: u16| Step {
                room,
                teleport: false,
            };
            g.set_route(Some(vec![
                step(here + 1),
                step(here + 2),
                step(here + 2 + COLS),
            ]));
            g.set_core_route(Some(vec![
                step(here - 1),
                step(here - 2),
                step(here - 2 - COLS),
            ]));
        }
        let letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let seen: Vec<SeenTeleporter> = (0..15u16)
            .map(|i| SeenTeleporter {
                room: i * 30,
                code: std::array::from_fn(|k| {
                    letters[(usize::from(i) * 5 + k * 3) % letters.len()]
                }),
            })
            .collect();
        g.set_teleporters(&seen);
        let Some(tape) = std::env::var_os("SQ_TAPE") else {
            return g;
        };
        let bytes = std::fs::read(tape).expect("the tape");
        let m = sidekick::Machine::from_tape(
            &bytes,
            sidekick::starquake::ENTRY_PC,
            sidekick::starquake::ENTRY_SP,
        )
        .expect("a machine");
        let graphic = |n: u8| sidekick::starquake::graphic(&m.zx.mem[..], n);
        let rooms: [(u16, [u8; 3]); 8] = [
            (176, [11, 12, 11]),
            (187, [10, 11, 12]),
            (200, [13, 12, 13]),
            (210, [11, 12, 11]),
            (265, [12, 11, 9]),
            (352, [12, 11, 12]),
            (362, [12, 11, 12]),
            (429, [10, 9, 10]),
        ];
        let codes: Vec<crate::frontend::guidance::DoorCode> = rooms
            .into_iter()
            .map(|(room, chips)| crate::frontend::guidance::DoorCode {
                room,
                chips,
                graphics: chips.map(graphic),
            })
            .collect();
        g.set_door_codes(&codes);
        g
    }

    /// Nine made-up holes, not the game's graphics: simple shapes, three
    /// filled, one carried.
    /// A few items out on the planet: some lying in rooms walked through,
    /// which level 3 marks, and some in rooms never entered, which level 4
    /// adds (#66).
    fn made_up_items(g: &Guidance) -> Vec<crate::frontend::guidance::Found> {
        use sidekick::starquake::Kind;
        let here = g.room().unwrap_or(0);
        let shape = |i: u8| {
            let mut graphic = [0u8; 32];
            for (k, b) in graphic.iter_mut().enumerate() {
                let row = (k % 8) as u8;
                *b = match (i + k as u8 / 8) % 3 {
                    0 => 0xFF >> row,
                    1 => 0x3C | (0x81 * u8::from(row.is_multiple_of(2))),
                    _ => 0x81 << (row % 4),
                };
            }
            graphic
        };
        [
            (here + 2, Kind::DoorCard, false, true),
            (here + COLS, Kind::PadKey, false, true),
            (here - 1, Kind::Trade, true, true),
            (7 * COLS + 4, Kind::Chip(b'2'), false, false),
            (19 * COLS + 11, Kind::Trade, true, false),
            (26 * COLS + 6, Kind::PadKey, false, false),
        ]
        .into_iter()
        .enumerate()
        .map(
            |(i, (room, kind, piece, seen))| crate::frontend::guidance::Found {
                room,
                kind,
                piece,
                graphic: shape(i as u8),
                seen,
            },
        )
        .collect()
    }

    fn made_up_core() -> Vec<crate::frontend::guidance::Hole> {
        (0..9u8)
            .map(|i| {
                let mut graphic = [0u8; 32];
                for (k, b) in graphic.iter_mut().enumerate() {
                    let row = (k % 8) as u8;
                    *b = match (i + k as u8 / 8) % 3 {
                        0 => 0xFF >> row,
                        1 => 0x3C | (0x81 * u8::from(row.is_multiple_of(2))),
                        _ => 0x81 << (row % 4),
                    };
                }
                crate::frontend::guidance::Hole {
                    graphic,
                    open: ![1, 4, 6].contains(&i),
                    carried: i == 0,
                }
            })
            .collect()
    }

    /// A made-up exploration, like the mockup's: a random walk over the
    /// map, whose steps are its only openings, with `codes` teleporters seen
    /// on the way. The codes are placeholders; the real ones are the
    /// original's text.
    fn explore(g: &mut Guidance, codes: usize) {
        let mut openings = vec![Openings::default(); usize::from(COLS * ROWS)];
        let mut unvisited = RoomSet([0xFF; 64]);
        let mut seen = Vec::new();
        let (mut col, mut row) = (7u16, 20u16);
        let mut rng = 7u32;
        for step in 0..420 {
            let room = row * COLS + col;
            unvisited.set(room, false);
            if step % 60 == 59 && seen.len() < codes {
                let letter = |k: usize| b'A' + ((seen.len() * 5 + k) % 26) as u8;
                seen.push(SeenTeleporter {
                    room,
                    code: [0, 1, 2, 3, 4].map(letter),
                });
            }
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            let (dc, dr) = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 0), (-1, 0)][rng as usize % 6];
            let (c, r) = (col as i32 + dc, row as i32 + dr);
            if !(0..i32::from(COLS)).contains(&c) || !(0..i32::from(ROWS)).contains(&r) {
                continue;
            }
            let next = r as u16 * COLS + c as u16;
            let (a, b) = (room.min(next) as usize, room.max(next) as usize);
            if dc != 0 {
                openings[a].right = true;
                openings[b].left = true;
            } else {
                openings[a].down = true;
                openings[b].up = true;
            }
            (col, row) = (c as u16, r as u16);
        }
        // Walls inside a few rooms along the walk: a bar down the middle,
        // a bar across, and a door's bar.
        let visited: Vec<usize> = (0..openings.len())
            .filter(|&r| !unvisited.contains(r as u16))
            .collect();
        let down = |door: bool| {
            let mut d = Divides::default();
            for row in 0..18 {
                d.cells[row] = 0b11 << 15;
                if door {
                    d.doors[row] = d.cells[row];
                }
            }
            d
        };
        let across = {
            let mut d = Divides::default();
            d.cells[8] = u32::MAX;
            d.cells[9] = u32::MAX;
            d
        };
        let shapes = [down(false), across, down(true)];
        for (k, room) in visited.iter().step_by(9).enumerate() {
            openings[*room].divides = shapes[k % shapes.len()];
        }
        g.set_openings(openings);
        g.set_unvisited(&unvisited);
        g.set_room(Some(row * COLS + col));
        g.set_teleporters(&seen);
    }

    /// Draws the overlay in its states to PNGs in the folder `SQ_PANEL_PNG`
    /// names, for comparing with the mockups without a window. Does nothing
    /// when it is not set.
    #[test]
    fn render_to_png() {
        let Some(out) = std::env::var_os("SQ_PANEL_PNG") else {
            return;
        };
        let out = std::path::PathBuf::from(out);
        // With a tape to hand, the codes are drawn in the game's own
        // letters, as the panel draws them in play (#49).
        let font = std::env::var_os("SQ_TAPE").map(|tape| {
            let bytes = std::fs::read(tape).expect("the tape");
            let m = sidekick::Machine::from_tape(
                &bytes,
                sidekick::starquake::ENTRY_PC,
                sidekick::starquake::ENTRY_SP,
            )
            .expect("a machine");
            sidekick::starquake::font(&m.zx.mem[..])
                .expect("the font")
                .to_vec()
        });
        let mut picker = Guidance::default();
        picker.set_level(3);
        picker.open();
        let mut record = Guidance::default();
        record.set_level(3);
        record.set_training(Training {
            time: true,
            ..Training::default()
        });
        let cases = [
            ("level0", Guidance::default(), Scene::Play, false),
            (
                "level3",
                {
                    let mut g = Guidance::default();
                    g.set_level(3);
                    g
                },
                Scene::Play,
                false,
            ),
            ("paused", Guidance::default(), Scene::Play, true),
            // The same screens as a Switch and a PlayStation pad have them
            // (#101): the letters follow the pad, and the marks are drawn.
            (
                "pad-nintendo-paused",
                {
                    let mut g = Guidance::default();
                    g.set_pad(gamepad::Layout::Nintendo);
                    g
                },
                Scene::Play,
                true,
            ),
            (
                "pad-playstation-paused",
                {
                    let mut g = Guidance::default();
                    g.set_pad(gamepad::Layout::PlayStation);
                    g
                },
                Scene::Play,
                true,
            ),
            (
                "pad-playstation-picker",
                {
                    let mut g = picker.clone();
                    g.set_pad(gamepad::Layout::PlayStation);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                // Level 1 with the core's slots, which move here in #66.
                "level1-core",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g.set_core(made_up_core());
                    let seen: Vec<SeenTeleporter> = ["ABCDE", "FGHIJ", "KLMNO"]
                        .iter()
                        .enumerate()
                        .map(|(i, code)| SeenTeleporter {
                            room: i as u16 * 40,
                            code: {
                                let mut c = [0u8; 5];
                                c.copy_from_slice(code.as_bytes());
                                c
                            },
                        })
                        .collect();
                    g.set_teleporters(&seen);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level2",
                {
                    let mut g = Guidance::default();
                    g.set_level(2);
                    explore(&mut g, 3);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level3",
                {
                    let mut g = Guidance::default();
                    g.set_level(3);
                    explore(&mut g, 3);
                    let mut pieces = RoomSet::default();
                    for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
                        pieces.set(row * COLS + col, true);
                    }
                    // One in the room Blob is in, to show its dot over the marker.
                    pieces.set(g.room().unwrap(), true);
                    g.set_pieces(&pieces);
                    g.set_core(made_up_core());
                    let items = made_up_items(&g);
                    g.set_items(&items);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                // Level 6 draws the whole planet, and every code (#66).
                "level6",
                {
                    let mut g = Guidance::default();
                    g.set_level(6);
                    explore(&mut g, 3);
                    let mut pieces = RoomSet::default();
                    for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
                        pieces.set(row * COLS + col, true);
                    }
                    g.set_pieces(&pieces);
                    g.set_core(made_up_core());
                    let items = made_up_items(&g);
                    g.set_items(&items);
                    // Every teleporter's code, not only the booths entered.
                    let seen: Vec<SeenTeleporter> = (0..15u16)
                        .map(|i| SeenTeleporter {
                            room: i * 33 + 7,
                            code: {
                                let mut c = [0u8; 5];
                                for (k, b) in c.iter_mut().enumerate() {
                                    *b = b'A' + ((i as u8 * 5 + k as u8) % 26);
                                }
                                c
                            },
                        })
                        .collect();
                    g.set_teleporters(&seen);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                // Level 4 adds what is lying in rooms never walked through.
                "level4-unseen",
                {
                    let mut g = Guidance::default();
                    g.set_level(4);
                    explore(&mut g, 3);
                    let mut pieces = RoomSet::default();
                    for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
                        pieces.set(row * COLS + col, true);
                    }
                    g.set_pieces(&pieces);
                    g.set_core(made_up_core());
                    let items = made_up_items(&g);
                    g.set_items(&items);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level4",
                {
                    let mut g = Guidance::default();
                    g.set_level(5);
                    explore(&mut g, 3);
                    let mut pieces = RoomSet::default();
                    for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
                        pieces.set(row * COLS + col, true);
                    }
                    g.set_pieces(&pieces);
                    g.set_core(made_up_core());
                    // Known steps: every open edge between two visited rooms,
                    // up and down both ways, as if walked both ways.
                    let mut known = sidekick::map::Known::default();
                    for room in 0..COLS * ROWS {
                        let o = g.openings()[usize::from(room)];
                        if g.visited(room) && o.right && g.visited(room + 1) {
                            known.walked(room, room + 1);
                        }
                        if g.visited(room)
                            && o.down
                            && room + COLS < COLS * ROWS
                            && g.visited(room + COLS)
                        {
                            known.walked(room, room + COLS);
                            known.walked(room + COLS, room);
                        }
                    }
                    // The walk's last step, into Blob's room, from the left.
                    let here = g.room().unwrap();
                    known.walked(here - 1, here);
                    let booths: Vec<u16> = g.teleporters().iter().map(|t| t.room).collect();
                    let route = known.route(here, &booths, &pieces, 199);
                    g.set_route(route);
                    // Switched to the second of the three nearest (#51).
                    g.set_piece_choice(Some(here), (2, 3));
                    // The made-up walk never reaches room 199, so a room far
                    // along it stands in for the core, as in the mockup (#44).
                    let far = (0..COLS * ROWS)
                        .filter(|&r| g.visited(r))
                        .max_by_key(|&r| {
                            (i32::from(r / COLS) - i32::from(here / COLS)).abs() * 3
                                + (i32::from(r % COLS) - i32::from(here % COLS)).abs()
                        })
                        .unwrap();
                    let mut core = RoomSet::default();
                    core.set(far, true);
                    g.set_core_route(known.route(here, &booths, &core, 999));
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level5",
                {
                    let mut g = Guidance::default();
                    g.set_level(5);
                    explore(&mut g, 3);
                    let here = g.room().unwrap();
                    let mut pieces = RoomSet::default();
                    for (col, row) in [(12, 13), (2, 24), (6, 8), (13, 29), (10, 4), (9, 21)] {
                        pieces.set(row * COLS + col, true);
                    }
                    g.set_pieces(&pieces);
                    g.set_core(made_up_core());
                    // A made-up route: three rooms along the walk, then on
                    // through rooms never visited towards the piece at (10, 4).
                    let mut steps = Vec::new();
                    let (mut col, mut row) = (here % COLS, here / COLS);
                    while (col, row) != (10, 4) {
                        if col == 10 {
                            row = if row < 4 { row + 1 } else { row - 1 };
                        } else {
                            col = if col < 10 { col + 1 } else { col - 1 };
                        }
                        steps.push(Step {
                            room: row * COLS + col,
                            teleport: false,
                        });
                    }
                    g.set_route(Some(steps));
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level2-many-codes",
                {
                    let mut g = Guidance::default();
                    g.set_level(2);
                    explore(&mut g, 7);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level1-none",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level1-codes",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    // Placeholders, not the game's codes.
                    let codes = [*b"ABCDE", *b"FGHIJ", *b"KLMNO", *b"PQRST", *b"UVWXY"];
                    let seen: Vec<SeenTeleporter> = codes
                        .iter()
                        .enumerate()
                        .map(|(i, &code)| SeenTeleporter {
                            room: i as u16 * 40,
                            code,
                        })
                        .collect();
                    g.set_teleporters(&seen);
                    g
                },
                Scene::Play,
                false,
            ),
            ("picker", picker.clone(), Scene::Play, true),
            (
                "picker-training",
                {
                    let mut g = picker.clone();
                    g.focus_down();
                    g.change(true);
                    g.focus_down();
                    g.focus_down();
                    g.change(true);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                // No harm from enemies focused, for its description's fit (#68).
                "picker-unharmed",
                {
                    let mut g = picker.clone();
                    for _ in 0..4 {
                        g.focus_down();
                    }
                    g.change(true);
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "picker-end-armed",
                {
                    let mut g = picker;
                    g.set_playing(true);
                    for _ in 0..5 {
                        g.focus_down();
                    }
                    g.enter();
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "picker-score-question",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g.open();
                    g.change(true);
                    g.change(true);
                    g.focus_down();
                    g.change(true);
                    g.enter();
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "picker-score-question-level",
                {
                    let mut g = Guidance::default();
                    g.set_level(1);
                    g.open();
                    g.change(true);
                    g.change(true);
                    g.enter();
                    g
                },
                Scene::Play,
                false,
            ),
            ("doors", doors_seen(4), Scene::Play, false),
            ("everything", doors_seen(5), Scene::Play, false),
            ("score", record, Scene::GameOver, false),
            ("heroes", heroes(), Scene::GameOver, false),
            (
                // Every training switch used: the table below them (#91).
                "heroes-training",
                {
                    let mut g = heroes();
                    g.set_level(6);
                    g.set_training(Training {
                        time: true,
                        full: true,
                        lives: true,
                        unharmed: true,
                    });
                    g
                },
                Scene::GameOver,
                false,
            ),
            ("score-none", Guidance::default(), Scene::GameOver, false),
        ];
        for (name, mut guidance, scene, paused) in cases {
            if let Some(font) = &font {
                guidance.set_font(font);
            }
            let (rgb, w, h) = render(&guidance, scene, paused);
            std::fs::write(
                out.join(format!("panel-{name}.png")),
                zx_core::png::encode(&rgb, w, h),
            )
            .unwrap();
        }
    }
}
