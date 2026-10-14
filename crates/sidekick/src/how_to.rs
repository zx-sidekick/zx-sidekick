//! The How to play screen's state: shown when a game starts, unless the player turned "Show
//! next time" off (kept in the game's folder), and from the menu's row. No SDL here; the window
//! feeds it and draws its card.

use crate::{compose::host_keys, keys, pad::Mapping};
use sidekick_def::{Definition, Role};
use sidekick_ui::{Face, Fonts, HowTo, HowToEntry, PadLabels, PadMark};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// The file in a game's folder that keeps whether the screen shows when a game starts.
pub(crate) const FILE: &str = "how-to-play.txt";

/// What the screen is asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    /// Turns "Show next time" over.
    Toggle,
    /// Play, or Back to the menu.
    Close,
}

/// What came of an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    /// Closed, back to the game.
    Play,
    /// Closed, back to the menu it was opened from.
    Menu,
}

/// The screen, and whether it shows when the next game starts.
#[derive(Debug)]
pub(crate) struct HowToPlay {
    /// Open, and whether from the menu.
    open: Option<bool>,
    show_next: bool,
    /// Where the setting is kept; none for a tape played plainly.
    folder: Option<PathBuf>,
    /// Whether a change is kept there: not for a crafted situation.
    keep: bool,
}

impl HowToPlay {
    /// The screen for a game whose setting is kept in `folder`, read from there; a change is
    /// kept there when `keep`.
    pub(crate) fn new(folder: Option<PathBuf>, keep: bool) -> Self {
        Self {
            open: None,
            show_next: folder.as_deref().is_none_or(shows),
            folder,
            keep,
        }
    }

    /// Whether it's open.
    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// A game started: the screen opens, when the definition has one and it's to show.
    pub(crate) fn game_started(&mut self, definition: Option<&Definition>) {
        if self.show_next && definition.is_some_and(|d| d.about.is_some()) {
            self.open = Some(false);
        }
    }

    /// Opened from the menu's row.
    pub(crate) const fn open_from_menu(&mut self) {
        self.open = Some(true);
    }

    /// Closed with no more said: a save loaded, say.
    pub(crate) const fn close(&mut self) {
        self.open = None;
    }

    /// Takes an input while it's open. Turning "Show next time" over keeps it at once.
    pub(crate) fn input(&mut self, input: Input) -> Outcome {
        let Some(from_menu) = self.open else {
            return Outcome::Stay;
        };
        match input {
            Input::Toggle => {
                self.show_next = !self.show_next;
                if let Some(folder) = self.folder.as_ref().filter(|_| self.keep) {
                    if let Err(e) = keep(folder, self.show_next) {
                        eprintln!(
                            "{}: How to play's setting couldn't be kept: {e}",
                            folder.display()
                        );
                    }
                }
                Outcome::Stay
            }
            Input::Close => {
                self.open = None;
                if from_menu {
                    Outcome::Menu
                } else {
                    Outcome::Play
                }
            }
        }
    }

    /// The card, while it's open, for the game `definition` describes, its keys from
    /// `mapping` in play, the keyboard as the menu set it (`keyboard`, the Spectrum's turned on
    /// for the session, or as the definition says), and its buttons as the pad plugged in, `pad`, marks
    /// them.
    pub(crate) fn card(
        &self,
        definition: Option<&Definition>,
        mapping: &Mapping,
        title: &str,
        pad: Option<PadLabels>,
        keyboard: bool,
    ) -> Option<HowTo> {
        let from_menu = self.open?;
        let definition = definition?;
        let about = definition.about.as_ref()?;
        let play = definition.state_for(Role::Play);
        let spectrum = keys::spectrum_on(Some(definition), play, keyboard);
        let mac = cfg!(target_os = "macos");
        Some(HowTo {
            title: title.to_owned(),
            goal: about.goal.clone(),
            entries: about
                .how
                .iter()
                .map(|how| HowToEntry {
                    keys: how
                        .keys
                        .clone()
                        .or_else(|| {
                            how.pad
                                .as_deref()
                                .and_then(|part| mapping.keyboard_for(play, part, spectrum))
                        })
                        .map(|keys| host_keys(&keys, mac)),
                    pad: how.pad.as_deref().and_then(mark),
                    name: how.name.clone(),
                    text: how.text.clone(),
                })
                .collect(),
            clips: Vec::new(),
            show_next: self.show_next,
            from_menu,
            pad,
        })
    }
}

/// The entries of a definition's How to play whose name or line is too wide for its card, as
/// it's drawn with a pad plugged in, by index.
pub(crate) fn too_wide(definition: &Definition, fonts: &mut Fonts) -> Vec<usize> {
    let mut screen = HowToPlay::new(None, false);
    screen.open_from_menu();
    let mapping = Mapping::new(Some(definition));
    screen
        .card(
            Some(definition),
            &mapping,
            &definition.game.title,
            Some(PadLabels::Letters),
            false,
        )
        .map(|card| card.too_wide(fonts))
        .unwrap_or_default()
}

/// How a card marks a pad's part, by the name a definition gives it.
pub(crate) fn mark(part: &str) -> Option<PadMark> {
    Some(match part {
        "west" => PadMark::Face(Face::West),
        "south" => PadMark::Face(Face::South),
        "east" => PadMark::Face(Face::East),
        "north" => PadMark::Face(Face::North),
        "dpad" | "stick" => PadMark::Directions,
        "start" => PadMark::Start,
        _ => return None,
    })
}

/// Whether the screen shows when a game starts, as kept in `folder`: yes unless turned off.
fn shows(folder: &Path) -> bool {
    fs::read_to_string(folder.join(FILE)).map_or(true, |text| text.trim() != "off")
}

/// Keeps whether the screen shows when a game starts.
fn keep(folder: &Path, show: bool) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    fs::write(folder.join(FILE), if show { "on\n" } else { "off\n" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(about: &str) -> Definition {
        Definition::from_toml(&format!(
            r#"
            schema = 1
            [game]
            id = "test"
            title = "Test"
            machine = "48k"
            [[media]]
            name = "the tape"
            [[states]]
            name = "play"
            role = "play"
            pc = 0x8000
            [pad.default]
            east = "pick-up"
            [[actions]]
            id = "pick-up"
            label = "Pick up"
            joystick = ["up"]
            {about}
            "#
        ))
        .expect("the test definition parses")
    }

    const ABOUT: &str = r#"
        [about]
        goal = "Find the pieces."
        [[about.how]]
        name = "Pick up"
        text = "Stand on it."
        pad = "east"
        [[about.how]]
        name = "Booths"
        text = "Walk in."
        keys = "← →"
        pad = "dpad"
    "#;

    #[test]
    fn it_shows_when_a_game_starts_until_turned_off_and_stays_off() {
        let folder = tempfile::tempdir().unwrap();
        let def = definition(ABOUT);
        let mut screen = HowToPlay::new(Some(folder.path().to_owned()), true);
        screen.game_started(Some(&def));
        assert!(screen.is_open(), "on by default");
        assert_eq!(screen.input(Input::Toggle), Outcome::Stay);
        assert_eq!(
            fs::read_to_string(folder.path().join(FILE)).unwrap(),
            "off\n",
            "kept at once"
        );
        assert_eq!(screen.input(Input::Close), Outcome::Play);
        assert!(!screen.is_open());
        screen.game_started(Some(&def));
        assert!(!screen.is_open(), "not at the next game");

        let mut next_run = HowToPlay::new(Some(folder.path().to_owned()), true);
        next_run.game_started(Some(&def));
        assert!(!next_run.is_open(), "nor the next time the program runs");
        next_run.open_from_menu();
        assert!(next_run.is_open(), "the menu opens it whatever");
        next_run.input(Input::Toggle);
        assert_eq!(
            next_run.input(Input::Close),
            Outcome::Menu,
            "back to the menu"
        );
        next_run.game_started(Some(&def));
        assert!(next_run.is_open(), "turned back on");
    }

    #[test]
    fn a_line_too_wide_for_the_card_is_found() {
        let mut fonts = Fonts::new();
        assert!(too_wide(&definition(ABOUT), &mut fonts).is_empty());
        let long = ABOUT.replace("Walk in.", &"Walk in and on. ".repeat(12));
        assert_eq!(too_wide(&definition(&long), &mut fonts), [1]);
    }

    #[test]
    fn a_game_without_one_shows_none() {
        let mut screen = HowToPlay::new(None, true);
        screen.game_started(Some(&definition("")));
        assert!(!screen.is_open());
        screen.game_started(None);
        assert!(!screen.is_open());
        assert_eq!(
            screen.input(Input::Close),
            Outcome::Stay,
            "nothing to close"
        );
    }

    #[test]
    fn without_a_folder_turning_it_off_lasts_the_session() {
        let def = definition(ABOUT);
        let mut screen = HowToPlay::new(None, true);
        screen.game_started(Some(&def));
        screen.input(Input::Toggle);
        screen.input(Input::Close);
        screen.game_started(Some(&def));
        assert!(!screen.is_open());
    }

    #[test]
    fn a_situation_reads_the_setting_and_leaves_it() {
        let folder = tempfile::tempdir().unwrap();
        let def = definition(ABOUT);
        let mut screen = HowToPlay::new(Some(folder.path().to_owned()), false);
        screen.game_started(Some(&def));
        screen.input(Input::Toggle);
        assert!(!folder.path().join(FILE).exists(), "not kept");
        keep(folder.path(), false).unwrap();
        let mut off = HowToPlay::new(Some(folder.path().to_owned()), false);
        off.game_started(Some(&def));
        assert!(!off.is_open(), "the player's setting holds");
    }

    #[test]
    fn the_card_has_the_definition_s_words_and_the_keys_that_play_them() {
        let def = definition(ABOUT);
        let mapping = Mapping::new(Some(&def));
        let mut screen = HowToPlay::new(None, true);
        assert!(
            screen
                .card(Some(&def), &mapping, "Test", None, false)
                .is_none(),
            "closed"
        );
        screen.game_started(Some(&def));
        let card = screen
            .card(
                Some(&def),
                &mapping,
                "Test",
                Some(PadLabels::Letters),
                false,
            )
            .expect("open");
        assert_eq!(card.goal, "Find the pieces.");
        assert_eq!(
            card.entries[0].keys.as_deref(),
            Some("X"),
            "the keyboard's east, as a pad"
        );
        let spectrum = screen
            .card(Some(&def), &mapping, "Test", None, true)
            .expect("open");
        assert_eq!(
            spectrum.entries[0].keys.as_deref(),
            Some("↑"),
            "with the Spectrum's keyboard on, the arrow that does the same"
        );
        assert_eq!(card.entries[0].pad, Some(PadMark::Face(Face::East)));
        assert_eq!(card.entries[1].keys.as_deref(), Some("← →"), "as written");
        assert_eq!(card.entries[1].pad, Some(PadMark::Directions));
        assert!(!card.from_menu);
        assert!(card.show_next);
        assert_eq!(card.pad, Some(PadLabels::Letters));
        screen.input(Input::Close);
        screen.open_from_menu();
        let card = screen
            .card(Some(&def), &mapping, "Test", None, false)
            .expect("open");
        assert!(card.from_menu, "its button reads Back");
        assert_eq!(card.pad, None, "keys alone without a pad");
    }
}
