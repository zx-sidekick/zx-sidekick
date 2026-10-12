//! The panel's map: places on a grid, walls, markers and routes
//! (`docs/mockups/starquake/play-level5.png`, `play-level6.png`), and the pins at the picture's
//! edge for a route's way out.

use crate::{colors, panel::Pixels, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// The largest a cell is drawn, in logical pixels: the mockups'.
const MOST_CELL: f32 = 15.0;

const GROUND: Color = Color::rgb(0x1A, 0x1D, 0x25);
const WALL: Color = Color::rgb(0x4A, 0x50, 0x60);
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
}

/// What a marker marks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    /// The player.
    Player,
    /// A teleport.
    Teleport,
    /// A door.
    Door,
    /// An item, as its picture.
    Item(Option<Pixels>),
    /// Somewhere to go.
    Target,
}

/// A marker in a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    /// Its cell's column.
    pub x: usize,
    /// Its cell's row.
    pub y: usize,
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

    /// Draws the map with its top left at (`x`, `y`), its cells `cell` pixels square; returns
    /// where it ends.
    pub(crate) fn draw(&self, c: &mut Canvas, (x, y): (f32, f32), cell: f32) -> f32 {
        #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
        let at = |cx: usize, cy: usize| (x + cx as f32 * cell, y + cy as f32 * cell);
        let centre = |cx: usize, cy: usize| {
            let (left, top) = at(cx, cy);
            (left + cell / 2.0, top + cell / 2.0)
        };
        let seen: Vec<&Cell> = self.cells.iter().filter(|c| c.seen).collect();
        for cy in 0..self.height {
            for cx in 0..self.width {
                if !seen.iter().any(|c| c.x == cx && c.y == cy) {
                    let (dx, dy) = centre(cx, cy);
                    c.circle((dx, dy), 1.5, Some(DOT), None);
                }
            }
        }
        for place in &seen {
            let (left, top) = at(place.x, place.y);
            c.rect((left, top, cell, cell), 0.0, Some(GROUND), None);
            for edge in &place.walls {
                let line = match edge {
                    Edge::North => (left, top, cell, 1.0),
                    Edge::South => (left, top + cell - 1.0, cell, 1.0),
                    Edge::West => (left, top, 1.0, cell),
                    Edge::East => (left + cell - 1.0, top, 1.0, cell),
                };
                c.rect(line, 0.0, Some(WALL), None);
            }
        }
        if seen.len() >= self.width * self.height {
            // The map's edge, once the whole map is known, as the mockup frames it at level 6;
            // a map of some rooms (the mockups' levels 2 to 5) has none.
            #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
            let (w, h) = (self.width as f32 * cell, self.height as f32 * cell);
            c.rect((x, y, w, h), 0.0, None, Some(Line::solid(WALL, 1.0)));
        }
        for route in &self.routes {
            let points: Vec<(f32, f32)> =
                route.steps.iter().map(|&(cx, cy)| centre(cx, cy)).collect();
            let line = Line {
                color: route.color.color(),
                width: 2.0,
                dash: Some((2.0, 3.0)),
            };
            c.polyline(&points, line);
        }
        for marker in &self.markers {
            let (mx, my) = centre(marker.x, marker.y);
            let (left, top) = at(marker.x, marker.y);
            match &marker.kind {
                MarkerKind::Player => {
                    c.rect((left, top - 3.0, cell, 2.0), 0.0, Some(PLAYER_BAR), None);
                    let edge = Line::solid(colors::TEXT, 2.0);
                    c.rect((mx - 4.5, my - 4.5, 9.0, 9.0), 2.0, None, Some(edge));
                }
                MarkerKind::Teleport => {
                    let r = 3.5 * std::f32::consts::SQRT_2;
                    let points = [(mx, my - r), (mx + r, my), (mx, my + r), (mx - r, my)];
                    c.polygon(&points, marker.color.unwrap_or(TEAL));
                }
                MarkerKind::Door => {
                    c.rect(
                        (mx - 3.5, my - 3.5, 7.0, 7.0),
                        1.0,
                        Some(marker.color.unwrap_or(TEAL)),
                        None,
                    );
                }
                MarkerKind::Target => {
                    let ring = Line::solid(marker.color.unwrap_or(colors::ACCENT), 1.5);
                    c.circle((mx, my), 4.0, None, Some(ring));
                }
                MarkerKind::Item(picture) => {
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
        #[allow(clippy::cast_precision_loss, reason = "a map's few cells")]
        let bottom = y + self.height as f32 * cell;
        bottom
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
