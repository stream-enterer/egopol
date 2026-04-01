// Port of C++ emMain/emMainPanel
// Root panel: splits into control (left) and content (right) sections
// with a draggable slider between them.

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPainter::emPainter;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;

use crate::emMainConfig::emMainConfig;
use crate::emMainContentPanel::emMainContentPanel;
use crate::emMainControlPanel::emMainControlPanel;

// ── SliderPanel ───────────────────────────────────────────────────────────────

/// Thin divider panel between control and content sections.
///
/// DIVERGED: C++ `emMainPanel::SliderPanel` supports dragging to resize the
/// split. Rust defers input/drag handling until slider interaction is wired.
pub(crate) struct SliderPanel;

impl PanelBehavior for SliderPanel {
    fn IsOpaque(&self) -> bool {
        true
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        painter.PaintRect(
            0.0,
            0.0,
            w,
            h,
            emColor::from_packed(0x333344FF),
            emColor::TRANSPARENT,
        );
    }
}

// ── emMainPanel ───────────────────────────────────────────────────────────────

/// Root panel that splits the view into control (left) and content (right)
/// sections with a draggable slider between them.
///
/// Port of C++ `emMainPanel`.
///
/// DIVERGED: C++ uses emSubViewPanel for control/content with independent zoom.
/// Rust creates direct child panels without independent views since emSubViewPanel
/// integration with the rendering pipeline is not fully wired yet for nested views.
pub struct emMainPanel {
    ctx: Rc<emContext>,
    config: Rc<RefCell<emMainConfig>>,
    control_tallness: f64,
    unified_slider_pos: f64,
    control_panel: Option<PanelId>,
    content_panel: Option<PanelId>,
    slider_panel: Option<PanelId>,
    startup_overlay: bool,
    children_created: bool,
    // Cached coordinates
    control_x: f64,
    control_y: f64,
    control_w: f64,
    control_h: f64,
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
    slider_x: f64,
    slider_y: f64,
    slider_w: f64,
    slider_h: f64,
    slider_min_y: f64,
    slider_max_y: f64,
    slider_pressed: bool,
    last_height: f64,
}

impl emMainPanel {
    /// Create a new emMainPanel.
    ///
    /// Port of C++ `emMainPanel::emMainPanel`.
    pub fn new(ctx: Rc<emContext>, control_tallness: f64) -> Self {
        let config = emMainConfig::Acquire(&ctx);
        let unified_slider_pos = config.borrow().GetControlViewSize();
        Self {
            ctx,
            config,
            control_tallness,
            unified_slider_pos,
            control_panel: None,
            content_panel: None,
            slider_panel: None,
            startup_overlay: true,
            children_created: false,
            control_x: 0.0,
            control_y: 0.0,
            control_w: 0.0,
            control_h: 0.0,
            content_x: 0.0,
            content_y: 0.0,
            content_w: 0.0,
            content_h: 0.0,
            slider_x: 0.0,
            slider_y: 0.0,
            slider_w: 0.0,
            slider_h: 0.0,
            slider_min_y: 0.0,
            slider_max_y: 0.0,
            slider_pressed: false,
            last_height: 1.0,
        }
    }

    /// Compute all layout coordinates given the panel height.
    ///
    /// Port of C++ `emMainPanel::UpdateCoordinates`.
    fn update_coordinates(&mut self, h: f64) {
        self.slider_min_y = 0.0;
        self.slider_max_y = self.control_tallness.min(h * 0.5);
        self.slider_y =
            (self.slider_max_y - self.slider_min_y) * self.unified_slider_pos + self.slider_min_y;
        self.slider_w = (1.0_f64.min(h) * 0.1).min(1.0_f64.max(h) * 0.02);
        self.slider_h = self.slider_w * 1.2;
        self.slider_x = 1.0 - self.slider_w;

        let space_fac = 1.015;
        let t = self.slider_h * 0.5;
        if self.slider_y < t {
            self.control_h = self.slider_y + self.slider_h * self.slider_y / t;
        } else {
            self.control_h = (self.slider_y + self.slider_h) / space_fac;
        }

        if self.control_h < 1e-5 {
            self.control_h = 1e-5;
            self.control_w = self.control_h / self.control_tallness;
            self.control_x = 0.5 * (1.0 - self.control_w);
            self.control_y = 0.0;
            self.content_x = 0.0;
            self.content_y = 0.0;
            self.content_w = 1.0;
            self.content_h = h;
        } else {
            self.control_w = self.control_h / self.control_tallness;
            self.control_x = ((1.0 - self.control_w) * 0.5).min(self.slider_x - self.control_w);
            self.control_y = 0.0;
            if self.control_x < 1e-5 {
                // Do not hide, because otherwise popping up the control view
                // by keyboard would not work properly.
                self.control_w = 1.0 - self.slider_w;
                self.control_x = 0.0;
                self.control_h = self.control_w * self.control_tallness;
                if self.control_h < self.slider_y {
                    self.control_h = self.slider_y;
                    self.control_w = self.control_h / self.control_tallness;
                } else if !self.slider_pressed {
                    self.slider_y = self.control_h * space_fac - self.slider_h;
                }
            }
            self.content_y = self.control_y + self.control_h * space_fac;
            self.content_x = 0.0;
            self.content_w = 1.0;
            self.content_h = h - self.content_y;
        }

        self.last_height = h;
    }

    /// Show or hide the startup overlay.
    ///
    /// Port of C++ `emMainPanel::SetStartupOverlay`.
    pub fn SetStartupOverlay(&mut self, overlay: bool) {
        self.startup_overlay = overlay;
    }

    /// Whether the startup overlay is active.
    ///
    /// Port of C++ `emMainPanel::HasStartupOverlay`.
    pub fn HasStartupOverlay(&self) -> bool {
        self.startup_overlay
    }
}

impl PanelBehavior for emMainPanel {
    fn IsOpaque(&self) -> bool {
        true
    }

    fn get_title(&self) -> Option<String> {
        Some("Eagle Mode".to_string())
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        // Paint the slider area background (black separator strip on the right).
        let slider_strip_x = self.slider_x;
        painter.PaintRect(
            slider_strip_x,
            0.0,
            w - slider_strip_x,
            h,
            emColor::from_packed(0x000000FF),
            emColor::TRANSPARENT,
        );
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        let h = rect.h;

        // Read latest slider position from config.
        self.unified_slider_pos = self.config.borrow().GetControlViewSize();
        self.update_coordinates(h);

        if !self.children_created {
            // Create control panel.
            let ctrl_ctx = Rc::clone(&self.ctx);
            let ctrl_id = ctx.create_child_with(
                "control",
                Box::new(emMainControlPanel::new(ctrl_ctx)),
            );
            self.control_panel = Some(ctrl_id);

            // Create content panel.
            let content_ctx = Rc::clone(&self.ctx);
            let content_id = ctx.create_child_with(
                "content",
                Box::new(emMainContentPanel::new(content_ctx)),
            );
            self.content_panel = Some(content_id);

            // Create slider panel.
            let slider_id =
                ctx.create_child_with("slider", Box::new(SliderPanel));
            self.slider_panel = Some(slider_id);

            self.children_created = true;
        }

        // Position children.
        if let Some(ctrl) = self.control_panel {
            ctx.layout_child(
                ctrl,
                self.control_x,
                self.control_y,
                self.control_w,
                self.control_h,
            );
        }
        if let Some(content) = self.content_panel {
            ctx.layout_child(
                content,
                self.content_x,
                self.content_y,
                self.content_w,
                self.content_h,
            );
        }
        if let Some(slider) = self.slider_panel {
            ctx.layout_child(
                slider,
                self.slider_x,
                self.slider_y,
                self.slider_w,
                self.slider_h,
            );
        }
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::LAYOUT_CHANGED | NoticeFlags::VIEW_CHANGED) {
            self.unified_slider_pos = self.config.borrow().GetControlViewSize();
            self.update_coordinates(state.height);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        assert!((panel.control_tallness - 5.0).abs() < 1e-10);
        assert!(panel.HasStartupOverlay());
    }

    #[test]
    fn test_update_coordinates() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        panel.update_coordinates(1.0);
        assert!(panel.slider_w > 0.0);
        assert!(panel.slider_h > 0.0);
        assert!(panel.control_w > 0.0);
        assert!(panel.content_w > 0.0);
    }

    #[test]
    fn test_coordinates_content_below_control() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        panel.update_coordinates(1.0);
        assert!(panel.content_y > panel.control_y);
    }

    #[test]
    fn test_title() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        assert_eq!(panel.get_title(), Some("Eagle Mode".to_string()));
    }

    #[test]
    fn test_behavior() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn test_update_coordinates_slider_near_top() {
        // When SliderY < SliderH*0.5, C++ uses: ControlH = SliderY + SliderH * SliderY / t
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        panel.unified_slider_pos = 0.01; // very small → SliderY near 0
        panel.update_coordinates(1.0);
        assert!(panel.control_h > 1e-5);
        assert!(panel.control_h < 0.1);
    }

    #[test]
    fn test_update_coordinates_control_collapsed() {
        // When ControlH < 1E-5, C++ sets ControlH=1E-5 and centers content
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        panel.unified_slider_pos = 0.0; // slider at very top
        panel.update_coordinates(0.001); // very short panel
        assert!(panel.content_h > 0.0);
        assert!(panel.content_x == 0.0);
        assert!(panel.content_w == 1.0);
    }

    #[test]
    fn test_update_coordinates_width_limited() {
        // When ControlX < 1E-5, the C++ branch sets control_w = 1 - slider_w
        // and control_x = 0. To enter this branch we need control_w =
        // control_h / control_tallness large enough that
        // min((1-control_w)*0.5, slider_x - control_w) < 1e-5.
        // control_tallness=0.1 makes control_w ≈ 1.02 (>> 1), guaranteeing entry.
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 0.1);
        panel.unified_slider_pos = 0.8; // slider pushed down
        panel.update_coordinates(1.0);
        // The branch must have been entered: control_x clamped to 0.
        assert_eq!(panel.control_x, 0.0);
        // And control_w set to 1 - slider_w by the branch formula.
        assert!((panel.control_w - (1.0 - panel.slider_w)).abs() < 1e-10);
    }

    #[test]
    fn test_update_coordinates_slider_min_max() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        panel.unified_slider_pos = 0.5;
        panel.update_coordinates(1.0);
        let expected_slider_y = 0.5 * 0.5; // (max-min)*pos + min = 0.5*0.5
        assert!((panel.slider_y - expected_slider_y).abs() < 1e-10);
    }
}
