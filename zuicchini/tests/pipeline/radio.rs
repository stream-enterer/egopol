//! Systematic interaction test for RadioButton at 1x and 2x zoom, driven
//! through the full input dispatch pipeline (PipelineTestHarness).
//!
//! Three radio buttons share a group, each installed in its own child panel
//! stacked vertically. Clicking each panel's center selects the corresponding
//! radio button. The test verifies correct selection at both 1x and 2x zoom.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{Look, RadioButton, RadioGroup};

use super::support::pipeline::PipelineTestHarness;

// ---------------------------------------------------------------------------
// RadioButtonBehavior -- minimal PanelBehavior wrapper for RadioButton
// ---------------------------------------------------------------------------

struct RadioButtonBehavior {
    widget: RadioButton,
}

impl RadioButtonBehavior {
    fn new(widget: RadioButton) -> Self {
        Self { widget }
    }
}

impl PanelBehavior for RadioButtonBehavior {
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
        self.widget.get_cursor()
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

/// Click each of three vertically-stacked radio buttons at 1x and 2x zoom,
/// verifying the group selection state after each click.
#[test]
fn radiobutton_select_1x_and_2x() {
    let look = Look::new();
    let group: Rc<RefCell<RadioGroup>> = RadioGroup::new();

    // Create 3 RadioButtons sharing the same group.
    let rb0 = RadioButton::new("Option A", look.clone(), group.clone(), 0);
    let rb1 = RadioButton::new("Option B", look.clone(), group.clone(), 1);
    let rb2 = RadioButton::new("Option C", look.clone(), group.clone(), 2);

    assert_eq!(group.borrow().count(), 3);
    assert_eq!(group.borrow().selected(), None);

    // ── Build pipeline harness (800x600 viewport) ────────────────────
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // Each radio button gets its own child panel, stacked vertically:
    //   panel 0: y=0.00..0.33  (top third)
    //   panel 1: y=0.33..0.66  (middle third)
    //   panel 2: y=0.66..1.00  (bottom third)
    let panel0 = h.add_panel_with(root, "radio0", Box::new(RadioButtonBehavior::new(rb0)));
    h.tree
        .set_layout_rect(panel0, 0.0, 0.0, 1.0, 1.0 / 3.0);

    let panel1 = h.add_panel_with(root, "radio1", Box::new(RadioButtonBehavior::new(rb1)));
    h.tree
        .set_layout_rect(panel1, 0.0, 1.0 / 3.0, 1.0, 1.0 / 3.0);

    let panel2 = h.add_panel_with(root, "radio2", Box::new(RadioButtonBehavior::new(rb2)));
    h.tree
        .set_layout_rect(panel2, 0.0, 2.0 / 3.0, 1.0, 1.0 / 3.0);

    // Settle layout and viewing geometry.
    h.tick_n(5);

    // Render so that RadioButton::paint() caches last_w/last_h (required
    // for hit_test to function).
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    // ── Helper: compute view-space center of a panel ─────────────────
    let panel_center = |harness: &PipelineTestHarness, panel_id| {
        let state = harness.tree.build_panel_state(
            panel_id,
            harness.view.window_focused(),
            harness.view.pixel_tallness(),
        );
        let vr = state.viewed_rect;
        (vr.x + vr.w * 0.5, vr.y + vr.h * 0.5)
    };

    // ── 1x zoom: click each radio button ─────────────────────────────
    {
        let (cx, cy) = panel_center(&h, panel0);
        h.click(cx, cy);
        assert_eq!(
            group.borrow().selected(),
            Some(0),
            "1x: clicking panel 0 should select radio button 0"
        );
    }
    {
        let (cx, cy) = panel_center(&h, panel1);
        h.click(cx, cy);
        assert_eq!(
            group.borrow().selected(),
            Some(1),
            "1x: clicking panel 1 should select radio button 1"
        );
    }
    {
        let (cx, cy) = panel_center(&h, panel2);
        h.click(cx, cy);
        assert_eq!(
            group.borrow().selected(),
            Some(2),
            "1x: clicking panel 2 should select radio button 2"
        );
    }

    // ── 2x zoom: same test at higher magnification ───────────────────
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    {
        let (cx, cy) = panel_center(&h, panel0);
        h.click(cx, cy);
        assert_eq!(
            group.borrow().selected(),
            Some(0),
            "2x: clicking panel 0 should select radio button 0"
        );
    }
    {
        let (cx, cy) = panel_center(&h, panel1);
        h.click(cx, cy);
        assert_eq!(
            group.borrow().selected(),
            Some(1),
            "2x: clicking panel 1 should select radio button 1"
        );
    }
    {
        let (cx, cy) = panel_center(&h, panel2);
        h.click(cx, cy);
        assert_eq!(
            group.borrow().selected(),
            Some(2),
            "2x: clicking panel 2 should select radio button 2"
        );
    }
}
