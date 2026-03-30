use slotmap::new_key_type;

use crate::emCore::emEngine::EngineId;

new_key_type! {
    /// Handle to a signal in the scheduler.
    pub struct SignalId;
}

/// A reference-counted connection between a signal and an engine.
#[derive(Debug)]
pub(crate) struct SignalConnection {
    pub engine: EngineId,
    pub ref_count: u32,
}

/// Internal state for a signal.
pub(crate) struct SignalData {
    pub pending: bool,
    pub connected_engines: Vec<SignalConnection>,
    /// Clock value when this signal was last processed. Used by `is_signaled`.
    pub clock: u64,
}

impl SignalData {
    pub fn new() -> Self {
        Self {
            pending: false,
            connected_engines: Vec::new(),
            clock: 0,
        }
    }
}
