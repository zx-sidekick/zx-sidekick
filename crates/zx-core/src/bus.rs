//! What the ULA charges the processor for using the bus.
//!
//! While the ULA is drawing it wants the bottom 16K of RAM for itself, and
//! holds the processor off rather than share, so what a memory access or a
//! port costs depends on which address it names and when it happens. The
//! machine's bus (`zx-spectrum`) charges every access through these.

use crate::timing::contention;

/// Whether an address is in the quarter of memory the ULA shares.
#[must_use]
pub const fn contended(addr: u16) -> bool {
    0x4000 <= addr && addr < 0x8000
}

/// Charges an I/O cycle, whose delays follow a different pattern from
/// memory's.
///
/// The address lines are on the bus for the whole four T-states, so a port in
/// the ULA's own range is contended when the cycle starts as well. A port
/// with bit 0 clear is the ULA's own, and it holds the processor for the
/// three T-states it takes to answer.
pub fn charge_io(t: &mut u32, port: u16) {
    let ula_range = (0x40..0x80).contains(&(port >> 8));
    let ula_port = port & 1 == 0;
    let tick = |t: &mut u32, contend: bool, len: u32| {
        if contend {
            *t += contention(*t);
        }
        *t += len;
    };
    match (ula_range, ula_port) {
        (true, true) => {
            tick(t, true, 1);
            tick(t, true, 3);
        }
        (true, false) => {
            for _ in 0..4 {
                tick(t, true, 1);
            }
        }
        (false, true) => {
            tick(t, false, 1);
            tick(t, true, 3);
        }
        (false, false) => tick(t, false, 4),
    }
}
