//! The program's screens, compared with golden images byte for byte. `UPDATE_GOLDEN=1`
//! rewrites them; look at a changed image before committing it.

use sidekick_ui::{
    Face, Fonts, Levels, LibraryEntry, LibraryPage, Menu, NoticeHint, PadLabels, PadMark,
    PauseNotice, Question, Refusal, Row, SwitchRow, TapePrompt,
};
use std::path::PathBuf;

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
    }
}

#[test]
fn the_tape_prompt() {
    let mut fonts = Fonts::new();
    let canvas = starquake().draw(&mut fonts, 1);
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
        &prompt.draw(&mut fonts, 1).png(),
    );
}

#[test]
fn the_tape_prompt_with_a_pad_plugged_in() {
    let mut fonts = Fonts::new();
    let prompt = TapePrompt {
        pad: Some(PadLabels::Letters),
        ..starquake()
    };
    golden("tape-prompt-pad.png", &prompt.draw(&mut fonts, 1).png());
}

#[test]
fn at_scale_2_every_logical_pixel_is_four() {
    let mut fonts = Fonts::new();
    let canvas = starquake().draw(&mut fonts, 2);
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
        pad: PadLabels::Letters,
    };
    golden("library.png", &page.draw(&mut fonts, 1).png());
    let playstation = LibraryPage {
        pad: PadLabels::Shapes,
        ..page
    };
    golden(
        "library-playstation.png",
        &playstation.draw(&mut fonts, 1).png(),
    );
}

fn paused(pad: PadLabels) -> PauseNotice {
    PauseNotice {
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
    let card = paused(PadLabels::Letters).draw(&mut fonts, 1);
    assert_eq!(card.size(), (438, 158));
    golden("paused.png", &card.png());
    golden(
        "paused-playstation.png",
        &paused(PadLabels::Shapes).draw(&mut fonts, 1).png(),
    );
    golden(
        "paused-unworded.png",
        &PauseNotice::default().draw(&mut fonts, 1).png(),
    );
}

fn starquake_menu() -> Menu {
    let level = |name: &str, line: &str| (name.to_owned(), line.to_owned());
    let switch = |label: &str| SwitchRow {
        label: label.to_owned(),
        on: false,
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
        actions: ["Back to the game", "End this game", "Exit Starquake"]
            .map(str::to_owned)
            .to_vec(),
        chosen: Row::Level,
        armed: None,
        pad: PadLabels::Letters,
    }
}

#[test]
fn the_guidance_menu() {
    let mut fonts = Fonts::new();
    let menu = starquake_menu();
    golden("menu.png", &menu.draw(&mut fonts, 1).png());

    let mut refused = starquake_menu();
    refused.switches[0].on = true;
    refused.switches[4].refusal =
        Some("Not this tape's code here: the switch can't go on.".to_owned());
    refused.chosen = Row::Switch(4);
    golden("menu-refused.png", &refused.draw(&mut fonts, 1).png());

    let armed = Menu {
        levels: None,
        switches: Vec::new(),
        chosen: Row::Action(1),
        armed: Some((1, "end this game".to_owned())),
        pad: PadLabels::Shapes,
        ..starquake_menu()
    };
    golden("menu-plain-armed.png", &armed.draw(&mut fonts, 1).png());

    let question = Question {
        items: vec!["Guidance level 4".to_owned(), "Full energy".to_owned()],
        pad: PadLabels::Letters,
    };
    golden("question.png", &question.draw(&mut fonts, 1).png());
}

#[test]
fn a_click_finds_the_menu_s_row() {
    let menu = starquake_menu();
    assert_eq!(menu.row_at(100.0, 150.0), Some(Row::Level));
    assert_eq!(menu.row_at(100.0, 285.0), Some(Row::Switch(0)));
    assert_eq!(menu.row_at(100.0, 420.0), Some(Row::Switch(4)));
    assert_eq!(menu.row_at(100.0, 478.0), Some(Row::Action(0)));
    assert_eq!(menu.row_at(100.0, 566.0), Some(Row::Action(2)));
    assert_eq!(menu.row_at(5.0, 478.0), None);
    assert_eq!(menu.row_at(100.0, 451.0), None, "the rule between");
}
