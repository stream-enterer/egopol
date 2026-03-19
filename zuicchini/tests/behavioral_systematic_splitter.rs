//! Systematic interaction test for Splitter at 1x and 2x zoom, driven through
//! the full input dispatch pipeline (PipelineTestHarness).
//!
//! Verifies Splitter drag behavior when dispatched through the coordinate-
//! transform pipeline at different zoom levels.

#[path = "support/mod.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::layout::Orientation;
use zuicchini::panel::{PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{Look, Splitter};

use crate::support::pipeline::PipelineTestHarness;

// ---------------------------------------------------------------------------
// PanelBehavior wrapper for Splitter (shared via Rc<RefCell>)
// ---------------------------------------------------------------------------

struct SharedSplitterPanel {
    inner: Rc<RefCell<Splitter>>,
}

impl PanelBehavior for SharedSplitterPanel {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        self.inner.borrow_mut().paint(painter, w, h, state.enabled);
    }

    fn input(
        &mut self,
        event: &InputEvent,
        state: &PanelState,
        input_state: &InputState,
    ) -> bool {
        self.inner.borrow_mut().input(event, state, input_state)
    }

    fn get_cursor(&self) -> Cursor {
        self.inner.borrow().get_cursor()
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Helper: create a harness with a shared Splitter at a given position.
// ---------------------------------------------------------------------------

fn setup_splitter(
    orientation: Orientation,
    initial_pos: f64,
) -> (PipelineTestHarness, Rc<RefCell<Splitter>>, SoftwareCompositor) {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = Look::new();
    let mut sp = Splitter::new(orientation, look);
    sp.set_position(initial_pos);
    let sp_ref = Rc::new(RefCell::new(sp));

    let _panel_id = h.add_panel_with(
        root,
        "splitter",
        Box::new(SharedSplitterPanel {
            inner: sp_ref.clone(),
        }),
    );
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    (h, sp_ref, compositor)
}

// ---------------------------------------------------------------------------
// Test: Horizontal Splitter drag at 1x and 2x zoom
// ---------------------------------------------------------------------------

/// Horizontal Splitter drag through the full pipeline at 1x and 2x zoom.
///
/// The Splitter's `input()` method computes grip geometry in normalized
/// `(1.0, tallness)` panel-local space, matching the coordinate system
/// used by the pipeline for mouse coordinates.
#[test]
fn splitter_drag_horizontal_1x_and_2x() {
    let (mut h, sp_ref, mut compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Verify initial state.
    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "Splitter should start at position 0.5"
    );

    // ── At 1x zoom ─────────────────────────────────────────────────
    // Drag from grip center (view 400,300) to ~30% (view 240,300).
    h.drag(400.0, 300.0, 240.0, 300.0);

    let pos_after_1x = sp_ref.borrow().position();

    assert!(
        (pos_after_1x - 0.3).abs() < 0.1,
        "After dragging to ~30%, position should be near 0.3. Got {pos_after_1x}"
    );

    // ── Reset position to 0.5 ──────────────────────────────────────
    sp_ref.borrow_mut().set_position(0.5);

    // ── At 2x zoom ─────────────────────────────────────────────────
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    // Verify position is still 0.5 after zoom change.
    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "Splitter position should remain 0.5 after zoom change"
    );

    // Drag at 2x zoom from grip center to ~30%.
    h.drag(400.0, 300.0, 240.0, 300.0);

    let pos_after_2x = sp_ref.borrow().position();

    assert!(
        (pos_after_2x - 0.3).abs() < 0.1,
        "After dragging to ~30% at 2x, position should be near 0.3. Got {pos_after_2x}"
    );
}

// ---------------------------------------------------------------------------
// Test: Vertical Splitter drag at 1x and 2x zoom
// ---------------------------------------------------------------------------

/// Vertical Splitter drag through the full pipeline at 1x and 2x zoom.
///
/// The Splitter's `input()` method computes grip geometry in normalized
/// `(1.0, tallness)` panel-local space, matching the coordinate system
/// used by the pipeline for mouse coordinates.
#[test]
fn splitter_drag_vertical_1x_and_2x() {
    let (mut h, sp_ref, mut compositor) = setup_splitter(Orientation::Vertical, 0.5);

    // Verify initial state.
    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "Vertical splitter should start at position 0.5"
    );

    // ── At 1x zoom ─────────────────────────────────────────────────
    // Grip center at panel_y=0.5 maps to view_y=400 at 1x.
    // Drag target ~30%: panel_y=0.3 maps to view_y=240.
    h.drag(400.0, 400.0, 400.0, 240.0);

    let pos_after_1x = sp_ref.borrow().position();

    assert!(
        (pos_after_1x - 0.3).abs() < 0.1,
        "After dragging to ~30%, vertical position should be near 0.3. Got {pos_after_1x}"
    );

    // ── Reset position to 0.5 ──────────────────────────────────────
    sp_ref.borrow_mut().set_position(0.5);

    // ── At 2x zoom ─────────────────────────────────────────────────
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "Vertical splitter position should remain 0.5 after zoom change"
    );

    // At 2x: viewed_width=1600, viewed_y=-300.
    // Grip center at panel_y=0.5: view_y = -300 + 0.5*1600 = 500.
    // Target ~30%: view_y = -300 + 0.3*1600 = 180.
    h.drag(400.0, 500.0, 400.0, 180.0);

    let pos_after_2x = sp_ref.borrow().position();

    assert!(
        (pos_after_2x - 0.3).abs() < 0.1,
        "After dragging to ~30% at 2x, vertical position should be near 0.3. Got {pos_after_2x}"
    );
}

// ---------------------------------------------------------------------------
// Test: Splitter position() and set_position() are coherent across zoom
// ---------------------------------------------------------------------------

/// Verify that programmatic position changes are preserved across zoom changes.
/// This does NOT involve drag -- it tests that set_position/position round-trip
/// correctly and that zooming + re-rendering does not alter position.
#[test]
fn splitter_position_stable_across_zoom() {
    let (mut h, sp_ref, mut compositor) = setup_splitter(Orientation::Horizontal, 0.25);

    // Initial position at 1x.
    assert!(
        (sp_ref.borrow().position() - 0.25).abs() < 0.001,
        "Splitter should start at position 0.25"
    );

    // Change to 2x zoom, re-render. Position should not change.
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    assert!(
        (sp_ref.borrow().position() - 0.25).abs() < 0.001,
        "Splitter position should remain 0.25 after zoom to 2x"
    );

    // Programmatically change position at 2x zoom.
    sp_ref.borrow_mut().set_position(0.75);
    assert!(
        (sp_ref.borrow().position() - 0.75).abs() < 0.001,
        "set_position(0.75) should set position to 0.75 at 2x"
    );

    // Zoom back to 1x. Position should remain 0.75.
    h.set_zoom(1.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    assert!(
        (sp_ref.borrow().position() - 0.75).abs() < 0.001,
        "Splitter position should remain 0.75 after returning to 1x"
    );
}

// ---------------------------------------------------------------------------
// Test: Splitter clamping with limits
// ---------------------------------------------------------------------------

/// Verify that set_position respects min/max limits at both zoom levels.
#[test]
fn splitter_limits_respected_across_zoom() {
    let (mut h, sp_ref, mut compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Set limits to [0.2, 0.8].
    sp_ref.borrow_mut().set_limits(0.2, 0.8);

    // Verify position is still 0.5 (within limits).
    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "Position 0.5 should remain within [0.2, 0.8] limits"
    );

    // Try to set position below minimum.
    sp_ref.borrow_mut().set_position(0.0);
    assert!(
        (sp_ref.borrow().position() - 0.2).abs() < 0.001,
        "Position should be clamped to min_position 0.2, got {}",
        sp_ref.borrow().position()
    );

    // Try to set position above maximum.
    sp_ref.borrow_mut().set_position(1.0);
    assert!(
        (sp_ref.borrow().position() - 0.8).abs() < 0.001,
        "Position should be clamped to max_position 0.8, got {}",
        sp_ref.borrow().position()
    );

    // Zoom to 2x -- clamped position should be preserved.
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    assert!(
        (sp_ref.borrow().position() - 0.8).abs() < 0.001,
        "Clamped position 0.8 should be preserved after zoom to 2x, got {}",
        sp_ref.borrow().position()
    );

    // Verify limits still work at 2x.
    sp_ref.borrow_mut().set_position(0.0);
    assert!(
        (sp_ref.borrow().position() - 0.2).abs() < 0.001,
        "Position should be clamped to min 0.2 at 2x zoom, got {}",
        sp_ref.borrow().position()
    );

    sp_ref.borrow_mut().set_position(1.0);
    assert!(
        (sp_ref.borrow().position() - 0.8).abs() < 0.001,
        "Position should be clamped to max 0.8 at 2x zoom, got {}",
        sp_ref.borrow().position()
    );
}
