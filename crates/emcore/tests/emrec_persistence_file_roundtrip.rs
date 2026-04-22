//! Phase 4d Task 4 — file-backed round-trip via `emRecFileWriter` and
//! `emRecFileReader`. Writes an `emBoolRec(true)` to a `NamedTempFile`,
//! reads it back into a fresh default-`false` `emBoolRec`, and asserts
//! byte-stability + value equality.
//!
//! Mirrors the in-memory `emrec_persistence_bool_roundtrip.rs` pattern
//! (Phase 4d Task 2) but exercises the file I/O layer.

use emcore::emBoolRec::emBoolRec;
use emcore::emClipboard::emClipboard;
use emcore::emContext::emContext;
use emcore::emEngineCtx::{DeferredAction, FrameworkDeferredAction, SchedCtx};
use emcore::emRec::emRec;
use emcore::emRecFileReader::emRecFileReader;
use emcore::emRecFileWriter::emRecFileWriter;
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
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    }
}

#[test]
fn bool_rec_file_roundtrip() {
    let mut sched = EngineScheduler::new();
    let mut actions: Vec<DeferredAction> = Vec::new();
    let ctx_root = emContext::NewRoot();
    let cb: RefCell<Option<Box<dyn emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<FrameworkDeferredAction>>> = Rc::new(RefCell::new(Vec::new()));
    let mut sc = make_sched_ctx(&mut sched, &mut actions, &ctx_root, &cb, &pa);

    // Pick a tempfile path. Drop the NamedTempFile handle so the writer can
    // re-open the path for writing (std::fs::write truncates on open).
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    drop(tmp);

    let rec = emBoolRec::new(&mut sc, true);
    let mut w = emRecFileWriter::new(&path);
    rec.TryWrite(&mut w).unwrap();
    w.finalize().unwrap();

    // On-disk bytes are exactly what the in-memory path would produce
    // (C++ emBoolRec::TryStartWriting → `yes`, emRec.cpp:371).
    let bytes_on_disk = std::fs::read(&path).unwrap();
    assert_eq!(bytes_on_disk.as_slice(), b"yes");

    let mut r = emRecFileReader::new(&path).unwrap();
    let mut rec2 = emBoolRec::new(&mut sc, false);
    rec2.TryRead(&mut r, &mut sc).unwrap();
    assert_eq!(rec2.GetValue(), rec.GetValue());

    // Teardown.
    let sig1 = rec.GetValueSignal();
    let sig2 = rec2.GetValueSignal();
    sc.scheduler.abort(sig1);
    sc.scheduler.abort(sig2);
    sc.remove_signal(sig1);
    sc.remove_signal(sig2);

    // Tidy up the tempfile path.
    let _ = std::fs::remove_file(&path);
}
