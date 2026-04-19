//! EngineCtx, SchedCtx, InitCtx — event-loop-threaded mutable-state bundles.
//!
//! This module replaces the `Rc<RefCell<EngineScheduler>>` ownership model.
//! See `docs/superpowers/specs/2026-04-19-port-ownership-rewrite-design.md` §3.1.
//!
//! All items here are `pub(crate)` scaffolding during the port-rewrite; they
//! will become the canonical ctx types once later phases retire the old
//! `emEngine::EngineCtx` and `emGUIFramework::DeferredAction`.

// Scaffolding introduced in phase-1 Task 1; consumers land in later phase-1
// tasks (Task 6 rewires construction sites, Task 10 wires the deferred-action
// pump). Allow dead-code until those tasks retire the old ctx types.
#![allow(dead_code)]

use std::collections::HashMap;
use std::rc::Rc;

use crate::emContext::emContext;
use crate::emEngine::{EngineId, Priority};
use crate::emScheduler::EngineScheduler;
use crate::emSignal::SignalId;

pub(crate) enum DeferredAction {
    /// Close a winit window after the current time slice. Drained by the
    /// framework's post-cycle action pump so that window teardown does not
    /// happen inside an engine's Cycle.
    CloseWindow(winit::window::WindowId),
    /// Materialize a popup's winit window after the current time slice.
    /// Popup materialization is deferred to the framework pump (Task 10)
    /// so `emView::RawVisitAbs` can request the popup without owning winit.
    MaterializePopup(winit::window::WindowId),
}

pub(crate) struct EngineCtx<'a> {
    pub(crate) scheduler: &'a mut EngineScheduler,
    pub(crate) windows: &'a mut HashMap<winit::window::WindowId, crate::emWindow::emWindow>,
    pub(crate) root_context: &'a Rc<emContext>,
    pub(crate) framework_actions: &'a mut Vec<DeferredAction>,
    /// Populated by the scheduler at Cycle-dispatch time; identifies the
    /// engine whose Cycle is currently executing. Read by ctx methods that
    /// need to attribute work to the calling engine.
    pub(crate) current_engine: Option<EngineId>,
}

pub(crate) struct SchedCtx<'a> {
    pub(crate) scheduler: &'a mut EngineScheduler,
    pub(crate) framework_actions: &'a mut Vec<DeferredAction>,
    pub(crate) root_context: &'a Rc<emContext>,
    /// Populated by the scheduler at Cycle-dispatch time; identifies the
    /// engine whose Cycle is currently executing. Read by ctx methods that
    /// need to attribute work to the calling engine.
    pub(crate) current_engine: Option<EngineId>,
}

/// Construction-only ctx used before the scheduler has started its first
/// time slice. Intentionally trait-only: exposes `ConstructCtx` so engines
/// can be registered and signals created, but does NOT expose
/// fire/connect/remove — those operations are only valid once scheduling
/// has begun (per spec §3.1).
pub(crate) struct InitCtx<'a> {
    pub(crate) scheduler: &'a mut EngineScheduler,
    pub(crate) framework_actions: &'a mut Vec<DeferredAction>,
    pub(crate) root_context: &'a Rc<emContext>,
}

pub(crate) trait ConstructCtx {
    fn create_signal(&mut self) -> SignalId;
    fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId;
    fn wake_up(&mut self, eng: EngineId);
}

impl EngineCtx<'_> {
    pub(crate) fn framework_action(&mut self, action: DeferredAction) {
        self.framework_actions.push(action);
    }

    pub(crate) fn create_signal(&mut self) -> SignalId {
        self.scheduler.create_signal()
    }

    pub(crate) fn fire(&mut self, id: SignalId) {
        self.scheduler.fire(id);
    }

    pub(crate) fn remove_signal(&mut self, id: SignalId) {
        self.scheduler.remove_signal(id);
    }

    pub(crate) fn wake_up(&mut self, id: EngineId) {
        self.scheduler.wake_up(id);
    }

    pub(crate) fn connect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.connect(signal, engine);
    }

    pub(crate) fn disconnect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.disconnect(signal, engine);
    }

    pub(crate) fn remove_engine(&mut self, id: EngineId) {
        self.scheduler.remove_engine(id);
    }

    pub(crate) fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId {
        self.scheduler.register_engine(pri, behavior)
    }
}

impl SchedCtx<'_> {
    pub(crate) fn create_signal(&mut self) -> SignalId {
        self.scheduler.create_signal()
    }

    pub(crate) fn fire(&mut self, id: SignalId) {
        self.scheduler.fire(id);
    }

    pub(crate) fn remove_signal(&mut self, id: SignalId) {
        self.scheduler.remove_signal(id);
    }

    pub(crate) fn connect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.connect(signal, engine);
    }

    pub(crate) fn disconnect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.disconnect(signal, engine);
    }

    pub(crate) fn remove_engine(&mut self, id: EngineId) {
        self.scheduler.remove_engine(id);
    }

    pub(crate) fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId {
        self.scheduler.register_engine(pri, behavior)
    }

    pub(crate) fn wake_up(&mut self, eng: EngineId) {
        self.scheduler.wake_up(eng);
    }
}

impl ConstructCtx for SchedCtx<'_> {
    fn create_signal(&mut self) -> SignalId {
        self.scheduler.create_signal()
    }

    fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId {
        self.scheduler.register_engine(pri, behavior)
    }

    fn wake_up(&mut self, eng: EngineId) {
        self.scheduler.wake_up(eng);
    }
}

impl ConstructCtx for InitCtx<'_> {
    fn create_signal(&mut self) -> SignalId {
        self.scheduler.create_signal()
    }

    fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId {
        self.scheduler.register_engine(pri, behavior)
    }

    fn wake_up(&mut self, eng: EngineId) {
        self.scheduler.wake_up(eng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emScheduler::EngineScheduler;

    #[test]
    fn sched_ctx_exposes_full_api() {
        let mut sched = EngineScheduler::new();
        let mut actions = Vec::new();
        let ctx_root = crate::emContext::emContext::NewRoot();
        let mut sc = SchedCtx {
            scheduler: &mut sched,
            framework_actions: &mut actions,
            root_context: &ctx_root,
            current_engine: None,
        };

        // create_signal returns distinct ids.
        let sig_a = sc.create_signal();
        let sig_b = sc.create_signal();
        assert_ne!(sig_a, sig_b);

        // fire marks the signal pending (observable via scheduler state).
        assert!(!sc.scheduler.is_pending(sig_a));
        sc.fire(sig_a);
        assert!(sc.scheduler.is_pending(sig_a));
        assert!(!sc.scheduler.is_pending(sig_b));

        // remove_signal drops the signal; a subsequent fire is a silent no-op
        // and is_pending reports false.
        sc.remove_signal(sig_a);
        sc.fire(sig_a);
        assert!(!sc.scheduler.is_pending(sig_a));
    }
}
