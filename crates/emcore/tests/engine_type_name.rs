//! Tests for type-name capture in EngineScheduler::register_engine and
//! register_engine_dyn.
//!
//! RUST_ONLY: (language-forced-utility) Rust trait objects do not preserve
//! concrete-type information through their vtables; C++ has full RTTI. These
//! tests verify the monomorphized type_name capture path.

use emcore::emEngine::{emEngine, Priority};
use emcore::emEngineCtx::EngineCtx;
use emcore::emPanelScope::PanelScope;
use emcore::emScheduler::EngineScheduler;

struct DummyEngine;

impl emEngine for DummyEngine {
    fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
        false
    }
}

#[test]
fn register_engine_captures_concrete_type_name() {
    let mut sched = EngineScheduler::new();
    let id = sched.register_engine(DummyEngine, Priority::Medium, PanelScope::Framework);
    let name = sched.engine_type_name(id).expect("engine must exist");
    assert!(
        name.ends_with("DummyEngine"),
        "expected type name ending with 'DummyEngine', got {name:?}"
    );
    sched.remove_engine(id);
}

#[test]
fn register_engine_dyn_uses_explicit_name() {
    let mut sched = EngineScheduler::new();
    let id = sched.register_engine_dyn(
        Box::new(DummyEngine),
        "test::ExplicitName",
        Priority::Medium,
        PanelScope::Framework,
    );
    let name = sched.engine_type_name(id).expect("engine must exist");
    assert_eq!(name, "test::ExplicitName");
    sched.remove_engine(id);
}
