//! Systematic interaction test for Button at 1x and 2x zoom, driven through
//! the full input dispatch pipeline (PipelineTestHarness).


use std::cell::Cell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{Button, Look};

use super::support::pipeline::PipelineTestHarness;

/// Minimal PanelBehavior wrapper for Button so it can be installed into the
/// panel tree. Delegates paint/input to the underlying widget.
struct ButtonPanel {
    widget: Button,
}

impl PanelBehavior for ButtonPanel {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        self.widget.paint(painter, w, h, state.enabled);
    }

    fn input(
        &mut self,
        event: &InputEvent,
        state: &PanelState,
        input_state: &InputState,
    ) -> bool {
        self.widget.input(event, state, input_state)
    }

    fn get_cursor(&self) -> Cursor {
        Cursor::Normal
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

#[test]
fn button_click_1x_and_2x() {
    // 1. Create PipelineTestHarness (800x600 viewport).
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // 2. Create Button with on_click callback incrementing a shared counter.
    let counter = Rc::new(Cell::new(0u32));
    let counter_clone = counter.clone();

    let look = Look::new();
    let mut btn = Button::new("Systematic Test", look);
    btn.on_click = Some(Box::new(move || {
        counter_clone.set(counter_clone.get() + 1);
    }));

    // 3. Wrap in PanelBehavior and add to tree.
    let _panel_id = h.add_panel_with(root, "button", Box::new(ButtonPanel { widget: btn }));

    // 4. Tick + render (SoftwareCompositor) to populate last_w/last_h.
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    // 5. At 1x zoom: click at viewport center (400, 300).
    h.click(400.0, 300.0);
    assert_eq!(
        counter.get(),
        1,
        "Button callback should have fired once after click at 1x zoom"
    );

    // 6. At 2x zoom: set_zoom, tick, re-render, then click at viewport center.
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    h.click(400.0, 300.0);
    assert_eq!(
        counter.get(),
        2,
        "Button callback should have fired again after click at 2x zoom"
    );
}
