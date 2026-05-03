//! Regression test for notice-dispatch PanelCtx reach loss.
//!
//! Spec: docs/superpowers/specs/2026-05-02-notice-dispatch-reach-loss-design.md
//! Investigation: docs/debug/investigations/notice-dispatch-reach-loss.md
//!
//! Asserts that the per-callback `PanelCtx` built inside
//! `emView::handle_notice_one` carries full scheduler reach
//! (`as_sched_ctx().is_some()`) for all four behavior dispatch sites.

use std::cell::Cell;
use std::rc::Rc;

use emcore::emEngineCtx::PanelCtx;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelTree::PanelTree;
use emcore::emView::emView;

#[derive(Default)]
struct ReachLog {
    notice: Cell<bool>,
    auto_expand: Cell<bool>,
    auto_shrink: Cell<bool>,
    layout_children: Cell<bool>,
}

struct ReachProbe(Rc<ReachLog>);

impl PanelBehavior for ReachProbe {
    fn notice(&mut self, _flags: NoticeFlags, _state: &PanelState, ctx: &mut PanelCtx) {
        self.0.notice.set(ctx.as_sched_ctx().is_some());
    }
    fn AutoExpand(&mut self, ctx: &mut PanelCtx) {
        self.0.auto_expand.set(ctx.as_sched_ctx().is_some());
    }
    fn AutoShrink(&mut self, ctx: &mut PanelCtx) {
        self.0.auto_shrink.set(ctx.as_sched_ctx().is_some());
    }
    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        self.0.layout_children.set(ctx.as_sched_ctx().is_some());
    }
}

/// Drive `notice`, `AutoExpand`, and `LayoutChildren` dispatch paths.
///
/// Flow (two HandleNotice calls to match C++ phase ordering):
///
/// Call 1: queue SOUGHT_NAME_CHANGED → Phase 2 fires `notice()`, sets
///   ae_decision_invalid=true (seek target set), re-adds root to ring.
///
/// Call 2: root re-enters ring with ae_decision_invalid; Phase 3 sees
///   `should_expand && !ae_expanded` → fires `AutoExpand()`.
///   Then Phase 4 sees children_layout_invalid → fires `LayoutChildren()`.
///
/// All three dispatch sites build PanelCtx via `with_scheduler` today, so
/// `as_sched_ctx()` returns None at each. Task 2 switches to
/// `with_sched_reach`, turning this test green.
#[test]
fn notice_dispatch_sites_carry_full_reach_notice_ae_layout() {
    use std::cell::RefCell;

    let log = Rc::new(ReachLog::default());

    let mut tree = PanelTree::new();
    let root = tree.create_root_deferred_view("root");
    // LayoutChildren requires at least one child.
    let _child = tree.create_child(root, "child", None);
    tree.set_behavior(root, ReachProbe(log.clone()));

    let mut view = emView::new(emcore::emContext::emContext::NewRoot(), root, 800.0, 600.0);
    let mut sched = emcore::emScheduler::EngineScheduler::new();
    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let fw_cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emEngineCtx::FrameworkDeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    // Make root the seek target so Phase 3 picks AutoExpand.
    tree.set_seek_pos_pub(root, "");
    // Mark children_layout_invalid for Phase 4 — will survive the Phase 2
    // return because Phase 2 only fires notice() and re-adds to ring.
    tree.set_children_layout_invalid_pub(root, true);

    // Call 1: Phase 2 path — queue notice → fires notice(), re-adds to ring.
    tree.queue_notice(root, NoticeFlags::SOUGHT_NAME_CHANGED, None);
    view.HandleNotice(
        &mut tree,
        &mut sched,
        Some(&root_ctx),
        None,
        &mut fw_actions,
        &fw_cb,
        &pa,
    );

    // Call 2: Phase 3+4 path — ae_decision_invalid set by Phase 2; fires
    // AutoExpand() then LayoutChildren().
    view.HandleNotice(
        &mut tree,
        &mut sched,
        Some(&root_ctx),
        None,
        &mut fw_actions,
        &fw_cb,
        &pa,
    );

    assert!(
        log.notice.get(),
        "notice dispatch must carry full scheduler reach"
    );
    assert!(
        log.auto_expand.get(),
        "AutoExpand dispatch must carry full scheduler reach"
    );
    assert!(
        log.layout_children.get(),
        "LayoutChildren dispatch must carry full scheduler reach"
    );
}

/// Drive the Phase-1 `AutoShrink` dispatch path in isolation.
///
/// Phase-1 path: set ae_invalid=true + ae_expanded=true, flip
/// `has_pending_notices` (so the safety-net scan enrolls the panel) but
/// do NOT queue a notice. This isolates Phase 1: `ae_invalid` is checked
/// before `pending_notices` in `handle_notice_one`, so Phase 1 fires and
/// returns before Phase 2 is reached. A queued notice would create a
/// Phase-2 path that could fire `AutoShrink` via Phase 3 anyway, making
/// the assertion vacuous.
#[test]
fn notice_dispatch_sites_carry_full_reach_autoshrink_phase1() {
    use std::cell::RefCell;

    let log = Rc::new(ReachLog::default());

    let mut tree = PanelTree::new();
    let root = tree.create_root_deferred_view("root");
    tree.set_behavior(root, ReachProbe(log.clone()));

    let mut view = emView::new(emcore::emContext::emContext::NewRoot(), root, 800.0, 600.0);
    let mut sched = emcore::emScheduler::EngineScheduler::new();
    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let fw_cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emEngineCtx::FrameworkDeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    // Phase-1 AutoShrink path: ae_invalid=true + ae_expanded=true.
    // Do NOT queue a notice here — that would also set up a Phase-2 path,
    // making the assertion pass even if Phase 1 never ran (Phase 2 would
    // eventually fire AutoShrink via the ae_decision_invalid→Phase-3 chain).
    // Instead, flip has_pending_notices directly so the safety-net scan
    // enrolls the panel, then handle_notice_one hits Phase 1 exclusively
    // (ae_invalid is checked before pending_notices, and there is no notice
    // queue entry to hand control to Phase 2).
    tree.set_ae_invalid_pub(root, true);
    tree.set_ae_expanded_pub(root, true);
    tree.force_pending_notices_flag_pub();

    view.HandleNotice(
        &mut tree,
        &mut sched,
        Some(&root_ctx),
        None,
        &mut fw_actions,
        &fw_cb,
        &pa,
    );

    assert!(
        log.auto_shrink.get(),
        "AutoShrink (Phase-1 path) dispatch must carry full scheduler reach"
    );
}

/// Drive the Phase-3 `AutoShrink` dispatch path.
///
/// Phase-3 path: set ae_decision_invalid=true + ae_expanded=true + no seek
/// target → Phase 3 sees `!should_expand && ae_expanded` → fires
/// `AutoShrink()`.
#[test]
fn notice_dispatch_sites_carry_full_reach_autoshrink_phase3() {
    use std::cell::RefCell;

    let log = Rc::new(ReachLog::default());

    let mut tree = PanelTree::new();
    let root = tree.create_root_deferred_view("root");
    tree.set_behavior(root, ReachProbe(log.clone()));

    let mut view = emView::new(emcore::emContext::emContext::NewRoot(), root, 800.0, 600.0);
    let mut sched = emcore::emScheduler::EngineScheduler::new();
    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let fw_cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emEngineCtx::FrameworkDeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    // No seek target → should_expand = false. ae_expanded=true → Phase 3
    // fires AutoShrink.
    tree.set_ae_decision_invalid_pub(root, true);
    tree.set_ae_expanded_pub(root, true);
    tree.force_pending_notices_flag_pub();

    view.HandleNotice(
        &mut tree,
        &mut sched,
        Some(&root_ctx),
        None,
        &mut fw_actions,
        &fw_cb,
        &pa,
    );

    assert!(
        log.auto_shrink.get(),
        "AutoShrink (Phase-3 path) dispatch must carry full scheduler reach"
    );
}
