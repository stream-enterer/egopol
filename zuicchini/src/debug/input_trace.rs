//! Diagnostic: trace input dispatch to understand which panels receive events
//! and why they do or don't consume them.
//!
//! Run: `cargo test --lib debug::input_trace -- --nocapture`

#[cfg(test)]
mod tests {
    use crate::emCore::emColor::emColor;
    use crate::emCore::rect::Rect;
    use crate::emCore::emInput::{emInputEvent, InputKey};
    use crate::emCore::emInputState::emInputState;
    use crate::emCore::emPanelTree::{PanelId, PanelTree, ViewConditionType};
    use crate::emCore::emPanel::{PanelBehavior, PanelState};
    use crate::emCore::emPanelCtx::PanelCtx;
    use crate::emCore::emView::{emView, ViewFlags};
    use crate::emCore::emPainter::emPainter;
    use crate::emCore::emButton::emButton;
    use crate::emCore::emCheckButton::emCheckButton;
    use crate::emCore::emLook::emLook;
    use slotmap::Key as _;

    use std::rc::Rc;

    fn default_panel_state() -> PanelState {
        PanelState {
            id: PanelId::null(),
            is_active: true,
            in_active_path: true,
            window_focused: true,
            enabled: true,
            viewed: true,
            clip_rect: Rect::new(0.0, 0.0, 1e6, 1e6),
            viewed_rect: Rect::new(0.0, 0.0, 200.0, 100.0),
            priority: 1.0,
            memory_limit: u64::MAX,
            pixel_tallness: 1.0,
            height: 0.5,
        }
    }

    struct ButtonPanel {
        widget: emButton,
    }
    impl PanelBehavior for ButtonPanel {
        fn paint(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
            self.widget.paint(p, w, h, _s.enabled);
        }
        fn input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
            self.widget.input(e, _s, _is)
        }
        fn is_opaque(&self) -> bool {
            true
        }
    }

    struct CheckButtonPanel {
        widget: emCheckButton,
    }
    impl PanelBehavior for CheckButtonPanel {
        fn paint(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
            self.widget.paint(p, w, h, _s.enabled);
        }
        fn input(&mut self, e: &emInputEvent, _s: &PanelState, _is: &emInputState) -> bool {
            self.widget.input(e, _s, _is)
        }
        fn is_opaque(&self) -> bool {
            true
        }
    }

    struct TestRoot {
        look: Rc<emLook>,
    }
    impl PanelBehavior for TestRoot {
        fn is_opaque(&self) -> bool {
            true
        }
        fn auto_expand(&self) -> bool {
            true
        }
        fn paint(&mut self, p: &mut emPainter, w: f64, h: f64, _s: &PanelState) {
            p.paint_rect(
                0.0,
                0.0,
                w,
                h,
                emColor::rgba(0x30, 0x40, 0x50, 0xFF),
                emColor::TRANSPARENT,
            );
        }
        fn layout_children(&mut self, ctx: &mut PanelCtx) {
            if ctx.children().is_empty() {
                let look = self.look.clone();
                ctx.create_child_with(
                    "btn1",
                    Box::new(ButtonPanel {
                        widget: emButton::new("Button1", look.clone()),
                    }),
                );
                ctx.create_child_with(
                    "btn2",
                    Box::new(ButtonPanel {
                        widget: emButton::new("Button2", look.clone()),
                    }),
                );
                ctx.create_child_with(
                    "chk1",
                    Box::new(CheckButtonPanel {
                        widget: emCheckButton::new("CheckBtn", look),
                    }),
                );
            }
            let children = ctx.children();
            let margin = 0.02;
            let cell_w = (1.0 - margin * 3.0) / 2.0;
            let cell_h = cell_w * 0.4;
            for (i, child) in children.iter().enumerate() {
                let col = i % 2;
                let row = i / 2;
                let x = margin + col as f64 * (cell_w + margin);
                let y = margin + row as f64 * (cell_h + margin);
                ctx.layout_child(*child, x, y, cell_w, cell_h);
            }
        }
    }

    fn settle(tree: &mut PanelTree, view: &mut emView) {
        for _ in 0..5 {
            tree.deliver_notices(view.window_focused(), view.pixel_tallness());
            view.update_viewing(tree);
        }
    }

    /// Test 1: Does emButton::hit_test work at all with a standalone button?
    #[test]
    fn trace_standalone_button() {
        let look: Rc<emLook> = emLook::new();
        let mut btn = emButton::new("Test", look.clone());

        // Paint to set last_w/last_h
        let mut img = crate::emCore::emImage::emImage::new(100, 40, 4);
        let mut painter = emPainter::new(&mut img);
        btn.paint(&mut painter, 1.0, 0.4, true);

        // Test content_round_rect directly
        let border =
            crate::emCore::emBorder::emBorder::new(crate::emCore::emBorder::OuterBorderType::InstrumentMoreRound)
                .with_caption("Test")
                .with_label_in_border(false);
        let (rect, r) = border.content_round_rect(1.0, 0.4, &look);
        eprintln!("\n=== STANDALONE BUTTON (w=1.0, h=0.4) ===");
        eprintln!(
            "  content_round_rect: x={:.6} y={:.6} w={:.6} h={:.6} r={:.6}",
            rect.x, rect.y, rect.w, rect.h, r
        );

        let hit_center = crate::emCore::widget_utils::check_mouse_round_rect(0.5, 0.2, &rect, r);
        let hit_origin = crate::emCore::widget_utils::check_mouse_round_rect(0.0, 0.0, &rect, r);
        let hit_outside = crate::emCore::widget_utils::check_mouse_round_rect(-1.0, -1.0, &rect, r);
        eprintln!("  hit_test(0.5, 0.2) [center]: {}", hit_center);
        eprintln!("  hit_test(0.0, 0.0) [origin]: {}", hit_origin);
        eprintln!("  hit_test(-1, -1) [outside]: {}", hit_outside);

        // Test via input
        let ps = default_panel_state();
        let is = emInputState::new();
        let press_center = emInputEvent::press(InputKey::MouseLeft).with_mouse(0.5, 0.2);
        let consumed = btn.input(&press_center, &ps, &is);
        eprintln!("  btn.input(press at center): consumed={}", consumed);

        let press_origin = emInputEvent::press(InputKey::MouseLeft).with_mouse(0.0, 0.0);
        let consumed2 = btn.input(&press_origin, &ps, &is);
        eprintln!("  btn.input(press at origin): consumed={}", consumed2);
    }

    /// Test 2: Full tree dispatch with coordinate transforms
    #[test]
    fn trace_tree_dispatch() {
        let look: Rc<emLook> = emLook::new();
        let mut tree = PanelTree::new();

        let root = tree.create_root("root");
        tree.set_behavior(root, Box::new(TestRoot { look: look.clone() }));
        tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);
        tree.set_auto_expansion_threshold(root, 900.0, ViewConditionType::Area);

        let mut view = emView::new(root, 800.0, 600.0);
        view.set_view_flags(view.flags | ViewFlags::ROOT_SAME_TALLNESS, &mut tree);
        settle(&mut tree, &mut view);

        // Dump tree
        let viewed = tree.viewed_panels_dfs();
        eprintln!("\n=== PANEL TREE ({} panels) ===", viewed.len());
        for &pid in &viewed {
            let name = tree.get(pid).map(|p| p.name.clone()).unwrap_or_default();
            let lr = tree.get(pid).map(|p| p.layout_rect).unwrap();
            let vx = tree.get(pid).map(|p| p.viewed_x).unwrap_or(0.0);
            let vy = tree.get(pid).map(|p| p.viewed_y).unwrap_or(0.0);
            let vw = tree.get(pid).map(|p| p.viewed_width).unwrap_or(0.0);
            let vh = tree.get(pid).map(|p| p.viewed_height).unwrap_or(0.0);
            eprintln!(
                "  {:?} layout=({:.4},{:.4},{:.4},{:.4}) viewed=({:.1},{:.1},{:.1},{:.1})",
                name, lr.x, lr.y, lr.w, lr.h, vx, vy, vw, vh
            );
        }

        // Paint all
        {
            let mut img = crate::emCore::emImage::emImage::new(800, 600, 4);
            for &pid in &viewed {
                if let Some(mut beh) = tree.take_behavior(pid) {
                    let lr = tree.get(pid).unwrap().layout_rect;
                    let tallness = if lr.w > 1e-100 { lr.h / lr.w } else { 1.0 };
                    let state = tree.build_panel_state(pid, true, view.pixel_tallness());
                    let mut painter = emPainter::new(&mut img);
                    beh.paint(&mut painter, 1.0, tallness, &state);
                    tree.put_behavior(pid, beh);
                }
            }
        }

        // Click at center of each widget
        let input_state = emInputState::default();
        let targets: Vec<_> = viewed
            .iter()
            .filter_map(|&pid| {
                let name = tree.get(pid)?.name.clone();
                if name == "root" {
                    None
                } else {
                    Some((pid, name))
                }
            })
            .collect();

        for (target_pid, target_name) in &targets {
            let vx = tree.get(*target_pid).unwrap().viewed_x;
            let vy = tree.get(*target_pid).unwrap().viewed_y;
            let vw = tree.get(*target_pid).unwrap().viewed_width;
            let vh = tree.get(*target_pid).unwrap().viewed_height;
            let cx = vx + vw / 2.0;
            let cy = vy + vh / 2.0;

            eprintln!(
                "\n=== CLICK {:?} at view ({:.1}, {:.1}) ===",
                target_name, cx, cy
            );

            let order = tree.viewed_panels_dfs();
            for &dpid in &order {
                let dname = tree.get(dpid).map(|p| p.name.clone()).unwrap_or_default();
                let lx = tree.view_to_panel_x(dpid, cx);
                let ly = tree.view_to_panel_y(dpid, cy, view.pixel_tallness());
                let ev = emInputEvent::press(InputKey::MouseLeft).with_mouse(lx, ly);

                if let Some(mut beh) = tree.take_behavior(dpid) {
                    let state = tree.build_panel_state(dpid, true, view.pixel_tallness());
                    let consumed = beh.input(&ev, &state, &input_state);
                    tree.put_behavior(dpid, beh);
                    eprintln!(
                        "  {:?} local=({:.4},{:.4}) consumed={}",
                        dname, lx, ly, consumed
                    );
                    if consumed {
                        eprintln!("  >>> CONSUMED");
                        break;
                    }
                }
            }
        }
    }
}
