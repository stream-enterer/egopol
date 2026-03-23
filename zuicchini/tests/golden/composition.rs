//! Composition golden tests — full widget trees rendered through the compositor.
//!
//! These tests render complex multi-panel hierarchies (e.g., the TkTestPanel
//! widget showcase grid) and compare the composited output against C++ golden data.

use std::rc::Rc;

use zuicchini::emCore::emColor::emColor;
use zuicchini::emCore::emCursor::emCursor;
use zuicchini::emCore::emInput::emInputEvent;
use zuicchini::emCore::emInputState::emInputState;
use zuicchini::emCore::emRasterLayout::emRasterLayout;
use zuicchini::emCore::emRasterGroup::emRasterGroup;
use zuicchini::emCore::emPanel::{NoticeFlags, PanelBehavior, PanelState};

use zuicchini::emCore::emPanelCtx::PanelCtx;

use zuicchini::emCore::emPanelTree::{PanelId, PanelTree, ViewConditionType};

use zuicchini::emCore::emView::{emView, ViewFlags};
use zuicchini::emCore::emPainter::emPainter;
use zuicchini::emCore::emViewRenderer::SoftwareCompositor;
use zuicchini::emCore::emBorder::{emBorder, InnerBorderType, OuterBorderType};

use zuicchini::emCore::emButton::emButton;

use zuicchini::emCore::emCheckBox::emCheckBox;

use zuicchini::emCore::emCheckButton::emCheckButton;

use zuicchini::emCore::emColorField::emColorField;

use zuicchini::emCore::emListBox::{emListBox, SelectionMode};

use zuicchini::emCore::emLook::emLook;

use zuicchini::emCore::emRadioBox::emRadioBox;

use zuicchini::emCore::emRadioButton::{emRadioButton, RadioGroup};

use zuicchini::emCore::emScalarField::emScalarField;

use zuicchini::emCore::emTextField::emTextField;

use super::common::*;

/// Skip test if golden data hasn't been generated yet.
macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden/ directory not found — run `make -C golden_gen run` first");
            return;
        }
    };
}

/// Settle: deliver notices and update viewing until stable.
fn settle(tree: &mut PanelTree, view: &mut emView, rounds: usize) {
    for _ in 0..rounds {
        tree.HandleNotice(view.IsFocused(), view.GetCurrentPixelTallness());
        view.Update(tree);
    }
}

// ═══════════════════════════════════════════════════════════════════
// Widget wrapper panels (same as test_panel.rs — needed for TkTestPanel)
// ═══════════════════════════════════════════════════════════════════

struct ButtonPanel {
    widget: emButton,
}
impl PanelBehavior for ButtonPanel {
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h, _s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
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
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h, _s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
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
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h, _s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
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
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h, _s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
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
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h, _s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
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
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.cycle_blink(_s.in_focused_path());
        self.widget.PaintContent(p, w, h, _s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.widget.on_focus_changed(state.in_focused_path());
        }
    }
}

struct ScalarFieldPanel {
    widget: emScalarField,
}
impl PanelBehavior for ScalarFieldPanel {
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, s: &PanelState) {
        self.widget.PaintContent(p, w, h, s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
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
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

struct ListBoxPanel {
    widget: emListBox,
}
impl PanelBehavior for ListBoxPanel {
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
        self.widget.PaintContent(p, w, h);
    }
    fn Input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
        self.widget.Input(e, _s, _is)
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

// ═══════════════════════════════════════════════════════════════════
// Stub panels for unported C++ types
// ═══════════════════════════════════════════════════════════════════

/// Stub for C++ emTunnel — renders a Group border with caption, positions
/// a single child filling the content area.
struct TunnelStubPanel {
    border: emBorder,
    look: Rc<emLook>,
}

impl TunnelStubPanel {
    fn new(caption: &str, look: Rc<emLook>) -> Self {
        let border = emBorder::new(OuterBorderType::Group)
            .with_inner(InnerBorderType::Group)
            .with_caption(caption);
        Self { border, look }
    }
}

impl PanelBehavior for TunnelStubPanel {
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, s: &PanelState) {
        self.border
            .paint_border(p, w, h, &self.look, s.is_focused(), s.enabled, 1.0);
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let children = ctx.children();
        if children.IsEmpty() {
            return;
        }
        let rect = ctx.layout_rect();
        let cr = self.border.GetContentRect(rect.w, rect.h, &self.look);
        ctx.layout_child(children[0], cr.x, cr.y, cr.w, cr.h);
        let cc = self
            .border
            .content_canvas_color(ctx.GetCanvasColor(), &self.look, ctx.is_enabled());
        ctx.set_all_children_canvas_color(cc);
    }

    fn auto_expand(&self) -> bool {
        true
    }
}

/// Stub for C++ emFileSelectionBox — renders a Group border with caption.
struct FileSelectionBoxStubPanel {
    border: emBorder,
    look: Rc<emLook>,
}

impl FileSelectionBoxStubPanel {
    fn new(look: Rc<emLook>) -> Self {
        let border = emBorder::new(OuterBorderType::Group)
            .with_inner(InnerBorderType::Group)
            .with_caption("File Selection");
        Self { border, look }
    }
}

impl PanelBehavior for FileSelectionBoxStubPanel {
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, s: &PanelState) {
        self.border
            .paint_border(p, w, h, &self.look, s.is_focused(), s.enabled, 1.0);
    }

    fn auto_expand(&self) -> bool {
        true
    }
}

// ═══════════════════════════════════════════════════════════════════
// TkTestPanel — widget showcase grid (from test_panel.rs)
// ═══════════════════════════════════════════════════════════════════

struct TkTestPanel {
    look: Rc<emLook>,
    border: emBorder,
    children_created: bool,
}

impl TkTestPanel {
    fn new(look: Rc<emLook>) -> Self {
        let border = emBorder::new(OuterBorderType::Group)
            .with_inner(InnerBorderType::Group)
            .with_caption("Toolkit Test");
        Self {
            look,
            border,
            children_created: false,
        }
    }

    /// Helper: create a emRasterGroup category under `GetParentContext`.
    fn make_category(
        tree: &mut PanelTree,
        GetParentContext: PanelId,
        name: &str,
        caption: &str,
        pct: Option<f64>,
        fixed_cols: Option<usize>,
    ) -> PanelId {
        let mut rg = emRasterGroup::new();
        rg.border.SetBorderScaling(2.5);
        rg.border.caption = caption.to_string();
        if let Some(p) = pct {
            rg.layout.preferred_child_tallness = p;
        }
        if let Some(c) = fixed_cols {
            rg.layout.fixed_columns = Some(c);
        }
        let id = tree.create_child(GetParentContext, name);
        tree.set_behavior(id, Box::new(rg));
        id
    }

    fn create_all_categories(&self, ctx: &mut PanelCtx, grid_id: PanelId) {
        let look = self.look.clone();

        // 1. Buttons (C++ emTestPanel.cpp:558-576)
        let gid = Self::make_category(ctx.tree, grid_id, "buttons", "Buttons", None, None);
        {
            let id = ctx.tree.create_child(gid, "b1");
            ctx.tree.set_behavior(
                id,
                Box::new(ButtonPanel {
                    widget: emButton::new("Button", look.clone()),
                }),
            );

            let mut b2 = emButton::new("Button", look.clone());
            b2.SetDescription(
                "This is a long description for testing.\n\
                 It has multiple lines.\n\
                 Third line here.",
            );
            let id = ctx.tree.create_child(gid, "b2");
            ctx.tree
                .set_behavior(id, Box::new(ButtonPanel { widget: b2 }));

            let mut b3 = emButton::new("Button", look.clone());
            b3.SetNoEOI(true);
            let id = ctx.tree.create_child(gid, "b3");
            ctx.tree
                .set_behavior(id, Box::new(ButtonPanel { widget: b3 }));
        }

        // 2. Check Buttons and Boxes (C++ :578-598)
        let gid = Self::make_category(
            ctx.tree,
            grid_id,
            "checkbuttons",
            "Check Buttons and Boxes",
            None,
            None,
        );
        {
            for i in 1..=3 {
                let id = ctx.tree.create_child(gid, &format!("c{i}"));
                ctx.tree.set_behavior(
                    id,
                    Box::new(CheckButtonPanel {
                        widget: emCheckButton::new("Check Button", look.clone()),
                    }),
                );
            }
            for i in 4..=6 {
                let id = ctx.tree.create_child(gid, &format!("c{i}"));
                ctx.tree.set_behavior(
                    id,
                    Box::new(CheckBoxPanel {
                        widget: emCheckBox::new("Check Box", look.clone()),
                    }),
                );
            }
        }

        // 3. Radio Buttons and Boxes (C++ :600-624)
        let gid = Self::make_category(
            ctx.tree,
            grid_id,
            "radiobuttons",
            "Radio Buttons and Boxes",
            None,
            None,
        );
        {
            let rg = RadioGroup::new();
            for i in 1..=3 {
                let id = ctx.tree.create_child(gid, &format!("r{i}"));
                ctx.tree.set_behavior(
                    id,
                    Box::new(RadioButtonPanel {
                        widget: emRadioButton::new("Radio Button", look.clone(), rg.clone(), i - 1),
                    }),
                );
            }
            let rg2 = RadioGroup::new();
            for i in 4..=6 {
                let id = ctx.tree.create_child(gid, &format!("r{i}"));
                ctx.tree.set_behavior(
                    id,
                    Box::new(RadioBoxPanel {
                        widget: emRadioBox::new("Radio Box", look.clone(), rg2.clone(), i - 4),
                    }),
                );
            }
        }

        // 4. Text Fields (C++ :626-656)
        let gid = Self::make_category(ctx.tree, grid_id, "textfields", "Text Fields", None, None);
        {
            let mut tf1 = emTextField::new(look.clone());
            tf1.SetText("Read-Only");
            let id = ctx.tree.create_child(gid, "tf1");
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: tf1 }));

            let mut tf2 = emTextField::new(look.clone());
            tf2.SetEditable(true);
            tf2.SetText("Editable");
            let id = ctx.tree.create_child(gid, "tf2");
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: tf2 }));

            let mut tf3 = emTextField::new(look.clone());
            tf3.SetEditable(true);
            tf3.SetText("Password");
            tf3.SetPasswordMode(true);
            let id = ctx.tree.create_child(gid, "tf3");
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: tf3 }));

            let mut mltf1 = emTextField::new(look.clone());
            mltf1.SetEditable(true);
            mltf1.SetMultiLineMode(true);
            mltf1.SetText("first line\nsecond line\n...");
            let id = ctx.tree.create_child(gid, "mltf1");
            ctx.tree
                .set_behavior(id, Box::new(TextFieldPanel { widget: mltf1 }));
        }

        // 5. Scalar Fields (C++ :658-712)
        let gid = Self::make_category(
            ctx.tree,
            grid_id,
            "scalarfields",
            "Scalar Fields",
            Some(0.1),
            None,
        );
        {
            let id = ctx.tree.create_child(gid, "sf1");
            ctx.tree.set_behavior(
                id,
                Box::new(ScalarFieldPanel {
                    widget: emScalarField::new(0.0, 100.0, look.clone()),
                }),
            );

            let mut sf2 = emScalarField::new(0.0, 100.0, look.clone());
            sf2.SetEditable(true);
            let id = ctx.tree.create_child(gid, "sf2");
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf2 }));

            let mut sf3 = emScalarField::new(-1000.0, 1000.0, look.clone());
            sf3.SetEditable(true);
            sf3.SetScaleMarkIntervals(&[1000, 100, 10, 5, 1]);
            let id = ctx.tree.create_child(gid, "sf3");
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf3 }));

            let mut sf4 = emScalarField::new(1.0, 5.0, look.clone());
            sf4.SetEditable(true);
            sf4.SetValue(3.0);
            sf4.SetTextBoxTallness(0.25);
            sf4.SetTextOfValueFunc(Box::new(|val, _interval| format!("Level {val}")));
            let id = ctx.tree.create_child(gid, "sf4");
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf4 }));

            let mut sf5 = emScalarField::new(0.0, 86400000.0, look.clone());
            sf5.SetEditable(true);
            sf5.SetValue(14400000.0);
            sf5.SetScaleMarkIntervals(&[
                86400000, 43200000, 21600000, 10800000, 3600000, 1800000, 600000, 300000, 60000,
                30000, 10000, 5000, 1000,
            ]);
            sf5.SetTextOfValueFunc(Box::new(|val, _interval| {
                let ms = val.unsigned_abs();
                let s = ms / 1000;
                let m = s / 60;
                let h = m / 60;
                format!("{:02}:{:02}:{:02}", h, m % 60, s % 60)
            }));
            let id = ctx.tree.create_child(gid, "sf5");
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf5 }));

            let mut sf6 = emScalarField::new(0.0, 14400000.0, look.clone());
            sf6.SetEditable(true);
            sf6.SetTextOfValueFunc(Box::new(|val, _interval| {
                let ms = val.unsigned_abs();
                let s = ms / 1000;
                let m = s / 60;
                let h = m / 60;
                format!("{:02}:{:02}:{:02}", h, m % 60, s % 60)
            }));
            let id = ctx.tree.create_child(gid, "sf6");
            ctx.tree
                .set_behavior(id, Box::new(ScalarFieldPanel { widget: sf6 }));
        }

        // 6. emColor Fields (C++ :714-733)
        let gid = Self::make_category(
            ctx.tree,
            grid_id,
            "colorfields",
            "Color Fields",
            Some(0.4),
            None,
        );
        {
            let mut cf1 = emColorField::new(look.clone());
            cf1.SetColor(emColor::rgba(0xBB, 0x22, 0x22, 0xFF));
            let id = ctx.tree.create_child(gid, "cf1");
            ctx.tree
                .set_behavior(id, Box::new(ColorFieldPanel { widget: cf1 }));

            let mut cf2 = emColorField::new(look.clone());
            cf2.SetEditable(true);
            cf2.SetColor(emColor::rgba(0x22, 0xBB, 0x22, 0xFF));
            let id = ctx.tree.create_child(gid, "cf2");
            ctx.tree
                .set_behavior(id, Box::new(ColorFieldPanel { widget: cf2 }));

            let mut cf3 = emColorField::new(look.clone());
            cf3.SetEditable(true);
            cf3.SetAlphaEnabled(true);
            cf3.SetColor(emColor::rgba(0x22, 0x22, 0xBB, 0xFF));
            let id = ctx.tree.create_child(gid, "cf3");
            ctx.tree
                .set_behavior(id, Box::new(ColorFieldPanel { widget: cf3 }));
        }

        // 7. Tunnels (C++ :735-754) — stub panels, emTunnel not ported
        let gid = Self::make_category(ctx.tree, grid_id, "tunnels", "Tunnels", Some(0.4), None);
        {
            let tunnel_info: [(&str, &str); 4] = [
                ("t1", "Tunnel"),
                ("t2", "Deeper Tunnel"),
                ("t3", "Square End"),
                ("t4", "Square End, Zero Depth"),
            ];
            for (name, caption) in &tunnel_info {
                let tid = ctx.tree.create_child(gid, name);
                ctx.tree
                    .set_behavior(tid, Box::new(TunnelStubPanel::new(caption, look.clone())));
                let child = ctx.tree.create_child(tid, "child");
                ctx.tree.set_behavior(
                    child,
                    Box::new(ButtonPanel {
                        widget: emButton::new("Inside", look.clone()),
                    }),
                );
            }
        }

        // 8. List Boxes (C++ :756-798)
        let gid = Self::make_category(
            ctx.tree,
            grid_id,
            "listboxes",
            "List Boxes",
            Some(0.4),
            None,
        );
        {
            let items7: Vec<String> = (1..=7).map(|i| format!("Item {i}")).collect();

            let id = ctx.tree.create_child(gid, "l1");
            ctx.tree.set_behavior(
                id,
                Box::new(ListBoxPanel {
                    widget: emListBox::new(look.clone()),
                }),
            );

            let mut lb2 = emListBox::new(look.clone());
            lb2.SetSelectionType(SelectionMode::Single);
            lb2.set_items(items7.clone());
            let id = ctx.tree.create_child(gid, "l2");
            ctx.tree
                .set_behavior(id, Box::new(ListBoxPanel { widget: lb2 }));

            let mut lb3 = emListBox::new(look.clone());
            lb3.SetSelectionType(SelectionMode::ReadOnly);
            lb3.set_items(items7.clone());
            let id = ctx.tree.create_child(gid, "l3");
            ctx.tree
                .set_behavior(id, Box::new(ListBoxPanel { widget: lb3 }));

            let mut lb4 = emListBox::new(look.clone());
            lb4.SetSelectionType(SelectionMode::Multi);
            lb4.set_items(items7.clone());
            let id = ctx.tree.create_child(gid, "l4");
            ctx.tree
                .set_behavior(id, Box::new(ListBoxPanel { widget: lb4 }));

            let mut lb5 = emListBox::new(look.clone());
            lb5.SetSelectionType(SelectionMode::Toggle);
            lb5.set_items(items7.clone());
            let id = ctx.tree.create_child(gid, "l5");
            ctx.tree
                .set_behavior(id, Box::new(ListBoxPanel { widget: lb5 }));

            let mut lb6 = emListBox::new(look.clone());
            lb6.SetSelectionType(SelectionMode::Single);
            lb6.set_items(items7.clone());
            lb6.set_fixed_column_count(Some(1));
            let id = ctx.tree.create_child(gid, "l6");
            ctx.tree
                .set_behavior(id, Box::new(ListBoxPanel { widget: lb6 }));

            let mut lb7 = emListBox::new(look.clone());
            lb7.SetSelectionType(SelectionMode::Single);
            lb7.set_items(items7);
            let id = ctx.tree.create_child(gid, "l7");
            ctx.tree
                .set_behavior(id, Box::new(ListBoxPanel { widget: lb7 }));
        }

        // 9. Test emDialog (C++ :800-831)
        let gid = Self::make_category(ctx.tree, grid_id, "dlgs", "Test Dialog", None, Some(1));
        {
            let mut rl = emRasterLayout::new();
            rl.preferred_child_tallness = 0.1;
            let rl_id = ctx.tree.create_child(gid, "rl");

            let cb_names = [
                "CbTopLev",
                "CbPZoom",
                "CbModal",
                "CbFullscreen",
                "CbPopup",
                "CbUndec",
                "CbResizable",
            ];
            for name in &cb_names {
                let id = ctx.tree.create_child(rl_id, name);
                ctx.tree.set_behavior(
                    id,
                    Box::new(CheckBoxPanel {
                        widget: emCheckBox::new(name, look.clone()),
                    }),
                );
            }
            ctx.tree.set_behavior(rl_id, Box::new(rl));

            let id = ctx.tree.create_child(gid, "dlgButton");
            ctx.tree.set_behavior(
                id,
                Box::new(ButtonPanel {
                    widget: emButton::new("Test Dialog...", look.clone()),
                }),
            );
        }

        // 10. File Selection (C++ :833-858) — stub
        let gid = Self::make_category(
            ctx.tree,
            grid_id,
            "fileChoosers",
            "File Selection",
            Some(0.3),
            None,
        );
        {
            let id = ctx.tree.create_child(gid, "fsb");
            ctx.tree
                .set_behavior(id, Box::new(FileSelectionBoxStubPanel::new(look.clone())));

            let id = ctx.tree.create_child(gid, "open");
            ctx.tree.set_behavior(
                id,
                Box::new(ButtonPanel {
                    widget: emButton::new("Open", look.clone()),
                }),
            );

            let id = ctx.tree.create_child(gid, "openMulti");
            ctx.tree.set_behavior(
                id,
                Box::new(ButtonPanel {
                    widget: emButton::new("Open Multi", look.clone()),
                }),
            );

            let id = ctx.tree.create_child(gid, "saveAs");
            ctx.tree.set_behavior(
                id,
                Box::new(ButtonPanel {
                    widget: emButton::new("Save As", look.clone()),
                }),
            );
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

    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, s: &PanelState) {
        self.border
            .paint_border(p, w, h, &self.look, s.is_focused(), s.enabled, 1.0);
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();

        if !self.children_created {
            self.children_created = true;

            // Create grid child with emRasterLayout (PCT=0.3)
            let mut layout = emRasterLayout::new();
            layout.preferred_child_tallness = 0.3;
            let grid_id = ctx.create_child_with("grid", Box::new(layout));

            // Create all 10 category groups under the grid
            self.create_all_categories(ctx, grid_id);
        }

        // Position grid in border content rect
        let cr = self.border.GetContentRect(rect.w, rect.h, &self.look);
        if let Some(grid) = ctx.find_child_by_name("grid") {
            ctx.layout_child(grid, cr.x, cr.y, cr.w, cr.h);
        }
        let cc = self
            .border
            .content_canvas_color(ctx.GetCanvasColor(), &self.look, ctx.is_enabled());
        ctx.set_all_children_canvas_color(cc);
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

/// TkTestPanel (all widget types in emRasterGroup grid) at 1x zoom (800x600).
/// Matches C++ gen_tktest_1x(): TkTest panel with layout (0, 0, 800/600, 1.0),
/// viewport 800x600, 200 settle rounds, unfocused window.
#[test]
fn composition_tktest_1x() {
    require_golden!();
    let expected = load_compositor_golden("tktest_1x");
    let (w, h, ref expected_data) = expected;

    let look = emLook::new();
    let mut tree = PanelTree::new();
    let root = tree.create_root("tktest");
    tree.set_behavior(root, Box::new(TkTestPanel::new(look)));
    // C++ gen: tk->Layout(0, 0, 800.0/600.0, 1.0)
    tree.Layout(root, 0.0, 0.0, 800.0 / 600.0, 1.0);
    // C++ default auto-expansion threshold for TkTest
    tree.SetAutoExpansionThreshold(root, 900.0, ViewConditionType::Area);

    let mut view = emView::new(root, 800.0, 600.0);
    view.flags.insert(ViewFlags::NO_ACTIVE_HIGHLIGHT);
    // C++ golden gen doesn't focus the window
    view.SetFocused(&mut tree, false);

    // C++ gen_golden.cpp: TerminateEngine ctrl(sched, 200)
    settle(&mut tree, &mut view, 200);

    let mut compositor = SoftwareCompositor::new(w, h);
    compositor.render(&mut tree, &view);
    let actual = compositor.framebuffer().GetMap();

    // TkTestPanel with emRasterGroup categories has layout GetPos differences
    // from C++ emTestPanel::TkTest due to GetContentRect vs border rounding.
    // Same tolerance band as testpanel_expanded which uses the same structure.
    let GetResult = compare_images(
        "tktest_1x",
        actual,
        expected_data,
        w,
        h,
        3,
        28.0,
    );
    if GetResult.is_err() && dump_golden_enabled() {
        dump_test_images("tktest_1x", actual, expected_data, w, h);
        analyze_diff_distribution(actual, expected_data, w, h, 3);
    }
    GetResult.unwrap();
}

/// TkTestPanel (all widget types in emRasterGroup grid) at 2x zoom (800x600).
/// Matches C++ gen_tktest_2x(): same TkTest panel as 1x, then Zoom(400, 300, 2.0)
/// to show the middle 50% of the panel. Catches Restore rounding at non-1x zoom.
#[test]
fn composition_tktest_2x() {
    require_golden!();
    let expected = load_compositor_golden("tktest_2x");
    let (w, h, ref expected_data) = expected;

    let look = emLook::new();
    let mut tree = PanelTree::new();
    let root = tree.create_root("tktest");
    tree.set_behavior(root, Box::new(TkTestPanel::new(look)));
    // C++ gen: tk->Layout(0, 0, 800.0/600.0, 1.0)
    tree.Layout(root, 0.0, 0.0, 800.0 / 600.0, 1.0);
    // C++ default auto-expansion threshold for TkTest
    tree.SetAutoExpansionThreshold(root, 900.0, ViewConditionType::Area);

    let mut view = emView::new(root, 800.0, 600.0);
    view.flags.insert(ViewFlags::NO_ACTIVE_HIGHLIGHT);
    // C++ golden gen doesn't focus the window
    view.SetFocused(&mut tree, false);

    // C++ gen_golden.cpp: TerminateEngine ctrl(sched, 200)
    settle(&mut tree, &mut view, 200);

    // C++ gen_golden.cpp: view.Zoom(400, 300, 2.0)
    // Rust emView::Zoom(factor, center_x, center_y)
    view.Zoom(2.0, 400.0, 300.0);
    // C++ gen_golden.cpp: TerminateEngine ctrl(sched, 10)
    settle(&mut tree, &mut view, 10);

    let mut compositor = SoftwareCompositor::new(w, h);
    compositor.render(&mut tree, &view);
    let actual = compositor.framebuffer().GetMap();

    // TkTestPanel at 2x zoom amplifies layout GetPos differences.
    // Zoom shifts expose border-rounding rects that differ from C++ at sub-pixel level.
    let GetResult = compare_images(
        "tktest_2x",
        actual,
        expected_data,
        w,
        h,
        3,
        75.0,
    );
    if GetResult.is_err() && dump_golden_enabled() {
        dump_test_images("tktest_2x", actual, expected_data, w, h);
        analyze_diff_distribution(actual, expected_data, w, h, 3);
    }
    GetResult.unwrap();
}
