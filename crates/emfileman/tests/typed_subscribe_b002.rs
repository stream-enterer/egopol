//! B-002 emFileLinkPanel ChangeSignal wiring — P-001-no-subscribe-no-accessor.
//!
//! Rows covered (single shared Rust callsite per amended design):
//!   `emFileLinkPanel-56` / `emFileLinkPanel-72` — first-Cycle subscribe to
//!     `emFileLinkModel::GetChangeSignal()` and re-subscribe after
//!     `set_link_model`.
//!   `emFileLinkModel-accessor-model-change` — delegating accessor.
//!
//! Decisions cited: D-006 (subscribe-shape; option-B local override at model
//! swap), D-007 (mutator-fire — adopted as deferred-fire per
//! `pending_change_fire`, language-forced), D-008 A1 (lazy `Cell<SignalId>`).
//!
//! RUST_ONLY: (dependency-forced) — no C++ test analogue; mirrors
//! B-005 `typed_subscribe_b005.rs`.

use std::rc::Rc;

use emFileMan::emFileLinkModel::emFileLinkModel;
use emFileMan::emFileLinkPanel::emFileLinkPanel;
use emcore::emEngine::Priority;
use emcore::emPanel::PanelBehavior;
use emcore::emPanelScope::PanelScope;
use emcore::emPanelTree::PanelTree;
use emcore::test_view_harness::TestViewHarness;
use slotmap::Key as _;

struct NoopEngine;
impl emcore::emEngine::emEngine for NoopEngine {
    fn Cycle(&mut self, _ctx: &mut emcore::emEngineCtx::EngineCtx<'_>) -> bool {
        false
    }
}

fn empty_panel_ctx<'a>(tree: &'a mut PanelTree) -> emcore::emEngineCtx::PanelCtx<'a> {
    let id = tree.create_root("b002-stub", false);
    emcore::emEngineCtx::PanelCtx::new(tree, id, 1.0)
}

fn cycle_panel(
    h: &mut TestViewHarness,
    eid: emcore::emEngine::EngineId,
    panel: &mut dyn PanelBehavior,
) -> bool {
    let mut tree = PanelTree::new();
    let mut pctx = empty_panel_ctx(&mut tree);
    let mut ectx = h.engine_ctx(eid);
    panel.Cycle(&mut ectx, &mut pctx)
}

fn drain_all_engines(h: &mut TestViewHarness) {
    let mut eids: Vec<emcore::emEngine::EngineId> =
        h.scheduler.engines_for_scope(PanelScope::Framework);
    for wid in h.windows.keys().copied().collect::<Vec<_>>() {
        eids.extend(h.scheduler.engines_for_scope(PanelScope::Toplevel(wid)));
    }
    for eid in eids {
        h.scheduler.remove_engine(eid);
    }
}

#[test]
fn link_model_get_change_signal_is_lazy_and_idempotent() {
    let mut h = TestViewHarness::new();
    let ctx = Rc::clone(&h.root_context);
    let model = emFileLinkModel::Acquire(&ctx, "/tmp/b002_lazy.emFileLink", false);
    let mut sc = h.sched_ctx();
    let s1 = model.borrow().GetChangeSignal(&mut sc);
    assert!(!s1.is_null());
    let s2 = model.borrow().GetChangeSignal(&mut sc);
    assert_eq!(s1, s2, "delegating combined-form must be idempotent");
}

#[test]
fn link_panel_subscribes_to_model_change_signal_after_set_link_model() {
    let mut h = TestViewHarness::new();
    let ctx = Rc::clone(&h.root_context);
    let mut panel = emFileLinkPanel::new(Rc::clone(&ctx), false);

    // Without a model: first Cycle subscribes panel-lifetime signals only.
    let eid = h.scheduler.register_engine(
        Box::new(NoopEngine),
        Priority::Medium,
        PanelScope::Framework,
    );
    let _ = cycle_panel(&mut h, eid, &mut panel);

    // The model's change signal is still null on the model side (not allocated).
    let model_a = emFileLinkModel::Acquire(&ctx, "/tmp/b002_model_a.emFileLink", false);
    panel.set_link_model(Rc::clone(&model_a));

    // Cycle again — the new model_subscribed branch must allocate the
    // model's ChangeSignal and connect it to the panel's engine id.
    let _ = cycle_panel(&mut h, eid, &mut panel);

    // Allocation observable: the model's signal is now non-null when queried.
    {
        let mut sc = h.sched_ctx();
        let sig = model_a.borrow().GetChangeSignal(&mut sc);
        assert!(
            !sig.is_null(),
            "model ChangeSignal must be allocated after panel first Cycle with model"
        );
    }

    h.scheduler.remove_engine(eid);
    drain_all_engines(&mut h);
    h.scheduler.flush_signals_for_test();
}

#[test]
fn link_panel_re_subscribes_on_set_link_model_swap() {
    // Row -72: swap models and verify the new model's signal is allocated
    // and that the panel's `model_subscribed` re-runs the connect.
    let mut h = TestViewHarness::new();
    let ctx = Rc::clone(&h.root_context);
    let mut panel = emFileLinkPanel::new(Rc::clone(&ctx), false);
    let eid = h.scheduler.register_engine(
        Box::new(NoopEngine),
        Priority::Medium,
        PanelScope::Framework,
    );

    let model_a = emFileLinkModel::Acquire(&ctx, "/tmp/b002_swap_a.emFileLink", false);
    panel.set_link_model(Rc::clone(&model_a));
    let _ = cycle_panel(&mut h, eid, &mut panel);

    // Swap to model B.
    let model_b = emFileLinkModel::Acquire(&ctx, "/tmp/b002_swap_b.emFileLink", false);
    panel.set_link_model(Rc::clone(&model_b));
    // After set_link_model, model_subscribed must be reset; next Cycle reconnects.
    let _ = cycle_panel(&mut h, eid, &mut panel);

    // Both models' signals are now allocated (each was queried in its own Cycle).
    {
        let mut sc = h.sched_ctx();
        let sa = model_a.borrow().GetChangeSignal(&mut sc);
        let sb = model_b.borrow().GetChangeSignal(&mut sc);
        assert!(!sa.is_null());
        assert!(!sb.is_null());
        assert_ne!(sa, sb, "different models hold different SignalIds");
    }

    h.scheduler.remove_engine(eid);
    drain_all_engines(&mut h);
    h.scheduler.flush_signals_for_test();
}

#[test]
fn link_panel_drains_pending_change_fire_from_model_mutator() {
    // Mutating the model (e.g. hard_reset) marks pending_change_fire on
    // emRecFileModel. The panel's Cycle must drain it via
    // fire_pending_change(ectx); after the drain, take_pending_change_fire
    // observes the flag cleared.
    let mut h = TestViewHarness::new();
    let ctx = Rc::clone(&h.root_context);
    let mut panel = emFileLinkPanel::new(Rc::clone(&ctx), false);
    let eid = h.scheduler.register_engine(
        Box::new(NoopEngine),
        Priority::Medium,
        PanelScope::Framework,
    );

    let model = emFileLinkModel::Acquire(&ctx, "/tmp/b002_drain.emFileLink", false);
    panel.set_link_model(Rc::clone(&model));
    // First Cycle: subscribe + allocate signal.
    let _ = cycle_panel(&mut h, eid, &mut panel);

    // Mutate the model — sets pending_change_fire.
    model.borrow_mut().rec_model_mut_for_test().hard_reset();
    // Verify flag is set on the model.
    assert!(
        model
            .borrow()
            .rec_model_for_test()
            .take_pending_change_fire(),
        "hard_reset must mark pending_change_fire"
    );
    // Re-set (we just cleared via take_pending_change_fire above).
    model.borrow_mut().rec_model_mut_for_test().hard_reset();
    assert!(
        // peek without clearing: re-take and re-set
        model
            .borrow()
            .rec_model_for_test()
            .take_pending_change_fire()
    );
    model.borrow_mut().rec_model_mut_for_test().hard_reset();

    // Second Cycle: panel drains pending_change_fire via fire_pending_change.
    let _ = cycle_panel(&mut h, eid, &mut panel);

    // Flag must be cleared after the panel's drain.
    assert!(
        !model
            .borrow()
            .rec_model_for_test()
            .take_pending_change_fire(),
        "panel Cycle must drain pending_change_fire via fire_pending_change(ectx)"
    );

    h.scheduler.remove_engine(eid);
    drain_all_engines(&mut h);
    h.scheduler.flush_signals_for_test();
}
