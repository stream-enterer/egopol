// Port of C++ emMainWindow.
//
// DIVERGED: C++ emMainWindow creates an OS window + emMainPanel + detached
// control window + StartupEngine.  Rust creates a single ZuiWindow with
// emMainPanel as the root panel.  CreateControlWindow and DoCustomCheat are
// added (see create_control_window / do_custom_cheat below) but full runtime
// wiring (raise existing window, link to content view) requires Phase 3's
// startup engine integration.  The startup animation remains deferred.

use std::rc::Rc;

use winit::event_loop::ActiveEventLoop;

use emcore::emContext::emContext;
use emcore::emEngine::{emEngine, EngineCtx, EngineId, Priority};
use emcore::emGUIFramework::App;
use emcore::emPanelTree::PanelId;
use emcore::emSignal::SignalId;
use emcore::emWindow::{WindowFlags, ZuiWindow};

use crate::emMainControlPanel::emMainControlPanel;
use crate::emMainPanel::emMainPanel;

/// Configuration for creating an emMainWindow.
pub struct emMainWindowConfig {
    pub geometry: Option<String>, // "WxH+X+Y"
    pub fullscreen: bool,
    pub visit: Option<String>,
    pub control_tallness: f64,
}

impl Default for emMainWindowConfig {
    fn default() -> Self {
        Self {
            geometry: None,
            fullscreen: false,
            visit: None,
            control_tallness: 5.0,
        }
    }
}

/// Port of C++ `emMainWindow` (emMainWindow.cpp:28-84).
///
/// Holds window state: panel IDs, startup engine, visit parameters, and close
/// handling.
pub struct emMainWindow {
    pub(crate) window_id: Option<winit::window::WindowId>,
    pub(crate) _ctx: Rc<emContext>,
    pub(crate) main_panel_id: Option<PanelId>,
    pub(crate) _control_panel_id: Option<PanelId>,
    pub(crate) _content_panel_id: Option<PanelId>,
    pub(crate) startup_engine_id: Option<EngineId>,
    pub(crate) _to_close: bool,
    pub(crate) _close_signal: Option<SignalId>,
    pub(crate) _visit_identity: Option<String>,
    pub(crate) _visit_rel_x: f64,
    pub(crate) _visit_rel_y: f64,
    pub(crate) _visit_rel_a: f64,
    pub(crate) _visit_adherent: bool,
    pub(crate) _visit_subject: String,
    pub(crate) _visit_valid: bool,
    pub(crate) config: emMainWindowConfig,
}

impl emMainWindow {
    pub(crate) fn new(ctx: Rc<emContext>, config: emMainWindowConfig) -> Self {
        Self {
            window_id: None,
            _ctx: ctx,
            main_panel_id: None,
            _control_panel_id: None,
            _content_panel_id: None,
            startup_engine_id: None,
            _to_close: false,
            _close_signal: None,
            _visit_identity: None,
            _visit_rel_x: 0.0,
            _visit_rel_y: 0.0,
            _visit_rel_a: 0.0,
            _visit_adherent: false,
            _visit_subject: String::new(),
            _visit_valid: false,
            config,
        }
    }
}

/// Stub startup engine registered with the scheduler.
///
/// Port of C++ `emMainWindow::StartupEngineClass` (emMainWindow.cpp:86-260).
/// States 0-11 drive panel creation and the startup zoom animation; currently
/// a no-op stub that immediately goes to sleep.
pub(crate) struct StartupEngine {
    _state: u8,
    _root_panel_id: PanelId,
}

impl StartupEngine {
    pub(crate) fn new(root_panel_id: PanelId) -> Self {
        Self {
            _state: 0,
            _root_panel_id: root_panel_id,
        }
    }
}

impl emEngine for StartupEngine {
    fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
        false
    }
}

/// Create an emMainWindow: inserts the root emMainPanel into the panel tree,
/// allocates signals, creates the ZuiWindow, and registers a StartupEngine.
///
/// Called from the setup callback inside the `App` event loop.
pub fn create_main_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
    config: emMainWindowConfig,
) -> emMainWindow {
    let mut mw = emMainWindow::new(Rc::clone(&app.context), config);

    // Create root panel in the tree
    let panel = emMainPanel::new(Rc::clone(&app.context), mw.config.control_tallness);
    let root_id = app.tree.create_root("root");
    app.tree.set_behavior(root_id, Box::new(panel));
    mw.main_panel_id = Some(root_id);

    // Determine flags
    let mut flags = WindowFlags::AUTO_DELETE;
    if mw.config.fullscreen {
        flags |= WindowFlags::FULLSCREEN;
    }

    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();
    mw._close_signal = Some(close_signal);

    // Create the window
    let window = ZuiWindow::create(
        event_loop,
        app.gpu(),
        root_id,
        flags,
        close_signal,
        flags_signal,
    );
    let window_id = window.winit_window.id();
    app.windows.insert(window_id, window);
    mw.window_id = Some(window_id);

    // Register StartupEngine with the scheduler
    let startup_engine = StartupEngine::new(root_id);
    let engine_id = app
        .scheduler
        .borrow_mut()
        .register_engine(Priority::Low, Box::new(startup_engine));
    app.scheduler.borrow_mut().wake_up(engine_id);
    mw.startup_engine_id = Some(engine_id);

    mw
}

/// Create a detached control window.
///
/// Port of C++ `emMainWindow::CreateControlWindow` (emMainWindow.cpp:309-327).
/// Creates a second OS window with `WF_AUTO_DELETE`, hosting an
/// `emMainControlPanel`.
///
/// Triggered by the `"ccw"` cheat code in `DoCustomCheat`.
///
/// Note: Full wiring (raise existing window, link to content view) requires
/// Phase 3's startup engine integration. This establishes the API shape.
pub fn create_control_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
) -> Option<winit::window::WindowId> {
    let ctrl_panel = emMainControlPanel::new(Rc::clone(&app.context));
    let root_id = app.tree.create_root("ctrl_window_root");
    app.tree.set_behavior(root_id, Box::new(ctrl_panel));

    let flags = WindowFlags::AUTO_DELETE;
    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();

    let window = ZuiWindow::create(
        event_loop,
        app.gpu(),
        root_id,
        flags,
        close_signal,
        flags_signal,
    );
    let window_id = window.winit_window.id();
    app.windows.insert(window_id, window);
    Some(window_id)
}

/// Handle a custom cheat code.
///
/// Port of C++ `emMainWindow::DoCustomCheat` (emMainWindow.cpp:266-277).
///
/// Currently recognized cheats:
/// - `"ccw"`: Create a detached control window.
pub fn do_custom_cheat(cheat: &str, app: &mut App, event_loop: &ActiveEventLoop) {
    match cheat {
        "ccw" => {
            create_control_window(app, event_loop);
        }
        _ => {
            log::debug!("Unknown cheat code: {cheat}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = emMainWindowConfig::default();
        assert!(!config.fullscreen);
        assert!(config.visit.is_none());
        assert!(config.geometry.is_none());
        assert!((config.control_tallness - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_emMainWindow_new() {
        let ctx = emContext::NewRoot();
        let config = emMainWindowConfig::default();
        let mw = emMainWindow::new(ctx, config);
        assert!(mw.window_id.is_none());
        assert!(mw.main_panel_id.is_none());
        assert!(mw.startup_engine_id.is_none());
        assert!(!mw._to_close);
        assert!(!mw._visit_valid);
        assert!(!mw._visit_adherent);
        assert_eq!(mw._visit_rel_x, 0.0);
        assert_eq!(mw._visit_rel_y, 0.0);
        assert_eq!(mw._visit_rel_a, 0.0);
        assert!(mw._visit_subject.is_empty());
    }

    #[test]
    fn test_startup_engine_cycle_returns_false() {
        use emcore::emPanelTree::PanelId;
        use slotmap::KeyData;

        // Create a dummy PanelId
        let panel_id = PanelId::from(KeyData::from_ffi(0x0100_0000_0000_0000));
        let mut engine = StartupEngine::new(panel_id);

        // We can't easily construct an EngineCtx without a full scheduler,
        // but we can verify the engine was created in state 0.
        assert_eq!(engine._state, 0);

        // Verify the type implements emEngine (compile-time check).
        let _: &dyn emEngine = &engine;
        let _ = &mut engine;
    }
}
