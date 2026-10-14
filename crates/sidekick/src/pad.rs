//! Game pads: each part by its position, what it does in play from the game's definition, and
//! what that presses on the Spectrum. No SDL here; the window feeds it.

use crate::keys::{key_for, key_named};
use sidekick_core::{Kempston, Key};
use sidekick_def::{Action, Definition, DirectionKeys, Joystick, Pad, PadTarget, Patch, Test};
use sidekick_ui::PadLabels;
use std::collections::{BTreeMap, BTreeSet};

/// How far the stick or a trigger moves before it counts: half its travel.
const HALF_TRAVEL: i16 = 16_384;

/// How many frames a tap lasts when the definition doesn't say.
pub(crate) const TAP_FRAMES: u32 = 2;

/// How many frames a button's tap of ENTER on a loading screen lasts: the ROM's `PAUSE` reads
/// the keyboard once a frame.
const LOADING_TAP_FRAMES: u32 = 4;

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
    /// One of the keyboard's arrows, which plays as the D-pad unless `[pad] arrows` says
    /// otherwise.
    Arrow(Way),
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
    /// Any other key of the keyboard, by its scancode: it does what a state's `any` does.
    Key(u16),
}

impl Part {
    /// Whether it goes on from a loading screen waiting for a key: a button (the face buttons,
    /// the shoulders, the triggers, Start) or a key. Not a direction, and not Select, which
    /// opens the guidance menu.
    pub(crate) const fn goes_on(self) -> bool {
        matches!(
            self,
            Self::West
                | Self::South
                | Self::East
                | Self::North
                | Self::L1
                | Self::R1
                | Self::L2
                | Self::R2
                | Self::Start
                | Self::HostKey(_)
                | Self::Key(_)
        )
    }
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
            Self::Nintendo => PadLabels::Nintendo,
            Self::Other => PadLabels::Letters,
        }
    }
}

/// What a press means on a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Nav {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
}

/// What `part`, pressed on a pad from `maker`, means on a page.
pub(crate) fn nav(part: Part, maker: Maker) -> Option<Nav> {
    match part {
        Part::DPad(Way::Up) | Part::Stick(Way::Up) | Part::Arrow(Way::Up) => Some(Nav::Up),
        Part::DPad(Way::Down) | Part::Stick(Way::Down) | Part::Arrow(Way::Down) => Some(Nav::Down),
        Part::DPad(Way::Left) | Part::Stick(Way::Left) | Part::Arrow(Way::Left) => Some(Nav::Left),
        Part::DPad(Way::Right) | Part::Stick(Way::Right) | Part::Arrow(Way::Right) => {
            Some(Nav::Right)
        }
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
/// `lasts` frames from the press), inputs tapped for `frames` frames from the press, and a
/// sequence of inputs tapped in turn, each for `frames` frames with as many between.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Does {
    /// A function of the definition's script to call on the press.
    script: Option<String>,
    hold: Vec<Input>,
    patches: Vec<Patch>,
    lasts: u32,
    tap: Vec<Input>,
    sequence: Vec<Input>,
    frames: u32,
}

impl Does {
    /// Whether it presses, taps and calls the same as `other`, however long for.
    fn alike(&self, other: &Self) -> bool {
        self.hold == other.hold
            && self.tap == other.tap
            && self.sequence == other.sequence
            && self.script == other.script
            && self.patches == other.patches
    }
}

/// Each part's `Does`.
type Parts = BTreeMap<Part, Does>;

/// What each part of a pad does in play, in each of the game's states, from its `[run]
/// joystick`, `[pad]` and `[[actions]]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mapping {
    /// With no state, or one without its own `[pad]`.
    default: Parts,
    /// States with their own `[pad]`, and the test of a value it applies while, if any.
    states: BTreeMap<String, (Option<Test>, Parts)>,
    /// What every other key does in the states whose `[pad]` has `any`.
    any: BTreeMap<String, Does>,
    /// The keys the Spectrum doesn't have that do an action, by SDL's name for them; each is
    /// `Part::HostKey` of its index.
    host_keys: Vec<String>,
    /// Each action, by id, for the program to do (the menu's End this game).
    actions: BTreeMap<String, Does>,
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
        let mut targets = Targets::default();
        for (part, target) in [
            ("dpad", "joystick"),
            ("stick", "joystick"),
            ("west", "fire"),
        ] {
            targets.set(part, &PadTarget::Named(target.to_owned()));
        }
        let pads = def.map(|d| &d.pad);
        let over = |targets: &mut Targets, pad: &Pad| {
            for (part, target) in Pad::parts(pad) {
                targets.set(part, target);
            }
        };
        if let Some(pad) = pads.and_then(|p| p.get("default")) {
            over(&mut targets, pad);
        }
        let mut host_keys = Vec::new();
        let mut parts = |targets: &Targets| -> Parts {
            let mut parts = Parts::new();
            for (name, target) in targets.resolved() {
                for (part, way) in parts_named(&name) {
                    let does = joy.does(&target, way);
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
        let mut any = BTreeMap::new();
        let states = pads
            .into_iter()
            .flatten()
            .filter(|(state, _)| *state != "default")
            .map(|(state, pad)| {
                let mut targets = targets.clone();
                if let Some(target) = &pad.any {
                    for name in ANY_PARTS {
                        targets.set(name, target);
                    }
                }
                over(&mut targets, pad);
                let mut parts = parts(&targets);
                if let Some(target) = &pad.any {
                    let does = joy.does(target, None);
                    for (_, host_key) in parts
                        .iter_mut()
                        .filter(|(p, _)| matches!(p, Part::HostKey(_)))
                    {
                        *host_key = does.clone();
                    }
                    any.insert(state.clone(), does);
                }
                (state.clone(), (pad.when.clone(), parts))
            })
            .collect();
        let actions = joy
            .actions
            .iter()
            .map(|a| (a.id.clone(), joy.action(a)))
            .collect();
        Self {
            default,
            states,
            any,
            host_keys,
            actions,
        }
    }

    /// What each part does in `state`: its own table while its `when` holds (`active`), or
    /// when it has none; the default's otherwise.
    fn parts(&self, state: Option<&str>, active: bool) -> &Parts {
        state
            .and_then(|s| self.states.get(s))
            .filter(|(when, _)| when.is_none() || active)
            .map_or(&self.default, |(_, parts)| parts)
    }

    /// The test of a value `state`'s own table applies while, if it has one.
    pub(crate) fn when(&self, state: Option<&str>) -> Option<&Test> {
        self.states.get(state?)?.0.as_ref()
    }

    /// What `part` does in `state`, if anything.
    fn does(&self, state: Option<&str>, active: bool, part: Part) -> Option<&Does> {
        match part {
            Part::Key(_) => self.any.get(state?),
            part => self.parts(state, active).get(&part),
        }
    }

    /// The part a key the Spectrum doesn't have is, when it does an action. `name` is SDL's
    /// name for the key.
    pub(crate) fn host_key(&self, name: &str) -> Option<Part> {
        let index = self.host_keys.iter().position(|k| k == name)?;
        Some(Part::HostKey(u16::try_from(index).ok()?))
    }
}

impl Mapping {
    /// The keyboard's keys that do what the pad's part `name` does in `state`, as a card
    /// shows them, with the Spectrum's keyboard on there or not (`spectrum`): the arrows for the
    /// D-pad; the key that plays the part itself; else the keys whose parts do the same there
    /// (the up arrow, for a button that pushes the joystick up), or the key an action takes.
    pub(crate) fn keyboard_for(
        &self,
        state: Option<&str>,
        name: &str,
        spectrum: bool,
    ) -> Option<String> {
        if matches!(name, "dpad" | "stick") {
            return Some("← ↑ → ↓".to_owned());
        }
        let &[(part, None)] = parts_named(name).as_slice() else {
            return None;
        };
        if let Some(key) = key_for(part, spectrum) {
            return Some(key.to_owned());
        }
        let parts = self.parts(state, true);
        let does = parts.get(&part)?;
        let same = |other: Part| parts.get(&other).is_some_and(|o| o.alike(does));
        let arrows: Vec<&str> = WAYS
            .iter()
            .map(|&w| Part::Arrow(w))
            .filter(|&p| same(p))
            .filter_map(|p| key_for(p, spectrum))
            .collect();
        if !arrows.is_empty() {
            return Some(arrows.join(" "));
        }
        if same(Part::West) {
            return key_for(Part::West, spectrum).map(str::to_owned);
        }
        self.host_keys
            .iter()
            .enumerate()
            .find(|&(i, _)| u16::try_from(i).is_ok_and(|i| same(Part::HostKey(i))))
            .map(|(_, key)| key.clone())
    }
}

/// What each `[pad]` entry is set to, the directions way by way (`dpad:up`), and which of the
/// arrows' ways have been set themselves rather than following the D-pad's.
#[derive(Clone, Debug, Default)]
struct Targets {
    named: BTreeMap<String, PadTarget>,
    arrows: BTreeSet<&'static str>,
}

const WAY_NAMES: [&str; 4] = ["up", "down", "left", "right"];

impl Targets {
    /// Sets entry `name`: a direction's ways all to a name or table, or some of them to theirs.
    fn set(&mut self, name: &str, target: &PadTarget) {
        if !matches!(name, "dpad" | "stick" | "arrows") {
            self.named.insert(name.to_owned(), target.clone());
            return;
        }
        let ways: Vec<(&'static str, PadTarget)> = match target {
            PadTarget::Ways(ways) => ways
                .ways()
                .into_iter()
                .map(|(way, n)| (way, PadTarget::Named(n.to_owned())))
                .collect(),
            target => WAY_NAMES.iter().map(|&w| (w, target.clone())).collect(),
        };
        for (way, target) in ways {
            if name == "arrows" {
                self.arrows.insert(way);
            }
            self.named.insert(format!("{name}:{way}"), target);
        }
    }

    /// Every entry, the arrows' ways not set themselves as the D-pad's.
    fn resolved(&self) -> Vec<(String, PadTarget)> {
        let mut all = self.named.clone();
        for way in WAY_NAMES.iter().filter(|w| !self.arrows.contains(*w)) {
            if let Some(target) = self.named.get(&format!("dpad:{way}")) {
                all.insert(format!("arrows:{way}"), target.clone());
            }
        }
        all.into_iter().collect()
    }
}

/// The parts a state's `any` stands for, unless its table names them: the buttons, not the
/// directions or Select.
const ANY_PARTS: [&str; 9] = [
    "west", "south", "east", "north", "l1", "r1", "l2", "r2", "start",
];

/// The parts a `[pad]` entry names, with the way each points for a direction.
fn parts_named(name: &str) -> Vec<(Part, Option<Way>)> {
    let one = |part| vec![(part, None)];
    match name {
        "dpad" => WAYS.iter().map(|&w| (Part::DPad(w), Some(w))).collect(),
        "stick" => WAYS.iter().map(|&w| (Part::Stick(w), Some(w))).collect(),
        "arrows" => WAYS.iter().map(|&w| (Part::Arrow(w), Some(w))).collect(),
        name if name.contains(':') => {
            let (pad, way) = name.split_once(':').unwrap_or_default();
            let way = match way {
                "up" => Way::Up,
                "down" => Way::Down,
                "left" => Way::Left,
                _ => Way::Right,
            };
            let part = match pad {
                "dpad" => Part::DPad(way),
                "stick" => Part::Stick(way),
                _ => Part::Arrow(way),
            };
            vec![(part, Some(way))]
        }
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
            // Expanded way by way before they get here (`Targets::set`).
            PadTarget::Ways(_) => Does::default(),
            PadTarget::Table(table) => {
                let mut does = Does {
                    hold: keys(&table.keys),
                    tap: keys(&table.tap),
                    sequence: keys(&table.sequence),
                    frames: table.frames.unwrap_or(TAP_FRAMES),
                    ..Does::default()
                };
                if let Some(action) = &table.action {
                    let action = self.named_action(action);
                    does.hold.extend(action.hold);
                    does.tap.extend(action.tap);
                    does.sequence.extend(action.sequence);
                    does.patches = action.patches;
                    does.lasts = action.lasts;
                    does.script = action.script;
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
            script: action.script.clone(),
            hold,
            patches: action.patch_for_frame.clone(),
            lasts: action.frames.unwrap_or(0),
            tap: keys(&action.tap),
            sequence: keys(&action.sequence),
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

/// Something going on for a number of frames from a press, however long it's held, after
/// `wait` frames.
#[derive(Debug)]
struct Lasting {
    inputs: Vec<Input>,
    patches: Vec<Patch>,
    wait: u32,
    left: u32,
}

/// A game being played by pad: its mapping, the game's state, what's going on for a number of
/// frames, and what's held on the Spectrum.
#[derive(Debug, Default)]
pub(crate) struct Play {
    mapping: Mapping,
    state: Option<String>,
    /// Whether the state's own `[pad]` table applies: its `when` holds, or it has none.
    active: bool,
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
    pub(crate) fn enter(
        &mut self,
        state: Option<&str>,
        holds: &dyn Fn(&Test) -> bool,
    ) -> Vec<Part> {
        let active = self.mapping.when(state).is_none_or(holds);
        if self.state.as_deref() == state && self.active == active {
            return Vec::new();
        }
        let before = self.mapping.parts(self.state.as_deref(), self.active);
        let after = self.mapping.parts(state, active);
        let changed = before
            .keys()
            .chain(after.keys())
            .filter(|part| before.get(part) != after.get(part))
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self.state = state.map(str::to_owned);
        self.active = active;
        changed
    }

    /// Whether `part` does anything in the game's state now.
    pub(crate) fn maps(&self, part: Part) -> bool {
        self.mapping
            .does(self.state.as_deref(), self.active, part)
            .is_some()
    }

    /// Whether every key of the keyboard does something in the game's state now: the
    /// state's `[pad]` has `any`.
    pub(crate) fn takes_any_key(&self) -> bool {
        self.maps(Part::Key(0))
    }

    /// A part was pressed, `loading` when a loader waits for a key (the ROM's `PAUSE 0`, its
    /// loading screen up): there a button or a key ([`Part::goes_on`]) taps ENTER, which the
    /// ROM's `PAUSE` takes, rather than what it does in play (a Kempston joystick's fire, which
    /// no `PAUSE` reads), so any of them goes on from every loading screen, as its card says.
    /// Elsewhere, [`Self::pressed`].
    pub(crate) fn press(&mut self, part: Part, loading: bool) -> Option<String> {
        if loading && part.goes_on() {
            self.lasting.push(Lasting {
                inputs: vec![Input::Key(SpectrumKey(Key::Enter))],
                patches: Vec::new(),
                wait: 0,
                left: LOADING_TAP_FRAMES,
            });
            return None;
        }
        self.pressed(part)
    }

    /// A part was pressed: starts its tap, and keeps what it holds going for the frames it
    /// lasts at least. Returns the script function it calls, if it calls one.
    pub(crate) fn pressed(&mut self, part: Part) -> Option<String> {
        let does = self
            .mapping
            .does(self.state.as_deref(), self.active, part)?;
        Self::start(&mut self.lasting, does);
        does.script.clone()
    }

    /// Does one of the game's actions, by id, as a press that's let go at once: its taps, and
    /// what it holds for the frames it lasts. Returns the script function it calls, if any.
    pub(crate) fn act(&mut self, id: &str) -> Option<String> {
        let does = self.mapping.actions.get(id)?;
        Self::start(&mut self.lasting, does);
        does.script.clone()
    }

    fn start(lasting: &mut Vec<Lasting>, does: &Does) {
        if !does.tap.is_empty() {
            lasting.push(Lasting {
                inputs: does.tap.clone(),
                patches: Vec::new(),
                wait: 0,
                left: does.frames,
            });
        }
        for (wait, &input) in (0..).step_by(2).zip(&does.sequence) {
            lasting.push(Lasting {
                inputs: vec![input],
                patches: Vec::new(),
                wait: wait * does.frames,
                left: does.frames,
            });
        }
        if does.lasts > 0 {
            lasting.push(Lasting {
                inputs: does.hold.clone(),
                patches: does.patches.clone(),
                wait: 0,
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
        for lasting in self.lasting.iter().filter(|l| l.wait == 0) {
            add(&lasting.inputs, &lasting.patches);
        }
        for part in held {
            if let Some(does) = self.mapping.does(self.state.as_deref(), self.active, part) {
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
            if lasting.wait > 0 {
                lasting.wait -= 1;
            } else {
                lasting.left = lasting.left.saturating_sub(1);
            }
        }
        self.lasting.retain(|l| l.left > 0);
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

    #[test]
    fn a_card_shows_the_keys_that_do_what_a_part_does() {
        let mapping = Mapping::new(Some(&def(r#"
            [run]
            joystick = "kempston"
            [pad.default]
            south = "build"
            east = "pick-up"
            north = "next"
            l1 = { tap = ["SPACE"] }
            [pad.title]
            south = "fire"
            [[actions]]
            id = "pick-up"
            label = "Pick up"
            joystick = ["up"]
            [[actions]]
            id = "build"
            label = "Build"
            joystick = ["down"]
            [[actions]]
            id = "next"
            label = "Next"
            script = "next"
            host_key = "Tab"
        "#)));
        let keys = |state, part| mapping.keyboard_for(state, part, true);
        assert_eq!(
            keys(None, "east").as_deref(),
            Some("↑"),
            "the joystick's up"
        );
        assert_eq!(keys(None, "south").as_deref(), Some("↓"));
        assert_eq!(
            keys(None, "west").as_deref(),
            Some("Right Ctrl"),
            "its own key"
        );
        assert_eq!(keys(None, "dpad").as_deref(), Some("← ↑ → ↓"));
        assert_eq!(
            keys(None, "north").as_deref(),
            Some("Tab"),
            "an action's key"
        );
        assert_eq!(keys(None, "l1"), None, "no key does the same");
        assert_eq!(keys(None, "r2"), None, "it does nothing");
        assert_eq!(
            keys(Some("title"), "south").as_deref(),
            Some("Right Ctrl"),
            "in a state of its own, what fires"
        );
        let pad = |part| mapping.keyboard_for(None, part, false);
        assert_eq!(
            pad("east").as_deref(),
            Some("↑"),
            "the keyboard as a pad: no key of its own, so the arrow that does the same"
        );
        assert_eq!(pad("south").as_deref(), Some("↓"));
        assert_eq!(pad("west").as_deref(), Some("Space"), "its own key");
        assert_eq!(pad("north").as_deref(), Some("Tab"), "an action's key");
    }

    /// What `play` holds after one update with `held`.
    fn holds(play: &mut Play, held: &[Part]) -> Vec<Input> {
        play.update(held.iter().copied());
        play.sent.iter().copied().collect()
    }

    #[test]
    fn a_state_s_table_with_when_applies_while_its_test_holds_and_the_arrows_follow_the_d_pad() {
        let mapping = Mapping::new(Some(&def(r#"
            [run]
            joystick = "kempston"
            [[states]]
            name = "play"
            role = "play"
            pc = 0x8000
            [values.mode]
            addr = 0x9000
            type = "u8"
            [pad.play]
            when = { value = "mode", not = 2 }
            dpad = { up = "none", down = "none" }
            stick = { up = "none" }
            arrows = "joystick"
        "#)));
        let mut play = Play::new(mapping);
        let up = Input::Kempston(Direction::Up);
        let left = Input::Kempston(Direction::Left);
        play.enter(Some("play"), &|_| true);
        assert_eq!(
            holds(&mut play, &[Part::DPad(Way::Up)]),
            [],
            "the D-pad's up does nothing"
        );
        assert_eq!(
            holds(&mut play, &[Part::DPad(Way::Left)]),
            [left],
            "its left moves"
        );
        assert_eq!(
            holds(&mut play, &[Part::Stick(Way::Up)]),
            [],
            "the stick's up, as listed"
        );
        assert_eq!(
            holds(&mut play, &[Part::Stick(Way::Down)]),
            [Input::Kempston(Direction::Down)],
            "its down, not listed, as before"
        );
        assert_eq!(
            holds(&mut play, &[Part::Arrow(Way::Up)]),
            [up],
            "the arrows have their own"
        );
        let changed = play.enter(Some("play"), &|_| false);
        assert!(changed.contains(&Part::DPad(Way::Up)), "{changed:?}");
        assert_eq!(
            holds(&mut play, &[Part::DPad(Way::Up)]),
            [up],
            "the test fails: default"
        );
        assert!(
            play.enter(Some("play"), &|_| false).is_empty(),
            "nothing changed"
        );
    }

    #[test]
    fn the_keyboard_s_arrows_do_what_the_d_pad_does_unless_named() {
        let mapping = Mapping::new(Some(&def(r#"
            [[states]]
            name = "title"
            pc = 0x8000
            [pad.title]
            dpad = "none"
        "#)));
        let mut play = Play::new(mapping);
        assert_eq!(
            holds(&mut play, &[Part::Arrow(Way::Right)]),
            [Input::Kempston(Direction::Right)]
        );
        play.enter(Some("title"), &|_| true);
        assert_eq!(
            holds(&mut play, &[Part::Arrow(Way::Right)]),
            [],
            "as the D-pad: nothing"
        );
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
    fn a_sequence_taps_its_keys_in_turn() {
        let mapping = Mapping::new(Some(&def(r#"
            [pad.default]
            start = { sequence = ["1", "0"], frames = 2 }
            "#)));
        let mut play = Play::new(mapping);
        play.pressed(Part::Start);
        let mut seen = Vec::new();
        for _ in 0..8 {
            seen.push(holds(&mut play, &[]));
            play.frame_ran();
        }
        let one = vec![key(Key::N1)];
        let zero = vec![key(Key::N0)];
        assert_eq!(
            seen,
            [
                one.clone(),
                one,
                vec![],
                vec![],
                zero.clone(),
                zero,
                vec![],
                vec![]
            ],
            "each for two frames, with two between"
        );
    }

    /// Presses and lets go on the Spectrum what `play` changed, as the window does.
    fn send(machine: &mut sidekick_core::Machine, changes: Changes) {
        let inputs = changes
            .release
            .into_iter()
            .map(|i| (i, false))
            .chain(changes.press.into_iter().map(|i| (i, true)));
        for (input, pressed) in inputs {
            match input {
                Input::Kempston(direction) => machine.kempston(direction.kempston(), pressed),
                Input::Key(key) => machine.key(key.0, pressed),
            }
        }
    }

    /// Runs a frame of `s` with what `play` holds for `held`.
    fn frame(s: &mut sidekick_core::Session, play: &mut Play, held: &[Part]) {
        let changes = play.update(held.iter().copied());
        send(s.machine_mut(), changes);
        s.run_frame().expect("a frame");
        play.frame_ran();
        play.enter(s.progress().last_known.as_deref(), &|_| true);
    }

    #[test]
    #[allow(clippy::print_stderr, reason = "a skipped test says why")]
    fn starquake_s_start_on_the_title_plays_on_the_joystick() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let zip = repo.join("assets/Starquake.tap.zip");
        if !zip.exists() {
            eprintln!("skipped: no {} (the player's own tape)", zip.display());
            return;
        }
        let tapes = sidekick_def::tapes_in(&zip).expect("the zip holds a tape");
        let package = sidekick_def::Package::load(&repo.join("games/starquake"))
            .expect("Starquake's definition");
        let mut play = Play::new(Mapping::new(Some(&package.definition)));
        let mut machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        machine.load_tap(&tapes[0].bytes).expect("the tape goes in");
        let mut s = sidekick_core::Session::new(machine, Some(package.definition));
        for _ in 0..400 {
            frame(&mut s, &mut play, &[]);
        }
        // The loading screen waits for a key, as on a Spectrum.
        s.machine_mut().key(Key::Space, true);
        for _ in 0..4 {
            frame(&mut s, &mut play, &[]);
        }
        s.machine_mut().key(Key::Space, false);
        let titled = (0..1500).any(|_| {
            frame(&mut s, &mut play, &[]);
            s.state() == Some("title")
        });
        assert!(titled, "the title");

        play.pressed(Part::Start);
        let mut seen = Vec::new();
        for _ in 0..300 {
            frame(&mut s, &mut play, &[]);
            seen.push(s.state().map(str::to_owned));
        }
        assert!(
            seen[100..].iter().all(Option::is_none),
            "Start on the title starts a game: its story shows until a press"
        );
        play.pressed(Part::Start);
        let playing = (0..100).any(|_| {
            frame(&mut s, &mut play, &[]);
            s.state() == Some("play")
        });
        assert!(playing, "Start goes on from the story into play");
        for _ in 0..100 {
            frame(&mut s, &mut play, &[]);
        }
        let x = |s: &sidekick_core::Session| s.reading()["blob_x"].number().expect("BLOB's x");
        let before = x(&s);
        for _ in 0..40 {
            frame(&mut s, &mut play, &[Part::DPad(Way::Left)]);
        }
        assert_ne!(
            x(&s),
            before,
            "the pad moves BLOB: the game is on the joystick"
        );
        assert_ne!(
            s.state(),
            Some("paused"),
            "the story's Start didn't pause it"
        );
    }

    /// A `.tap` block: its length, then the flag, the data and their XOR.
    fn block(flag: u8, data: &[u8]) -> Vec<u8> {
        let checksum = data.iter().fold(flag, |x, b| x ^ b);
        let len = u16::try_from(data.len() + 2).expect("a short block");
        let mut out = len.to_le_bytes().to_vec();
        out.push(flag);
        out.extend_from_slice(data);
        out.push(checksum);
        out
    }

    /// A program of one line of BASIC tokens, auto-running, as a header and a data block.
    fn program(line: &[u8]) -> Vec<u8> {
        let mut line = line.to_vec();
        line.push(0x0D);
        let mut program = vec![0x00, 0x0A];
        program.extend_from_slice(&u16::try_from(line.len()).expect("short").to_le_bytes());
        program.extend_from_slice(&line);
        let len = u16::try_from(program.len()).expect("short").to_le_bytes();
        let mut header = vec![0x00];
        header.extend_from_slice(b"stand-in  ");
        header.extend_from_slice(&len);
        header.extend_from_slice(&10u16.to_le_bytes());
        header.extend_from_slice(&len);
        let mut tap = block(0x00, &header);
        tap.extend(block(0xFF, &program));
        tap
    }

    /// A session at a loader's wait for a key, from a stand-in tape, `10 PAUSE 0: LOAD ""`,
    /// then `10 BORDER 2`: loading that waits for a key halfway, as a loading screen does. Its
    /// definition maps nothing to Start.
    pub(crate) fn at_a_loader_s_wait() -> sidekick_core::Session {
        let hidden = |n: u8| [0x0E, 0x00, 0x00, n, 0x00, 0x00];
        let mut first = vec![0xF2, b'0'];
        first.extend_from_slice(&hidden(0));
        first.extend_from_slice(&[b':', 0xEF, b'"', b'"']);
        let mut second = vec![0xE7, b'2'];
        second.extend_from_slice(&hidden(2));
        let mut tap = program(&first);
        tap.extend(program(&second));

        let mut machine =
            sidekick_core::Machine::new(sidekick_core::Model::Sinclair48K).expect("a 48K");
        machine.load_tap(&tap).expect("the tape goes in");
        let mut s = sidekick_core::Session::new(machine, Some(def("")));
        let mut play = Play::new(Mapping::new(s.definition()));
        assert!(!play.maps(Part::Start), "nothing mapped to Start");
        let waits = (0..1000).any(|_| {
            frame(&mut s, &mut play, &[]);
            s.machine().waits_on_loading_screen()
        });
        assert!(waits, "the loader waits for a key");
        for _ in 0..50 {
            frame(&mut s, &mut play, &[]);
        }
        assert!(s.machine().waits_on_loading_screen(), "until one comes");
        s
    }

    /// Whether pressing `part` on the loader's wait in `s`, held for a few frames as a press
    /// is, goes on loading.
    pub(crate) fn goes_on_loading(mut s: sidekick_core::Session, part: Part) -> bool {
        let mut play = Play::new(Mapping::new(s.definition()));
        play.press(part, s.machine().waits_on_loading_screen());
        (0..50).any(|i| {
            let held: &[Part] = if i < 6 { &[part] } else { &[] };
            frame(&mut s, &mut play, held);
            !s.machine().waits_on_loading_screen()
        })
    }

    #[test]
    fn every_button_and_key_goes_on_from_a_loader_s_wait_with_nothing_mapped_to_start() {
        let s = at_a_loader_s_wait();
        for part in [
            Part::West,
            Part::South,
            Part::East,
            Part::North,
            Part::L1,
            Part::R1,
            Part::L2,
            Part::R2,
            Part::Start,
            Part::Key(20),
            Part::HostKey(0),
        ] {
            assert!(part.goes_on(), "{part:?}");
            assert!(
                goes_on_loading(s.clone(), part),
                "{part:?} taps a key PAUSE takes"
            );
        }
        for part in [Part::DPad(Way::Left), Part::Stick(Way::Up), Part::Select] {
            assert!(!part.goes_on(), "{part:?}");
            assert!(!goes_on_loading(s.clone(), part), "{part:?} doesn't go on");
        }
    }

    #[test]
    fn a_state_s_any_is_every_button_and_key_it_doesn_t_name_but_not_the_directions() {
        let mut play = Play::new(Mapping::new(Some(&def(r#"
            [[states]]
            name = "title"
            pc = 0x8000
            [pad.title]
            any = { tap = ["0"] }
            north = { tap = ["1"] }
            "#))));
        assert!(!play.takes_any_key(), "not before the title");
        assert!(!play.maps(Part::Key(20)));
        play.enter(Some("title"), &|_| true);
        assert!(play.takes_any_key());
        for part in [
            Part::West,
            Part::South,
            Part::L2,
            Part::Start,
            Part::Key(20),
        ] {
            play.pressed(part);
            assert_eq!(holds(&mut play, &[]), [key(Key::N0)], "{part:?}");
            play.frame_ran();
            play.frame_ran();
        }
        play.pressed(Part::North);
        assert_eq!(holds(&mut play, &[]), [key(Key::N1)], "a part it names");
        play.frame_ran();
        play.frame_ran();
        assert_eq!(
            holds(&mut play, &[Part::DPad(Way::Left)]),
            [Input::Kempston(Direction::Left)],
            "the directions do what they did"
        );
        assert!(!play.maps(Part::Select), "Select is the menu's");
    }

    #[test]
    fn a_button_on_a_loading_screen_taps_enter_whatever_it_does_in_play() {
        let mut play = Play::new(Mapping::new(Some(&def(r#"
            [pad.default]
            start = { tap = ["SPACE"] }
            "#))));
        play.press(Part::Start, true);
        assert_eq!(holds(&mut play, &[]), [key(Key::Enter)]);
        let mut play = Play::new(Mapping::default());
        play.press(Part::West, true);
        assert_eq!(
            holds(&mut play, &[]),
            [key(Key::Enter)],
            "not the Kempston joystick's fire, which no PAUSE reads"
        );
        play.press(Part::West, false);
        assert_eq!(
            holds(&mut play, &[Part::West]),
            [Input::Kempston(Direction::Fire), key(Key::Enter)],
            "elsewhere, what it does in play"
        );
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
        assert_eq!(nav(Part::Stick(Way::Left), Maker::Other), Some(Nav::Left));
        assert_eq!(nav(Part::West, Maker::Other), None);
        assert_eq!(Maker::PlayStation.labels(), PadLabels::Shapes);
        assert_eq!(Maker::Nintendo.labels(), PadLabels::Nintendo);
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
        assert!(
            play.enter(None, &|_| true).is_empty(),
            "no state to begin with"
        );
        assert_eq!(holds(&mut play, &[Part::South]), [key(Key::H)]);
        assert_eq!(
            play.enter(Some("title"), &|_| true),
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
        assert!(
            play.enter(Some("title"), &|_| true).is_empty(),
            "the same state"
        );
        assert_eq!(
            play.enter(Some("play"), &|_| true),
            [Part::South, Part::Start]
        );
        assert_eq!(
            holds(&mut play, &[Part::South]),
            [key(Key::H)],
            "no [pad.play]"
        );
        assert!(holds(&mut play, &[Part::Start]).is_empty());
        assert!(!play.maps(Part::Start), "Start does nothing in play here");
        play.enter(Some("title"), &|_| true);
        assert!(play.maps(Part::Start), "but on the title");
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
    fn the_program_can_do_an_action_by_id() {
        let mut play = Play::new(Mapping::new(Some(&def(ACTIONS))));
        play.act("end-game");
        for _ in 0..3 {
            assert_eq!(holds(&mut play, &[]), [key(Key::A), key(Key::S)]);
            play.frame_ran();
        }
        assert!(holds(&mut play, &[]).is_empty());
        play.act("no-such");
        assert!(holds(&mut play, &[]).is_empty());
    }

    #[test]
    fn a_key_the_spectrum_lacks_does_its_action() {
        let mapping = Mapping::new(Some(&def(ACTIONS)));
        let tab = mapping.host_key("Tab").expect("Tab does an action");
        assert!(mapping.host_key("F7").is_none());
        let mut play = Play::new(mapping);
        assert_eq!(
            play.pressed(Part::R1),
            Some("next_route".to_owned()),
            "its script"
        );
        assert!(holds(&mut play, &[Part::R1]).is_empty(), "and nothing else");
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
