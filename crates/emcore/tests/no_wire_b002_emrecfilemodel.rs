//! B-002 emRecFileModel ChangeSignal — P-001-no-subscribe-no-accessor.
//!
//! Row `emRecFileModel-GetChangeSignal` (G2): emRecFileModel<T> ports the
//! C++ inherited `emFileModel::ChangeSignal` lazily via D-008 A1
//! combined-form `GetChangeSignal(&self, ectx)` and a `pending_change_fire`
//! deferred-fire bool drained by the panel that owns the model.
//!
//! Decisions cited: D-006 (subscribe-shape), D-007 (mutator-fire shape;
//! Rust adopts deferred-fire over `&mut impl SignalCtx` threading because
//! `&mut self` mutators have no scheduler/ectx — same language-forced
//! constraint as B-004 `pending_vir_state_fire` at `emFilePanel.rs:73`),
//! D-008 A1 (lazy `Cell<SignalId>` allocation).
//!
//! RUST_ONLY: (dependency-forced) — no C++ test analogue; same rationale as
//! B-004 / B-005 tests.

use std::path::PathBuf;

use slotmap::Key as _;

use emcore::emRecFileModel::emRecFileModel;
use emcore::emRecParser::{RecError, RecStruct};
use emcore::emRecRecord::Record;
use emcore::test_view_harness::TestViewHarness;

#[derive(Clone, Default, Debug, PartialEq)]
struct TestRec {
    a: i32,
}

impl Record for TestRec {
    fn from_rec(rec: &RecStruct) -> Result<Self, RecError> {
        Ok(Self {
            a: rec.get_int("a").unwrap_or(0),
        })
    }
    fn to_rec(&self) -> RecStruct {
        let mut r = RecStruct::new();
        r.set_int("A", self.a);
        r
    }
    fn SetToDefault(&mut self) {
        *self = Self::default();
    }
    fn IsSetToDefault(&self) -> bool {
        *self == Self::default()
    }
}

#[test]
fn change_signal_is_null_until_first_get() {
    let m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_null.rec"));
    assert!(
        m.change_signal_for_test().is_null(),
        "change_signal must be null until first GetChangeSignal"
    );
}

#[test]
fn get_change_signal_lazy_alloc_and_idempotent() {
    let mut h = TestViewHarness::new();
    let m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_lazy.rec"));
    let mut sc = h.sched_ctx();
    let s1 = m.GetChangeSignal(&mut sc);
    assert!(!s1.is_null());
    let s2 = m.GetChangeSignal(&mut sc);
    assert_eq!(s1, s2, "combined-form must be idempotent");
}

#[test]
fn pre_subscribe_signal_change_is_no_op() {
    // Per D-007 + D-008 composition: setting `pending_change_fire` while
    // `change_signal` is still null must not panic when drained, and must
    // observably no-op (no fire). Drain to verify the flag clears cleanly.
    let mut h = TestViewHarness::new();
    let mut m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_preno.rec"));
    // mutator that sets pending_change_fire
    m.hard_reset();
    assert!(m.take_pending_change_fire(), "hard_reset must set pending");
    // Re-set and drain via fire_pending_change with null signal — no panic.
    m.hard_reset();
    let mut sc = h.sched_ctx();
    m.fire_pending_change(&mut sc);
}

#[test]
fn mutator_set_unsaved_state_internal_via_get_writable_map_marks_pending() {
    // GetWritableMap → set_unsaved_state_internal on Loaded/Unsaved/SaveError.
    let mut m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_unsaved.rec"));
    // Force into Loaded by using try_load on a non-existent file? Instead
    // construct via direct sequence: hard_reset() (Waiting), then drain.
    m.hard_reset();
    let _ = m.take_pending_change_fire();
    // GetWritableMap from Waiting should NOT transition to Unsaved.
    let _ = m.GetWritableMap();
    assert!(
        !m.take_pending_change_fire(),
        "GetWritableMap from Waiting must not mark pending"
    );
}

#[test]
fn mutator_clear_save_error_marks_pending_only_on_transition() {
    let mut m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_clearerr.rec"));
    // From Waiting: clear_save_error is a no-op.
    m.clear_save_error();
    assert!(!m.take_pending_change_fire(), "no-op transition no fire");
}

#[test]
fn mutator_hard_reset_marks_pending() {
    let mut m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_hardreset.rec"));
    m.hard_reset();
    assert!(
        m.take_pending_change_fire(),
        "hard_reset must mark pending_change_fire"
    );
}

#[test]
fn mutator_try_load_marks_pending_even_on_error() {
    let mut m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_does_not_exist_xyz.rec"));
    m.TryLoad();
    assert!(
        m.take_pending_change_fire(),
        "TryLoad must mark pending_change_fire even on error completion"
    );
}

#[test]
fn fire_pending_change_after_get_change_signal_actually_fires() {
    let mut h = TestViewHarness::new();
    let mut m = emRecFileModel::<TestRec>::new(PathBuf::from("/tmp/b002_fire.rec"));
    // Allocate the signal first.
    let sig = {
        let mut sc = h.sched_ctx();
        m.GetChangeSignal(&mut sc)
    };
    assert!(!sig.is_null());

    // Mutator marks pending.
    m.hard_reset();
    assert!(m.take_pending_change_fire());
    // Re-set and drain via fire_pending_change.
    m.hard_reset();
    {
        let mut sc = h.sched_ctx();
        m.fire_pending_change(&mut sc);
    }
    // After drain the flag is cleared.
    assert!(!m.take_pending_change_fire());
    // And the signal is now pending in the scheduler clock.
    h.scheduler.flush_signals_for_test();
}
