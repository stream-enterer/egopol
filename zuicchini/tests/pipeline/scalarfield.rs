//! Systematic interaction test for ScalarField at 1x and 2x zoom, driven
//! through the full input dispatch pipeline (PipelineTestHarness).
//!
//! Verifies that click and drag interactions correctly update the widget's
//! value at both zoom levels, using approximate assertions to account for
//! border insets in the content area.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{Look, ScalarField};

use super::support::pipeline::PipelineTestHarness;

/// PanelBehavior wrapper for ScalarField so it can be installed into the
/// panel tree. Delegates paint/input to the underlying widget and syncs
/// the value to a shared handle after every input event.
struct ScalarFieldPanel {
    sf: ScalarField,
    /// Shared handle so the test can read the value after interaction.
    value: Rc<RefCell<f64>>,
}

impl ScalarFieldPanel {
    fn new(sf: ScalarField, value: Rc<RefCell<f64>>) -> Self {
        Self { sf, value }
    }
}

impl PanelBehavior for ScalarFieldPanel {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        self.sf.paint(painter, w, h, state.enabled);
    }

    fn input(
        &mut self,
        event: &InputEvent,
        state: &PanelState,
        input_state: &InputState,
    ) -> bool {
        let consumed = self.sf.input(event, state, input_state);
        *self.value.borrow_mut() = self.sf.value();
        consumed
    }

    fn get_cursor(&self) -> Cursor {
        self.sf.get_cursor()
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

/// Helper: assert that `actual` is within `tolerance` of `expected`.
fn assert_approx(actual: f64, expected: f64, tolerance: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: expected ~{expected:.1} (+-{tolerance}), got {actual:.1}"
    );
}

#[test]
fn scalarfield_click_and_drag_1x_and_2x() {
    // 1. Create PipelineTestHarness (800x600 viewport).
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // 2. Create ScalarField (range 0-100, value 50, editable).
    let look = Look::new();
    let mut sf = ScalarField::new(0.0, 100.0, look);
    sf.set_value(50.0);
    sf.set_editable(true);

    let value = Rc::new(RefCell::new(50.0));
    let value_read = value.clone();

    // 3. Wrap in ScalarFieldPanel and add to tree.
    let behavior = ScalarFieldPanel::new(sf, value);
    let _panel_id = h.add_panel_with(root, "scalar_field", Box::new(behavior));

    // 4. Tick + render via SoftwareCompositor to populate last_w/last_h.
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    let vw = 800.0;
    let vh = 600.0;
    let mid_y = vh * 0.5;

    // ── 5. At 1x zoom ──────────────────────────────────────────────────
    //
    // The ScalarField has an Instrument outer border, InputField inner
    // border, and HowTo space on the left. These insets eat into the
    // usable scale area, so viewport percentages do not map linearly to
    // value percentages. We click at positions well inside the scale
    // area and use generous tolerances (+-15).

    // Click at ~40% of viewport width -> value should be somewhere near 30-40.
    let click_x_40 = vw * 0.40;
    h.click(click_x_40, mid_y);
    let val_after_click_1x = *value_read.borrow();
    assert!(
        val_after_click_1x > 10.0 && val_after_click_1x < 55.0,
        "1x click at 40% viewport: expected value in 10..55, got {val_after_click_1x:.1}"
    );

    // Drag from 40% to ~65% of viewport width -> value should increase
    // significantly toward the mid-to-high range.
    let drag_to_x = vw * 0.65;
    h.drag(click_x_40, mid_y, drag_to_x, mid_y);
    let val_after_drag_1x = *value_read.borrow();
    assert!(
        val_after_drag_1x > val_after_click_1x + 5.0,
        "1x drag from 40% to 65%: value should increase by >5 from {val_after_click_1x:.1}, \
         got {val_after_drag_1x:.1}"
    );
    assert!(
        val_after_drag_1x > 40.0 && val_after_drag_1x < 90.0,
        "1x drag to 65% viewport: expected value in 40..90, got {val_after_drag_1x:.1}"
    );

    // ── 6. At 2x zoom ──────────────────────────────────────────────────
    //
    // At 2x zoom the panel is magnified 2x: the viewport shows only
    // the center 50% of the panel. The viewport center (400,300) still
    // maps to the panel center (value ~50).

    // Set zoom to 2x, tick, re-render.
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    // Click at viewport center to reset value to ~50.
    let center_x = vw * 0.5;
    h.click(center_x, mid_y);
    let val_after_center_2x = *value_read.borrow();
    assert_approx(
        val_after_center_2x,
        50.0,
        15.0,
        "2x click at viewport center",
    );

    // Click at ~30% of viewport width at 2x zoom.
    // At 2x the visible panel portion is 25%-75% of the panel, so 30%
    // viewport maps to roughly 40% panel -> value ~30-40.
    let click_x_30_2x = vw * 0.30;
    h.click(click_x_30_2x, mid_y);
    let val_after_click_2x = *value_read.borrow();
    assert!(
        (val_after_click_2x - val_after_center_2x).abs() > 1.0,
        "2x click at 30% viewport should change value from {val_after_center_2x:.1}, \
         but got {val_after_click_2x:.1}"
    );

    // Drag from 30% to ~70% of viewport width at 2x zoom.
    let drag_to_x_2x = vw * 0.70;
    h.drag(click_x_30_2x, mid_y, drag_to_x_2x, mid_y);
    let val_after_drag_2x = *value_read.borrow();
    assert!(
        val_after_drag_2x > val_after_click_2x,
        "2x drag from 30% to 70% should increase value: \
         was {val_after_click_2x:.1}, now {val_after_drag_2x:.1}"
    );
}
