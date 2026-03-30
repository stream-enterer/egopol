//! Debug log toggle and macro.
//!
//! Port of C++ `emEnableDLog` / `emIsDLogEnabled` / `EM_DLOG`. A global
//! `AtomicBool` controls whether debug log output is enabled. The `dlog!`
//! macro checks the flag and outputs to stderr with a module prefix.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static DLOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Optional capture buffer for testing. When `Some`, dlog! appends here
/// instead of (in addition to) stderr.
static DLOG_CAPTURE: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// Check whether debug logging is enabled.
pub fn emIsDLogEnabled() -> bool {
    DLOG_ENABLED.load(Ordering::Relaxed)
}

/// Enable or disable debug logging.
pub fn emEnableDLog(enable: bool) {
    DLOG_ENABLED.store(enable, Ordering::Relaxed);
}

/// Start capturing dlog output into a buffer (for testing).
pub fn start_capture() {
    *DLOG_CAPTURE.lock().unwrap() = Some(Vec::new());
}

/// Stop capturing and return all captured lines.
pub fn stop_capture() -> Vec<String> {
    DLOG_CAPTURE
        .lock()
        .unwrap()
        .take()
        .unwrap_or_default()
}

/// Push a line to the capture buffer if active.
#[doc(hidden)]
pub fn _capture_line(line: &str) {
    if let Ok(mut guard) = DLOG_CAPTURE.lock() {
        if let Some(ref mut buf) = *guard {
            buf.push(line.to_string());
        }
    }
}

/// Debug log macro. Checks `is_dlog_enabled()` and outputs to stderr with
/// a module prefix derived from `module_path!()`.
///
/// Usage: `dlog!("message {}", value);`
#[macro_export]
macro_rules! dlog {
    ($($arg:tt)*) => {
        if $crate::emCore::emStd1::emIsDLogEnabled() {
            let path = module_path!();
            // Strip crate prefix for readability
            let short = path.strip_prefix("eaglemode_rs::").unwrap_or(path);
            let msg = format!("[{}] {}", short, format_args!($($arg)*));
            eprintln!("{}", msg);
            $crate::emCore::emStd1::_capture_line(&msg);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_dlog() {
        assert!(!emIsDLogEnabled());
        emEnableDLog(true);
        assert!(emIsDLogEnabled());
        emEnableDLog(false);
        assert!(!emIsDLogEnabled());
    }

    #[test]
    fn dlog_macro_fires_when_enabled() {
        emEnableDLog(true);
        // Should output to stderr without panicking
        dlog!("test message: {}", 42);
        emEnableDLog(false);
        // Should be a no-op when disabled
        dlog!("this should not appear");
    }

    #[test]
    fn dlog_capture_stderr_output() {
        emEnableDLog(true);
        start_capture();

        dlog!("captured line {}", 1);
        dlog!("captured line {}", 2);

        let lines = stop_capture();
        emEnableDLog(false);

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("captured line 1"));
        assert!(lines[1].contains("captured line 2"));
    }
}

/// Global flag: whether fatal errors should be displayed graphically.
static FATAL_ERROR_GRAPHICAL: AtomicBool = AtomicBool::new(false);

/// Enable or disable graphical display of fatal errors.
///
/// Matches C++ emSetFatalErrorGraphical. When enabled, a future fatal-error
/// handler could show a dialog instead of just logging to stderr.
/// Currently only stores the flag; no graphical dialog is implemented yet.
pub fn emSetFatalErrorGraphical(enable: bool) {
    FATAL_ERROR_GRAPHICAL.store(enable, Ordering::Relaxed);
}

/// Query whether fatal errors should be displayed graphically.
pub fn is_fatal_error_graphical() -> bool {
    FATAL_ERROR_GRAPHICAL.load(Ordering::Relaxed)
}
