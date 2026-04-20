//! Phase 5 W3 popup cancellation test: if a popup is torn down before its
//! OS surface is materialized, the deferred materialization aborts cleanly.
//!
//! DEFERRED (Task 8): Phase-2 port-ownership-rewrite changed popup
//! ownership so the original `Rc::strong_count(&win_rc) == 1` cancellation
//! check no longer applies. Popups live in `emView::PopupWindow` (plain
//! `Option<emWindow>`); cancellation is now "no Pending popup exists in
//! any view at drain time", which `materialize_pending_popup` detects
//! naturally. The assertion surface of this test (Weak ref counts,
//! App::windows size) no longer maps. Task 8 will restore a meaningful
//! assertion.

#[test]
fn popup_materialization_aborts_when_popup_dropped_before_drain() {
    eprintln!(
        "popup_materialization_aborts_when_popup_dropped_before_drain: \
         stubbed for Task-8; Phase-2 port-ownership-rewrite changed popup \
         ownership model."
    );
}
