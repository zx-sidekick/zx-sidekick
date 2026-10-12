//! The program's screens, compared with golden images byte for byte. `UPDATE_GOLDEN=1`
//! rewrites them; look at a changed image before committing it.

use sidekick_ui::{Fonts, LibraryEntry, LibraryPage, Refusal, TapePrompt};
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
    };
    golden("library.png", &page.draw(&mut fonts, 1).png());
}
