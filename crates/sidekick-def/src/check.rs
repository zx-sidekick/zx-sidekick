//! What makes a definition complete and consistent, beyond its shape.

use crate::{
    Add, Address, Amount, Charset, Count, Definition, Field, Layout, Machine, MarkerShape,
    OneOrMany, PadTarget, Patch, Role, Test, Value, ValueType, WidgetKind, SCHEMA,
};
use std::{collections::BTreeSet, fmt};

/// One problem, with the path to where it is (`values.lives.addr`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Where: the section and key, dotted.
    pub path: String,
    /// What's wrong.
    pub message: String,
    /// Only worth saying: the definition works, but maybe not as its author meant.
    pub warning: bool,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = if self.warning { "warning: " } else { "" };
        write!(f, "{}: {kind}{}", self.path, self.message)
    }
}

/// The Spectrum's keys, as definitions name them.
const KEYS: &[&str] = &[
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "0",
    "Q",
    "W",
    "E",
    "R",
    "T",
    "Y",
    "U",
    "I",
    "O",
    "P",
    "A",
    "S",
    "D",
    "F",
    "G",
    "H",
    "J",
    "K",
    "L",
    "ENTER",
    "CAPS SHIFT",
    "Z",
    "X",
    "C",
    "V",
    "B",
    "N",
    "M",
    "SYMBOL SHIFT",
    "SPACE",
];

/// What a pad part or an action may name besides an action.
const DIRECTIONS: &[&str] = &["up", "down", "left", "right", "fire"];

/// Every role a state can have.
const ROLES: [Role; 4] = [Role::Title, Role::Play, Role::Paused, Role::GameOver];

/// The pad's parts a notice's hint, or a How to play entry, can show: the face buttons, the
/// directions and Start.
const NOTICE_PARTS: &[&str] = &["dpad", "stick", "west", "south", "east", "north", "start"];

/// The program's own keys, which an action can't take.
const PROGRAM_KEYS: &[&str] = &["Esc", "F5", "F9", "F11"];

/// The keys that an action can take, by SDL's names for them: none the keyboard plays as a
/// pad with (the arrows, W A S D, Z X C V, Space, Enter, the right Control and Alt, `/`) or
/// types on the Spectrum with (letters, digits, the shifts, Backspace).
const HOST_KEYS: &[&str] = &[
    "Tab", "F1", "F2", "F3", "F4", "F6", "F7", "F8", "F10", "F12", "Insert", "Delete", "Home",
    "End", "PageUp", "PageDown",
];

/// A value type's name, as definitions write it.
fn name_of(kind: ValueType) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

struct Checker<'a> {
    def: &'a Definition,
    problems: Vec<Problem>,
}

impl Checker<'_> {
    fn report(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.problems.push(Problem {
            path: path.into(),
            message: message.into(),
            warning: false,
        });
    }

    /// Says something that works but may not be what was meant.
    fn warn(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.problems.push(Problem {
            path: path.into(),
            message: message.into(),
            warning: true,
        });
    }

    fn value(&self, name: &str) -> Option<&Value> {
        self.def.values.get(name)
    }

    fn state_exists(&self, name: &str) -> bool {
        self.def.states.iter().any(|s| s.name == name)
    }

    fn action_exists(&self, id: &str) -> bool {
        self.def.actions.iter().any(|a| a.id == id)
    }

    fn need_state(&mut self, path: &str, name: &str) {
        if !self.state_exists(name) {
            self.report(path, format!("no state named {name:?}"));
        }
    }

    fn need_value(&mut self, path: &str, name: &str) -> Option<&Value> {
        if self.value(name).is_none() {
            self.report(path, format!("no value named {name:?}"));
        }
        self.def.values.get(name)
    }

    fn need_writable_number(&mut self, path: &str, name: &str) {
        if let Some(value) = self.need_value(path, name) {
            let (number, writable) = (value.kind.is_number(), value.writable);
            if !number {
                self.report(path, format!("{name:?} isn't a number"));
            }
            if !writable {
                self.report(path, format!("{name:?} isn't writable"));
            }
        }
    }

    fn keys(&mut self, path: &str, keys: &[String]) {
        for key in keys {
            if !KEYS.contains(&key.as_str()) {
                self.report(path, format!("{key:?} isn't a Spectrum key"));
            }
        }
    }

    fn address(&mut self, path: &str, addr: Option<Address>, bank: Option<u8>) {
        if addr.is_none() {
            self.report(format!("{path}.addr"), "missing");
        }
        if let Some(bank) = bank {
            if self.def.game.machine != Machine::Spectrum128K {
                self.report(format!("{path}.bank"), "banks are only on a 128K");
            } else if bank > 7 {
                self.report(format!("{path}.bank"), "a 128K's banks are 0 to 7");
            }
            if addr.is_some_and(|a| a.0 < 0xC000) {
                self.report(
                    format!("{path}.bank"),
                    "a bank is paged in at 0xC000 and up",
                );
            }
        }
    }

    fn patches(&mut self, path: &str, patches: &[Patch]) {
        for (i, patch) in patches.iter().enumerate() {
            let path = format!("{path}[{i}]");
            self.address(&path, patch.addr, patch.bank);
            if patch.bytes.0.is_empty() {
                self.report(format!("{path}.bytes"), "empty");
            }
            if patch.bytes.0.len() != patch.expect.0.len() {
                self.report(&path, "bytes and expect differ in length");
            }
        }
    }

    fn charset(&mut self, path: &str, charset: Option<&Charset>) {
        if let Some(Charset::Table { chars, .. }) = charset {
            if chars.is_empty() {
                self.report(format!("{path}.charset.chars"), "empty");
            }
        }
    }

    fn len_for(&mut self, path: &str, kind: ValueType, len: Option<u32>) {
        if kind.has_len() && len.is_none() {
            self.report(
                format!("{path}.len"),
                format!("a {} value needs its length", name_of(kind)),
            );
        }
    }

    fn test(&mut self, path: &str, test: &Test) {
        let combines = !test.all.is_empty() || !test.any.is_empty();
        match &test.value {
            Some(name) => {
                self.need_value(&format!("{path}.value"), name);
            }
            None if !combines => self.report(path, "names no value, and combines no tests"),
            None => {}
        }
        for (i, t) in test.all.iter().enumerate() {
            self.test(&format!("{path}.all[{i}]"), t);
        }
        for (i, t) in test.any.iter().enumerate() {
            self.test(&format!("{path}.any[{i}]"), t);
        }
    }

    fn field(&mut self, path: &str, field: &Field, stride: Option<u32>) {
        if matches!(
            field.kind,
            ValueType::Table | ValueType::Font | ValueType::Sprites
        ) {
            self.report(
                format!("{path}.type"),
                "a field can't be a table, font or sprites",
            );
        }
        self.len_for(path, field.kind, field.len);
        self.charset(path, field.charset.as_ref());
        if let Some(stride) = stride {
            if field.offset + field.len.unwrap_or(1) > stride {
                self.report(path, "runs past the record's stride");
            }
        }
    }

    fn game(&mut self) {
        let id = &self.def.game.id;
        let ok = !id.is_empty()
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if !ok {
            self.report("game.id", "lowercase letters, digits, - and _ only");
        }
        if self.def.media.is_empty() {
            self.report("media", "names no dump");
        }
        for (i, media) in self.def.media.iter().enumerate() {
            match &media.sha1 {
                None => self.report(format!("media[{i}].sha1"), "missing"),
                Some(sha1) if sha1.len() != 40 || !sha1.chars().all(|c| c.is_ascii_hexdigit()) => {
                    self.report(format!("media[{i}].sha1"), "40 hex digits");
                }
                Some(_) => {}
            }
        }
    }

    fn run(&mut self) {
        let Some(run) = &self.def.run else { return };
        match (&run.joystick, &run.keys) {
            (crate::Joystick::Keys, None) => {
                self.report(
                    "run.keys",
                    "joystick = \"keys\" needs the game's keys for each direction",
                );
            }
            (crate::Joystick::Keys, Some(keys)) => {
                let named =
                    [&keys.up, &keys.down, &keys.left, &keys.right, &keys.fire].map(Clone::clone);
                self.keys("run.keys", &named);
            }
            (_, Some(_)) => self.report("run.keys", "only with joystick = \"keys\""),
            (_, None) => {}
        }
        if !run.keyboard_in.is_empty() && run.keyboard != crate::Keyboard::Spectrum {
            self.report("run.keyboard_in", "only with keyboard = \"spectrum\"");
        }
        for state in &run.keyboard_in {
            self.need_state("run.keyboard_in", state);
        }
    }

    fn notice(&mut self, path: &str, state: &crate::State) {
        let Some(notice) = &state.notice else { return };
        if !matches!(state.role, Some(Role::Paused | Role::Title)) {
            self.report(
                path,
                "only the states with role = \"paused\" or \"title\" have a notice",
            );
        }
        for (i, hint) in notice.hints.iter().enumerate() {
            let path = format!("{path}.hints[{i}]");
            if hint.keys.is_none() && hint.pad.is_none() {
                self.report(&path, "needs keys, pad or both");
            }
            if let Some(pad) = &hint.pad {
                if !NOTICE_PARTS.contains(&pad.as_str()) {
                    self.report(
                        format!("{path}.pad"),
                        format!(
                            "{pad:?} isn't a part a notice shows: one of {}",
                            NOTICE_PARTS.join(", ")
                        ),
                    );
                }
            }
        }
    }

    fn states(&mut self) {
        let mut seen = BTreeSet::new();
        let mut roles = BTreeSet::new();
        for (i, state) in self.def.states.iter().enumerate() {
            self.notice(&format!("states[{i}].notice"), state);
            for set in state.set.iter().flat_map(OneOrMany::iter) {
                self.need_writable_number(&format!("states[{i}].set"), &set.value);
            }
            let path = format!("states[{i}]");
            if !seen.insert(&state.name) {
                self.report(
                    format!("{path}.name"),
                    format!("{:?} is named twice", state.name),
                );
            }
            if let Some(role) = state.role {
                if !roles.insert(role) {
                    self.report(
                        format!("{path}.role"),
                        format!("another state is already {:?}", role.name()),
                    );
                }
            }
            // A name the program once gave a meaning: say the meaning is a role now.
            let named_as_role = ROLES.iter().find(|r| r.name() == state.name);
            if let (Some(role), None) = (named_as_role, state.role) {
                self.warn(
                    format!("{path}.role"),
                    format!(
                        "a state named {:?} means nothing by its name: add role = {:?} for that",
                        state.name,
                        role.name()
                    ),
                );
            }
            if state.pc.is_none() && state.when.is_none() {
                self.report(&path, "needs pc or when");
            }
            if let Some(when) = &state.when {
                self.test(&format!("{path}.when"), when);
            }
        }
    }

    fn values(&mut self) {
        for (name, value) in &self.def.values {
            let path = format!("values.{name}");
            self.address(&path, value.addr, value.bank);
            self.len_for(&path, value.kind, value.len);
            self.charset(&path, value.charset.as_ref());
            if value.writable && !value.kind.is_number() && value.kind != ValueType::Bitset {
                self.report(
                    format!("{path}.writable"),
                    "only numbers and bitsets can be written",
                );
            }
            match value.kind {
                ValueType::Table => {
                    if value.count.is_none() {
                        self.report(format!("{path}.count"), "missing");
                    }
                    if value.stride.is_none() {
                        self.report(format!("{path}.stride"), "missing");
                    }
                    if value.fields.is_empty() {
                        self.report(format!("{path}.fields"), "a table needs fields");
                    }
                    for (field_name, field) in &value.fields {
                        self.field(&format!("{path}.fields.{field_name}"), field, value.stride);
                    }
                }
                ValueType::Font => {
                    for (key, missing) in [
                        ("first", value.first.is_none()),
                        ("count", value.count.is_none()),
                        ("width", value.width.is_none()),
                        ("height", value.height.is_none()),
                    ] {
                        if missing {
                            self.report(format!("{path}.{key}"), "missing");
                        }
                    }
                }
                ValueType::Sprites => {
                    for (key, missing) in [
                        ("count", value.count.is_none()),
                        ("width", value.width.is_none()),
                        ("height", value.height.is_none()),
                    ] {
                        if missing {
                            self.report(format!("{path}.{key}"), "missing");
                        }
                    }
                    if value.width.is_some_and(|w| w == 0 || w % 8 != 0) {
                        self.report(format!("{path}.width"), "a multiple of 8");
                    }
                    let in_cells = value.layout == Some(Layout::Cells);
                    if in_cells && value.height.is_some_and(|h| h % 8 != 0) {
                        self.report(format!("{path}.height"), "a multiple of 8, for cells");
                    }
                    if let Some(colors) = &value.colors {
                        self.need_value(&format!("{path}.colors"), &colors.value);
                    }
                }
                _ => {}
            }
            if let Some(Count::Value(counted)) = &value.count {
                self.need_value(&format!("{path}.count"), counted);
            }
        }
    }

    fn cheats(&mut self) {
        let mut seen = BTreeSet::new();
        for (i, cheat) in self.def.cheats.iter().enumerate() {
            let path = format!("cheats[{i}]");
            if !seen.insert(&cheat.id) {
                self.report(
                    format!("{path}.id"),
                    format!("{:?} is used twice", cheat.id),
                );
            }
            if cheat.set.is_none() && cheat.hold.is_none() && cheat.patch.is_empty() {
                self.report(&path, "needs set, hold or patch");
            }
            for set in cheat.set.iter().flat_map(OneOrMany::iter) {
                let set_path = format!("{path}.set");
                self.need_writable_number(&set_path, &set.value);
                if matches!(set.to, Amount::Max(_))
                    && self.value(&set.value).is_some_and(|v| v.max.is_none())
                {
                    self.report(
                        &set_path,
                        format!("{:?} has no max to fill it to", set.value),
                    );
                }
            }
            for held in cheat.hold.iter().flat_map(OneOrMany::iter) {
                self.need_writable_number(&format!("{path}.hold"), held);
            }
            self.patches(&format!("{path}.patch"), &cheat.patch);
            if cheat.call.is_some() && self.def.run.as_ref().and_then(|r| r.sync).is_none() {
                self.report(
                    format!("{path}.call"),
                    "a routine is called at the sync point, and there's none (run.sync)",
                );
            }
        }
    }

    fn pad(&mut self) {
        for (state, pad) in &self.def.pad {
            if state != "default" && !self.state_exists(state) {
                self.report(format!("pad.{state}"), format!("no state named {state:?}"));
            }
            for (part, target) in pad.parts() {
                let path = format!("pad.{state}.{part}");
                if part == "select" && *target != PadTarget::Named("menu".to_owned()) {
                    self.report(&path, "select always opens the guidance menu");
                    continue;
                }
                if part == "any" && state == "default" {
                    self.report(&path, "any is for a state's own table, not every state's");
                    continue;
                }
                match target {
                    PadTarget::Named(name) => {
                        let ok = match name.as_str() {
                            "joystick" => matches!(part, "dpad" | "stick"),
                            "menu" | "none" => true,
                            n => DIRECTIONS.contains(&n) || self.action_exists(n),
                        };
                        if !ok {
                            self.report(
                                path,
                                format!(
                                    "{name:?} isn't a direction, fire, menu, none or an action"
                                ),
                            );
                        }
                    }
                    PadTarget::Table(table) => {
                        self.keys(&path, &table.keys);
                        self.keys(&path, &table.tap);
                        self.keys(&path, &table.sequence);
                        if let Some(action) = &table.action {
                            if !self.action_exists(action) {
                                self.report(&path, format!("no action named {action:?}"));
                            }
                        }
                        let taps = table.tap.is_empty() && table.sequence.is_empty();
                        match (table.frames, taps) {
                            (Some(0), false) => {
                                self.report(&path, "a tap lasts at least one frame");
                            }
                            (Some(_), true) => {
                                self.report(&path, "frames is only for a tap or a sequence");
                            }
                            _ => {}
                        }
                        if table.keys.is_empty() && taps && table.action.is_none() {
                            self.report(path, "needs keys, tap, sequence or action");
                        }
                    }
                }
            }
        }
    }

    fn actions(&mut self) {
        let mut seen = BTreeSet::new();
        for (i, action) in self.def.actions.iter().enumerate() {
            let path = format!("actions[{i}]");
            if !seen.insert(&action.id) {
                self.report(
                    format!("{path}.id"),
                    format!("{:?} is used twice", action.id),
                );
            }
            self.keys(&path, &action.keys);
            self.keys(&path, &action.tap);
            self.keys(&path, &action.sequence);
            for direction in &action.joystick {
                if !DIRECTIONS.contains(&direction.as_str()) {
                    self.report(
                        format!("{path}.joystick"),
                        format!("{direction:?} isn't a direction or fire"),
                    );
                }
            }
            self.patches(&format!("{path}.patch_for_frame"), &action.patch_for_frame);
            for state in &action.menu_in {
                self.need_state(&format!("{path}.menu_in"), state);
            }
            // A row whose action lasts no frame and calls nothing would do nothing.
            if action.menu.is_some()
                && action.tap.is_empty()
                && action.sequence.is_empty()
                && action.frames.is_none()
                && action.script.is_none()
            {
                self.report(
                    format!("{path}.menu"),
                    "a menu row's action needs tap, sequence, frames or script, or it does nothing",
                );
            }
            if !action.menu_in.is_empty() && action.menu.is_none() {
                self.report(format!("{path}.menu_in"), "only with menu");
            }
            if action.id == "end-game" && action.menu.is_none() {
                self.warn(
                    format!("{path}.menu"),
                    "an action named \"end-game\" means nothing by its name: add menu = \"End this game\" for its row",
                );
            }
            if action.script.is_some() && self.def.script.is_none() {
                self.report(format!("{path}.script"), "the definition has no [script]");
            }
            if action.keys.is_empty()
                && action.tap.is_empty()
                && action.sequence.is_empty()
                && action.joystick.is_empty()
                && action.patch_for_frame.is_empty()
                && action.script.is_none()
            {
                self.report(&path, "does nothing");
            }
            if let Some(key) = &action.host_key {
                if PROGRAM_KEYS.contains(&key.as_str()) {
                    self.report(
                        format!("{path}.host_key"),
                        format!("{key} is the program's own"),
                    );
                } else if !HOST_KEYS.contains(&key.as_str()) {
                    self.report(
                        format!("{path}.host_key"),
                        format!(
                            "{key:?} isn't one of the keys the Spectrum lacks: {}",
                            HOST_KEYS.join(", ")
                        ),
                    );
                }
            }
        }
    }

    fn entries(&mut self) {
        let sections = [
            ("text_entry", &self.def.text_entry),
            ("name_entry", &self.def.name_entry),
        ];
        for (key, entry) in sections {
            let Some(entry) = entry else {
                continue;
            };
            if !self.state_exists(&entry.state) {
                self.report(
                    format!("{key}.state"),
                    format!("no state named {:?}", entry.state),
                );
            }
            if entry.letters.is_empty() {
                self.report(format!("{key}.letters"), "empty");
            }
            if entry.length == 0 {
                self.report(format!("{key}.length"), "at least 1");
            }
            if let Some(list) = &entry.choices {
                if !self.def.persist.contains_key(list) {
                    self.report(
                        format!("{key}.choices"),
                        format!("no kept list named {list:?}"),
                    );
                }
            }
        }
        let Some(names) = &self.def.name_entry else {
            return;
        };
        if self
            .def
            .text_entry
            .as_ref()
            .is_some_and(|t| t.state == names.state)
        {
            self.report(
                "name_entry.state",
                "text_entry's state: a state has one card",
            );
        }
        match self.def.high_scores.as_ref().and_then(|t| t.name.as_ref()) {
            None => self.report("name_entry", "needs [high_scores] with its name"),
            Some(name) if name.len.is_some_and(|len| names.length > len) => {
                self.report("name_entry.length", "longer than high_scores.name");
            }
            Some(_) => {}
        }
    }

    fn persist(&mut self) {
        for (name, list) in &self.def.persist {
            match &list.add {
                Some(Add::Value { value, in_state }) => {
                    let path = format!("persist.{name}.add");
                    self.need_value(&path, value);
                    if !self.state_exists(in_state) {
                        self.report(path, format!("no state named {in_state:?}"));
                    }
                }
                Some(Add::Script(_)) if self.def.script.is_none() => {
                    self.report(
                        format!("persist.{name}.add"),
                        "the definition has no [script]",
                    );
                }
                _ => {}
            }
        }
    }

    fn high_scores(&mut self) {
        let Some(table) = &self.def.high_scores else {
            return;
        };
        self.address("high_scores", table.addr, None);
        for (key, missing) in [
            ("count", table.count.is_none()),
            ("stride", table.stride.is_none()),
            ("name", table.name.is_none()),
            ("score", table.score.is_none()),
        ] {
            if missing {
                self.report(format!("high_scores.{key}"), "missing");
            }
        }
        if let Some(name) = &table.name {
            self.field("high_scores.name", name, table.stride);
            if name.kind != ValueType::Text {
                self.report("high_scores.name", "a name is text");
            }
        }
        if let Some(score) = &table.score {
            self.field("high_scores.score", score, table.stride);
            if !matches!(
                score.kind,
                ValueType::Digits | ValueType::Bcd | ValueType::U8 | ValueType::U16
            ) {
                self.report("high_scores.score", "a score is digits, bcd, u8 or u16");
            }
        }
    }

    fn panel(&mut self) {
        let levels = self.def.guidance.as_ref().map_or(0, |g| g.levels.len());
        for (i, widget) in self.def.panel.iter().enumerate() {
            let path = format!("panel[{i}]");
            if usize::try_from(widget.from_level).map_or(true, |l| l >= levels) {
                self.report(
                    format!("{path}.from_level"),
                    format!("the definition has {levels} guidance levels"),
                );
            }
            match widget.source.split_once(':') {
                Some(("value", name)) => {
                    self.need_value(&format!("{path}.source"), name);
                }
                Some(("persist", name)) if !self.def.persist.contains_key(name) => {
                    self.report(
                        format!("{path}.source"),
                        format!("no kept list named {name:?}"),
                    );
                }
                Some(("script", _)) if self.def.script.is_none() => {
                    self.report(format!("{path}.source"), "the definition has no [script]");
                }
                Some(("persist" | "script", _)) => {}
                _ => self.report(
                    format!("{path}.source"),
                    "value:<name>, persist:<list> or script:<result>",
                ),
            }
            for (key, name, kind) in [
                ("pictures", &widget.pictures, ValueType::Sprites),
                ("font", &widget.font, ValueType::Font),
            ] {
                if let Some(name) = name {
                    let p = format!("{path}.{key}");
                    if let Some(value) = self.need_value(&p, name) {
                        if value.kind != kind {
                            self.report(p, format!("{name:?} isn't a {} value", name_of(kind)));
                        }
                    }
                }
            }
            if widget.kind == WidgetKind::IconGrid && widget.pictures.is_none() {
                self.report(format!("{path}.pictures"), "an icon-grid needs pictures");
            }
            match (
                widget.kind == WidgetKind::MapGrid,
                widget.markers.is_empty(),
            ) {
                (true, true) => self.warn(
                    format!("{path}.markers"),
                    "a map-grid's markers aren't drawn until it says each kind's shape",
                ),
                (false, false) => self.report(format!("{path}.markers"), "only for a map-grid"),
                _ => {}
            }
            for kind in widget.places.keys() {
                if !widget.markers.contains_key(kind) {
                    self.report(
                        format!("{path}.places.{kind}"),
                        format!("no marker kind {kind:?} in markers"),
                    );
                }
            }
            let pictured = widget.markers.values().any(|&s| s == MarkerShape::Picture);
            if pictured && widget.pictures.is_none() {
                self.report(
                    format!("{path}.pictures"),
                    "a marker drawn as its picture needs pictures",
                );
            }
        }
    }

    fn hints(&mut self) {
        let levels = self.def.guidance.as_ref().map_or(0, |g| g.levels.len());
        for (i, hint) in self.def.hints.iter().enumerate() {
            let path = format!("hints[{i}]");
            if hint.text.trim().is_empty() {
                self.report(format!("{path}.text"), "says nothing");
            } else if hint.text.contains('\n') {
                self.report(format!("{path}.text"), "one line");
            }
            if hint.from_level == 0 {
                self.report(
                    format!("{path}.from_level"),
                    "hints show from level 1: level 0 is no guidance",
                );
            } else if usize::try_from(hint.from_level).map_or(true, |l| l >= levels) {
                self.report(
                    format!("{path}.from_level"),
                    format!("the definition has {levels} guidance levels"),
                );
            }
            if let Some(to) = hint.to_level {
                if to < hint.from_level {
                    self.report(
                        format!("{path}.to_level"),
                        format!("below its from_level, {}", hint.from_level),
                    );
                } else if usize::try_from(to).map_or(true, |l| l >= levels) {
                    self.report(
                        format!("{path}.to_level"),
                        format!("the definition has {levels} guidance levels"),
                    );
                }
            }
            for name in hint.state.iter().flat_map(OneOrMany::iter) {
                self.need_state(&format!("{path}.state"), name);
            }
            if let Some(when) = &hint.when {
                self.test(&format!("{path}.when"), when);
            }
            if hint.script.is_some() && self.def.script.is_none() {
                self.report(format!("{path}.script"), "the definition has no [script]");
            }
            if hint.for_seconds == Some(0) {
                self.report(format!("{path}.for_seconds"), "at least 1");
            }
            if hint.state.is_none() && hint.when.is_none() && hint.script.is_none() {
                self.report(&path, "needs state, when or script");
            }
        }
    }

    fn about(&mut self) {
        let Some(about) = &self.def.about else { return };
        if about.goal.trim().is_empty() {
            self.report("about.goal", "says nothing");
        }
        for (i, how) in about.how.iter().enumerate() {
            let path = format!("about.how[{i}]");
            for (key, words) in [("name", &how.name), ("text", &how.text)] {
                if words.trim().is_empty() {
                    self.report(format!("{path}.{key}"), "says nothing");
                } else if words.contains('\n') {
                    self.report(format!("{path}.{key}"), "one line");
                }
            }
            if let Some(pad) = &how.pad {
                if !NOTICE_PARTS.contains(&pad.as_str()) {
                    self.report(
                        format!("{path}.pad"),
                        format!(
                            "{pad:?} isn't a part the screen shows: one of {}",
                            NOTICE_PARTS.join(", ")
                        ),
                    );
                }
            }
        }
        if self.def.state_for(Role::Play).is_none() {
            self.report(
                "about",
                "shown when a game starts: it needs a state with role = \"play\"",
            );
        }
    }

    fn situations(&mut self) {
        if let Some(situations) = &self.def.situations {
            if situations.enter.is_none() {
                self.report("situations.enter", "missing");
            }
            self.keys("situations.start", &situations.start);
            if self.def.state_for(Role::Play).is_none() {
                self.report(
                    "situations",
                    "a situation enters play: it needs a state with role = \"play\"",
                );
            }
        }
    }
}

pub(crate) fn check(def: &Definition) -> Vec<Problem> {
    let mut checker = Checker {
        def,
        problems: Vec::new(),
    };
    if def.schema != SCHEMA {
        checker.report("schema", format!("this program reads format {SCHEMA}"));
    }
    checker.game();
    checker.run();
    checker.states();
    checker.values();
    checker.cheats();
    checker.pad();
    checker.actions();
    checker.entries();
    checker.persist();
    checker.high_scores();
    checker.panel();
    checker.hints();
    checker.about();
    checker.situations();
    checker.problems
}
