//! What the panel shows: the guidance level, and the widgets of the game's definition shown
//! at it, from the values read this frame and what the script returned.

use crate::picture::color;
use sidekick_core::{Bitmap, Fact, Font, Read, Reading, Session};
use sidekick_def::{Column, Definition, Widget, WidgetKind};
use sidekick_ui::{
    CardRow, Chip, Color, Icon, Look, Panel, PanelBody, PanelColumn, PanelWidget, Pixels,
};
use std::collections::BTreeMap;

/// The panel for a session's game at guidance level `level`; `None` when its definition has
/// no guidance levels, and so no panel.
pub(crate) fn of(session: &Session, level: usize) -> Option<Panel> {
    let script = Script {
        results: session.script_results(),
        off: session.script_off(),
    };
    let graphics = |w: &Widget| Graphics {
        font: w.font.as_deref().and_then(|f| session.font(f)),
        pictures: w
            .pictures
            .as_deref()
            .and_then(|p| session.sprites(p))
            .unwrap_or_default(),
    };
    panel(
        session.definition(),
        level,
        session.reading(),
        &script,
        &graphics,
    )
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

/// What the script returned last, and why it stopped, if it did.
pub(crate) struct Script<'a> {
    pub(crate) results: &'a BTreeMap<String, Fact>,
    pub(crate) off: Option<&'a str>,
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
        // The map (#72) comes later.
        .filter(|w| w.kind != WidgetKind::MapGrid)
        .filter_map(|w| {
            let source = if let Some(name) = w.source.strip_prefix("value:") {
                let max = definition.values.get(name).and_then(|v| v.max);
                Source::Value(reading.get(name)?, max)
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
                ) => pictured(w, fact, &graphics(w))?,
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
    })
}

/// A widget drawn in the game's font or pictures, from what the script returned: its
/// `empty` words when there's nothing to show.
fn pictured(widget: &Widget, fact: &Fact, graphics: &Graphics) -> Option<PanelBody> {
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
                        outlined: flag(item, "outlined"),
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

    fn no_graphics(_: &Widget) -> Graphics {
        Graphics::default()
    }

    fn none() -> Script<'static> {
        Script {
            results: NO_RESULTS,
            off: None,
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
                Fact::List(vec![map(vec![
                    ("label", Fact::Number(1)),
                    ("cards", Fact::List(vec![card])),
                ])]),
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
        };
        let shown = panel(Some(&def), 1, &reading(), &running, &stand_ins).expect("a panel");
        let px = |n| pixels(&dot(n));
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Chips(vec![
                Chip {
                    glyphs: vec![px(16), px(17)],
                    outlined: false
                },
                Chip {
                    glyphs: vec![px(17)],
                    outlined: true
                },
            ]),
            "A and B are the font's 17th and 18th, from 1"
        );
        assert_eq!(
            shown.widgets[2].body,
            PanelBody::Cards(vec![CardRow {
                label: vec![px(0)],
                cards: vec![(px(0), true)],
            }])
        );

        let nothing = BTreeMap::from([("codes".to_owned(), Fact::List(Vec::new()))]);
        let empty = Script {
            results: &nothing,
            off: None,
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
