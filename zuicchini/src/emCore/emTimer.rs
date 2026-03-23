use std::time::{Duration, Instant};

use slotmap::{new_key_type, SlotMap};

use crate::emCore::emSignal::SignalId;

new_key_type! {
    /// Handle to a timer managed by `TimerCentral`.
    pub struct TimerId;
}

struct TimerEntry {
    signal_id: SignalId,
    interval_ms: u64,
    periodic: bool,
    next_fire: Instant,
    active: bool,
}

/// An engine that manages timers. Each `cycle()` checks wall clock time
/// and fires signals for elapsed timers.
pub(crate) struct TimerCentral {
    timers: SlotMap<TimerId, TimerEntry>,
}

impl TimerCentral {
    pub fn new() -> Self {
        Self {
            timers: SlotMap::with_key(),
        }
    }

    /// Create a new timer in stopped state. Call `start_timer` to begin.
    pub fn create_timer(&mut self, signal_id: SignalId) -> TimerId {
        self.timers.insert(TimerEntry {
            signal_id,
            interval_ms: 0,
            periodic: false,
            next_fire: Instant::now(),
            active: false,
        })
    }

    /// Start (or restart) a timer with the given interval and periodicity.
    pub fn start_timer(&mut self, id: TimerId, interval_ms: u64, periodic: bool) {
        if let Some(entry) = self.timers.get_mut(id) {
            // C++ clamps periodic interval to at least 1ms to prevent spin-loop,
            // but initial fire uses raw interval (SigTime = now + millisecs).
            let period_ms = if periodic { interval_ms.max(1) } else { 0 };
            entry.interval_ms = period_ms;
            entry.periodic = periodic;
            entry.next_fire = Instant::now() + Duration::from_millis(interval_ms);
            entry.active = true;
        }
    }

    /// Restart an existing timer in-place with new interval and periodicity.
    pub fn restart_timer(&mut self, id: TimerId, interval_ms: u64, periodic: bool) {
        self.start_timer(id, interval_ms, periodic);
    }

    /// Cancel a timer. Returns the signal ID so the caller can abort
    /// any already-queued signal.
    pub fn cancel_timer(&mut self, id: TimerId, abort_signal: bool) -> Option<SignalId> {
        if let Some(entry) = self.timers.get_mut(id) {
            entry.active = false;
            if abort_signal {
                Some(entry.signal_id)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Remove a cancelled timer, freeing its slot.
    pub fn remove_timer(&mut self, id: TimerId) {
        self.timers.remove(id);
    }

    /// Check if a timer is still active (running).
    pub fn is_running(&self, id: TimerId) -> bool {
        self.timers.get(id).is_some_and(|t| t.active)
    }

    /// Run timer checks and collect signals to fire. Called directly
    /// by the scheduler (not as a registered engine) at VERY_HIGH priority
    /// equivalent position in the time slice.
    pub fn check_and_collect(&mut self) -> Vec<SignalId> {
        let now = Instant::now();
        // Collect expired timers with their fire time for chronological sorting.
        // C++ processes timers from a sorted linked list (ascending SigTime),
        // so the earliest-scheduled timer fires first.
        let mut expired: Vec<(Instant, SignalId)> = Vec::new();

        for (_, timer) in &mut self.timers {
            if !timer.active {
                continue;
            }
            if now >= timer.next_fire {
                expired.push((timer.next_fire, timer.signal_id));
                if timer.periodic {
                    timer.next_fire += Duration::from_millis(timer.interval_ms);
                    // Clamp to current time to prevent burst catch-up
                    // (matches C++: `if (st<ct) st=ct;`)
                    if timer.next_fire < now {
                        timer.next_fire = now;
                    }
                } else {
                    timer.active = false;
                }
            }
        }

        // Sort by fire time ascending to match C++ sorted-list iteration order
        expired.sort_by_key(|&(fire_time, _)| fire_time);

        // Purge inactive timers to prevent unbounded growth
        self.timers.retain(|_, t| t.active);

        expired.into_iter().map(|(_, sig)| sig).collect()
    }
}

// TimerCentral is no longer used as an emEngine trait object.
// It is called directly by the scheduler. This avoids the dead
// timer_engine_id pattern.

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn timer_created_inactive() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        assert!(!tc.is_running(id));

        let fired = tc.check_and_collect();
        assert!(fired.is_empty());
    }

    #[test]
    fn timer_fires_after_start() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        tc.start_timer(id, 0, false); // 0ms = fires immediately

        let fired = tc.check_and_collect();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], sig);
        assert!(tc.timers.is_empty()); // one-shot purged after fire
    }

    #[test]
    fn periodic_stays_active() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        // interval_ms=0: initial fire is immediate (raw interval),
        // but periodic refire interval is clamped to 1ms.
        tc.start_timer(id, 0, true);

        let fired = tc.check_and_collect();
        assert_eq!(fired.len(), 1);
        assert!(!tc.timers.is_empty()); // periodic stays
    }

    #[test]
    fn periodic_zero_interval_initial_fires_immediately() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        // C++: Start(0, true) sets Period=1 but SigTime=now+0 (fires immediately)
        tc.start_timer(id, 0, true);

        let fired = tc.check_and_collect();
        assert_eq!(
            fired.len(),
            1,
            "periodic timer with 0ms should fire immediately on first check"
        );
    }

    #[test]
    fn cancel_timer_with_abort() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        tc.start_timer(id, 0, false);
        let abort_sig = tc.cancel_timer(id, true);
        assert_eq!(abort_sig, Some(sig));

        let fired = tc.check_and_collect();
        assert!(fired.is_empty());
    }

    #[test]
    fn cancel_timer_without_abort() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        tc.start_timer(id, 0, false);
        let abort_sig = tc.cancel_timer(id, false);
        assert!(abort_sig.is_none());
    }

    #[test]
    fn restart_timer() {
        let mut signals: SlotMap<SignalId, ()> = SlotMap::with_key();
        let sig = signals.insert(());

        let mut tc = TimerCentral::new();
        let id = tc.create_timer(sig);
        tc.start_timer(id, 1000, false); // 1s, won't fire soon
        assert!(tc.is_running(id));

        // Restart with 0ms — should fire immediately
        tc.restart_timer(id, 0, false);
        let fired = tc.check_and_collect();
        assert_eq!(fired.len(), 1);
    }
}
