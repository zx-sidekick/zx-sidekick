//! The How to play screen: a card over the paused game with the game's goal and its main moves,
//! shown when a game starts and from the menu (`docs/mockups/2026-10-14-how-to-play.png`).

use crate::{
    colors,
    hints::{hints_from, hints_on_accent, row_width, Hint, PadLabels, Press, GAP},
    menu::toggle,
    pause::{presses, PadMark},
    Align, Canvas, Color, Fonts, Line, Style, Weight,
};

/// The card's width, in logical pixels. Its height follows what's in it.
pub const WIDTH: u32 = 648;

const CARD_EDGE: Color = Color::rgb(0x2B, 0x2E, 0x36);
const SIDE: f32 = 27.0;
/// A clip's place: a crop of the game's picture at its own scale, 96 × 72 Spectrum pixels.
const CLIP: (f32, f32) = (288.0, 216.0);
const CLIP_GAP: f32 = 18.0;
const GOAL_LINE: f32 = 22.0;
const ENTRY_STEP: f32 = 44.0;
const BUTTON_HEIGHT: f32 = 40.0;

const TITLE: Style = Style::new(24.0, colors::TEXT).weight(Weight::Bold);
const HEADER: Style = Style::new(12.0, colors::MUTED);
const GOAL: Style = Style::new(15.0, colors::BODY);
const NAME: Style = Style::new(14.0, colors::TEXT).weight(Weight::SemiBold);
const TEXT: Style = Style::new(13.0, colors::MUTED);
const TOGGLE: Style = Style::new(14.0, colors::TEXT).weight(Weight::Medium);
const BUTTON: Style = Style::new(16.0, colors::PROMPT).weight(Weight::SemiBold);
const CLIP_MARK: Style = Style::new(12.0, colors::FAINT);

/// One of the main moves: what to press, its name and a line on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HowToEntry {
    /// The keyboard's key as shown, or arrows.
    pub keys: Option<String>,
    /// The pad's part, shown while a pad is plugged in.
    pub pad: Option<PadMark>,
    /// Its name.
    pub name: String,
    /// One line on it.
    pub text: String,
}

/// A clip's place on the card, and what it shows being pressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipSlot {
    /// The keyboard's key as shown, or arrows.
    pub keys: Option<String>,
    /// The pad's part, shown while a pad is plugged in.
    pub pad: Option<PadMark>,
    /// What it does.
    pub does: String,
}

/// What a click on the card lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HowToTarget {
    /// The "Show next time" toggle.
    Toggle,
    /// The button: Play, or Back to the menu.
    Button,
}

/// The How to play card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HowTo {
    /// The game's title, at the top right.
    pub title: String,
    /// The goal, in a sentence or two.
    pub goal: String,
    /// The main moves.
    pub entries: Vec<HowToEntry>,
    /// The clips' places, side by side above the goal; none leaves the card shorter.
    pub clips: Vec<ClipSlot>,
    /// Whether it shows when the next game starts.
    pub show_next: bool,
    /// Opened from the menu: the button reads Back, and goes back to it.
    pub from_menu: bool,
    /// How the pad plugged in marks its buttons; `None` without one, which shows keys alone.
    pub pad: Option<PadLabels>,
}

/// Where the card's parts go.
struct Layout {
    goal: f32,
    entries: f32,
    badges: f32,
    rule: f32,
    button: (f32, f32, f32, f32),
    toggle: (f32, f32, f32, f32),
    height: f32,
}

#[allow(clippy::cast_precision_loss, reason = "a card's few lines and entries")]
impl HowTo {
    const fn button_words(&self) -> &'static str {
        if self.from_menu {
            "Back"
        } else {
            "Play"
        }
    }

    fn button_keys(&self) -> Vec<Press<'static>> {
        let mut keys = vec![Press::Key("Enter")];
        keys.extend(self.pad.map(PadLabels::confirm));
        keys
    }

    fn layout(&self, fonts: &mut Fonts) -> Layout {
        let width = WIDTH as f32;
        let mut y = 70.0;
        if !self.clips.is_empty() {
            y += CLIP.1 + 12.0 + 20.0 + 22.0;
        }
        let goal = y;
        let goal_lines = fonts.wrap(&self.goal, &GOAL, width - 2.0 * SIDE).len();
        y += goal_lines as f32 * GOAL_LINE + 18.0;
        let entries = y;
        y += self.entries.len() as f32 * ENTRY_STEP + 4.0;
        let badges = self
            .entries
            .iter()
            .map(|e| press_width(fonts, &presses(e.keys.as_deref(), e.pad, self.pad)))
            .fold(0.0, f32::max);
        let rule = y;
        let foot = rule + 17.0;
        let keys = self.button_keys();
        let words = fonts.measure(self.button_words(), &BUTTON);
        let button_width = 20.0 + words + 12.0 + press_width(fonts, &keys) + 20.0;
        let button = (
            width - SIDE - button_width,
            foot,
            button_width,
            BUTTON_HEIGHT,
        );
        let toggle_width = 40.0 + 12.0 + fonts.measure("Show next time", &TOGGLE);
        Layout {
            goal,
            entries,
            badges,
            rule,
            button,
            toggle: (SIDE, foot, toggle_width, BUTTON_HEIGHT),
            height: foot + BUTTON_HEIGHT + 18.0,
        }
    }

    /// The card's height, in logical pixels.
    #[must_use]
    pub fn height(&self, fonts: &mut Fonts) -> u32 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a card's height"
        )]
        let height = self.layout(fonts).height.ceil() as u32;
        height
    }

    /// Where the entries' names and lines start, after the badges' column.
    fn words_left(layout: &Layout) -> f32 {
        SIDE + layout.badges + if layout.badges > 0.0 { 16.0 } else { 0.0 }
    }

    /// The entries whose name or line is wider than the card has room for, by index.
    #[must_use]
    pub fn too_wide(&self, fonts: &mut Fonts) -> Vec<usize> {
        let layout = self.layout(fonts);
        let room = WIDTH as f32 - SIDE - Self::words_left(&layout);
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                fonts.measure(&e.name, &NAME) > room || fonts.measure(&e.text, &TEXT) > room
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// What a point on the card, in logical pixels, lands on.
    #[must_use]
    pub fn target_at(&self, fonts: &mut Fonts, x: f32, y: f32) -> Option<HowToTarget> {
        let layout = self.layout(fonts);
        let inside = |(left, top, w, h): (f32, f32, f32, f32)| {
            (left..left + w).contains(&x) && (top..top + h).contains(&y)
        };
        if inside(layout.button) {
            Some(HowToTarget::Button)
        } else if inside(layout.toggle) {
            Some(HowToTarget::Toggle)
        } else {
            None
        }
    }

    /// Draws the card at `scale` device pixels to a logical one; outside its rounded corners
    /// stays clear.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: f32) -> Canvas {
        let layout = self.layout(fonts);
        let mut c = Canvas::new(WIDTH, self.height(fonts), scale);
        let width = WIDTH as f32;
        c.rect(
            (0.0, 0.0, width, layout.height),
            12.0,
            Some(colors::CARD),
            Some(Line::solid(CARD_EDGE, 1.0)),
        );
        let line = Fonts::normal_line_height(&TITLE);
        c.text(
            fonts,
            "How to play",
            (SIDE, 22.0),
            line,
            Align::Left,
            &TITLE,
        );
        let header = format!("{}  ·  the game is paused", self.title);
        c.text(
            fonts,
            &header,
            (width - SIDE, 29.0),
            16.0,
            Align::Right,
            &HEADER,
        );

        for (i, clip) in self.clips.iter().enumerate() {
            let x = SIDE + i as f32 * (CLIP.0 + CLIP_GAP);
            self.clip_slot(&mut c, fonts, (x, 68.0), clip);
        }

        c.paragraph(
            fonts,
            &self.goal,
            (SIDE, layout.goal),
            width - 2.0 * SIDE,
            GOAL_LINE,
            &GOAL,
        );

        let words = Self::words_left(&layout);
        for (i, entry) in self.entries.iter().enumerate() {
            let y = layout.entries + i as f32 * ENTRY_STEP;
            let press = presses(entry.keys.as_deref(), entry.pad, self.pad);
            if !press.is_empty() {
                let hint = [Hint {
                    press: &press,
                    does: "",
                }];
                // Right-aligned in their column, so the names line up.
                let left = SIDE + layout.badges - press_width(fonts, &press);
                hints_from(&mut c, fonts, (left, y + 9.0), &hint);
            }
            c.text(fonts, &entry.name, (words, y), 20.0, Align::Left, &NAME);
            c.text(
                fonts,
                &entry.text,
                (words, y + 20.0),
                18.0,
                Align::Left,
                &TEXT,
            );
        }

        c.rect(
            (1.0, layout.rule, width - 2.0, 1.0),
            0.0,
            Some(CARD_EDGE),
            None,
        );
        let (tx, ty, _, _) = layout.toggle;
        toggle(&mut c, (tx, ty + 9.0), if self.show_next { 100 } else { 0 });
        c.text(
            fonts,
            "Show next time",
            (tx + 52.0, ty),
            BUTTON_HEIGHT,
            Align::Left,
            &TOGGLE,
        );

        let (bx, by, bw, bh) = layout.button;
        c.rect((bx, by, bw, bh), 6.0, Some(colors::ACCENT), None);
        let advance = c.text(
            fonts,
            self.button_words(),
            (bx + 20.0, by),
            bh,
            Align::Left,
            &BUTTON,
        );
        let keys = self.button_keys();
        hints_on_accent(
            &mut c,
            fonts,
            (bx + 20.0 + advance + 12.0, by + 10.0),
            &[Hint {
                press: &keys,
                does: "",
            }],
        );
        c
    }

    /// A clip's place, marked, with what it shows pressed under it.
    fn clip_slot(&self, c: &mut Canvas, fonts: &mut Fonts, (x, y): (f32, f32), clip: &ClipSlot) {
        c.rect(
            (x, y, CLIP.0, CLIP.1),
            6.0,
            None,
            Some(Line {
                color: colors::EDGE,
                width: 1.0,
                dash: Some((6.0, 4.0)),
            }),
        );
        c.text(
            fonts,
            "The game runs here",
            (x + CLIP.0 / 2.0, y + CLIP.1 / 2.0 - 9.0),
            18.0,
            Align::Center,
            &CLIP_MARK,
        );
        let press = presses(clip.keys.as_deref(), clip.pad, self.pad);
        let hint = [Hint {
            press: &press,
            does: &clip.does,
        }];
        let width = row_width(fonts, &hint);
        let under = y + CLIP.1 + 12.0;
        hints_from(c, fonts, (x + (CLIP.0 - width) / 2.0, under), &hint);
    }
}

/// The width of badges alone, with no words after them.
fn press_width(fonts: &mut Fonts, press: &[Press]) -> f32 {
    if press.is_empty() {
        return 0.0;
    }
    row_width(fonts, &[Hint { press, does: "" }]) - GAP
}
