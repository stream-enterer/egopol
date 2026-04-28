/// B-008-typed-subscribe-misc behavioral tests.
///
/// Covers signal-accessor stability for the three rows wired in this bucket:
///
/// Row -67  (emMainPanel):       `emView::EOISignal` is a stable Option<SignalId>
///                               on every emView (installed by RegisterEngines).
/// Row -69  (emMainPanel):       `emWindow::GetWindowFlagsSignal` already covered
///                               by B-006 `typed_subscribe_b006::row_218_window_flags_signal_stable`;
///                               not duplicated here.
/// Row -104 (emVirtualCosmos):   `emFileModel::AcquireUpdateSignalModel` returns
///                               the shared `App::file_update_signal` (post-B-007).
///
/// The full Cycle-driven click-through tests live in
/// `emMainPanel::tests` and `emVirtualCosmos::tests` (internal `mod tests`)
/// because they require access to private fields (`subscribed_init`,
/// `eoi_signal`, `flags_signal`, `file_update_signal`, `change_signal`)
/// and the `emMainWindow` thread-local. This integration file covers the
/// public stability properties, mirroring the B-006 `typed_subscribe_b006.rs`
/// rationale.
///
/// RUST_ONLY: (dependency-forced) no C++ test analogue — mirrors B-003/B-006/B-007
/// typed_subscribe test rationale.
use std::collections::HashMap;
use std::rc::Rc;

use emcore::emScheduler::EngineScheduler;
use slotmap::Key as _;

/// Helper: build a minimal EngineCtx for the lifetime of `f`.
fn with_engine_ctx<R>(
    sched: &mut EngineScheduler,
    f: impl FnOnce(&mut emcore::emEngineCtx::EngineCtx<'_>) -> R,
) -> R {
    use std::cell::RefCell;
    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut windows: HashMap<winit::window::WindowId, emcore::emWindow::emWindow> = HashMap::new();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let mut pending_inputs: Vec<(winit::window::WindowId, emcore::emInput::emInputEvent)> =
        Vec::new();
    let mut input_state = emcore::emInputState::emInputState::new();
    let fw_cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));
    let engine_id = emcore::emEngine::EngineId::default();
    let mut ectx = emcore::emEngineCtx::EngineCtx {
        scheduler: sched,
        tree: None,
        windows: &mut windows,
        root_context: &root_ctx,
        framework_actions: &mut fw_actions,
        pending_inputs: &mut pending_inputs,
        input_state: &mut input_state,
        framework_clipboard: &fw_cb,
        engine_id,
        pending_actions: &pa,
    };
    f(&mut ectx)
}

/// Row -104 signal accessor: `emFileModel::AcquireUpdateSignalModel` returns
/// the shared broadcast `App::file_update_signal`, stable across calls.
///
/// Mirrors C++ row 104: `AddWakeUpSignal(FileUpdateSignalModel->Sig)`.
#[test]
fn row_104_file_update_signal_stable() {
    let mut sched = EngineScheduler::new();
    let bcast = sched.create_signal();
    sched.file_update_signal = bcast;

    let sig_a = with_engine_ctx(&mut sched, |ectx| {
        emcore::emFileModel::emFileModel::<()>::AcquireUpdateSignalModel(ectx)
    });
    let sig_b = with_engine_ctx(&mut sched, |ectx| {
        emcore::emFileModel::emFileModel::<()>::AcquireUpdateSignalModel(ectx)
    });

    assert_eq!(
        sig_a, sig_b,
        "AcquireUpdateSignalModel must return a stable SignalId"
    );
    assert_eq!(
        sig_a, bcast,
        "AcquireUpdateSignalModel must return App::file_update_signal post-B-007"
    );
    assert!(!sig_a.is_null(), "broadcast signal must be non-null");

    sched.remove_signal(bcast);
}

/// Row -67 signal accessor: `emView::EOISignal` is a public field of type
/// `Option<SignalId>`, populated by `RegisterEngines` at view-construction
/// time. We don't construct a full emView here (it requires a registered
/// view-tree); instead we verify the field shape via the public API surface
/// — `EOISignal` is accessible from outside the crate, which is the
/// observable property B-008 row -67's wire depends on.
///
/// Mirrors C++ row 67: `AddWakeUpSignal(GetControlView().GetEOISignal())`.
#[test]
fn row_67_eoi_signal_field_addressable() {
    // Compile-time assertion: the field exists on emView and is reachable
    // from outside the crate. If this compiles, the consumer-side wire
    // (emMainPanel reading control_view_panel.sub_view.EOISignal) is
    // type-stable. Behavioural coverage is in
    // emMainPanel::tests::b008_row_67_eoi_signal_finalises_slider_hide.
    fn _shape_check(v: &emcore::emView::emView) -> Option<emcore::emSignal::SignalId> {
        v.EOISignal
    }
}
