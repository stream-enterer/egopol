// RUST_ONLY: (dependency-forced) no C++ test analogue (C++ test surface is X11 integration).
//
// D4 self-update integration tests: verifies FactorFieldPanel self-update via
// per-field signal subscription introduced in D4, and verifies that Reset
// updates scalar fields in-place (no child rebuild).
//
// Child-cycle note: FactorFieldPanel children have PanelCycleEngines
// registered as Toplevel(dummy_window_id). Because tests run without a real
// emWindow in the `windows` map, the scheduler silently skips those engines.
// To exercise FactorFieldPanel::Cycle, the test wrapper engine manually drives
// each child's Cycle via take_behavior/call/put_behavior — mirroring exactly
// what PanelCycleEngine does in production.

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use emcore::emCoreConfig::emCoreConfig;
use emcore::emCoreConfigPanel::{
    factor_cfg_to_val, ButtonsPanel, FactorFieldPanel, KBGroup, MouseMiscGroup,
};
use emcore::emEngine::{emEngine, Priority};
use emcore::emEngineCtx::{EngineCtx, PanelCtx, SchedCtx};
use emcore::emLook::emLook;
use emcore::emPanel::PanelBehavior;
use emcore::emPanelScope::PanelScope;
use emcore::emPanelTree::{PanelId, PanelTree};
use emcore::emRec::emRec;
use emcore::emRecNodeConfigModel::emRecNodeConfigModel;
use emcore::emScheduler::EngineScheduler;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn do_slice(sched: &mut EngineScheduler) {
    use winit::window::WindowId;
    let mut windows: HashMap<WindowId, emcore::emWindow::emWindow> = HashMap::new();
    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let mut pending_inputs: Vec<(WindowId, emcore::emInput::emInputEvent)> = Vec::new();
    let mut input_state = emcore::emInputState::emInputState::new();
    let cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));
    sched.DoTimeSlice(
        &mut windows,
        &root_ctx,
        &mut fw,
        &mut pending_inputs,
        &mut input_state,
        &cb,
        &pa,
    );
}

fn make_config(
    sched: &mut EngineScheduler,
    fw_actions: &mut Vec<emcore::emEngineCtx::DeferredAction>,
    root_ctx: &Rc<emcore::emContext::emContext>,
    cb: &RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>>,
    pa: &Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>>,
    install_path: std::path::PathBuf,
) -> Rc<RefCell<emRecNodeConfigModel<emCoreConfig>>> {
    let mut sc = SchedCtx {
        scheduler: sched,
        framework_actions: fw_actions,
        root_context: root_ctx,
        view_context: None,
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    };
    let cfg = emCoreConfig::new(&mut sc);
    let model = emRecNodeConfigModel::new(cfg, install_path, &mut sc);
    Rc::new(RefCell::new(model))
}

fn modify_config(
    config: &Rc<RefCell<emRecNodeConfigModel<emCoreConfig>>>,
    sched: &mut EngineScheduler,
    fw_actions: &mut Vec<emcore::emEngineCtx::DeferredAction>,
    root_ctx: &Rc<emcore::emContext::emContext>,
    cb: &RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>>,
    pa: &Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>>,
    f: impl FnOnce(&mut emCoreConfig, &mut SchedCtx<'_>),
) {
    let mut cm = config.borrow_mut();
    let mut sc = SchedCtx {
        scheduler: sched,
        framework_actions: fw_actions,
        root_context: root_ctx,
        view_context: None,
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    };
    cm.modify(f, &mut sc);
}

fn detach_config(
    config: &Rc<RefCell<emRecNodeConfigModel<emCoreConfig>>>,
    sched: &mut EngineScheduler,
    fw_actions: &mut Vec<emcore::emEngineCtx::DeferredAction>,
    root_ctx: &Rc<emcore::emContext::emContext>,
    cb: &RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>>,
    pa: &Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>>,
) {
    let mut sc = SchedCtx {
        scheduler: sched,
        framework_actions: fw_actions,
        root_context: root_ctx,
        view_context: None,
        framework_clipboard: cb,
        current_engine: None,
        pending_actions: pa,
    };
    config.borrow_mut().detach(&mut sc);
}

/// Manually drive FactorFieldPanel::Cycle for the child at `child_id`.
///
/// Child PanelCycleEngines are registered as Toplevel(dummy_window_id).
/// The test harness passes an empty `windows` map, so the scheduler skips
/// those engines. This helper mirrors what PanelCycleEngine does in
/// production: take the behavior, call Cycle, put it back.
fn cycle_factor_field(
    tree: &mut PanelTree,
    child_id: PanelId,
    sched_ptr: *mut EngineScheduler,
    fw_ptr: *mut Vec<emcore::emEngineCtx::DeferredAction>,
    root_ctx: &Rc<emcore::emContext::emContext>,
    cb: &RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>>,
    pa: &Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>>,
    engine_id: emcore::emEngine::EngineId,
) {
    let mut behavior = match tree.take_behavior(child_id) {
        Some(b) => b,
        None => return,
    };
    {
        let mut ectx = EngineCtx {
            scheduler: unsafe { &mut *sched_ptr },
            tree: None,
            windows: &mut HashMap::new(),
            root_context: root_ctx,
            view_context: None,
            framework_actions: unsafe { &mut *fw_ptr },
            pending_inputs: &mut Vec::new(),
            input_state: &mut emcore::emInputState::emInputState::new(),
            framework_clipboard: cb,
            engine_id,
            pending_actions: pa,
        };
        let mut pctx = PanelCtx::with_sched_reach(
            tree,
            child_id,
            1.0,
            unsafe { &mut *sched_ptr },
            unsafe { &mut *fw_ptr },
            root_ctx,
            cb,
            pa,
        );
        behavior.Cycle(&mut ectx, &mut pctx);
    }
    if tree.contains(child_id) {
        tree.put_behavior(child_id, behavior);
    }
}

// ---------------------------------------------------------------------------
// Test 1: FactorFieldPanel self-updates when per-field signal fires
// ---------------------------------------------------------------------------

/// D4 Test 1: After KBGroup creates its children (subscribing FactorFieldPanel
/// to KeyboardZoomSpeed's per-field signal), mutating KeyboardZoomSpeed fires
/// the signal, and FactorFieldPanel::Cycle calls `set_value_silent` — the
/// zoom slider reads back `factor_cfg_to_val(4.0, 0.25, 4.0) = 200.0`.
///
/// This verifies the D4 self-update wiring in FactorFieldPanel::Cycle:
/// subscribes to `config_sig` in first Cycle, then on signal fires reads
/// `get_config_val()` and updates the display value in place.
#[test]
fn scalar_field_panel_self_updates_on_field_change() {
    let mut sched = EngineScheduler::new();
    let install_path = std::env::temp_dir().join("d4_test1_kb_self_update.rec");
    let _ = std::fs::remove_file(&install_path);

    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    let config = make_config(
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        install_path.clone(),
    );

    // Pre-state: KeyboardZoomSpeed = 1.0 (default).
    assert_eq!(
        *config.borrow().GetRec().KeyboardZoomSpeed.GetValue(),
        1.0,
        "pre-state: KeyboardZoomSpeed must be 1.0 (default)"
    );

    let look = emLook::new();
    let panel_rc: Rc<RefCell<KBGroup>> =
        Rc::new(RefCell::new(KBGroup::new(Rc::clone(&config), look)));

    let tree_rc: Rc<RefCell<PanelTree>> = Rc::new(RefCell::new(PanelTree::new()));
    let root: PanelId = tree_rc
        .borrow_mut()
        .create_root_deferred_view("d4_test1_kb_self_update");

    // Keep the engine_id for use in cycle_factor_field.
    let engine_id_cell: Rc<RefCell<Option<emcore::emEngine::EngineId>>> =
        Rc::new(RefCell::new(None));

    struct PanelEngine {
        panel: Rc<RefCell<KBGroup>>,
        tree: Rc<RefCell<PanelTree>>,
        root: PanelId,
        children_built: bool,
        cycles_run: u32,
        engine_id_out: Rc<RefCell<Option<emcore::emEngine::EngineId>>>,
    }
    impl emEngine for PanelEngine {
        fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
            let sched_ptr: *mut EngineScheduler = &mut *ctx.scheduler;
            let fw_ptr: *mut Vec<emcore::emEngineCtx::DeferredAction> = &mut *ctx.framework_actions;
            // Record engine_id for use in cycle_factor_field.
            *self.engine_id_out.borrow_mut() = Some(ctx.engine_id);
            {
                let mut tree_borrow = self.tree.borrow_mut();
                // Create_children phase (pctx scope).
                {
                    let mut pctx = PanelCtx::with_sched_reach(
                        &mut *tree_borrow,
                        self.root,
                        1.0,
                        unsafe { &mut *sched_ptr },
                        unsafe { &mut *fw_ptr },
                        ctx.root_context,
                        ctx.framework_clipboard,
                        ctx.pending_actions,
                    );
                    if !self.children_built {
                        self.panel.borrow_mut().create_children(&mut pctx);
                        self.children_built = true;
                    }
                } // pctx dropped here — releases mutable borrow on tree_borrow.

                // Manually drive FactorFieldPanel::Cycle for each KB child.
                // PanelCycleEngine for children is Toplevel(dummy) and is skipped
                // by the scheduler when windows is empty — drive it manually here.
                let zoom_id = tree_borrow.find_child_by_name(self.root, "zoom");
                let scroll_id = tree_borrow.find_child_by_name(self.root, "scroll");
                if let Some(id) = zoom_id {
                    cycle_factor_field(
                        &mut tree_borrow,
                        id,
                        sched_ptr,
                        fw_ptr,
                        ctx.root_context,
                        ctx.framework_clipboard,
                        ctx.pending_actions,
                        ctx.engine_id,
                    );
                }
                if let Some(id) = scroll_id {
                    cycle_factor_field(
                        &mut tree_borrow,
                        id,
                        sched_ptr,
                        fw_ptr,
                        ctx.root_context,
                        ctx.framework_clipboard,
                        ctx.pending_actions,
                        ctx.engine_id,
                    );
                }
            }
            self.cycles_run += 1;
            self.cycles_run < 4
        }
    }

    let engine = PanelEngine {
        panel: Rc::clone(&panel_rc),
        tree: Rc::clone(&tree_rc),
        root,
        children_built: false,
        cycles_run: 0,
        engine_id_out: Rc::clone(&engine_id_cell),
    };
    let eid = sched.register_engine(engine, Priority::Low, PanelScope::Framework);
    sched.wake_up(eid);

    // First slice: create_children runs; FactorFieldPanel::Cycle subscribes
    // to KeyboardZoomSpeed's per-field signal.
    do_slice(&mut sched);

    // Mutate config: set KeyboardZoomSpeed to max (4.0).
    // This fires the per-field signal via emDoubleRec::SetValue.
    modify_config(
        &config,
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        |c, sc| {
            c.KeyboardZoomSpeed.SetValue(4.0, sc);
        },
    );
    assert_eq!(
        *config.borrow().GetRec().KeyboardZoomSpeed.GetValue(),
        4.0,
        "config must have KeyboardZoomSpeed = 4.0 after mutation"
    );

    // Second slice: per-field signal fires → PanelEngine::Cycle calls
    // cycle_factor_field → FactorFieldPanel::Cycle sees IsSignaled → calls
    // set_value_silent(get_config_val()) = factor_cfg_to_val(4.0) = 200.0.
    do_slice(&mut sched);

    // Assert: zoom child's scalar_field.GetValue() == factor_cfg_to_val(4.0, 0.25, 4.0) = 200.0.
    let expected = factor_cfg_to_val(4.0, 0.25, 4.0);
    let zoom_id = tree_rc
        .borrow()
        .find_child_by_name(root, "zoom")
        .expect("zoom child must exist");
    let actual = tree_rc
        .borrow_mut()
        .with_behavior_as::<FactorFieldPanel, _>(zoom_id, |p| p.scalar_field.GetValue())
        .expect("zoom panel must downcast to FactorFieldPanel");

    assert!(
        (actual - expected).abs() < 1e-9,
        "zoom scalar_field.GetValue() must equal factor_cfg_to_val(4.0, 0.25, 4.0) = {expected} \
         after per-field signal; got {actual}"
    );

    sched.remove_engine(eid);
    {
        let mut tree = tree_rc.borrow_mut();
        tree.remove(root, Some(&mut sched));
    }
    detach_config(&config, &mut sched, &mut fw_actions, &root_ctx, &cb, &pa);
    sched.clear_pending_for_tests();
    drop(panel_rc);
    drop(config);
    let _ = std::fs::remove_file(&install_path);
}

// ---------------------------------------------------------------------------
// Test 2: MouseMiscGroup::update_output fires on config aggregate signal
// ---------------------------------------------------------------------------

/// D4 Test 2: After MouseMiscGroup subscribes to the config aggregate signal,
/// mutating StickMouseWhenNavigating fires the aggregate, and Cycle calls
/// `update_output` → `set_checked_silent` on the stick checkbox.
///
/// This verifies the D4 config-aggregate subscribe path in MouseMiscGroup::Cycle:
/// `subscribed_to_config` guard → `ectx.connect(config_sig, eid)` →
/// `ectx.IsSignaled(config_sig)` → `update_output(ctx)`.
#[test]
fn mouse_misc_group_update_output_on_config_change() {
    use slotmap::Key as _;

    let mut sched = EngineScheduler::new();
    let install_path = std::env::temp_dir().join("d4_test2_mouse_misc_update.rec");
    let _ = std::fs::remove_file(&install_path);

    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    let config = make_config(
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        install_path.clone(),
    );

    // Pre-state: StickMouseWhenNavigating = false.
    modify_config(
        &config,
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        |c, sc| {
            c.StickMouseWhenNavigating.SetValue(false, sc);
        },
    );
    assert!(
        !*config.borrow().GetRec().StickMouseWhenNavigating.GetValue(),
        "pre-state: StickMouseWhenNavigating must be false"
    );

    let look = emLook::new();
    let panel_rc: Rc<RefCell<MouseMiscGroup>> = Rc::new(RefCell::new(MouseMiscGroup::new(
        Rc::clone(&config),
        look,
        true, // stick_possible
    )));

    let tree_rc: Rc<RefCell<PanelTree>> = Rc::new(RefCell::new(PanelTree::new()));
    let root: PanelId = tree_rc
        .borrow_mut()
        .create_root_deferred_view("d4_test2_mouse_misc_update");

    struct PanelEngine {
        panel: Rc<RefCell<MouseMiscGroup>>,
        tree: Rc<RefCell<PanelTree>>,
        root: PanelId,
        children_built: bool,
        cycles_run: u32,
    }
    impl emEngine for PanelEngine {
        fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
            let sched_ptr: *mut EngineScheduler = &mut *ctx.scheduler;
            let fw_ptr: *mut Vec<emcore::emEngineCtx::DeferredAction> = &mut *ctx.framework_actions;
            let stay_awake = {
                let mut ectx = EngineCtx {
                    scheduler: unsafe { &mut *sched_ptr },
                    tree: None,
                    windows: &mut *ctx.windows,
                    root_context: ctx.root_context,
                    view_context: None,
                    framework_actions: unsafe { &mut *fw_ptr },
                    pending_inputs: &mut *ctx.pending_inputs,
                    input_state: &mut *ctx.input_state,
                    framework_clipboard: ctx.framework_clipboard,
                    engine_id: ctx.engine_id,
                    pending_actions: ctx.pending_actions,
                };
                let mut tree_borrow = self.tree.borrow_mut();
                let mut pctx = PanelCtx::with_sched_reach(
                    &mut *tree_borrow,
                    self.root,
                    1.0,
                    unsafe { &mut *sched_ptr },
                    unsafe { &mut *fw_ptr },
                    ctx.root_context,
                    ctx.framework_clipboard,
                    ctx.pending_actions,
                );
                if !self.children_built {
                    self.panel.borrow_mut().create_children(&mut pctx);
                    self.children_built = true;
                }
                self.panel.borrow_mut().Cycle(&mut ectx, &mut pctx)
            };
            self.cycles_run += 1;
            stay_awake || self.cycles_run < 4
        }
    }

    let engine = PanelEngine {
        panel: Rc::clone(&panel_rc),
        tree: Rc::clone(&tree_rc),
        root,
        children_built: false,
        cycles_run: 0,
    };
    let eid = sched.register_engine(engine, Priority::Low, PanelScope::Framework);
    sched.wake_up(eid);

    // First slice: create_children runs + Cycle subscribes to config_sig.
    do_slice(&mut sched);

    let config_sig = panel_rc.borrow().config_sig_for_test();
    assert!(
        !config_sig.is_null(),
        "MouseMiscGroup must have a non-null config_sig after construction"
    );

    // Mutate config: set StickMouseWhenNavigating = true.
    // This fires both the per-field signal AND the config aggregate signal.
    modify_config(
        &config,
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        |c, sc| {
            c.StickMouseWhenNavigating.SetValue(true, sc);
        },
    );
    assert!(
        *config.borrow().GetRec().StickMouseWhenNavigating.GetValue(),
        "config must have StickMouseWhenNavigating = true after mutation"
    );

    // Second slice: config aggregate signal fires →
    // MouseMiscGroup::Cycle → update_output → set_checked_silent(true).
    do_slice(&mut sched);

    // Assert: stick checkbox IsChecked() == true.
    let checked = panel_rc
        .borrow()
        .get_stick_checked_for_test(&mut tree_rc.borrow_mut());
    assert!(
        checked,
        "stick checkbox IsChecked() must be true after update_output fired via config_sig"
    );

    sched.remove_engine(eid);
    {
        let mut tree = tree_rc.borrow_mut();
        tree.remove(root, Some(&mut sched));
    }
    detach_config(&config, &mut sched, &mut fw_actions, &root_ctx, &cb, &pa);
    sched.clear_pending_for_tests();
    drop(panel_rc);
    drop(config);
    let _ = std::fs::remove_file(&install_path);
}

// ---------------------------------------------------------------------------
// Test 3: Reset updates scalar fields in-place (no child rebuild)
// ---------------------------------------------------------------------------

/// D4 Test 3 — regression guard: after Reset fires, per-field signals propagate
/// to FactorFieldPanel::Cycle which calls `set_value_silent` in place — the
/// zoom child's PanelId is unchanged (no destroy/recreate), and its
/// scalar_field.GetValue() reflects the default (1.0 → 0.0 slider value).
///
/// Without D4 self-update wiring this test still passes for the "in-place"
/// assertion, but the value assertion fails because the slider would keep its
/// pre-reset stale value of 200.0.
#[test]
fn reset_button_updates_in_place_no_rebuild() {
    use slotmap::Key as _;

    let mut sched = EngineScheduler::new();
    let install_path_kb = std::env::temp_dir().join("d4_test3_kb.rec");
    let install_path_bt = std::env::temp_dir().join("d4_test3_bt.rec");
    let _ = std::fs::remove_file(&install_path_kb);
    let _ = std::fs::remove_file(&install_path_bt);

    let root_ctx = emcore::emContext::emContext::NewRoot();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> = RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emGUIFramework::DeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    // Shared config — both ButtonsPanel and KBGroup use the same model.
    // ButtonsPanel writes to it on Reset; KBGroup's children observe per-field signals.
    let config = make_config(
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        install_path_kb.clone(),
    );

    // Pre-state: KeyboardZoomSpeed = 4.0 (non-default; default is 1.0).
    modify_config(
        &config,
        &mut sched,
        &mut fw_actions,
        &root_ctx,
        &cb,
        &pa,
        |c, sc| {
            c.KeyboardZoomSpeed.SetValue(4.0, sc);
        },
    );
    assert_eq!(
        *config.borrow().GetRec().KeyboardZoomSpeed.GetValue(),
        4.0,
        "pre-state: KeyboardZoomSpeed must be 4.0"
    );

    // --- KBGroup panel + tree ---
    let look = emLook::new();
    let kb_panel_rc: Rc<RefCell<KBGroup>> =
        Rc::new(RefCell::new(KBGroup::new(Rc::clone(&config), look.clone())));
    let kb_tree_rc: Rc<RefCell<PanelTree>> = Rc::new(RefCell::new(PanelTree::new()));
    let kb_root: PanelId = kb_tree_rc
        .borrow_mut()
        .create_root_deferred_view("d4_test3_kb");

    struct KbEngine {
        panel: Rc<RefCell<KBGroup>>,
        tree: Rc<RefCell<PanelTree>>,
        root: PanelId,
        children_built: bool,
        cycles_run: u32,
    }
    impl emEngine for KbEngine {
        fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
            let sched_ptr: *mut EngineScheduler = &mut *ctx.scheduler;
            let fw_ptr: *mut Vec<emcore::emEngineCtx::DeferredAction> = &mut *ctx.framework_actions;
            {
                let mut tree_borrow = self.tree.borrow_mut();
                // Create_children phase (pctx scope).
                {
                    let mut pctx = PanelCtx::with_sched_reach(
                        &mut *tree_borrow,
                        self.root,
                        1.0,
                        unsafe { &mut *sched_ptr },
                        unsafe { &mut *fw_ptr },
                        ctx.root_context,
                        ctx.framework_clipboard,
                        ctx.pending_actions,
                    );
                    if !self.children_built {
                        self.panel.borrow_mut().create_children(&mut pctx);
                        self.children_built = true;
                    }
                } // pctx dropped here.
                  // Manually drive FactorFieldPanel::Cycle for each KB child.
                let zoom_id = tree_borrow.find_child_by_name(self.root, "zoom");
                let scroll_id = tree_borrow.find_child_by_name(self.root, "scroll");
                if let Some(id) = zoom_id {
                    cycle_factor_field(
                        &mut tree_borrow,
                        id,
                        sched_ptr,
                        fw_ptr,
                        ctx.root_context,
                        ctx.framework_clipboard,
                        ctx.pending_actions,
                        ctx.engine_id,
                    );
                }
                if let Some(id) = scroll_id {
                    cycle_factor_field(
                        &mut tree_borrow,
                        id,
                        sched_ptr,
                        fw_ptr,
                        ctx.root_context,
                        ctx.framework_clipboard,
                        ctx.pending_actions,
                        ctx.engine_id,
                    );
                }
            }
            self.cycles_run += 1;
            self.cycles_run < 4
        }
    }

    let kb_engine = KbEngine {
        panel: Rc::clone(&kb_panel_rc),
        tree: Rc::clone(&kb_tree_rc),
        root: kb_root,
        children_built: false,
        cycles_run: 0,
    };
    let kb_eid = sched.register_engine(kb_engine, Priority::Low, PanelScope::Framework);
    sched.wake_up(kb_eid);

    // --- ButtonsPanel + tree (separate tree, shared config) ---
    let bt_panel_rc: Rc<RefCell<ButtonsPanel>> =
        Rc::new(RefCell::new(ButtonsPanel::new(Rc::clone(&config), look)));
    let bt_tree_rc: Rc<RefCell<PanelTree>> = Rc::new(RefCell::new(PanelTree::new()));
    let bt_root: PanelId = bt_tree_rc
        .borrow_mut()
        .create_root_deferred_view("d4_test3_bt");

    struct BtEngine {
        panel: Rc<RefCell<ButtonsPanel>>,
        tree: Rc<RefCell<PanelTree>>,
        root: PanelId,
        children_built: bool,
        cycles_run: u32,
    }
    impl emEngine for BtEngine {
        fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
            let sched_ptr: *mut EngineScheduler = &mut *ctx.scheduler;
            let fw_ptr: *mut Vec<emcore::emEngineCtx::DeferredAction> = &mut *ctx.framework_actions;
            let stay_awake = {
                let mut ectx = EngineCtx {
                    scheduler: unsafe { &mut *sched_ptr },
                    tree: None,
                    windows: &mut *ctx.windows,
                    root_context: ctx.root_context,
                    view_context: None,
                    framework_actions: unsafe { &mut *fw_ptr },
                    pending_inputs: &mut *ctx.pending_inputs,
                    input_state: &mut *ctx.input_state,
                    framework_clipboard: ctx.framework_clipboard,
                    engine_id: ctx.engine_id,
                    pending_actions: ctx.pending_actions,
                };
                let mut tree_borrow = self.tree.borrow_mut();
                let mut pctx = PanelCtx::with_sched_reach(
                    &mut *tree_borrow,
                    self.root,
                    1.0,
                    unsafe { &mut *sched_ptr },
                    unsafe { &mut *fw_ptr },
                    ctx.root_context,
                    ctx.framework_clipboard,
                    ctx.pending_actions,
                );
                if !self.children_built {
                    self.panel.borrow_mut().create_children(&mut pctx);
                    self.children_built = true;
                }
                self.panel.borrow_mut().Cycle(&mut ectx, &mut pctx)
            };
            self.cycles_run += 1;
            stay_awake || self.cycles_run < 4
        }
    }

    let bt_engine = BtEngine {
        panel: Rc::clone(&bt_panel_rc),
        tree: Rc::clone(&bt_tree_rc),
        root: bt_root,
        children_built: false,
        cycles_run: 0,
    };
    let bt_eid = sched.register_engine(bt_engine, Priority::Low, PanelScope::Framework);
    sched.wake_up(bt_eid);

    // First slice: both create_children run; FactorFieldPanel subscribes to
    // KeyboardZoomSpeed's per-field signal; ButtonsPanel subscribes to reset signal.
    do_slice(&mut sched);

    // Record zoom child ID before reset — must not change after.
    let zoom_id_before = kb_tree_rc
        .borrow()
        .find_child_by_name(kb_root, "zoom")
        .expect("zoom child must exist after create_children");

    // Verify the initial slider value reflects the pre-state (4.0 → 200.0).
    // FactorFieldPanel was initialized with factor_cfg_to_val(4.0, 0.25, 4.0) = 200.0.
    let val_before = kb_tree_rc
        .borrow_mut()
        .with_behavior_as::<FactorFieldPanel, _>(zoom_id_before, |p| p.scalar_field.GetValue())
        .expect("zoom must downcast");
    let expected_before = factor_cfg_to_val(4.0, 0.25, 4.0);
    assert!(
        (val_before - expected_before).abs() < 1e-9,
        "pre-reset: zoom scalar value must be {expected_before} (factor_cfg_to_val(4.0)); \
         got {val_before}"
    );

    // Fire the Reset button's click signal.
    let bt_reset_sig = bt_panel_rc.borrow().bt_reset_sig_for_test();
    assert!(
        !bt_reset_sig.is_null(),
        "ButtonsPanel must have captured a non-null reset click_signal"
    );
    sched.fire(bt_reset_sig);

    // Second slice: ButtonsPanel::Cycle fires reset → config.modify sets
    // KeyboardZoomSpeed = 1.0 → per-field signal fires →
    // KbEngine::Cycle calls cycle_factor_field → FactorFieldPanel::Cycle
    // calls set_value_silent(factor_cfg_to_val(1.0, 0.25, 4.0) = 0.0).
    do_slice(&mut sched);

    // Assert A: zoom child ID is unchanged (no rebuild).
    let zoom_id_after = kb_tree_rc
        .borrow()
        .find_child_by_name(kb_root, "zoom")
        .expect("zoom child must still exist after reset");
    assert_eq!(
        zoom_id_before, zoom_id_after,
        "zoom child PanelId must be unchanged after Reset (in-place update, not rebuild)"
    );

    // Assert B: zoom slider value reflects the default (1.0 → 0.0 in slider units).
    let expected_after = factor_cfg_to_val(1.0, 0.25, 4.0); // = 0.0
    let val_after = kb_tree_rc
        .borrow_mut()
        .with_behavior_as::<FactorFieldPanel, _>(zoom_id_after, |p| p.scalar_field.GetValue())
        .expect("zoom must downcast after reset");
    assert!(
        (val_after - expected_after).abs() < 1e-9,
        "post-reset: zoom scalar value must be {expected_after} (factor_cfg_to_val(1.0)); \
         got {val_after} — FactorFieldPanel self-update via per-field signal must have fired"
    );

    // Teardown.
    sched.remove_engine(kb_eid);
    sched.remove_engine(bt_eid);
    {
        let mut tree = kb_tree_rc.borrow_mut();
        tree.remove(kb_root, Some(&mut sched));
    }
    {
        let mut tree = bt_tree_rc.borrow_mut();
        tree.remove(bt_root, Some(&mut sched));
    }
    detach_config(&config, &mut sched, &mut fw_actions, &root_ctx, &cb, &pa);
    sched.clear_pending_for_tests();
    drop(kb_panel_rc);
    drop(bt_panel_rc);
    drop(config);
    let _ = std::fs::remove_file(&install_path_kb);
    let _ = std::fs::remove_file(&install_path_bt);
}
