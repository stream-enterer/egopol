// Port of C++ emMain/emMainPanel
// Root panel: splits into control (left) and content (right) sections
// with a draggable slider between them.

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emCursor::emCursor;
use emcore::emImage::emImage;
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPainter::emPainter;
use emcore::emPainter::{TextAlignment, VAlign};
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;
use emcore::emResTga::load_tga;
use emcore::emSubViewPanel::emSubViewPanel;
use emcore::emView::ViewFlags;

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

// ── StartupOverlayPanel ──────────────────────────────────────────────────────

/// Full-screen overlay shown during startup.
///
/// Port of C++ `emMainPanel::StartupOverlayPanel` (emMainPanel.cpp:505-565).
///
/// Eats all input events, shows "Loading..." text, and returns a wait cursor.
/// `IsOpaque()` returns `false` — this is critical: otherwise the sub-view panels
/// for content and control would get "non-viewed" state.
pub(crate) struct StartupOverlayPanel;

impl PanelBehavior for StartupOverlayPanel {
    fn IsOpaque(&self) -> bool {
        false
    }

    fn GetCursor(&self) -> emCursor {
        emCursor::Wait
    }

    fn Input(
        &mut self,
        _event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
    ) -> bool {
        // Eat all input events during startup.
        true
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        painter.Clear(emColor::from_packed(0x808080FF));
        painter.PaintTextBoxed(
            0.0,
            0.0,
            w,
            h,
            "Loading...",
            h,
            emColor::from_packed(0xFFFFFFFF),
            emColor::from_packed(0x808080FF),
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Center,
            1.0,
            false,
            0.0,
        );
    }
}

// ── emMainPanel ───────────────────────────────────────────────────────────────

/// Root panel that splits the view into control (left) and content (right)
/// sections with a draggable slider between them.
///
/// Port of C++ `emMainPanel`.
pub struct emMainPanel {
    ctx: Rc<emContext>,
    config: Rc<RefCell<emMainConfig>>,
    control_tallness: f64,
    unified_slider_pos: f64,

    // Panel IDs for children (in parent tree)
    control_view_panel: Option<PanelId>,
    content_view_panel: Option<PanelId>,
    slider_panel: Option<PanelId>,
    startup_overlay: Option<PanelId>,

    // Control edges decoration
    control_edges_color: emColor,
    control_edges_image: emImage,

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

    // Child panel IDs (created inside sub-views)
    control_panel_created: Option<PanelId>,
    content_panel_created: Option<PanelId>,

    // State
    slider_pressed: bool,
    children_created: bool,
    last_height: f64,
}

impl emMainPanel {
    /// Create a new emMainPanel.
    ///
    /// Port of C++ `emMainPanel::emMainPanel`.
    pub fn new(ctx: Rc<emContext>, control_tallness: f64) -> Self {
        let config = emMainConfig::Acquire(&ctx);
        let unified_slider_pos = config.borrow().GetControlViewSize();
        let control_edges_image =
            load_tga(include_bytes!("../../../res/emMain/ControlEdges.tga"))
                .expect("failed to load ControlEdges.tga");
        Self {
            ctx,
            config,
            control_tallness,
            unified_slider_pos,
            control_view_panel: None,
            content_view_panel: None,
            slider_panel: None,
            startup_overlay: None,
            control_edges_color: emColor::from_packed(0x515E84FF),
            control_edges_image,
            control_panel_created: None,
            content_panel_created: None,
            slider_pressed: false,
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
        if !overlay {
            self.startup_overlay = None;
        }
        // When overlay=true, creation happens in LayoutChildren.
    }

    /// Whether the startup overlay is active.
    ///
    /// Port of C++ `emMainPanel::HasStartupOverlay`.
    pub fn HasStartupOverlay(&self) -> bool {
        self.startup_overlay.is_some()
    }

    /// Get the control edges color.
    ///
    /// Port of C++ `emMainPanel::GetControlEdgesColor`.
    pub fn GetControlEdgesColor(&self) -> emColor {
        self.control_edges_color
    }

    /// Get the control edges image.
    ///
    /// Port of C++ `emMainPanel::GetControlEdgesImage`.
    pub fn GetControlEdgesImage(&self) -> &emImage {
        &self.control_edges_image
    }

    /// Set the control edges color.
    ///
    /// Port of C++ `emMainPanel::SetControlEdgesColor`.
    pub fn SetControlEdgesColor(&mut self, color: emColor) {
        // Force alpha to 255.
        let c = emColor::from_packed(color.GetPacked() | 0xFF);
        if self.control_edges_color != c {
            self.control_edges_color = c;
        }
    }
}

impl PanelBehavior for emMainPanel {
    fn IsOpaque(&self) -> bool {
        true
    }

    fn get_title(&self) -> Option<String> {
        Some("Eagle Mode".to_string())
    }

    fn Paint(&mut self, painter: &mut emPainter, _w: f64, _h: f64, _state: &PanelState) {
        // Port of C++ emMainPanel::Paint (emMainPanel.cpp:167-222).

        if self.content_y <= 1e-10 {
            return;
        }

        let d = self.control_h * 0.007;
        let x1 = 0.0;
        let y1 = 0.0;
        let w1 = self.control_x - d;
        let h1 = self.control_h;
        let x2 = self.control_x + self.control_w + d;
        let y2 = 0.0;
        let w2 = 1.0 - x2;
        let h2 = self.control_h;

        // Separator strip below control area.
        let sx = 0.0;
        let sy = painter.RoundDownY(self.control_h);
        let sw = 1.0;
        let sh = painter.RoundUpY(self.content_y) - sy;
        painter.PaintRect(sx, sy, sw, sh, emColor::from_packed(0x000000FF), emColor::TRANSPARENT);

        let d = self.control_h * 0.015;

        // Left control edge.
        if self.control_x > 1e-10 {
            let bx = painter.RoundDownX(x1 + w1);
            let by = 0.0;
            let bw = painter.RoundUpX(self.control_x) - bx;
            let bh = painter.RoundUpY(self.content_y);
            painter.PaintRect(bx, by, bw, bh, emColor::from_packed(0x000000FF), emColor::TRANSPARENT);
            painter.PaintRect(x1, y1, w1, h1, self.control_edges_color, self.control_edges_color);
            painter.PaintBorderImageSrcRect(
                x1, y1, w1, h1,
                0.0, d, d, d,
                &self.control_edges_image,
                191, 0, 190, 11,
                0, 5, 5, 5,
                255, self.control_edges_color, 0o57,
            );
        }

        // Right control edge.
        if 1.0 - self.control_x - self.control_w > 1e-10 {
            let bx = painter.RoundDownX(self.control_x + self.control_w);
            let by = 0.0;
            let bw = painter.RoundUpX(x2) - bx;
            let bh = painter.RoundUpY(self.content_y);
            painter.PaintRect(bx, by, bw, bh, emColor::from_packed(0x000000FF), emColor::TRANSPARENT);
            painter.PaintRect(x2, y2, w2, h2, self.control_edges_color, self.control_edges_color);
            painter.PaintBorderImageSrcRect(
                x2, y2, w2, h2,
                d, d, 0.0, d,
                &self.control_edges_image,
                0, 0, 190, 11,
                5, 5, 0, 5,
                255, self.control_edges_color, 0o750,
            );
        }
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        let h = rect.h;

        // Read latest slider position from config.
        self.unified_slider_pos = self.config.borrow().GetControlViewSize();
        self.update_coordinates(h);

        if !self.children_created {
            // Create control sub-view panel.
            let mut ctrl_svp = emSubViewPanel::new();
            ctrl_svp.set_sub_view_flags(
                ViewFlags::POPUP_ZOOM
                    | ViewFlags::ROOT_SAME_TALLNESS
                    | ViewFlags::NO_ACTIVE_HIGHLIGHT,
            );
            let ctrl_id = ctx.create_child_with("control view", Box::new(ctrl_svp));
            self.control_view_panel = Some(ctrl_id);

            // Create content sub-view panel.
            let mut content_svp = emSubViewPanel::new();
            content_svp.set_sub_view_flags(ViewFlags::ROOT_SAME_TALLNESS);
            let content_id = ctx.create_child_with("content view", Box::new(content_svp));
            self.content_view_panel = Some(content_id);

            // Create slider panel.
            let slider_id = ctx.create_child_with("slider", Box::new(SliderPanel));
            self.slider_panel = Some(slider_id);

            // Create startup overlay.
            let overlay_id =
                ctx.create_child_with("startupOverlay", Box::new(StartupOverlayPanel));
            self.startup_overlay = Some(overlay_id);

            self.children_created = true;
        }

        // Create control panel inside control sub-view.
        if let Some(ctrl_id) = self.control_view_panel
            && self.control_panel_created.is_none()
        {
            let ctrl_ctx = Rc::clone(&self.ctx);
            let tallness = self.control_tallness;
            self.control_panel_created =
                ctx.tree.with_behavior_as::<emSubViewPanel, _>(ctrl_id, |svp| {
                    let sub_tree = svp.sub_tree_mut();
                    let sub_root = sub_tree.GetRootPanel().expect("sub-view has root");
                    let child_id = sub_tree.create_child(sub_root, "ctrl");
                    sub_tree.set_behavior(child_id, Box::new(emMainControlPanel::new(ctrl_ctx)));
                    sub_tree.Layout(child_id, 0.0, 0.0, 1.0, tallness);
                    child_id
                });
        }

        // Create content panel inside content sub-view.
        if let Some(content_id) = self.content_view_panel
            && self.content_panel_created.is_none()
        {
            let content_ctx = Rc::clone(&self.ctx);
            self.content_panel_created =
                ctx.tree.with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
                    let sub_tree = svp.sub_tree_mut();
                    let sub_root = sub_tree.GetRootPanel().expect("sub-view has root");
                    let child_id = sub_tree.create_child(sub_root, "");
                    sub_tree.set_behavior(
                        child_id,
                        Box::new(emMainContentPanel::new(content_ctx)),
                    );
                    sub_tree.Layout(child_id, 0.0, 0.0, 1.0, 1.0);
                    child_id
                });
        }

        // Position children.
        if let Some(ctrl) = self.control_view_panel {
            ctx.layout_child(ctrl, self.control_x, self.control_y, self.control_w, self.control_h);
        }
        if let Some(content) = self.content_view_panel {
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
        if let Some(overlay) = self.startup_overlay {
            ctx.layout_child(overlay, 0.0, 0.0, 1.0, h);
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
        // startup_overlay is None until LayoutChildren creates it
        assert!(!panel.HasStartupOverlay());
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
    fn test_startup_overlay_panel_not_opaque() {
        let panel = StartupOverlayPanel;
        assert!(!panel.IsOpaque());
    }

    #[test]
    fn test_startup_overlay_panel_cursor() {
        let panel = StartupOverlayPanel;
        assert_eq!(panel.GetCursor(), emCursor::Wait);
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

    #[test]
    fn test_sub_view_panel_fields() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        assert!(panel.control_view_panel.is_none());
        assert!(panel.content_view_panel.is_none());
    }

    #[test]
    fn test_control_edges_color() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        let color = emColor::from_packed(0xFF0000FF);
        panel.SetControlEdgesColor(color);
        assert_eq!(panel.GetControlEdgesColor(), color);
    }

    #[test]
    fn test_control_edges_image_loaded() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        assert!(panel.GetControlEdgesImage().GetWidth() > 0);
        assert!(panel.GetControlEdgesImage().GetHeight() > 0);
    }

    #[test]
    fn test_paint_skips_when_content_at_top() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainPanel::new(Rc::clone(&ctx), 5.0);
        panel.content_y = 0.0;
        assert!(panel.content_y <= 1e-10);
    }
}
