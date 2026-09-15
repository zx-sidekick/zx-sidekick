//! The guidance panel beside the game, the picker, and the note of how much
//! help a game had (#25), drawn to the approved mockups; and, over the
//! picture, the pause notice.
//!
//! Everything is drawn in the overlay's layout units (`overlay.rs`): the
//! picture takes the left `PICTURE_W`, the panel the rest. Legends follow
//! the rule on #3: a keyboard key is a squarish badge, a pad button a round
//! one, and a direction a bare arrow.

use super::guidance::{Choice, Guidance, LEVELS, Setting};
use super::notice;
use super::overlay::{HEIGHT as WINDOW_H, PICTURE_W, WIDTH as WINDOW_W};
use super::text::{Canvas, Fonts, Rgb, Span, Weight, palette};
use super::track::Scene;
use sidekick::map::{COLS, ROWS, Step};
use sidekick::starquake::SeenTeleporter;

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
const MAP_DOT: Rgb = [0x17, 0x1a, 0x22];
const WALL: Rgb = [0x9a, 0xaa, 0xd0];
const HERE: Rgb = [0xe8, 0xec, 0xf4];
const PIECE: Rgb = [0xf0, 0x7a, 0xb0];
const PIECE_ROOM: Rgb = [0x15, 0x1a, 0x26];
const PIECE_ROOM_LINE: Rgb = [0x6b, 0x75, 0x94];
const TILE: Rgb = [0x1b, 0x1f, 0x29];
const ROUTE: Rgb = [0xf5, 0xb8, 0x4b];
/// The words on the two border arrows and in their legend (#44).
const PIECE_WORD: &str = "item";
const CORE_WORD: &str = "core";
/// How far apart the two border arrows stand when they leave the same way.
const ARROW_APART: f32 = 22.0;
/// The room the routes' legend takes under the map (#44).
const LEGEND_H: f32 = 52.0;
const DELIVERED: Rgb = [0x3a, 0x3f, 0x4b];

/// The core column (#7): its tiles' size and pitch, and how many layout
/// units a pixel of a piece's graphic is.
const TILE_W: f32 = 40.0;
const TILE_PITCH: f32 = 46.0;
const PIECE_PIXEL: f32 = 2.25;
const CODE_FILL: Rgb = [0x14, 0x25, 0x2a];

/// What each level adds, for the picker. The levels are built in their own
/// tickets (#3); until then they say so.
const ADDS: [&str; 6] = [
    "The original game, no help.",
    "The codes of the teleporters you have seen.",
    "A map of the rooms you have visited.",
    "The missing core pieces, marked on the map.",
    "An arrow along routes you know.",
    "Not built yet: the arrow routed through the whole map.",
];

/// A legend entry: a key, a pad button, or arrows.
#[derive(Clone, Copy)]
enum Hint {
    Key(&'static str),
    Button(&'static str),
    Arrows(&'static [&'static str]),
}

/// The height of a legend's badges.
const HINT_H: f32 = 22.0;

pub struct Panel {
    fonts: Fonts,
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
            self.spaced(canvas, left, 26.0, "GUIDANCE");
            let level = guidance.level();
            let title = if level == 0 {
                "Off".to_string()
            } else {
                format!("Level {level} \u{b7} {}", LEVELS[level as usize])
            };
            self.fonts.text(
                Some(canvas),
                left,
                44.0,
                Some(width - 48.0),
                1.2,
                &[span(&title, 19.0, Weight::SemiBold, BRIGHT)],
            );
            let esc_w = self.fonts.key_width(HINT_H, "Esc");
            self.fonts
                .key_badge(canvas, WINDOW_W - 24.0 - esc_w, 24.0, HINT_H, "Esc");
            // Level 4 (#9, #44): the first teleport on each route has its
            // chip outlined in that route's colour.
            let jump = if level >= 4 {
                jumps(guidance)
            } else {
                Vec::new()
            };
            let below = if level >= 1 {
                self.teleporters(canvas, left, width - 48.0, guidance.teleporters(), &jump)
            } else {
                WINDOW_H
            };
            if level >= 4 && guidance.room().is_some() {
                self.route_line(canvas, guidance, below, &jump);
            }
            if level >= 2 {
                let explored = format!("explored {} of {} rooms", guidance.explored(), COLS * ROWS);
                self.fonts.text(
                    Some(canvas),
                    left,
                    72.0,
                    None,
                    1.0,
                    &[span(&explored, 12.0, Weight::Regular, LABEL)],
                );
                // Level 4 (#44): the two routes' legend under the map.
                let legend = if level >= 4 { LEGEND_H } else { 0.0 };
                let map_bottom =
                    self.map(canvas, guidance, level >= 3, 96.0, below - 16.0 - legend);
                if level >= 4 {
                    self.route_legend(canvas, guidance, map_bottom + 14.0);
                }
                if level >= 3 && !guidance.core().is_empty() {
                    self.core(canvas, guidance, 72.0, 96.0);
                }
            }
            let lines = match level {
                0 => ["No guidance.", "Press Esc or Select to choose a level."],
                1 => ["The map appears at level 2.", ""],
                _ => ["", ""],
            };
            for (i, line) in lines.into_iter().enumerate().filter(|(_, l)| !l.is_empty()) {
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

        // Level 4 (#9, #44): an arrow in the picture's border for each route
        // that walks out of the room next, side by side when both leave the
        // same way.
        if scene == Scene::Play
            && guidance.level() >= 4
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
            notice::draw(&mut self.fonts, canvas);
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
    /// marked. With `pieces` (level 3, #6), so is every room holding a core
    /// piece still needed, and one not visited is outlined so the mark has
    /// somewhere to sit.
    fn map(
        &mut self,
        canvas: &mut Canvas,
        guidance: &Guidance,
        pieces: bool,
        top: f32,
        bottom: f32,
    ) -> f32 {
        let (cols, rows) = (f32::from(COLS), f32::from(ROWS));
        // 18 pixels a room as in the mockup, smaller when the teleporter codes
        // take more than one row.
        let pitch = ((bottom - top) / rows).floor().min(18.0);
        let unit = pitch / 18.0;
        // Centred at level 2; from level 3 at the panel's left margin, with
        // the core column beside it (#7).
        let x0 = if pieces {
            PICTURE_W + 24.0
        } else {
            (PICTURE_W + (WINDOW_W - PICTURE_W - pitch * cols) / 2.0).floor()
        };
        let rooms = COLS * ROWS;
        let at = |room: u16| {
            (
                x0 + f32::from(room % COLS) * pitch,
                top + f32::from(room / COLS) * pitch,
            )
        };

        for room in 0..rooms {
            let (x, y) = at(room);
            if guidance.visited(room) {
                canvas.round_rect(x, y, pitch, pitch, 0.0, FLOOR);
            } else if pieces && guidance.piece(room) {
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
        for room in (0..rooms).filter(|&r| guidance.visited(r)) {
            let (x, y) = at(room);
            let open = guidance
                .openings()
                .get(room as usize)
                .copied()
                .unwrap_or_default();
            let long = pitch + 2.0 * overhang;
            if !open.up {
                canvas.round_rect(x - overhang, y - overhang, long, line, 0.0, WALL);
            }
            if !open.down {
                canvas.round_rect(x - overhang, y + pitch - overhang, long, line, 0.0, WALL);
            }
            if !open.left {
                canvas.round_rect(x - overhang, y - overhang, line, long, 0.0, WALL);
            }
            if !open.right {
                canvas.round_rect(x + pitch - overhang, y - overhang, line, long, 0.0, WALL);
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
                        WALL,
                    );
                }
            }
        }
        // Level 4 (#9, #44): the routes, through the centres of the rooms
        // walked, above the floor and walls and below the markers: to the
        // core in orange and to the nearest piece in pink. Where both take
        // the same step they run side by side, thinner, so neither hides
        // the other. A teleport step jumps, so no line joins it.
        if guidance.level() >= 4
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
                    stroke(
                        canvas,
                        (ax + ox, ay + oy),
                        (bx + ox, by + oy),
                        width,
                        None,
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
        for room in (0..rooms).filter(|&r| pieces && guidance.piece(r)) {
            let (x, y) = at(room);
            let r = 4.5 * unit;
            let (cx, cy) = (x + pitch / 2.0, y + pitch / 2.0);
            canvas.round_rect(cx - r, cy - r, 2.0 * r, 2.0 * r, r, PIECE);
        }
        // The core's end of its route, ringed in the route's colour (#44):
        // the map marks no core room otherwise.
        if guidance.level() >= 4
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
        top + rows * pitch
    }

    /// Level 4 (#44): what the two route colours mean, under the map at
    /// `y`: each route's word in a chip of its colour, then what it leads to.
    /// The core's line is dimmed while no piece it needs is carried.
    fn route_legend(&mut self, canvas: &mut Canvas, guidance: &Guidance, y: f32) {
        let x = PICTURE_W + 24.0;
        let lines = [
            (PIECE_WORD, PIECE, "To the nearest missing piece", true),
            (
                CORE_WORD,
                ROUTE,
                "To the core, while you carry a piece it needs",
                guidance.core_route().is_some(),
            ),
        ];
        let word = |text| [span(text, 11.0, Weight::SemiBold, PANEL)];
        let chip_w = lines
            .iter()
            .map(|l| self.fonts.measure(&word(l.0)))
            .fold(0.0, f32::max)
            + 10.0;
        for (i, (text, colour, meaning, lit)) in lines.into_iter().enumerate() {
            let y = y + i as f32 * 22.0;
            let w = self.fonts.measure(&word(text));
            canvas.round_rect(x, y - 9.0, chip_w, 17.0, 3.0, colour);
            self.fonts.text(
                Some(canvas),
                x + (chip_w - w) / 2.0,
                y - 8.0,
                None,
                1.0,
                &word(text),
            );
            let spans = [span(
                meaning,
                12.0,
                Weight::Regular,
                if lit { SOFT } else { QUIET },
            )];
            self.fonts
                .text(Some(canvas), x + chip_w + 10.0, y - 8.0, None, 1.0, &spans);
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

    /// Level 3 (#7): the core's nine holes in a column at the panel's right
    /// margin, its label on the `label_y` line and its first tile level with
    /// the map's top at `top`. An open hole shows its piece in white,
    /// outlined in white while it is carried; a filled one shows its
    /// placeholder, dimmed, as the core room does.
    fn core(&mut self, canvas: &mut Canvas, guidance: &Guidance, label_y: f32, top: f32) {
        let x = WINDOW_W - 24.0 - TILE_W;
        let label = "CORE";
        let width: f32 = label
            .chars()
            .map(|c| self.fonts.advance(c, 11.0, Weight::SemiBold) + 11.0 * 0.14)
            .sum::<f32>()
            - 11.0 * 0.14;
        self.spaced(canvas, x + (TILE_W - width) / 2.0, label_y + 1.0, label);
        for (i, hole) in guidance.core().iter().enumerate() {
            let y = top + i as f32 * TILE_PITCH;
            canvas.round_rect(x, y, TILE_W, TILE_W, 4.0, TILE);
            if hole.carried {
                canvas.outline(x, y, TILE_W, TILE_W, 4.0, 2.0, None, HERE);
            }
            let colour = if hole.open { HERE } else { DELIVERED };
            let inset = (TILE_W - 16.0 * PIECE_PIXEL) / 2.0;
            for (cell, (cy, cx)) in [(0, 0), (0, 8), (8, 0), (8, 8)].into_iter().enumerate() {
                for row in 0..8 {
                    let byte = hole.graphic[cell * 8 + row];
                    for bit in 0..8 {
                        if byte & (0x80 >> bit) != 0 {
                            canvas.cell(
                                x + inset + (cx + bit) as f32 * PIECE_PIXEL,
                                y + inset + (cy + row) as f32 * PIECE_PIXEL,
                                PIECE_PIXEL,
                                colour,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Level 4 (#9): a line above the teleporter codes, between them and the
    /// map: which code to select when the next step is a teleport, or that
    /// no route is known.
    fn route_line(
        &mut self,
        canvas: &mut Canvas,
        guidance: &Guidance,
        label_y: f32,
        jumps: &[Jump],
    ) {
        let next: Vec<&Jump> = jumps.iter().filter(|j| j.next).collect();
        let code = |j: &Jump| String::from_utf8_lossy(&j.code).into_owned();
        let (a, b) = (next.first().map(|j| code(j)), next.get(1).map(|j| code(j)));
        let texts: Vec<(String, Rgb)> = match (next.as_slice(), a, b) {
            ([one], Some(a), _) => vec![(format!("Select code {a}"), one.colour)],
            ([p, _], Some(a), Some(b)) if a == b => vec![
                ("Select code ".into(), SOFT),
                (a, p.colour),
                (" for the item and the core".into(), SOFT),
            ],
            ([p, c], Some(a), Some(b)) => vec![
                ("Select code ".into(), SOFT),
                (a, p.colour),
                (" for the item, ".into(), SOFT),
                (b, c.colour),
                (" for the core".into(), SOFT),
            ],
            _ if guidance.route().is_none() => {
                vec![("No known route to a missing piece".into(), QUIET)]
            }
            _ => return,
        };
        let spans: Vec<Span> = texts
            .iter()
            .map(|(t, c)| span(t, 13.0, Weight::SemiBold, *c))
            .collect();
        self.fonts.text(
            Some(canvas),
            PICTURE_W + 24.0,
            label_y - 21.0,
            None,
            1.0,
            &spans,
        );
    }

    /// Level 1 (#4): the codes of the teleporters seen this game, as chips
    /// along the panel's bottom edge, in the order their booths were
    /// entered, or a line saying none are seen yet. Returns where the block
    /// starts, which anything above it must stop short of.
    fn teleporters(
        &mut self,
        canvas: &mut Canvas,
        left: f32,
        width: f32,
        seen: &[SeenTeleporter],
        highlight: &[Jump],
    ) -> f32 {
        let (chip_h, gap) = (26.0, 8.0);
        // Lay the chips out in rows first, so the block can sit on the
        // panel's bottom edge however many rows there are.
        let mut rows: Vec<Vec<(String, f32)>> = vec![Vec::new()];
        let mut used = 0.0;
        for teleporter in seen {
            let text = String::from_utf8_lossy(&teleporter.code).into_owned();
            let w = self
                .fonts
                .measure(&[span(&text, 14.0, Weight::SemiBold, CODE)])
                + 16.0;
            if used + w > width && !rows[rows.len() - 1].is_empty() {
                rows.push(Vec::new());
                used = 0.0;
            }
            used += w + gap;
            rows.last_mut().unwrap().push((text, w));
        }
        let lines = if seen.is_empty() {
            1.0
        } else {
            rows.len() as f32
        };
        let top = WINDOW_H - 24.0 - lines * (chip_h + gap) + gap - 22.0;
        self.spaced(canvas, left, top, "TELEPORTERS SEEN");
        if seen.is_empty() {
            self.fonts.text(
                Some(canvas),
                left,
                top + 24.0,
                Some(width),
                1.0,
                &[span(
                    "None yet: a code shows once you enter its booth.",
                    13.0,
                    Weight::Regular,
                    QUIET,
                )],
            );
            return top;
        }
        let mut y = top + 22.0;
        for row in rows {
            let mut x = left;
            for (text, w) in row {
                canvas.round_rect(x, y, w, chip_h, 4.0, CODE_FILL);
                // The piece route's outline on the chip, the core route's
                // around it when both use the same one.
                for (i, j) in highlight
                    .iter()
                    .filter(|j| String::from_utf8_lossy(&j.code) == text)
                    .enumerate()
                {
                    let out = i as f32 * 4.0;
                    canvas.outline(
                        x - out,
                        y - out,
                        w + 2.0 * out,
                        chip_h + 2.0 * out,
                        4.0 + out,
                        2.0,
                        None,
                        j.colour,
                    );
                }
                self.fonts.text(
                    Some(canvas),
                    x + 8.0,
                    y + 5.0,
                    None,
                    1.0,
                    &[span(&text, 14.0, Weight::SemiBold, CODE)],
                );
                x += w + gap;
            }
            y += chip_h + gap;
        }
        top
    }

    fn score_note(&mut self, canvas: &mut Canvas, left: f32, guidance: &Guidance) {
        let record = guidance.record();
        self.spaced(canvas, left, 26.0, "THIS GAME");
        self.fonts.text(
            Some(canvas),
            left,
            300.0,
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
            322.0,
            None,
            1.0,
            &[span(&headline, 22.0, Weight::SemiBold, BRIGHT)],
        );
        let mut y = 356.0;
        if let Some(detail) = detail {
            self.fonts.text(
                Some(canvas),
                left,
                y,
                None,
                1.0,
                &[span(detail, 14.0, Weight::Regular, SOFT)],
            );
            y += 40.0;
        } else {
            y += 18.0;
        }
        if record.training {
            canvas.round_rect(left, y + 6.0, 8.0, 8.0, 4.0, TRAINING);
            self.fonts.text(
                Some(canvas),
                left + 18.0,
                y,
                None,
                1.0,
                &[span(
                    "Training mode was used",
                    15.0,
                    Weight::Regular,
                    BRIGHT,
                )],
            );
        }
    }

    fn picker(&mut self, canvas: &mut Canvas, guidance: &Guidance) {
        canvas.shade(0.0, 0.0, WINDOW_W, WINDOW_H, DIM, 184);
        // Two settings, then the actions, each taller while it waits for its
        // second press.
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
        let (w, h) = (520.0, 372.0 + 8.0 + actions_h + 12.0 + 52.0);
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

        let focus = guidance.focus();
        let (level, training) = guidance.picked();

        // The guidance level: a number and a name, the notches, and what it adds.
        let (rx, rw) = (x + 12.0, w - 24.0);
        let top = y + 56.0;
        let focused = focus == Setting::Level;
        self.setting_box(canvas, rx, top, rw, 184.0, focused, "GUIDANCE LEVEL");
        self.arrows(canvas, rx, rw, top + 69.0, focused, level > 0, level < 5);
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
        let (nx, nw, gap) = (rx + 16.0, rw - 32.0, 6.0);
        let step = (nw - 4.0 * gap) / 5.0;
        for i in 1..=5u8 {
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

        // Training mode: off or on.
        let top = y + 250.0;
        let focused = focus == Setting::Training;
        self.setting_box(canvas, rx, top, rw, 106.0, focused, "TRAINING MODE");
        self.arrows(canvas, rx, rw, top + 50.0, focused, training, !training);
        let mid = rx + rw / 2.0;
        for (label, chosen, cx, fill, text) in [
            ("Off", !training, mid - 29.0, SWITCH_OFF, BRIGHT),
            ("On", training, mid + 29.0, SWITCH_ON, ON_TEXT),
        ] {
            let spans = [span(
                label,
                15.0,
                Weight::SemiBold,
                if chosen { text } else { PAUSED },
            )];
            let tw = self.fonts.measure(&spans);
            if chosen {
                canvas.round_rect(cx - tw / 2.0 - 14.0, top + 36.0, tw + 28.0, 28.0, 6.0, fill);
            }
            self.fonts
                .text(Some(canvas), cx - tw / 2.0, top + 41.0, None, 1.0, &spans);
        }
        self.centred_in(
            canvas,
            rx,
            rw,
            top + 76.0,
            &[span(
                "Not built yet: energy will stop draining.",
                13.0,
                Weight::Regular,
                HINT_KEY,
            )],
        );

        // The actions: pressed once, a row turns red and asks again.
        let mut ay = y + 380.0;
        canvas.round_rect(x + 1.0, y + 372.0, w - 2.0, 1.0, 0.0, RULE);
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
        let foot = y + h - 52.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        self.hints(
            canvas,
            x + 28.0,
            foot + 16.0,
            &[
                (&[Hint::Arrows(&["\u{2191}", "\u{2193}"])], "choose"),
                (&[Hint::Arrows(&["\u{2190}", "\u{2192}"])], "change"),
                (&[Hint::Key("Enter"), Hint::Button("A")], "OK"),
                (&[Hint::Key("Esc"), Hint::Button("B")], "cancel"),
            ],
        );

        if let Some(choice) = guidance.asking() {
            canvas.shade(x, y, w, h, DIM, 150);
            self.score_question(canvas, guidance, choice);
        }
    }

    /// The question over the picker when leaving it would add to the score
    /// note: exactly what changed since it opened, what the score will say,
    /// and buttons named for what they do.
    fn score_question(&mut self, canvas: &mut Canvas, guidance: &Guidance, choice: Choice) {
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
        if training != was_training {
            changes.push(format!(
                "Training mode {} \u{2192} {}",
                on_off(was_training),
                on_off(training)
            ));
        }
        let mut shows = Vec::new();
        if level > record.highest {
            shows.push(format!("guidance up to level {level}"));
        }
        if training && !record.training {
            shows.push("that training mode was used".to_string());
        }
        let later = match shows.len() {
            1 if level > record.highest => "turn it down",
            1 => "turn it off",
            _ => "change them back",
        };
        let explanation = format!(
            "This game's score will show {}. That stays, even if you {later} later.",
            shows.join(" and ")
        );
        let (keep, undo) = match (level != was_level, training != was_training) {
            (true, false) => (
                format!("Keep level {level}"),
                format!("Back to level {was_level}"),
            ),
            (false, true) => (
                "Keep training on".to_string(),
                "Turn training off".to_string(),
            ),
            _ => ("Keep both changes".to_string(), "Undo both".to_string()),
        };

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
        let h =
            58.0 + 22.0 * changes.len() as f32 + 10.0 + explanation_h + 20.0 + 40.0 + 20.0 + 44.0;
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
        let by = ly + explanation_h + 20.0;
        let bw = (inner - 12.0) / 2.0;
        for (i, (label, this)) in [(keep.as_str(), Choice::Use), (undo.as_str(), Choice::Undo)]
            .into_iter()
            .enumerate()
        {
            let bx = x + 24.0 + i as f32 * (bw + 12.0);
            let chosen = choice == this;
            if chosen {
                canvas.round_rect(bx, by, bw, 40.0, 8.0, ACCENT);
            } else {
                canvas.round_rect(bx, by, bw, 40.0, 8.0, BUTTON_LINE);
                canvas.round_rect(bx + 1.0, by + 1.0, bw - 2.0, 38.0, 7.0, DIALOG);
            }
            let spans = [span(
                label,
                14.0,
                Weight::SemiBold,
                if chosen { DIALOG } else { VALUE_DIM },
            )];
            let tw = self.fonts.measure(&spans);
            self.fonts.text(
                Some(canvas),
                bx + (bw - tw) / 2.0,
                by + 12.0,
                None,
                1.0,
                &spans,
            );
        }
        let foot = y + h - 44.0;
        canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
        self.hints(
            canvas,
            x + 24.0,
            foot + 12.0,
            &[
                (&[Hint::Arrows(&["\u{2190}", "\u{2192}"])], "choose"),
                (&[Hint::Key("Enter"), Hint::Button("A")], "confirm"),
                (&[Hint::Key("Esc"), Hint::Button("B")], "back"),
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
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let (nx, ny) = (-uy * width / 2.0, ux * width / 2.0);
    // Half a width past each end, as the edge lines overhang their corners.
    let (from, to) = (-width / 2.0, length + width / 2.0);
    let (on, step) = dash.map_or((to - from, to - from), |d| (d, 2.0 * d));
    let mut s = from;
    while s < to {
        let e = (s + on).min(to);
        let p = |t: f32| (a.0 + ux * t, a.1 + uy * t);
        let (p0, p1) = (p(s), p(e));
        let corners = [
            (p0.0 + nx, p0.1 + ny),
            (p1.0 + nx, p1.1 + ny),
            (p1.0 - nx, p1.1 - ny),
            (p0.0 - nx, p0.1 - ny),
        ];
        canvas.triangle([corners[0], corners[1], corners[2]], colour);
        canvas.triangle([corners[0], corners[2], corners[3]], colour);
        s += step;
    }
}

/// The teleport a route takes, for its chip and the "Select code" line:
/// its code, the route's colour, and whether it is the route's next step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Jump {
    code: [u8; 5],
    colour: Rgb,
    next: bool,
}

/// The teleports the panel names (#9, #44): the piece route's first, then
/// the core route's first, each in its route's colour; either may be
/// missing.
fn jumps(guidance: &Guidance) -> Vec<Jump> {
    let first = |route: Option<&[Step]>, colour| {
        let route = route?;
        let step = route.iter().find(|s| s.teleport)?;
        let seen = guidance
            .teleporters()
            .iter()
            .find(|t| t.room == step.room)?;
        Some(Jump {
            code: seen.code,
            colour,
            next: route.first().is_some_and(|s| s.teleport),
        })
    };
    let (piece, core) = (
        first(guidance.route(), PIECE),
        first(guidance.core_route(), ROUTE),
    );
    piece.into_iter().chain(core).collect()
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
mod tests {
    use super::*;
    use sidekick::map::{Divides, Openings, RoomSet};

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
        g.set_level(4);
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

    #[test]
    fn a_step_both_routes_take_shows_both_colours() {
        // Blob in 200; both routes go right to 201, then apart.
        let g = routed(
            200,
            &[(201, false), (202, false)],
            &[(201, false), (217, false)],
        );
        let (pixels, w, _) = render(&g, Scene::Play, false);
        // The map's pitch and origin at level 4, as `map` works them out.
        let below = WINDOW_H - 24.0 - 22.0 - 22.0 - 16.0 - LEGEND_H;
        let pitch = ((below - 96.0) / f32::from(ROWS)).floor().min(18.0);
        let at = |room: u16| {
            (
                PICTURE_W + 24.0 + f32::from(room % COLS) * pitch,
                96.0 + f32::from(room / COLS) * pitch,
            )
        };
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
        let jump = |code: &[u8; 5], colour, next| Jump {
            code: *code,
            colour,
            next,
        };
        let mut g = routed(200, &[(201, false), (300, true)], &[(400, true)]);
        seen(&mut g);
        assert_eq!(
            jumps(&g),
            [jump(b"AAAAA", PIECE, false), jump(b"BBBBB", ROUTE, true)],
            "both routes' teleports, the piece route's first"
        );
        let mut g = routed(200, &[(300, true)], &[(300, true)]);
        seen(&mut g);
        assert_eq!(
            jumps(&g),
            [jump(b"AAAAA", PIECE, true), jump(b"AAAAA", ROUTE, true)],
            "the same teleport for both: named twice, one outline each"
        );
        let mut g = routed(200, &[(201, false)], &[(400, true)]);
        seen(&mut g);
        assert_eq!(
            jumps(&g),
            [jump(b"BBBBB", ROUTE, true)],
            "only the core route teleports"
        );
        let mut g = routed(200, &[(201, false)], &[]);
        seen(&mut g);
        assert!(jumps(&g).is_empty(), "no teleport, nothing named");
    }

    #[test]
    fn a_chip_both_routes_use_has_both_outlines() {
        let mut g = routed(200, &[(300, true)], &[(300, true)]);
        g.set_teleporters(&[SeenTeleporter {
            room: 300,
            code: *b"AAAAA",
        }]);
        let (pixels, w, _) = render(&g, Scene::Play, false);
        let chips = (PICTURE_W + 10.0, WINDOW_H - 60.0, 140.0, 50.0);
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

    /// Nine made-up holes, not the game's graphics: simple shapes, three
    /// filled, one carried.
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
        let mut picker = Guidance::default();
        picker.set_level(3);
        picker.open();
        let mut record = Guidance::default();
        record.set_level(3);
        record.set_training(true);
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
                    g
                },
                Scene::Play,
                false,
            ),
            (
                "level4",
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
                    g.focus_down();
                    g.focus_down();
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
            ("score", record, Scene::GameOver, false),
            ("score-none", Guidance::default(), Scene::GameOver, false),
        ];
        for (name, guidance, scene, paused) in cases {
            let (rgb, w, h) = render(&guidance, scene, paused);
            std::fs::write(
                out.join(format!("panel-{name}.png")),
                zx_core::png::encode(&rgb, w, h),
            )
            .unwrap();
        }
    }
}
