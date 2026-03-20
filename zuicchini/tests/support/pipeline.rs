#![allow(dead_code)]

use zuicchini::input::{InputEvent, InputKey, InputState, InputVariant};
use zuicchini::panel::{
    KeyboardZoomScrollVIF, MouseZoomScrollVIF, PanelBehavior, PanelId, PanelTree, View,
    ViewInputFilter,
};
use zuicchini::scheduler::EngineScheduler;

/// Test harness that dispatches input through the FULL coordinate transform
/// pipeline (VIF chain, hit test, view_to_panel_x/y transform), matching
/// the production path in `ZuiWindow::dispatch_input`.
///
/// Unlike `TestHarness` which passes view-space coordinates directly to
/// `behavior.input()`, this harness transforms mouse coordinates from view
/// space to panel-local space before delivery — exactly as the real window
/// does.
pub struct PipelineTestHarness {
    pub tree: PanelTree,
    pub scheduler: EngineScheduler,
    pub view: View,
    pub vif_chain: Vec<Box<dyn ViewInputFilter>>,
    pub input_state: InputState,
    root: PanelId,
}

impl PipelineTestHarness {
    /// Create a harness with root panel (focusable, layout 0,0,1,1), 800x600 view.
    pub fn new() -> Self {
        let mut tree = PanelTree::new();
        let root = tree.create_root("root");
        tree.set_focusable(root, true);
        tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);

        let mut view = View::new(root, 800.0, 600.0);
        view.update_viewing(&mut tree);

        let vif_chain: Vec<Box<dyn ViewInputFilter>> = vec![
            {
                let mut mouse_vif = MouseZoomScrollVIF::new();
                let zflpp = view.get_zoom_factor_log_per_pixel();
                mouse_vif.set_mouse_anim_params(1.0, 0.25, zflpp);
                mouse_vif.set_wheel_anim_params(1.0, 0.25, zflpp);
                Box::new(mouse_vif)
            },
            Box::new(KeyboardZoomScrollVIF::new()),
        ];

        Self {
            tree,
            scheduler: EngineScheduler::new(),
            view,
            vif_chain,
            input_state: InputState::new(),
            root,
        }
    }

    pub fn root(&self) -> PanelId {
        self.root
    }

    // ── Frame / tick ─────────────────────────────────────────────

    /// Run one frame: scheduler time slice, deliver notices, update viewing.
    pub fn tick(&mut self) {
        self.scheduler.do_time_slice();
        self.tree
            .deliver_notices(self.view.window_focused(), self.view.pixel_tallness());
        self.view.update_viewing(&mut self.tree);
    }

    /// Run n frames.
    pub fn tick_n(&mut self, n: usize) {
        for _ in 0..n {
            self.tick();
        }
    }

    // ── Panel management ─────────────────────────────────────────

    /// Create a focusable child panel with a layout rect.
    pub fn add_panel(&mut self, parent: PanelId, name: &str) -> PanelId {
        let id = self.tree.create_child(parent, name);
        self.tree.set_focusable(id, true);
        self.tree.set_layout_rect(id, 0.0, 0.0, 1.0, 1.0);
        id
    }

    /// Create a focusable child panel with a layout rect and behavior.
    pub fn add_panel_with(
        &mut self,
        parent: PanelId,
        name: &str,
        behavior: Box<dyn PanelBehavior>,
    ) -> PanelId {
        let id = self.add_panel(parent, name);
        self.tree.set_behavior(id, behavior);
        id
    }

    // ── Zoom control ─────────────────────────────────────────────

    /// Set the zoom level relative to the default "fit in viewport" (1x).
    ///
    /// At `level = 1.0` the root panel just fills the viewport (same as
    /// raw_zoom_out). At `level = 2.0` the root panel appears at 2x linear
    /// magnification (4x area).
    ///
    /// The zoom is centered on the viewport center so rel_x/rel_y stay at 0.
    pub fn set_zoom(&mut self, level: f64) {
        // Step 1: reset to the 1x baseline (raw_zoom_out sets rel_a to
        // zoom_out_rel_a and calls update_viewing internally).
        self.view.raw_zoom_out(&mut self.tree);

        // Step 2: apply the magnification factor. zoom() multiplies rel_a by
        // the given factor. Since viewed_width ~ sqrt(rel_a), multiplying
        // rel_a by level^2 gives level-x linear magnification.
        // Centering at viewport center keeps rel_x = rel_y = 0.
        if (level - 1.0).abs() > 1e-12 {
            let (vw, vh) = self.view.viewport_size();
            self.view.zoom(level * level, vw * 0.5, vh * 0.5);
        }

        // Step 3: refresh viewed geometry for all panels.
        self.view.update_viewing(&mut self.tree);
    }

    // ── Auto-expansion ─────────────────────────────────────────

    /// Set zoom to trigger auto-expansion and run enough ticks for
    /// `layout_children` to execute. `update_auto_expansion` runs inside
    /// `update_viewing`, so `set_zoom` already triggers it; the extra
    /// ticks let notices propagate and child panels get laid out.
    pub fn expand_to(&mut self, zoom_level: f64) {
        self.set_zoom(zoom_level);
        // Several ticks to propagate notices and execute layout_children
        self.tick_n(10);
    }

    /// Query whether a panel is currently auto-expanded.
    pub fn is_expanded(&self, panel_id: PanelId) -> bool {
        self.tree.is_auto_expanded(panel_id)
    }

    // ── Input dispatch (full pipeline) ───────────────────────────

    /// Dispatch an input event through the full coordinate transform pipeline,
    /// matching `ZuiWindow::dispatch_input`:
    ///
    /// 1. VIF chain filter
    /// 2. Hit test and set active panel (for mouse press)
    /// 3. Transform mouse coords from view space to panel-local space
    /// 4. Keyboard suppression for non-active-path panels
    /// 5. Deliver to behavior
    pub fn dispatch(&mut self, event: &InputEvent) {
        // Run VIF chain
        for vif in &mut self.vif_chain {
            if vif.filter(event, &self.input_state, &mut self.view) {
                return;
            }
        }

        // Tab / Shift+Tab focus cycling (C++ emPanel.cpp FocusNext/FocusPrev)
        if event.key == InputKey::Tab && event.variant == InputVariant::Press {
            if self.input_state.shift() {
                self.view.visit_prev(&mut self.tree);
            } else {
                self.view.visit_next(&mut self.tree);
            }
            return;
        }

        // For mouse press: hit test and set active panel
        if event.variant == InputVariant::Press
            && matches!(
                event.key,
                InputKey::MouseLeft | InputKey::MouseRight | InputKey::MouseMiddle
            )
        {
            let panel = self
                .view
                .get_focusable_panel_at(&self.tree, event.mouse_x, event.mouse_y)
                .unwrap_or_else(|| self.view.root());
            self.view.set_active_panel(&mut self.tree, panel, false);
        }

        // Stamp modifier keys from InputState onto the event
        let ev = event.clone().with_modifiers(&self.input_state);

        // Dispatch to ALL viewed panels in post-order, transforming mouse
        // coordinates to panel-local space for each panel.
        let wf = self.view.window_focused();
        let viewed = self.tree.viewed_panels_dfs();
        for panel_id in viewed {
            // Transform view-space mouse coords to panel-local coords
            let mut panel_ev = ev.clone();
            panel_ev.mouse_x = self.tree.view_to_panel_x(panel_id, ev.mouse_x);
            panel_ev.mouse_y =
                self.tree
                    .view_to_panel_y(panel_id, ev.mouse_y, self.view.pixel_tallness());

            if let Some(mut behavior) = self.tree.take_behavior(panel_id) {
                let panel_state =
                    self.tree
                        .build_panel_state(panel_id, wf, self.view.pixel_tallness());

                // C++ RecurseInput: keyboard events are suppressed for
                // panels not in the active path.
                if panel_ev.is_keyboard_event() && !panel_state.in_active_path {
                    self.tree.put_behavior(panel_id, behavior);
                    continue;
                }

                let consumed = behavior.input(&panel_ev, &panel_state, &self.input_state);
                self.tree.put_behavior(panel_id, behavior);
                if consumed {
                    self.view.invalidate_painting(&self.tree, panel_id);
                    break;
                }
            }
        }
    }

    // ── High-level input helpers ─────────────────────────────────

    /// Click (press + release) the left mouse button at view-space coordinates.
    pub fn click(&mut self, view_x: f64, view_y: f64) {
        let press = InputEvent::press(InputKey::MouseLeft).with_mouse(view_x, view_y);
        let release = InputEvent::release(InputKey::MouseLeft).with_mouse(view_x, view_y);
        self.dispatch(&press);
        self.dispatch(&release);
    }

    /// Drag: press at `from`, move to `to`, release at `to`. All coordinates
    /// are in view space.
    pub fn drag(&mut self, from_x: f64, from_y: f64, to_x: f64, to_y: f64) {
        let press = InputEvent::press(InputKey::MouseLeft).with_mouse(from_x, from_y);
        let move_ev = InputEvent::mouse_move(InputKey::MouseLeft, to_x, to_y);
        let release = InputEvent::release(InputKey::MouseLeft).with_mouse(to_x, to_y);
        self.dispatch(&press);
        self.dispatch(&move_ev);
        self.dispatch(&release);
    }

    /// Press and release a keyboard key.
    pub fn press_key(&mut self, key: InputKey) {
        let press = InputEvent::press(key);
        let release = InputEvent::release(key);
        self.dispatch(&press);
        self.dispatch(&release);
    }

    /// Press and release a character key, including the `chars` field so that
    /// text-input widgets (e.g. TextField) receive the typed character.
    pub fn press_char(&mut self, ch: char) {
        let press = InputEvent::press(InputKey::Key(ch)).with_chars(&ch.to_string());
        let release = InputEvent::release(InputKey::Key(ch));
        self.dispatch(&press);
        self.dispatch(&release);
    }
}
