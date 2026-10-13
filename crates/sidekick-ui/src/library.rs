//! The library: the games installed, to choose one (`docs/mockups/starquake/library.png`).

use crate::{
    colors,
    hints::{hints, Hint, PadLabels, Press},
    Align, Canvas, Color, Fonts, Line, Style, Weight,
};

/// The page's size, in logical pixels: the tape prompt's.
const WIDTH: u32 = crate::tape_prompt::WIDTH;
const HEIGHT: u32 = crate::tape_prompt::HEIGHT;

/// The first card's top, each card's height, and the step from one to the next.
const FIRST: f32 = 186.0;
const CARD: f32 = 72.0;
const STEP: f32 = 84.0;
/// How many cards fit.
const SHOWN: usize = 6;

const CHOSEN_BG: Color = Color::rgb(0x1A, 0x1F, 0x2B);
const CARD_EDGE: Color = Color::rgb(0x2B, 0x2E, 0x36);

/// A game in the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEntry {
    /// Its title.
    pub title: String,
    /// Its year and publisher, as a line.
    pub line: String,
    /// Whether its tape is kept, so it plays at once.
    pub ready: bool,
}

/// The library page: the games, and which one is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryPage {
    /// The games, in the order shown.
    pub games: Vec<LibraryEntry>,
    /// The chosen one.
    pub chosen: usize,
    /// The one pressed, drawn pressed while its press shows.
    pub pressed: Option<usize>,
    /// What the last pad used marks its buttons with.
    pub pad: PadLabels,
}

impl LibraryPage {
    /// The first game shown: the list scrolls so the chosen one stays in view.
    #[must_use]
    pub fn first_shown(&self) -> usize {
        let last_first = self.games.len().saturating_sub(SHOWN);
        self.chosen.saturating_sub(SHOWN - 1).min(last_first)
    }

    /// The game whose card is under a point in logical pixels, if any.
    #[must_use]
    pub fn game_at(&self, x: f32, y: f32) -> Option<usize> {
        if !(120.0..840.0).contains(&x) || y < FIRST {
            return None;
        }
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "y is past FIRST, and the page is a few hundred pixels tall"
        )]
        let row = ((y - FIRST) / STEP) as usize;
        let inside_card = (y - FIRST) % STEP < CARD;
        let game = self.first_shown() + row;
        (row < SHOWN && inside_card && game < self.games.len()).then_some(game)
    }

    /// Draws the page at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let mut c = Canvas::new(WIDTH, HEIGHT, scale);
        c.fill(colors::PROMPT);
        let label = Style::new(11.0, colors::LABEL)
            .weight(Weight::Medium)
            .tracking(0.12);
        c.text(
            fonts,
            "ZX SIDEKICK",
            (120.0, 97.0),
            Fonts::normal_line_height(&label),
            Align::Left,
            &label,
        );
        let heading = Style::new(31.0, colors::TEXT).weight(Weight::SemiBold);
        c.text(
            fonts,
            "Choose a game",
            (120.0, 118.0),
            Fonts::normal_line_height(&heading),
            Align::Left,
            &heading,
        );

        let first = self.first_shown();
        let mut bottom = FIRST;
        for (row, (i, game)) in self
            .games
            .iter()
            .enumerate()
            .skip(first)
            .take(SHOWN)
            .enumerate()
        {
            #[allow(clippy::cast_precision_loss, reason = "a few rows")]
            let y = FIRST + row as f32 * STEP;
            let fill = if self.pressed == Some(i) {
                Some(colors::PRESSED)
            } else {
                (i == self.chosen).then_some(CHOSEN_BG)
            };
            self.card(&mut c, fonts, y, game, fill);
            bottom = y + CARD;
        }

        let note = Style::new(13.0, colors::FAINT);
        c.paragraph(
            fonts,
            "Games come from their definitions. Add one by putting its file in the games folder of ZX Sidekick's data folder; a game that needs its tape asks for it when chosen.",
            (120.0, bottom + 34.0),
            720.0,
            20.0,
            &note,
        );
        let small = Style::new(12.0, colors::FAINT);
        c.text(
            fonts,
            "No game is included: each runs from your own copy.",
            (120.0, 711.0),
            18.0,
            Align::Left,
            &small,
        );
        hints(
            &mut c,
            fonts,
            (840.0, 709.0),
            &[
                Hint {
                    press: &[Press::Arrows("↑↓")],
                    does: "choose",
                },
                Hint {
                    press: &[Press::Key("Enter"), self.pad.confirm()],
                    does: "play",
                },
                Hint {
                    press: &[Press::Key("Esc"), self.pad.back()],
                    does: "quit",
                },
            ],
        );
        c
    }

    #[allow(clippy::unused_self, reason = "drawn as part of the page")]
    /// A game's card; `lit` is its fill when it's chosen or pressed.
    fn card(
        &self,
        c: &mut Canvas,
        fonts: &mut Fonts,
        y: f32,
        game: &LibraryEntry,
        lit: Option<Color>,
    ) {
        let (fill, edge) = lit.map_or((colors::CARD, Line::solid(CARD_EDGE, 1.0)), |fill| {
            (fill, Line::solid(colors::ACCENT, 1.5))
        });
        c.rect((120.0, y, 720.0, CARD), 10.0, Some(fill), Some(edge));
        let title = Style::new(18.0, colors::TEXT).weight(Weight::SemiBold);
        c.text(
            fonts,
            &game.title,
            (144.0, y + 15.0),
            Fonts::normal_line_height(&title),
            Align::Left,
            &title,
        );
        let line = Style::new(13.0, colors::MUTED);
        c.text(
            fonts,
            &game.line,
            (144.0, y + 44.0),
            Fonts::normal_line_height(&line),
            Align::Left,
            &line,
        );
        let (status, color) = if game.ready {
            ("Ready", colors::ACCENT)
        } else {
            ("Needs its tape", colors::FAINT)
        };
        let status_style = Style::new(13.0, color).weight(Weight::Medium);
        c.text(
            fonts,
            status,
            (816.0, y + 25.0),
            Fonts::normal_line_height(&status_style),
            Align::Right,
            &status_style,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(count: usize, chosen: usize) -> LibraryPage {
        LibraryPage {
            games: (0..count)
                .map(|i| LibraryEntry {
                    title: format!("Game {i}"),
                    line: String::new(),
                    ready: false,
                })
                .collect(),
            chosen,
            pressed: None,
            pad: PadLabels::Letters,
        }
    }

    #[test]
    fn a_click_finds_the_card_under_it() {
        let p = page(3, 0);
        assert_eq!(p.game_at(400.0, FIRST + 10.0), Some(0));
        assert_eq!(p.game_at(400.0, FIRST + STEP + 10.0), Some(1));
        assert_eq!(
            p.game_at(400.0, FIRST + CARD + 5.0),
            None,
            "the gap between cards"
        );
        assert_eq!(p.game_at(100.0, FIRST + 10.0), None, "left of the cards");
        assert_eq!(
            p.game_at(400.0, FIRST + 3.0 * STEP + 10.0),
            None,
            "past the last game"
        );
    }

    #[test]
    fn the_list_scrolls_to_keep_the_chosen_game_in_view() {
        assert_eq!(page(10, 0).first_shown(), 0);
        assert_eq!(page(10, 5).first_shown(), 0);
        assert_eq!(page(10, 6).first_shown(), 1);
        assert_eq!(page(10, 9).first_shown(), 4);
        assert_eq!(page(3, 2).first_shown(), 0);
        assert_eq!(page(10, 7).game_at(400.0, FIRST + 10.0), Some(2));
    }
}
