//! Debug log toggle and macro.
//!
//! Port of C++ `emEnableDLog` / `emIsDLogEnabled` / `EM_DLOG`. A global
//! `AtomicBool` controls whether debug log output is enabled. The `dlog!`
//! macro checks the flag and outputs to stderr with a module prefix.

use std::sync::atomic::{AtomicBool, Ordering};

static DLOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check whether debug logging is enabled.
pub fn is_dlog_enabled() -> bool {
    DLOG_ENABLED.load(Ordering::Relaxed)
}

/// Enable or disable debug logging.
pub fn set_dlog_enabled(enable: bool) {
    DLOG_ENABLED.store(enable, Ordering::Relaxed);
}

/// Debug log macro. Checks `is_dlog_enabled()` and outputs to stderr with
/// a module prefix derived from `module_path!()`.
///
/// Usage: `dlog!("message {}", value);`
#[macro_export]
macro_rules! dlog {
    ($($arg:tt)*) => {
        if $crate::foundation::is_dlog_enabled() {
            let path = module_path!();
            // Strip crate prefix for readability
            let short = path.strip_prefix("zuicchini::").unwrap_or(path);
            eprintln!("[{}] {}", short, format_args!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_dlog() {
        assert!(!is_dlog_enabled());
        set_dlog_enabled(true);
        assert!(is_dlog_enabled());
        set_dlog_enabled(false);
        assert!(!is_dlog_enabled());
    }

    #[test]
    fn dlog_macro_fires_when_enabled() {
        set_dlog_enabled(true);
        // Should output to stderr without panicking
        dlog!("test message: {}", 42);
        set_dlog_enabled(false);
        // Should be a no-op when disabled
        dlog!("this should not appear");
    }
}
