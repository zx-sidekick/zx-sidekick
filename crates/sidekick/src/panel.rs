//! What the panel shows: the guidance level, and the widgets of the game's definition shown
//! at it, from the values read this frame.

use sidekick_core::{Read, Reading};
use sidekick_def::{Column, Definition, WidgetKind};
use sidekick_ui::{Panel, PanelBody, PanelColumn, PanelWidget};

/// The panel for a game at guidance level `level`, with this frame's `reading`; `None` when
/// its definition has no guidance levels, and so no panel.
pub(crate) fn panel(
    definition: Option<&Definition>,
    level: usize,
    reading: &Reading,
) -> Option<Panel> {
    let definition = definition?;
    let levels = &definition.guidance.as_ref()?.levels;
    let at = levels.get(level).or_else(|| levels.last())?;
    let widgets = definition
        .panel
        .iter()
        .filter(|w| usize::try_from(w.from_level).is_ok_and(|from| from <= level) && level > 0)
        .filter_map(|w| {
            let name = w.source.strip_prefix("value:")?;
            let read = reading.get(name)?;
            let body = match w.kind {
                WidgetKind::Text => PanelBody::Text(read.as_text()),
                WidgetKind::Counter => PanelBody::Counter(read.number()?),
                WidgetKind::Meter => PanelBody::Meter {
                    value: read.number()?,
                    max: definition.values.get(name)?.max?,
                },
                // The game's font and pictures (#71) and the map (#72) come later.
                WidgetKind::ChipList
                | WidgetKind::IconGrid
                | WidgetKind::CardSet
                | WidgetKind::MapGrid => return None,
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
        let some = panel(Some(&def), 1, &reading()).expect("a panel");
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
        let more = panel(Some(&def), 2, &reading()).expect("a panel");
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
        let off = panel(Some(&def), 0, &reading()).expect("a panel");
        assert!(off.widgets.is_empty());
        assert_eq!(off.description, "No guidance.");
        let plain = Definition::from_toml(
            "schema = 1\n[game]\nid = \"p\"\ntitle = \"P\"\nmachine = \"48k\"\n[[media]]\nname = \"t\"\n",
        )
        .expect("parses");
        assert!(panel(Some(&plain), 0, &reading()).is_none());
        assert!(panel(None, 0, &reading()).is_none());
    }
}
