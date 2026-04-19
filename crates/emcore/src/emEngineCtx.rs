//! EngineCtx, SchedCtx, InitCtx — event-loop-threaded mutable-state bundles.
//!
//! This module replaces the `Rc<RefCell<EngineScheduler>>` ownership model.
//! See `docs/superpowers/specs/2026-04-19-port-ownership-rewrite-design.md` §3.1.

use std::collections::HashMap;
use std::rc::Rc;

use crate::emContext::emContext;
use crate::emEngine::{EngineId, Priority};
use crate::emScheduler::EngineScheduler;
use crate::emSignal::SignalId;

pub enum DeferredAction {
    CloseWindow(winit::window::WindowId),
    MaterializePopup(winit::window::WindowId),
}

pub struct EngineCtx<'a> {
    pub scheduler: &'a mut EngineScheduler,
    pub windows: &'a mut HashMap<winit::window::WindowId, crate::emWindow::emWindow>,
    pub root_context: &'a Rc<emContext>,
    pub framework_actions: &'a mut Vec<DeferredAction>,
    pub current_engine: Option<EngineId>,
}

pub struct SchedCtx<'a> {
    pub scheduler: &'a mut EngineScheduler,
    pub framework_actions: &'a mut Vec<DeferredAction>,
    pub root_context: &'a Rc<emContext>,
    pub current_engine: Option<EngineId>,
}

pub struct InitCtx<'a> {
    pub scheduler: &'a mut EngineScheduler,
    pub framework_actions: &'a mut Vec<DeferredAction>,
    pub root_context: &'a Rc<emContext>,
}

pub trait ConstructCtx {
    fn create_signal(&mut self) -> SignalId;
    fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId;
    fn wake_up(&mut self, eng: EngineId);
}

impl<'a> EngineCtx<'a> {
    pub fn framework_action(&mut self, action: DeferredAction) {
        self.framework_actions.push(action);
    }

    pub fn create_signal(&mut self) -> SignalId {
        self.scheduler.create_signal()
    }

    pub fn fire(&mut self, id: SignalId) {
        self.scheduler.fire(id);
    }

    pub fn remove_signal(&mut self, id: SignalId) {
        self.scheduler.remove_signal(id);
    }

    pub fn wake_up(&mut self, id: EngineId) {
        self.scheduler.wake_up(id);
    }

    pub fn connect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.connect(signal, engine);
    }

    pub fn disconnect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.disconnect(signal, engine);
    }

    pub fn remove_engine(&mut self, id: EngineId) {
        self.scheduler.remove_engine(id);
    }

    pub fn register_engine(
        &mut self,
        behavior: Box<dyn crate::emEngine::emEngine>,
        pri: Priority,
    ) -> EngineId {
        self.scheduler.register_engine(pri, behavior)
    }
}

impl<'a> SchedCtx<'a> {
    pub fn fire(&mut self, id: SignalId) {
        self.scheduler.fire(id);
    }

    pub fn remove_signal(&mut self, id: SignalId) {
        self.scheduler.remove_signal(id);
    }

    pub fn connect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.connect(signal, engine);
    }

    pub fn disconnect(&mut self, signal: SignalId, engine: EngineId) {
        self.scheduler.disconnect(signal, engine);
    }

    pub fn remove_engine(&mut self, id: EngineId) {
        self.scheduler.remove_engine(id);
    }
}

impl<'a> ConstructCtx for SchedCtx<'a> {
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

impl<'a> ConstructCtx for InitCtx<'a> {
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
        let sig = sc.create_signal();
        sc.fire(sig);
        sc.remove_signal(sig);
    }
}
