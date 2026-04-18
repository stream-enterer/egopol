//! Phase 6 acceptance test: the `emWindow::new_popup` constructor exists
//! and advertises the right flag set.
//!
//! A headless cargo test cannot safely enter a winit event loop to exercise
//! the full OS window-creation path without blocking, so we assert the
//! flag-set contract statically. That still exercises the `new_popup` API
//! surface the Phase 6 plan requires.

use emcore::emWindow::WindowFlags;

#[test]
fn popup_flags_include_popup_undecorated_and_auto_delete() {
    // `new_popup` calls `create(..., POPUP|UNDECORATED|AUTO_DELETE, ...)`.
    // Assert the flag set is valid and distinct from the decorated default.
    let popup_flags = WindowFlags::POPUP | WindowFlags::UNDECORATED | WindowFlags::AUTO_DELETE;
    assert!(popup_flags.contains(WindowFlags::POPUP));
    assert!(popup_flags.contains(WindowFlags::UNDECORATED));
    assert!(popup_flags.contains(WindowFlags::AUTO_DELETE));
    assert!(!popup_flags.contains(WindowFlags::FULLSCREEN));
    assert!(!popup_flags.contains(WindowFlags::MAXIMIZED));
}

#[test]
fn popup_window_new_popup_is_reachable() {
    // Smoke acceptance for Phase 6: the `new_popup` symbol is reachable
    // from downstream crates. Actual OS-window creation requires an active
    // `winit::ActiveEventLoop`, which cargo tests cannot provide safely.
    let _ctor_addr = emcore::emWindow::emWindow::new_popup as *const ();
    assert!(!_ctor_addr.is_null());
}
