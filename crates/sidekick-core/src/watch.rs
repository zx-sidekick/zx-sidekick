//! What the machine watches for as it runs: the sync address, the addresses whose running a
//! definition's states ask about, and the tape's blocks going in through the ROM's loader.

use rustzx_core::host::DebugInterface;
use rustzx_z80::Breakpoints;

/// Where the 48K ROM's loader (LD-BYTES) hands a block to the core's fast loader: reached once
/// for each block the tape gives up.
const LOADER_TRAP: u16 = 0x056B;

/// Asked by the core about every instruction, through its `DebugInterface`.
#[derive(Clone)]
pub struct Watch {
    sync: Option<u16>,
    /// Whether the sync address has stopped the machine in this frame.
    synced: bool,
    watched: Box<Breakpoints>,
    passed: Vec<u16>,
    /// The tape's blocks, and how many the ROM's loader has taken.
    blocks: usize,
    loads: usize,
    /// Whether to stop once the tape has loaded, and whether that's why it stopped last.
    stop_when_loaded: bool,
    loaded: bool,
}

impl Watch {
    pub fn new() -> Self {
        Self {
            sync: None,
            synced: false,
            watched: Box::new(Breakpoints::new()),
            passed: Vec::new(),
            blocks: 0,
            loads: 0,
            stop_when_loaded: false,
            loaded: false,
        }
    }

    /// A tape of `blocks` blocks has gone in: none of them loaded yet.
    pub const fn new_tape(&mut self, blocks: usize) {
        self.blocks = blocks;
        self.loads = 0;
    }

    pub const fn set_stop_when_loaded(&mut self, stop: bool) {
        self.stop_when_loaded = stop;
    }

    /// Whether the tape's every block has gone in through the ROM's loader.
    pub const fn tape_loaded(&self) -> bool {
        self.blocks > 0 && self.loads >= self.blocks
    }

    /// Whether the last stop was the tape having loaded; asking clears it.
    pub const fn take_loaded(&mut self) -> bool {
        std::mem::replace(&mut self.loaded, false)
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
        // The instruction after the trap is the first after the block has loaded.
        if self.stop_when_loaded
            && self.blocks > 0
            && self.loads >= self.blocks
            && addr != LOADER_TRAP
        {
            self.stop_when_loaded = false;
            self.loaded = true;
            return true;
        }
        if addr == LOADER_TRAP {
            self.loads += 1;
        }
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
