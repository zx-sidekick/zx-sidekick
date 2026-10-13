//! The panel's map: places on a grid, walls, what's inside each place, markers and routes
//! (`docs/mockups/starquake/play-level5.png`, `play-level6.png`,
//! `docs/mockups/2026-10-13-inner-walls.png`), and the pins at the picture's edge for a
//! route's way out.

use crate::{colors, panel::Pixels, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// The largest a cell is drawn, in logical pixels: the mockups'.
const MOST_CELL: f32 = 15.0;
/// A label marker's text size.
const LABEL_SIZE: f32 = 9.0;

const FLOOR: Color = Color::rgb(0x23, 0x27, 0x33);
const FLOOR_DIM: Color = Color::rgb(0x1E, 0x21, 0x29);
const WALL: Color = Color::rgb(0x9A, 0xA3, 0xB5);
const WALL_DIM: Color = Color::rgb(0x5A, 0x60, 0x70);
const DOT: Color = Color::rgb(0x23, 0x26, 0x2D);
const PLAYER_BAR: Color = Color::rgb(0x8A, 0x90, 0xA0);
const TEAL: Color = Color::rgb(0x6F, 0xD6, 0xC4);
const BODY: Color = Color::rgb(0xC4, 0xC7, 0xCE);
const PIN_TEXT: Color = Color::rgb(0x1B, 0x13, 0x08);

/// A route's colour: the first route's pink, the second's gold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RouteColor {
    /// Pink.
    #[default]
    First,
    /// Gold.
    Second,
}

impl RouteColor {
    /// The colour itself.
    #[must_use]
    pub const fn color(self) -> Color {
        match self {
            Self::First => Color::rgb(0xE8, 0x79, 0xB0),
            Self::Second => Color::rgb(0xF0, 0xB1, 0x4A),
        }
    }
}

/// An edge of a cell, or of the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// Up.
    North,
    /// Right.
    East,
    /// Down.
    South,
    /// Left.
    West,
}

/// A place on the grid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// Its column and row.
    pub x: usize,
    /// Its row.
    pub y: usize,
    /// The edges without an opening.
    pub walls: Vec<Edge>,
    /// Whether it's been seen, so its walls show.
    pub seen: bool,
    /// Drawn dimmer: seen on the map, but never entered.
    pub dim: bool,
    /// Not seen, but outlined with dashes: something to fetch is there.
    pub dashed: bool,
    /// What's inside it, on the map's `inside` grid.
    pub inside: Inside,
}

/// What's inside a place: cells of its own grid, by column and row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inside {
    /// Wall cells, drawn as small blocks.
    pub walls: Vec<(usize, usize)>,
    /// A door's cells, drawn as a checkerboard.
    pub doors: Vec<(usize, usize)>,
}

/// What a marker marks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    /// Where the player is: a box, with a bar above its cell.
    Player,
    /// A diamond.
    Diamond,
    /// A small square.
    Square,
    /// The marker's own picture.
    Picture(Option<Pixels>),
    /// A ring.
    Ring,
    /// A few characters of text: a door's number.
    Label(String),
}

/// A marker in a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    /// Its cell's column.
    pub x: usize,
    /// Its cell's row.
    pub y: usize,
    /// The cell of its cell's inside it sits on, on the map's `inside` grid; its cell's
    /// middle when `None`.
    pub at: Option<(usize, usize)>,
    /// What it marks.
    pub kind: MarkerKind,
    /// Its colour, where the kind has none of its own.
    pub color: Option<Color>,
}

/// A route through cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Its colour.
    pub color: RouteColor,
    /// The cells it goes through, in order.
    pub steps: Vec<(usize, usize)>,
    /// What it leads to: "Item 1/3".
    pub label: Option<String>,
    /// The way out of the room, for its pin at the picture's edge.
    pub exit: Option<Edge>,
}

/// The map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapGrid {
    /// Its size in cells.
    pub width: usize,
    /// Its height in cells.
    pub height: usize,
    /// How many columns and rows a cell's inside is divided into.
    pub inside: Option<(usize, usize)>,
    /// The cells it knows; the rest are unseen.
    pub cells: Vec<Cell>,
    /// Its markers.
    pub markers: Vec<Marker>,
    /// Its routes.
    pub routes: Vec<Route>,
}

impl MapGrid {
    /// The size a cell is drawn at, to fit `room` logical pixels across and down.
    #[must_use]
    pub fn cell_size(&self, (across, down): (f32, f32)) -> f32 {
        #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
        let (w, h) = (self.width.max(1) as f32, self.height.max(1) as f32);
        (across / w).min(down / h).min(MOST_CELL).floor().max(1.0)
    }

    /// Draws the map with its top left at (`x`, `y`), its cells `cell` pixels square: every
    /// floor, then every wall over them, then routes and markers. Returns where it ends.
    pub(crate) fn draw(
        &self,
        c: &mut Canvas,
        fonts: &mut Fonts,
        (x, y): (f32, f32),
        cell: f32,
    ) -> f32 {
        let plan = Plan {
            map: self,
            origin: (x, y),
            cell,
            known: self.by_place(),
        };
        plan.floors(c);
        // Walls go over every floor, so a line's overhang past its corner shows.
        for place in self.cells.iter().filter(|c| c.seen) {
            plan.walls(c, place);
            plan.inside(c, place);
        }
        if self.cells.iter().filter(|c| c.seen).count() >= self.width * self.height {
            // The map's edge, once the whole map is known, as the mockup frames it at level 6;
            // a map of some rooms (the mockups' levels 2 to 5) has none.
            #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
            let (w, h) = (self.width as f32 * cell, self.height as f32 * cell);
            c.rect((x, y, w, h), 0.0, None, Some(Line::solid(WALL_DIM, 1.0)));
        }
        for route in &self.routes {
            let points: Vec<(f32, f32)> = route
                .steps
                .iter()
                .map(|&(cx, cy)| plan.centre(cx, cy))
                .collect();
            let line = Line {
                color: route.color.color(),
                width: 2.0,
                dash: Some((2.0, 3.0)),
            };
            c.polyline(&points, line);
        }
        for marker in &self.markers {
            plan.marker(c, fonts, marker);
        }
        #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
        let bottom = y + self.height as f32 * cell;
        bottom
    }

    /// Each place on the grid, row by row, with the cell the map knows there.
    fn by_place(&self) -> Vec<Option<&Cell>> {
        let mut places = vec![None; self.width * self.height];
        for place in &self.cells {
            if place.x < self.width && place.y < self.height {
                places[place.y * self.width + place.x] = Some(place);
            }
        }
        places
    }
    /// Draws the routes' labels underlined in their colours, in a row from (`x`, `y`).
    pub(crate) fn legend(&self, c: &mut Canvas, fonts: &mut Fonts, (x, y): (f32, f32)) {
        let style = Style::new(12.0, BODY);
        let mut left = x;
        for route in &self.routes {
            let Some(label) = &route.label else { continue };
            let width = c.text(
                fonts,
                label,
                (left, y),
                Fonts::normal_line_height(&style),
                Align::Left,
                &style,
            );
            let underline = (width + 6.0).round();
            c.rect(
                (left, y + 15.0, underline, 2.0),
                0.0,
                Some(route.color.color()),
                None,
            );
            left += underline + 19.0;
        }
    }
}

/// A map laid out for drawing: where it is, how large its cells are, and what's at each place.
struct Plan<'a> {
    map: &'a MapGrid,
    origin: (f32, f32),
    cell: f32,
    known: Vec<Option<&'a Cell>>,
}

impl Plan<'_> {
    /// A cell's top left.
    #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
    fn at(&self, cx: usize, cy: usize) -> (f32, f32) {
        (
            self.origin.0 + cx as f32 * self.cell,
            self.origin.1 + cy as f32 * self.cell,
        )
    }

    fn centre(&self, cx: usize, cy: usize) -> (f32, f32) {
        let (left, top) = self.at(cx, cy);
        (left + self.cell / 2.0, top + self.cell / 2.0)
    }

    /// The seen cell at a place, if there is one.
    fn seen(&self, x: Option<usize>, y: Option<usize>) -> Option<&Cell> {
        match (x, y) {
            (Some(x), Some(y)) if x < self.map.width && y < self.map.height => {
                self.known[y * self.map.width + x].filter(|c| c.seen)
            }
            _ => None,
        }
    }

    /// The seen cell across an edge of `place`.
    fn across(&self, place: &Cell, edge: Edge) -> Option<&Cell> {
        match edge {
            Edge::North => self.seen(Some(place.x), place.y.checked_sub(1)),
            Edge::South => self.seen(Some(place.x), place.y.checked_add(1)),
            Edge::West => self.seen(place.x.checked_sub(1), Some(place.y)),
            Edge::East => self.seen(place.x.checked_add(1), Some(place.y)),
        }
    }

    /// Every place's floor, a dashed outline, or a dot.
    fn floors(&self, c: &mut Canvas) {
        let cell = self.cell;
        for cy in 0..self.map.height {
            for cx in 0..self.map.width {
                let (left, top) = self.at(cx, cy);
                match self.known[cy * self.map.width + cx] {
                    Some(place) if place.seen => {
                        let floor = if place.dim { FLOOR_DIM } else { FLOOR };
                        c.rect((left, top, cell, cell), 0.0, Some(floor), None);
                    }
                    Some(place) if place.dashed => {
                        let dashes = Line {
                            color: WALL_DIM,
                            width: 1.0,
                            dash: Some((2.0, 2.0)),
                        };
                        c.rect((left, top, cell, cell), 0.0, None, Some(dashes));
                    }
                    _ => c.circle(self.centre(cx, cy), 1.5, Some(DOT), None),
                }
            }
        }
    }

    /// A seen place's edges without an opening, each line overhanging its corners by a pixel.
    fn walls(&self, c: &mut Canvas, place: &Cell) {
        let cell = self.cell;
        let (left, top) = self.at(place.x, place.y);
        // A wall between two seen places is drawn once, as the east or south of the one left
        // of or above it, bright if either place is.
        let drawn_by_neighbour = |edge: Edge| {
            let facing = match edge {
                Edge::North => Edge::South,
                Edge::West => Edge::East,
                Edge::East | Edge::South => return false,
            };
            self.across(place, edge)
                .is_some_and(|other| other.walls.contains(&facing))
        };
        for &edge in place.walls.iter().filter(|&&e| !drawn_by_neighbour(e)) {
            let line = match edge {
                Edge::North => (left - 1.0, top, cell + 2.0, 1.0),
                Edge::South => (left - 1.0, top + cell - 1.0, cell + 2.0, 1.0),
                Edge::West => (left, top - 1.0, 1.0, cell + 2.0),
                Edge::East => (left + cell - 1.0, top - 1.0, 1.0, cell + 2.0),
            };
            let dim = place.dim && self.across(place, edge).is_none_or(|other| other.dim);
            c.rect(line, 0.0, Some(if dim { WALL_DIM } else { WALL }), None);
        }
    }

    /// A seen place's inside: its wall cells as small blocks, its door cells as a checkerboard.
    fn inside(&self, c: &mut Canvas, place: &Cell) {
        let Some((w, h)) = self.map.inside.filter(|&(w, h)| w > 0 && h > 0) else {
            return;
        };
        let (left, top) = self.at(place.x, place.y);
        #[allow(clippy::cast_precision_loss, reason = "a place's few cells")]
        let (bw, bh) = (self.cell / w as f32, self.cell / h as f32);
        #[allow(clippy::cast_precision_loss, reason = "a place's few cells")]
        let block =
            |(bx, by): (usize, usize)| (left + bx as f32 * bw, top + by as f32 * bh, bw, bh);
        let within = |&&(bx, by): &&(usize, usize)| bx < w && by < h;
        let wall = if place.dim { WALL_DIM } else { WALL };
        for &spot in place.inside.walls.iter().filter(within) {
            c.rect(block(spot), 0.0, Some(wall), None);
        }
        let checked = |spot: &&(usize, usize)| within(spot) && (spot.0 + spot.1).is_multiple_of(2);
        for &spot in place.inside.doors.iter().filter(checked) {
            c.rect(block(spot), 0.0, Some(TEAL), None);
        }
    }

    /// Where a marker sits: on its inside cell's middle, or its cell's.
    fn spot(&self, marker: &Marker) -> (f32, f32) {
        match (marker.at, self.map.inside) {
            (Some((ax, ay)), Some((w, h))) if w > 0 && h > 0 => {
                let (left, top) = self.at(marker.x, marker.y);
                #[allow(clippy::cast_precision_loss, reason = "a place's few cells")]
                let spot = (
                    left + (ax as f32 + 0.5) * self.cell / w as f32,
                    top + (ay as f32 + 0.5) * self.cell / h as f32,
                );
                spot
            }
            _ => self.centre(marker.x, marker.y),
        }
    }

    fn marker(&self, c: &mut Canvas, fonts: &mut Fonts, marker: &Marker) {
        let cell = self.cell;
        let (left, top) = self.at(marker.x, marker.y);
        let (mx, my) = self.spot(marker);
        match &marker.kind {
            MarkerKind::Player => {
                c.rect((left, top - 3.0, cell, 2.0), 0.0, Some(PLAYER_BAR), None);
                let edge = Line::solid(colors::TEXT, 2.0);
                c.rect((mx - 4.5, my - 4.5, 9.0, 9.0), 2.0, None, Some(edge));
            }
            MarkerKind::Diamond => {
                let r = 3.5 * std::f32::consts::SQRT_2;
                let points = [(mx, my - r), (mx + r, my), (mx, my + r), (mx - r, my)];
                c.polygon(&points, marker.color.unwrap_or(TEAL));
            }
            MarkerKind::Square => {
                c.rect(
                    (mx - 3.5, my - 3.5, 7.0, 7.0),
                    1.0,
                    Some(marker.color.unwrap_or(TEAL)),
                    None,
                );
            }
            MarkerKind::Label(text) => {
                let style =
                    Style::new(LABEL_SIZE, marker.color.unwrap_or(TEAL)).weight(Weight::Bold);
                let height = Fonts::normal_line_height(&style);
                c.text(
                    fonts,
                    text,
                    (mx, my - height / 2.0),
                    height,
                    Align::Center,
                    &style,
                );
            }
            MarkerKind::Ring => {
                let ring = Line::solid(marker.color.unwrap_or(colors::ACCENT), 1.5);
                c.circle((mx, my), 4.0, None, Some(ring));
            }
            MarkerKind::Picture(picture) => {
                let color = marker.color.unwrap_or_else(|| RouteColor::First.color());
                if let Some(picture) = picture {
                    #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
                    let (pw, ph) = (picture.width as f32, picture.height as f32);
                    crate::panel::draw_pixels(
                        c,
                        picture,
                        ((mx - pw / 2.0).round(), (my - ph / 2.0).round()),
                        1.0,
                        color,
                    );
                } else {
                    c.rect((mx - 2.0, my - 2.0, 4.0, 4.0), 0.0, Some(color), None);
                }
            }
        }
    }
}

/// A pin at the picture's edge: where a route leaves the room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    /// The edge.
    pub edge: Edge,
    /// The route's colour.
    pub color: RouteColor,
    /// What it says: "item", "core".
    pub label: String,
}

/// The picture's size, in logical pixels, for drawing pins over it.
pub const PICTURE: (u32, u32) = (960, 768);

/// Draws pins over the picture (`PICTURE` logical pixels, clear but for the pins): in the
/// border at the edge each leaves by, side by side where they share one.
#[must_use]
pub fn pins(pins: &[Pin], fonts: &mut Fonts, scale: u32) -> Canvas {
    let mut c = Canvas::new(PICTURE.0, PICTURE.1, scale);
    let text = Style::new(14.0, PIN_TEXT).weight(Weight::SemiBold);
    for edge in [Edge::North, Edge::East, Edge::South, Edge::West] {
        let here: Vec<&Pin> = pins.iter().filter(|p| p.edge == edge).collect();
        #[allow(clippy::cast_precision_loss, reason = "a pin or two")]
        let first = -(here.len().saturating_sub(1) as f32) * 22.0;
        for (i, pin) in here.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a pin or two")]
            let shift = first + i as f32 * 44.0;
            let (x, y) = match edge {
                Edge::North => (456.0 + shift, 28.0),
                Edge::South => (456.0 + shift, 700.0),
                Edge::West => (28.0, 364.0 + shift),
                Edge::East => (884.0, 364.0 + shift),
            };
            let (w, h) = (48.0, 40.0);
            // The arrow points out of the room.
            let shape: [(f32, f32); 5] = match edge {
                Edge::North => [(0.5, 0.0), (1.0, 0.3), (1.0, 1.0), (0.0, 1.0), (0.0, 0.3)],
                Edge::South => [(0.0, 0.0), (1.0, 0.0), (1.0, 0.7), (0.5, 1.0), (0.0, 0.7)],
                Edge::West => [(0.0, 0.5), (0.3, 0.0), (1.0, 0.0), (1.0, 1.0), (0.3, 1.0)],
                Edge::East => [(0.0, 0.0), (0.7, 0.0), (1.0, 0.5), (0.7, 1.0), (0.0, 1.0)],
            };
            let points: Vec<(f32, f32)> = shape
                .iter()
                .map(|&(px, py)| (x + px * w, y + py * h))
                .collect();
            c.polygon(&points, pin.color.color());
            c.text(fonts, &pin.label, (x + w / 2.0, y), h, Align::Center, &text);
        }
    }
    c
}
