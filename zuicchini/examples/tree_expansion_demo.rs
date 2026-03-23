//! Tree-expansion demo derived from C++ `TreeExpansionExample.cpp`.
//!
//! Each panel fills itself with its background color and, on auto-expand,
//! creates four children whose color is the inverse of the parent's.
//! Zooming into any panel reveals the next level.

use zuicchini::emCore::emColor::emColor;
use zuicchini::emCore::emPanel::{PanelBehavior, PanelState};
use zuicchini::emCore::emPanelCtx::PanelCtx;
use zuicchini::emCore::emView::ViewFlags;
use zuicchini::emCore::emPainter::emPainter;
use zuicchini::emCore::emGUIFramework::App;
use zuicchini::emCore::emWindow::WindowFlags;

struct MyPanel {
    bg: emColor,
}

impl PanelBehavior for MyPanel {
    fn is_opaque(&self) -> bool {
        self.bg.is_opaque()
    }

    fn paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        painter.paint_rect(0.0, 0.0, w, h, self.bg, emColor::TRANSPARENT);
    }

    fn auto_expand(&self) -> bool {
        true
    }

    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        let children = ctx.children();
        let rect = ctx.layout_rect();
        let h = rect.h / rect.w;

        if !children.is_empty() {
            // Reposition existing children.
            for (idx, child) in children.iter().enumerate() {
                let i = idx;
                let cx = 0.1 + (i & 1) as f64 * 0.5;
                let cy = (0.1 + ((i >> 1) & 1) as f64 * 0.5) * h;
                ctx.layout_child(*child, cx, cy, 0.3, 0.3 * h);
            }
            return;
        }

        // Create four children with inverted color.
        let inv = emColor::rgba(
            255 - self.bg.r(),
            255 - self.bg.g(),
            255 - self.bg.b(),
            self.bg.a(),
        );
        for i in 0..4u32 {
            let name = format!("{i}");
            let cx = 0.1 + (i & 1) as f64 * 0.5;
            let cy = (0.1 + ((i >> 1) & 1) as f64 * 0.5) * h;
            let child = ctx.create_child_with(&name, Box::new(MyPanel { bg: inv }));
            ctx.layout_child(child, cx, cy, 0.3, 0.3 * h);
        }
    }
}

fn main() {
    let app = App::new(Box::new(|app, event_loop| {
        let root = app.tree.create_root("root");
        app.tree
            .set_behavior(root, Box::new(MyPanel { bg: emColor::WHITE }));
        app.tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0);

        let close_sig = app.scheduler.create_signal();
        let flags_sig = app.scheduler.create_signal();
        let win = zuicchini::emCore::emWindow::ZuiWindow::create(
            event_loop,
            app.gpu(),
            root,
            WindowFlags::AUTO_DELETE,
            close_sig,
            flags_sig,
        );
        let wid = win.winit_window.id();
        app.windows.insert(wid, win);
        app.windows.get_mut(&wid).unwrap().view_mut().flags |= ViewFlags::ROOT_SAME_TALLNESS;
    }));
    app.run();
}
