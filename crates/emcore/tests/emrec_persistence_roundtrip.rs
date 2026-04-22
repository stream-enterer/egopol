//! Phase 4d Task 3 — round-trip tests for every concrete primitive emRec
//! type's `TryRead` / `TryWrite` pair.
//!
//! Byte-stability contract: for each type, write → read → write must produce
//! identical bytes. Every test below asserts this.
//!
//! Compound types (emStructRec, emUnionRec, emArrayRec, emTArrayRec) are NOT
//! covered here — see the Task 3 closeout report for the BLOCKED reason
//! (emRecNode trait carries no TryRead/TryWrite, so boxed children cannot
//! dispatch dynamically).

use emcore::emAlignment::{EM_ALIGN_BOTTOM, EM_ALIGN_RIGHT, EM_ALIGN_TOP_LEFT};
use emcore::emAlignmentRec::emAlignmentRec;
use emcore::emClipboard::emClipboard;
use emcore::emColor::emColor;
use emcore::emColorRec::emColorRec;
use emcore::emContext::emContext;
use emcore::emDoubleRec::emDoubleRec;
use emcore::emEngineCtx::{DeferredAction, FrameworkDeferredAction, SchedCtx};
use emcore::emEnumRec::emEnumRec;
use emcore::emFlagsRec::emFlagsRec;
use emcore::emIntRec::emIntRec;
use emcore::emRec::emRec;
use emcore::emRecMemReader::emRecMemReader;
use emcore::emRecMemWriter::emRecMemWriter;
use emcore::emScheduler::EngineScheduler;
use emcore::emStringRec::emStringRec;
use std::cell::RefCell;
use std::rc::Rc;

fn make_sched_ctx<'a>(
    sched: &'a mut EngineScheduler,
    actions: &'a mut Vec<DeferredAction>,
    ctx_root: &'a Rc<emContext>,
    cb: &'a RefCell<Option<Box<dyn emClipboard>>>,
    pa: &'a Rc<RefCell<Vec<FrameworkDeferredAction>>>,
) -> SchedCtx<'a> {
    SchedCtx {
        scheduler: sched,
        framework_actions: actions,
        root_context: ctx_root,
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    }
}

/// Owned scheduler bundle (keeps all SchedCtx borrows alive for a whole test).
struct Fixture {
    sched: EngineScheduler,
    actions: Vec<DeferredAction>,
    ctx_root: Rc<emContext>,
    cb: RefCell<Option<Box<dyn emClipboard>>>,
    pa: Rc<RefCell<Vec<FrameworkDeferredAction>>>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            sched: EngineScheduler::new(),
            actions: Vec::new(),
            ctx_root: emContext::NewRoot(),
            cb: RefCell::new(None),
            pa: Rc::new(RefCell::new(Vec::new())),
        }
    }
    fn sc(&mut self) -> SchedCtx<'_> {
        make_sched_ctx(
            &mut self.sched,
            &mut self.actions,
            &self.ctx_root,
            &self.cb,
            &self.pa,
        )
    }
}

#[test]
fn int_rec_roundtrip() {
    let mut fx = Fixture::new();

    let mut sc = fx.sc();
    let mut rec = emIntRec::new(&mut sc, 0, -100, 100);
    rec.SetValue(42, &mut sc);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    assert_eq!(bytes.as_slice(), b"42");

    let mut sc = fx.sc();
    let mut rec2 = emIntRec::new(&mut sc, 0, -100, 100);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    let s1 = rec.GetValueSignal();
    let s2 = rec2.GetValueSignal();
    sc.scheduler.abort(s1);
    sc.scheduler.abort(s2);
    sc.remove_signal(s1);
    sc.remove_signal(s2);
}

#[test]
fn int_rec_rejects_out_of_range() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let mut rec = emIntRec::new(&mut sc, 0, 0, 10);
    let mut r = emRecMemReader::new(b"999");
    let err = rec.TryRead(&mut r, &mut sc).unwrap_err().to_string();
    assert!(err.contains("too large"), "{err}");

    let mut r = emRecMemReader::new(b"-1");
    let err = rec.TryRead(&mut r, &mut sc).unwrap_err().to_string();
    assert!(err.contains("too small"), "{err}");

    let s = rec.GetValueSignal();
    sc.scheduler.abort(s);
    sc.remove_signal(s);
}

#[test]
fn double_rec_roundtrip() {
    let mut fx = Fixture::new();

    let mut sc = fx.sc();
    let mut rec = emDoubleRec::new(&mut sc, 0.0, -1e6, 1e6);
    rec.SetValue(3.14159, &mut sc);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();

    let mut sc = fx.sc();
    let mut rec2 = emDoubleRec::new(&mut sc, 0.0, -1e6, 1e6);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    let s1 = rec.GetValueSignal();
    let s2 = rec2.GetValueSignal();
    sc.scheduler.abort(s1);
    sc.scheduler.abort(s2);
    sc.remove_signal(s1);
    sc.remove_signal(s2);
}

#[test]
fn string_rec_roundtrip() {
    let mut fx = Fixture::new();

    let mut sc = fx.sc();
    let mut rec = emStringRec::new(&mut sc, String::new());
    rec.SetValue("hello \"world\"\n".to_string(), &mut sc);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();

    let mut sc = fx.sc();
    let mut rec2 = emStringRec::new(&mut sc, String::new());
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    let s1 = rec.GetValueSignal();
    let s2 = rec2.GetValueSignal();
    sc.scheduler.abort(s1);
    sc.scheduler.abort(s2);
    sc.remove_signal(s1);
    sc.remove_signal(s2);
}

#[test]
fn enum_rec_roundtrip() {
    let mut fx = Fixture::new();
    let ids = || vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

    let mut sc = fx.sc();
    let mut rec = emEnumRec::new(&mut sc, 0, ids());
    rec.SetValue(2, &mut sc); // "gamma"
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    assert_eq!(bytes.as_slice(), b"gamma");

    let mut sc = fx.sc();
    let mut rec2 = emEnumRec::new(&mut sc, 0, ids());
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    // Int-form read also works (C++ emRec.cpp:678-683).
    let mut sc = fx.sc();
    let mut rec3 = emEnumRec::new(&mut sc, 0, ids());
    let mut r = emRecMemReader::new(b"1");
    rec3.TryRead(&mut r, &mut sc).unwrap();
    assert_eq!(*rec3.GetValue(), 1);
    drop(sc);

    let mut sc = fx.sc();
    for s in [
        rec.GetValueSignal(),
        rec2.GetValueSignal(),
        rec3.GetValueSignal(),
    ] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn enum_rec_rejects_unknown_identifier() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let mut rec = emEnumRec::new(&mut sc, 0, vec!["alpha".to_string(), "beta".to_string()]);
    let mut r = emRecMemReader::new(b"gamma");
    let err = rec.TryRead(&mut r, &mut sc).unwrap_err().to_string();
    assert!(err.contains("Unknown identifier"), "{err}");

    let s = rec.GetValueSignal();
    sc.scheduler.abort(s);
    sc.remove_signal(s);
}

#[test]
fn flags_rec_roundtrip_multiple_bits() {
    let mut fx = Fixture::new();

    let mut sc = fx.sc();
    let mut rec = emFlagsRec::new(&mut sc, 0, &["read", "write", "exec"]);
    rec.SetValue(0b101, &mut sc); // read + exec
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    assert_eq!(bytes.as_slice(), b"{read exec}");

    let mut sc = fx.sc();
    let mut rec2 = emFlagsRec::new(&mut sc, 0, &["read", "write", "exec"]);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    for s in [rec.GetValueSignal(), rec2.GetValueSignal()] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn flags_rec_empty_set_roundtrip() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let rec = emFlagsRec::new(&mut sc, 0, &["a", "b"]);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    assert_eq!(bytes.as_slice(), b"{}");

    let mut sc = fx.sc();
    let mut rec2 = emFlagsRec::new(&mut sc, 0b11, &["a", "b"]);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    assert_eq!(*rec2.GetValue(), 0);
    drop(sc);

    let mut sc = fx.sc();
    for s in [rec.GetValueSignal(), rec2.GetValueSignal()] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn alignment_rec_roundtrip_combo() {
    let mut fx = Fixture::new();

    let mut sc = fx.sc();
    let mut rec = emAlignmentRec::new(&mut sc, 0);
    rec.SetValue(EM_ALIGN_TOP_LEFT, &mut sc);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    // Top bit emits first, then left — joined by `-`.
    assert_eq!(bytes.as_slice(), b"top-left");

    let mut sc = fx.sc();
    let mut rec2 = emAlignmentRec::new(&mut sc, 0);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    for s in [rec.GetValueSignal(), rec2.GetValueSignal()] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn alignment_rec_center_when_no_bits_set() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let rec = emAlignmentRec::new(&mut sc, 0);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    assert_eq!(w.into_bytes().as_slice(), b"center");

    let mut sc = fx.sc();
    let mut rec2 = emAlignmentRec::new(&mut sc, EM_ALIGN_BOTTOM | EM_ALIGN_RIGHT);
    let mut r = emRecMemReader::new(b"center");
    rec2.TryRead(&mut r, &mut sc).unwrap();
    assert_eq!(*rec2.GetValue(), 0);
    drop(sc);

    let mut sc = fx.sc();
    for s in [rec.GetValueSignal(), rec2.GetValueSignal()] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn color_rec_roundtrip_rgb() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let mut rec = emColorRec::new(&mut sc, emColor::BLACK, false);
    rec.SetValue(emColor::rgba(0x11, 0x22, 0x33, 255), &mut sc);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    assert_eq!(bytes.as_slice(), b"{17 34 51}");

    let mut sc = fx.sc();
    let mut rec2 = emColorRec::new(&mut sc, emColor::BLACK, false);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    for s in [rec.GetValueSignal(), rec2.GetValueSignal()] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn color_rec_roundtrip_rgba() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let mut rec = emColorRec::new(&mut sc, emColor::BLACK, true);
    rec.SetValue(emColor::rgba(10, 20, 30, 128), &mut sc);
    drop(sc);

    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();
    assert_eq!(bytes.as_slice(), b"{10 20 30 128}");

    let mut sc = fx.sc();
    let mut rec2 = emColorRec::new(&mut sc, emColor::BLACK, true);
    let mut r = emRecMemReader::new(&bytes);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    drop(sc);
    assert_eq!(rec2.GetValue(), rec.GetValue());

    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(w2.into_bytes(), bytes);

    let mut sc = fx.sc();
    for s in [rec.GetValueSignal(), rec2.GetValueSignal()] {
        sc.scheduler.abort(s);
        sc.remove_signal(s);
    }
}

#[test]
fn color_rec_accepts_quoted_hex_form() {
    // C++ emRec.cpp:1191-1198 reads a quoted string via emColor::TryParse.
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let mut rec = emColorRec::new(&mut sc, emColor::BLACK, true);
    let mut r = emRecMemReader::new(b"\"#112233\"");
    rec.TryRead(&mut r, &mut sc).unwrap();
    assert_eq!(*rec.GetValue(), emColor::rgba(0x11, 0x22, 0x33, 255));

    let s = rec.GetValueSignal();
    sc.scheduler.abort(s);
    sc.remove_signal(s);
}

#[test]
fn color_rec_rejects_channel_out_of_range() {
    let mut fx = Fixture::new();
    let mut sc = fx.sc();
    let mut rec = emColorRec::new(&mut sc, emColor::BLACK, false);
    let mut r = emRecMemReader::new(b"{300 0 0}");
    let err = rec.TryRead(&mut r, &mut sc).unwrap_err().to_string();
    assert!(err.contains("out of range"), "{err}");

    let s = rec.GetValueSignal();
    sc.scheduler.abort(s);
    sc.remove_signal(s);
}
