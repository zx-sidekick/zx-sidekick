//! A game running: its machine and its definition, the named values read each frame, the
//! game's state, and writes to it by name.

use crate::{
    value::{self, Read, WriteError},
    Error, Machine, Stop,
};
use sidekick_def::{Amount, Cheat, Definition, Patch, State, Test};
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
    /// The last state that matched, through frames that matched none.
    last_known: Option<String>,
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
            last_known: None,
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
        let mut read = false;
        loop {
            match self.machine.run()? {
                Stop::Sync => {
                    self.take_reading();
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
        self.state = self.decide();
        self.track_games();
        Ok(())
    }

    /// Reads the values for this frame, and lets the switches that are on act on them: only in
    /// play when the definition names a `play` state (the last state known, so a frame that
    /// matches none doesn't skip), since outside it a game may use the same memory for other
    /// things.
    fn take_reading(&mut self) {
        let now = self.read();
        let previous = std::mem::replace(&mut self.reading, now);
        let Some(definition) = self.definition.take() else {
            return;
        };
        let has_play = definition.states.iter().any(|s| s.name == "play");
        if has_play && self.last_known.as_deref() != Some("play") {
            self.definition = Some(definition);
            return;
        }
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
        let after = self.last_known.as_deref();
        if state == "play" && matches!(after, None | Some("title" | "game_over")) {
            self.record = Record::default();
            self.note_use();
        }
        self.last_known = Some(state);
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
            self.note_use();
        } else {
            for patch in &cheat.patch {
                self.unpatch(patch);
            }
            self.switches.remove(id);
        }
        Ok(())
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
