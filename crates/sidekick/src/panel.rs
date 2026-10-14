//! What the panel shows: the guidance level, and the widgets of the game's definition shown
//! at it, from the values read this frame and what the script returned.

use crate::picture::color;
use sidekick_core::{Bitmap, Fact, Font, Read, Reading, Session};
use sidekick_def::{Column, Definition, MarkerPlace, MarkerShape, Widget, WidgetKind};
use sidekick_ui::{
    CardRow, Cell, Chip, Color, Edge, HintSlot, Icon, Inside, Look, MapGrid, Marker, MarkerKind,
    PadLabels, Panel, PanelBody, PanelColumn, PanelWidget, Pin, Pixels, Place, Route, RouteColor,
    Step, StepLine,
};
use std::collections::{BTreeMap, BTreeSet};

/// The panel for a session's game at guidance level `level`, with the training switches on
/// and the keys of the pad plugged in, if one is; `None` when its definition has no guidance
/// levels, and so no panel.
pub(crate) fn of(
    session: &Session,
    level: usize,
    chosen: Option<&str>,
    pad: Option<PadLabels>,
) -> Option<Panel> {
    let script = Script {
        results: session.script_results(),
        off: session.script_off(),
        kept: session.kept(),
        chosen,
    };
    let graphics = |w: &Widget| Graphics {
        font: w.font.as_deref().and_then(|f| session.font(f)),
        pictures: w
            .pictures
            .as_deref()
            .and_then(|p| session.sprites(p))
            .unwrap_or_default(),
    };
    let shown = panel(
        session.definition(),
        level,
        session.reading(),
        &script,
        &graphics,
    )?;
    let hint = match shown.hint {
        HintSlot::Empty => session
            .hint(u8::try_from(level).unwrap_or(u8::MAX))
            .map_or(HintSlot::Empty, |hint| HintSlot::Shows(hint.to_owned())),
        slot => slot,
    };
    Some(Panel {
        training: training(session.definition(), session.switches_on()),
        pad,
        hint,
        ..shown
    })
}

/// The labels of the training switches `on`, in the definition's order.
pub(crate) fn training(definition: Option<&Definition>, on: &BTreeSet<String>) -> Vec<String> {
    definition
        .map(|d| {
            d.cheats
                .iter()
                .filter(|c| on.contains(&c.id))
                .map(|c| c.label.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// A widget's font and pictures, read from the game's memory.
#[derive(Default)]
pub(crate) struct Graphics {
    pub(crate) font: Option<Font>,
    pub(crate) pictures: Vec<Bitmap>,
}

impl Graphics {
    /// A word in the font; `None` without a font.
    fn word(&self, text: &str) -> Option<Vec<Pixels>> {
        let font = self.font.as_ref()?;
        Some(
            text.chars()
                .filter_map(|letter| font.glyph(letter).or_else(|| font.glyph(' ')))
                .map(pixels)
                .collect(),
        )
    }

    fn picture(&self, fact: Option<&Fact>) -> Option<Pixels> {
        match fact? {
            Fact::Number(n) => self.pictures.get(usize::try_from(*n).ok()?).map(pixels),
            _ => None,
        }
    }
}

/// A game's picture as the panel draws it, in its own ink if it has one.
fn pixels(bitmap: &Bitmap) -> Pixels {
    Pixels {
        width: bitmap.width,
        height: bitmap.height,
        ink: bitmap.ink.clone(),
        color: bitmap.color.map(|c| {
            let [r, g, b, _] = color(c);
            Color::rgb(r, g, b)
        }),
    }
}

/// What the script returned last, why it stopped, if it did, and the kept lists.
pub(crate) struct Script<'a> {
    pub(crate) results: &'a BTreeMap<String, Fact>,
    pub(crate) off: Option<&'a str>,
    pub(crate) kept: &'a BTreeMap<String, Vec<String>>,
    /// The word chosen on the text-entry card, outlined where a chip shows it.
    pub(crate) chosen: Option<&'a str>,
}

/// What a widget's `source` names.
enum Source<'a> {
    Value(&'a Read, Option<i64>),
    Script(&'a Fact),
    Stopped(&'a str),
}

/// The panel for a game at guidance level `level`, with this frame's `reading` and the
/// script's results; `None` when its definition has no guidance levels, and so no panel.
pub(crate) fn panel(
    definition: Option<&Definition>,
    level: usize,
    reading: &Reading,
    script: &Script,
    graphics: &dyn Fn(&Widget) -> Graphics,
) -> Option<Panel> {
    let definition = definition?;
    let levels = &definition.guidance.as_ref()?.levels;
    let at = levels.get(level).or_else(|| levels.last())?;
    let widgets = definition
        .panel
        .iter()
        .filter(|w| usize::try_from(w.from_level).is_ok_and(|from| from <= level) && level > 0)
        .filter_map(|w| {
            let kept;
            let source = if let Some(name) = w.source.strip_prefix("value:") {
                let max = definition.values.get(name).and_then(|v| v.max);
                Source::Value(reading.get(name)?, max)
            } else if let Some(name) = w.source.strip_prefix("persist:") {
                let entries = script.kept.get(name).map(Vec::as_slice).unwrap_or_default();
                kept = Fact::List(entries.iter().cloned().map(Fact::Text).collect());
                Source::Script(&kept)
            } else {
                let name = w.source.strip_prefix("script:")?;
                match script.off {
                    Some(why) => Source::Stopped(why),
                    None => Source::Script(script.results.get(name)?),
                }
            };
            let body = match (w.kind, source) {
                (_, Source::Stopped(why)) => PanelBody::Empty(format!("The script stopped: {why}")),
                (
                    WidgetKind::ChipList | WidgetKind::IconGrid | WidgetKind::CardSet,
                    Source::Script(fact),
                ) => pictured(w, fact, &graphics(w), script.chosen)?,
                (WidgetKind::MapGrid, Source::Script(fact)) => {
                    PanelBody::Map(map(fact, &graphics(w), &w.markers, &w.places)?)
                }
                (WidgetKind::Text, Source::Value(read, _)) => PanelBody::Text(read.as_text()),
                (WidgetKind::Counter, Source::Value(read, _)) => PanelBody::Counter(read.number()?),
                (WidgetKind::Meter, Source::Value(read, max)) => PanelBody::Meter {
                    value: read.number()?,
                    max: max?,
                },
                (WidgetKind::Text, Source::Script(fact)) => PanelBody::Text(text(fact)),
                (WidgetKind::Counter, Source::Script(Fact::Number(n))) => PanelBody::Counter(*n),
                (WidgetKind::Meter, Source::Script(Fact::Map(m))) => {
                    let number = |k: &str| match m.get(k) {
                        Some(Fact::Number(n)) => Some(*n),
                        _ => None,
                    };
                    PanelBody::Meter {
                        value: number("value")?,
                        max: number("max")?,
                    }
                }
                _ => return None,
            };
            Some(PanelWidget {
                column: match w.column {
                    Column::Full => PanelColumn::Full,
                    Column::Left => PanelColumn::Left,
                    Column::Right => PanelColumn::Right,
                },
                title: w.title.clone(),
                body,
            })
        })
        .collect();
    Some(Panel {
        level,
        name: at.name.clone(),
        description: at.description.clone(),
        widgets,
        training: Vec::new(),
        pad: None,
        // A game with hints keeps their slot from level 1, filled by `of` while one holds.
        hint: if level > 0 && !definition.hints.is_empty() {
            HintSlot::Empty
        } else {
            HintSlot::Absent
        },
    })
}

/// A chip's outline: the accent for `true`, a route's colour for `"first"` or `"second"`.
fn outline(fact: Option<&Fact>) -> Option<Color> {
    match fact? {
        Fact::Flag(true) => Some(sidekick_ui::colors::ACCENT),
        Fact::Text(t) => route_color(t).map(RouteColor::color),
        _ => None,
    }
}

/// A marker's colour: a route's (`"first"`, `"second"`), a Spectrum colour 0 to 15, or
/// `"#RRGGBB"`.
fn marker_color(fact: Option<&Fact>) -> Option<Color> {
    match fact? {
        Fact::Text(t) => route_color(t)
            .map(RouteColor::color)
            .or_else(|| hex_color(t)),
        Fact::Number(n) => u8::try_from(*n).ok().filter(|&c| c < 16).map(|c| {
            let [r, g, b, _] = color(c);
            Color::rgb(r, g, b)
        }),
        _ => None,
    }
}

fn hex_color(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#').filter(|d| d.len() == 6)?;
    let [_, r, g, b] = u32::from_str_radix(digits, 16).ok()?.to_be_bytes();
    Some(Color::rgb(r, g, b))
}

fn route_color(name: &str) -> Option<RouteColor> {
    match name {
        "first" => Some(RouteColor::First),
        "second" => Some(RouteColor::Second),
        _ => None,
    }
}

fn edge(name: &str) -> Option<Edge> {
    match name {
        "N" => Some(Edge::North),
        "E" => Some(Edge::East),
        "S" => Some(Edge::South),
        "W" => Some(Edge::West),
        _ => None,
    }
}

/// A field of a map fact.
fn get<'a>(fact: &'a Fact, key: &str) -> Option<&'a Fact> {
    match fact {
        Fact::Map(map) => map.get(key),
        _ => None,
    }
}

fn number(fact: Option<&Fact>) -> Option<usize> {
    match fact? {
        Fact::Number(n) => usize::try_from(*n).ok(),
        _ => None,
    }
}

fn words(fact: Option<&Fact>) -> Option<&str> {
    match fact? {
        Fact::Text(t) => Some(t),
        _ => None,
    }
}

const fn flag(fact: Option<&Fact>) -> bool {
    matches!(fact, Some(Fact::Flag(true)))
}

/// A pair of numbers, `[x, y]`.
fn pair(fact: &Fact) -> Option<(usize, usize)> {
    match fact {
        Fact::List(xy) if xy.len() == 2 => Some((number(xy.first())?, number(xy.get(1))?)),
        _ => None,
    }
}

/// A route's step, `[x, y]`, or `[x, y, how]` where `how` is `"dashed"` or `"jump"`.
fn step(fact: &Fact) -> Option<Step> {
    let Fact::List(parts) = fact else {
        return None;
    };
    let line = match parts.get(2) {
        None => StepLine::Solid,
        Some(Fact::Text(how)) if how == "dashed" => StepLine::Dashed,
        Some(Fact::Text(how)) if how == "jump" => StepLine::Jump,
        Some(_) => return None,
    };
    (parts.len() <= 3)
        .then(|| {
            Some(Step::new(
                number(parts.first())?,
                number(parts.get(1))?,
                line,
            ))
        })
        .flatten()
}

/// A list of `[x, y]` pairs; any that isn't one is left out.
fn pairs(fact: Option<&Fact>) -> Vec<(usize, usize)> {
    match fact {
        Some(Fact::List(items)) => items.iter().filter_map(pair).collect(),
        _ => Vec::new(),
    }
}

/// The script's map: `{ width, height, inside, cells, markers, routes }`.
fn map(
    fact: &Fact,
    graphics: &Graphics,
    shapes: &BTreeMap<String, MarkerShape>,
    places: &BTreeMap<String, MarkerPlace>,
) -> Option<MapGrid> {
    let list = |key: &str| match get(fact, key) {
        Some(Fact::List(items)) => items.as_slice(),
        _ => &[],
    };
    let cells = list("cells")
        .iter()
        .filter_map(|cell| {
            Some(Cell {
                x: number(get(cell, "x"))?,
                y: number(get(cell, "y"))?,
                walls: words(get(cell, "walls"))
                    .unwrap_or_default()
                    .chars()
                    .filter_map(|c| edge(&c.to_string()))
                    .collect(),
                seen: flag(get(cell, "seen")),
                dim: flag(get(cell, "dim")),
                dashed: flag(get(cell, "dashed")),
                inside: get(cell, "inside").map_or_else(Inside::default, |inside| Inside {
                    walls: pairs(get(inside, "walls")),
                    doors: pairs(get(inside, "doors")),
                }),
            })
        })
        .collect();
    let markers = list("markers")
        .iter()
        .filter_map(|marker| {
            // The script's kind, drawn as the widget's markers say; one it doesn't list isn't
            // drawn.
            let name = words(get(marker, "kind"))?;
            let place = match places.get(name).copied().unwrap_or_default() {
                MarkerPlace::Middle => Place::Middle,
                MarkerPlace::TopLeft => Place::TopLeft,
                MarkerPlace::TopRight => Place::TopRight,
            };
            let kind = match shapes.get(name)? {
                MarkerShape::Player => MarkerKind::Player,
                MarkerShape::Diamond => MarkerKind::Diamond,
                MarkerShape::Square => MarkerKind::Square,
                MarkerShape::Ring => MarkerKind::Ring,
                MarkerShape::Picture => {
                    MarkerKind::Picture(graphics.picture(get(marker, "picture")))
                }
                MarkerShape::Label => MarkerKind::Label(match get(marker, "label")? {
                    Fact::Text(text) => text.clone(),
                    Fact::Number(n) => n.to_string(),
                    _ => return None,
                }),
            };
            Some(Marker {
                x: number(get(marker, "x"))?,
                y: number(get(marker, "y"))?,
                at: get(marker, "at").and_then(pair),
                place,
                kind,
                color: marker_color(get(marker, "color")),
            })
        })
        .collect();
    let routes = list("routes")
        .iter()
        .filter_map(|route| {
            let steps = match get(route, "steps")? {
                Fact::List(steps) => steps.iter().filter_map(step).collect(),
                _ => return None,
            };
            Some(Route {
                color: words(get(route, "color"))
                    .and_then(route_color)
                    .unwrap_or_default(),
                steps,
                label: words(get(route, "label")).map(str::to_owned),
                exit: words(get(route, "exit")).and_then(edge),
            })
        })
        .collect();
    Some(MapGrid {
        width: number(get(fact, "width"))?,
        height: number(get(fact, "height"))?,
        inside: get(fact, "inside").and_then(pair),
        cells,
        markers,
        routes,
    })
}

/// The pins at the picture's edge for the panel's routes that leave the room: each says the
/// first word of its route's label.
pub(crate) fn pins(panel: &Panel) -> Vec<Pin> {
    panel
        .widgets
        .iter()
        .filter_map(|w| match &w.body {
            PanelBody::Map(map) => Some(&map.routes),
            _ => None,
        })
        .flatten()
        .filter_map(|route| {
            let label = route.label.as_deref()?.split_whitespace().next()?;
            Some(Pin {
                edge: route.exit?,
                color: route.color,
                label: label.to_lowercase(),
            })
        })
        .collect()
}

/// A widget drawn in the game's font or pictures, from what the script returned: its
/// `empty` words when there's nothing to show.
fn pictured(
    widget: &Widget,
    fact: &Fact,
    graphics: &Graphics,
    chosen: Option<&str>,
) -> Option<PanelBody> {
    let Fact::List(items) = fact else {
        return None;
    };
    if items.is_empty() {
        return widget.empty.clone().map(PanelBody::Empty);
    }
    let field = |item: &Fact, key: &str| match item {
        Fact::Map(map) => map.get(key).cloned(),
        _ => None,
    };
    let flag = |item: &Fact, key: &str| matches!(field(item, key), Some(Fact::Flag(true)));
    Some(match widget.kind {
        WidgetKind::ChipList => PanelBody::Chips(
            items
                .iter()
                .filter_map(|item| {
                    let text = match item {
                        Fact::Text(t) => t.clone(),
                        _ => match field(item, "text")? {
                            Fact::Text(t) => t,
                            _ => return None,
                        },
                    };
                    Some(Chip {
                        glyphs: graphics.word(&text)?,
                        outline: if chosen == Some(text.as_str()) {
                            Some(sidekick_ui::colors::ACCENT)
                        } else {
                            outline(field(item, "outlined").as_ref())
                        },
                    })
                })
                .collect(),
        ),
        WidgetKind::IconGrid => PanelBody::Icons {
            columns: widget.columns.map_or(3, |c| c as usize),
            icons: items
                .iter()
                .filter_map(|item| {
                    let look = match field(item, "look") {
                        Some(Fact::Text(t)) if t == "dim" => Look::Dim,
                        Some(Fact::Text(t)) if t == "outlined" => Look::Outlined,
                        _ => Look::Bright,
                    };
                    Some(Icon {
                        picture: graphics.picture(field(item, "picture").as_ref())?,
                        look,
                    })
                })
                .collect(),
        },
        WidgetKind::CardSet => PanelBody::Cards(
            items
                .iter()
                .filter_map(|item| {
                    let label = match field(item, "label") {
                        Some(Fact::Text(t)) => t,
                        Some(Fact::Number(n)) => n.to_string(),
                        _ => String::new(),
                    };
                    let Some(Fact::List(cards)) = field(item, "cards") else {
                        return None;
                    };
                    Some(CardRow {
                        outline: outline(field(item, "outlined").as_ref()),
                        label: graphics.word(&label)?,
                        cards: cards
                            .iter()
                            .filter_map(|card| {
                                let picture = graphics.picture(field(card, "picture").as_ref())?;
                                Some((picture, flag(card, "answered")))
                            })
                            .collect(),
                    })
                })
                .collect(),
        ),
        WidgetKind::Text | WidgetKind::Counter | WidgetKind::Meter | WidgetKind::MapGrid => {
            return None;
        }
    })
}

/// A script's fact as a line of text.
fn text(fact: &Fact) -> String {
    match fact {
        Fact::Nothing | Fact::Map(_) => String::new(),
        Fact::Flag(b) => b.to_string(),
        Fact::Number(n) => n.to_string(),
        Fact::Real(r) => r.to_string(),
        Fact::Text(t) => t.clone(),
        Fact::List(list) => list.iter().map(text).collect::<Vec<_>>().join(", "),
    }
}

/// A value as text, for a `text` widget: text as it is, anything else as it reads.
trait AsText {
    fn as_text(&self) -> String;
}

impl AsText for Read {
    fn as_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            other => other.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick_core::Reading;

    #[test]
    fn a_marker_s_color_is_a_route_s_a_spectrum_one_or_any() {
        let text = |t: &str| marker_color(Some(&Fact::Text(t.to_owned())));
        assert_eq!(text("first"), Some(RouteColor::First.color()));
        assert_eq!(text("#A066FF"), Some(Color::rgb(0xA0, 0x66, 0xFF)));
        assert_eq!(text("#A066F"), None, "six digits");
        assert_eq!(text("A066FF"), None, "after a #");
        assert_eq!(text("#A066FG"), None, "hex");
        let [r, g, b, _] = color(13);
        assert_eq!(
            marker_color(Some(&Fact::Number(13))),
            Some(Color::rgb(r, g, b))
        );
        assert_eq!(marker_color(Some(&Fact::Number(16))), None, "0 to 15");
    }

    fn definition(panel: &str) -> Definition {
        Definition::from_toml(&format!(
            r#"
schema = 1
[game]
id = "test"
title = "Test"
machine = "48k"
[[media]]
name = "the tape"

[guidance]
levels = [
  {{ name = "Off", description = "No guidance." }},
  {{ name = "Some", description = "Some help." }},
  {{ name = "More", description = "More help." }},
]

[values.lives]
addr = 0x8000
type = "u8"
[values.energy]
addr = 0x8001
type = "u8"
max = 200
[values.room]
addr = 0x8002
type = "text"
len = 4
{panel}
"#
        ))
        .expect("the test definition parses")
    }

    fn reading() -> Reading {
        [
            ("lives".to_owned(), Read::Number(3)),
            ("energy".to_owned(), Read::Number(150)),
            ("room".to_owned(), Read::Text("CAVE".to_owned())),
        ]
        .into_iter()
        .collect()
    }

    const NO_RESULTS: &BTreeMap<String, Fact> = &BTreeMap::new();
    const NO_KEPT: &BTreeMap<String, Vec<String>> = &BTreeMap::new();

    fn no_graphics(_: &Widget) -> Graphics {
        Graphics::default()
    }

    fn none() -> Script<'static> {
        Script {
            results: NO_RESULTS,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        }
    }

    #[test]
    fn script_sources_show_what_it_returned_or_why_it_stopped() {
        let def = definition(
            r#"
[[panel]]
kind = "counter"
title = "PIECES"
from_level = 1
source = "script:pieces"

[[panel]]
kind = "meter"
title = "NEAR"
from_level = 1
source = "script:near"

[[panel]]
kind = "text"
title = "ROUTE"
from_level = 1
source = "script:route"
"#,
        );
        let near = BTreeMap::from([
            ("value".to_owned(), Fact::Number(3)),
            ("max".to_owned(), Fact::Number(9)),
        ]);
        let results = BTreeMap::from([
            ("pieces".to_owned(), Fact::Number(4)),
            ("near".to_owned(), Fact::Map(near)),
            (
                "route".to_owned(),
                Fact::List(vec![Fact::Text("N".to_owned()), Fact::Text("E".to_owned())]),
            ),
        ]);
        let running = Script {
            results: &results,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &running, &no_graphics).expect("a panel");
        let bodies: Vec<PanelBody> = shown.widgets.into_iter().map(|w| w.body).collect();
        assert_eq!(
            bodies,
            [
                PanelBody::Counter(4),
                PanelBody::Meter { value: 3, max: 9 },
                PanelBody::Text("N, E".to_owned()),
            ]
        );
        let stopped = Script {
            results: &results,
            off: Some("on_frame: took too many steps"),
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &stopped, &no_graphics).expect("a panel");
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Empty("The script stopped: on_frame: took too many steps".to_owned())
        );
    }

    /// A stand-in glyph or picture: one ink pixel at (`n`, 0) of a 4 × 1.
    fn dot(n: usize) -> Bitmap {
        let mut ink = vec![false; 4];
        ink[n % 4] = true;
        Bitmap {
            width: 4,
            height: 1,
            ink,
            color: (n == 2).then_some(10),
        }
    }

    const PICTURED: &str = r#"
[[panel]]
kind = "chip-list"
title = "CODES"
from_level = 1
font = "font"
source = "script:codes"
empty = "None yet"

[[panel]]
kind = "icon-grid"
title = "CORE"
from_level = 1
columns = 2
pictures = "pictures"
source = "script:core"

[[panel]]
kind = "card-set"
title = "DOORS"
from_level = 1
font = "font"
pictures = "pictures"
source = "script:doors"
"#;

    fn map(pairs: Vec<(&str, Fact)>) -> Fact {
        Fact::Map(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
    }

    /// What a script might return for the pictured widgets.
    fn pictured_results() -> BTreeMap<String, Fact> {
        let card = map(vec![
            ("picture", Fact::Number(0)),
            ("answered", Fact::Flag(true)),
        ]);
        BTreeMap::from([
            (
                "codes".to_owned(),
                Fact::List(vec![
                    Fact::Text("AB".to_owned()),
                    map(vec![
                        ("text", Fact::Text("B".to_owned())),
                        ("outlined", Fact::Flag(true)),
                    ]),
                ]),
            ),
            (
                "core".to_owned(),
                Fact::List(vec![
                    map(vec![
                        ("picture", Fact::Number(2)),
                        ("look", Fact::Text("dim".to_owned())),
                    ]),
                    map(vec![("picture", Fact::Number(9))]),
                ]),
            ),
            (
                "doors".to_owned(),
                Fact::List(vec![
                    map(vec![
                        ("label", Fact::Number(1)),
                        ("cards", Fact::List(vec![card.clone()])),
                    ]),
                    map(vec![
                        ("label", Fact::Number(2)),
                        ("cards", Fact::List(vec![card])),
                        ("outlined", Fact::Text("second".to_owned())),
                    ]),
                ]),
            ),
        ])
    }

    /// Stand-ins for a game's font, from code 0x31, and its pictures.
    fn stand_ins(_: &Widget) -> Graphics {
        Graphics {
            font: Some(Font {
                first: 0x31,
                glyphs: (0..18).map(dot).collect(),
            }),
            pictures: (0..4).map(dot).collect(),
        }
    }

    #[test]
    fn words_are_drawn_in_the_game_s_font() {
        let def = definition(PICTURED);
        let results = pictured_results();
        let running = Script {
            results: &results,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &running, &stand_ins).expect("a panel");
        let px = |n| pixels(&dot(n));
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Chips(vec![
                Chip {
                    glyphs: vec![px(16), px(17)],
                    outline: None
                },
                Chip {
                    glyphs: vec![px(17)],
                    outline: Some(sidekick_ui::colors::ACCENT)
                },
            ]),
            "A and B are the font's 17th and 18th, from 1"
        );
        assert_eq!(
            shown.widgets[2].body,
            PanelBody::Cards(vec![
                CardRow {
                    label: vec![px(0)],
                    cards: vec![(px(0), true)],
                    outline: None,
                },
                CardRow {
                    label: vec![px(1)],
                    cards: vec![(px(0), true)],
                    outline: Some(RouteColor::Second.color()),
                },
            ]),
            "door 2 outlined in the second route's colour"
        );

        let nothing = BTreeMap::from([("codes".to_owned(), Fact::List(Vec::new()))]);
        let empty = Script {
            results: &nothing,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &empty, &stand_ins).expect("a panel");
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Empty("None yet".to_owned())
        );
    }

    #[test]
    fn pictures_are_the_game_s_in_their_own_ink() {
        let def = definition(PICTURED);
        let results = pictured_results();
        let running = Script {
            results: &results,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &running, &stand_ins).expect("a panel");
        let px = |n| pixels(&dot(n));
        assert_eq!(
            shown.widgets[1].body,
            PanelBody::Icons {
                columns: 2,
                icons: vec![Icon {
                    picture: px(2),
                    look: Look::Dim
                }],
            },
            "picture 9 isn't there"
        );
        assert_eq!(
            px(2).color,
            Some(Color::rgb(0xFF, 0, 0)),
            "its own ink: bright red"
        );
    }

    /// A map marker as a script returns it.
    fn at(x: i64, y: i64, kind: &str) -> Fact {
        map(vec![
            ("x", Fact::Number(x)),
            ("y", Fact::Number(y)),
            ("kind", Fact::Text(kind.to_owned())),
        ])
    }

    #[test]
    fn a_route_s_step_says_how_the_route_comes_to_it() {
        let cell = |how: Option<&str>| {
            let mut xy = vec![Fact::Number(3), Fact::Number(4)];
            xy.extend(how.map(|h| Fact::Text(h.to_owned())));
            step(&Fact::List(xy))
        };
        assert_eq!(cell(None), Some(Step::new(3, 4, StepLine::Solid)));
        assert_eq!(
            cell(Some("dashed")),
            Some(Step::new(3, 4, StepLine::Dashed))
        );
        assert_eq!(cell(Some("jump")), Some(Step::new(3, 4, StepLine::Jump)));
        assert_eq!(cell(Some("hop")), None, "neither: left out");
        assert_eq!(step(&Fact::List(vec![Fact::Number(3)])), None);
    }

    #[test]
    fn the_script_s_map_becomes_the_map_and_its_pins() {
        let def = definition(
            r#"
[[panel]]
kind = "map-grid"
from_level = 1
pictures = "pictures"
source = "script:map"
markers = { player = "player", item = "picture", door = "square" }
"#,
        );
        let xy = |x, y| Fact::List(vec![Fact::Number(x), Fact::Number(y)]);
        let text = |t: &str| Fact::Text(t.to_owned());
        let the_map = map(vec![
            ("width", Fact::Number(2)),
            ("height", Fact::Number(2)),
            (
                "cells",
                Fact::List(vec![map(vec![
                    ("x", Fact::Number(1)),
                    ("y", Fact::Number(0)),
                    ("walls", text("NE")),
                    ("seen", Fact::Flag(true)),
                ])]),
            ),
            (
                "markers",
                Fact::List(vec![
                    at(0, 1, "player"),
                    map(vec![
                        ("x", Fact::Number(1)),
                        ("y", Fact::Number(1)),
                        ("kind", text("item")),
                        ("picture", Fact::Number(2)),
                        ("color", text("second")),
                    ]),
                    at(0, 0, "ghost"),
                    at(1, 0, "door"),
                ]),
            ),
            (
                "routes",
                Fact::List(vec![map(vec![
                    ("color", text("second")),
                    ("steps", Fact::List(vec![xy(0, 1), xy(1, 1)])),
                    ("label", text("Core 2/9")),
                    ("exit", text("E")),
                ])]),
            ),
        ]);
        let results = BTreeMap::from([("map".to_owned(), the_map)]);
        let running = Script {
            results: &results,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &running, &stand_ins).expect("a panel");
        let PanelBody::Map(grid) = &shown.widgets[0].body else {
            panic!("a map: {:?}", shown.widgets[0].body);
        };
        assert_eq!((grid.width, grid.height), (2, 2));
        assert_eq!(
            grid.cells,
            [Cell {
                x: 1,
                y: 0,
                walls: vec![Edge::North, Edge::East],
                seen: true,
                ..Cell::default()
            }]
        );
        assert_eq!(grid.markers.len(), 3, "a kind it doesn't know is left out");
        assert_eq!(
            grid.markers[2].kind,
            MarkerKind::Square,
            "a kind drawn as the shape the widget gives it"
        );
        assert_eq!(
            grid.markers[1],
            Marker {
                x: 1,
                y: 1,
                at: None,
                place: Place::Middle,
                kind: MarkerKind::Picture(Some(pixels(&dot(2)))),
                color: Some(RouteColor::Second.color()),
            }
        );
        let solid = |x, y| Step::new(x, y, StepLine::Solid);
        assert_eq!(grid.routes[0].steps, [solid(0, 1), solid(1, 1)]);
        assert_eq!(
            pins(&shown),
            [Pin {
                edge: Edge::East,
                color: RouteColor::Second,
                label: "core".to_owned()
            }]
        );
    }

    #[test]
    fn a_map_s_cells_carry_what_is_inside_them() {
        let def = definition(
            r#"
[[panel]]
kind = "map-grid"
from_level = 1
source = "script:map"
markers = { door = "label" }
"#,
        );
        let xy = |x, y| Fact::List(vec![Fact::Number(x), Fact::Number(y)]);
        let number = |n| Fact::Number(n);
        let the_map = map(vec![
            ("width", number(2)),
            ("height", number(2)),
            ("inside", xy(8, 4)),
            (
                "cells",
                Fact::List(vec![
                    map(vec![
                        ("x", number(0)),
                        ("y", number(0)),
                        ("seen", Fact::Flag(true)),
                        ("dim", Fact::Flag(true)),
                        (
                            "inside",
                            map(vec![
                                ("walls", Fact::List(vec![xy(3, 0), xy(3, 1), number(5)])),
                                ("doors", Fact::List(vec![xy(3, 2)])),
                            ]),
                        ),
                    ]),
                    map(vec![
                        ("x", number(1)),
                        ("y", number(1)),
                        ("dashed", Fact::Flag(true)),
                    ]),
                ]),
            ),
            (
                "markers",
                Fact::List(vec![
                    map(vec![
                        ("x", number(0)),
                        ("y", number(0)),
                        ("at", xy(3, 2)),
                        ("kind", Fact::Text("door".to_owned())),
                        ("label", number(1)),
                    ]),
                    at(1, 0, "door"),
                ]),
            ),
        ]);
        let results = BTreeMap::from([("map".to_owned(), the_map)]);
        let running = Script {
            results: &results,
            off: None,
            kept: NO_KEPT,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &running, &stand_ins).expect("a panel");
        let PanelBody::Map(grid) = &shown.widgets[0].body else {
            panic!("a map: {:?}", shown.widgets[0].body);
        };
        assert_eq!(grid.inside, Some((8, 4)));
        assert_eq!(
            grid.cells,
            [
                Cell {
                    x: 0,
                    y: 0,
                    seen: true,
                    dim: true,
                    inside: Inside {
                        walls: vec![(3, 0), (3, 1)],
                        doors: vec![(3, 2)],
                    },
                    ..Cell::default()
                },
                Cell {
                    x: 1,
                    y: 1,
                    dashed: true,
                    ..Cell::default()
                },
            ],
            "a cell's inside, leaving out what isn't a place"
        );
        assert_eq!(
            grid.markers,
            [Marker {
                x: 0,
                y: 0,
                at: Some((3, 2)),
                place: Place::Middle,
                kind: MarkerKind::Label("1".to_owned()),
                color: None,
            }],
            "a door's number at its place inside the cell; one without a number isn't drawn"
        );
    }

    #[test]
    fn a_kept_list_gives_a_chip_list_its_words() {
        let def = definition(
            r#"
[[panel]]
kind = "chip-list"
title = "CODES"
from_level = 1
font = "font"
source = "persist:codes"
empty = "None yet"
"#,
        );
        let kept = BTreeMap::from([("codes".to_owned(), vec!["AB".to_owned()])]);
        let script = Script {
            results: NO_RESULTS,
            off: None,
            kept: &kept,
            chosen: None,
        };
        let shown = panel(Some(&def), 1, &reading(), &script, &stand_ins).expect("a panel");
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Chips(vec![Chip {
                glyphs: vec![pixels(&dot(16)), pixels(&dot(17))],
                outline: None
            }])
        );
        let shown = panel(Some(&def), 1, &reading(), &none(), &stand_ins).expect("a panel");
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Empty("None yet".to_owned())
        );
        let choosing = Script {
            chosen: Some("AB"),
            ..script
        };
        let shown = panel(Some(&def), 1, &reading(), &choosing, &stand_ins).expect("a panel");
        let PanelBody::Chips(chips) = &shown.widgets[0].body else {
            panic!("chips")
        };
        assert_eq!(
            chips[0].outline,
            Some(sidekick_ui::colors::ACCENT),
            "the word chosen on the card"
        );
    }

    const PANEL: &str = r#"
[[panel]]
kind = "counter"
title = "LIVES"
column = "left"
from_level = 1
source = "value:lives"

[[panel]]
kind = "meter"
title = "ENERGY"
column = "right"
from_level = 2
source = "value:energy"

[[panel]]
kind = "text"
title = "ROOM"
from_level = 1
source = "value:room"

[[panel]]
kind = "icon-grid"
title = "CORE"
from_level = 1
source = "script:core"
"#;

    #[test]
    fn widgets_show_from_their_level_with_their_values() {
        let def = definition(PANEL);
        let some = panel(Some(&def), 1, &reading(), &none(), &no_graphics).expect("a panel");
        assert_eq!(some.name, "Some");
        assert_eq!(
            some.widgets,
            [
                PanelWidget {
                    column: PanelColumn::Left,
                    title: Some("LIVES".to_owned()),
                    body: PanelBody::Counter(3),
                },
                PanelWidget {
                    column: PanelColumn::Full,
                    title: Some("ROOM".to_owned()),
                    body: PanelBody::Text("CAVE".to_owned()),
                },
            ],
            "energy from level 2; the icon grid not drawn yet"
        );
        let more = panel(Some(&def), 2, &reading(), &none(), &no_graphics).expect("a panel");
        assert_eq!(
            more.widgets[1].body,
            PanelBody::Meter {
                value: 150,
                max: 200
            }
        );
    }

    const CHEATS: &str = r#"
[[cheats]]
id = "lives"
label = "Endless lives"
set = { value = "lives", to = 3 }

[[cheats]]
id = "energy"
label = "Full energy"
set = { value = "energy", to = "max" }
"#;

    #[test]
    fn the_training_list_holds_the_switches_on_in_the_definition_s_order() {
        let def = definition(CHEATS);
        let on = |ids: &[&str]| ids.iter().map(|&id| id.to_owned()).collect::<BTreeSet<_>>();
        assert!(training(Some(&def), &on(&[])).is_empty());
        assert_eq!(
            training(Some(&def), &on(&["energy", "lives"])),
            ["Endless lives", "Full energy"]
        );
        assert_eq!(
            training(Some(&def), &on(&["energy", "gone"])),
            ["Full energy"]
        );
        assert!(training(None, &on(&["energy"])).is_empty());
    }

    #[test]
    fn the_panel_lists_the_switches_on_at_every_level_with_the_pad_plugged_in() {
        let machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        let mut s = Session::new(machine, Some(definition(CHEATS)));
        let off = of(&s, 0, None, None).expect("a panel");
        assert!(off.training.is_empty(), "nothing on, nothing listed");
        assert_eq!(off.pad, None);
        s.switch("energy", true).expect("the switch goes on");
        for level in 0..3 {
            let shown = of(&s, level, None, Some(PadLabels::Shapes)).expect("a panel");
            assert_eq!(shown.training, ["Full energy"], "at level {level}");
            assert_eq!(shown.pad, Some(PadLabels::Shapes));
        }
        s.switch("energy", false).expect("the switch goes off");
        assert!(of(&s, 1, None, None).expect("a panel").training.is_empty());
    }

    #[test]
    fn a_game_with_hints_keeps_their_slot_from_level_1() {
        let hinted = definition(
            "[[hints]]\ntext = \"Lives low\"\npriority = 1\nwhen = { value = \"lives\", at_most = 1 }\n",
        );
        let slot = |def: &Definition, level| {
            panel(Some(def), level, &reading(), &none(), &no_graphics)
                .expect("a panel")
                .hint
        };
        assert_eq!(slot(&hinted, 1), HintSlot::Empty, "kept, empty");
        assert_eq!(slot(&hinted, 2), HintSlot::Empty);
        assert_eq!(slot(&hinted, 0), HintSlot::Absent, "no guidance, no slot");
        assert_eq!(slot(&definition(PANEL), 1), HintSlot::Absent, "no hints");
    }

    #[test]
    fn the_hint_that_holds_fills_the_slot() {
        let def = definition(
            "[[states]]\nname = \"any\"\npc = 0x0038\n[[hints]]\ntext = \"Lives low\"\npriority = 1\nwhen = { value = \"lives\", at_most = 1 }\n",
        );
        let mut machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        for _ in 0..100 {
            machine.run_frame().expect("a frame");
        }
        let mut s = Session::new(machine, Some(def));
        s.machine_mut().write(0x8000, 3);
        s.run_frame().expect("a frame");
        assert_eq!(
            of(&s, 1, None, None).expect("a panel").hint,
            HintSlot::Empty
        );
        s.machine_mut().write(0x8000, 1);
        s.run_frame().expect("a frame");
        let shown = |level| of(&s, level, None, None).expect("a panel").hint;
        assert_eq!(shown(1), HintSlot::Shows("Lives low".to_owned()));
        assert_eq!(shown(0), HintSlot::Absent);
    }

    #[test]
    fn level_0_shows_no_widgets_and_no_levels_no_panel() {
        let def = definition(PANEL);
        let off = panel(Some(&def), 0, &reading(), &none(), &no_graphics).expect("a panel");
        assert!(off.widgets.is_empty());
        assert_eq!(off.description, "No guidance.");
        let plain = Definition::from_toml(
            "schema = 1\n[game]\nid = \"p\"\ntitle = \"P\"\nmachine = \"48k\"\n[[media]]\nname = \"t\"\n",
        )
        .expect("parses");
        assert!(panel(Some(&plain), 0, &reading(), &none(), &no_graphics).is_none());
        assert!(panel(None, 0, &reading(), &none(), &no_graphics).is_none());
    }
}

#[cfg(test)]
mod real_tape {
    use super::*;
    use crate::{
        games::{Game, Playing},
        situations,
    };
    use sidekick_def::{tapes_in, Package};
    use sidekick_ui::{Edge, Look, PanelBody as Body};
    use std::path::Path;
    use tempfile::TempDir;

    /// A new game of Starquake from the player's tape, at `level`; `None` without the tape.
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn new_game(level: u8) -> Option<(Playing, TempDir)> {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zip = repo.join("assets/Starquake.tap.zip");
        if !zip.exists() {
            eprintln!("skipped: no {} (the player's own tape)", zip.display());
            return None;
        }
        let tapes = tapes_in(&zip).expect("the zip holds a tape");
        let package = Package::load(&repo.join("games/starquake")).expect("its definition");
        let data = TempDir::new().expect("a folder");
        let mut playing = Game::new(package, data.path())
            .playing(&tapes[0])
            .expect("the tape plays");
        let s = &mut playing.session;
        let file = repo.join("games/starquake/tests/stocked-up.toml");
        situations::load(s, &playing.saves, &file).expect("a new game");
        s.set_level(level);
        for _ in 0..10 {
            s.run_frame().expect("a frame");
        }
        assert_eq!(s.script_off(), None, "the script runs");
        Some((playing, data))
    }

    /// The body of the widget with `title`, or of the map when `title` is `None`.
    fn body(s: &Session, level: usize, title: Option<&str>) -> Body {
        of(s, level, None, None)
            .expect("a panel")
            .widgets
            .into_iter()
            .find(|w| {
                title.map_or(matches!(w.body, Body::Map(_)), |t| {
                    w.title.as_deref() == Some(t)
                })
            })
            .map(|w| w.body)
            .expect("the widget")
    }

    #[test]
    fn starquake_s_level_1_shows_its_core_once_seen() {
        let Some((mut playing, _data)) = new_game(1) else {
            return;
        };
        let s = &mut playing.session;
        let addr = |s: &Session, name: &str| {
            s.definition().expect("a definition").values[name]
                .addr
                .expect("an address")
                .0
        };
        assert_eq!(
            body(s, 1, Some("CORE")),
            Body::Empty("Not seen yet".to_owned()),
            "a new game hasn't been to the core"
        );
        // Been in the core's room (199): its bit cleared in the game's set of rooms not visited.
        let visited = addr(s, "visited") + 199 / 8;
        let bits = s.machine().peek(visited) & !(0x80 >> (199 % 8));
        s.machine_mut().write(visited, bits);
        s.run_frame().expect("a frame");
        let Body::Icons { icons, .. } = body(s, 1, Some("CORE")) else {
            panic!("the core's icons");
        };
        assert_eq!(icons.len(), 9, "nine slots");
        assert!(
            icons.iter().all(|i| i.look == Look::Dim),
            "none delivered yet"
        );
        assert!(
            icons
                .iter()
                .all(|i| i.picture.width == 16 && i.picture.height == 16),
            "the game's own pictures"
        );
        assert_eq!(
            body(s, 1, Some("TELEPORTS")),
            Body::Empty("None yet".to_owned())
        );
        assert_eq!(
            body(s, 1, Some("DOORS")),
            Body::Empty("None yet".to_owned())
        );

        // The first slot's piece carried, the second delivered: what the game writes then, at
        // the addresses the definition gives.
        let (carried, slots) = (addr(s, "carried"), addr(s, "core_slots"));
        let first = s.machine().peek(slots) & 0x7F;
        let machine = s.machine_mut();
        machine.write(carried, first);
        machine.write(carried + 1, 0x47);
        machine.write(slots + 1, 1);
        s.run_frame().expect("a frame");
        let Body::Icons { icons, .. } = body(s, 1, Some("CORE")) else {
            panic!("the core's icons");
        };
        let looks: Vec<Look> = icons.iter().take(3).map(|i| i.look).collect();
        assert_eq!(looks, [Look::Outlined, Look::Bright, Look::Dim]);

        s.set_level(6);
        s.run_frame().expect("a frame");
        let Body::Chips(codes) = body(s, 6, Some("TELEPORTS")) else {
            panic!("every code at level 6");
        };
        assert_eq!(codes.len(), 15);
        assert!(
            codes.iter().all(|c| c.glyphs.len() == 5),
            "five letters each"
        );
    }

    #[test]
    fn starquake_s_map_starts_with_the_start_room() {
        let Some((playing, _data)) = new_game(2) else {
            return;
        };
        let Body::Map(map) = body(&playing.session, 2, None) else {
            panic!("the map at level 2");
        };
        let start = &map.cells[0];
        assert_eq!(
            (map.cells.len(), start.x, start.y),
            (1, 8, 0),
            "the start room"
        );
        assert_eq!(
            start.walls,
            [Edge::North],
            "the sky above; open left, right and down"
        );
    }
}
