//! A game running: its machine and its definition, the named values read each frame, the
//! game's state, and writes to it by name.

use crate::{
    value::{self, Read, WriteError},
    Error, Machine, Stop,
};
use sidekick_def::{Definition, Patch, State, Test};
use std::collections::BTreeMap;

/// Every named value, as read at one moment.
pub type Reading = BTreeMap<String, Read>;

/// A game being played: the machine, and what its definition says about it.
#[derive(Clone)]
pub struct Session {
    /// Boxed: the machine is tens of kilobytes, too much to move about on a small stack
    /// (Windows gives its main thread 1 MB, less in a debug build's frames).
    machine: Box<Machine>,
    definition: Option<Definition>,
    reading: Reading,
    state: Option<String>,
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
        }
    }

    /// Runs a frame: the values are read when it reaches the sync address, or at its end if
    /// it doesn't, and the state is decided at its end.
    ///
    /// # Errors
    /// If the machine fails.
    pub fn run_frame(&mut self) -> Result<(), Error> {
        let mut read = false;
        while self.machine.run()? == Stop::Sync {
            self.reading = self.read();
            read = true;
        }
        if !read {
            self.reading = self.read();
        }
        self.state = self.decide();
        Ok(())
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
