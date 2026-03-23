//! Systematic interaction test for emSplitter at 1x and 2x zoom, driven through
//! the full input dispatch pipeline (PipelineTestHarness).
//!
//! Verifies emSplitter drag behavior when dispatched through the coordinate-
//! transform pipeline at different zoom levels.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::emCore::emCursor::emCursor;
use zuicchini::emCore::emInput::{emInputEvent, InputKey};
use zuicchini::emCore::emInputState::emInputState;
use zuicchini::emCore::emTiling::Orientation;
use zuicchini::emCore::emPanel::{PanelBehavior, PanelState};
use zuicchini::emCore::emPainter::emPainter;
use zuicchini::emCore::emViewRenderer::SoftwareCompositor;
use zuicchini::emCore::emLook::emLook;
use zuicchini::emCore::emSplitter::emSplitter;

use super::support::pipeline::PipelineTestHarness;

// ---------------------------------------------------------------------------
// PanelBehavior wrapper for emSplitter (shared via Rc<RefCell>)
// ---------------------------------------------------------------------------

struct SharedSplitterPanel {
    inner: Rc<RefCell<emSplitter>>,
}

impl PanelBehavior for SharedSplitterPanel {
    fn paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        self.inner.borrow_mut().paint(painter, w, h, state.enabled);
    }

    fn input(
        &mut self,
        event: &emInputEvent,
        state: &PanelState,
        input_state: &emInputState,
    ) -> bool {
        self.inner.borrow_mut().input(event, state, input_state)
    }

    fn get_cursor(&self) -> emCursor {
        self.inner.borrow().get_cursor()
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Helper: create a harness with a shared emSplitter at a given position.
// ---------------------------------------------------------------------------

fn setup_splitter(
    orientation: Orientation,
    initial_pos: f64,
) -> (PipelineTestHarness, Rc<RefCell<emSplitter>>, SoftwareCompositor) {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = emLook::new();
    let mut sp = emSplitter::new(orientation, look);
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
// Test: Horizontal emSplitter drag at 1x and 2x zoom
// ---------------------------------------------------------------------------

/// Horizontal emSplitter drag through the full pipeline at 1x and 2x zoom.
///
/// The emSplitter's `input()` method computes grip geometry in normalized
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
// Test: Vertical emSplitter drag at 1x and 2x zoom
// ---------------------------------------------------------------------------

/// Vertical emSplitter drag through the full pipeline at 1x and 2x zoom.
///
/// The emSplitter's `input()` method computes grip geometry in normalized
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
// Test: emSplitter position() and set_position() are coherent across zoom
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
// Test: emSplitter clamping with limits
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

// ---------------------------------------------------------------------------
// BP-11: emSplitter drag behavioral parity tests
// ---------------------------------------------------------------------------

// Helper that also returns the panel id (needed for enable_switch tests).
fn setup_splitter_with_id(
    orientation: Orientation,
    initial_pos: f64,
) -> (PipelineTestHarness, Rc<RefCell<emSplitter>>, SoftwareCompositor, zuicchini::emCore::emPanelTree::PanelId) {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = emLook::new();
    let mut sp = emSplitter::new(orientation, look);
    sp.set_position(initial_pos);
    let sp_ref = Rc::new(RefCell::new(sp));

    let panel_id = h.add_panel_with(
        root,
        "splitter",
        Box::new(SharedSplitterPanel {
            inner: sp_ref.clone(),
        }),
    );
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    (h, sp_ref, compositor, panel_id)
}

/// BP-11: Press on the grip enters dragging state.
/// C++ ref: emSplitter.cpp:144-150 — Pressed=true on left-button in grip.
#[test]
fn splitter_press_on_grip_starts_drag() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    assert!(!sp_ref.borrow().is_dragging(), "should not be dragging initially");

    // Press at grip center (view x=400 at 1x maps to panel x≈0.5 which hits
    // the grip centered at 0.5).
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);

    assert!(
        sp_ref.borrow().is_dragging(),
        "pressing on the grip should enter dragging state"
    );
}

/// BP-11: Move during drag updates position continuously.
/// C++ ref: emSplitter.cpp:117-137 — position updates on mouse move while Pressed.
#[test]
fn splitter_move_during_drag_updates_position() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);
    assert!(sp_ref.borrow().is_dragging());

    // Move to ~60% of viewport width (480px).
    let move1 = emInputEvent::mouse_move(InputKey::MouseLeft, 480.0, 300.0);
    h.dispatch(&move1);
    let pos1 = sp_ref.borrow().position();
    assert!(
        (pos1 - 0.6).abs() < 0.1,
        "position should be near 0.6 after move to 480px, got {pos1}"
    );

    // Move again to ~80% (640px).
    let move2 = emInputEvent::mouse_move(InputKey::MouseLeft, 640.0, 300.0);
    h.dispatch(&move2);
    let pos2 = sp_ref.borrow().position();
    assert!(
        (pos2 - 0.8).abs() < 0.15,
        "position should be near 0.8 after move to 640px, got {pos2}"
    );

    // Position should have changed between the two moves.
    assert!(
        (pos2 - pos1).abs() > 0.05,
        "position should update continuously during drag"
    );
}

/// BP-11: Release after drag clears dragging state.
/// C++ ref: emSplitter.cpp:138-142 — Pressed=false when button released.
#[test]
fn splitter_release_ends_drag() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);
    assert!(sp_ref.borrow().is_dragging());

    // Release.
    let release = emInputEvent::release(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&release);
    assert!(
        !sp_ref.borrow().is_dragging(),
        "releasing the mouse should clear dragging state"
    );
}

/// BP-11: Press outside the grip does not start drag.
/// C++ ref: emSplitter.cpp:144 — gated on MouseInGrip hit test.
#[test]
fn splitter_press_outside_grip_no_drag() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Press far from the grip (x=100, which is ~12.5% of viewport — well away
    // from grip at ~50%).
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(100.0, 300.0);
    h.dispatch(&press);

    assert!(
        !sp_ref.borrow().is_dragging(),
        "pressing outside the grip should not start a drag"
    );

    // Position should remain unchanged.
    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "position should remain 0.5 when clicking outside grip"
    );
}

/// BP-11: Drag beyond max clamps position to max_position.
/// C++ ref: emSplitter.cpp:124/134 → SetPos → clamp(min,max).
#[test]
fn splitter_drag_clamp_to_max() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);

    // Drag far to the right (beyond the viewport).
    let move_ev = emInputEvent::mouse_move(InputKey::MouseLeft, 900.0, 300.0);
    h.dispatch(&move_ev);

    assert!(
        (sp_ref.borrow().position() - 1.0).abs() < 0.001,
        "dragging beyond right edge should clamp to max (1.0), got {}",
        sp_ref.borrow().position()
    );
}

/// BP-11: Drag below min clamps position to min_position.
/// C++ ref: emSplitter.cpp:124/134 → SetPos → clamp(min,max).
#[test]
fn splitter_drag_clamp_to_min() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);

    // Drag far to the left (beyond the viewport).
    let move_ev = emInputEvent::mouse_move(InputKey::MouseLeft, -100.0, 300.0);
    h.dispatch(&move_ev);

    assert!(
        (sp_ref.borrow().position() - 0.0).abs() < 0.001,
        "dragging beyond left edge should clamp to min (0.0), got {}",
        sp_ref.borrow().position()
    );
}

/// BP-11: Drag with custom limits [0.2, 0.8] clamps correctly.
/// C++ ref: emSplitter.cpp:124/134 → SetPos → clamp(MinPos,MaxPos).
#[test]
fn splitter_drag_with_custom_limits() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    sp_ref.borrow_mut().set_limits(0.2, 0.8);

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);

    // Drag far right — should clamp to 0.8.
    let move_right = emInputEvent::mouse_move(InputKey::MouseLeft, 900.0, 300.0);
    h.dispatch(&move_right);
    assert!(
        (sp_ref.borrow().position() - 0.8).abs() < 0.001,
        "drag right should clamp to max_position 0.8, got {}",
        sp_ref.borrow().position()
    );

    // Release and re-press at the new grip position (~80% = 640px).
    let release = emInputEvent::release(InputKey::MouseLeft).with_mouse(900.0, 300.0);
    h.dispatch(&release);

    // Reset to middle of range.
    sp_ref.borrow_mut().set_position(0.5);

    // Re-render so paint caches are updated.
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    // Press at new grip center (~50% = 400px).
    let press2 = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press2);

    // Drag far left — should clamp to 0.2.
    let move_left = emInputEvent::mouse_move(InputKey::MouseLeft, -100.0, 300.0);
    h.dispatch(&move_left);
    assert!(
        (sp_ref.borrow().position() - 0.2).abs() < 0.001,
        "drag left should clamp to min_position 0.2, got {}",
        sp_ref.borrow().position()
    );
}

/// BP-11: on_position callback fires during drag.
/// C++ ref: emSplitter.cpp:124/134 → SetPos → PosSignal emission.
#[test]
fn splitter_on_position_callback_fires_during_drag() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Horizontal, 0.5);

    let positions: Rc<RefCell<Vec<f64>>> = Rc::new(RefCell::new(Vec::new()));
    let positions_clone = positions.clone();
    sp_ref.borrow_mut().on_position = Some(Box::new(move |pos| {
        positions_clone.borrow_mut().push(pos);
    }));

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);

    // Move to a new position.
    let move_ev = emInputEvent::mouse_move(InputKey::MouseLeft, 560.0, 300.0);
    h.dispatch(&move_ev);

    let recorded = positions.borrow();
    assert!(
        !recorded.is_empty(),
        "on_position callback should have fired at least once during drag"
    );
    // The last recorded position should match the current splitter position.
    let last = *recorded.last().unwrap();
    assert!(
        (last - sp_ref.borrow().position()).abs() < 0.001,
        "last callback position ({last}) should match current position ({})",
        sp_ref.borrow().position()
    );
}

/// BP-11: Disabled splitter rejects input (press on grip does not start drag).
/// C++ ref: emSplitter.cpp:144 — gated on IsEnabled().
#[test]
fn splitter_disabled_rejects_input() {
    let (mut h, sp_ref, mut compositor, panel_id) =
        setup_splitter_with_id(Orientation::Horizontal, 0.5);

    // Disable the panel via the tree.
    h.tree.set_enable_switch(panel_id, false);
    h.tick_n(3);
    // Re-render so the emSplitter caches enabled=false from the paint call.
    compositor.render(&mut h.tree, &h.view);

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 300.0);
    h.dispatch(&press);

    assert!(
        !sp_ref.borrow().is_dragging(),
        "disabled splitter should not enter dragging state"
    );

    // Position should remain unchanged.
    assert!(
        (sp_ref.borrow().position() - 0.5).abs() < 0.001,
        "disabled splitter position should remain 0.5, got {}",
        sp_ref.borrow().position()
    );
}

/// BP-11: Vertical splitter drag — press on grip, move, release.
/// C++ ref: emSplitter.cpp:118-126 — vertical branch (mig=my-gy).
#[test]
fn splitter_vertical_drag_states() {
    let (mut h, sp_ref, _compositor) = setup_splitter(Orientation::Vertical, 0.5);

    assert!(!sp_ref.borrow().is_dragging());

    // With layout_rect (0,0,1,1) and viewed_width=800, paint receives
    // w=800, h=800 (paint_h = vw * layout_h/layout_w = 800*1=800).
    // So tallness = 800/800 = 1.0. Vertical grip: gs = 0.015*1.0 = 0.015,
    // gy = 0.5*(1.0-0.015) = 0.4925. panel_y = vy/800 (pixel_tallness=1.0).
    // Grip center at panel_y ≈ 0.5 → vy = 0.5*800 = 400.

    // Press at grip center.
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(400.0, 400.0);
    h.dispatch(&press);
    assert!(
        sp_ref.borrow().is_dragging(),
        "pressing on vertical grip should start drag"
    );

    // Drag upward to ~30%: panel_y = 0.3 → vy = 0.3 * 800 = 240.
    let move_ev = emInputEvent::mouse_move(InputKey::MouseLeft, 400.0, 240.0);
    h.dispatch(&move_ev);
    let pos = sp_ref.borrow().position();
    assert!(
        (pos - 0.3).abs() < 0.1,
        "vertical drag to 30% should move position near 0.3, got {pos}"
    );

    // Release.
    let release = emInputEvent::release(InputKey::MouseLeft).with_mouse(400.0, 240.0);
    h.dispatch(&release);
    assert!(
        !sp_ref.borrow().is_dragging(),
        "releasing should clear vertical dragging state"
    );
}
