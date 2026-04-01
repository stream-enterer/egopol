// Port of C++ emMainWindow.
//
// DIVERGED: C++ emMainWindow creates an OS window + emMainPanel + detached
// control window + StartupEngine.  Rust creates a single ZuiWindow with
// emMainPanel as the root panel.  CreateControlWindow and DoCustomCheat are
// added (see create_control_window / do_custom_cheat below) but full runtime
// wiring (raise existing window, link to content view) requires Phase 3's
// startup engine integration.  The startup animation remains deferred.

use std::cell::RefCell;
use std::rc::Rc;

use winit::event_loop::ActiveEventLoop;

use emcore::emContext::emContext;
use emcore::emEngine::{emEngine, EngineCtx, EngineId, Priority};
use emcore::emGUIFramework::App;
use emcore::emInput::{emInputEvent, InputKey};
use emcore::emInputState::emInputState;
use emcore::emPanelTree::PanelId;
use emcore::emSignal::SignalId;
use emcore::emWindow::{WindowFlags, ZuiWindow};

use crate::emMainControlPanel::emMainControlPanel;
use crate::emMainPanel::emMainPanel;

/// Shared state between StartupEngine and emMainWindow.
///
/// The engine advances `state` as it progresses through startup stages;
/// the window reads it to drive panel creation.
#[derive(Debug)]
pub(crate) struct StartupState {
    pub(crate) state: u8,
    pub(crate) done: bool,
}

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
    pub(crate) startup_state: Option<Rc<RefCell<StartupState>>>,
    pub to_close: bool,
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
            startup_state: None,
            to_close: false,
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

    /// Read shared startup state and drive panel creation stages.
    ///
    /// Called from the application event loop after the scheduler runs engines.
    /// Port of C++ `emMainWindow` startup handling (emMainWindow.cpp:362-422).
    pub fn cycle_startup(&mut self, app: &mut App) {
        let Some(ref shared) = self.startup_state else {
            return;
        };

        // Check if startup is done — remove overlay and engine.
        if shared.borrow().done {
            if let Some(main_id) = self.main_panel_id {
                app.tree
                    .with_behavior_as::<emMainPanel, _>(main_id, |mp| {
                        mp.SetStartupOverlay(false);
                    });
            }
            if let Some(eid) = self.startup_engine_id.take() {
                app.scheduler.borrow_mut().remove_engine(eid);
            }
            self.startup_state = None;
            return;
        }

        let state = shared.borrow().state;

        match state {
            5 => {
                // Advance emMainPanel to creation_stage 1 (create control panel).
                if let Some(main_id) = self.main_panel_id {
                    app.tree
                        .with_behavior_as::<emMainPanel, _>(main_id, |mp| {
                            mp.advance_creation_stage();
                        });
                }
            }
            6 => {
                // Advance emMainPanel to creation_stage 2 (create content panel).
                if let Some(main_id) = self.main_panel_id {
                    app.tree
                        .with_behavior_as::<emMainPanel, _>(main_id, |mp| {
                            mp.advance_creation_stage();
                        });
                }
            }
            _ => {}
        }
    }

    /// Port of C++ `emMainWindow::ToggleFullscreen`.
    pub fn ToggleFullscreen(&self, app: &mut App) {
        if let Some(win) = self.window_id.and_then(|id| app.windows.get_mut(&id)) {
            let new_flags = win.flags ^ WindowFlags::FULLSCREEN;
            win.SetWindowFlags(new_flags);
        }
    }

    /// Port of C++ `emMainWindow::ReloadFiles`.
    pub fn ReloadFiles(&self) {
        log::info!("emMainWindow::ReloadFiles");
    }

    /// Port of C++ `emMainWindow::ToggleControlView`.
    pub fn ToggleControlView(&mut self, app: &mut App) {
        if let Some(main_id) = self.main_panel_id {
            app.tree
                .with_behavior_as::<emMainPanel, _>(main_id, |mp| {
                    mp.DoubleClickSlider();
                });
        }
    }

    /// Port of C++ `emMainWindow::Close`.
    pub fn Close(&mut self) {
        self.to_close = true;
    }

    /// Port of C++ `emMainWindow::Quit`.
    pub fn Quit(&self, app: &App) {
        app.scheduler.borrow_mut().InitiateTermination();
    }

    /// Port of C++ `emMainWindow::Input` (emMainWindow.cpp:193-263).
    ///
    /// DIVERGED: C++ Input uses emInputEvent, Rust uses the same struct but
    /// reads modifier state from emInputState (matching C++ behavior of
    /// checking the global input state rather than per-event modifiers).
    pub fn handle_input(
        &mut self,
        event: &emInputEvent,
        input_state: &emInputState,
        app: &mut App,
    ) -> bool {
        match event.key {
            InputKey::F4
                if !input_state.GetShift()
                    && !input_state.GetCtrl()
                    && input_state.GetAlt() =>
            {
                self.Close();
                true
            }
            InputKey::F4
                if input_state.GetShift()
                    && !input_state.GetCtrl()
                    && input_state.GetAlt() =>
            {
                self.Quit(app);
                true
            }
            InputKey::F5
                if !input_state.GetShift()
                    && !input_state.GetCtrl()
                    && !input_state.GetAlt() =>
            {
                self.ReloadFiles();
                true
            }
            InputKey::F11
                if !input_state.GetShift()
                    && !input_state.GetCtrl()
                    && !input_state.GetAlt() =>
            {
                self.ToggleFullscreen(app);
                true
            }
            InputKey::Escape
                if !input_state.GetShift()
                    && !input_state.GetCtrl()
                    && !input_state.GetAlt() =>
            {
                self.ToggleControlView(app);
                true
            }
            _ => false,
        }
    }
}

/// Startup engine registered with the scheduler.
///
/// Port of C++ `emMainWindow::StartupEngineClass` (emMainWindow.cpp:86-260).
/// States 0-6 drive panel creation; states 7-11 drive the startup zoom
/// animation.
pub(crate) struct StartupEngine {
    state: u8,
    _root_panel_id: PanelId,
    shared: Rc<RefCell<StartupState>>,
    clock: std::time::Instant,
}

impl StartupEngine {
    pub(crate) fn new(root_panel_id: PanelId, shared: Rc<RefCell<StartupState>>) -> Self {
        Self {
            state: 0,
            _root_panel_id: root_panel_id,
            shared,
            clock: std::time::Instant::now(),
        }
    }
}

impl emEngine for StartupEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        match self.state {
            // States 0-2: idle wake-ups.
            0..=2 => {
                self.state += 1;
                true
            }
            // State 3: MainPanel already created (Task 3). Update shared state and advance.
            3 => {
                self.shared.borrow_mut().state = 3;
                self.state += 1;
                true
            }
            // State 4: signal bookmark acquisition.
            4 => {
                self.shared.borrow_mut().state = 4;
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 5: signal control panel creation.
            5 => {
                self.shared.borrow_mut().state = 5;
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 6: signal content panel creation.
            6 => {
                self.shared.borrow_mut().state = 6;
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 7: Start zoom animation — record clock, advance.
            7 => {
                self.clock = std::time::Instant::now();
                self.shared.borrow_mut().state = 7;
                self.state += 1;
                true
            }
            // State 8: Wait up to 2 seconds for root zoom.
            8 => {
                if self.clock.elapsed().as_millis() < 2000 {
                    true // keep waiting
                } else {
                    self.state += 1;
                    true
                }
            }
            // State 9: Set goal to visit target (if any).
            9 => {
                self.clock = std::time::Instant::now();
                self.shared.borrow_mut().state = 9;
                self.state += 1;
                true
            }
            // State 10: Wait up to 2 seconds, then signal overlay removal.
            10 => {
                if self.clock.elapsed().as_millis() < 2000 {
                    true
                } else {
                    self.shared.borrow_mut().state = 10;
                    self.clock = std::time::Instant::now();
                    self.state += 1;
                    true
                }
            }
            // State 11: 100ms pause, then signal done.
            11 => {
                if self.clock.elapsed().as_millis() < 100 {
                    true
                } else {
                    self.shared.borrow_mut().done = true;
                    false // engine stops
                }
            }
            _ => false,
        }
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

    // Create shared startup state for engine ↔ window communication.
    let shared = Rc::new(RefCell::new(StartupState {
        state: 0,
        done: false,
    }));
    mw.startup_state = Some(Rc::clone(&shared));

    // Register StartupEngine with the scheduler
    let startup_engine = StartupEngine::new(root_id, shared);
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
        assert!(!mw.to_close);
        assert!(!mw._visit_valid);
        assert!(!mw._visit_adherent);
        assert_eq!(mw._visit_rel_x, 0.0);
        assert_eq!(mw._visit_rel_y, 0.0);
        assert_eq!(mw._visit_rel_a, 0.0);
        assert!(mw._visit_subject.is_empty());
    }

    #[test]
    fn test_startup_engine_initial_state() {
        use emcore::emPanelTree::PanelId;
        use slotmap::KeyData;

        let panel_id = PanelId::from(KeyData::from_ffi(0x0100_0000_0000_0000));
        let shared = Rc::new(RefCell::new(StartupState {
            state: 0,
            done: false,
        }));
        let engine = StartupEngine::new(panel_id, Rc::clone(&shared));

        assert_eq!(engine.state, 0);
        assert_eq!(engine._root_panel_id, panel_id);
        assert_eq!(shared.borrow().state, 0);
        assert!(!shared.borrow().done);

        // Verify the type implements emEngine (compile-time check).
        let _: &dyn emEngine = &engine;
    }

    #[test]
    fn test_startup_state_debug() {
        let state = StartupState {
            state: 3,
            done: false,
        };
        let debug = format!("{state:?}");
        assert!(debug.contains("state: 3"));
        assert!(debug.contains("done: false"));
    }

    #[test]
    fn test_close_sets_flag() {
        let ctx = emContext::NewRoot();
        let config = emMainWindowConfig::default();
        let mut mw = emMainWindow::new(ctx, config);
        assert!(!mw.to_close);
        mw.Close();
        assert!(mw.to_close);
    }

    #[test]
    fn test_startup_state_done() {
        let shared = Rc::new(RefCell::new(StartupState {
            state: 0,
            done: false,
        }));
        shared.borrow_mut().done = true;
        assert!(shared.borrow().done);
    }
}
