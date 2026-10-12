//! Game pads: each part by its position, what it does in play from the game's definition, and
//! what that presses on the Spectrum. No SDL here; the window feeds it.

use crate::keys::key_named;
use sidekick_core::{Kempston, Key};
use sidekick_def::{Definition, DirectionKeys, Joystick, Pad, PadTarget};
use sidekick_ui::PadLabels;
use std::collections::{BTreeMap, BTreeSet};

/// How far the stick or a trigger moves before it counts: half its travel.
const HALF_TRAVEL: i16 = 16_384;

/// How many frames a tap lasts when the definition doesn't say.
pub(crate) const TAP_FRAMES: u32 = 2;

/// A way the D-pad or the stick points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Way {
    Up,
    Down,
    Left,
    Right,
}

const WAYS: [Way; 4] = [Way::Up, Way::Down, Way::Left, Way::Right];

/// A part of a pad, by its position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Part {
    DPad(Way),
    Stick(Way),
    West,
    South,
    East,
    North,
    L1,
    R1,
    L2,
    R2,
    Start,
    Select,
}

/// The pad's axes that count as parts: the left stick and the triggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    StickX,
    StickY,
    L2,
    R2,
}

/// Who made a pad, which decides its confirm and back buttons and how they're marked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Maker {
    /// Nintendo: A, on the right, confirms; B, at the bottom, goes back.
    Nintendo,
    /// Sony: × at the bottom confirms, ○ on the right goes back.
    PlayStation,
    /// Everyone else, Xbox's layout: A at the bottom confirms, B on the right goes back.
    #[default]
    Other,
}

impl Maker {
    /// The button that confirms.
    pub(crate) const fn confirm(self) -> Part {
        match self {
            Self::Nintendo => Part::East,
            Self::PlayStation | Self::Other => Part::South,
        }
    }

    /// The button that goes back.
    pub(crate) const fn back(self) -> Part {
        match self {
            Self::Nintendo => Part::South,
            Self::PlayStation | Self::Other => Part::East,
        }
    }

    /// How the pages show its confirm and back buttons.
    pub(crate) const fn labels(self) -> PadLabels {
        match self {
            Self::PlayStation => PadLabels::Shapes,
            Self::Nintendo | Self::Other => PadLabels::Letters,
        }
    }
}

/// What a press means on a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Nav {
    Up,
    Down,
    Confirm,
    Back,
}

/// What `part`, pressed on a pad from `maker`, means on a page.
pub(crate) fn nav(part: Part, maker: Maker) -> Option<Nav> {
    match part {
        Part::DPad(Way::Up) | Part::Stick(Way::Up) => Some(Nav::Up),
        Part::DPad(Way::Down) | Part::Stick(Way::Down) => Some(Nav::Down),
        p if p == maker.confirm() => Some(Nav::Confirm),
        p if p == maker.back() => Some(Nav::Back),
        _ => None,
    }
}

/// What's held on every pad plugged in, and what's held back: a part still held when the
/// screen changed counts for nothing until it's let go.
#[derive(Debug, Default)]
pub(crate) struct Pads {
    held: BTreeSet<(u32, Part)>,
    held_back: BTreeSet<(u32, Part)>,
    /// Each pad's stick, x then y.
    sticks: BTreeMap<u32, (i16, i16)>,
}

impl Pads {
    /// A button on pad `pad` went down or up. Returns the part when it's a press that counts.
    pub(crate) fn button(&mut self, pad: u32, part: Part, down: bool) -> Option<Part> {
        if down {
            self.held.insert((pad, part)).then_some(part)
        } else {
            self.held.remove(&(pad, part));
            self.held_back.remove(&(pad, part));
            None
        }
    }

    /// An axis of pad `pad` moved. Returns the parts it pressed.
    pub(crate) fn axis(&mut self, pad: u32, axis: Axis, value: i16) -> Vec<Part> {
        let trigger = |part| {
            let down = value >= HALF_TRAVEL;
            (part, down)
        };
        let changes = match axis {
            Axis::L2 => vec![trigger(Part::L2)],
            Axis::R2 => vec![trigger(Part::R2)],
            Axis::StickX | Axis::StickY => {
                let stick = self.sticks.entry(pad).or_default();
                if axis == Axis::StickX {
                    stick.0 = value;
                } else {
                    stick.1 = value;
                }
                let ways = stick_ways(*stick);
                WAYS.iter()
                    .map(|&way| (Part::Stick(way), ways.contains(&way)))
                    .collect()
            }
        };
        changes
            .into_iter()
            .filter_map(|(part, down)| {
                let was = self.held.contains(&(pad, part));
                if down == was {
                    None
                } else {
                    self.button(pad, part, down)
                }
            })
            .collect()
    }

    /// Pad `pad` was unplugged: whatever it held is let go.
    pub(crate) fn removed(&mut self, pad: u32) {
        self.held.retain(|&(p, _)| p != pad);
        self.held_back.retain(|&(p, _)| p != pad);
        self.sticks.remove(&pad);
    }

    /// The screen changed: everything held now is held back until it's let go.
    pub(crate) fn hold_back(&mut self) {
        self.held_back.extend(self.held.iter().copied());
    }

    /// The parts held that count, on any pad.
    pub(crate) fn held(&self) -> impl Iterator<Item = Part> + '_ {
        self.held.difference(&self.held_back).map(|&(_, part)| part)
    }
}

/// The ways a stick at (`x`, `y`) points, past half its travel.
fn stick_ways((x, y): (i16, i16)) -> Vec<Way> {
    let mut ways = Vec::new();
    if y <= -HALF_TRAVEL {
        ways.push(Way::Up);
    }
    if y >= HALF_TRAVEL {
        ways.push(Way::Down);
    }
    if x <= -HALF_TRAVEL {
        ways.push(Way::Left);
    }
    if x >= HALF_TRAVEL {
        ways.push(Way::Right);
    }
    ways
}

/// A Spectrum key, ordered so a set can hold it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpectrumKey(pub(crate) Key);

impl SpectrumKey {
    const fn index(self) -> u8 {
        self.0 as u8
    }
}

impl PartialEq for SpectrumKey {
    fn eq(&self, other: &Self) -> bool {
        self.index() == other.index()
    }
}

impl Eq for SpectrumKey {}

impl PartialOrd for SpectrumKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpectrumKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index().cmp(&other.index())
    }
}

/// The joystick's directions and its fire button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
    Fire,
}

impl Direction {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "up" => Self::Up,
            "down" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            "fire" => Self::Fire,
            _ => return None,
        })
    }

    const fn of(way: Way) -> Self {
        match way {
            Way::Up => Self::Up,
            Way::Down => Self::Down,
            Way::Left => Self::Left,
            Way::Right => Self::Right,
        }
    }

    /// Its own of five things, given up, down, left, right and fire.
    fn pick<T>(self, [up, down, left, right, fire]: [T; 5]) -> T {
        match self {
            Self::Up => up,
            Self::Down => down,
            Self::Left => left,
            Self::Right => right,
            Self::Fire => fire,
        }
    }

    pub(crate) const fn kempston(self) -> Kempston {
        match self {
            Self::Up => Kempston::Up,
            Self::Down => Kempston::Down,
            Self::Left => Kempston::Left,
            Self::Right => Kempston::Right,
            Self::Fire => Kempston::Fire,
        }
    }
}

/// Something held on the Spectrum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Input {
    Kempston(Direction),
    Key(SpectrumKey),
}

/// What a part does in play: inputs held while it is, and inputs tapped for some frames when
/// it's pressed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Does {
    hold: Vec<Input>,
    tap: Vec<Input>,
    frames: u32,
}

/// What each part of a pad does in play, from a game's `[run] joystick` and `[pad.default]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mapping {
    parts: BTreeMap<Part, Does>,
}

impl Default for Mapping {
    /// No definition: the directions drive a Kempston joystick and west fires.
    fn default() -> Self {
        Self::new(None)
    }
}

impl Mapping {
    /// The mapping a definition gives, or the plain one without a definition.
    pub(crate) fn new(def: Option<&Definition>) -> Self {
        let run = def.and_then(|d| d.run.as_ref());
        let joystick = Joy {
            kind: run.map(|r| r.joystick).unwrap_or_default(),
            keys: run.and_then(|r| r.keys.clone()),
        };
        let mut targets: BTreeMap<&str, PadTarget> = [
            ("dpad", "joystick"),
            ("stick", "joystick"),
            ("west", "fire"),
        ]
        .into_iter()
        .map(|(part, target)| (part, PadTarget::Named(target.to_owned())))
        .collect();
        if let Some(pad) = def.and_then(|d| d.pad.get("default")) {
            for (part, target) in Pad::parts(pad) {
                targets.insert(part, target.clone());
            }
        }
        let mut parts = BTreeMap::new();
        for (name, target) in &targets {
            for (part, way) in parts_named(name) {
                let does = joystick.does(target, way);
                if does != Does::default() {
                    parts.insert(part, does);
                }
            }
        }
        Self { parts }
    }
}

/// The parts a `[pad]` entry names, with the way each points for a direction.
fn parts_named(name: &str) -> Vec<(Part, Option<Way>)> {
    let one = |part| vec![(part, None)];
    match name {
        "dpad" => WAYS.iter().map(|&w| (Part::DPad(w), Some(w))).collect(),
        "stick" => WAYS.iter().map(|&w| (Part::Stick(w), Some(w))).collect(),
        "west" => one(Part::West),
        "south" => one(Part::South),
        "east" => one(Part::East),
        "north" => one(Part::North),
        "l1" => one(Part::L1),
        "r1" => one(Part::R1),
        "l2" => one(Part::L2),
        "r2" => one(Part::R2),
        "start" => one(Part::Start),
        "select" => one(Part::Select),
        _ => Vec::new(),
    }
}

/// The joystick a game reads.
struct Joy {
    kind: Joystick,
    keys: Option<DirectionKeys>,
}

impl Joy {
    /// What one of the joystick's directions, or fire, presses.
    fn input(&self, direction: Direction) -> Option<Input> {
        let key = match &self.kind {
            Joystick::Kempston => return Some(Input::Kempston(direction)),
            Joystick::Sinclair1 => direction.pick([Key::N9, Key::N8, Key::N6, Key::N7, Key::N0]),
            Joystick::Sinclair2 => direction.pick([Key::N4, Key::N3, Key::N1, Key::N2, Key::N5]),
            Joystick::Cursor => direction.pick([Key::N7, Key::N6, Key::N5, Key::N8, Key::N0]),
            Joystick::Keys => {
                let k = self.keys.as_ref()?;
                key_named(direction.pick([&k.up, &k.down, &k.left, &k.right, &k.fire]))?
            }
        };
        Some(Input::Key(SpectrumKey(key)))
    }

    /// What a part does for a `[pad]` target; `way` is the part's, for `"joystick"`.
    fn does(&self, target: &PadTarget, way: Option<Way>) -> Does {
        let keys = |names: &[String]| -> Vec<Input> {
            names
                .iter()
                .filter_map(|n| key_named(n))
                .map(|k| Input::Key(SpectrumKey(k)))
                .collect()
        };
        match target {
            PadTarget::Named(name) => {
                let direction = match (name.as_str(), way) {
                    ("joystick", Some(way)) => Some(Direction::of(way)),
                    (name, _) => Direction::named(name),
                };
                // `menu` comes with the guidance menu, actions with per-state mappings.
                Does {
                    hold: direction.and_then(|d| self.input(d)).into_iter().collect(),
                    ..Does::default()
                }
            }
            PadTarget::Table(table) => Does {
                hold: keys(&table.keys),
                tap: keys(&table.tap),
                frames: table.frames.unwrap_or(TAP_FRAMES),
            },
        }
    }
}

/// A game being played by pad: its mapping, the taps running, and what's held on the Spectrum.
#[derive(Debug, Default)]
pub(crate) struct Play {
    mapping: Mapping,
    /// Inputs tapped, and the frames each has left.
    taps: Vec<(Input, u32)>,
    sent: BTreeSet<Input>,
}

/// What to press and let go on the Spectrum.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Changes {
    pub(crate) press: Vec<Input>,
    pub(crate) release: Vec<Input>,
}

impl Play {
    pub(crate) fn new(mapping: Mapping) -> Self {
        Self {
            mapping,
            ..Self::default()
        }
    }

    /// A part was pressed: starts its tap, if it has one.
    pub(crate) fn pressed(&mut self, part: Part) {
        if let Some(does) = self.mapping.parts.get(&part) {
            for &input in &does.tap {
                self.taps.retain(|&(i, _)| i != input);
                self.taps.push((input, does.frames));
            }
        }
    }

    /// What has to change on the Spectrum before the next frame, for the parts `held`.
    pub(crate) fn update(&mut self, held: impl Iterator<Item = Part>) -> Changes {
        let mut wanted: BTreeSet<Input> = self.taps.iter().map(|&(input, _)| input).collect();
        for part in held {
            if let Some(does) = self.mapping.parts.get(&part) {
                wanted.extend(does.hold.iter().copied());
            }
        }
        let changes = Changes {
            press: wanted.difference(&self.sent).copied().collect(),
            release: self.sent.difference(&wanted).copied().collect(),
        };
        self.sent = wanted;
        changes
    }

    /// A frame ran: each tap has one frame fewer left.
    pub(crate) fn frame_ran(&mut self) {
        for (_, left) in &mut self.taps {
            *left = left.saturating_sub(1);
        }
        self.taps.retain(|&(_, left)| left > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(toml: &str) -> Definition {
        let base = r#"
            schema = 1
            [game]
            id = "test"
            title = "Test"
            machine = "48k"
            [[media]]
            name = "the tape"
        "#;
        Definition::from_toml(&format!("{base}\n{toml}")).expect("the test definition parses")
    }

    fn key(k: Key) -> Input {
        Input::Key(SpectrumKey(k))
    }

    /// What `play` holds after one update with `held`.
    fn holds(play: &mut Play, held: &[Part]) -> Vec<Input> {
        play.update(held.iter().copied());
        play.sent.iter().copied().collect()
    }

    #[test]
    fn without_a_definition_the_directions_drive_a_kempston_joystick_and_west_fires() {
        let mut play = Play::new(Mapping::default());
        assert_eq!(
            holds(&mut play, &[Part::DPad(Way::Left), Part::West]),
            [
                Input::Kempston(Direction::Left),
                Input::Kempston(Direction::Fire)
            ]
        );
        assert_eq!(
            holds(&mut play, &[Part::Stick(Way::Up)]),
            [Input::Kempston(Direction::Up)]
        );
        assert!(holds(&mut play, &[Part::South, Part::Start, Part::Select]).is_empty());
    }

    #[test]
    fn each_joystick_presses_its_own_keys() {
        let cases = [
            ("sinclair1", [Key::N9, Key::N8, Key::N6, Key::N7, Key::N0]),
            ("sinclair2", [Key::N4, Key::N3, Key::N1, Key::N2, Key::N5]),
            ("cursor", [Key::N7, Key::N6, Key::N5, Key::N8, Key::N0]),
        ];
        for (joystick, [up, down, left, right, fire]) in cases {
            let mapping = Mapping::new(Some(&def(&format!("[run]\njoystick = \"{joystick}\""))));
            let mut play = Play::new(mapping);
            for (part, want) in [
                (Part::DPad(Way::Up), up),
                (Part::DPad(Way::Down), down),
                (Part::Stick(Way::Left), left),
                (Part::Stick(Way::Right), right),
                (Part::West, fire),
            ] {
                assert_eq!(
                    holds(&mut play, &[part]),
                    [key(want)],
                    "{joystick} {part:?}"
                );
            }
        }
    }

    #[test]
    fn a_game_s_own_keys_drive_it() {
        let mapping = Mapping::new(Some(&def(r#"
            [run]
            joystick = "keys"
            keys = { up = "Q", down = "A", left = "O", right = "P", fire = "SPACE" }
            "#)));
        let mut play = Play::new(mapping);
        assert_eq!(
            holds(
                &mut play,
                &[Part::DPad(Way::Up), Part::DPad(Way::Right), Part::West]
            ),
            [key(Key::Q), key(Key::P), key(Key::Space)]
        );
    }

    #[test]
    fn a_definition_s_pad_changes_parts_one_by_one() {
        let mapping = Mapping::new(Some(&def(r#"
            [pad.default]
            west = "none"
            south = "fire"
            north = "up"
            start = { keys = ["SYMBOL SHIFT", "A"] }
            "#)));
        let mut play = Play::new(mapping);
        assert!(
            holds(&mut play, &[Part::West]).is_empty(),
            "west does nothing now"
        );
        assert_eq!(
            holds(&mut play, &[Part::South, Part::North]),
            [
                Input::Kempston(Direction::Up),
                Input::Kempston(Direction::Fire)
            ]
        );
        assert_eq!(
            holds(&mut play, &[Part::DPad(Way::Down)]),
            [Input::Kempston(Direction::Down)],
            "the D-pad keeps its joystick"
        );
        assert_eq!(
            holds(&mut play, &[Part::Start]),
            [key(Key::A), key(Key::SymShift)]
        );
    }

    #[test]
    fn only_what_changed_is_pressed_or_let_go() {
        let mut play = Play::new(Mapping::default());
        let fire = Input::Kempston(Direction::Fire);
        let left = Input::Kempston(Direction::Left);
        assert_eq!(
            play.update(std::iter::once(Part::West)),
            Changes {
                press: vec![fire],
                release: vec![]
            }
        );
        assert_eq!(play.update(std::iter::once(Part::West)), Changes::default());
        assert_eq!(
            play.update(std::iter::once(Part::DPad(Way::Left))),
            Changes {
                press: vec![left],
                release: vec![fire]
            }
        );
    }

    #[test]
    fn a_tap_lasts_its_frames_however_long_the_button_is_held() {
        let mapping = Mapping::new(Some(&def(r#"
            [pad.default]
            north = { tap = ["P"], frames = 3 }
            east = { tap = ["H"] }
            "#)));
        let mut play = Play::new(mapping);
        play.pressed(Part::North);
        for frame in 0..3 {
            assert_eq!(
                holds(&mut play, &[Part::North]),
                [key(Key::P)],
                "frame {frame}"
            );
            play.frame_ran();
        }
        assert!(
            holds(&mut play, &[Part::North]).is_empty(),
            "held on, it's over"
        );

        play.pressed(Part::East);
        for _ in 0..TAP_FRAMES {
            assert_eq!(
                holds(&mut play, &[]),
                [key(Key::H)],
                "let go at once, it goes on"
            );
            play.frame_ran();
        }
        assert!(holds(&mut play, &[]).is_empty());
    }

    #[test]
    fn the_stick_counts_past_half_its_travel() {
        let mut pads = Pads::default();
        assert!(pads.axis(0, Axis::StickX, 16_000).is_empty());
        assert_eq!(
            pads.axis(0, Axis::StickX, 20_000),
            [Part::Stick(Way::Right)]
        );
        assert!(
            pads.axis(0, Axis::StickX, 30_000).is_empty(),
            "already held"
        );
        assert_eq!(pads.axis(0, Axis::StickY, -32_768), [Part::Stick(Way::Up)]);
        assert_eq!(
            pads.held().collect::<Vec<_>>(),
            [Part::Stick(Way::Up), Part::Stick(Way::Right)]
        );
        assert!(pads.axis(0, Axis::StickX, -100).is_empty());
        assert_eq!(pads.held().collect::<Vec<_>>(), [Part::Stick(Way::Up)]);
        assert_eq!(pads.axis(0, Axis::R2, 32_767), [Part::R2]);
        assert!(pads.axis(0, Axis::R2, 0).is_empty());
        assert_eq!(pads.held().collect::<Vec<_>>(), [Part::Stick(Way::Up)]);
    }

    #[test]
    fn a_part_held_across_a_screen_change_counts_once_let_go() {
        let mut pads = Pads::default();
        assert_eq!(pads.button(0, Part::South, true), Some(Part::South));
        pads.axis(0, Axis::StickX, -32_768);
        pads.hold_back();
        assert_eq!(pads.held().count(), 0, "both held back");
        assert_eq!(
            pads.button(1, Part::West, true),
            Some(Part::West),
            "pressed after"
        );
        assert_eq!(pads.held().collect::<Vec<_>>(), [Part::West]);
        pads.button(0, Part::South, false);
        pads.axis(0, Axis::StickX, 0);
        assert_eq!(pads.button(0, Part::South, true), Some(Part::South));
        assert_eq!(pads.held().collect::<Vec<_>>(), [Part::South, Part::West]);
    }

    #[test]
    fn an_unplugged_pad_lets_go() {
        let mut pads = Pads::default();
        pads.button(3, Part::West, true);
        pads.axis(3, Axis::StickY, 32_767);
        pads.button(4, Part::West, true);
        pads.removed(3);
        assert_eq!(pads.held().collect::<Vec<_>>(), [Part::West], "pad 4's");
        assert_eq!(pads.axis(3, Axis::StickY, 32_767), [Part::Stick(Way::Down)]);
    }

    #[test]
    fn confirm_and_back_follow_the_maker() {
        assert_eq!(nav(Part::South, Maker::Other), Some(Nav::Confirm));
        assert_eq!(nav(Part::East, Maker::Other), Some(Nav::Back));
        assert_eq!(nav(Part::South, Maker::PlayStation), Some(Nav::Confirm));
        assert_eq!(nav(Part::East, Maker::Nintendo), Some(Nav::Confirm));
        assert_eq!(nav(Part::South, Maker::Nintendo), Some(Nav::Back));
        assert_eq!(nav(Part::Stick(Way::Down), Maker::Other), Some(Nav::Down));
        assert_eq!(nav(Part::DPad(Way::Up), Maker::Other), Some(Nav::Up));
        assert_eq!(nav(Part::West, Maker::Other), None);
        assert_eq!(Maker::PlayStation.labels(), PadLabels::Shapes);
        assert_eq!(Maker::Nintendo.labels(), PadLabels::Letters);
    }
}
