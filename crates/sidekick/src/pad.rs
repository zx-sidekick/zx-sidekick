//! Game pads: each part by its position, what it does in play from the game's definition, and
//! what that presses on the Spectrum. No SDL here; the window feeds it.

use crate::keys::key_named;
use sidekick_core::{Kempston, Key};
use sidekick_def::{Action, Definition, DirectionKeys, Joystick, Pad, PadTarget, Patch};
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
    /// A key the Spectrum doesn't have that does one of the game's actions, by its index in
    /// the mapping's.
    HostKey(u16),
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

    /// The game's state changed what these parts do: those held now are held back until
    /// they're let go.
    pub(crate) fn hold_back_parts(&mut self, parts: &[Part]) {
        let held = self.held.iter().filter(|(_, part)| parts.contains(part));
        self.held_back.extend(held.copied());
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

/// What a part does in play: inputs held and patches applied while it's held (and for at least
/// `lasts` frames from the press), and inputs tapped for `frames` frames from the press.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Does {
    hold: Vec<Input>,
    patches: Vec<Patch>,
    lasts: u32,
    tap: Vec<Input>,
    frames: u32,
}

/// Each part's `Does`.
type Parts = BTreeMap<Part, Does>;

/// What each part of a pad does in play, in each of the game's states, from its `[run]
/// joystick`, `[pad]` and `[[actions]]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mapping {
    /// With no state, or one without its own `[pad]`.
    default: Parts,
    /// States with their own `[pad]`.
    states: BTreeMap<String, Parts>,
    /// The keys the Spectrum doesn't have that do an action, by SDL's name for them; each is
    /// `Part::HostKey` of its index.
    host_keys: Vec<String>,
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
        let joy = Joy {
            kind: run.map(|r| r.joystick).unwrap_or_default(),
            keys: run.and_then(|r| r.keys.clone()),
            actions: def.map(|d| d.actions.as_slice()).unwrap_or_default(),
        };
        let mut targets: BTreeMap<&str, PadTarget> = [
            ("dpad", "joystick"),
            ("stick", "joystick"),
            ("west", "fire"),
        ]
        .into_iter()
        .map(|(part, target)| (part, PadTarget::Named(target.to_owned())))
        .collect();
        let pads = def.map(|d| &d.pad);
        let over = |targets: &mut BTreeMap<&'static str, PadTarget>, pad: &Pad| {
            for (part, target) in Pad::parts(pad) {
                targets.insert(part, target.clone());
            }
        };
        if let Some(pad) = pads.and_then(|p| p.get("default")) {
            over(&mut targets, pad);
        }
        let mut host_keys = Vec::new();
        let mut parts = |targets: &BTreeMap<&str, PadTarget>| -> Parts {
            let mut parts = Parts::new();
            for (name, target) in targets {
                for (part, way) in parts_named(name) {
                    let does = joy.does(target, way);
                    if does != Does::default() {
                        parts.insert(part, does);
                    }
                }
            }
            for action in joy.actions {
                if let Some(key) = &action.host_key {
                    let index = host_keys.iter().position(|k| k == key).unwrap_or_else(|| {
                        host_keys.push(key.clone());
                        host_keys.len() - 1
                    });
                    let index = u16::try_from(index).unwrap_or(u16::MAX);
                    parts.insert(Part::HostKey(index), joy.action(action));
                }
            }
            parts
        };
        let default = parts(&targets);
        let states = pads
            .into_iter()
            .flatten()
            .filter(|(state, _)| *state != "default")
            .map(|(state, pad)| {
                let mut targets = targets.clone();
                over(&mut targets, pad);
                (state.clone(), parts(&targets))
            })
            .collect();
        Self {
            default,
            states,
            host_keys,
        }
    }

    fn parts(&self, state: Option<&str>) -> &Parts {
        state
            .and_then(|s| self.states.get(s))
            .unwrap_or(&self.default)
    }

    /// The part a key the Spectrum doesn't have is, when it does an action. `name` is SDL's
    /// name for the key.
    pub(crate) fn host_key(&self, name: &str) -> Option<Part> {
        let index = self.host_keys.iter().position(|k| k == name)?;
        Some(Part::HostKey(u16::try_from(index).ok()?))
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

/// The joystick a game reads, and its actions.
struct Joy<'a> {
    kind: Joystick,
    keys: Option<DirectionKeys>,
    actions: &'a [Action],
}

impl Joy<'_> {
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
        match target {
            PadTarget::Named(name) => {
                let direction = match (name.as_str(), way) {
                    ("joystick", Some(way)) => Some(Direction::of(way)),
                    (name, _) => Direction::named(name),
                };
                if let Some(direction) = direction {
                    return Does {
                        hold: self.input(direction).into_iter().collect(),
                        ..Does::default()
                    };
                }
                // `menu` comes with the guidance menu (#14).
                self.named_action(name)
            }
            PadTarget::Table(table) => {
                let mut does = Does {
                    hold: keys(&table.keys),
                    tap: keys(&table.tap),
                    frames: table.frames.unwrap_or(TAP_FRAMES),
                    ..Does::default()
                };
                if let Some(action) = &table.action {
                    let action = self.named_action(action);
                    does.hold.extend(action.hold);
                    does.tap.extend(action.tap);
                    does.patches = action.patches;
                    does.lasts = action.lasts;
                    if table.frames.is_none() {
                        does.frames = action.frames;
                    }
                }
                does
            }
        }
    }

    fn named_action(&self, id: &str) -> Does {
        self.actions
            .iter()
            .find(|a| a.id == id)
            .map(|a| self.action(a))
            .unwrap_or_default()
    }

    /// What an action does. A `script` does nothing until scripts come (#15).
    fn action(&self, action: &Action) -> Does {
        let mut hold = keys(&action.keys);
        hold.extend(
            action
                .joystick
                .iter()
                .filter_map(|d| Direction::named(d))
                .filter_map(|d| self.input(d)),
        );
        Does {
            hold,
            patches: action.patch_for_frame.clone(),
            lasts: action.frames.unwrap_or(0),
            tap: keys(&action.tap),
            frames: action.frames.unwrap_or(TAP_FRAMES),
        }
    }
}

/// Spectrum keys by name; a name that isn't one is left out (`sidekick check` reports it).
fn keys(names: &[String]) -> Vec<Input> {
    names
        .iter()
        .filter_map(|n| key_named(n))
        .map(|k| Input::Key(SpectrumKey(k)))
        .collect()
}

/// Something going on for a number of frames from a press, however long it's held.
#[derive(Debug)]
struct Lasting {
    inputs: Vec<Input>,
    patches: Vec<Patch>,
    left: u32,
}

/// A game being played by pad: its mapping, the game's state, what's going on for a number of
/// frames, and what's held on the Spectrum.
#[derive(Debug, Default)]
pub(crate) struct Play {
    mapping: Mapping,
    state: Option<String>,
    lasting: Vec<Lasting>,
    sent: BTreeSet<Input>,
}

/// What to press, again or anew, and what to let go on the Spectrum, and the patches to apply
/// for the next frame.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Changes {
    pub(crate) press: Vec<Input>,
    pub(crate) release: Vec<Input>,
    pub(crate) patches: Vec<Patch>,
}

impl Play {
    pub(crate) fn new(mapping: Mapping) -> Self {
        Self {
            mapping,
            ..Self::default()
        }
    }

    pub(crate) const fn mapping(&self) -> &Mapping {
        &self.mapping
    }

    /// The game's state for the frames to come. Returns the parts whose meaning it changed,
    /// for those held to be held back: a part that does the same in both carries on, so a
    /// state that flickers doesn't cut out a held direction.
    pub(crate) fn enter(&mut self, state: Option<&str>) -> Vec<Part> {
        if self.state.as_deref() == state {
            return Vec::new();
        }
        let before = self.mapping.parts(self.state.as_deref());
        let after = self.mapping.parts(state);
        let changed = before
            .keys()
            .chain(after.keys())
            .filter(|part| before.get(part) != after.get(part))
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self.state = state.map(str::to_owned);
        changed
    }

    /// A part was pressed: starts its tap, and keeps what it holds going for the frames it
    /// lasts at least.
    pub(crate) fn pressed(&mut self, part: Part) {
        let Some(does) = self.mapping.parts(self.state.as_deref()).get(&part) else {
            return;
        };
        if !does.tap.is_empty() {
            self.lasting.push(Lasting {
                inputs: does.tap.clone(),
                patches: Vec::new(),
                left: does.frames,
            });
        }
        if does.lasts > 0 {
            self.lasting.push(Lasting {
                inputs: does.hold.clone(),
                patches: does.patches.clone(),
                left: does.lasts,
            });
        }
    }

    /// What to press and let go on the Spectrum before the next frame, and the patches to
    /// apply for it, for the parts `held`. Everything held is pressed again each frame, so the
    /// keyboard letting go of a key a pad holds too (0 on a Sinclair joystick) doesn't let it
    /// go for the pad.
    pub(crate) fn update(&mut self, held: impl Iterator<Item = Part>) -> Changes {
        let mut wanted: BTreeSet<Input> = BTreeSet::new();
        let mut patches: Vec<Patch> = Vec::new();
        let mut add = |inputs: &[Input], more: &[Patch]| {
            wanted.extend(inputs.iter().copied());
            for patch in more {
                if !patches.contains(patch) {
                    patches.push(patch.clone());
                }
            }
        };
        for lasting in &self.lasting {
            add(&lasting.inputs, &lasting.patches);
        }
        let parts = self.mapping.parts(self.state.as_deref());
        for part in held {
            if let Some(does) = parts.get(&part) {
                add(&does.hold, &does.patches);
            }
        }
        let changes = Changes {
            press: wanted.iter().copied().collect(),
            release: self.sent.difference(&wanted).copied().collect(),
            patches,
        };
        self.sent = wanted;
        changes
    }

    /// A frame ran: what's lasting has one frame fewer left.
    pub(crate) fn frame_ran(&mut self) {
        for lasting in &mut self.lasting {
            lasting.left = lasting.left.saturating_sub(1);
        }
        self.lasting.retain(|l| l.left > 0);
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
    fn what_s_held_is_pressed_each_frame_and_let_go_once() {
        let mut play = Play::new(Mapping::default());
        let fire = Input::Kempston(Direction::Fire);
        let left = Input::Kempston(Direction::Left);
        assert_eq!(
            play.update(std::iter::once(Part::West)),
            Changes {
                press: vec![fire],
                release: vec![],
                ..Changes::default()
            }
        );
        assert_eq!(
            play.update(std::iter::once(Part::West)),
            Changes {
                press: vec![fire],
                release: vec![],
                ..Changes::default()
            },
            "again, in case the keyboard let it go"
        );
        assert_eq!(
            play.update(std::iter::once(Part::DPad(Way::Left))),
            Changes {
                press: vec![left],
                release: vec![fire],
                ..Changes::default()
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
    fn a_state_change_holds_back_only_the_parts_it_changes() {
        let mut pads = Pads::default();
        pads.button(0, Part::Start, true);
        pads.button(0, Part::DPad(Way::Left), true);
        pads.hold_back_parts(&[Part::Start, Part::South]);
        assert_eq!(pads.held().collect::<Vec<_>>(), [Part::DPad(Way::Left)]);
        pads.button(0, Part::Start, false);
        pads.button(0, Part::Start, true);
        assert_eq!(
            pads.held().collect::<Vec<_>>(),
            [Part::DPad(Way::Left), Part::Start]
        );
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

    /// What `play` holds and patches after one update with `held`.
    fn holds_and_patches(play: &mut Play, held: &[Part]) -> (Vec<Input>, usize) {
        let changes = play.update(held.iter().copied());
        (play.sent.iter().copied().collect(), changes.patches.len())
    }

    #[test]
    fn a_state_s_pad_changes_the_default_s_parts_one_by_one_while_it_lasts() {
        let mapping = Mapping::new(Some(&def(r#"
            [[states]]
            name = "title"
            pc = 0x8000
            [[states]]
            name = "play"
            pc = 0x9000

            [pad.default]
            south = { keys = ["H"] }

            [pad.title]
            south = { keys = ["1"] }
            start = { keys = ["2"] }
            "#)));
        let mut play = Play::new(mapping);
        assert!(play.enter(None).is_empty(), "no state to begin with");
        assert_eq!(holds(&mut play, &[Part::South]), [key(Key::H)]);
        assert_eq!(
            play.enter(Some("title")),
            [Part::South, Part::Start],
            "only the parts it changes"
        );
        assert_eq!(holds(&mut play, &[Part::South]), [key(Key::N1)]);
        assert_eq!(holds(&mut play, &[Part::Start]), [key(Key::N2)]);
        assert_eq!(
            holds(&mut play, &[Part::West]),
            [Input::Kempston(Direction::Fire)],
            "the default's west"
        );
        assert!(play.enter(Some("title")).is_empty(), "the same state");
        assert_eq!(play.enter(Some("play")), [Part::South, Part::Start]);
        assert_eq!(
            holds(&mut play, &[Part::South]),
            [key(Key::H)],
            "no [pad.play]"
        );
        assert!(holds(&mut play, &[Part::Start]).is_empty());
    }

    const ACTIONS: &str = r#"
        [[actions]]
        id = "pick-up"
        joystick = ["up"]
        patch_for_frame = [{ addr = 0x6C10, bytes = "18", expect = "20" }]

        [[actions]]
        id = "end-game"
        tap = ["A", "S"]
        frames = 3

        [[actions]]
        id = "jump"
        keys = ["SPACE"]
        frames = 2
        host_key = "Tab"

        [[actions]]
        id = "next-route"
        script = "next_route"
        host_key = "F6"

        [pad.default]
        north = "pick-up"
        east = { action = "end-game" }
        south = "jump"
        r1 = "next-route"
    "#;

    #[test]
    fn an_action_holds_and_patches_while_its_button_is_held() {
        let mut play = Play::new(Mapping::new(Some(&def(ACTIONS))));
        play.pressed(Part::North);
        assert_eq!(
            holds_and_patches(&mut play, &[Part::North]),
            (vec![Input::Kempston(Direction::Up)], 1)
        );
        play.frame_ran();
        assert_eq!(
            holds_and_patches(&mut play, &[Part::North, Part::DPad(Way::Up)]),
            (vec![Input::Kempston(Direction::Up)], 1),
            "one patch, however many parts ask for it"
        );
        play.frame_ran();
        assert_eq!(holds_and_patches(&mut play, &[]), (vec![], 0), "let go");
    }

    #[test]
    fn an_action_lasts_its_frames_however_briefly_its_button_is_pressed() {
        let mut play = Play::new(Mapping::new(Some(&def(ACTIONS))));
        play.pressed(Part::East);
        for frame in 0..3 {
            assert_eq!(holds(&mut play, &[]), [key(Key::A), key(Key::S)], "{frame}");
            play.frame_ran();
        }
        assert!(holds(&mut play, &[]).is_empty());

        play.pressed(Part::South);
        for _ in 0..2 {
            assert_eq!(
                holds(&mut play, &[]),
                [key(Key::Space)],
                "at least its frames"
            );
            play.frame_ran();
        }
        assert!(holds(&mut play, &[]).is_empty());
        assert_eq!(
            holds(&mut play, &[Part::South]),
            [key(Key::Space)],
            "and as long as it's held"
        );
    }

    #[test]
    fn a_key_the_spectrum_lacks_does_its_action() {
        let mapping = Mapping::new(Some(&def(ACTIONS)));
        let tab = mapping.host_key("Tab").expect("Tab does an action");
        assert!(mapping.host_key("F7").is_none());
        let mut play = Play::new(mapping);
        assert!(
            holds(&mut play, &[Part::R1]).is_empty(),
            "a script does nothing yet"
        );
        play.pressed(tab);
        assert_eq!(holds(&mut play, &[tab]), [key(Key::Space)]);
        let f6 = play.mapping().host_key("F6").expect("F6 does an action");
        play.pressed(f6);
        assert_eq!(
            holds(&mut play, &[f6]),
            [key(Key::Space)],
            "only Tab's jump"
        );
    }
}
