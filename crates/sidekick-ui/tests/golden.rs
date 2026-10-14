//! The program's screens, compared with golden images byte for byte. `UPDATE_GOLDEN=1`
//! rewrites them; look at a changed image before committing it.

use sidekick_ui::{
    pins, CardRow, Cell, Chip, ClipSlot, Edge, EntryCard, EntryMode, Face, Fonts, HintSlot, HowTo,
    HowToEntry, HowToTarget, Icon, Inside, Levels, LibraryEntry, LibraryPage, Look, MapGrid,
    Marker, MarkerKind, Menu, Note, NoticeHint, Offer, PadLabels, PadMark, Panel, PanelBody,
    PanelColumn, PanelWidget, PauseNotice, Pin, Pixels, Place, PromptButton, Question, Refusal,
    Route, RouteColor, Row, ScoreRow, Step, StepLine, SwitchRow, TapePrompt, ThisGame,
};
use std::path::PathBuf;

/// The panel's size in device pixels at scale 1, and at 2.
const PANEL_AT_1: (u32, u32) = (408, 768);
const PANEL_AT_2: (u32, u32) = (816, 1536);
/// The panel's size beside the picture in a 1920 × 1080 window: the window's height.
const PANEL_AT_1080: (u32, u32) = (574, 1080);

fn golden(name: &str, png: &[u8]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, png).expect("the golden image writes");
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_default();
    assert!(
        expected == png,
        "{name} differs from its golden image; look at it, then run with UPDATE_GOLDEN=1"
    );
}

fn starquake() -> TapePrompt {
    TapePrompt {
        title: "Starquake".to_owned(),
        credit: "Starquake © 1985 Stephen Crow / Bubble Bus Software. Not affiliated.".to_owned(),
        tape_file: "starquake.tap".to_owned(),
        download_file: Some("Starquake.tap.zip".to_owned()),
        download: Some((
            "World of Spectrum".to_owned(),
            "worldofspectrum.net/item/0004873".to_owned(),
        )),
        refusal: None,
        pad: None,
        pressed: None,
    }
}

#[test]
fn the_tape_prompt() {
    let mut fonts = Fonts::new();
    let canvas = starquake().draw(&mut fonts, 1.0);
    assert_eq!(canvas.size(), (960, 768));
    golden("tape-prompt.png", &canvas.png());
}

#[test]
fn the_tape_prompt_refusing_a_different_dump() {
    let mut fonts = Fonts::new();
    let prompt = TapePrompt {
        refusal: Some(Refusal {
            heading: "That isn't the Starquake tape this version needs".to_owned(),
            detail: "The tape in starquake-other.zip is a different dump. This version works only with the original Bubble Bus release (SHA-1 65450d6f…).".to_owned(),
        }),
        ..starquake()
    };
    golden(
        "tape-prompt-wrong-dump.png",
        &prompt.draw(&mut fonts, 1.0).png(),
    );
}

#[test]
fn the_tape_prompt_with_a_pad_plugged_in() {
    let mut fonts = Fonts::new();
    let prompt = TapePrompt {
        pad: Some(PadLabels::Letters),
        ..starquake()
    };
    golden("tape-prompt-pad.png", &prompt.draw(&mut fonts, 1.0).png());
}

#[test]
fn the_tape_prompt_s_buttons_pressed() {
    let mut fonts = Fonts::new();
    let locate = TapePrompt {
        pressed: Some(PromptButton::Locate),
        ..starquake()
    };
    golden(
        "tape-prompt-locate-pressed.png",
        &locate.draw(&mut fonts, 1.0).png(),
    );
    let download = TapePrompt {
        pressed: Some(PromptButton::Download),
        ..starquake()
    };
    golden(
        "tape-prompt-download-pressed.png",
        &download.draw(&mut fonts, 1.0).png(),
    );
}

#[test]
fn at_scale_2_every_logical_pixel_is_four() {
    let mut fonts = Fonts::new();
    let canvas = starquake().draw(&mut fonts, 2.0);
    assert_eq!(canvas.size(), (1920, 1536));
}

#[test]
fn the_library() {
    let mut fonts = Fonts::new();
    let entry = |title: &str, line: &str, ready| LibraryEntry {
        title: title.to_owned(),
        line: line.to_owned(),
        ready,
    };
    let page = LibraryPage {
        games: vec![
            entry("Starquake", "1985 · Bubble Bus Software", true),
            entry("Lunar Courier", "1984 · Example Software", false),
            entry("Deep Vault", "1986 · Another Label", false),
        ],
        chosen: 0,
        pressed: None,
        pad: PadLabels::Letters,
    };
    golden("library.png", &page.draw(&mut fonts, 1.0).png());
    let pressed = LibraryPage {
        chosen: 1,
        pressed: Some(1),
        ..page.clone()
    };
    golden("library-pressed.png", &pressed.draw(&mut fonts, 1.0).png());
    let playstation = LibraryPage {
        pad: PadLabels::Shapes,
        ..page
    };
    golden(
        "library-playstation.png",
        &playstation.draw(&mut fonts, 1.0).png(),
    );
}

fn paused(pad: PadLabels) -> PauseNotice {
    PauseNotice {
        heading: "Paused".to_owned(),
        text: "Move or fire to go on.".to_owned(),
        hints: vec![
            NoticeHint {
                keys: Some("← ↑ → ↓".to_owned()),
                pad: Some(PadMark::Directions),
                does: "move".to_owned(),
            },
            NoticeHint {
                keys: Some("Alt".to_owned()),
                pad: Some(PadMark::Face(Face::West)),
                does: "fire".to_owned(),
            },
        ],
        pad,
    }
}

#[test]
fn the_pause_notice() {
    let mut fonts = Fonts::new();
    let card = paused(PadLabels::Letters).draw(&mut fonts, 1.0);
    assert_eq!(card.size(), (438, 158));
    golden("paused.png", &card.png());
    golden(
        "paused-playstation.png",
        &paused(PadLabels::Shapes).draw(&mut fonts, 1.0).png(),
    );
    golden(
        "paused-unworded.png",
        &PauseNotice::default().draw(&mut fonts, 1.0).png(),
    );
    // As Starquake's definition words it.
    let title = PauseNotice {
        heading: "Ignore the list".to_owned(),
        text: "Press any key or button to start.".to_owned(),
        hints: vec![
            NoticeHint {
                keys: Some("← ↑ → ↓".to_owned()),
                pad: Some(PadMark::Directions),
                does: "move".to_owned(),
            },
            NoticeHint {
                keys: Some("Space".to_owned()),
                pad: Some(PadMark::Face(Face::West)),
                does: "fire".to_owned(),
            },
        ],
        ..paused(PadLabels::Letters)
    };
    golden("title-notice.png", &title.draw(&mut fonts, 1.0).png());
    let shapes = PauseNotice {
        pad: PadLabels::Shapes,
        ..title
    };
    golden(
        "title-notice-playstation.png",
        &shapes.draw(&mut fonts, 1.0).png(),
    );
    // As the program words the loading screen's: keys apart by ` / `, each in its badge.
    let loading = PauseNotice {
        heading: "Loading screen".to_owned(),
        text: "Press any button to load the rest of the game.".to_owned(),
        hints: vec![NoticeHint {
            keys: Some("Enter / Space / Z / X / C / V".to_owned()),
            pad: None,
            does: "go on".to_owned(),
        }],
        ..paused(PadLabels::Letters)
    };
    golden("loading-notice.png", &loading.draw(&mut fonts, 1.0).png());
}

fn starquake_menu() -> Menu {
    let level = |name: &str, line: &str| (name.to_owned(), line.to_owned());
    let switch = |label: &str| SwitchRow {
        label: label.to_owned(),
        on: false,
        knob: 0,
        refusal: None,
    };
    Menu {
        levels: Some(Levels {
            levels: vec![
                level("Off", ""),
                level("Codes and the core", ""),
                level("The map you have walked", ""),
                level(
                    "What you have seen",
                    "Core pieces still needed, in the rooms you have walked through.",
                ),
                level("What you have not", ""),
                level("Routes", ""),
                level("Everything", ""),
            ],
            level: 3,
        }),
        switches: [
            "Full energy",
            "Full bridging platforms",
            "Full laser",
            "Endless lives",
            "No harm from enemies",
        ]
        .map(switch)
        .to_vec(),
        keyboard: Some(SwitchRow {
            label: "Use the Spectrum keyboard".to_owned(),
            on: false,
            knob: 0,
            refusal: None,
        }),
        actions: ["Back to the game", "End this game", "Exit Starquake"]
            .map(str::to_owned)
            .to_vec(),
        chosen: Row::Level,
        pressed: None,
        armed: None,
        pad: PadLabels::Letters,
        saves: Some(false),
    }
}

#[test]
fn the_guidance_menu() {
    let mut fonts = Fonts::new();
    let menu = starquake_menu();
    golden("menu.png", &menu.draw(&mut fonts, 1.0).png());

    let mut refused = starquake_menu();
    refused.switches[0].on = true;
    refused.switches[0].knob = 100;
    // Halfway, sliding on.
    refused.switches[1].on = true;
    refused.switches[1].knob = 50;
    refused.switches[4].refusal =
        Some("Not this tape's code here: the switch can't go on.".to_owned());
    refused.chosen = Row::Switch(4);
    golden("menu-refused.png", &refused.draw(&mut fonts, 1.0).png());

    // A tape played plainly: the Spectrum's keyboard on, so no toggle.
    let armed = Menu {
        levels: None,
        switches: Vec::new(),
        keyboard: None,
        chosen: Row::Action(1),
        armed: Some((1, "end this game".to_owned())),
        pad: PadLabels::Shapes,
        ..starquake_menu()
    };
    golden("menu-plain-armed.png", &armed.draw(&mut fonts, 1.0).png());

    let keyboard = Menu {
        keyboard: Some(SwitchRow {
            label: "Use the Spectrum keyboard".to_owned(),
            on: true,
            knob: 100,
            refusal: None,
        }),
        chosen: Row::Keyboard,
        ..starquake_menu()
    };
    golden(
        "menu-keyboard-on.png",
        &keyboard.draw(&mut fonts, 1.0).png(),
    );

    // As the mockup has it (`docs/mockups/2026-10-13-pressed-button.png`).
    let save = Menu {
        chosen: Row::Save(0),
        pressed: Some(Row::Save(0)),
        ..starquake_menu()
    };
    golden("menu-save-pressed.png", &save.draw(&mut fonts, 1.0).png());
    let back = Menu {
        levels: None,
        switches: Vec::new(),
        chosen: Row::Action(0),
        pressed: Some(Row::Action(0)),
        ..starquake_menu()
    };
    golden("menu-row-pressed.png", &back.draw(&mut fonts, 1.0).png());

    let question = Question {
        items: vec!["Guidance level 4".to_owned(), "Full energy".to_owned()],
        pad: PadLabels::Letters,
    };
    golden("question.png", &question.draw(&mut fonts, 1.0).png());
}

#[test]
fn a_click_finds_the_menu_s_row() {
    let menu = starquake_menu();
    assert_eq!(menu.row_at(100.0, 150.0), Some(Row::Level));
    assert_eq!(menu.row_at(100.0, 285.0), Some(Row::Switch(0)));
    assert_eq!(menu.row_at(100.0, 420.0), Some(Row::Switch(4)));
    assert_eq!(menu.row_at(100.0, 480.0), Some(Row::Keyboard));
    assert_eq!(menu.row_at(100.0, 538.0), Some(Row::Action(0)));
    assert_eq!(menu.row_at(100.0, 584.0), Some(Row::Save(0)));
    assert_eq!(menu.row_at(450.0, 584.0), Some(Row::Save(2)));
    assert_eq!(menu.row_at(100.0, 670.0), Some(Row::Action(2)));
    assert_eq!(menu.row_at(5.0, 538.0), None);
    assert_eq!(menu.row_at(100.0, 511.0), None, "the rule between");
}

#[test]
fn the_panel() {
    let mut fonts = Fonts::new();
    let off = Panel {
        level: 0,
        name: "Off".to_owned(),
        description: "Guidance or training".to_owned(),
        widgets: Vec::new(),
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden("panel-level0.png", &off.draw(&mut fonts, PANEL_AT_1).png());
    for (name, pad) in [
        ("panel-level0-pad.png", PadLabels::Letters),
        ("panel-level0-playstation.png", PadLabels::Shapes),
        ("panel-level0-nintendo.png", PadLabels::Nintendo),
    ] {
        let with_pad = Panel {
            pad: Some(pad),
            ..off.clone()
        };
        golden(name, &with_pad.draw(&mut fonts, PANEL_AT_1).png());
    }
    let training = Panel {
        training: vec!["Full energy".to_owned(), "Endless lives".to_owned()],
        pad: Some(PadLabels::Letters),
        ..off.clone()
    };
    golden(
        "panel-level0-training.png",
        &training.draw(&mut fonts, PANEL_AT_1).png(),
    );
    let long = Panel {
        description: "No guidance yet: open the menu to choose a level or a training switch"
            .to_owned(),
        pad: Some(PadLabels::Letters),
        ..off
    };
    golden(
        "panel-level0-long.png",
        &long.draw(&mut fonts, PANEL_AT_1).png(),
    );

    let widget = |column, title: &str, body| PanelWidget {
        column,
        title: Some(title.to_owned()),
        body,
    };
    let plain = Panel {
        level: 1,
        name: "Codes and the core".to_owned(),
        description: String::new(),
        widgets: vec![
            widget(PanelColumn::Left, "Lives", PanelBody::Counter(3)),
            widget(
                PanelColumn::Right,
                "Teleports",
                PanelBody::Empty("None yet".to_owned()),
            ),
            widget(
                PanelColumn::Left,
                "Energy",
                PanelBody::Meter {
                    value: 120,
                    max: 200,
                },
            ),
            widget(
                PanelColumn::Right,
                "Doors",
                PanelBody::Empty("None yet".to_owned()),
            ),
            widget(
                PanelColumn::Full,
                "Room",
                PanelBody::Text("The eastern caverns".to_owned()),
            ),
            widget(
                PanelColumn::Full,
                "Laser",
                PanelBody::Meter {
                    value: 50,
                    max: 200,
                },
            ),
        ],
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden("panel-plain.png", &plain.draw(&mut fonts, PANEL_AT_1).png());
    let training = Panel {
        training: vec!["Full energy".to_owned(), "Endless lives".to_owned()],
        ..plain
    };
    golden(
        "panel-training.png",
        &training.draw(&mut fonts, PANEL_AT_1).png(),
    );
}

/// A stand-in picture: an 8 × 8 pattern, mirrored left to right, from a seed. Plainly not any
/// game's.
fn stand_in(seed: u32) -> Pixels {
    let mut state = seed.wrapping_mul(2_654_435_761);
    let mut ink = vec![false; 64];
    for y in 0..8 {
        for x in 0..4 {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let on = (state >> 16) & 1 == 1 || (x == 3 && (2..6).contains(&y));
            ink[y * 8 + x] = on;
            ink[y * 8 + 7 - x] = on;
        }
    }
    Pixels {
        width: 8,
        height: 8,
        ink,
        color: None,
    }
}

/// A stand-in word: a picture a letter.
fn stand_in_word(word: &str) -> Vec<Pixels> {
    word.bytes().map(|b| stand_in(u32::from(b) + 100)).collect()
}

#[test]
fn the_panel_in_the_game_s_own_font_and_pictures() {
    let mut fonts = Fonts::new();
    let icons = (1..=9)
        .map(|i| Icon {
            picture: stand_in(i),
            look: match i {
                1 => Look::Outlined,
                4 | 8 => Look::Dim,
                _ => Look::Bright,
            },
        })
        .collect();
    let chips = ["LUMEN", "TOPAZ", "NIV"]
        .iter()
        .enumerate()
        .map(|(i, w)| Chip {
            glyphs: stand_in_word(w),
            outline: (i == 1).then_some(sidekick_ui::colors::ACCENT),
        })
        .collect();
    let cards = (1..=3)
        .map(|r| CardRow {
            outline: None,
            label: stand_in_word(&r.to_string()),
            cards: (0..3)
                .map(|c| (stand_in(40 + r * 3 + c), (r + c) % 3 == 0))
                .collect(),
        })
        .collect();
    let panel = Panel {
        level: 6,
        name: "Everything".to_owned(),
        description: String::new(),
        widgets: vec![
            PanelWidget {
                column: PanelColumn::Left,
                title: Some("Core".to_owned()),
                body: PanelBody::Icons { columns: 3, icons },
            },
            PanelWidget {
                column: PanelColumn::Right,
                title: Some("Teleports".to_owned()),
                body: PanelBody::Chips(chips),
            },
            PanelWidget {
                column: PanelColumn::Right,
                title: Some("Doors".to_owned()),
                body: PanelBody::Cards(cards),
            },
        ],
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden(
        "panel-game-pictures.png",
        &panel.draw(&mut fonts, PANEL_AT_1).png(),
    );
    // Beside a 1920 × 1080 window's picture: a scale between whole ones, the game's pictures
    // and letters at whole device pixels.
    let wide = panel.draw(&mut fonts, PANEL_AT_1080);
    assert_eq!(wide.size(), PANEL_AT_1080);
    golden("panel-game-pictures-1080.png", &wide.png());
}

/// A route's steps through cells, every line dashed, as through rooms not yet visited.
fn dashed(cells: &[(usize, usize)]) -> Vec<Step> {
    cells
        .iter()
        .map(|&(x, y)| Step::new(x, y, StepLine::Dashed))
        .collect()
}

/// A stand-in map, 16 × 32 as the mockups' is: walls from a seeded coin where `seen`.
fn stand_in_map(seen: bool) -> MapGrid {
    let mut state = 7u32;
    let mut coin = || {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (state >> 16) % 100 < 45
    };
    let cells = (0..32)
        .flat_map(|y| (0..16).map(move |x| (x, y)))
        .map(|(x, y)| {
            let mut walls = Vec::new();
            if coin() {
                walls.push(Edge::East);
            }
            if coin() {
                walls.push(Edge::South);
            }
            Cell {
                x,
                y,
                walls,
                seen,
                ..Cell::default()
            }
        })
        .collect();
    let item = |x, y, seed| Marker {
        x,
        y,
        at: None,
        place: Place::Middle,
        kind: MarkerKind::Picture(Some(stand_in(seed))),
        color: None,
    };
    let mut markers = vec![
        Marker::new(8, 1, MarkerKind::Player),
        item(0, 1, 11),
        item(2, 5, 14),
        item(6, 7, 12),
        item(12, 14, 15),
    ];
    if seen {
        markers.push(Marker::new(1, 20, MarkerKind::Diamond));
    }
    MapGrid {
        width: 16,
        height: 32,
        inside: None,
        cells,
        markers,
        routes: vec![
            Route {
                color: RouteColor::First,
                steps: dashed(&[(8, 1), (8, 0), (10, 0), (10, 2), (8, 2), (8, 3)]),
                label: Some("Item 1/3".to_owned()),
                exit: Some(Edge::North),
            },
            Route {
                color: RouteColor::Second,
                steps: dashed(&[
                    (8, 1),
                    (7, 1),
                    (7, 6),
                    (6, 6),
                    (6, 7),
                    (8, 7),
                    (8, 13),
                    (6, 13),
                ]),
                label: Some("Core".to_owned()),
                exit: Some(Edge::South),
            },
        ],
    }
}

#[test]
fn the_map() {
    let mut fonts = Fonts::new();
    let icons: Vec<Icon> = (1..=9)
        .map(|i| Icon {
            picture: stand_in(i),
            look: Look::Bright,
        })
        .collect();
    let training = || {
        [
            "Full energy",
            "Full bridging platforms",
            "Full laser",
            "Endless lives",
            "No harm from enemies",
        ]
        .map(str::to_owned)
        .to_vec()
    };
    for (name, seen, training) in [
        ("panel-map-dots.png", false, Vec::new()),
        ("panel-map-walls.png", true, Vec::new()),
        ("panel-map-training.png", true, training()),
    ] {
        let panel = Panel {
            level: 5,
            name: "Routes".to_owned(),
            description: String::new(),
            widgets: vec![
                PanelWidget {
                    column: PanelColumn::Left,
                    title: Some("Core".to_owned()),
                    body: PanelBody::Icons {
                        columns: 3,
                        icons: icons.clone(),
                    },
                },
                PanelWidget {
                    column: PanelColumn::Right,
                    title: Some("Teleports".to_owned()),
                    body: PanelBody::Chips(
                        [
                            "LUMEN", "TOPAZ", "NIVEN", "ORBIT", "CIRRO", "VAPOR", "BRISK",
                        ]
                        .iter()
                        .enumerate()
                        .map(|(i, w)| Chip {
                            glyphs: stand_in_word(w),
                            outline: match i {
                                2 => Some(RouteColor::First.color()),
                                5 => Some(RouteColor::Second.color()),
                                _ => None,
                            },
                        })
                        .collect(),
                    ),
                },
                PanelWidget {
                    column: PanelColumn::Left,
                    title: None,
                    body: PanelBody::Map(stand_in_map(seen)),
                },
            ],
            training,
            pad: None,
            hint: HintSlot::Absent,
        };
        golden(name, &panel.draw(&mut fonts, PANEL_AT_1).png());
    }
    let pin = |edge, color, label: &str| Pin {
        edge,
        color,
        label: label.to_owned(),
    };
    golden(
        "pins.png",
        &pins(
            &[
                pin(Edge::South, RouteColor::First, "item"),
                pin(Edge::South, RouteColor::Second, "core"),
                pin(Edge::North, RouteColor::First, "item"),
                pin(Edge::East, RouteColor::Second, "core"),
            ],
            &mut fonts,
            1.0,
            (960, 768),
        )
        .png(),
    );
    // Over the screen alone (no border), at five device pixels to a Spectrum one.
    let screen = pins(
        &[
            pin(Edge::West, RouteColor::First, "item"),
            pin(Edge::South, RouteColor::Second, "core"),
        ],
        &mut fonts,
        5.0 / 3.0,
        (768, 576),
    );
    assert_eq!(screen.size(), (1280, 960));
    golden("pins-screen.png", &screen.png());
}

#[test]
fn the_text_entry_card() {
    let mut fonts = Fonts::new();
    let word = |w: &str| w.chars().map(Some).collect::<Vec<_>>();
    let choosing = EntryCard {
        title_choose: "Choose a code".to_owned(),
        title_spell: "Spell the code".to_owned(),
        letters: word("TOPAZ"),
        mode: EntryMode::Choosing { at: 1, of: 4 },
        can_choose: true,
        each_place: false,
        pad: PadLabels::Letters,
    };
    golden("entry-choose.png", &choosing.draw(&mut fonts, 1.0).png());
    let spelling = EntryCard {
        letters: word("TOMAZ"),
        mode: EntryMode::Spelling { slot: 2 },
        ..choosing.clone()
    };
    golden("entry-spell.png", &spelling.draw(&mut fonts, 1.0).png());
    let blank = EntryCard {
        letters: vec![Some('T'), None, None, None, None],
        mode: EntryMode::Spelling { slot: 1 },
        can_choose: false,
        pad: PadLabels::Shapes,
        ..choosing
    };
    golden("entry-spell-blank.png", &blank.draw(&mut fonts, 1.0).png());
}

#[test]
fn the_name_card_takes_each_place_and_shows_a_space() {
    let mut fonts = Fonts::new();
    let name = EntryCard {
        title_choose: "Choose a name".to_owned(),
        title_spell: "Spell a name".to_owned(),
        letters: vec![Some('J'), Some(' '), None],
        mode: EntryMode::Spelling { slot: 1 },
        can_choose: true,
        each_place: true,
        pad: PadLabels::Letters,
    };
    golden("entry-name-spell.png", &name.draw(&mut fonts, 1.0).png());
}

#[test]
fn the_panel_after_a_game() {
    let mut fonts = Fonts::new();
    let row = |name: &str, mark: &str, mine| ScoreRow {
        name: name.to_owned(),
        mark: mark.to_owned(),
        mine,
    };
    let page = ThisGame {
        played_with: "Guidance up to level 3".to_owned(),
        detail: "What you have seen".to_owned(),
        title: "Core of heroes".to_owned(),
        rows: vec![
            row("ANN", "level 3 · What you have seen", true),
            row("BOB", "no guidance", false),
            row("CAT", "from the tape", false),
            row("DAN", "level 5 · Routes · loaded", false),
            row("EVE", "from the tape", false),
            row("FAY", "from the tape", false),
            row("GUS", "level 1 · Codes and the core", false),
            row("HAL", "from the tape", false),
        ],
    };
    golden(
        "panel-this-game.png",
        &page.draw(&mut fonts, PANEL_AT_1).png(),
    );
}

#[test]
fn the_notice() {
    let mut fonts = Fonts::new();
    let note = Note {
        text: "Game saved".to_owned(),
    };
    assert_eq!(note.width(&mut fonts), 128, "the mockup's pill");
    assert_eq!(note.height(&mut fonts), 35);
    assert_eq!(note.seconds(), 2);
    golden("note.png", &note.draw(&mut fonts, 1.0).png());
}

#[test]
fn a_long_notice_wraps_and_shows_longer() {
    let mut fonts = Fonts::new();
    let note = Note {
        text: "Definition not reloaded: games/starquake/game.toml: unknown field `snyc`, expected one of `id`, `title`, `year`, `publisher`, `authors`, `machine` at line 13 column 1".to_owned(),
    };
    assert!(
        note.width(&mut fonts) <= 640,
        "no wider than the widest pill"
    );
    assert!(note.height(&mut fonts) > 35, "more than a line");
    assert!(note.seconds() > 2 && note.seconds() <= 10);
    golden("note-long.png", &note.draw(&mut fonts, 1.0).png());
}

#[test]
fn the_offer_to_continue() {
    let mut fonts = Fonts::new();
    let offer = Offer {
        title: "Starquake".to_owned(),
        pad: PadLabels::Letters,
    };
    golden("offer.png", &offer.draw(&mut fonts, 1.0).png());
}

#[test]
fn a_map_of_some_rooms_walked_has_no_frame() {
    let mut fonts = Fonts::new();
    let walked = [(8, 0, "N"), (9, 0, "NE"), (8, 1, "W"), (8, 2, "SW")];
    let map = MapGrid {
        width: 16,
        height: 32,
        inside: None,
        cells: walked
            .iter()
            .map(|&(x, y, walls)| Cell {
                x,
                y,
                walls: walls
                    .chars()
                    .map(|c| match c {
                        'N' => Edge::North,
                        'E' => Edge::East,
                        'S' => Edge::South,
                        _ => Edge::West,
                    })
                    .collect(),
                seen: true,
                ..Cell::default()
            })
            .collect(),
        markers: vec![Marker::new(8, 1, MarkerKind::Player)],
        routes: Vec::new(),
    };
    let panel = Panel {
        level: 2,
        name: "The map you have walked".to_owned(),
        description: String::new(),
        widgets: vec![PanelWidget {
            column: PanelColumn::Left,
            title: None,
            body: PanelBody::Map(map),
        }],
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden(
        "panel-map-walked.png",
        &panel.draw(&mut fonts, PANEL_AT_1).png(),
    );
}

#[test]
fn routes_are_solid_through_rooms_walked_and_side_by_side_where_they_meet() {
    let mut fonts = Fonts::new();
    let walked: Vec<Cell> = [(8, 1), (8, 2), (8, 3), (9, 3), (9, 4)]
        .iter()
        .map(|&(x, y)| Cell {
            x,
            y,
            seen: true,
            ..Cell::default()
        })
        .collect();
    let step = Step::new;
    let map = MapGrid {
        width: 16,
        height: 32,
        inside: None,
        cells: walked,
        markers: vec![
            Marker::new(8, 1, MarkerKind::Player),
            Marker {
                color: Some(RouteColor::Second.color()),
                ..Marker::new(9, 12, MarkerKind::Ring)
            },
        ],
        routes: vec![
            // Through rooms walked, out into one not, then a teleport.
            Route {
                color: RouteColor::First,
                steps: vec![
                    step(8, 1, StepLine::Solid),
                    step(8, 2, StepLine::Solid),
                    step(8, 3, StepLine::Solid),
                    step(9, 3, StepLine::Solid),
                    step(10, 3, StepLine::Dashed),
                    step(3, 20, StepLine::Jump),
                    step(3, 21, StepLine::Dashed),
                ],
                label: Some("Item 1/3".to_owned()),
                exit: Some(Edge::South),
            },
            // The same first steps, then its own way.
            Route {
                color: RouteColor::Second,
                steps: vec![
                    step(8, 1, StepLine::Solid),
                    step(8, 2, StepLine::Solid),
                    step(8, 3, StepLine::Solid),
                    step(9, 3, StepLine::Solid),
                    step(9, 4, StepLine::Solid),
                    step(9, 12, StepLine::Dashed),
                ],
                label: Some("Core".to_owned()),
                exit: Some(Edge::South),
            },
        ],
    };
    let cards = (1..=2)
        .map(|r| CardRow {
            outline: (r == 2).then(|| RouteColor::First.color()),
            label: stand_in_word(&r.to_string()),
            cards: (0..3).map(|c| (stand_in(40 + r * 3 + c), c == 0)).collect(),
        })
        .collect();
    let panel = Panel {
        level: 5,
        name: "Routes".to_owned(),
        description: String::new(),
        widgets: vec![
            PanelWidget {
                column: PanelColumn::Left,
                title: Some("Core".to_owned()),
                body: PanelBody::Icons {
                    columns: 3,
                    icons: (1..=9)
                        .map(|i| Icon {
                            picture: stand_in(i),
                            look: Look::Dim,
                        })
                        .collect(),
                },
            },
            PanelWidget {
                column: PanelColumn::Right,
                title: Some("Doors".to_owned()),
                body: PanelBody::Cards(cards),
            },
            PanelWidget {
                column: PanelColumn::Left,
                title: None,
                body: PanelBody::Map(map),
            },
        ],
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden(
        "panel-map-routes.png",
        &panel.draw(&mut fonts, PANEL_AT_1).png(),
    );
}

#[test]
fn the_map_shows_what_is_inside_each_room() {
    let mut fonts = Fonts::new();
    let edges = |walls: &str| -> Vec<Edge> {
        walls
            .chars()
            .map(|c| match c {
                'N' => Edge::North,
                'E' => Edge::East,
                'S' => Edge::South,
                _ => Edge::West,
            })
            .collect()
    };
    let room = |x, y, walls: &str| Cell {
        x,
        y,
        walls: edges(walls),
        seen: true,
        ..Cell::default()
    };
    let block = |cols: std::ops::Range<usize>, rows: std::ops::Range<usize>| {
        rows.flat_map(|y| cols.clone().map(move |x| (x, y)))
            .collect::<Vec<_>>()
    };
    // A ledge from the left wall parts one room; a door's wall parts the one beside it.
    let ledge = Cell {
        inside: Inside {
            walls: block(0..12, 6..8),
            doors: Vec::new(),
        },
        ..room(8, 1, "W")
    };
    let mut door_wall = block(19..21, 0..9);
    door_wall.extend(block(19..21, 12..18));
    let door = Cell {
        inside: Inside {
            walls: door_wall,
            doors: block(19..21, 9..12),
        },
        ..room(9, 1, "ES")
    };
    let never_entered = Cell {
        dim: true,
        ..room(10, 0, "NES")
    };
    let to_fetch = Cell {
        x: 11,
        y: 3,
        dashed: true,
        ..Cell::default()
    };
    let map = MapGrid {
        width: 16,
        height: 32,
        inside: Some((32, 18)),
        cells: vec![
            room(8, 0, "NW"),
            room(9, 0, "N"),
            ledge,
            door,
            room(8, 2, "WSE"),
            never_entered,
            to_fetch,
        ],
        markers: vec![
            Marker::new(9, 0, MarkerKind::Player),
            Marker {
                at: Some((23, 10)),
                ..Marker::new(9, 1, MarkerKind::Label("1".to_owned()))
            },
            Marker::new(11, 3, MarkerKind::Picture(Some(stand_in(5)))),
        ],
        routes: Vec::new(),
    };
    let panel = Panel {
        level: 4,
        name: "Everything".to_owned(),
        description: String::new(),
        widgets: vec![PanelWidget {
            column: PanelColumn::Left,
            title: None,
            body: PanelBody::Map(map),
        }],
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden(
        "panel-map-inside.png",
        &panel.draw(&mut fonts, PANEL_AT_1).png(),
    );
    golden(
        "panel-map-inside-2x.png",
        &panel.draw(&mut fonts, PANEL_AT_2).png(),
    );
}

#[test]
fn every_marker_in_a_room_shows() {
    let mut fonts = Fonts::new();
    let room = |x, y| Cell {
        x,
        y,
        walls: vec![Edge::North],
        seen: true,
        ..Cell::default()
    };
    let teleport = Marker {
        place: Place::TopLeft,
        ..Marker::new(4, 2, MarkerKind::Diamond)
    };
    let picture = |x, y, seed| Marker::new(x, y, MarkerKind::Picture(Some(stand_in(seed))));
    // BLOB's room holds a teleport and two items; the one beside it, an item and a door.
    let map = MapGrid {
        width: 8,
        height: 4,
        inside: Some((32, 18)),
        cells: vec![room(3, 2), room(4, 2), room(5, 2)],
        markers: vec![
            teleport,
            picture(4, 2, 11),
            picture(4, 2, 14),
            picture(5, 2, 12),
            Marker {
                at: Some((23, 10)),
                ..Marker::new(5, 2, MarkerKind::Label("2".to_owned()))
            },
            Marker::new(4, 2, MarkerKind::Player),
        ],
        routes: Vec::new(),
    };
    let panel = Panel {
        level: 4,
        name: "Everything".to_owned(),
        description: String::new(),
        widgets: vec![PanelWidget {
            column: PanelColumn::Left,
            title: None,
            body: PanelBody::Map(map),
        }],
        training: Vec::new(),
        pad: None,
        hint: HintSlot::Absent,
    };
    golden(
        "panel-map-markers.png",
        &panel.draw(&mut fonts, PANEL_AT_1).png(),
    );
    golden(
        "panel-map-markers-2x.png",
        &panel.draw(&mut fonts, PANEL_AT_2).png(),
    );
}

/// A level-4 panel over stand-ins, with `hint` in its slot.
fn hinted(hint: HintSlot) -> Panel {
    let icons = (1..=9)
        .map(|i| Icon {
            picture: stand_in(i),
            look: if [2, 5, 9].contains(&i) {
                Look::Dim
            } else {
                Look::Bright
            },
        })
        .collect();
    let chips = ["LUMEN", "TOPAZ"]
        .iter()
        .map(|w| Chip {
            glyphs: stand_in_word(w),
            outline: None,
        })
        .collect();
    let door = CardRow {
        outline: Some(sidekick_ui::colors::ACCENT),
        label: stand_in_word("4"),
        cards: (0..3).map(|c| (stand_in(40 + c), false)).collect(),
    };
    Panel {
        level: 4,
        name: "What you have not".to_owned(),
        description: String::new(),
        training: Vec::new(),
        pad: None,
        hint,
        widgets: vec![
            PanelWidget {
                column: PanelColumn::Left,
                title: Some("Core".to_owned()),
                body: PanelBody::Icons { columns: 3, icons },
            },
            PanelWidget {
                column: PanelColumn::Right,
                title: Some("Teleports".to_owned()),
                body: PanelBody::Chips(chips),
            },
            PanelWidget {
                column: PanelColumn::Right,
                title: Some("Doors".to_owned()),
                body: PanelBody::Cards(vec![door]),
            },
            PanelWidget {
                column: PanelColumn::Left,
                title: None,
                body: PanelBody::Map(stand_in_map(false)),
            },
        ],
    }
}

#[test]
fn a_hint_shows_on_a_band_in_a_slot_that_stays_when_empty() {
    let mut fonts = Fonts::new();
    let shown = hinted(HintSlot::Shows(
        "This door's cards are among the violet items".to_owned(),
    ))
    .draw(&mut fonts, PANEL_AT_1);
    let empty = hinted(HintSlot::Empty).draw(&mut fonts, PANEL_AT_1);
    let absent = hinted(HintSlot::Absent).draw(&mut fonts, PANEL_AT_1);
    golden("panel-hint.png", &shown.png());
    golden("panel-hint-empty.png", &empty.png());
    // Below the band, a hint coming or going moves nothing.
    let below = |c: &sidekick_ui::Canvas| c.rgba()[408 * 4 * 101..].to_vec();
    assert_eq!(below(&shown), below(&empty), "the widgets stay put");
    assert_ne!(
        below(&empty),
        below(&absent),
        "a game without hints keeps no slot"
    );
}

#[test]
fn a_hint_fits_the_panel_s_width_or_doesn_t() {
    let mut fonts = Fonts::new();
    let fits = sidekick_ui::hint_width(
        &mut fonts,
        "Platforms low: mind pits you can't climb out of",
    );
    assert!(fits < sidekick_ui::HINT_ROOM, "{fits}");
    let long = sidekick_ui::hint_width(
        &mut fonts,
        "Far too long a hint for one line in the panel, by a good many words",
    );
    assert!(long > sidekick_ui::HINT_ROOM, "{long}");
}

/// Starquake's How to play, its keys as the keyboard plays them, with no pad plugged in.
fn how_to() -> HowTo {
    let entry = |keys: &str, pad: PadMark, name: &str, text: &str| HowToEntry {
        keys: Some(keys.to_owned()),
        pad: Some(pad),
        name: name.to_owned(),
        text: text.to_owned(),
    };
    HowTo {
        title: "Starquake".to_owned(),
        goal: "The planet's core has broken apart. Find the pieces it needs, scattered across the planet, and carry them back to rebuild it. The core shows which pieces it wants only in its own room.".to_owned(),
        entries: vec![
            entry("X", PadMark::Face(Face::East), "Pick up or swap", "Stand on an item to take it. Four at most: a fifth puts down the oldest."),
            entry("Z", PadMark::Face(Face::South), "Build a platform", "A step to cross a gap or climb out of a pit. They run out."),
            entry("← →", PadMark::Directions, "Teleport booths", "Walk into one and type another booth's code. Each booth shows its own."),
            entry("Space", PadMark::Face(Face::West), "Fire", "Clears creatures from your way. The laser runs down as it fires."),
        ],
        clips: Vec::new(),
        show_next: true,
        from_menu: false,
        pad: None,
    }
}

#[test]
fn how_to_play() {
    let mut fonts = Fonts::new();
    let keys = how_to();
    golden("how-to-play.png", &keys.draw(&mut fonts, 1.0).png());
    let pad = HowTo {
        pad: Some(PadLabels::Letters),
        ..how_to()
    };
    golden("how-to-play-pad.png", &pad.draw(&mut fonts, 1.0).png());
    let back = HowTo {
        from_menu: true,
        show_next: false,
        pad: Some(PadLabels::Shapes),
        ..how_to()
    };
    golden("how-to-play-back.png", &back.draw(&mut fonts, 1.0).png());
    let clip = |keys: &str, pad, does: &str| ClipSlot {
        keys: Some(keys.to_owned()),
        pad: Some(pad),
        does: does.to_owned(),
    };
    let clips = HowTo {
        clips: vec![
            clip("X", PadMark::Face(Face::East), "pick up or swap"),
            clip("Z", PadMark::Face(Face::South), "build a platform"),
        ],
        pad: Some(PadLabels::Letters),
        ..how_to()
    };
    golden("how-to-play-clips.png", &clips.draw(&mut fonts, 1.0).png());
    assert!(
        clips.height(&mut fonts) > keys.height(&mut fonts) + 216,
        "the clips' room comes only with clips"
    );
}

#[test]
fn an_entry_too_wide_for_the_card_is_found() {
    let mut fonts = Fonts::new();
    let mut card = HowTo {
        pad: Some(PadLabels::Letters),
        ..how_to()
    };
    assert!(card.too_wide(&mut fonts).is_empty());
    card.entries[2].text =
        "Far too long a line for the card, by a good many words more than it could ever have room for"
            .to_owned();
    assert_eq!(card.too_wide(&mut fonts), [2]);
}

#[test]
fn a_click_finds_the_toggle_and_the_button() {
    let mut fonts = Fonts::new();
    let card = how_to();
    let foot = f32::from(u16::try_from(card.height(&mut fonts)).unwrap()) - 38.0;
    assert_eq!(
        card.target_at(&mut fonts, 40.0, foot),
        Some(HowToTarget::Toggle)
    );
    assert_eq!(
        card.target_at(&mut fonts, 600.0, foot),
        Some(HowToTarget::Button)
    );
    assert_eq!(card.target_at(&mut fonts, 300.0, 100.0), None);
}
