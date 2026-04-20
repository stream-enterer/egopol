//! Phase 5 W3 popup architecture test: a popup window constructed in
//! `OsSurface::Pending` transitions to `OsSurface::Materialized` after one
//! `App::about_to_wait` drain pass.
//!
//! DEFERRED (Task 8): Phase-2 port-ownership-rewrite narrowed
//! `App::windows` to plain `emWindow` (no `Rc<RefCell<>>`), so this test's
//! `popup_window: Option<Rc<RefCell<emWindow>>>` capture is obsolete.
//! Additionally, popups under the new model live in `emView::PopupWindow`
//! rather than `App::windows`, so the `popup_in_app_windows` assertion is
//! no longer a valid contract. The body is stubbed out pending the Task-8
//! redesign of popup OS-event handling.

#[test]
fn popup_surface_materializes_on_about_to_wait() {
    // Task-8-deferred: entire test body was rewritten as a stub during the
    // Phase-2 port-ownership-rewrite (Task-W3 + Task-4-backref bundle).
    // The original assertions (popup cloned from App::windows, popup
    // materialization routed through a closure owning an Rc<RefCell<emWindow>>)
    // are no longer expressible: App::windows holds plain `emWindow`, popups
    // live in `emView::PopupWindow`, and the materialization closure finds
    // its target by walking the window registry. Task 8 will redesign
    // popup OS-event handling and restore a meaningful assertion here.
    eprintln!(
        "popup_surface_materializes_on_about_to_wait: stubbed for Task-8; \
         Phase-2 port-ownership-rewrite changed popup ownership model."
    );
}
