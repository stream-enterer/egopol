// DIVERGED: C++ emPanel inherits from emEngine directly (emPanel.h:33 —
// `class emPanel : public emEngine`). In Rust, `PanelBehavior` trait
// objects are owned by `PanelTree::panels` and are `take`n during cycling
// so the tree can lend a `PanelCtx`; a `PanelBehavior` therefore cannot
// simultaneously live in the scheduler's `Box<dyn emEngine>` slot-map.
// This adapter is the minimum concession: one adapter engine per panel,
// registered with the scheduler, whose `Cycle` drives the panel's
// `PanelBehavior::Cycle` via the standard take/put path.
//
// Observable behavior matches C++ (panel cycling runs through the
// scheduler's normal engine loop, uses the panel's own view's
// `CurrentPixelTallness`).

use super::emEngine::emEngine;
use super::emEngineCtx::EngineCtx;
use super::emEngineCtx::PanelCtx;
use super::emPanelScope::PanelScope;
use super::emPanelTree::PanelId;
use super::emScheduler::EngineScheduler;

/// Probe attached to a `PanelCycleEngine` in test/test-support builds.
/// Records the scheduler's `time_slice_counter` on the engine's first
/// `Cycle` invocation. Used by SP4.5-FIX-1 timing fixtures (Tasks 5-7)
/// to measure slices-between-create-and-first-Cycle.
#[cfg(any(test, feature = "test-support"))]
#[derive(Clone)]
pub(crate) struct PanelCycleEngineFirstCycleProbe {
    pub captured_slice: std::rc::Rc<std::cell::Cell<Option<u64>>>,
}

pub(crate) struct PanelCycleEngine {
    pub(crate) panel_id: PanelId,
    /// Phase 2 Task 5: replaces `view: Weak<RefCell<emView>>`. The engine
    /// now identifies its owning view by scope (top-level `WindowId` or
    /// sub-view panel id), resolved at `Cycle` entry through
    /// `PanelScope::resolve_view`.
    pub(crate) scope: PanelScope,
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) first_cycle_probe: Option<PanelCycleEngineFirstCycleProbe>,
}

impl emEngine for PanelCycleEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(probe) = &self.first_cycle_probe {
            if probe.captured_slice.get().is_none() {
                probe.captured_slice.set(Some(ctx.time_slice_counter()));
            }
        }

        // Phase 2 Task 7: tallness is cached on `PanelTree`
        // (`cached_pixel_tallness`, kept in sync by `emView::SetGeometry`).
        // Phase 3.5.A Task 6.2: `ctx.tree` is now `Option<&mut PanelTree>`;
        // a PanelCycleEngine is always registered with a window-scoped
        // `PanelScope` (Toplevel or SubView), so `ctx.tree` is Some.
        let _ = &self.scope;
        // Phase 3.5.A Task 6.2: `ctx.tree` is `Option<&mut PanelTree>`;
        // a PanelCycleEngine is always registered with a window-scoped
        // `PanelScope` (Toplevel or SubView), so `ctx.tree` is Some.
        let (tallness, behavior) = {
            let ctx_tree = ctx
                .tree
                .as_deref_mut()
                .expect("PanelCycleEngine: tree is Some for window-scoped engines");
            let tallness = ctx_tree.cached_pixel_tallness;
            let Some(behavior) = ctx_tree.take_behavior(self.panel_id) else {
                return false;
            };
            (tallness, behavior)
        };
        let mut behavior = behavior;

        // SAFETY / borrow split: `tree` is held exclusively by `pctx`; the
        // other ctx fields are re-borrowed into a fresh `EngineCtx` whose
        // `tree` field points at a throwaway tree (we intentionally do NOT
        // hand the original tree twice). Since `Cycle` impls must reach the
        // tree via `pctx`, not `ectx.tree`, swapping in a dummy is sound.
        //
        // We use `PanelTree` default for the dummy; it's discarded after the
        // call. The cost is one PanelTree allocation per cycled panel, which
        // matches the pre-Phase-1.5 path's per-cycle take/put cost profile.
        let mut dummy_tree = crate::emPanelTree::PanelTree::new();
        let stay_awake = {
            // SAFETY: `ectx.scheduler` and `pctx.scheduler` alias the same
            // `EngineScheduler`. This is sound because:
            //   1. The event loop is single-threaded — no concurrent access.
            //   2. `EngineScheduler` methods (wake_up, register_engine, etc.)
            //      are safe to call re-entrantly from within a Cycle callback;
            //      this mirrors C++ where emEngine::Cycle receives a raw pointer
            //      to the scheduler and may call scheduler methods freely.
            //   3. No two `&mut` operations on distinct subfields alias each
            //      other — all scheduler mutations go through the same handle.
            let sched_ptr: *mut EngineScheduler = &mut *ctx.scheduler;
            let fw_ptr: *mut Vec<crate::emEngineCtx::DeferredAction> = &mut *ctx.framework_actions;
            let mut ectx = crate::emEngineCtx::EngineCtx {
                // SAFETY: see above — aliased borrow of scheduler is sound here.
                scheduler: unsafe { &mut *sched_ptr },
                // Phase 3.5.A Task 6.2: `tree` is `Option<&mut PanelTree>`.
                // We hand the dummy tree through so nested Cycle callers
                // that happen to inspect `ectx.tree` see a live tree; the
                // real tree is carried via `pctx` below.
                tree: Some(&mut dummy_tree),
                windows: &mut *ctx.windows,
                root_context: ctx.root_context,
                // SAFETY: `framework_actions` is aliased with `pctx` below.
                // Same justification as scheduler — single-threaded re-entrant
                // access through distinct handles to the same Vec is sound;
                // no concurrent mutation. Phase-3 B3.1 extension.
                framework_actions: unsafe { &mut *fw_ptr },
                pending_inputs: &mut *ctx.pending_inputs,
                input_state: &mut *ctx.input_state,
                framework_clipboard: ctx.framework_clipboard,
                engine_id: ctx.engine_id,
            };
            let pctx_tree = ctx
                .tree
                .as_deref_mut()
                .expect("PanelCycleEngine: tree is Some for window-scoped engines");
            let mut pctx = PanelCtx::with_sched_reach(
                pctx_tree,
                self.panel_id,
                tallness,
                // SAFETY: see above — aliased borrow of scheduler is sound here.
                unsafe { &mut *sched_ptr },
                // SAFETY: see above — aliased borrow of framework_actions is sound here.
                unsafe { &mut *fw_ptr },
                ctx.root_context,
                ctx.framework_clipboard,
            );
            behavior.Cycle(&mut ectx, &mut pctx)
        };
        drop(dummy_tree);
        // Re-borrow ctx.tree (lifetime reset after the `ctx_tree` borrow ended
        // with `pctx`'s scope).
        let ctx_tree = ctx
            .tree
            .as_deref_mut()
            .expect("PanelCycleEngine: tree is Some for window-scoped engines");
        if ctx_tree.panels.contains_key(self.panel_id) {
            ctx_tree.put_behavior(self.panel_id, behavior);
        }
        stay_awake
    }
}
