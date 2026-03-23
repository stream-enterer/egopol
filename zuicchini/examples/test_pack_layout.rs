//! Pack layout test derived from C++ `emTestPackLayout.cpp`.
//!
//! Creates N panels (default 20, configurable via CLI arg) with random weights
//! and preferred tallnesses, arranged by a `emPackLayout`. Each panel paints a
//! colored `emBorder` with its tallness value as caption.

use rand::Rng;

use zuicchini::emCore::emColor::emColor;
use zuicchini::emCore::emPackLayout::emPackLayout;
use zuicchini::emCore::emTiling::ChildConstraint;
use zuicchini::emCore::emPanel::PanelBehavior;
use zuicchini::emCore::emView::ViewFlags;
use zuicchini::emCore::emPainter::emPainter;
use zuicchini::emCore::emBorder::{emBorder, OuterBorderType};
use zuicchini::emCore::emLook::emLook;
use zuicchini::emCore::emGUIFramework::App;
use zuicchini::emCore::emWindow::WindowFlags;

struct BorderPanel {
    border: emBorder,
    look: emLook,
}

impl PanelBehavior for BorderPanel {
    fn paint(
        &mut self,
        painter: &mut emPainter,
        w: f64,
        h: f64,
        _state: &zuicchini::emCore::emPanel::PanelState,
    ) {
        self.border
            .paint_border(painter, w, h, &self.look, false, true, 1.0);
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

fn main() {
    let panel_count: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    let app = App::new(Box::new(move |app, event_loop| {
        let root = app.tree.create_root("root");
        let mut layout = emPackLayout::new();
        let mut rng = rand::rng();

        for i in 0..panel_count {
            let weight: f64 = rng.random_range(1.0..100.0);
            let pct: f64 = rng.random_range(-2.5_f64..2.5).exp();
            let hue: u32 = rng.random_range(0..360);

            let color = emColor::SetHSVA(hue as f32, 0.5, 0.5);

            let look = emLook {
                bg_color: color,
                ..emLook::default()
            };

            let caption = format!("{pct:.4}");
            let border = emBorder::new(OuterBorderType::Filled).with_caption(&caption);

            let child = app.tree.create_child(root, &format!("{i:06}"));
            app.tree
                .set_behavior(child, Box::new(BorderPanel { border, look }));

            layout.set_child_constraint(
                child,
                ChildConstraint {
                    weight,
                    preferred_tallness: pct,
                    ..Default::default()
                },
            );
        }

        app.tree.set_behavior(root, Box::new(layout));
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
