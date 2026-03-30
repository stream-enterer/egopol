use crate::emCore::emSignal::SignalId;

/// An observable value that tracks whether it changed on set.
///
/// `set()` returns `true` when the value actually changed, allowing the caller
/// to fire the associated signal via the scheduler.
pub struct WatchedVar<T: PartialEq> {
    value: T,
    signal_id: SignalId,
}

impl<T: PartialEq> WatchedVar<T> {
    pub fn new(value: T, signal_id: SignalId) -> Self {
        Self { value, signal_id }
    }

    pub fn GetRec(&self) -> &T {
        &self.value
    }

    /// Replace the value. Returns `true` if it actually changed.
    pub fn Set(&mut self, new_value: T) -> bool {
        if self.value == new_value {
            return false;
        }
        self.value = new_value;
        true
    }

    pub fn signal_id(&self) -> SignalId {
        self.signal_id
    }
}
