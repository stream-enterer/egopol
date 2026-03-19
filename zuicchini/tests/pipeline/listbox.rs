//! Systematic interaction test for ListBox at 1x and 2x zoom, driven through
//! the full input dispatch pipeline (PipelineTestHarness).
//!
//! Verifies that clicking on different items selects the correct item at both
//! zoom levels, using view-space coordinates derived from the panel's viewed
//! geometry and the border's content rect.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputState};
use zuicchini::panel::{NoticeFlags, PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{
    Border, InnerBorderType, ListBox, Look, OuterBorderType, SelectionMode,
};

use super::support::pipeline::PipelineTestHarness;

/// PanelBehavior wrapper for ListBox, allowing shared access via Rc<RefCell>.
///
/// Copied from `behavioral_interaction.rs` SharedListBoxPanel pattern.
struct SharedListBoxPanel {
    inner: Rc<RefCell<ListBox>>,
}

impl PanelBehavior for SharedListBoxPanel {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        self.inner.borrow_mut().paint(painter, w, h);
    }

    fn input(
        &mut self,
        event: &InputEvent,
        state: &PanelState,
        input_state: &InputState,
    ) -> bool {
        self.inner.borrow_mut().input(event, state, input_state)
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.inner
                .borrow_mut()
                .on_focus_changed(state.in_active_path);
        }
        if flags.intersects(NoticeFlags::ENABLE_CHANGED) {
            self.inner.borrow_mut().on_enable_changed(state.enabled);
        }
    }

    fn is_opaque(&self) -> bool {
        true
    }

    fn get_cursor(&self) -> Cursor {
        Cursor::Normal
    }
}

/// Compute the view-space Y coordinate for the vertical center of item `n`
/// (0-indexed) in a ListBox with `item_count` items.
///
/// The items are positioned within the border's content rect in panel-local
/// space (x in [0,1], y in [0,tallness]). This function:
///   1. Constructs a border matching ListBox's default config
///   2. Queries content_rect_unobscured in normalized panel-local space
///   3. Computes item N's center within the content rect
///   4. Maps the panel-local coordinate to view space using the viewed rect
fn item_center_view_y(
    vr: &zuicchini::foundation::Rect,
    pixel_tallness: f64,
    n: usize,
    item_count: usize,
) -> f64 {
    let look = Look::new();

    // Reconstruct the border with the same config as ListBox::new.
    let border = Border::new(OuterBorderType::Instrument)
        .with_inner(InnerBorderType::InputField)
        .with_how_to(true);

    // Panel-local coordinate space: x in [0, 1], y in [0, tallness].
    // tallness = (panel_pixel_h / panel_pixel_w) * pixel_tallness
    let tallness = (vr.h / vr.w) * pixel_tallness;

    let cr = border.content_rect_unobscured(1.0, tallness, &look);

    // Item N's center Y in panel-local space.
    let item_local_y = cr.y + (n as f64 + 0.5) / item_count as f64 * cr.h;

    // Map panel-local Y to view-space Y.
    // panel-local y in [0, tallness] maps to view-space [vr.y, vr.y + vr.h].
    vr.y + (item_local_y / tallness) * vr.h
}

/// Compute the view-space X coordinate at the horizontal center of the
/// content rect.
fn content_center_view_x(
    vr: &zuicchini::foundation::Rect,
    pixel_tallness: f64,
) -> f64 {
    let look = Look::new();
    let border = Border::new(OuterBorderType::Instrument)
        .with_inner(InnerBorderType::InputField)
        .with_how_to(true);

    let tallness = (vr.h / vr.w) * pixel_tallness;
    let cr = border.content_rect_unobscured(1.0, tallness, &look);

    let local_x = cr.x + cr.w * 0.5;
    vr.x + local_x * vr.w
}

#[test]
fn listbox_click_items_1x_and_2x() {
    // 1. Create PipelineTestHarness (800x600 viewport).
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // 2. Create ListBox with 5 items, SelectionMode::Single.
    let look = Look::new();
    let mut lb = ListBox::new(look);
    lb.set_selection_mode(SelectionMode::Single);
    lb.add_item("item0".to_string(), "Alpha".to_string());
    lb.add_item("item1".to_string(), "Beta".to_string());
    lb.add_item("item2".to_string(), "Gamma".to_string());
    lb.add_item("item3".to_string(), "Delta".to_string());
    lb.add_item("item4".to_string(), "Epsilon".to_string());

    let lb_ref = Rc::new(RefCell::new(lb));

    // 3. Wrap in SharedListBoxPanel and add to tree.
    let panel_id = h.add_panel_with(
        root,
        "listbox",
        Box::new(SharedListBoxPanel {
            inner: lb_ref.clone(),
        }),
    );

    // 4. Tick + render (SoftwareCompositor) to populate last_w/last_h.
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    let pt = h.view.pixel_tallness();

    // ---------- 5. At 1x zoom ----------

    let state = h.tree.build_panel_state(
        panel_id,
        h.view.window_focused(),
        pt,
    );
    let vr = state.viewed_rect;
    let click_x = content_center_view_x(&vr, pt);

    // Click item 0
    h.click(click_x, item_center_view_y(&vr, pt, 0, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(0),
        "At 1x zoom: clicking item 0 should select it"
    );

    // Click item 2
    h.click(click_x, item_center_view_y(&vr, pt, 2, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(2),
        "At 1x zoom: clicking item 2 should select it"
    );

    // Click item 4
    h.click(click_x, item_center_view_y(&vr, pt, 4, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(4),
        "At 1x zoom: clicking item 4 should select it"
    );

    // ---------- 6. At 2x zoom ----------

    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    let state_2x = h.tree.build_panel_state(
        panel_id,
        h.view.window_focused(),
        pt,
    );
    let vr2 = state_2x.viewed_rect;
    let click_x_2x = content_center_view_x(&vr2, pt);

    // Click item 0
    h.click(click_x_2x, item_center_view_y(&vr2, pt, 0, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(0),
        "At 2x zoom: clicking item 0 should select it"
    );

    // Click item 2
    h.click(click_x_2x, item_center_view_y(&vr2, pt, 2, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(2),
        "At 2x zoom: clicking item 2 should select it"
    );

    // Click item 4
    h.click(click_x_2x, item_center_view_y(&vr2, pt, 4, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(4),
        "At 2x zoom: clicking item 4 should select it"
    );
}
