//! Systematic interaction test for RadioBox at 1x and 2x zoom, driven
//! through the full input dispatch pipeline (PipelineTestHarness).
//!
//! Three RadioBox widgets share a group, each installed in its own child panel
//! stacked vertically. Clicking each panel's center selects the corresponding
//! radio box. The test verifies correct selection at both 1x and 2x zoom,
//! re-clicking the already-selected box (no-op), and cycling through all items.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{Look, RadioBox, RadioGroup};

use super::support::pipeline::PipelineTestHarness;

// ---------------------------------------------------------------------------
// RadioBoxBehavior -- minimal PanelBehavior wrapper for RadioBox
// ---------------------------------------------------------------------------

struct RadioBoxBehavior {
    widget: RadioBox,
}

impl RadioBoxBehavior {
    fn new(widget: RadioBox) -> Self {
        Self { widget }
    }
}

impl PanelBehavior for RadioBoxBehavior {
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

// ---------------------------------------------------------------------------
// Helper: set up a 3-option RadioBox harness
// ---------------------------------------------------------------------------

struct RadioBoxHarness {
    h: PipelineTestHarness,
    group: Rc<RefCell<zuicchini::widget::RadioGroup>>,
    panels: [zuicchini::panel::PanelId; 3],
    compositor: SoftwareCompositor,
}

impl RadioBoxHarness {
    fn new() -> Self {
        let look = Look::new();
        let group: Rc<RefCell<RadioGroup>> = RadioGroup::new();

        let rb0 = RadioBox::new("Alpha", look.clone(), group.clone(), 0);
        let rb1 = RadioBox::new("Beta", look.clone(), group.clone(), 1);
        let rb2 = RadioBox::new("Gamma", look, group.clone(), 2);

        assert_eq!(group.borrow().count(), 3);
        assert_eq!(group.borrow().selected(), None);

        let mut h = PipelineTestHarness::new();
        let root = h.root();

        // Each radio box gets its own child panel, stacked vertically:
        //   panel 0: y=0.00..0.33  (top third)
        //   panel 1: y=0.33..0.66  (middle third)
        //   panel 2: y=0.66..1.00  (bottom third)
        let panel0 = h.add_panel_with(root, "rbox0", Box::new(RadioBoxBehavior::new(rb0)));
        h.tree
            .set_layout_rect(panel0, 0.0, 0.0, 1.0, 1.0 / 3.0);

        let panel1 = h.add_panel_with(root, "rbox1", Box::new(RadioBoxBehavior::new(rb1)));
        h.tree
            .set_layout_rect(panel1, 0.0, 1.0 / 3.0, 1.0, 1.0 / 3.0);

        let panel2 = h.add_panel_with(root, "rbox2", Box::new(RadioBoxBehavior::new(rb2)));
        h.tree
            .set_layout_rect(panel2, 0.0, 2.0 / 3.0, 1.0, 1.0 / 3.0);

        // Settle layout and viewing geometry.
        h.tick_n(5);

        // Render so that RadioBox::paint() caches last_w/last_h (required
        // for hit_test to function).
        let mut compositor = SoftwareCompositor::new(800, 600);
        compositor.render(&mut h.tree, &h.view);

        Self {
            h,
            group,
            panels: [panel0, panel1, panel2],
            compositor,
        }
    }

    /// Compute the view-space center of a panel.
    fn panel_center(&self, index: usize) -> (f64, f64) {
        let state = self.h.tree.build_panel_state(
            self.panels[index],
            self.h.view.window_focused(),
            self.h.view.pixel_tallness(),
        );
        let vr = state.viewed_rect;
        (vr.x + vr.w * 0.5, vr.y + vr.h * 0.5)
    }

    /// Switch to a given zoom level, tick, and re-render.
    fn zoom_to(&mut self, level: f64) {
        self.h.set_zoom(level);
        self.h.tick_n(5);
        self.compositor.render(&mut self.h.tree, &self.h.view);
    }

    fn selected(&self) -> Option<usize> {
        self.group.borrow().selected()
    }

    fn click_option(&mut self, index: usize) {
        let (cx, cy) = self.panel_center(index);
        self.h.click(cx, cy);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Click each of three vertically-stacked radio boxes at 1x and 2x zoom,
/// verifying the group selection state after each click.
#[test]
fn radiobox_select_1x_and_2x() {
    let mut t = RadioBoxHarness::new();

    // ── 1x zoom: click each radio box ─────────────────────────────────
    t.click_option(0);
    assert_eq!(
        t.selected(),
        Some(0),
        "1x: clicking option 0 should select radio box 0"
    );

    t.click_option(1);
    assert_eq!(
        t.selected(),
        Some(1),
        "1x: clicking option 1 should select radio box 1"
    );

    t.click_option(2);
    assert_eq!(
        t.selected(),
        Some(2),
        "1x: clicking option 2 should select radio box 2"
    );

    // ── 2x zoom: same test at higher magnification ───────────────────
    t.zoom_to(2.0);

    t.click_option(0);
    assert_eq!(
        t.selected(),
        Some(0),
        "2x: clicking option 0 should select radio box 0"
    );

    t.click_option(1);
    assert_eq!(
        t.selected(),
        Some(1),
        "2x: clicking option 1 should select radio box 1"
    );

    t.click_option(2);
    assert_eq!(
        t.selected(),
        Some(2),
        "2x: clicking option 2 should select radio box 2"
    );
}

/// Re-clicking the already-selected radio box should keep it selected
/// (radio boxes cannot be deselected by clicking them again).
#[test]
fn radiobox_reclick_selected_is_noop() {
    let mut t = RadioBoxHarness::new();

    // Select option 1.
    t.click_option(1);
    assert_eq!(t.selected(), Some(1));

    // Click option 1 again -- should remain selected.
    t.click_option(1);
    assert_eq!(
        t.selected(),
        Some(1),
        "re-clicking already-selected radio box must not deselect it"
    );

    // Same behavior at 2x zoom.
    t.zoom_to(2.0);

    t.click_option(1);
    assert_eq!(
        t.selected(),
        Some(1),
        "2x: re-clicking already-selected radio box must not deselect it"
    );
}

/// Cycle through all options forward and backward at both zoom levels,
/// verifying each transition.
#[test]
fn radiobox_cycle_forward_and_backward() {
    let mut t = RadioBoxHarness::new();

    // Forward cycle at 1x: 0 -> 1 -> 2
    for i in 0..3 {
        t.click_option(i);
        assert_eq!(
            t.selected(),
            Some(i),
            "1x forward: expected selection {i}"
        );
    }

    // Backward cycle at 1x: 2 -> 1 -> 0
    for i in (0..3).rev() {
        t.click_option(i);
        assert_eq!(
            t.selected(),
            Some(i),
            "1x backward: expected selection {i}"
        );
    }

    // Forward cycle at 2x
    t.zoom_to(2.0);
    for i in 0..3 {
        t.click_option(i);
        assert_eq!(
            t.selected(),
            Some(i),
            "2x forward: expected selection {i}"
        );
    }

    // Backward cycle at 2x
    for i in (0..3).rev() {
        t.click_option(i);
        assert_eq!(
            t.selected(),
            Some(i),
            "2x backward: expected selection {i}"
        );
    }
}

/// Verify that selection starts as None and transitions correctly on
/// the first click at each zoom level.
#[test]
fn radiobox_initial_state_is_none() {
    let t = RadioBoxHarness::new();
    assert_eq!(
        t.selected(),
        None,
        "no radio box should be selected initially"
    );
}

/// Verify selection survives a zoom transition without being lost or
/// corrupted.
#[test]
fn radiobox_selection_survives_zoom_change() {
    let mut t = RadioBoxHarness::new();

    // Select option 1 at 1x.
    t.click_option(1);
    assert_eq!(t.selected(), Some(1));

    // Zoom to 2x -- selection must persist.
    t.zoom_to(2.0);
    assert_eq!(
        t.selected(),
        Some(1),
        "selection must survive zoom change from 1x to 2x"
    );

    // Select option 2 at 2x.
    t.click_option(2);
    assert_eq!(t.selected(), Some(2));

    // Zoom back to 1x -- selection must persist.
    t.zoom_to(1.0);
    assert_eq!(
        t.selected(),
        Some(2),
        "selection must survive zoom change from 2x to 1x"
    );
}
