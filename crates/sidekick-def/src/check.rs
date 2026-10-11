//! What makes a definition complete and consistent, beyond its shape.

use crate::{
    Add, Address, Amount, Charset, Count, Definition, Field, Machine, OneOrMany, PadTarget, Patch,
    Test, Value, ValueType, WidgetKind, SCHEMA,
};
use std::{collections::BTreeSet, fmt};

/// One problem, with the path to where it is (`values.lives.addr`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Where: the section and key, dotted.
    pub path: String,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
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

/// The program's own keys, which an action can't take.
const PROGRAM_KEYS: &[&str] = &["Esc", "F5", "F9", "F11"];

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

    fn states(&mut self) {
        let mut seen = BTreeSet::new();
        for (i, state) in self.def.states.iter().enumerate() {
            let path = format!("states[{i}]");
            if !seen.insert(&state.name) {
                self.report(
                    format!("{path}.name"),
                    format!("{:?} is named twice", state.name),
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
        }
    }

    fn pad(&mut self) {
        for (state, pad) in &self.def.pad {
            if state != "default" && !self.state_exists(state) {
                self.report(format!("pad.{state}"), format!("no state named {state:?}"));
            }
            for (part, target) in pad.parts() {
                let path = format!("pad.{state}.{part}");
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
                        if let Some(action) = &table.action {
                            if !self.action_exists(action) {
                                self.report(&path, format!("no action named {action:?}"));
                            }
                        }
                        if table.keys.is_empty() && table.tap.is_empty() && table.action.is_none() {
                            self.report(path, "needs keys, tap or action");
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
            for direction in &action.joystick {
                if !DIRECTIONS.contains(&direction.as_str()) {
                    self.report(
                        format!("{path}.joystick"),
                        format!("{direction:?} isn't a direction or fire"),
                    );
                }
            }
            self.patches(&format!("{path}.patch_for_frame"), &action.patch_for_frame);
            if action.script.is_some() && self.def.script.is_none() {
                self.report(format!("{path}.script"), "the definition has no [script]");
            }
            if action.keys.is_empty()
                && action.tap.is_empty()
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
                }
            }
        }
    }

    fn text_entry(&mut self) {
        let Some(entry) = &self.def.text_entry else {
            return;
        };
        if !self.state_exists(&entry.state) {
            self.report(
                "text_entry.state",
                format!("no state named {:?}", entry.state),
            );
        }
        if entry.letters.is_empty() {
            self.report("text_entry.letters", "empty");
        }
        if entry.length == 0 {
            self.report("text_entry.length", "at least 1");
        }
        if let Some(list) = &entry.choices {
            if !self.def.persist.contains_key(list) {
                self.report("text_entry.choices", format!("no kept list named {list:?}"));
            }
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
        }
        if let Some(score) = &table.score {
            self.field("high_scores.score", score, table.stride);
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
        }
    }

    fn situations(&mut self) {
        if let Some(situations) = &self.def.situations {
            if situations.enter.is_none() {
                self.report("situations.enter", "missing");
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
    checker.states();
    checker.values();
    checker.cheats();
    checker.pad();
    checker.actions();
    checker.text_entry();
    checker.persist();
    checker.high_scores();
    checker.panel();
    checker.situations();
    checker.problems
}
