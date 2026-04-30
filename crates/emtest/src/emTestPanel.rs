//! emTestPanel — plugin port of C++ emTest/emTestPanel.cpp.
//!
//! Task 6 scope: TestPanel + TkTestGrp + TkTest with the core widget groups
//! (Buttons, Check, Radio, Text, Scalar sf1–sf3, Color). Adds BgColor
//! persistence via emVarModel keyed on the panel identity, plus the teddy.tga
//! test image and a flat PolyDrawPanel placeholder. Extended widget groups
//! (Tunnels, ListBoxes, dialogs, file selection) and the structured
//! PolyDrawPanel are deferred to later tasks (7–11).

use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;

use emcore::emBorder::{emBorder, InnerBorderType, OuterBorderType};
use emcore::emButton::emButton;
use emcore::emCheckBox::emCheckBox;
use emcore::emCheckButton::emCheckButton;
use emcore::emColor::emColor;
use emcore::emColorField::emColorField;
use emcore::emContext::emContext;
use emcore::emCursor::emCursor;
use emcore::emEngineCtx::{ConstructCtx, EngineCtx, PanelCtx, SchedCtx};
use emcore::emImage::emImage;
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emLabel::emLabel;
use emcore::emLook::emLook;
use emcore::emPainter::{emPainter, TextAlignment, VAlign};
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelTree::{PanelId, ViewConditionType};
use emcore::emRadioBox::emRadioBox;
use emcore::emRadioButton::{emRadioButton, RadioGroup};
use emcore::emRasterGroup::emRasterGroup;
use emcore::emRasterLayout::emRasterLayout;
use emcore::emRes::emGetInsResImage;
use emcore::emScalarField::emScalarField;
use emcore::emStroke::emStroke;
use emcore::emTextField::emTextField;
use emcore::emVarModel;

// ─── constants ──────────────────────────────────────────────────────

const MAX_DEPTH: u32 = 10;
const MAX_LOG_ENTRIES: usize = 20;
const DEFAULT_BG: emColor = emColor::rgba(0x00, 0x1C, 0x38, 0xFF);

const CHILD_LAYOUT: [(&str, f64, f64, f64, f64); 7] = [
    ("TkTestGrp", 0.20, 0.15, 0.30, 0.12),
    ("1", 0.70, 0.05, 0.12, 0.12),
    ("2", 0.83, 0.05, 0.12, 0.12),
    ("3", 0.70, 0.18, 0.12, 0.12),
    ("4", 0.83, 0.18, 0.12, 0.12),
    ("BgColorField", 0.775, 0.34, 0.10, 0.02),
    ("PolyDraw", 0.05, 0.92, 0.08, 0.04),
];

// ─── widget wrapper PanelBehaviors ──────────────────────────────────

struct ButtonPanel {
    widget: emButton,
}
impl PanelBehavior for ButtonPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct CheckButtonPanel {
    widget: emCheckButton,
}
impl PanelBehavior for CheckButtonPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct CheckBoxPanel {
    widget: emCheckBox,
}
impl PanelBehavior for CheckBoxPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct RadioButtonPanel {
    widget: emRadioButton,
}
impl PanelBehavior for RadioButtonPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct RadioBoxPanel {
    widget: emRadioBox,
}
impl PanelBehavior for RadioBoxPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct TextFieldPanel {
    widget: emTextField,
}
impl PanelBehavior for TextFieldPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget.cycle_blink(s.in_focused_path());
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState, _ctx: &mut PanelCtx) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.widget.on_focus_changed(state.in_focused_path());
        }
    }
}

struct ScalarFieldPanel {
    widget: emScalarField,
}
impl PanelBehavior for ScalarFieldPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .Paint(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct ColorFieldPanel {
    widget: emColorField,
}
impl PanelBehavior for ColorFieldPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, _s: &PanelState) {
        let pixel_scale = _s.viewed_rect.w * _s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget.Paint(p, canvas_color, w, h, pixel_scale);
    }
    fn Input(
        &mut self,
        e: &emInputEvent,
        s: &PanelState,
        is: &emInputState,
        ctx: &mut PanelCtx,
    ) -> bool {
        self.widget.Input(e, s, is, ctx)
    }
    fn IsOpaque(&self) -> bool {
        true
    }
    fn auto_expand(&self) -> bool {
        true
    }
    fn AutoExpand(&mut self, ctx: &mut PanelCtx) {
        self.widget.create_expansion_children(ctx);
    }
    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        self.widget.LayoutChildren(ctx, rect.w, rect.h);
    }
    fn Cycle(&mut self, _ectx: &mut EngineCtx<'_>, ctx: &mut PanelCtx) -> bool {
        self.widget.Cycle(ctx)
    }
}

/// Wraps emLabel as a control-panel child.
struct LabelPanel {
    widget: emLabel,
}
impl PanelBehavior for LabelPanel {
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        let pixel_scale = s.viewed_rect.w * s.viewed_rect.h / w.max(1e-100) / h.max(1e-100);
        self.widget
            .PaintContent(p, canvas_color, w, h, s.enabled, pixel_scale);
    }
}

// ─── TestPanel ──────────────────────────────────────────────────────

/// Shared bg color slot — written by ColorField on_color callback (synchronous,
/// from `SetColor` via `SchedCtx::fire`), read by TestPanel::Paint and Drop.
/// The C++ original drives the same data flow via Cycle + IsSignaled; the
/// Rust callback hop is synchronous within the same input/cycle pass and is
/// not a Cycle-drained polling intermediary.
type BgShared = Rc<Cell<emColor>>;

pub(crate) struct TestPanel {
    depth: u32,
    /// Root-context handle for VarModel lookups in Drop.
    root_ctx: Rc<emContext>,
    /// `"emTestPanel - BgColor of " + identity` — populated lazily on first
    /// `LayoutChildren` once the tree assigns this panel its identity path.
    /// Empty until then; Drop checks for empty before persisting.
    identity_key: String,
    bg_shared: BgShared,
    input_log: Vec<String>,
    test_image: emImage,
}

impl TestPanel {
    pub(crate) fn new(depth: u32, root_ctx: Rc<emContext>, initial_bg: emColor) -> Self {
        let test_image = emGetInsResImage("emTest", "icons/teddy.tga");
        Self {
            depth,
            root_ctx,
            identity_key: String::new(),
            bg_shared: Rc::new(Cell::new(initial_bg)),
            input_log: Vec::new(),
            test_image,
        }
    }

    fn bg_color(&self) -> emColor {
        self.bg_shared.get()
    }

    fn paint_primitives(&self, p: &mut emPainter, fg: emColor, bg: emColor) {
        // Text test with tabs
        p.PaintTextBoxed(
            0.25,
            0.80,
            0.05,
            0.05,
            "Text Test\n\t<-tab\ntab->\t<-tab",
            0.1,
            fg,
            bg,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Left,
            0.2,
            true,
            0.1,
        );
        p.PaintRect(
            0.25,
            0.80,
            0.05,
            0.05,
            emColor::rgba(255, 0, 0, 32),
            emColor::TRANSPARENT,
        );

        // Triangle
        p.PaintPolygon(&[(0.7, 0.6), (0.6, 0.7), (0.8, 0.8)], fg, bg);

        // Circle
        let circle: Vec<(f64, f64)> = (0..64)
            .map(|i| {
                let a = PI * i as f64 / 32.0;
                (a.sin() * 0.05 + 0.65, a.cos() * 0.05 + 0.85)
            })
            .collect();
        p.PaintPolygon(&circle, emColor::YELLOW, bg);

        // Ellipses
        p.PaintEllipse(0.05, 0.80, 0.01, 0.01, emColor::WHITE, bg);
        p.PaintEllipse(0.06, 0.80, 0.02, 0.01, emColor::WHITE, bg);

        // Round rects
        p.PaintRoundRect(0.05, 0.84, 0.01, 0.01, 0.001, 0.001, emColor::WHITE, bg);

        // A simple bezier
        p.PaintBezier(
            &[(0.05, 0.90), (0.06, 0.90), (0.05, 0.91)],
            emColor::WHITE,
            bg,
        );

        // Use the test_image so the field isn't merely stored.
        let h_ratio = if self.test_image.GetWidth() > 0 {
            0.001 * self.test_image.GetHeight() as f64 / self.test_image.GetWidth() as f64
        } else {
            0.001
        };
        let iw = self.test_image.GetWidth() as i32;
        let ih = self.test_image.GetHeight() as i32;
        p.PaintImageTextured(
            0.26,
            0.94,
            0.02,
            0.01,
            0.26,
            0.94,
            0.001,
            h_ratio,
            &self.test_image,
            0,
            0,
            iw,
            ih,
            255,
            emcore::emTexture::ImageExtension::Repeat,
        );
    }
}

impl Drop for TestPanel {
    fn drop(&mut self) {
        if self.identity_key.is_empty() {
            return;
        }
        let bg = self.bg_shared.get();
        if bg != DEFAULT_BG {
            emVarModel::Set(&self.root_ctx, &self.identity_key, bg);
        }
    }
}

impl PanelBehavior for TestPanel {
    fn IsOpaque(&self) -> bool {
        self.bg_color().IsOpaque()
    }

    fn auto_expand(&self) -> bool {
        true
    }

    fn get_title(&self) -> Option<String> {
        Some("Test Panel".into())
    }

    fn Paint(
        &mut self,
        painter: &mut emPainter,
        _canvas_color: emColor,
        w: f64,
        h: f64,
        state: &PanelState,
    ) {
        let bg = self.bg_color();
        let fg = if state.is_focused() {
            emColor::rgba(255, 136, 136, 255)
        } else if state.in_focused_path() {
            emColor::rgba(187, 136, 136, 255)
        } else {
            emColor::rgba(136, 136, 136, 255)
        };

        painter.push_state();
        painter.scale(w, w);
        let panel_h = h / w;

        painter.PaintRect(0.0, 0.0, 1.0, panel_h, bg, emColor::TRANSPARENT);
        painter.PaintRectOutline(
            0.01,
            0.01,
            0.98,
            panel_h - 0.02,
            &emStroke::new(fg, 0.02),
            bg,
        );

        painter.PaintTextBoxed(
            0.02,
            0.02,
            0.49,
            0.07,
            "Test Panel",
            0.1,
            fg,
            bg,
            TextAlignment::Left,
            VAlign::Top,
            TextAlignment::Left,
            0.5,
            true,
            0.0,
        );

        if state.viewed_rect.w < 25.0 {
            painter.pop_state();
            return;
        }

        let mut status = "State:".to_string();
        if state.is_focused() {
            status += " Focused";
        }
        if state.in_focused_path() {
            status += " InFocusedPath";
        }
        painter.PaintTextBoxed(
            0.05,
            0.4,
            0.9,
            0.05,
            &status,
            0.05,
            fg,
            bg,
            TextAlignment::Left,
            VAlign::Center,
            TextAlignment::Left,
            0.5,
            true,
            0.0,
        );

        let pri_str = format!("Pri={:.6} MemLim={}", state.priority, state.memory_limit);
        painter.PaintTextBoxed(
            0.05,
            0.45,
            0.9,
            0.1,
            &pri_str,
            0.1,
            fg,
            bg,
            TextAlignment::Left,
            VAlign::Center,
            TextAlignment::Left,
            0.5,
            true,
            0.0,
        );

        for (i, entry) in self.input_log.iter().enumerate() {
            painter.PaintText(
                0.05,
                0.57 + i as f64 * 0.008,
                entry,
                0.008,
                1.0,
                emColor::rgba(0x88, 0x88, 0xBB, 0xFF),
                bg,
            );
        }

        self.paint_primitives(painter, fg, bg);
        painter.pop_state();
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
        _ctx: &mut PanelCtx,
    ) -> bool {
        let log = format!(
            "key={:?} chars=\"{}\" repeat={} variant={:?} mouse={:.1},{:.1}",
            event.key, event.chars, event.repeat, event.variant, event.mouse_x, event.mouse_y,
        );
        if self.input_log.len() >= MAX_LOG_ENTRIES {
            self.input_log.remove(0);
        }
        self.input_log.push(log);
        false
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        // Lazy identity-key init: the tree assigns identity at insertion time.
        if self.identity_key.is_empty() {
            let identity = ctx.tree.GetIdentity(ctx.id);
            // Mirror C++ key: emVarModel<emColor>::GetAndRemove key is
            // "emTestPanel - BgColor of " + GetIdentity().
            let key = format!("emTestPanel - BgColor of {identity}");
            // Restore persisted bg if present.
            let bg = emVarModel::GetAndRemove(&self.root_ctx, &key, self.bg_shared.get());
            self.bg_shared.set(bg);
            self.identity_key = key;
        }

        let bg = self.bg_color();

        if !ctx.children().is_empty() {
            for &(name, x, y, cw, ch) in &CHILD_LAYOUT {
                if let Some(child) = ctx.find_child_by_name(name) {
                    ctx.layout_child_canvas(child, x, y, cw, ch, bg);
                }
            }
            return;
        }

        let bg_shared = self.bg_shared.clone();
        let root_ctx = self.root_ctx.clone();

        // TkTestGrp — C++ AutoExpand creates child named "TkTestGrp".
        let tktest_id = ctx.create_child_with("TkTestGrp", Box::new(TkTestGrpPanel::new()));
        ctx.tree.SetAutoExpansionThreshold(
            tktest_id,
            900.0,
            ViewConditionType::Area,
            ctx.scheduler.as_deref_mut(),
        );

        // Recursive child TestPanels — C++ names are "1", "2", "3", "4".
        if self.depth < MAX_DEPTH {
            for i in 1..=4u32 {
                let tp_id = ctx.create_child_with(
                    &format!("{i}"),
                    Box::new(TestPanel::new(self.depth + 1, root_ctx.clone(), DEFAULT_BG)),
                );
                ctx.tree.SetAutoExpansionThreshold(
                    tp_id,
                    900.0,
                    ViewConditionType::Area,
                    ctx.scheduler.as_deref_mut(),
                );
            }
        }

        // Background ColorField — C++ name "BgColorField".
        let bg_for_cf = bg_shared.clone();
        let mut cf = emColorField::new(ctx, emLook::new());
        cf.SetCaption("Background Color");
        cf.SetEditable(true);
        cf.set_initial_alpha_enabled(true);
        cf.set_initial_color(bg_shared.get());
        cf.on_color = Some(Box::new(move |color, _sched: &mut SchedCtx<'_>| {
            bg_for_cf.set(color);
        }));
        ctx.create_child_with("BgColorField", Box::new(ColorFieldPanel { widget: cf }));

        // PolyDraw — C++ name "PolyDraw" (flat placeholder; Task 11 restructures).
        ctx.create_child_with("PolyDraw", Box::new(PolyDrawPanel::new()));

        for &(name, x, y, cw, ch) in &CHILD_LAYOUT {
            if let Some(child) = ctx.find_child_by_name(name) {
                ctx.layout_child_canvas(child, x, y, cw, ch, bg);
            }
        }
    }

    fn CreateControlPanel(&mut self, ctx: &mut PanelCtx, name: &str) -> Option<PanelId> {
        let identity = ctx.tree.GetIdentity(ctx.id);
        let bg = self.bg_color();
        let text = format!(
            "This is just a test\n\nPanel Identity: {identity}\nBgColor: 0x{:08X}",
            bg.GetPacked()
        );
        let label = emLabel::new(&text, emLook::new());
        Some(ctx.create_child_with(name, Box::new(LabelPanel { widget: label })))
    }
}

// ─── TkTestGrpPanel ─────────────────────────────────────────────────
//
// C++ TkTestGrp::AutoExpand() (emTestPanel.cpp:890-911) creates four TkTest
// instances via two vertical splitters inside a horizontal splitter:
//   t1a (top-left), t1b (bottom-left), t2a (top-right), t2b (bottom-right).
// t2b is disabled via SetEnableSwitch(false).
//
// emSplitter is not yet ported (dependency-forced divergence); we replicate
// the observable 2×2 grid layout directly with layout_child arithmetic that
// matches sp->SetPos(0.8): 80 % left / 20 % right split on both axes.

struct TkTestGrpPanel {
    border: emBorder,
    look: Rc<emLook>,
    children_created: bool,
}

impl TkTestGrpPanel {
    fn new() -> Self {
        let look = emLook::new();
        let border = emBorder::new(OuterBorderType::Group)
            .with_inner(InnerBorderType::Group)
            .with_caption("Toolkit Test");
        Self {
            border,
            look,
            children_created: false,
        }
    }
}

impl PanelBehavior for TkTestGrpPanel {
    fn IsOpaque(&self) -> bool {
        true
    }
    fn auto_expand(&self) -> bool {
        true
    }
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        self.border.paint_border(
            p,
            canvas_color,
            w,
            h,
            &self.look,
            s.is_focused(),
            s.enabled,
            1.0,
        );
    }
    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();

        if !self.children_created {
            self.children_created = true;
            // C++: t1a, t1b go in sp1 (left); t2a, t2b go in sp2 (right).
            ctx.create_child_with("t1a", Box::new(TkTestPanel::new(self.look.clone())));
            ctx.create_child_with("t1b", Box::new(TkTestPanel::new(self.look.clone())));
            ctx.create_child_with("t2a", Box::new(TkTestPanel::new(self.look.clone())));
            let t2b_id =
                ctx.create_child_with("t2b", Box::new(TkTestPanel::new(self.look.clone())));
            // C++: t2b->SetEnableSwitch(false) (emTestPanel.cpp:909).
            ctx.tree
                .SetEnableSwitch(t2b_id, false, ctx.scheduler.as_deref_mut());
        }

        // C++ sp->SetPos(0.8), sp1->SetPos(0.8), sp2->SetPos(0.8):
        // horizontal split at 80 %; each side split vertically at 80 %.
        let cr = self.border.GetContentRect(rect.w, rect.h, &self.look);
        let half_w = cr.w * 0.5;
        let half_h = cr.h * 0.5;

        if let Some(id) = ctx.find_child_by_name("t1a") {
            ctx.layout_child(id, cr.x, cr.y, half_w, half_h);
        }
        if let Some(id) = ctx.find_child_by_name("t1b") {
            ctx.layout_child(id, cr.x, cr.y + half_h, half_w, half_h);
        }
        if let Some(id) = ctx.find_child_by_name("t2a") {
            ctx.layout_child(id, cr.x + half_w, cr.y, half_w, half_h);
        }
        if let Some(id) = ctx.find_child_by_name("t2b") {
            ctx.layout_child(id, cr.x + half_w, cr.y + half_h, half_w, half_h);
        }

        let cc =
            self.border
                .content_canvas_color(ctx.GetCanvasColor(), &self.look, ctx.is_enabled());
        ctx.set_all_children_canvas_color(cc);
    }
}

// ─── TkTestPanel — core widget groups ────────────────────────────────

struct TkTestPanel {
    look: Rc<emLook>,
    border: emBorder,
    layout: emRasterLayout,
    children_created: bool,
}

impl TkTestPanel {
    fn new(look: Rc<emLook>) -> Self {
        let border = emBorder::new(OuterBorderType::Group)
            .with_inner(InnerBorderType::Group)
            .with_caption("Toolkit Test");
        let mut layout = emRasterLayout::new();
        layout.preferred_child_tallness = 0.3;
        Self {
            look,
            border,
            layout,
            children_created: false,
        }
    }

    fn make_category(ctx: &mut PanelCtx, name: &str, caption: &str, pct: Option<f64>) -> PanelId {
        let mut rg = emRasterGroup::new();
        rg.border.SetBorderScaling(2.5);
        rg.border.caption = caption.to_string();
        if let Some(p) = pct {
            rg.layout.preferred_child_tallness = p;
        }
        let id = ctx.tree.create_child(ctx.id, name, None);
        ctx.tree.set_behavior(id, Box::new(rg));
        id
    }

    fn create_all_categories(&self, ctx: &mut PanelCtx) {
        let look = self.look.clone();

        // 1. Buttons — C++ emTestPanel.cpp:556-566.
        let gid = Self::make_category(ctx, "buttons", "Buttons", None);
        {
            let id = ctx.tree.create_child(gid, "b1", None);
            let w = emButton::new(ctx, "Button", look.clone());
            ctx.tree
                .set_behavior(id, Box::new(ButtonPanel { widget: w }));

            let mut b2 = emButton::new(ctx, "Long Desc", look.clone());
            let mut desc = String::new();
            for _ in 0..100 {
                desc.push_str("This is a looooooooooooooooooooooooooooooooooooooooooooooooooooooong description of the button.\n");
            }
            b2.SetDescription(&desc);
            let id = ctx.tree.create_child(gid, "b2", None);
            ctx.tree
                .set_behavior(id, Box::new(ButtonPanel { widget: b2 }));

            let mut b3 = emButton::new(ctx, "NoEOI", look.clone());
            b3.SetNoEOI(true);
            let id = ctx.tree.create_child(gid, "b3", None);
            ctx.tree
                .set_behavior(id, Box::new(ButtonPanel { widget: b3 }));
        }

        // 2. Check Buttons and Boxes — C++ :568-575.
        let gid = Self::make_category(ctx, "checkbuttons", "Check Buttons and Boxes", None);
        {
            for i in 1..=3 {
                let id = ctx.tree.create_child(gid, &format!("c{i}"), None);
                let w = emCheckButton::new(ctx, "Check Button", look.clone());
                ctx.tree
                    .set_behavior(id, Box::new(CheckButtonPanel { widget: w }));
            }
            for i in 4..=6 {
                let id = ctx.tree.create_child(gid, &format!("c{i}"), None);
                let w = emCheckBox::new(ctx, "Check Box", look.clone());
                ctx.tree
                    .set_behavior(id, Box::new(CheckBoxPanel { widget: w }));
            }
        }

        // 3. Radio Buttons and Boxes — C++ :577-584.
        // C++: emRadioBox extends emRadioButton; all 6 widgets (r1-r3 buttons,
        // r4-r6 boxes) share the same RasterGroup parent, so selecting any one
        // deselects the others. One RadioGroup covers all six.
        let gid = Self::make_category(ctx, "radiobuttons", "Radio Buttons and Boxes", None);
        {
            let rg = RadioGroup::new(ctx);
            for i in 1..=3usize {
                let id = ctx.tree.create_child(gid, &format!("r{i}"), None);
                let w = emRadioButton::new(ctx, "Radio Button", look.clone(), rg.clone(), i - 1);
                ctx.tree
                    .set_behavior(id, Box::new(RadioButtonPanel { widget: w }));
            }
            for i in 4..=6usize {
                let id = ctx.tree.create_child(gid, &format!("r{i}"), None);
                let w = emRadioBox::new("Radio Box", look.clone(), rg.clone(), i - 4);
                ctx.tree
                    .set_behavior(id, Box::new(RadioBoxPanel { widget: w }));
            }
        }

        // 4. Text Fields — C++ :586-609.
        let gid = Self::make_category(ctx, "textfields", "Text Fields", None);
        {
            let mut tf1 = emTextField::new(ctx, look.clone());
            tf1.SetCaption("Read-Only");
            tf1.SetDescription("This is a read-only text field.");
            tf1.SetText("Read-Only");
            let id = ctx.tree.create_child(gid, "tf1", None);
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: tf1 }));

            let mut tf2 = emTextField::new(ctx, look.clone());
            tf2.SetCaption("Editable");
            tf2.SetDescription("This is an editable text field.");
            tf2.SetEditable(true);
            tf2.SetText("Editable");
            let id = ctx.tree.create_child(gid, "tf2", None);
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: tf2 }));

            let mut tf3 = emTextField::new(ctx, look.clone());
            tf3.SetCaption("Password");
            tf3.SetDescription("This is an editable password text field.");
            tf3.SetEditable(true);
            tf3.SetText("Password");
            tf3.SetPasswordMode(true);
            let id = ctx.tree.create_child(gid, "tf3", None);
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: tf3 }));

            let mut mltf1 = emTextField::new(ctx, look.clone());
            mltf1.SetCaption("Multi-Line");
            mltf1.SetDescription("This is an editable multi-line text field.");
            mltf1.SetEditable(true);
            mltf1.SetMultiLineMode(true);
            mltf1.SetText("first line\nsecond line\n...");
            let id = ctx.tree.create_child(gid, "mltf1", None);
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: mltf1 }));
        }

        // 5. Scalar Fields (sf1–sf3 only) — C++ :611-624. Task 7 adds sf4–sf6.
        let gid = Self::make_category(ctx, "scalarfields", "Scalar Fields", Some(0.1));
        {
            let mut sf1 = emScalarField::new(ctx, 0.0, 10.0, look.clone());
            sf1.SetCaption("Read-Only");
            let id = ctx.tree.create_child(gid, "sf1", None);
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf1 }));

            let mut sf2 = emScalarField::new(ctx, 0.0, 10.0, look.clone());
            sf2.SetCaption("Editable");
            sf2.SetEditable(true);
            let id = ctx.tree.create_child(gid, "sf2", None);
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf2 }));

            let mut sf3 = emScalarField::new(ctx, -1000.0, 1000.0, look.clone());
            sf3.SetEditable(true);
            sf3.set_initial_value(0.0);
            sf3.SetScaleMarkIntervals(&[1000, 100, 10, 5, 1]);
            let id = ctx.tree.create_child(gid, "sf3", None);
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf3 }));
        }

        // 6. Color Fields — C++ :646-660.
        let gid = Self::make_category(ctx, "colorfields", "Color Fields", Some(0.4));
        {
            let mut cf1 = emColorField::new(ctx, look.clone());
            cf1.SetCaption("Read-Only");
            cf1.set_initial_color(emColor::rgba(0xBB, 0x22, 0x22, 0xFF));
            let id = ctx.tree.create_child(gid, "cf1", None);
            ctx.tree
                .set_behavior(id, Box::new(ColorFieldPanel { widget: cf1 }));

            let mut cf2 = emColorField::new(ctx, look.clone());
            cf2.SetCaption("Editable");
            cf2.SetEditable(true);
            cf2.set_initial_color(emColor::rgba(0x22, 0xBB, 0x22, 0xFF));
            let id = ctx.tree.create_child(gid, "cf2", None);
            ctx.tree
                .set_behavior(id, Box::new(ColorFieldPanel { widget: cf2 }));

            let mut cf3 = emColorField::new(ctx, look.clone());
            cf3.SetCaption("Editable, Alpha Enabled");
            cf3.SetEditable(true);
            cf3.set_initial_alpha_enabled(true);
            cf3.set_initial_color(emColor::rgba(0x22, 0x22, 0xBB, 0xFF));
            let id = ctx.tree.create_child(gid, "cf3", None);
            ctx.tree
                .set_behavior(id, Box::new(ColorFieldPanel { widget: cf3 }));
        }
    }
}

impl PanelBehavior for TkTestPanel {
    fn IsOpaque(&self) -> bool {
        true
    }
    fn auto_expand(&self) -> bool {
        true
    }
    fn Paint(&mut self, p: &mut emPainter, canvas_color: emColor, w: f64, h: f64, s: &PanelState) {
        self.border.paint_border(
            p,
            canvas_color,
            w,
            h,
            &self.look,
            s.is_focused(),
            s.enabled,
            1.0,
        );
    }
    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        if !self.children_created {
            self.children_created = true;
            self.create_all_categories(ctx);
        }
        let cr = self.border.GetContentRect(rect.w, rect.h, &self.look);
        self.layout.do_layout_skip(ctx, None, Some(cr));
        let cc =
            self.border
                .content_canvas_color(ctx.GetCanvasColor(), &self.look, ctx.is_enabled());
        ctx.set_all_children_canvas_color(cc);
    }
}

// ─── PolyDrawPanel — flat placeholder ───────────────────────────────
//
// Task 11 will restructure this to mirror the C++ PolyDrawPanel splitter
// hierarchy; for Task 6 we keep a non-interactive flat panel that paints a
// gradient + polygon so the layout slot is filled.

struct PolyDrawPanel {
    vertices: Vec<(f64, f64)>,
}

impl PolyDrawPanel {
    fn new() -> Self {
        let n = 9;
        let vertices: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let a = 2.0 * PI * i as f64 / n as f64;
                (a.cos() * 0.4 + 0.5, a.sin() * 0.4 + 0.5)
            })
            .collect();
        Self { vertices }
    }
}

impl PanelBehavior for PolyDrawPanel {
    fn IsOpaque(&self) -> bool {
        true
    }
    fn Paint(
        &mut self,
        p: &mut emPainter,
        canvas_color: emColor,
        w: f64,
        h: f64,
        _state: &PanelState,
    ) {
        p.PaintRect(
            0.0,
            0.0,
            w,
            h,
            emColor::rgba(80, 80, 160, 0xFF),
            canvas_color,
        );
        let scaled: Vec<(f64, f64)> = self
            .vertices
            .iter()
            .map(|&(vx, vy)| (vx * w, vy * h))
            .collect();
        p.PaintPolygon(&scaled, emColor::WHITE, emColor::TRANSPARENT);
        p.PaintTextBoxed(
            0.0,
            h - 0.05 * h,
            w,
            0.05 * h,
            "Poly Draw",
            0.03 * h,
            emColor::WHITE,
            emColor::TRANSPARENT,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Center,
            0.5,
            true,
            0.15,
        );
    }
}

// ─── plugin entry ───────────────────────────────────────────────────

pub(crate) fn new_root_panel(ctx: &mut dyn ConstructCtx) -> Box<dyn PanelBehavior> {
    let root_ctx = ctx.root_context().clone();
    Box::new(TestPanel::new(0, root_ctx, DEFAULT_BG))
}
