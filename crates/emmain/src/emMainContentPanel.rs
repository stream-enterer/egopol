// Port of C++ emMain/emMainContentPanel
// Content container: gradient background and eagle logo placement.

use std::rc::Rc;

use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPainter::emPainter;
use emcore::emPanelCtx::PanelCtx;
use emcore::emPanelTree::PanelId;

// ── Eagle coordinate constants ─────────────────────────────────────────────

/// Centre of the eagle shape in its own coordinate system.
const EAGLE_CX: f64 = 78_450.0;
const EAGLE_CY: f64 = 47_690.0;

/// Maximum horizontal scale: 1 pixel per 180 000 units.
const EAGLE_MAX_SCALE: f64 = 1.0 / 180_000.0;
/// Scale denominator when derived from height: height / 120 000.
const EAGLE_HEIGHT_DENOM: f64 = 120_000.0;

// ── emMainContentPanel ────────────────────────────────────────────────────

/// Content container with gradient background and virtual-cosmos panel at
/// the eagle-logo position.
///
/// Port of C++ `emMainContentPanel` from `emMain/emMainContentPanel.cpp`.
pub struct emMainContentPanel {
    ctx: Rc<emContext>,
    cosmos_panel: Option<PanelId>,
    eagle_scale_x: f64,
    eagle_scale_y: f64,
    eagle_shift_x: f64,
    eagle_shift_y: f64,
    /// Cached panel height from the last notice/layout call.
    last_height: f64,
}

impl emMainContentPanel {
    /// Create a new `emMainContentPanel`.
    ///
    /// Port of C++ `emMainContentPanel` constructor.
    pub fn new(ctx: Rc<emContext>) -> Self {
        Self {
            ctx,
            cosmos_panel: None,
            eagle_scale_x: EAGLE_MAX_SCALE,
            eagle_scale_y: EAGLE_MAX_SCALE,
            eagle_shift_x: 0.5 - EAGLE_MAX_SCALE * EAGLE_CX,
            eagle_shift_y: 0.5 - EAGLE_MAX_SCALE * EAGLE_CY,
            last_height: 1.0,
        }
    }

    /// Recompute eagle coordinate transform from the panel height.
    ///
    /// Port of C++ `emMainContentPanel::UpdateCoordinates`.
    ///
    /// ```
    /// EagleScaleX = emMin(1/180000, height/120000);
    /// EagleScaleY = EagleScaleX;
    /// EagleShiftX = 0.5 - EagleScaleX * 78450;
    /// EagleShiftY = height * 0.5 - EagleScaleY * 47690;
    /// ```
    pub fn update_coordinates(&mut self, height: f64) {
        let scale = EAGLE_MAX_SCALE.min(height / EAGLE_HEIGHT_DENOM);
        self.eagle_scale_x = scale;
        self.eagle_scale_y = scale;
        self.eagle_shift_x = 0.5 - scale * EAGLE_CX;
        self.eagle_shift_y = height * 0.5 - scale * EAGLE_CY;
        self.last_height = height;
    }
}

impl PanelBehavior for emMainContentPanel {
    fn IsOpaque(&self) -> bool {
        true
    }

    fn get_title(&self) -> Option<String> {
        Some("Eagle".to_string())
    }

    fn GetIconFileName(&self) -> Option<String> {
        Some("virtual_cosmos.tga".to_string())
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::LAYOUT_CHANGED | NoticeFlags::VIEW_CHANGED) {
            self.update_coordinates(state.height);
        }
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        let top_color = emColor::from_packed(0x91ABF2FF); // emColor(145, 171, 242)
        let bot_color = emColor::from_packed(0xE1DDB7FF); // emColor(225, 221, 183)
        let canvas = emColor::from_packed(0x000000FF);

        // Gradient background: top blue → bottom gold.
        painter.paint_linear_gradient(0.0, 0.0, w, h, top_color, bot_color, false, canvas);

        // Eagle logo placeholder: "Eagle Mode" text centered at the eagle position.
        // DIVERGED: C++ PaintEagle — the full polygon eagle shape (hundreds of
        // coordinate pairs) is replaced with a text label until the polygon data
        // is ported.
        let lx = EAGLE_CX * self.eagle_scale_x + self.eagle_shift_x;
        let ly = EAGLE_CY * self.eagle_scale_y + self.eagle_shift_y;
        let font_h = (self.eagle_scale_x * 8_000.0).max(0.008).min(h * 0.05);
        painter.PaintText(
            lx,
            ly - font_h * 0.5,
            "Eagle Mode",
            font_h,
            1.0,
            emColor::from_packed(0xFFFFFFFF),
            emColor::TRANSPARENT,
        );
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        // Compute cosmos panel position using the eagle-coordinate transform.
        // Port of C++ emMainContentPanel::UpdateChildLayout.
        let sz = 40.0;
        let cx = (EAGLE_CX - sz * 0.5) * self.eagle_scale_x + self.eagle_shift_x;
        let cy = (EAGLE_CY - sz * 0.5) * self.eagle_scale_y + self.eagle_shift_y;
        let cw = sz * self.eagle_scale_x;
        let ch = sz * self.eagle_scale_y;
        let canvas = emColor::from_packed(0x000000FF);

        // Create cosmos panel lazily (C++ constructor creates it immediately, but
        // we need PanelCtx to attach it to the tree).
        let cosmos_id = match self.cosmos_panel {
            Some(id) => id,
            None => {
                let cosmos = Box::new(
                    crate::emVirtualCosmos::emVirtualCosmosPanel::new(Rc::clone(&self.ctx)),
                );
                // C++ uses child name "".
                let id = ctx.create_child_with("", cosmos);
                self.cosmos_panel = Some(id);
                id
            }
        };

        ctx.layout_child_canvas(cosmos_id, cx, cy, cw, ch, canvas);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainContentPanel::new(Rc::clone(&ctx));
        assert!(panel.cosmos_panel.is_none()); // created lazily in LayoutChildren
    }

    #[test]
    fn test_update_coordinates() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainContentPanel::new(Rc::clone(&ctx));
        panel.update_coordinates(1.0); // height=1.0
        // EagleScaleX = min(1/180000, 1/120000) = 1/180000
        let expected = 1.0 / 180_000.0;
        assert!((panel.eagle_scale_x - expected).abs() < 1e-15);
    }

    #[test]
    fn test_update_coordinates_tall() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainContentPanel::new(Rc::clone(&ctx));
        panel.update_coordinates(2.0); // tall panel
        // EagleScaleX = min(1/180000, 2/120000) = 1/180000
        let expected = 1.0 / 180_000.0;
        assert!((panel.eagle_scale_x - expected).abs() < 1e-15);
    }

    #[test]
    fn test_update_coordinates_wide() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainContentPanel::new(Rc::clone(&ctx));
        // height/120000 < 1/180000 when height < 120000/180000 = 0.6667
        // So for height=0.5: min(1/180000, 0.5/120000=1/240000) = 1/240000
        panel.update_coordinates(0.5);
        let expected = 0.5 / 120_000.0;
        assert!((panel.eagle_scale_x - expected).abs() < 1e-20);
    }

    #[test]
    fn test_title() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainContentPanel::new(Rc::clone(&ctx));
        assert_eq!(panel.get_title(), Some("Eagle".to_string()));
    }

    #[test]
    fn test_opaque() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainContentPanel::new(Rc::clone(&ctx));
        assert!(panel.IsOpaque());
    }

    #[test]
    fn test_icon_file() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emMainContentPanel::new(Rc::clone(&ctx));
        assert_eq!(panel.GetIconFileName(), Some("virtual_cosmos.tga".to_string()));
    }

    #[test]
    fn test_shift_at_unit_height() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emMainContentPanel::new(Rc::clone(&ctx));
        panel.update_coordinates(1.0);
        let s = panel.eagle_scale_x;
        let expected_sx = 0.5 - s * EAGLE_CX;
        let expected_sy = 0.5 - s * EAGLE_CY;
        assert!((panel.eagle_shift_x - expected_sx).abs() < 1e-15);
        assert!((panel.eagle_shift_y - expected_sy).abs() < 1e-15);
    }
}
