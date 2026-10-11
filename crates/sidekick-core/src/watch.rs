//! What the machine watches for as it runs: the sync address, and the addresses whose running a
//! definition's states ask about.

use rustzx_core::host::DebugInterface;
use rustzx_z80::Breakpoints;

/// Asked by the core about every instruction, through its `DebugInterface`.
#[derive(Clone)]
pub struct Watch {
    sync: Option<u16>,
    /// Whether the sync address has stopped the machine in this frame.
    synced: bool,
    watched: Box<Breakpoints>,
    passed: Vec<u16>,
}

impl Watch {
    pub fn new() -> Self {
        Self {
            sync: None,
            synced: false,
            watched: Box::new(Breakpoints::new()),
            passed: Vec::new(),
        }
    }

    pub const fn set_sync(&mut self, sync: Option<u16>) {
        self.sync = sync;
    }

    pub fn watch(&mut self, addrs: &[u16]) {
        self.watched.clear();
        for &addr in addrs {
            self.watched.insert(addr);
        }
        self.passed.retain(|&addr| self.watched.contains(addr));
    }

    pub fn passed(&self) -> &[u16] {
        &self.passed
    }

    /// Starts a new frame: the sync address may stop the machine again, and nothing has run yet.
    pub fn new_frame(&mut self) {
        self.synced = false;
        self.passed.clear();
    }
}

impl DebugInterface for Watch {
    fn check_pc_breakpoint(&mut self, addr: u16) -> bool {
        if self.watched.contains(addr) && !self.passed.contains(&addr) {
            self.passed.push(addr);
        }
        if self.sync == Some(addr) && !self.synced {
            self.synced = true;
            return true;
        }
        false
    }
}
