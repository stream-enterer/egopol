//! Phase 4d Task 2 — emBoolRec round-trip via emRecMemWriter/emRecMemReader.
//!
//! Builds an `emBoolRec` with value `true`, writes it through
//! `emRecMemWriter`, reads it back into a fresh `emBoolRec` (default `false`)
//! through `emRecMemReader`, and asserts the value round-trips. Exercises
//! the byte-format symmetry between C++'s `emBoolRec::TryStartWriting`
//! (emRec.cpp:369-372 — emits `yes`/`no` identifier) and
//! `emBoolRec::TryStartReading` (emRec.cpp:334-355 — accepts
//! yes/no/y/n/true/false + 0/1).
//!
//! Co-located `make_sched_ctx` helper mirrors the pattern in
//! `emrec_compound_integration.rs` (Phase 4c); TestFixture does not exist
//! in this crate.

use emcore::emBoolRec::emBoolRec;
use emcore::emClipboard::emClipboard;
use emcore::emContext::emContext;
use emcore::emEngineCtx::{DeferredAction, FrameworkDeferredAction, SchedCtx};
use emcore::emRec::emRec;
use emcore::emRecMemReader::emRecMemReader;
use emcore::emRecMemWriter::emRecMemWriter;
use emcore::emScheduler::EngineScheduler;
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
        view_context: None,
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    }
}

#[test]
fn bool_rec_roundtrip() {
    let mut sched = EngineScheduler::new();
    let mut actions: Vec<DeferredAction> = Vec::new();
    let ctx_root = emContext::NewRoot();
    let cb: RefCell<Option<Box<dyn emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<FrameworkDeferredAction>>> = Rc::new(RefCell::new(Vec::new()));
    let mut sc = make_sched_ctx(&mut sched, &mut actions, &ctx_root, &cb, &pa);

    let rec = emBoolRec::new(&mut sc, true);
    let mut w = emRecMemWriter::new();
    rec.TryWrite(&mut w).unwrap();
    let bytes = w.into_bytes();

    // C++ emBoolRec::TryStartWriting emits `yes` for true (emRec.cpp:371).
    assert_eq!(
        bytes.as_slice(),
        b"yes",
        "emBoolRec(true) must serialise to the bare identifier `yes`",
    );

    let mut r = emRecMemReader::new(&bytes);
    let mut rec2 = emBoolRec::new(&mut sc, false);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    assert_eq!(rec2.GetValue(), rec.GetValue());

    // Write→read→write byte-stability: writing the round-tripped rec must
    // produce the exact same bytes the first write produced.
    let mut w2 = emRecMemWriter::new();
    rec2.TryWrite(&mut w2).unwrap();
    assert_eq!(
        w2.into_bytes(),
        bytes,
        "second write must match first write byte-for-byte",
    );

    // Teardown — signals created by both recs must be removed.
    let sig1 = rec.GetValueSignal();
    let sig2 = rec2.GetValueSignal();
    sc.scheduler.abort(sig1);
    sc.scheduler.abort(sig2);
    sc.remove_signal(sig1);
    sc.remove_signal(sig2);
}

#[test]
fn bool_rec_rejects_unknown_identifier() {
    let mut sched = EngineScheduler::new();
    let mut actions: Vec<DeferredAction> = Vec::new();
    let ctx_root = emContext::NewRoot();
    let cb: RefCell<Option<Box<dyn emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<FrameworkDeferredAction>>> = Rc::new(RefCell::new(Vec::new()));
    let mut sc = make_sched_ctx(&mut sched, &mut actions, &ctx_root, &cb, &pa);

    let mut r = emRecMemReader::new(b"maybe");
    let mut rec = emBoolRec::new(&mut sc, false);
    let err = rec.TryRead(&mut r, &mut sc).unwrap_err();
    // Error carries the source name + line. `Display` format is
    // `rec memory buffer:<line>: <msg>` (see RecIoError tests).
    let msg = err.to_string();
    assert!(
        msg.contains(":1:"),
        "error should carry line-1 location, got: {msg}",
    );

    let sig = rec.GetValueSignal();
    sc.scheduler.abort(sig);
    sc.remove_signal(sig);
}

#[test]
fn bool_rec_rejects_out_of_range_int() {
    // emBoolRec accepts only 0 or 1 as ET_INT (emRec.cpp:350-353).
    let mut sched = EngineScheduler::new();
    let mut actions: Vec<DeferredAction> = Vec::new();
    let ctx_root = emContext::NewRoot();
    let cb: RefCell<Option<Box<dyn emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<FrameworkDeferredAction>>> = Rc::new(RefCell::new(Vec::new()));
    let mut sc = make_sched_ctx(&mut sched, &mut actions, &ctx_root, &cb, &pa);

    let mut r = emRecMemReader::new(b"2");
    let mut rec = emBoolRec::new(&mut sc, false);
    assert!(rec.TryRead(&mut r, &mut sc).is_err());

    let sig = rec.GetValueSignal();
    sc.scheduler.abort(sig);
    sc.remove_signal(sig);
}

#[test]
fn bool_rec_accepts_all_cpp_aliases() {
    // C++ emBoolRec::TryStartReading (emRec.cpp:334-355) accepts
    // 0/1 as integer and case-insensitive yes/no/y/n/true/false.
    let cases: &[(&[u8], bool)] = &[
        (b"yes", true),
        (b"YES", true),
        (b"no", false),
        (b"No", false),
        (b"y", true),
        (b"N", false),
        (b"true", true),
        (b"FALSE", false),
        (b"1", true),
        (b"0", false),
    ];

    let mut sched = EngineScheduler::new();
    let mut actions: Vec<DeferredAction> = Vec::new();
    let ctx_root = emContext::NewRoot();
    let cb: RefCell<Option<Box<dyn emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<FrameworkDeferredAction>>> = Rc::new(RefCell::new(Vec::new()));

    for (input, expected) in cases {
        let mut sc = make_sched_ctx(&mut sched, &mut actions, &ctx_root, &cb, &pa);
        let mut r = emRecMemReader::new(input);
        let mut rec = emBoolRec::new(&mut sc, !*expected);
        rec.TryRead(&mut r, &mut sc)
            .unwrap_or_else(|e| panic!("input {:?}: {}", input, e));
        assert_eq!(
            rec.GetValue(),
            expected,
            "input {:?} expected {}",
            input,
            expected,
        );
        let sig = rec.GetValueSignal();
        sc.scheduler.abort(sig);
        sc.remove_signal(sig);
    }
}
