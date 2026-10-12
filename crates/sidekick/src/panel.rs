//! What the panel shows: the guidance level, and the widgets of the game's definition shown
//! at it, from the values read this frame and what the script returned.

use sidekick_core::{Fact, Read, Reading, Session};
use sidekick_def::{Column, Definition, WidgetKind};
use sidekick_ui::{Panel, PanelBody, PanelColumn, PanelWidget};
use std::collections::BTreeMap;

/// The panel for a session's game at guidance level `level`; `None` when its definition has
/// no guidance levels, and so no panel.
pub(crate) fn of(session: &Session, level: usize) -> Option<Panel> {
    let script = Script {
        results: session.script_results(),
        off: session.script_off(),
    };
    panel(session.definition(), level, session.reading(), &script)
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
) -> Option<Panel> {
    let definition = definition?;
    let levels = &definition.guidance.as_ref()?.levels;
    let at = levels.get(level).or_else(|| levels.last())?;
    let widgets = definition
        .panel
        .iter()
        .filter(|w| usize::try_from(w.from_level).is_ok_and(|from| from <= level) && level > 0)
        // The game's font and pictures (#71) and the map (#72) come later.
        .filter(|w| is_plain(w.kind))
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

/// Whether a widget kind is drawn yet.
const fn is_plain(kind: WidgetKind) -> bool {
    matches!(
        kind,
        WidgetKind::Text | WidgetKind::Counter | WidgetKind::Meter
    )
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
        let shown = panel(Some(&def), 1, &reading(), &running).expect("a panel");
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
        let shown = panel(Some(&def), 1, &reading(), &stopped).expect("a panel");
        assert_eq!(
            shown.widgets[0].body,
            PanelBody::Empty("The script stopped: on_frame: took too many steps".to_owned())
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
        let some = panel(Some(&def), 1, &reading(), &none()).expect("a panel");
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
        let more = panel(Some(&def), 2, &reading(), &none()).expect("a panel");
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
        let off = panel(Some(&def), 0, &reading(), &none()).expect("a panel");
        assert!(off.widgets.is_empty());
        assert_eq!(off.description, "No guidance.");
        let plain = Definition::from_toml(
            "schema = 1\n[game]\nid = \"p\"\ntitle = \"P\"\nmachine = \"48k\"\n[[media]]\nname = \"t\"\n",
        )
        .expect("parses");
        assert!(panel(Some(&plain), 0, &reading(), &none()).is_none());
        assert!(panel(None, 0, &reading(), &none()).is_none());
    }
}
