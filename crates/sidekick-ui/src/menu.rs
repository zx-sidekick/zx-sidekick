//! The guidance menu: a card over the game, built from its definition
//! (`docs/mockups/starquake/menu.png`), and the question it asks before the score shows more.

use crate::{
    colors,
    hints::{hints_from, Hint, PadLabels, Press},
    Align, Canvas, Color, Fonts, Line, Style, Weight,
};

/// The card's width, in logical pixels. Its height follows what's in it.
pub const WIDTH: u32 = 518;

const CARD_EDGE: Color = Color::rgb(0x2B, 0x2E, 0x36);
const CHOSEN_BG: Color = Color::rgb(0x1A, 0x1F, 0x2B);
const BAR_OFF: Color = Color::rgb(0x2E, 0x32, 0x3C);
const BODY: Color = Color::rgb(0xC4, 0xC7, 0xCE);
const SWITCH_TEXT: Color = Color::rgb(0xD6, 0xD8, 0xDD);
const PILL_OFF: Color = Color::rgb(0x2A, 0x2D, 0x35);

/// The guidance levels, and the one chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Levels {
    /// Each level's name and one-line description, from 0 up.
    pub levels: Vec<(String, String)>,
    /// The level chosen.
    pub level: usize,
}

/// A training switch's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchRow {
    /// Its label.
    pub label: String,
    /// Whether it's on.
    pub on: bool,
    /// Where its knob is, from 0 at the left (off) to 100 at the right (on); between them while
    /// it slides.
    pub knob: u8,
    /// Why it wouldn't go on, when it wouldn't.
    pub refusal: Option<String>,
}

/// A row of the menu that can be chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// The guidance level.
    Level,
    /// A training switch, by index.
    Switch(usize),
    /// An action (Back to the game, End this game, Exit), by index.
    Action(usize),
    /// One of the saves row's buttons: 0 Save, 1 Load, 2 Open a save….
    Save(usize),
}

/// The guidance menu, as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// The guidance levels, when the game has them.
    pub levels: Option<Levels>,
    /// The training switches.
    pub switches: Vec<SwitchRow>,
    /// The actions, in order: "Back to the game" first.
    pub actions: Vec<String>,
    /// The row chosen.
    pub chosen: Row,
    /// The row or button pressed, drawn pressed while its press shows.
    pub pressed: Option<Row>,
    /// An action pressed once, waiting for a second press: its index, and what the second
    /// press does ("end this game").
    pub armed: Option<(usize, String)>,
    /// How the last pad used marks its buttons.
    pub pad: PadLabels,
    /// The saves row, after the first action, when saves can be made: whether there's a save
    /// to load.
    pub saves: Option<bool>,
}

/// The saves row's buttons: their words and keys.
const SAVE_BUTTONS: [(&str, &str); 3] = [("Save", "F5"), ("Load", "F9"), ("Open a save…", "")];
/// From the first action to the saves row, and the room it takes.
const TO_SAVES: f32 = 33.0;
const SAVES_ROOM: f32 = 44.0;

/// Where everything goes, for a menu's contents.
struct Layout {
    /// The level box's top.
    level: Option<f32>,
    /// The training section's label's top.
    training: Option<f32>,
    /// Each switch row's text top.
    switches: Vec<f32>,
    /// The rule above the actions.
    rule: f32,
    /// The saves row's top.
    saves: Option<f32>,
    /// Each action's text top.
    actions: Vec<f32>,
    /// The rule above the hints.
    bottom_rule: f32,
    height: f32,
}

/// From the header to the first section.
const FIRST: f32 = 56.0;
const LEVEL_BOX: f32 = 182.0;
/// From the level box to what follows it.
const AFTER_BOX: f32 = 16.0;
/// From the training label to the first switch, and from one switch to the next.
const TO_SWITCHES: f32 = 26.0;
const SWITCH_STEP: f32 = 34.0;
/// A refusal's line under its switch.
const REFUSAL: f32 = 18.0;
/// From the rule to the first action, and from one action to the next.
const TO_ACTIONS: f32 = 20.0;
const ACTION_STEP: f32 = 44.0;
/// From the last action's text to the bottom rule, and from that to the card's bottom.
const TO_BOTTOM_RULE: f32 = 40.0;
const TO_BOTTOM: f32 = 51.0;

impl Menu {
    fn layout(&self) -> Layout {
        let mut y = FIRST;
        let level = self.levels.as_ref().map(|_| {
            let top = y;
            y += LEVEL_BOX + AFTER_BOX;
            top
        });
        let training = (!self.switches.is_empty()).then_some(y);
        let mut switches = Vec::new();
        if training.is_some() {
            y += TO_SWITCHES;
            for switch in &self.switches {
                switches.push(y);
                y += SWITCH_STEP;
                if switch.refusal.is_some() {
                    y += REFUSAL;
                }
            }
        }
        let rule = y + 1.0;
        let room = if self.saves.is_some() {
            SAVES_ROOM
        } else {
            0.0
        };
        #[allow(clippy::cast_precision_loss, reason = "a few actions")]
        let actions: Vec<f32> = (0..self.actions.len())
            .map(|i| {
                let after_saves = if i > 0 { room } else { 0.0 };
                rule + TO_ACTIONS + i as f32 * ACTION_STEP + after_saves
            })
            .collect();
        let saves = self
            .saves
            .and_then(|_| actions.first().map(|first| first + TO_SAVES));
        let bottom_rule = actions
            .last()
            .map_or(rule + TO_ACTIONS, |a| a + TO_BOTTOM_RULE);
        Layout {
            level,
            training,
            switches,
            rule,
            saves,
            actions,
            bottom_rule,
            height: bottom_rule + TO_BOTTOM,
        }
    }

    /// The card's height, in logical pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a card's height"
        )]
        let height = self.layout().height as u32;
        height
    }

    /// The row under a point on the card, in logical pixels.
    #[must_use]
    pub fn row_at(&self, x: f32, y: f32) -> Option<Row> {
        if !(12.0..506.0).contains(&x) {
            return None;
        }
        let layout = self.layout();
        if let Some(top) = layout.level {
            if (top..top + LEVEL_BOX).contains(&y) {
                return Some(Row::Level);
            }
        }
        if let Some(top) = layout.saves {
            if (top..top + 39.0).contains(&y) {
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped to the three buttons first"
                )]
                let button = ((x - 12.0) / 168.0).clamp(0.0, 2.0) as usize;
                return Some(Row::Save(button));
            }
        }
        let within = |top: f32| (top - 9.0..top + 25.0).contains(&y);
        let switch = layout.switches.iter().position(|&top| within(top));
        let action = layout.actions.iter().position(|&top| within(top));
        switch.map(Row::Switch).or_else(|| action.map(Row::Action))
    }

    /// Draws the card at `scale` device pixels to a logical one; outside its rounded corners
    /// stays clear.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let layout = self.layout();
        let mut c = Canvas::new(WIDTH, self.height(), scale);
        #[allow(clippy::cast_precision_loss, reason = "a card's size")]
        let w = WIDTH as f32;
        c.rect(
            (0.0, 0.0, w, layout.height),
            12.0,
            Some(colors::CARD),
            Some(Line::solid(CARD_EDGE, 1.0)),
        );
        let title = Style::new(20.0, colors::TEXT).weight(Weight::SemiBold);
        text(&mut c, fonts, "Guidance", (27.0, 20.0), Align::Left, &title);
        let paused = Style::new(12.0, colors::MUTED);
        span(
            &mut c,
            fonts,
            "The game is paused",
            (369.0, 27.0),
            Align::Left,
            &paused,
        );

        if let (Some(top), Some(levels)) = (layout.level, &self.levels) {
            self.level_box(&mut c, fonts, top, levels);
        }
        if let Some(top) = layout.training {
            let label = Style::new(11.0, colors::LABEL)
                .weight(Weight::Medium)
                .tracking(0.12);
            text(
                &mut c,
                fonts,
                "TRAINING MODE",
                (27.0, top),
                Align::Left,
                &label,
            );
        }
        for (i, (switch, &top)) in self.switches.iter().zip(&layout.switches).enumerate() {
            self.switch_row(&mut c, fonts, top, switch, self.lit(Row::Switch(i)));
        }
        c.rect((1.0, layout.rule, w - 2.0, 1.0), 0.0, Some(CARD_EDGE), None);
        for (i, (action, &top)) in self.actions.iter().zip(&layout.actions).enumerate() {
            self.action_row(&mut c, fonts, top, i, action);
        }
        if let (Some(top), Some(can_load)) = (layout.saves, self.saves) {
            self.saves_row(&mut c, fonts, top, can_load);
        }
        c.rect(
            (1.0, layout.bottom_rule, w - 2.0, 1.0),
            0.0,
            Some(CARD_EDGE),
            None,
        );
        hints_from(
            &mut c,
            fonts,
            (27.0, layout.bottom_rule + 17.0),
            &[
                Hint {
                    press: &[Press::Arrows("↑↓")],
                    does: "choose",
                },
                Hint {
                    press: &[Press::Arrows("←→")],
                    does: "change",
                },
                Hint {
                    press: &[Press::Key("Enter"), self.pad.confirm()],
                    does: "OK",
                },
                Hint {
                    press: &[Press::Key("Esc"), self.pad.back()],
                    does: "cancel",
                },
            ],
        );
        c
    }

    /// A row's fill when it's lit: pressed, or chosen.
    fn lit(&self, row: Row) -> Option<Color> {
        if self.pressed == Some(row) {
            Some(colors::PRESSED)
        } else {
            (self.chosen == row).then_some(CHOSEN_BG)
        }
    }

    fn chosen_box(c: &mut Canvas, (x, y, w, h): (f32, f32, f32, f32), fill: Color) {
        c.rect(
            (x, y, w, h),
            8.0,
            Some(fill),
            Some(Line::solid(colors::ACCENT, 1.5)),
        );
    }

    fn level_box(&self, c: &mut Canvas, fonts: &mut Fonts, top: f32, levels: &Levels) {
        let lit = self.lit(Row::Level);
        let chosen = lit.is_some();
        if let Some(fill) = lit {
            Self::chosen_box(c, (12.0, top, 494.0, LEVEL_BOX), fill);
        } else {
            c.rect(
                (12.0, top, 494.0, LEVEL_BOX),
                8.0,
                None,
                Some(Line::solid(CARD_EDGE, 1.0)),
            );
        }
        let accent = if chosen {
            colors::ACCENT
        } else {
            colors::LABEL
        };
        let label = Style::new(11.0, accent)
            .weight(Weight::Medium)
            .tracking(0.12);
        text(
            c,
            fonts,
            "GUIDANCE LEVEL",
            (27.0, top + 15.0),
            Align::Left,
            &label,
        );
        if chosen {
            let (y0, y1) = (top + 63.0, top + 76.0);
            let middle = f32::midpoint(y0, y1);
            c.polygon(&[(39.0, y0), (39.0, y1), (29.0, middle)], colors::ACCENT);
            c.polygon(&[(479.0, y0), (479.0, y1), (489.0, middle)], colors::ACCENT);
        }
        let (name, description) = levels.levels.get(levels.level).cloned().unwrap_or_default();
        let number = Style::new(40.0, colors::TEXT).weight(Weight::SemiBold);
        let level = levels.level.to_string();
        text(
            c,
            fonts,
            &level,
            (259.0, top + 31.0),
            Align::Center,
            &number,
        );
        let name_style = Style::new(17.0, colors::TEXT).weight(Weight::Medium);
        text(
            c,
            fonts,
            &name,
            (259.0, top + 79.0),
            Align::Center,
            &name_style,
        );

        let bars = levels.levels.len().saturating_sub(1).max(1);
        #[allow(clippy::cast_precision_loss, reason = "a few levels")]
        let (count, filled) = (bars as f32, levels.level);
        let bar = (462.0 - 6.0 * (count - 1.0)) / count;
        for i in 0..bars {
            #[allow(clippy::cast_precision_loss, reason = "a few levels")]
            let x = 27.0 + i as f32 * (bar + 6.0);
            let color = if i < filled { accent } else { BAR_OFF };
            c.rect((x, top + 115.0, bar, 7.0), 4.0, Some(color), None);
        }
        let small = Style::new(11.0, colors::FAINT);
        span(
            c,
            fonts,
            "less help",
            (27.0, top + 129.0),
            Align::Left,
            &small,
        );
        span(
            c,
            fonts,
            "more help",
            (434.0, top + 129.0),
            Align::Left,
            &small,
        );
        let line = Style::new(13.0, BODY);
        text(
            c,
            fonts,
            &description,
            (259.0, top + 152.0),
            Align::Center,
            &line,
        );
    }

    #[allow(clippy::unused_self, reason = "drawn as part of the menu")]
    fn switch_row(
        &self,
        c: &mut Canvas,
        fonts: &mut Fonts,
        top: f32,
        row: &SwitchRow,
        lit: Option<Color>,
    ) {
        if let Some(fill) = lit {
            let height = if row.refusal.is_some() {
                30.0 + REFUSAL
            } else {
                30.0
            };
            Self::chosen_box(c, (12.0, top - 7.0, 494.0, height), fill);
        }
        let label = Style::new(14.0, SWITCH_TEXT).weight(Weight::Medium);
        span(c, fonts, &row.label, (27.0, top), Align::Left, &label);
        // A toggle: a track, its knob at the right when on, the track's colour following it.
        let track = between(PILL_OFF, colors::ACCENT, row.knob);
        c.rect((451.0, top - 3.0, 40.0, 22.0), 11.0, Some(track), None);
        let knob_x = 18.0f32.mul_add(f32::from(row.knob.min(100)) / 100.0, 462.0);
        c.circle((knob_x, top + 8.0), 8.0, Some(colors::TEXT), None);
        if let Some(refusal) = &row.refusal {
            let red = Style::new(12.0, colors::RED);
            span(c, fonts, refusal, (27.0, top + 22.0), Align::Left, &red);
        }
    }

    fn saves_row(&self, c: &mut Canvas, fonts: &mut Fonts, top: f32, can_load: bool) {
        let words = Style::new(15.0, colors::TEXT).weight(Weight::Medium);
        let key = Style::new(12.0, colors::FAINT);
        for (i, (word, keys)) in SAVE_BUTTONS.iter().enumerate() {
            #[allow(clippy::cast_precision_loss, reason = "three buttons")]
            let x = 12.0 + i as f32 * 168.0;
            if let Some(fill) = self.lit(Row::Save(i)) {
                Self::chosen_box(c, (x, top, 160.0, 39.0), fill);
            } else {
                c.rect(
                    (x, top, 160.0, 39.0),
                    6.0,
                    None,
                    Some(Line::solid(colors::EDGE, 1.0)),
                );
            }
            // Load waits for a save, faded as the mockup has it.
            let alpha = if i == 1 && !can_load { 115 } else { 255 };
            let words = Style {
                color: Color {
                    a: alpha,
                    ..words.color
                },
                ..words
            };
            let key_style = Style {
                color: Color {
                    a: alpha,
                    ..key.color
                },
                ..key
            };
            // As the mockup's markup has it: a space, then the key 6 pixels on; without a
            // key the space falls at the line's end and goes, the 6 pixels stay.
            let space = if keys.is_empty() {
                0.0
            } else {
                fonts.measure(" ", &words)
            };
            let gap = space + 6.0;
            let width = fonts.measure(word, &words) + gap + fonts.measure(keys, &key_style);
            let left = x + (160.0 - width) / 2.0;
            let advance = c.text(fonts, word, (left, top + 1.0), 37.0, Align::Left, &words);
            if !keys.is_empty() {
                c.text(
                    fonts,
                    keys,
                    (left + advance + gap, top + 1.0),
                    37.0,
                    Align::Left,
                    &key_style,
                );
            }
        }
    }

    fn action_row(&self, c: &mut Canvas, fonts: &mut Fonts, top: f32, i: usize, action: &str) {
        if let Some(fill) = self.lit(Row::Action(i)) {
            Self::chosen_box(c, (12.0, top - 9.0, 494.0, 36.0), fill);
        }
        let armed = self.armed.as_ref().filter(|(at, _)| *at == i);
        let (words, color) = armed.map_or_else(
            || (action.to_owned(), colors::TEXT),
            |(_, what)| {
                let button = match self.pad.confirm() {
                    Press::Pad(letter) => letter,
                    _ => "×",
                };
                (
                    format!("Press Enter or {button} again to {what}"),
                    colors::RED,
                )
            },
        );
        let style = Style::new(15.0, color).weight(Weight::Medium);
        span(c, fonts, &words, (27.0, top), Align::Left, &style);
    }
}

/// One line of text, on a 16-pixel line's baseline, as the mockup's `<span>`s of smaller
/// text are.
fn span(
    c: &mut Canvas,
    fonts: &mut Fonts,
    words: &str,
    (x, y): (f32, f32),
    align: Align,
    style: &Style,
) {
    text(
        c,
        fonts,
        words,
        (x, y + Fonts::in_body_line(style)),
        align,
        style,
    );
}

/// One line of text at its normal line height.
fn text(
    c: &mut Canvas,
    fonts: &mut Fonts,
    words: &str,
    at: (f32, f32),
    align: Align,
    style: &Style,
) {
    c.text(
        fonts,
        words,
        at,
        Fonts::normal_line_height(style),
        align,
        style,
    );
}

/// The question before the score shows more: what it would show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    /// What the score would show that it doesn't yet: "Guidance level 4", "Full energy".
    pub items: Vec<String>,
    /// How the last pad used marks its buttons.
    pub pad: PadLabels,
}

/// The question card's width, in logical pixels.
pub const QUESTION_WIDTH: u32 = 438;

impl Question {
    /// The card's height, in logical pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        u32::try_from(self.items.len()).unwrap_or(0) * 22 + 158
    }

    /// Draws the card at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let mut c = Canvas::new(QUESTION_WIDTH, self.height(), scale);
        #[allow(clippy::cast_precision_loss, reason = "a card's size")]
        let (w, h) = (QUESTION_WIDTH as f32, self.height() as f32);
        c.rect(
            (0.0, 0.0, w, h),
            12.0,
            Some(colors::CARD),
            Some(Line::solid(CARD_EDGE, 1.0)),
        );
        let title = Style::new(20.0, colors::TEXT).weight(Weight::SemiBold);
        text(
            &mut c,
            fonts,
            "Your score will show this",
            (27.0, 22.0),
            Align::Left,
            &title,
        );
        let body = Style::new(15.0, BODY);
        let mut y = 60.0;
        for item in &self.items {
            span(&mut c, fonts, item, (27.0, y), Align::Left, &body);
            y += 22.0;
        }
        let rule = h - 51.0;
        c.rect((1.0, rule, w - 2.0, 1.0), 0.0, Some(CARD_EDGE), None);
        hints_from(
            &mut c,
            fonts,
            (27.0, rule + 17.0),
            &[
                Hint {
                    press: &[Press::Key("Enter"), self.pad.confirm()],
                    does: "use these settings",
                },
                Hint {
                    press: &[Press::Key("Esc"), self.pad.back()],
                    does: "change them",
                },
            ],
        );
        c
    }
}

/// The colour `percent` of the way from `from` to `to`, opaque.
fn between(from: Color, to: Color, percent: u8) -> Color {
    let t = u16::from(percent.min(100));
    let mix = |a: u8, b: u8| {
        let (a, b) = (u16::from(a), u16::from(b));
        let mixed = (a * (100 - t) + b * t) / 100;
        u8::try_from(mixed).unwrap_or(u8::MAX)
    };
    Color::rgb(mix(from.r, to.r), mix(from.g, to.g), mix(from.b, to.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toggle_s_track_fades_with_its_knob() {
        assert_eq!(between(PILL_OFF, colors::ACCENT, 0), PILL_OFF);
        assert_eq!(between(PILL_OFF, colors::ACCENT, 100), colors::ACCENT);
        let half = between(Color::rgb(0, 100, 200), Color::rgb(100, 200, 0), 50);
        assert_eq!(half, Color::rgb(50, 150, 100));
        assert_eq!(
            between(PILL_OFF, colors::ACCENT, 255),
            colors::ACCENT,
            "past the end"
        );
    }
}
