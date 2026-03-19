//! Systematic interaction tests for CheckButton and CheckBox at 1x and 2x zoom,
//! driven through the full input dispatch pipeline (PipelineTestHarness).
//!
//! These tests verify that mouse clicks toggle the checked state correctly when
//! dispatched through the coordinate-transform pipeline at different zoom levels.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{CheckBox, CheckButton, Look};

use super::support::pipeline::PipelineTestHarness;

// ---------------------------------------------------------------------------
// PanelBehavior wrapper for CheckButton (shared via Rc<RefCell>)
// ---------------------------------------------------------------------------

struct SharedCheckButtonPanel {
    inner: Rc<RefCell<CheckButton>>,
}

impl PanelBehavior for SharedCheckButtonPanel {
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
// PanelBehavior wrapper for CheckBox (shared via Rc<RefCell>)
// ---------------------------------------------------------------------------

struct SharedCheckBoxPanel {
    inner: Rc<RefCell<CheckBox>>,
}

impl PanelBehavior for SharedCheckBoxPanel {
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
// Test 1: CheckButton toggle at 1x and 2x zoom
// ---------------------------------------------------------------------------

#[test]
fn checkbutton_toggle_1x_and_2x() {
    // 1. Create PipelineTestHarness (800x600 viewport).
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // 2. Create CheckButton (initially unchecked).
    let look = Look::new();
    let cb = CheckButton::new("Toggle Me", look);
    let cb_ref = Rc::new(RefCell::new(cb));

    // 3. Wrap in PanelBehavior, add to tree, tick + render.
    let _panel_id = h.add_panel_with(
        root,
        "check_button",
        Box::new(SharedCheckButtonPanel {
            inner: cb_ref.clone(),
        }),
    );
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    // Verify initial state.
    assert!(
        !cb_ref.borrow().is_checked(),
        "CheckButton should start unchecked"
    );

    // 4. At 1x: click center -> assert checked() == true.
    h.click(400.0, 300.0);
    assert!(
        cb_ref.borrow().is_checked(),
        "CheckButton should be checked after first click at 1x"
    );

    // 5. Click again -> assert checked() == false (toggle back).
    h.click(400.0, 300.0);
    assert!(
        !cb_ref.borrow().is_checked(),
        "CheckButton should be unchecked after second click at 1x"
    );

    // 6. At 2x: set_zoom(2.0), tick, render. Click center -> assert checked() == true.
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    h.click(400.0, 300.0);
    assert!(
        cb_ref.borrow().is_checked(),
        "CheckButton should be checked after first click at 2x"
    );

    // 7. Click again -> assert checked() == false.
    h.click(400.0, 300.0);
    assert!(
        !cb_ref.borrow().is_checked(),
        "CheckButton should be unchecked after second click at 2x"
    );
}

// ---------------------------------------------------------------------------
// Test 2: CheckBox toggle at 1x and 2x zoom
// ---------------------------------------------------------------------------

#[test]
fn checkbox_toggle_1x_and_2x() {
    // 1. Create PipelineTestHarness (800x600 viewport).
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // 2. Create CheckBox (initially unchecked).
    let look = Look::new();
    let cb = CheckBox::new("Enable Option", look);
    let cb_ref = Rc::new(RefCell::new(cb));

    // 3. Wrap in PanelBehavior, add to tree, tick + render.
    let _panel_id = h.add_panel_with(
        root,
        "check_box",
        Box::new(SharedCheckBoxPanel {
            inner: cb_ref.clone(),
        }),
    );
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    // Verify initial state.
    assert!(
        !cb_ref.borrow().is_checked(),
        "CheckBox should start unchecked"
    );

    // 4. At 1x: click center -> assert is_checked() == true.
    h.click(400.0, 300.0);
    assert!(
        cb_ref.borrow().is_checked(),
        "CheckBox should be checked after first click at 1x"
    );

    // 5. Click again -> assert is_checked() == false (toggle back).
    h.click(400.0, 300.0);
    assert!(
        !cb_ref.borrow().is_checked(),
        "CheckBox should be unchecked after second click at 1x"
    );

    // 6. At 2x: set_zoom(2.0), tick, render. Click center -> assert is_checked() == true.
    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    h.click(400.0, 300.0);
    assert!(
        cb_ref.borrow().is_checked(),
        "CheckBox should be checked after first click at 2x"
    );

    // 7. Click again -> assert is_checked() == false.
    h.click(400.0, 300.0);
    assert!(
        !cb_ref.borrow().is_checked(),
        "CheckBox should be unchecked after second click at 2x"
    );
}
