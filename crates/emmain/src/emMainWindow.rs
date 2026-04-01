// Port of C++ emMainWindow.
//
// DIVERGED: C++ emMainWindow creates an OS window + emMainPanel + detached
// control window + StartupEngine.  Rust creates a single ZuiWindow with
// emMainPanel as the root panel.  The detached control window and startup
// animation are deferred.

use std::rc::Rc;

use winit::event_loop::ActiveEventLoop;

use emcore::emGUIFramework::App;
use emcore::emWindow::{WindowFlags, ZuiWindow};

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

/// Create an emMainWindow: inserts the root emMainPanel into the panel tree,
/// allocates signals, and creates the ZuiWindow.
///
/// Called from the setup callback inside the `App` event loop.
pub fn create_main_window(
    app: &mut App,
    event_loop: &ActiveEventLoop,
    config: &emMainWindowConfig,
) {
    // Create root panel in the tree
    let panel = emMainPanel::new(Rc::clone(&app.context), config.control_tallness);
    let root_id = app.tree.create_root("root");
    app.tree.set_behavior(root_id, Box::new(panel));

    // Determine flags
    let mut flags = WindowFlags::AUTO_DELETE;
    if config.fullscreen {
        flags |= WindowFlags::FULLSCREEN;
    }

    let close_signal = app.scheduler.borrow_mut().create_signal();
    let flags_signal = app.scheduler.borrow_mut().create_signal();

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
}
