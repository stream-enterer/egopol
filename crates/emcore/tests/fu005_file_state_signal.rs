//! FU-005: emRecFileModel file-state-signal lifecycle and fire coverage.
//!
//! Phase 1 establishes that `GetFileStateSignal` returns a real (non-null)
//! id after `ensure_file_state_signal` is called at first Cycle. Phase 2
//! adds the fire-side coverage (separate test functions below).
//!
//! RUST_ONLY: (dependency-forced) — no C++ test analogue; same rationale as
//! `no_wire_b002_emrecfilemodel`.

use std::path::PathBuf;

use slotmap::Key as _;

use emcore::emRecFileModel::emRecFileModel;
use emcore::emRecParser::{RecError, RecStruct};
use emcore::emRecRecord::Record;
use emcore::test_view_harness::TestViewHarness;

#[derive(Clone, Default, Debug, PartialEq)]
struct DummyRec {
    a: i32,
}

impl Record for DummyRec {
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
fn get_file_state_signal_returns_null_until_ensured() {
    use emcore::emFileModel::FileModelState;

    let m = emRecFileModel::<DummyRec>::new(PathBuf::from("/tmp/fu005_null.rec"));

    // Pre-ensure: trait impl returns null per the lazy invariant.
    assert!(
        m.GetFileStateSignal().is_null(),
        "FileStateSignal must be null before ensure_file_state_signal is called"
    );
    assert!(
        m.file_state_signal_for_test().is_null(),
        "raw cell slot must be null pre-ensure"
    );

    // Promote via the lazy accessor.
    let mut h = TestViewHarness::new();
    let id1 = {
        let mut sc = h.sched_ctx();
        m.ensure_file_state_signal(&mut sc)
    };
    assert!(!id1.is_null(), "ensure_file_state_signal must return a real id");

    // Idempotent: second call returns the same id.
    let id2 = {
        let mut sc = h.sched_ctx();
        m.ensure_file_state_signal(&mut sc)
    };
    assert_eq!(id1, id2, "ensure_file_state_signal must be idempotent");

    // Trait impl now returns the real id.
    assert_eq!(
        m.GetFileStateSignal(),
        id1,
        "post-ensure, GetFileStateSignal must return the live id"
    );
}
