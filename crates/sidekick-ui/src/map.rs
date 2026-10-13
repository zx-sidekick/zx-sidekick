//! The panel's map: places on a grid, walls, what's inside each place, markers and routes
//! (`docs/mockups/starquake/play-level5.png`, `play-level6.png`,
//! `docs/mockups/2026-10-13-inner-walls.png`), and the pins at the picture's edge for a
//! route's way out.

use crate::{colors, panel::Pixels, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// The largest a cell is drawn, in logical pixels: the mockups'.
const MOST_CELL: f32 = 15.0;
/// A label marker's text size.
const LABEL_SIZE: f32 = 9.0;
/// How far apart routes sharing a step are drawn, middle to middle.
const SIDE_BY_SIDE: f32 = 2.5;

const FLOOR: Color = Color::rgb(0x23, 0x27, 0x33);
const FLOOR_DIM: Color = Color::rgb(0x1E, 0x21, 0x29);
const WALL: Color = Color::rgb(0x9A, 0xA3, 0xB5);
const WALL_DIM: Color = Color::rgb(0x5A, 0x60, 0x70);
const DOT: Color = Color::rgb(0x23, 0x26, 0x2D);
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
    /// Where the player is: an outline round its cell, so what's in it still shows.
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
    /// Where in its cell it sits, when `at` doesn't say.
    pub place: Place,
    /// What it marks.
    pub kind: MarkerKind,
    /// Its colour, where the kind has none of its own.
    pub color: Option<Color>,
}

impl Marker {
    /// A marker of `kind` in the middle of the cell at (`x`, `y`), in its kind's own colour.
    #[must_use]
    pub const fn new(x: usize, y: usize, kind: MarkerKind) -> Self {
        Self {
            x,
            y,
            at: None,
            place: Place::Middle,
            kind,
            color: None,
        }
    }
}

/// Where in its cell a marker sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Place {
    /// The middle.
    #[default]
    Middle,
    /// The top-left corner, small.
    TopLeft,
    /// The top-right corner, small.
    TopRight,
}

/// How a route comes to a step from the one before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepLine {
    /// A solid line.
    #[default]
    Solid,
    /// A dashed line: into or out of a place not yet visited.
    Dashed,
    /// No line: a jump, such as a teleport.
    Jump,
}

/// A step of a route: the cell it goes through, and how it comes there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// The cell's column and row.
    pub at: (usize, usize),
    /// The line from the step before.
    pub line: StepLine,
}

impl Step {
    /// A step to (`x`, `y`), drawn as `line` says.
    #[must_use]
    pub const fn new(x: usize, y: usize, line: StepLine) -> Self {
        Self { at: (x, y), line }
    }
}

/// A route through cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Its colour.
    pub color: RouteColor,
    /// The cells it goes through, in order.
    pub steps: Vec<Step>,
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
        for (r, route) in self.routes.iter().enumerate() {
            plan.route(c, r, route);
        }
        // One picture a place, so the room's first item shows whole; a dot says there are more.
        let mut pictured = std::collections::BTreeSet::new();
        let mut more = std::collections::BTreeSet::new();
        for marker in &self.markers {
            if matches!(marker.kind, MarkerKind::Picture(_))
                && marker.at.is_none()
                && !pictured.insert((marker.x, marker.y))
            {
                more.insert((marker.x, marker.y));
                continue;
            }
            plan.marker(c, fonts, marker);
        }
        for &(cx, cy) in &more {
            plan.more(c, cx, cy);
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

    /// A route, through its cells' middles: each step drawn as its `line` says, and where
    /// routes share a step, side by side and thinner. `r` is its place among the map's routes.
    fn route(&self, c: &mut Canvas, r: usize, route: &Route) {
        // A run of steps drawn alike, as one line, so its dashes go on round corners.
        let mut run: Vec<(f32, f32)> = Vec::new();
        let mut drawn: Option<(Line, (StepLine, u32))> = None;
        let flush = |c: &mut Canvas, run: &mut Vec<(f32, f32)>, drawn: Option<(Line, _)>| {
            if let Some((line, _)) = drawn {
                c.polyline(run, line);
            }
            run.clear();
        };
        for pair in route.steps.windows(2) {
            let (from, to) = (pair[0].at, pair[1].at);
            if pair[1].line == StepLine::Jump || from == to {
                flush(c, &mut run, drawn.take());
                continue;
            }
            let (offset, width) = self.beside(r, from, to);
            let line = Line {
                color: route.color.color(),
                width,
                dash: (pair[1].line == StepLine::Dashed).then_some((2.0, 3.0)),
            };
            let key = (pair[1].line, offset.to_bits());
            let (a, b) = (self.centre(from.0, from.1), self.centre(to.0, to.1));
            // Offset across the step, the same side whichever way a route takes it.
            let (lo, hi) = if from < to { (a, b) } else { (b, a) };
            let length = (hi.0 - lo.0).hypot(hi.1 - lo.1).max(f32::EPSILON);
            let (nx, ny) = ((lo.1 - hi.1) / length, (hi.0 - lo.0) / length);
            if drawn.is_none_or(|(_, k)| k != key) {
                flush(c, &mut run, drawn.take());
                run.push((a.0 + nx * offset, a.1 + ny * offset));
            }
            run.push((b.0 + nx * offset, b.1 + ny * offset));
            drawn = Some((line, key));
        }
        flush(c, &mut run, drawn);
    }

    /// Where route `r`'s step between two cells is drawn across it, and how wide: in the
    /// middle, unless other routes take the same step, when each is drawn beside the others.
    fn beside(&self, r: usize, from: (usize, usize), to: (usize, usize)) -> (f32, f32) {
        let takes = |route: &Route| {
            route.steps.windows(2).any(|pair| {
                pair[1].line != StepLine::Jump
                    && ((pair[0].at, pair[1].at) == (from, to)
                        || (pair[0].at, pair[1].at) == (to, from))
            })
        };
        let sharing: Vec<usize> = (0..self.map.routes.len())
            .filter(|&k| takes(&self.map.routes[k]))
            .collect();
        match sharing.iter().position(|&k| k == r) {
            Some(place) if sharing.len() > 1 => {
                #[allow(clippy::cast_precision_loss, reason = "a route or two")]
                let middle = place as f32 - (sharing.len() - 1) as f32 / 2.0;
                (middle * SIDE_BY_SIDE, 1.5)
            }
            _ => (0.0, 2.0),
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
            _ => {
                let (left, top) = self.at(marker.x, marker.y);
                let corner = self.cell * 0.25;
                match marker.place {
                    Place::Middle => self.centre(marker.x, marker.y),
                    Place::TopLeft => (left + corner, top + corner),
                    Place::TopRight => (left + self.cell - corner, top + corner),
                }
            }
        }
    }

    /// The dot in a place's bottom-right corner that says it holds more items than the one
    /// drawn.
    fn more(&self, c: &mut Canvas, cx: usize, cy: usize) {
        let (left, top) = self.at(cx, cy);
        let inset = self.cell * 0.2;
        c.circle(
            (left + self.cell - inset, top + self.cell - inset),
            1.25,
            Some(colors::TEXT),
            None,
        );
    }

    fn marker(&self, c: &mut Canvas, fonts: &mut Fonts, marker: &Marker) {
        let (mx, my) = self.spot(marker);
        // A shape in a corner is drawn at two thirds of its size, so the middle stays clear.
        let small = if marker.place == Place::Middle || marker.at.is_some() {
            1.0
        } else {
            2.0 / 3.0
        };
        match &marker.kind {
            MarkerKind::Player => {
                let (left, top) = self.at(marker.x, marker.y);
                let edge = Line::solid(colors::TEXT, 1.5);
                let cell = self.cell;
                c.rect(
                    (left + 0.75, top + 0.75, cell - 1.5, cell - 1.5),
                    1.5,
                    None,
                    Some(edge),
                );
            }
            MarkerKind::Diamond => {
                let r = 3.5 * small * std::f32::consts::SQRT_2;
                let points = [(mx, my - r), (mx + r, my), (mx, my + r), (mx - r, my)];
                c.polygon(&points, marker.color.unwrap_or(TEAL));
            }
            MarkerKind::Square => {
                let half = 3.5 * small;
                c.rect(
                    (mx - half, my - half, half * 2.0, half * 2.0),
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
                c.circle((mx, my), 4.0 * small, None, Some(ring));
            }
            MarkerKind::Picture(picture) => {
                let color = marker.color.unwrap_or_else(|| RouteColor::First.color());
                if let Some(picture) = picture {
                    // A whole step smaller than the panel's own pixels, where the screen has one:
                    // a device pixel each on a 2x screen, so the room around it shows.
                    let step = (c.scale().round() - 1.0).max(1.0) / c.scale();
                    #[allow(clippy::cast_precision_loss, reason = "a picture's few pixels")]
                    let (pw, ph) = (picture.width as f32 * step, picture.height as f32 * step);
                    crate::panel::draw_pixels(
                        c,
                        picture,
                        (mx - pw / 2.0, my - ph / 2.0),
                        step,
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

/// Draws pins over a picture `size` logical pixels large, near the edge each leaves by.
///
/// The picture is three logical pixels to a Spectrum one, drawn at `scale` device pixels to a
/// logical one; the canvas is clear but for the pins, side by side where they share an edge.
#[must_use]
pub fn pins(pins: &[Pin], fonts: &mut Fonts, scale: f32, (width, height): (u32, u32)) -> Canvas {
    let mut c = Canvas::new(width, height, scale);
    #[allow(clippy::cast_precision_loss, reason = "a picture's size")]
    let (width, height) = (width as f32, height as f32);
    let text = Style::new(14.0, PIN_TEXT).weight(Weight::SemiBold);
    let (w, h) = (48.0, 40.0);
    for edge in [Edge::North, Edge::East, Edge::South, Edge::West] {
        let here: Vec<&Pin> = pins.iter().filter(|p| p.edge == edge).collect();
        #[allow(clippy::cast_precision_loss, reason = "a pin or two")]
        let first = -(here.len().saturating_sub(1) as f32) * 22.0;
        for (i, pin) in here.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "a pin or two")]
            let shift = first + i as f32 * 44.0;
            let (x, y) = match edge {
                Edge::North => ((width - w) / 2.0 + shift, INSET),
                Edge::South => ((width - w) / 2.0 + shift, height - INSET - h),
                Edge::West => (INSET, (height - h) / 2.0 + shift),
                Edge::East => (width - INSET - w, (height - h) / 2.0 + shift),
            };
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

/// How far a pin sits in from the picture's edge, in logical pixels.
const INSET: f32 = 28.0;
