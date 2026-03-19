//! Systematic interaction tests for TextField at 1x and 2x zoom.
//!
//! These tests drive input through the full PipelineTestHarness dispatch
//! pipeline (VIF chain, hit test, coordinate transform, keyboard suppression)
//! and assert on widget STATE (text content, cursor position), not pixels.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputKey, InputState};
use zuicchini::panel::{NoticeFlags, PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{Look, TextField};

use super::support::pipeline::PipelineTestHarness;

// ---------------------------------------------------------------------------
// SharedTextFieldPanel -- PanelBehavior wrapper with shared TextField access
// ---------------------------------------------------------------------------

/// PanelBehavior wrapper for TextField. The widget is stored behind
/// Rc<RefCell> so the test can inspect state after input dispatch.
struct SharedTextFieldPanel {
    inner: Rc<RefCell<TextField>>,
}

impl PanelBehavior for SharedTextFieldPanel {
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

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.inner
                .borrow_mut()
                .on_focus_changed(state.in_active_path);
        }
    }

    fn get_cursor(&self) -> Cursor {
        self.inner.borrow().get_cursor()
    }

    fn is_opaque(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Helper: set up a pipeline harness with a single editable TextField panel
// ---------------------------------------------------------------------------

/// Create a PipelineTestHarness with an editable TextField child panel
/// filling the entire root. Returns the harness and the shared TextField ref.
fn setup_textfield_harness() -> (PipelineTestHarness, Rc<RefCell<TextField>>) {
    let look = Look::new();
    let mut tf = TextField::new(look);
    tf.set_editable(true);

    let tf_ref = Rc::new(RefCell::new(tf));

    let mut h = PipelineTestHarness::new();
    let root = h.root();
    let _panel_id = h.add_panel_with(
        root,
        "text_field",
        Box::new(SharedTextFieldPanel {
            inner: tf_ref.clone(),
        }),
    );

    // Settle layout.
    h.tick_n(5);

    (h, tf_ref)
}

/// Render the harness at the given viewport size so that paint() is called on
/// the TextField, populating its cached last_w / last_h dimensions (required
/// for mouse hit-testing and the min_ext guard in input()).
fn render(h: &mut PipelineTestHarness, width: u32, height: u32) {
    let mut compositor = SoftwareCompositor::new(width, height);
    compositor.render(&mut h.tree, &h.view);
}

/// Type a string character-by-character through the pipeline using press_char.
fn type_string(h: &mut PipelineTestHarness, s: &str) {
    for ch in s.chars() {
        h.press_char(ch);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

/// Type "abc" at 1x zoom and "xyz" at 2x zoom. Verify the text content after
/// each sequence.
#[test]
fn textfield_type_1x_and_2x() {
    let (mut h, tf_ref) = setup_textfield_harness();

    // ── 1x zoom ────────────────────────────────────────────────────────
    render(&mut h, 800, 600);

    // Click at viewport center to focus the text field panel.
    h.click(400.0, 300.0);

    // Type "abc" through the full pipeline.
    type_string(&mut h, "abc");

    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abc",
            "After typing 'abc' at 1x zoom, text should be 'abc' but got '{}'",
            tf.text()
        );
        assert_eq!(
            tf.cursor_pos(),
            3,
            "Cursor should be at end of 'abc' (byte 3), got {}",
            tf.cursor_pos()
        );
    }

    // ── 2x zoom ────────────────────────────────────────────────────────
    // Clear the field via direct API (dispatch doesn't expose modifier keys).
    tf_ref.borrow_mut().set_text("");
    assert_eq!(tf_ref.borrow().text(), "", "Text should be cleared");

    // Zoom to 2x.
    h.set_zoom(2.0);
    h.tick_n(5);
    render(&mut h, 800, 600);

    // Click at viewport center to re-focus.
    h.click(400.0, 300.0);

    // Type "xyz" at 2x zoom.
    type_string(&mut h, "xyz");

    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "xyz",
            "After typing 'xyz' at 2x zoom, text should be 'xyz' but got '{}'",
            tf.text()
        );
        assert_eq!(
            tf.cursor_pos(),
            3,
            "Cursor should be at end of 'xyz' (byte 3), got {}",
            tf.cursor_pos()
        );
    }
}

/// Verify that Backspace deletes the last character at both zoom levels.
#[test]
fn textfield_backspace_1x_and_2x() {
    let (mut h, tf_ref) = setup_textfield_harness();
    render(&mut h, 800, 600);

    // Focus
    h.click(400.0, 300.0);

    // ── 1x: type "hello", backspace twice → "hel" ─────────────────────
    type_string(&mut h, "hello");
    assert_eq!(tf_ref.borrow().text(), "hello");

    h.press_key(InputKey::Backspace);
    h.press_key(InputKey::Backspace);

    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "hel",
            "After 2 backspaces from 'hello', expected 'hel' but got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 3);
    }

    // ── 2x: clear, type "world", backspace once → "worl" ──────────────
    tf_ref.borrow_mut().set_text("");

    h.set_zoom(2.0);
    h.tick_n(5);
    render(&mut h, 800, 600);

    h.click(400.0, 300.0);
    type_string(&mut h, "world");
    assert_eq!(tf_ref.borrow().text(), "world");

    h.press_key(InputKey::Backspace);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "worl",
            "After 1 backspace from 'world' at 2x, expected 'worl' but got '{}'",
            tf.text()
        );
    }
}

/// Verify arrow key navigation moves the cursor correctly.
#[test]
fn textfield_arrow_navigation() {
    let (mut h, tf_ref) = setup_textfield_harness();
    render(&mut h, 800, 600);

    // Focus and type initial text.
    h.click(400.0, 300.0);
    type_string(&mut h, "abcde");
    assert_eq!(tf_ref.borrow().cursor_pos(), 5);

    // ArrowLeft 3 times → cursor at position 2.
    h.press_key(InputKey::ArrowLeft);
    h.press_key(InputKey::ArrowLeft);
    h.press_key(InputKey::ArrowLeft);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        2,
        "After 3 left arrows from pos 5, cursor should be at 2"
    );

    // ArrowRight once → cursor at 3.
    h.press_key(InputKey::ArrowRight);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        3,
        "After 1 right arrow from pos 2, cursor should be at 3"
    );

    // Home → cursor at 0.
    h.press_key(InputKey::Home);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        0,
        "Home should move cursor to 0"
    );

    // End → cursor at 5.
    h.press_key(InputKey::End);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        5,
        "End should move cursor to 5 (end of 'abcde')"
    );
}

/// Verify that typing inserts at the cursor position (mid-string insertion).
#[test]
fn textfield_insert_at_cursor() {
    let (mut h, tf_ref) = setup_textfield_harness();
    render(&mut h, 800, 600);

    h.click(400.0, 300.0);
    type_string(&mut h, "ac");
    assert_eq!(tf_ref.borrow().text(), "ac");

    // Move cursor left once (between 'a' and 'c'), then type 'b'.
    h.press_key(InputKey::ArrowLeft);
    assert_eq!(tf_ref.borrow().cursor_pos(), 1);

    h.press_char('b');
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abc",
            "Inserting 'b' between 'a' and 'c' should produce 'abc', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 2, "Cursor should advance to 2 after insert");
    }
}

/// Verify Delete key removes the character AFTER the cursor.
#[test]
fn textfield_delete_key() {
    let (mut h, tf_ref) = setup_textfield_harness();
    render(&mut h, 800, 600);

    h.click(400.0, 300.0);
    type_string(&mut h, "abcd");

    // Move to position 1 (after 'a').
    h.press_key(InputKey::Home);
    h.press_key(InputKey::ArrowRight);
    assert_eq!(tf_ref.borrow().cursor_pos(), 1);

    // Delete should remove 'b'.
    h.press_key(InputKey::Delete);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "acd",
            "Delete at pos 1 in 'abcd' should produce 'acd', got '{}'",
            tf.text()
        );
        assert_eq!(
            tf.cursor_pos(),
            1,
            "Cursor should remain at 1 after delete"
        );
    }
}

/// Verify that a non-editable TextField rejects typed characters.
#[test]
fn textfield_non_editable_rejects_input() {
    let (mut h, tf_ref) = setup_textfield_harness();

    // Make the field non-editable.
    tf_ref.borrow_mut().set_editable(false);

    render(&mut h, 800, 600);
    h.click(400.0, 300.0);

    type_string(&mut h, "abc");

    assert_eq!(
        tf_ref.borrow().text(),
        "",
        "Non-editable TextField should not accept typed characters"
    );
}

/// Verify that pre-populated text is preserved and new text appends correctly.
#[test]
fn textfield_prepopulated_text() {
    let (mut h, tf_ref) = setup_textfield_harness();

    // Pre-populate the text field.
    tf_ref.borrow_mut().set_text("hello");

    render(&mut h, 800, 600);
    h.click(400.0, 300.0);

    // The click positions the cursor at the click location within the text,
    // so move to the end explicitly before typing.
    h.press_key(InputKey::End);
    assert_eq!(tf_ref.borrow().cursor_pos(), 5);

    // Type additional text.
    type_string(&mut h, "!");

    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "hello!",
            "Typing '!' after 'hello' should produce 'hello!', got '{}'",
            tf.text()
        );
    }
}

/// Combined test: type at 1x, switch to 2x, type more, verify full text.
#[test]
fn textfield_type_across_zoom_levels() {
    let (mut h, tf_ref) = setup_textfield_harness();

    // ── 1x: type "foo" ─────────────────────────────────────────────────
    render(&mut h, 800, 600);
    h.click(400.0, 300.0);
    type_string(&mut h, "foo");
    assert_eq!(tf_ref.borrow().text(), "foo");

    // ── Switch to 2x and type "bar" ────────────────────────────────────
    h.set_zoom(2.0);
    h.tick_n(5);
    render(&mut h, 800, 600);

    // Click at a slightly different position to avoid double-click detection
    // with the prior click (same coords within 500ms would trigger word
    // selection, replacing existing text on the next typed character).
    h.click(410.0, 310.0);

    // Move cursor to end so we append after "foo".
    h.press_key(InputKey::End);
    type_string(&mut h, "bar");

    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "foobar",
            "After typing 'foo' at 1x and 'bar' at 2x, text should be 'foobar', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 6);
    }
}
