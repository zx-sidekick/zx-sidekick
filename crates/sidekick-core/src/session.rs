//! A game running: its machine and its definition, the named values read each frame, the
//! game's state, and writes to it by name.

use crate::{
    value::{self, Read, WriteError},
    Error, Machine, Stop,
};
use sidekick_def::{Amount, Cheat, Definition, Patch, Role, State, Test};
use sidekick_script::{Fact, Script, View};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// Every named value, as read at one moment.
pub type Reading = BTreeMap<String, Read>;

/// What a game was played with: the highest guidance level, and the training switches on at
/// any time that mark the score.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    /// The highest guidance level used.
    pub level: u8,
    /// The switches used, by id.
    pub switches: BTreeSet<String>,
    /// Whether a save was loaded into the game.
    pub loaded: bool,
}

/// A moment in a session the program acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Milestone {
    /// The game reached its title for the first time: time to put its kept high scores in.
    FirstTitle,
    /// The title came round after a game: time to read the high scores back.
    GameEnded,
}

/// An entry of a high-score table: its name and its score, as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreEntry {
    /// The name, without trailing spaces.
    pub name: String,
    /// The score.
    pub score: String,
}

/// Why a switch didn't go on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitchError {
    /// No training switch has that id.
    Unknown,
    /// A patch's `expect` bytes aren't in memory at its address: the tape, or the moment, isn't
    /// what the definition expects.
    Mismatch {
        /// The patch's address.
        addr: u16,
    },
}

impl fmt::Display for SwitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("no training switch has that id"),
            Self::Mismatch { addr } => write!(
                f,
                "the game's code at 0x{addr:04X} isn't what the definition expects"
            ),
        }
    }
}

impl std::error::Error for SwitchError {}

/// A game being played: the machine, and what its definition says about it.
#[derive(Clone)]
pub struct Session {
    /// Boxed: the machine is tens of kilobytes, too much to move about on a small stack
    /// (Windows gives its main thread 1 MB, less in a debug build's frames).
    machine: Box<Machine>,
    definition: Option<Definition>,
    reading: Reading,
    state: Option<String>,
    /// Held on the loading screen, until released.
    holding: bool,
    /// The training switches on, by id.
    switches: BTreeSet<String>,
    /// The guidance level, and what the game being played has used.
    level: u8,
    record: Record,
    /// The definition's script, whether `on_load` has run, and what it returned last.
    script: Option<Script>,
    script_loaded: bool,
    results: BTreeMap<String, Fact>,
    /// Why the script isn't running: it failed, or couldn't be compiled.
    script_off: Option<String>,
    /// The kept lists, by name, and those changed since the program last saved them.
    kept: BTreeMap<String, Vec<String>>,
    kept_changed: BTreeSet<String>,
    /// How far the session's got, and the milestones not yet taken.
    progress: Progress,
    milestones: Vec<Milestone>,
    /// What carries over to what comes next.
    next: Next,
    /// Routines switches turned on asked for, called at the next sync point in play.
    calls: Vec<u16>,
}

/// What carries over from one moment of a session to what comes next.
#[derive(Clone, Copy, Debug, Default)]
struct Next {
    /// A snapshot alone was loaded: the next game's record starts marked loaded.
    loaded: bool,
    /// The machine stopped at the sync address this frame, and the values were read there.
    read_at_sync: bool,
}

/// How far a session's got, as a save keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// The last state that matched, through frames that matched none.
    pub last_known: Option<String>,
    /// Whether the title has been seen.
    pub seen_title: bool,
    /// Whether a game has been played since the title was last.
    pub in_game: bool,
}

impl Session {
    /// The game a definition describes, on this machine; `None` plays a tape plainly, with
    /// nothing named. The machine stops at the definition's sync address and watches its
    /// states' addresses.
    #[must_use]
    pub fn new(mut machine: Machine, definition: Option<Definition>) -> Self {
        let sync = definition
            .as_ref()
            .and_then(|d| d.run.as_ref())
            .and_then(|r| r.sync)
            .map(|a| a.0);
        machine.set_sync(sync);
        let hold = definition
            .as_ref()
            .and_then(|d| d.run.as_ref())
            .is_some_and(|r| r.hold_loading_screen);
        machine.stop_when_loaded(hold);
        let watched: Vec<u16> = definition
            .iter()
            .flat_map(|d| &d.states)
            .flat_map(|s| s.pc.iter().flatten())
            .map(|a| a.0)
            .collect();
        machine.watch(&watched);
        Self {
            machine: Box::new(machine),
            definition,
            reading: Reading::new(),
            state: None,
            holding: false,
            switches: BTreeSet::new(),
            level: 0,
            record: Record::default(),
            script: None,
            script_loaded: false,
            results: BTreeMap::new(),
            script_off: None,
            kept: BTreeMap::new(),
            kept_changed: BTreeSet::new(),
            progress: Progress::default(),
            milestones: Vec::new(),
            next: Next::default(),
            calls: Vec::new(),
        }
    }

    /// Runs a frame: the values are read when it reaches the sync address, or at its end if
    /// it doesn't, and the state is decided at its end. When the definition holds the loading
    /// screen (`[run] hold_loading_screen`), the frame stops the moment the tape's last block
    /// has loaded, with the screen drawn from memory; nothing runs until
    /// [`Session::release`], and the next call carries the frame on.
    ///
    /// # Errors
    /// If the machine fails.
    pub fn run_frame(&mut self) -> Result<(), Error> {
        if self.holding {
            return Ok(());
        }
        // A frame stopped at the sync address by `run_to_sync` was read there already.
        let mut read = std::mem::take(&mut self.next.read_at_sync);
        loop {
            match self.machine.run()? {
                Stop::Sync => {
                    self.take_reading();
                    self.make_calls();
                    read = true;
                }
                Stop::Loaded => {
                    self.holding = true;
                    self.machine.draw_screen_from_memory();
                    return Ok(());
                }
                Stop::FrameEnd => break,
            }
        }
        if !read {
            self.take_reading();
        }
        self.end_frame();
        Ok(())
    }

    /// Runs on to the sync address, stopping there mid-frame, so what's done next is done at
    /// the top of the game's main loop; the next [`Session::run_frame`] finishes the frame.
    /// Returns false, having run no further, if the frame ended (or the tape loaded) first.
    ///
    /// # Errors
    /// If the machine fails.
    pub fn run_to_sync(&mut self) -> Result<bool, Error> {
        if self.holding {
            return Ok(false);
        }
        match self.machine.run()? {
            Stop::Sync => {
                self.take_reading();
                self.next.read_at_sync = true;
                Ok(true)
            }
            Stop::Loaded => {
                self.holding = true;
                self.machine.draw_screen_from_memory();
                Ok(false)
            }
            Stop::FrameEnd => {
                self.take_reading();
                self.end_frame();
                Ok(false)
            }
        }
    }

    /// What follows a frame's end: the state decided, games tracked, the script run, and the
    /// kept lists filled.
    fn end_frame(&mut self) {
        self.state = self.decide();
        self.track_games();
        self.run_script();
        self.keep_from_values();
    }

    /// Gives the session its kept lists, as the program last saved them.
    pub fn set_kept(&mut self, kept: BTreeMap<String, Vec<String>>) {
        self.kept = kept;
    }

    /// The kept lists, by name.
    #[must_use]
    pub const fn kept(&self) -> &BTreeMap<String, Vec<String>> {
        &self.kept
    }

    /// The names of the lists changed since this was last asked, for the program to save.
    pub fn take_kept_changes(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.kept_changed)
    }

    /// Empties every kept list.
    pub fn forget_kept(&mut self) {
        for (name, list) in &mut self.kept {
            if !list.is_empty() {
                list.clear();
                self.kept_changed.insert(name.clone());
            }
        }
    }

    /// Adds an entry to a kept list the definition has, unless it's there or empty.
    fn keep(&mut self, list: &str, entry: &str) {
        let entry = entry.trim();
        let Some(persist) = self.definition.as_ref().and_then(|d| d.persist.get(list)) else {
            return;
        };
        if entry.is_empty() {
            return;
        }
        let most = persist
            .most
            .and_then(|m| usize::try_from(m).ok())
            .unwrap_or(usize::MAX);
        let entries = self.kept.entry(list.to_owned()).or_default();
        if !entries.iter().any(|e| e == entry) {
            entries.push(entry.to_owned());
            // The oldest go first, past the list's most.
            let over = entries.len().saturating_sub(most);
            entries.drain(..over);
            self.kept_changed.insert(list.to_owned());
        }
    }

    /// Adds what the game shows to the lists that keep a value in a state.
    fn keep_from_values(&mut self) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let adds: Vec<(String, String)> = self
            .definition
            .iter()
            .flat_map(|d| &d.persist)
            .filter_map(|(list, persist)| match &persist.add {
                Some(sidekick_def::Add::Value { value, in_state }) if *in_state == state => {
                    let entry = match self.reading.get(value)? {
                        Read::Text(t) => t.clone(),
                        Read::Number(n) => n.to_string(),
                        _ => return None,
                    };
                    Some((list.clone(), entry))
                }
                _ => None,
            })
            .collect();
        for (list, entry) in adds {
            self.keep(&list, &entry);
        }
    }

    /// Adds what a script returned under `kept`: a map of list names to new entries.
    fn keep_from_script(&mut self, results: &mut BTreeMap<String, Fact>) {
        let Some(Fact::Map(lists)) = results.remove("kept") else {
            return;
        };
        for (list, entries) in lists {
            let entries = match entries {
                Fact::List(entries) => entries,
                one => vec![one],
            };
            for entry in entries {
                if let Fact::Text(text) = entry {
                    self.keep(&list, &text);
                }
            }
        }
    }

    /// Gives the session its definition's script, compiled; or why it couldn't be.
    pub fn set_script(&mut self, script: Result<Script, String>) {
        match script {
            Ok(script) => self.script = Some(script),
            Err(why) => self.script_off = Some(why),
        }
    }

    /// Runs the script for a frame: `on_load` once, when the tape has loaded or the game's
    /// state is first known; then `on_frame`.
    fn run_script(&mut self) {
        if self.script.is_none() || self.script_off.is_some() {
            return;
        }
        if !self.script_loaded && !self.machine.tape_loaded() && self.state.is_none() {
            return;
        }
        let view = self.view();
        let Some(script) = &mut self.script else {
            return;
        };
        let ran = if self.script_loaded {
            script.on_frame(&view).map(|mut results| {
                self.keep_from_script(&mut results);
                self.results = results;
            })
        } else {
            self.script_loaded = true;
            script.on_load(&view)
        };
        if let Err(why) = ran {
            self.script_off = Some(why);
        }
    }

    /// Runs one of the script's action functions, and makes the writes it asks for through
    /// the one place for writes.
    ///
    /// # Errors
    /// The script is off, has no such function, or failed; or a write was refused.
    pub fn script_action(&mut self, name: &str) -> Result<(), String> {
        if let Some(why) = &self.script_off {
            return Err(why.clone());
        }
        let view = self.view();
        let script = self.script.as_mut().ok_or("the definition has no script")?;
        let (mut results, writes) = script.action(name, &view).inspect_err(|why| {
            if script.off().is_some() {
                self.script_off = Some(why.clone());
            }
        })?;
        self.keep_from_script(&mut results);
        self.results.extend(results);
        for (value, number) in writes {
            self.write(&value, number)
                .map_err(|e| format!("{name}: writing {value}: {e}"))?;
        }
        Ok(())
    }

    /// The milestones passed since this was last asked, in order.
    pub fn take_milestones(&mut self) -> Vec<Milestone> {
        std::mem::take(&mut self.milestones)
    }

    /// The high-score table as it is in memory now, when the definition describes one.
    #[must_use]
    pub fn scores(&self) -> Option<Vec<ScoreEntry>> {
        let table = self.definition.as_ref()?.high_scores.as_ref()?;
        let (name, score) = (table.name.as_ref()?, table.score.as_ref()?);
        let addr = u32::from(table.addr?.0);
        let memory = value::Memory {
            machine: &self.machine,
            bank: None,
        };
        let text = |read: Read| match read {
            Read::Text(t) => t.trim_end().to_owned(),
            Read::Number(n) => n.to_string(),
            other => other.to_string(),
        };
        let rows = (0..table.count?)
            .map(|i| {
                let at = addr + i * table.stride?;
                Some(ScoreEntry {
                    name: text(value::read_field(&memory, name, at)),
                    score: text(value::read_field(&memory, score, at)),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(rows)
    }

    /// Writes entries into the high-score table, in order: the game then shows them. An entry
    /// that can't be written in the table's fields is left as the game has it.
    pub fn set_scores(&mut self, entries: &[ScoreEntry]) {
        let Some(table) = self.definition.as_ref().and_then(|d| d.high_scores.clone()) else {
            return;
        };
        let (Some(name), Some(score), Some(addr), Some(stride), Some(count)) = (
            table.name,
            table.score,
            table.addr,
            table.stride,
            table.count,
        ) else {
            return;
        };
        for (i, entry) in (0..count).zip(entries) {
            let at = u32::from(addr.0) + i * stride;
            let (Some(name_bytes), Some(score_bytes)) = (
                value::encode_field(&name, &entry.name),
                value::encode_field(&score, &entry.score),
            ) else {
                continue;
            };
            for (field, bytes) in [(&name, name_bytes), (&score, score_bytes)] {
                let start = at + field.offset;
                if start < 0x4000 {
                    continue;
                }
                for (a, byte) in (start..).zip(bytes) {
                    if let Ok(a) = u16::try_from(a) {
                        self.machine.write(a, byte);
                    }
                }
            }
        }
    }

    /// What a script sees now.
    fn view(&self) -> View {
        View {
            memory: (0..=u16::MAX).map(|a| self.machine.peek(a)).collect(),
            values: self
                .definition
                .iter()
                .flat_map(|d| d.values.keys())
                .map(|k| (k.clone(), self.reading.get(k).map_or(Fact::Nothing, fact)))
                .collect(),
            state: self.state.clone(),
            level: i64::from(self.level),
            kept: self.kept.clone(),
        }
    }

    /// The game's font a `font` value names, read from memory now.
    #[must_use]
    pub fn font(&self, name: &str) -> Option<crate::Font> {
        let value = self.definition.as_ref()?.values.get(name)?;
        crate::graphics::font(&self.machine, value)
    }

    /// The game's pictures a `sprites` value names, read from memory now, each with its ink
    /// when the value's `colors` gives one.
    #[must_use]
    pub fn sprites(&self, name: &str) -> Option<Vec<crate::Bitmap>> {
        let value = self.definition.as_ref()?.values.get(name)?;
        let colors = value
            .colors
            .as_ref()
            .and_then(|c| self.reading.get(&c.value));
        crate::graphics::sprites(&self.machine, value, colors)
    }

    /// What the script returned last, by name, for `script:` sources.
    #[must_use]
    pub const fn script_results(&self) -> &BTreeMap<String, Fact> {
        &self.results
    }

    /// Why the script isn't running, when it isn't.
    #[must_use]
    pub fn script_off(&self) -> Option<&str> {
        self.script_off.as_deref()
    }

    /// Reads the values for this frame, and lets the switches that are on act on them, in play.
    fn take_reading(&mut self) {
        let now = self.read();
        let previous = std::mem::replace(&mut self.reading, now);
        if !self.in_play() {
            return;
        }
        let Some(definition) = self.definition.take() else {
            return;
        };
        let on: Vec<&Cheat> = definition
            .cheats
            .iter()
            .filter(|c| self.switches.contains(&c.id))
            .collect();
        for cheat in on {
            for set in cheat.set.iter().flatten() {
                let to = match &set.to {
                    Amount::Number(n) => Some(*n),
                    Amount::Max(_) => definition.values.get(&set.value).and_then(|v| v.max),
                };
                if let Some(to) = to {
                    self.write_now(&definition, &set.value, to);
                }
            }
            for name in cheat.hold.iter().flatten() {
                let before = previous.get(name).and_then(Read::number);
                let now = self.reading.get(name).and_then(Read::number);
                if let (Some(before), Some(now)) = (before, now) {
                    if now < before {
                        self.write_now(&definition, name, before);
                    }
                }
            }
        }
        self.definition = Some(definition);
    }

    /// Whether the switches may act: in the state with role `play`, when the definition has one
    /// (the last state known, so a frame that matches none doesn't skip), since outside it a
    /// game may use the same memory for other things.
    fn in_play(&self) -> bool {
        self.definition.as_ref().is_some_and(|d| {
            d.state_for(Role::Play).is_none()
                || d.is(self.progress.last_known.as_deref(), Role::Play)
        })
    }

    /// Calls the routines switches turned on asked for, at the sync point in play: each once,
    /// chained so each returns into the next and the last to the sync address.
    fn make_calls(&mut self) {
        if self.calls.is_empty() || !self.in_play() {
            return;
        }
        let mut back = self.machine.registers().get_pc();
        for addr in std::mem::take(&mut self.calls) {
            self.machine.call(addr, back);
            back = addr;
        }
    }

    /// Writes a value and keeps this frame's reading in step with it. A write refused (a
    /// definition `sidekick check` would fault) does nothing.
    fn write_now(&mut self, definition: &Definition, name: &str, number: i64) {
        let Some(value) = definition.values.get(name) else {
            return;
        };
        if let Ok((addr, bytes)) = value::encode(value, number) {
            for (at, byte) in (addr..).zip(bytes) {
                self.machine.write(at, byte);
            }
            self.reading.insert(name.to_owned(), Read::Number(number));
        }
    }

    /// Starts a new game's record when play begins after the title, a game over, or nothing
    /// known yet. Frames that match no state don't count, so a flicker isn't a new game.
    fn track_games(&mut self) {
        let Some(state) = self.state.clone() else {
            return;
        };
        let (starts, came_to_title) = {
            let Some(definition) = &self.definition else {
                return;
            };
            let after = self.progress.last_known.as_deref();
            let now = Some(state.as_str());
            let from_start = after.is_none()
                || definition.is(after, Role::Title)
                || definition.is(after, Role::GameOver);
            (
                definition.is(now, Role::Play) && from_start,
                definition.is(now, Role::Title) && !definition.is(after, Role::Title),
            )
        };
        if starts {
            self.record = Record {
                loaded: std::mem::take(&mut self.next.loaded),
                ..Record::default()
            };
            self.note_use();
            self.progress.in_game = true;
        }
        if came_to_title {
            if !self.progress.seen_title {
                self.progress.seen_title = true;
                self.milestones.push(Milestone::FirstTitle);
            }
            if std::mem::take(&mut self.progress.in_game) {
                self.milestones.push(Milestone::GameEnded);
            }
        }
        self.progress.last_known = Some(state);
    }

    /// Adds the guidance level and the switches on now to the record.
    fn note_use(&mut self) {
        self.record.level = self.record.level.max(self.level);
        let Some(definition) = &self.definition else {
            return;
        };
        let marking = definition
            .cheats
            .iter()
            .filter(|c| c.marks_score && self.switches.contains(&c.id))
            .map(|c| c.id.clone());
        self.record.switches.extend(marking);
    }

    /// Turns a training switch on or off. On, its patches are applied, all or none; off, the
    /// game's bytes are put back.
    ///
    /// # Errors
    /// If there's no such switch, or one of its patches doesn't find its `expect` bytes; the
    /// switch then stays off and nothing is written.
    pub fn switch(&mut self, id: &str, on: bool) -> Result<(), SwitchError> {
        let cheat: Cheat = self
            .definition
            .as_ref()
            .and_then(|d| d.cheats.iter().find(|c| c.id == id))
            .cloned()
            .ok_or(SwitchError::Unknown)?;
        if on == self.switches.contains(id) {
            return Ok(());
        }
        if on {
            if let Some(patch) = cheat.patch.iter().find(|p| !self.expected(p)) {
                return Err(SwitchError::Mismatch {
                    addr: patch.addr.map_or(0, |a| a.0),
                });
            }
            for patch in &cheat.patch {
                self.patch(patch);
            }
            self.switches.insert(id.to_owned());
            if let Some(addr) = cheat.call {
                if !self.calls.contains(&addr.0) {
                    self.calls.push(addr.0);
                }
            }
            self.note_use();
        } else {
            for patch in &cheat.patch {
                self.unpatch(patch);
            }
            self.switches.remove(id);
        }
        Ok(())
    }

    /// The training switches on, by id.
    #[must_use]
    pub const fn switches_on(&self) -> &BTreeSet<String> {
        &self.switches
    }

    /// The guidance level in use.
    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// How far the session's got, for a save.
    #[must_use]
    pub const fn progress(&self) -> &Progress {
        &self.progress
    }

    /// Puts back what a save held of the program's own: the guidance level, the switches on,
    /// the record, and where the session had got. Switches the definition doesn't have are left
    /// off. A switch's patches are in the saved memory, but one the switch has gained since the
    /// save was taken isn't: it's applied now, where the game's own bytes still are.
    pub fn restore(
        &mut self,
        level: u8,
        switches: &BTreeSet<String>,
        record: Record,
        progress: Progress,
    ) {
        let known: BTreeSet<String> = self
            .definition
            .iter()
            .flat_map(|d| &d.cheats)
            .map(|c| c.id.clone())
            .collect();
        self.level = level;
        self.switches = switches.intersection(&known).cloned().collect();
        let patches: Vec<Patch> = self
            .definition
            .iter()
            .flat_map(|d| &d.cheats)
            .filter(|c| self.switches.contains(&c.id))
            .flat_map(|c| c.patch.iter().cloned())
            .collect();
        for patch in &patches {
            // Writes only over the game's own bytes: a patch already in the save stays as it is.
            self.patch(patch);
        }
        self.record = record;
        // The game the save was taken in goes on, not a new one, in the state it was in.
        self.state.clone_from(&progress.last_known);
        self.progress = progress;
        self.next.loaded = false;
        self.holding = false;
        self.reading_anew();
    }

    /// The values read from the machine as it is now, so the next frame's are told against
    /// them: a hold switch mustn't take the machine before a load, or before values were set
    /// up, for the game's own.
    pub fn reading_anew(&mut self) {
        self.reading = self.read();
    }

    /// A snapshot alone was loaded into the machine: the switches are forgotten, since what
    /// their bytes hold now is the snapshot's; where the session had got isn't known; and the
    /// game, this one or the next to start, is marked loaded.
    pub fn loaded_snapshot(&mut self) {
        self.switches.clear();
        self.record.loaded = true;
        self.next.loaded = true;
        self.progress.last_known = None;
        self.progress.in_game = false;
        self.holding = false;
        self.state = None;
        self.reading_anew();
    }

    /// Marks the game being played as not played from its start: loaded, or set up.
    pub const fn mark_loaded(&mut self) {
        self.record.loaded = true;
    }

    /// Whether a switch could go on now, without turning it on.
    ///
    /// # Errors
    /// As [`Session::switch`] would refuse it.
    pub fn can_switch_on(&self, id: &str) -> Result<(), SwitchError> {
        let cheat = self
            .definition
            .as_ref()
            .and_then(|d| d.cheats.iter().find(|c| c.id == id))
            .ok_or(SwitchError::Unknown)?;
        if self.switches.contains(id) {
            return Ok(());
        }
        cheat
            .patch
            .iter()
            .find(|p| !self.expected(p))
            .map_or(Ok(()), |patch| {
                Err(SwitchError::Mismatch {
                    addr: patch.addr.map_or(0, |a| a.0),
                })
            })
    }

    /// Whether a patch's `expect` bytes are where it would go, in RAM.
    fn expected(&self, patch: &Patch) -> bool {
        patch.bank.is_none()
            && patch.addr.is_some_and(|a| {
                a.0 >= 0x4000
                    && (a.0..)
                        .zip(&patch.expect.0)
                        .all(|(at, &byte)| self.machine.peek(at) == byte)
            })
    }

    /// Whether a training switch is on.
    #[must_use]
    pub fn switched_on(&self, id: &str) -> bool {
        self.switches.contains(id)
    }

    /// Sets the guidance level, for the record.
    pub fn set_level(&mut self, level: u8) {
        self.level = level;
        self.note_use();
    }

    /// What the game being played has used.
    #[must_use]
    pub const fn record(&self) -> &Record {
        &self.record
    }

    /// Whether the game is held on its loading screen.
    #[must_use]
    pub const fn holding(&self) -> bool {
        self.holding
    }

    /// Lets the game go on from its loading screen.
    pub const fn release(&mut self) {
        self.holding = false;
    }

    /// Every named value, as of now.
    #[must_use]
    pub fn read(&self) -> Reading {
        let Some(definition) = &self.definition else {
            return Reading::new();
        };
        let mut reading = Reading::new();
        // Tables counted by a value read after the value: plain values first.
        let (tables, plain): (Vec<_>, Vec<_>) = definition
            .values
            .iter()
            .partition(|(_, v)| v.kind == sidekick_def::ValueType::Table);
        for (name, value) in plain.into_iter().chain(tables) {
            let number = |n: &str| reading.get(n).and_then(Read::number);
            if let Some(read) = value::read(&self.machine, value, number) {
                reading.insert(name.clone(), read);
            }
        }
        reading
    }

    /// The values as read in the last frame.
    #[must_use]
    pub const fn reading(&self) -> &Reading {
        &self.reading
    }

    /// The game's state in the last frame: the first of its states that matched, `None` when
    /// none did.
    #[must_use]
    pub fn state(&self) -> Option<&str> {
        self.state.as_deref()
    }

    /// The first state that matches the last frame.
    fn decide(&self) -> Option<String> {
        let definition = self.definition.as_ref()?;
        definition
            .states
            .iter()
            .find(|s| self.matches(s))
            .map(|s| s.name.clone())
    }

    /// Whether a state matches the last frame. One that tests nothing yet (a draft's) never
    /// does.
    fn matches(&self, state: &State) -> bool {
        if state.pc.is_none() && state.when.is_none() {
            return false;
        }
        let pc = state
            .pc
            .as_ref()
            .is_none_or(|pcs| pcs.iter().any(|a| self.machine.passed().contains(&a.0)));
        pc && state.when.as_ref().is_none_or(|t| holds(t, &self.reading))
    }

    /// Writes a number to a named value, now: the one way the program changes the game.
    ///
    /// # Errors
    /// If there's no such value, it isn't writable, it isn't a number, the number doesn't fit,
    /// or it isn't in RAM the processor sees.
    pub fn write(&mut self, name: &str, number: i64) -> Result<(), WriteError> {
        let value = self
            .definition
            .as_ref()
            .and_then(|d| d.values.get(name))
            .ok_or(WriteError::Unknown)?;
        let (addr, bytes) = value::encode(value, number)?;
        for (at, byte) in (addr..).zip(bytes) {
            self.machine.write(at, byte);
        }
        Ok(())
    }

    /// Writes a patch's bytes over the game's, if its `expect` bytes are there. Returns
    /// whether it was applied. A patch reaching into the ROM or past the top of memory, or into
    /// a 128K bank (which comes with 128K play), is never applied.
    pub fn patch(&mut self, patch: &Patch) -> bool {
        self.swap(patch, &patch.expect.0, &patch.bytes.0)
    }

    /// Puts the game's bytes back where a patch was applied, if the patch's bytes are still
    /// there. Returns whether it was undone.
    pub fn unpatch(&mut self, patch: &Patch) -> bool {
        self.swap(patch, &patch.bytes.0, &patch.expect.0)
    }

    /// Writes `to` at a patch's address if `from` is there.
    fn swap(&mut self, patch: &Patch, from: &[u8], to: &[u8]) -> bool {
        let Some(addr) = patch.addr.map(|a| a.0) else {
            return false;
        };
        let fits = u32::from(addr) + u32::try_from(to.len()).unwrap_or(u32::MAX) <= 0x1_0000;
        let there = (addr..)
            .zip(from)
            .all(|(at, &byte)| self.machine.peek(at) == byte);
        if patch.bank.is_some() || addr < 0x4000 || !fits || from.len() != to.len() || !there {
            return false;
        }
        for (at, &byte) in (addr..).zip(to) {
            self.machine.write(at, byte);
        }
        true
    }

    /// The machine.
    #[must_use]
    pub const fn machine(&self) -> &Machine {
        &self.machine
    }

    /// The machine, to press keys on and the like.
    pub const fn machine_mut(&mut self) -> &mut Machine {
        &mut self.machine
    }

    /// The game's definition, if it has one.
    #[must_use]
    pub const fn definition(&self) -> Option<&Definition> {
        self.definition.as_ref()
    }
}

/// A value read, as a script sees it.
fn fact(read: &Read) -> Fact {
    match read {
        Read::Number(n) => Fact::Number(*n),
        Read::Flag(b) => Fact::Flag(*b),
        Read::Bits(bits) => Fact::List(bits.iter().map(|&b| Fact::Flag(b)).collect()),
        Read::Bytes(bytes) => {
            Fact::List(bytes.iter().map(|&b| Fact::Number(i64::from(b))).collect())
        }
        Read::Text(text) => Fact::Text(text.clone()),
        Read::Table(records) => Fact::List(
            records
                .iter()
                .map(|r| Fact::Map(r.iter().map(|(k, v)| (k.clone(), fact(v))).collect()))
                .collect(),
        ),
    }
}

/// Whether a test holds for the values read. A test of a value that isn't a number, or wasn't
/// read, fails.
fn holds(test: &Test, reading: &Reading) -> bool {
    let all = test.all.iter().all(|t| holds(t, reading));
    let any = test.any.is_empty() || test.any.iter().any(|t| holds(t, reading));
    let own = test.value.as_ref().is_none_or(|name| {
        reading.get(name).and_then(Read::number).is_some_and(|n| {
            test.equals.is_none_or(|e| n == e)
                && test.not.is_none_or(|e| n != e)
                && (test.one_of.is_empty() || test.one_of.contains(&n))
                && test.at_least.is_none_or(|e| n >= e)
                && test.at_most.is_none_or(|e| n <= e)
                && test
                    .bits
                    .is_none_or(|b| u64::try_from(n).is_ok_and(|n| n & b == b))
        })
    });
    all && any && own
}
