//! A button pressed: drawn pressed for a moment before what it does happens, so a click, Enter
//! or the pad's confirm is seen to have landed.

use std::time::{Duration, Instant};

/// How long a press is drawn before what it does happens.
pub(crate) const SHOWN: Duration = Duration::from_millis(120);

/// A press being shown: what was pressed, and when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pressed<T> {
    what: T,
    at: Instant,
}

impl<T: Copy> Pressed<T> {
    /// `what` pressed at `at`.
    pub(crate) const fn new(what: T, at: Instant) -> Self {
        Self { what, at }
    }

    /// What was pressed.
    pub(crate) const fn what(&self) -> T {
        self.what
    }
}

/// Starts a press of `what` at `now` in `slot`, unless one is showing there already. Returns
/// whether it started.
pub(crate) const fn start<T: Copy>(slot: &mut Option<Pressed<T>>, what: T, now: Instant) -> bool {
    if slot.is_some() {
        return false;
    }
    *slot = Some(Pressed::new(what, now));
    true
}

/// Takes the press out of `slot` once it has been shown long enough at `now`, for what it
/// does to happen.
pub(crate) fn take_due<T: Copy>(slot: &mut Option<Pressed<T>>, now: Instant) -> Option<T> {
    let pressed = (*slot)?;
    if now.saturating_duration_since(pressed.at) < SHOWN {
        return None;
    }
    *slot = None;
    Some(pressed.what)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_shows_for_120_ms_then_is_taken_once() {
        let t0 = Instant::now();
        let mut slot = Some(Pressed::new('s', t0));
        assert_eq!(take_due(&mut slot, t0), None);
        assert_eq!(
            take_due(&mut slot, t0 + Duration::from_millis(119)),
            None,
            "still showing"
        );
        assert_eq!(slot.map(|p| p.what()), Some('s'));
        assert_eq!(take_due(&mut slot, t0 + SHOWN), Some('s'));
        assert_eq!(slot, None);
        assert_eq!(take_due(&mut slot, t0 + SHOWN * 2), None, "taken once");
    }

    #[test]
    fn a_second_press_waits_for_the_first_to_be_done() {
        let t0 = Instant::now();
        let mut slot = None;
        assert!(start(&mut slot, 'a', t0));
        assert!(
            !start(&mut slot, 'b', t0 + SHOWN / 2),
            "the first still shows"
        );
        assert_eq!(take_due(&mut slot, t0 + SHOWN), Some('a'));
        assert!(start(&mut slot, 'b', t0 + SHOWN));
        assert_eq!(slot.map(|p| p.what()), Some('b'));
    }

    #[test]
    fn a_clock_read_before_the_press_doesn_t_take_it() {
        let t0 = Instant::now();
        let mut slot = Some(Pressed::new(1, t0 + SHOWN));
        assert_eq!(take_due(&mut slot, t0), None);
    }
}
