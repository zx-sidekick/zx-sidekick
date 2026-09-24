//! The picker (#148, #153): laid over the picture, opened with Esc or
//! Select, and holding the game while it is open, as Starquake's picker
//! does. The guidance level, five training switches, a row to go to a
//! cavern, End this game and Exit.
//! Left and right change the highlighted row in the picker only; Enter or
//! A keeps the changes and closes it, or does the action; Esc, B or Select
//! close it and keep nothing. An action asks for a second press.

use manicminer::play::Training;
use sidekick_frontend::gamepad::Layout;
use sidekick_frontend::overlay::{self, HEIGHT};
use sidekick_frontend::text::palette::{
    ACCENT, ACCENT_DIM, ARROW, BRIGHT, DANGER, DANGER_FILL, DANGER_TEXT, DANGER_TITLE, DIALOG, DIM,
    FOCUS_TITLE as TITLE, HINT_KEY, LABEL, LABEL_FOCUSED, NOTCH, ON_TEXT, PAUSED, RULE, SELECTED,
    SWITCH_OFF, SWITCH_ON, VALUE_DIM,
};
use sidekick_frontend::text::{Canvas, Fonts, PadMark, Rgb, Span, Weight, span};

use crate::frontend::PANEL_W;

/// The picker's rows, top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    /// The guidance level.
    Level,
    /// One of the five switches, by its place in [`SWITCHES`].
    Switch(usize),
    Cavern,
    EndGame,
    Exit,
}

const ROWS: [Row; 9] = [
    Row::Level,
    Row::Switch(0),
    Row::Switch(1),
    Row::Switch(2),
    Row::Switch(3),
    Row::Switch(4),
    Row::Cavern,
    Row::EndGame,
    Row::Exit,
];

/// Each guidance level's name, as the panel and the picker show it (#153).
pub const LEVELS: [&str; 5] = [
    "Off",
    "On screen",
    "Items and portal",
    "What can hurt you",
    "Jumps",
];

/// What each level adds, which the picker says under it.
const ADDS: [&str; 5] = [
    "The original game, no help.",
    "Air in seconds, the items left, the portal, and the cavern.",
    "The cavern drawn beside the picture, its items and portal ringed.",
    "Nasty tiles, crumbling floor, conveyors, and each guardian's path.",
    "Where a jump from where Willy stands would land, or that it kills.",
];

/// The highest level.
const TOP_LEVEL: u8 = LEVELS.len() as u8 - 1;

/// Each switch's name, and what it does, which the picker says under them
/// while it is highlighted.
const SWITCHES: [(&str, &str); 5] = [
    (
        "Endless lives",
        "A life lost is not taken from the lives left.",
    ),
    (
        "Air stays full",
        "The air never runs down, not even under the light beam.",
    ),
    ("Safe falls", "A fall of any height lands Willy safely."),
    (
        "No harm from guardians",
        "Guardians, Eugene, the Kong Beast and the Skylabs pass through Willy.",
    ),
    ("No harm from nasties", "Nasty tiles do not kill."),
];

/// The switch at `i` in `t`.
fn switch(t: &mut Training, i: usize) -> &mut bool {
    match i {
        0 => &mut t.lives,
        1 => &mut t.air,
        2 => &mut t.falls,
        3 => &mut t.guardians,
        _ => &mut t.nasties,
    }
}

/// What the picker asks the machine's loop to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    EndGame,
    Exit,
    /// Go to the cavern, 0 to 19.
    GoTo(u8),
}

/// The picker's state, which the window and the machine's thread share.
#[derive(Clone, Debug, Default)]
pub struct Picker {
    open: bool,
    /// The highlighted row, by its place in [`ROWS`].
    focus: usize,
    /// The guidance level and switches in force, and those chosen in the
    /// picker, not yet kept.
    level: u8,
    training: Training,
    picked_level: u8,
    picked: Training,
    /// The cavern the Go to cavern row shows, 0 to 19.
    cavern: u8,
    /// The caverns' names, as the game has them, read from the tape.
    names: Vec<String>,
    /// An action pressed once, waiting for its second press.
    armed: Option<Row>,
    requested: Option<Action>,
    /// Moves whenever anything the picker shows does, so the window redraws.
    version: u64,
}

impl Picker {
    /// The caverns' names, read from the game as it starts.
    pub fn set_names(&mut self, names: Vec<String>) {
        self.names = names;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// The switches in force.
    pub fn training(&self) -> Training {
        self.training
    }

    /// The guidance level in force, 0 to 4.
    pub fn level(&self) -> u8 {
        self.level
    }

    /// Starts at guidance `level` (headless, which has no picker).
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(TOP_LEVEL);
        self.version += 1;
    }

    /// Keeps what is chosen in the picker.
    fn keep(&mut self) {
        self.level = self.picked_level;
        self.training = self.picked;
    }

    /// Opens the picker on the switches in force, with Go to cavern showing
    /// the cavern being played, `cavern`.
    pub fn open(&mut self, cavern: u8) {
        self.open = true;
        self.picked_level = self.level;
        self.picked = self.training;
        self.cavern = cavern.min(19);
        self.focus = 0;
        self.armed = None;
        self.version += 1;
    }

    /// Esc, B or Select: closes it, keeping nothing chosen in it.
    pub fn back(&mut self) {
        self.close();
    }

    fn close(&mut self) {
        self.open = false;
        self.armed = None;
        self.version += 1;
    }

    pub fn focus_up(&mut self) {
        self.move_focus(-1);
    }

    pub fn focus_down(&mut self) {
        self.move_focus(1);
    }

    /// Moving away from an action pressed once cancels it.
    fn move_focus(&mut self, by: isize) {
        let to = (self.focus as isize + by).clamp(0, ROWS.len() as isize - 1);
        self.focus = to as usize;
        self.armed = None;
        self.version += 1;
    }

    /// Left and right: the level down or up, a switch off or on, or the
    /// cavern down or up, in the picker only until it is kept.
    pub fn change(&mut self, up: bool) {
        match ROWS[self.focus] {
            Row::Level => {
                self.picked_level = if up {
                    (self.picked_level + 1).min(TOP_LEVEL)
                } else {
                    self.picked_level.saturating_sub(1)
                };
            }
            Row::Switch(i) => *switch(&mut self.picked, i) = up,
            Row::Cavern => {
                self.cavern = if up {
                    (self.cavern + 1) % 20
                } else {
                    (self.cavern + 19) % 20
                };
            }
            Row::EndGame | Row::Exit => return,
        }
        self.version += 1;
    }

    /// Enter or A. On the level or a switch it keeps what is chosen and
    /// closes the picker; on Go to cavern it keeps them and goes there; on
    /// an action the first press asks for a second, and the second does it.
    pub fn enter(&mut self) {
        match ROWS[self.focus] {
            Row::Level | Row::Switch(_) => {
                self.keep();
                self.close();
            }
            Row::Cavern => {
                self.keep();
                self.requested = Some(Action::GoTo(self.cavern));
                self.close();
            }
            row @ (Row::EndGame | Row::Exit) => {
                if self.armed == Some(row) {
                    self.requested = Some(if row == Row::Exit {
                        Action::Exit
                    } else {
                        Action::EndGame
                    });
                    self.close();
                } else {
                    self.armed = Some(row);
                    self.version += 1;
                }
            }
        }
    }

    /// The action requested, if any, taken once.
    pub fn take(&mut self) -> Option<Action> {
        self.requested.take()
    }

    /// Whether Exit has been requested and not yet taken.
    pub fn exiting(&self) -> bool {
        self.requested == Some(Action::Exit)
    }
}

/// The picker's keys and buttons, as every game's picker takes them.
impl sidekick_frontend::picker::Picker for Picker {
    fn back(&mut self) {
        Picker::back(self);
    }
    fn focus_up(&mut self) {
        Picker::focus_up(self);
    }
    fn focus_down(&mut self) {
        Picker::focus_down(self);
    }
    fn change(&mut self, up: bool) {
        Picker::change(self, up);
    }
    fn enter(&mut self) {
        Picker::enter(self);
    }
}

const LINE: Rgb = [0x2a, 0x2f, 0x3b];

/// The guidance level's box, as tall as Starquake's.
const LEVEL_BOX: f32 = 184.0;
/// The TRAINING heading over the switches.
const SWITCH_HEAD: f32 = 22.0;

/// A row's height, and the space it takes.
const PITCH: f32 = 34.0;

/// A pad button in the legend: its letter, or the PlayStation's mark.
enum Badge {
    Button(&'static str),
    Mark(PadMark),
}

/// The buttons that confirm and go back on the pad `layout`.
fn pad_badges(layout: Layout) -> (Badge, Badge) {
    match layout {
        Layout::PlayStation => (Badge::Mark(PadMark::Cross), Badge::Mark(PadMark::Circle)),
        Layout::Nintendo | Layout::Xbox => (Badge::Button("A"), Badge::Button("B")),
    }
}

/// Draws the picker over the picture, which it dims.
pub fn draw(fonts: &mut Fonts, canvas: &mut Canvas, p: &Picker, layout: Layout) {
    let (ww, wh) = (overlay::width(PANEL_W), HEIGHT);
    canvas.shade(0.0, 0.0, ww, wh, DIM, 184);
    let action_h = |row: Row| if p.armed == Some(row) { 54.0 } else { 40.0 };
    let actions_h = action_h(Row::EndGame) + 4.0 + action_h(Row::Exit);
    let switches_top = 56.0 + LEVEL_BOX + 16.0 + SWITCH_HEAD;
    let rule = switches_top + 6.0 * PITCH + 18.0 + 6.0;
    let (w, h) = (560.0, rule + 8.0 + actions_h + 12.0 + 52.0);
    let x = (ww - w) / 2.0;
    let y = (wh - h) / 2.0;
    canvas.round_rect(x, y, w, h, 12.0, LINE);
    canvas.round_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 11.0, DIALOG);
    fonts.text(
        Some(canvas),
        x + 28.0,
        y + 20.0,
        None,
        1.0,
        &[span("Guidance", 19.0, Weight::SemiBold, BRIGHT)],
    );
    let paused = [span("The game is paused", 12.0, Weight::Regular, LABEL)];
    let pw = fonts.measure(&paused);
    fonts.text(
        Some(canvas),
        x + w - 28.0 - pw,
        y + 27.0,
        None,
        1.0,
        &paused,
    );

    let (rx, rw) = (x + 12.0, w - 24.0);
    let focus = ROWS[p.focus];
    let highlight = |canvas: &mut Canvas, top: f32| {
        canvas.round_rect(rx, top, rw, PITCH - 4.0, 8.0, ACCENT);
        canvas.round_rect(rx + 2.0, top + 2.0, rw - 4.0, PITCH - 8.0, 6.0, SELECTED);
    };
    level_box(fonts, canvas, p, rx, y + 56.0, rw, focus == Row::Level);
    fonts.spaced(
        canvas,
        rx + 14.0,
        y + 56.0 + LEVEL_BOX + 16.0,
        "TRAINING",
        11.0,
        LABEL,
    );
    let mut top = y + switches_top;
    let mut says = None;
    let mut picked = p.picked;
    for (i, (label, does)) in SWITCHES.iter().enumerate() {
        let focused = focus == Row::Switch(i);
        if focused {
            highlight(canvas, top);
            says = Some((*does).to_string());
        }
        let colour = if focused { TITLE } else { VALUE_DIM };
        fonts.text(
            Some(canvas),
            rx + 14.0,
            top + 7.0,
            None,
            1.0,
            &[span(label, 14.0, Weight::SemiBold, colour)],
        );
        // Off and On at the row's right, the one chosen filled.
        let on = *switch(&mut picked, i);
        let mut bx = rx + rw - 14.0;
        for (text, chosen, fill, ink) in [
            ("On", on, SWITCH_ON, ON_TEXT),
            ("Off", !on, SWITCH_OFF, BRIGHT),
        ] {
            let spans = [span(
                text,
                13.0,
                Weight::SemiBold,
                if chosen { ink } else { PAUSED },
            )];
            let tw = fonts.measure(&spans);
            bx -= tw + 20.0;
            if chosen {
                canvas.round_rect(bx - 8.0, top + 5.0, tw + 16.0, 20.0, 5.0, fill);
            }
            fonts.text(Some(canvas), bx, top + 8.0, None, 1.0, &spans);
        }
        top += PITCH;
    }

    // Go to cavern: its number and name, with arrows either side.
    let focused = focus == Row::Cavern;
    if focused {
        highlight(canvas, top);
        says = Some(format!(
            "Enter or {} goes there with the game's own cheat, starting that cavern again.",
            layout.confirm_name()
        ));
    }
    let colour = if focused { TITLE } else { VALUE_DIM };
    fonts.text(
        Some(canvas),
        rx + 14.0,
        top + 7.0,
        None,
        1.0,
        &[span("Go to cavern", 14.0, Weight::SemiBold, colour)],
    );
    let name = p
        .names
        .get(usize::from(p.cavern))
        .map_or("", String::as_str);
    let value = format!("{}  {name}", p.cavern + 1);
    let spans = [span(&value, 13.0, Weight::SemiBold, colour)];
    let tw = fonts.measure(&spans);
    let right = rx + rw - 14.0;
    let vx = right - 22.0 - tw;
    fonts.text(Some(canvas), vx, top + 8.0, None, 1.0, &spans);
    let arrow = if focused { ACCENT } else { ARROW };
    let cy = top + 15.0;
    canvas.triangle(
        [
            (vx - 22.0, cy),
            (vx - 12.0, cy - 6.0),
            (vx - 12.0, cy + 6.0),
        ],
        arrow,
    );
    canvas.triangle(
        [
            (right, cy),
            (right - 10.0, cy - 6.0),
            (right - 10.0, cy + 6.0),
        ],
        arrow,
    );
    top += PITCH;
    // What the highlighted row does, on a line of its own under them all,
    // so choosing a row never moves the rest.
    if let Some(does) = says {
        let s = [span(&does, 12.0, Weight::Regular, HINT_KEY)];
        let tw = fonts.measure(&s);
        fonts.text(Some(canvas), rx + (rw - tw) / 2.0, top + 2.0, None, 1.0, &s);
    }

    // The actions: pressed once, a row turns red and asks again.
    canvas.round_rect(x + 1.0, y + rule, w - 2.0, 1.0, 0.0, RULE);
    let mut ay = y + rule + 8.0;
    let ok = layout.confirm_name();
    for (row, label, again) in [
        (
            Row::EndGame,
            "End this game",
            format!("Press Enter or {ok} again to end it"),
        ),
        (
            Row::Exit,
            "Exit Manic Miner",
            format!("Press Enter or {ok} again to exit"),
        ),
    ] {
        let rh = action_h(row);
        let armed = p.armed == Some(row);
        let focused = focus == row;
        if armed {
            canvas.round_rect(rx, ay, rw, rh, 10.0, DANGER);
            canvas.round_rect(rx + 2.0, ay + 2.0, rw - 4.0, rh - 4.0, 8.0, DANGER_FILL);
        } else if focused {
            canvas.round_rect(rx, ay, rw, rh, 10.0, ACCENT);
            canvas.round_rect(rx + 2.0, ay + 2.0, rw - 4.0, rh - 4.0, 8.0, SELECTED);
        }
        let colour = if armed {
            DANGER_TITLE
        } else if focused {
            TITLE
        } else {
            VALUE_DIM
        };
        fonts.text(
            Some(canvas),
            rx + 16.0,
            ay + 11.0,
            None,
            1.0,
            &[span(label, 15.0, Weight::SemiBold, colour)],
        );
        if armed {
            fonts.text(
                Some(canvas),
                rx + 16.0,
                ay + 31.0,
                None,
                1.0,
                &[span(&again, 12.0, Weight::Regular, DANGER_TEXT)],
            );
        }
        ay += rh + 4.0;
    }

    // What the keys do.
    let foot = y + h - 52.0;
    canvas.round_rect(x + 1.0, foot, w - 2.0, 1.0, 0.0, RULE);
    let (hy, hh) = (foot + 16.0, 22.0);
    let (ok, back) = pad_badges(layout);
    let badge = |fonts: &mut Fonts, canvas: &mut Canvas, x: f32, b: &Badge| match *b {
        Badge::Button(letter) => fonts.button_badge(canvas, x, hy, hh, letter),
        Badge::Mark(mark) => fonts.mark_badge(canvas, x, hy, hh, mark),
    };
    let mut hx = x + 28.0;
    hx += fonts.arrows(canvas, hx, hy, hh, &["\u{2191}", "\u{2193}"]) + 5.0;
    hx += fonts.word(canvas, hx + 2.0, hy, hh, "choose") + 22.0;
    hx += fonts.arrows(canvas, hx, hy, hh, &["\u{2190}", "\u{2192}"]) + 5.0;
    hx += fonts.word(canvas, hx + 2.0, hy, hh, "change") + 22.0;
    hx += fonts.key_badge(canvas, hx, hy, hh, "Enter") + 5.0;
    hx += fonts.word(canvas, hx, hy, hh, "/") + 5.0;
    hx += badge(fonts, canvas, hx, &ok) + 5.0;
    hx += fonts.word(canvas, hx + 2.0, hy, hh, "OK") + 22.0;
    hx += fonts.key_badge(canvas, hx, hy, hh, "Esc") + 5.0;
    hx += fonts.word(canvas, hx, hy, hh, "/") + 5.0;
    hx += badge(fonts, canvas, hx, &back) + 5.0;
    fonts.word(canvas, hx + 2.0, hy, hh, "cancel");
}

/// The guidance level, as Starquake's picker shows it: the number and its
/// name between arrows, a notch a level, and what the level adds.
fn level_box(
    fonts: &mut Fonts,
    canvas: &mut Canvas,
    p: &Picker,
    x: f32,
    y: f32,
    w: f32,
    focused: bool,
) {
    let level = p.picked_level;
    if focused {
        canvas.round_rect(x, y, w, LEVEL_BOX, 10.0, ACCENT);
        canvas.round_rect(x + 2.0, y + 2.0, w - 4.0, LEVEL_BOX - 4.0, 8.0, SELECTED);
    }
    fonts.spaced(
        canvas,
        x + 16.0,
        y + 14.0,
        "GUIDANCE LEVEL",
        11.0,
        if focused { LABEL_FOCUSED } else { LABEL },
    );
    let arrow = |on: bool| match (on, focused) {
        (true, true) => ACCENT,
        (true, false) => ARROW,
        (false, _) => NOTCH,
    };
    let (l, r, cy) = (x + 16.0, x + w - 16.0, y + 69.0);
    canvas.triangle(
        [(l, cy), (l + 14.0, cy - 8.0), (l + 14.0, cy + 8.0)],
        arrow(level > 0),
    );
    canvas.triangle(
        [(r, cy), (r - 14.0, cy - 8.0), (r - 14.0, cy + 8.0)],
        arrow(level < TOP_LEVEL),
    );
    let value = if focused { TITLE } else { VALUE_DIM };
    let centred = |fonts: &mut Fonts, canvas: &mut Canvas, top: f32, spans: &[Span]| {
        let tw = fonts.measure(spans);
        fonts.text(Some(canvas), x + (w - tw) / 2.0, top, None, 1.0, spans);
    };
    centred(
        fonts,
        canvas,
        y + 30.0,
        &[span(&level.to_string(), 40.0, Weight::SemiBold, value)],
    );
    centred(
        fonts,
        canvas,
        y + 80.0,
        &[span(
            LEVELS[usize::from(level)],
            17.0,
            Weight::SemiBold,
            value,
        )],
    );
    let (nx, nw, gap) = (x + 16.0, w - 32.0, 6.0);
    let step = (nw - f32::from(TOP_LEVEL - 1) * gap) / f32::from(TOP_LEVEL);
    for i in 1..=TOP_LEVEL {
        let colour = match (i <= level, focused) {
            (true, true) => ACCENT,
            (true, false) => ACCENT_DIM,
            (false, _) => NOTCH,
        };
        canvas.round_rect(
            nx + f32::from(i - 1) * (step + gap),
            y + 116.0,
            step,
            8.0,
            3.0,
            colour,
        );
    }
    fonts.text(
        Some(canvas),
        nx,
        y + 130.0,
        None,
        1.0,
        &[span("less help", 11.0, Weight::Regular, PAUSED)],
    );
    let more = [span("more help", 11.0, Weight::Regular, PAUSED)];
    let mw = fonts.measure(&more);
    fonts.text(Some(canvas), nx + nw - mw, y + 130.0, None, 1.0, &more);
    centred(
        fonts,
        canvas,
        y + 152.0,
        &[span(
            ADDS[usize::from(level)],
            13.0,
            Weight::Regular,
            HINT_KEY,
        )],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws the picker as the program does, focused on a switch and on Go
    /// to cavern, to PNGs in the folder `MM_PICKER_PNG` names, for comparing
    /// with the mockup. Does nothing when it is not set.
    #[test]
    fn render_to_png() {
        let Some(out) = std::env::var_os("MM_PICKER_PNG") else {
            return;
        };
        let out = std::path::PathBuf::from(out);
        let mut p = Picker::default();
        p.set_names((1..=20).map(|n| format!("Cavern {n}")).collect());
        p.open(4);
        p.change(true);
        for _ in 0..3 {
            p.focus_down();
        }
        p.change(true);
        let mut fonts = Fonts::load();
        for (name, focus_cavern) in [("switch", false), ("cavern", true)] {
            if focus_cavern {
                for _ in 0..3 {
                    p.focus_down();
                }
            }
            let (w, h) = (overlay::width(PANEL_W) as usize, HEIGHT as usize);
            let mut pixels = vec![0u8; w * h * 4];
            let mut canvas = Canvas {
                pixels: &mut pixels,
                width: w,
                height: h,
                scale: 1.0,
            };
            canvas.clear_transparent();
            draw(&mut fonts, &mut canvas, &p, Layout::Xbox);
            let rgb: Vec<u32> = pixels
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
                .collect();
            std::fs::write(
                out.join(format!("picker-{name}.png")),
                zx_core::png::encode(&rgb, w, h),
            )
            .unwrap();
        }
    }

    #[test]
    fn a_change_is_kept_only_with_enter() {
        let mut p = Picker::default();
        p.open(0);
        p.change(true);
        p.focus_down();
        p.change(true);
        p.back();
        assert_eq!(p.level(), 0, "Esc keeps nothing");
        assert_eq!(p.training(), Training::default(), "Esc keeps nothing");
        p.open(0);
        p.change(true);
        p.focus_down();
        p.change(true);
        p.enter();
        assert_eq!(p.level(), 1, "Enter keeps the level");
        assert!(p.training().lives, "and the switches");
        assert!(!p.is_open());
    }

    #[test]
    fn the_level_stops_at_off_and_at_the_top() {
        let mut p = Picker::default();
        p.open(0);
        p.change(false);
        p.enter();
        assert_eq!(p.level(), 0);
        p.open(0);
        for _ in 0..10 {
            p.change(true);
        }
        p.enter();
        assert_eq!(p.level(), TOP_LEVEL);
    }

    #[test]
    fn go_to_cavern_wraps_and_is_requested_with_enter() {
        let mut p = Picker::default();
        p.open(0);
        for _ in 0..6 {
            p.focus_down();
        }
        p.change(false);
        p.enter();
        assert_eq!(p.take(), Some(Action::GoTo(19)));
        assert_eq!(p.take(), None, "taken once");
    }

    #[test]
    fn an_action_needs_a_second_press() {
        let mut p = Picker::default();
        p.open(3);
        for _ in 0..10 {
            p.focus_down();
        }
        p.enter();
        assert!(p.is_open(), "the first press asks again");
        assert!(!p.exiting());
        p.enter();
        assert!(p.exiting());
        assert!(!p.is_open());
        // Moving away cancels an armed action.
        p.open(3);
        for _ in 0..10 {
            p.focus_down();
        }
        p.enter();
        p.focus_up();
        p.focus_down();
        p.enter();
        assert!(p.is_open(), "armed again, not done");
    }
}
