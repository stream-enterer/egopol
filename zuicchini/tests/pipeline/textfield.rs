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

// ===========================================================================
// BP-4: TextField cursor navigation tests
// ===========================================================================

/// Helper: set up a focused, editable TextField pre-populated with `text`,
/// cursor at `cursor_pos`. Returns harness + shared TextField ref.
fn setup_nav_harness(text: &str, cursor_pos: usize) -> (PipelineTestHarness, Rc<RefCell<TextField>>) {
    let (mut h, tf_ref) = setup_textfield_harness();
    tf_ref.borrow_mut().set_text(text);
    tf_ref.borrow_mut().set_cursor_index(cursor_pos);

    render(&mut h, 800, 600);
    h.click(400.0, 300.0);

    // After click, cursor may have moved to click position; restore it.
    tf_ref.borrow_mut().set_cursor_index(cursor_pos);
    // Clear any selection that the click may have created.
    tf_ref.borrow_mut().deselect();

    (h, tf_ref)
}

/// Helper: set up a focused, editable, multi-line TextField.
fn setup_multiline_nav_harness(
    text: &str,
    cursor_pos: usize,
) -> (PipelineTestHarness, Rc<RefCell<TextField>>) {
    let (mut h, tf_ref) = setup_textfield_harness();
    tf_ref.borrow_mut().set_multi_line(true);
    tf_ref.borrow_mut().set_text(text);
    tf_ref.borrow_mut().set_cursor_index(cursor_pos);

    render(&mut h, 800, 600);
    h.click(400.0, 300.0);

    tf_ref.borrow_mut().set_cursor_index(cursor_pos);
    tf_ref.borrow_mut().deselect();

    (h, tf_ref)
}

// ---------------------------------------------------------------------------
// Left / Right (single char)
// ---------------------------------------------------------------------------

#[test]
fn textfield_left_moves_cursor() {
    // "Hello World" with cursor at 5 → Left → cursor at 4
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 5);
    h.press_key(InputKey::ArrowLeft);
    assert_eq!(tf_ref.borrow().cursor_pos(), 4);
    assert!(tf_ref.borrow().is_selection_empty());
}

#[test]
fn textfield_left_at_start_stays() {
    let (mut h, tf_ref) = setup_nav_harness("Hello", 0);
    h.press_key(InputKey::ArrowLeft);
    assert_eq!(tf_ref.borrow().cursor_pos(), 0);
}

#[test]
fn textfield_right_moves_cursor() {
    // "Hello World" with cursor at 5 → Right → cursor at 6
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 5);
    h.press_key(InputKey::ArrowRight);
    assert_eq!(tf_ref.borrow().cursor_pos(), 6);
    assert!(tf_ref.borrow().is_selection_empty());
}

#[test]
fn textfield_right_at_end_stays() {
    let (mut h, tf_ref) = setup_nav_harness("Hello", 5);
    h.press_key(InputKey::ArrowRight);
    assert_eq!(tf_ref.borrow().cursor_pos(), 5);
}

// ---------------------------------------------------------------------------
// Ctrl+Left / Ctrl+Right (word boundary)
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_left_skips_word() {
    // "foo bar baz" cursor at 8 (start of "baz") → Ctrl+Left → 4 (start of "bar")
    // prev_word_index(8): scans i=0, next_word_index(0)=4 (<8), i=4,
    //   next_word_index(4)=8 (>=8), return 4.
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 8);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowLeft);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        4,
        "Ctrl+Left from pos 8 in 'foo bar baz' should go to 4 (start of 'bar')"
    );
    assert!(tf_ref.borrow().is_selection_empty());
}

#[test]
fn textfield_ctrl_left_from_word_start() {
    // "foo bar" cursor at 4 (start of "bar") → Ctrl+Left → 0 (start of "foo")
    // prev_word_index(4): i=0, next_word_index(0)=4, 4>=4 → return 0
    let (mut h, tf_ref) = setup_nav_harness("foo bar", 4);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowLeft);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().cursor_pos(), 0);
}

#[test]
fn textfield_ctrl_right_skips_word() {
    // "foo bar baz" cursor at 0 → Ctrl+Right → 4 (start of "bar")
    // next_word_index(0): 'f' is word char, scans "foo"→3 (delim), continue,
    //   scans " "→4 (!delim) → return 4
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 0);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowRight);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        4,
        "Ctrl+Right from pos 0 in 'foo bar baz' should go to 4 (start of 'bar')"
    );
    assert!(tf_ref.borrow().is_selection_empty());
}

#[test]
fn textfield_ctrl_right_from_middle() {
    // "foo bar baz" cursor at 5 (in "bar") → Ctrl+Right → 8 (start of "baz")
    // next_word_index(5): 'a' word char, scans "ar"→7 (delim ' '), continue
    //   p=7, scans " "→8 (!delim) → return 8
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 5);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowRight);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().cursor_pos(), 8);
}

#[test]
fn textfield_ctrl_right_at_end() {
    let (mut h, tf_ref) = setup_nav_harness("foo bar", 7);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowRight);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().cursor_pos(), 7);
}

// ---------------------------------------------------------------------------
// Home / End
// ---------------------------------------------------------------------------

#[test]
fn textfield_home_moves_to_start() {
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 7);
    h.press_key(InputKey::Home);
    assert_eq!(tf_ref.borrow().cursor_pos(), 0);
    assert!(tf_ref.borrow().is_selection_empty());
}

#[test]
fn textfield_end_moves_to_end() {
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 3);
    h.press_key(InputKey::End);
    assert_eq!(tf_ref.borrow().cursor_pos(), 11);
    assert!(tf_ref.borrow().is_selection_empty());
}

// ---------------------------------------------------------------------------
// Shift+Left / Shift+Right (extend selection one char)
// ---------------------------------------------------------------------------

#[test]
fn textfield_shift_left_extends_selection() {
    // "Hello" cursor at 3 → Shift+Left → cursor 2, selection [2,3)
    let (mut h, tf_ref) = setup_nav_harness("Hello", 3);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::ArrowLeft);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 2);
        assert_eq!(tf.selection_start(), 2);
        assert_eq!(tf.selection_end(), 3);
        assert!(!tf.is_selection_empty());
    }
}

#[test]
fn textfield_shift_right_extends_selection() {
    // "Hello" cursor at 2 → Shift+Right → cursor 3, selection [2,3)
    let (mut h, tf_ref) = setup_nav_harness("Hello", 2);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::ArrowRight);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 3);
        assert_eq!(tf.selection_start(), 2);
        assert_eq!(tf.selection_end(), 3);
    }
}

#[test]
fn textfield_shift_left_twice_extends_two_chars() {
    // "Hello" cursor at 4 → Shift+Left twice → cursor 2, selection [2,4)
    let (mut h, tf_ref) = setup_nav_harness("Hello", 4);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::ArrowLeft);
    h.press_key(InputKey::ArrowLeft);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 2);
        assert_eq!(tf.selection_start(), 2);
        assert_eq!(tf.selection_end(), 4);
    }
}

#[test]
fn textfield_shift_right_twice_extends_two_chars() {
    // "Hello" cursor at 1 → Shift+Right twice → cursor 3, selection [1,3)
    let (mut h, tf_ref) = setup_nav_harness("Hello", 1);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::ArrowRight);
    h.press_key(InputKey::ArrowRight);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 3);
        assert_eq!(tf.selection_start(), 1);
        assert_eq!(tf.selection_end(), 3);
    }
}

// ---------------------------------------------------------------------------
// Shift+Ctrl+Left / Shift+Ctrl+Right (extend selection by word)
// ---------------------------------------------------------------------------

#[test]
fn textfield_shift_ctrl_left_extends_selection_word() {
    // "foo bar baz" cursor at 8 → Shift+Ctrl+Left → cursor 4, selection [4,8)
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 8);
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowLeft);
    h.input_state.release(InputKey::Ctrl);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 4);
        assert_eq!(tf.selection_start(), 4);
        assert_eq!(tf.selection_end(), 8);
    }
}

#[test]
fn textfield_shift_ctrl_right_extends_selection_word() {
    // "foo bar baz" cursor at 0 → Shift+Ctrl+Right → cursor 4, selection [0,4)
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 0);
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowRight);
    h.input_state.release(InputKey::Ctrl);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 4);
        assert_eq!(tf.selection_start(), 0);
        assert_eq!(tf.selection_end(), 4);
    }
}

// ---------------------------------------------------------------------------
// Shift+Home / Shift+End (extend selection to line boundaries)
// ---------------------------------------------------------------------------

#[test]
fn textfield_shift_home_extends_selection_to_start() {
    // "Hello World" cursor at 6 → Shift+Home → cursor 0, selection [0,6)
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 6);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::Home);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 0);
        assert_eq!(tf.selection_start(), 0);
        assert_eq!(tf.selection_end(), 6);
    }
}

#[test]
fn textfield_shift_end_extends_selection_to_end() {
    // "Hello World" cursor at 5 → Shift+End → cursor 11, selection [5,11)
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 5);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::End);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 11);
        assert_eq!(tf.selection_start(), 5);
        assert_eq!(tf.selection_end(), 11);
    }
}

// ---------------------------------------------------------------------------
// Plain arrow clears existing selection (C++ EmptySelection path)
// ---------------------------------------------------------------------------

#[test]
fn textfield_left_clears_selection() {
    // Pre-select [2,5) in "Hello World", then Left without Shift → selection cleared
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 5);
    // Create selection first
    tf_ref.borrow_mut().select(2, 5);
    tf_ref.borrow_mut().set_cursor_index(5);

    h.press_key(InputKey::ArrowLeft);
    {
        let tf = tf_ref.borrow();
        assert!(tf.is_selection_empty(), "Left without Shift should clear selection");
        assert_eq!(tf.cursor_pos(), 4);
    }
}

#[test]
fn textfield_right_clears_selection() {
    let (mut h, tf_ref) = setup_nav_harness("Hello World", 2);
    tf_ref.borrow_mut().select(2, 5);
    tf_ref.borrow_mut().set_cursor_index(2);

    h.press_key(InputKey::ArrowRight);
    {
        let tf = tf_ref.borrow();
        assert!(tf.is_selection_empty(), "Right without Shift should clear selection");
        assert_eq!(tf.cursor_pos(), 3);
    }
}

// ---------------------------------------------------------------------------
// Up / Down in multi-line mode
// ---------------------------------------------------------------------------

#[test]
fn textfield_down_moves_to_next_row() {
    // "abc\ndef\nghi" cursor at 1 (in first row) → Down → should land in second row
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 1);
    h.press_key(InputKey::ArrowDown);
    {
        let tf = tf_ref.borrow();
        // Row 0: "abc\n" (indices 0..4), Row 1: "def\n" (4..8), Row 2: "ghi" (8..11)
        // Down from pos 1 (col 1, row 0) → col 1, row 1 → index 5
        assert_eq!(
            tf.cursor_pos(),
            5,
            "Down from pos 1 in 'abc\\ndef\\nghi' should go to pos 5"
        );
        assert!(tf.is_selection_empty());
    }
}

#[test]
fn textfield_up_moves_to_prev_row() {
    // "abc\ndef\nghi" cursor at 5 (in second row, col 1) → Up → pos 1
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 5);
    h.press_key(InputKey::ArrowUp);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.cursor_pos(),
            1,
            "Up from pos 5 in 'abc\\ndef\\nghi' should go to pos 1"
        );
    }
}

#[test]
fn textfield_up_at_first_row_stays() {
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef", 2);
    h.press_key(InputKey::ArrowUp);
    {
        let tf = tf_ref.borrow();
        // Up from first row: prev_row_index should return 0 or stay at row start
        // Let me check the behavior - it should clamp to the same row
        // prev_row_index when already at row 0 returns col_row_to_index(col, row-1)
        // which for row=0 means row=-1 effectively → should clamp to 0
        assert!(
            tf.cursor_pos() <= 2,
            "Up from first row should not go past start"
        );
    }
}

#[test]
fn textfield_down_at_last_row_stays() {
    // "abc\ndef" cursor at 5 (row 1, col 1) → Down → should stay in last row
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef", 5);
    h.press_key(InputKey::ArrowDown);
    {
        let tf = tf_ref.borrow();
        // Down from last row should not go past end
        assert!(
            tf.cursor_pos() >= 4 && tf.cursor_pos() <= 7,
            "Down from last row should stay in last row, got {}",
            tf.cursor_pos()
        );
    }
}

#[test]
fn textfield_shift_down_extends_selection_multiline() {
    // "abc\ndef\nghi" cursor at 1 → Shift+Down → selection from 1 to 5
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 1);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::ArrowDown);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 5);
        assert_eq!(tf.selection_start(), 1);
        assert_eq!(tf.selection_end(), 5);
    }
}

#[test]
fn textfield_shift_up_extends_selection_multiline() {
    // "abc\ndef\nghi" cursor at 9 (row 2, col 1) → Shift+Up → should extend selection
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 9);
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::ArrowUp);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(tf.cursor_pos(), 5);
        assert_eq!(tf.selection_start(), 5);
        assert_eq!(tf.selection_end(), 9);
    }
}

// ---------------------------------------------------------------------------
// Up / Down ignored in single-line mode (C++: guarded by MultiLineMode)
// ---------------------------------------------------------------------------

#[test]
fn textfield_down_ignored_single_line() {
    let (mut h, tf_ref) = setup_nav_harness("Hello", 2);
    h.press_key(InputKey::ArrowDown);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        2,
        "Down in single-line mode should be ignored"
    );
}

#[test]
fn textfield_up_ignored_single_line() {
    let (mut h, tf_ref) = setup_nav_harness("Hello", 2);
    h.press_key(InputKey::ArrowUp);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        2,
        "Up in single-line mode should be ignored"
    );
}

// ---------------------------------------------------------------------------
// Ctrl+Home / Ctrl+End in multi-line mode
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_home_multiline_goes_to_zero() {
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 9);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Home);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().cursor_pos(), 0);
}

#[test]
fn textfield_ctrl_end_multiline_goes_to_len() {
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 0);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::End);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().cursor_pos(), 11);
}

// ---------------------------------------------------------------------------
// Home / End in multi-line mode → row start / row end
// ---------------------------------------------------------------------------

#[test]
fn textfield_home_multiline_goes_to_row_start() {
    // "abc\ndef\nghi" cursor at 6 (row 1, col 2) → Home → 4 (row 1 start)
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 6);
    h.press_key(InputKey::Home);
    assert_eq!(
        tf_ref.borrow().cursor_pos(),
        4,
        "Home in multi-line should go to row start"
    );
}

#[test]
fn textfield_end_multiline_goes_to_row_end() {
    // "abc\ndef\nghi" cursor at 4 (row 1, col 0) → End → 7 (row 1 end, before \n)
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 4);
    h.press_key(InputKey::End);
    {
        let tf = tf_ref.borrow();
        // row_end for row 1 ("def\n") should be 7 (the position of '\n')
        assert_eq!(
            tf.cursor_pos(),
            7,
            "End in multi-line should go to row end"
        );
    }
}

// ---------------------------------------------------------------------------
// Ctrl+Up / Ctrl+Down (paragraph navigation) in multi-line
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_down_next_paragraph() {
    // "abc\ndef\nghi" cursor at 0 → Ctrl+Down → next paragraph
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 0);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowDown);
    h.input_state.release(InputKey::Ctrl);
    {
        let tf = tf_ref.borrow();
        // next_paragraph_index from 0 should jump past the first \n
        assert!(
            tf.cursor_pos() > 0,
            "Ctrl+Down should move cursor forward"
        );
    }
}

#[test]
fn textfield_ctrl_up_prev_paragraph() {
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 9);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::ArrowUp);
    h.input_state.release(InputKey::Ctrl);
    {
        let tf = tf_ref.borrow();
        assert!(
            tf.cursor_pos() < 9,
            "Ctrl+Up should move cursor backward"
        );
    }
}

// ===========================================================================
// BP-5: TextField editing operations
// ===========================================================================

// ---------------------------------------------------------------------------
// Ctrl+Backspace (delete word before cursor)
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_backspace_deletes_word_before_cursor() {
    // "foo bar baz" cursor at 7 (end of "bar") → Ctrl+Backspace
    // prev_word_index(7) = 4 (start of "bar"), deletes chars 4..7 ("bar") → "foo  baz"
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 7);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Backspace);
    h.input_state.release(InputKey::Ctrl);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "foo  baz",
            "Ctrl+Backspace from pos 7 in 'foo bar baz' should delete 'bar', got '{}'",
            tf.text()
        );
        assert_eq!(
            tf.cursor_pos(),
            4,
            "Cursor should be at 4 after Ctrl+Backspace"
        );
    }
}

#[test]
fn textfield_ctrl_backspace_at_start_does_nothing() {
    let (mut h, tf_ref) = setup_nav_harness("hello", 0);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Backspace);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().text(), "hello");
    assert_eq!(tf_ref.borrow().cursor_pos(), 0);
}

// ---------------------------------------------------------------------------
// Ctrl+Delete (delete word after cursor)
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_delete_deletes_word_after_cursor() {
    // "foo bar baz" cursor at 4 (start of "bar") → Ctrl+Delete → "foo baz"
    // next_word_index(4) should find end of "bar" + skip space (8), deleting "bar " → "foo baz"
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 4);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Delete);
    h.input_state.release(InputKey::Ctrl);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "foo baz",
            "Ctrl+Delete from pos 4 in 'foo bar baz' should delete 'bar ', got '{}'",
            tf.text()
        );
        assert_eq!(
            tf.cursor_pos(),
            4,
            "Cursor should remain at 4 after Ctrl+Delete"
        );
    }
}

#[test]
fn textfield_ctrl_delete_at_end_does_nothing() {
    let (mut h, tf_ref) = setup_nav_harness("hello", 5);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Delete);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(tf_ref.borrow().text(), "hello");
    assert_eq!(tf_ref.borrow().cursor_pos(), 5);
}

// ---------------------------------------------------------------------------
// Shift+Ctrl+Backspace (delete to start of line)
// ---------------------------------------------------------------------------

#[test]
fn textfield_shift_ctrl_backspace_deletes_to_line_start() {
    // "hello world" cursor at 7 → Shift+Ctrl+Backspace → "orld"
    // row_start(7) = 0 (single line), so deletes chars 0..7 → "orld"
    let (mut h, tf_ref) = setup_nav_harness("hello world", 7);
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Backspace);
    h.input_state.release(InputKey::Ctrl);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "orld",
            "Shift+Ctrl+Backspace from pos 7 in 'hello world' should delete to line start, got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 0);
    }
}

#[test]
fn textfield_shift_ctrl_backspace_multiline_deletes_to_row_start() {
    // "abc\ndef\nghi" cursor at 6 (row 1, col 2 = 'f') → Shift+Ctrl+Backspace
    // row_start(6) = 4, deletes chars 4..6 → "abc\nf\nghi"
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 6);
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Backspace);
    h.input_state.release(InputKey::Ctrl);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abc\nf\nghi",
            "Shift+Ctrl+Backspace from col 2 in row 1 should delete 'de', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 4);
    }
}

// ---------------------------------------------------------------------------
// Shift+Ctrl+Delete (delete to end of line)
// ---------------------------------------------------------------------------

#[test]
fn textfield_shift_ctrl_delete_deletes_to_line_end() {
    // "hello world" cursor at 5 → Shift+Ctrl+Delete → "hello"
    // row_end(5) = 11 (single line, end of text), deletes 5..11 → "hello"
    let (mut h, tf_ref) = setup_nav_harness("hello world", 5);
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Delete);
    h.input_state.release(InputKey::Ctrl);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "hello",
            "Shift+Ctrl+Delete from pos 5 in 'hello world' should delete to line end, got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 5);
    }
}

#[test]
fn textfield_shift_ctrl_delete_multiline_deletes_to_row_end() {
    // "abc\ndef\nghi" cursor at 4 (row 1, col 0 = 'd') → Shift+Ctrl+Delete
    // row_end(4) = 7 (before \n), deletes 4..7 → "abc\n\nghi"
    let (mut h, tf_ref) = setup_multiline_nav_harness("abc\ndef\nghi", 4);
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Delete);
    h.input_state.release(InputKey::Ctrl);
    h.input_state.release(InputKey::Shift);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abc\n\nghi",
            "Shift+Ctrl+Delete from col 0 in row 1 should delete 'def', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 4);
    }
}

// ---------------------------------------------------------------------------
// Backspace with selection (deletes selection, C++ DeleteSelectedText path)
// ---------------------------------------------------------------------------

#[test]
fn textfield_backspace_with_selection_deletes_selection() {
    // "abcdef" with selection [2,4) → Backspace → "abef", cursor at 2
    let (mut h, tf_ref) = setup_nav_harness("abcdef", 4);
    tf_ref.borrow_mut().select(2, 4);
    tf_ref.borrow_mut().set_cursor_index(4);

    h.press_key(InputKey::Backspace);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abef",
            "Backspace with selection [2,4) in 'abcdef' should delete 'cd', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 2);
        assert!(tf.is_selection_empty());
    }
}

// ---------------------------------------------------------------------------
// Delete with selection (deletes selection, C++ DeleteSelectedText path)
// ---------------------------------------------------------------------------

#[test]
fn textfield_delete_with_selection_deletes_selection() {
    // "abcdef" with selection [1,3) → Delete → "adef", cursor at 1
    let (mut h, tf_ref) = setup_nav_harness("abcdef", 3);
    tf_ref.borrow_mut().select(1, 3);
    tf_ref.borrow_mut().set_cursor_index(3);

    h.press_key(InputKey::Delete);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "adef",
            "Delete with selection [1,3) in 'abcdef' should delete 'bc', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 1);
        assert!(tf.is_selection_empty());
    }
}

// ---------------------------------------------------------------------------
// Typing with selection replaces selection (C++ ModifySelectedText path)
// ---------------------------------------------------------------------------

#[test]
fn textfield_typing_with_selection_replaces_selection() {
    // "abcdef" with selection [2,5) → type 'X' → "abXf", cursor at 3
    let (mut h, tf_ref) = setup_nav_harness("abcdef", 5);
    tf_ref.borrow_mut().select(2, 5);
    tf_ref.borrow_mut().set_cursor_index(5);

    h.press_char('X');
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abXf",
            "Typing 'X' with selection [2,5) in 'abcdef' should produce 'abXf', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 3);
        assert!(tf.is_selection_empty());
    }
}

// ---------------------------------------------------------------------------
// Insert key toggles overwrite mode (C++ EM_KEY_INSERT + IsNoMod)
// ---------------------------------------------------------------------------

#[test]
fn textfield_insert_toggles_overwrite_mode() {
    let (mut h, tf_ref) = setup_nav_harness("hello", 0);
    assert!(!tf_ref.borrow().is_overwrite_mode());

    h.press_key(InputKey::Insert);
    assert!(
        tf_ref.borrow().is_overwrite_mode(),
        "Insert should toggle overwrite mode ON"
    );

    h.press_key(InputKey::Insert);
    assert!(
        !tf_ref.borrow().is_overwrite_mode(),
        "Insert again should toggle overwrite mode OFF"
    );
}

// ---------------------------------------------------------------------------
// Typing in overwrite mode replaces char at cursor
// (C++ OverwriteMode && CursorIndex < GetRowEndIndex path)
// ---------------------------------------------------------------------------

#[test]
fn textfield_overwrite_mode_replaces_char() {
    // "abcde" with overwrite mode, cursor at 1 → type 'X' → "aXcde", cursor at 2
    let (mut h, tf_ref) = setup_nav_harness("abcde", 1);
    tf_ref.borrow_mut().set_overwrite_mode(true);

    h.press_char('X');
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "aXcde",
            "Overwrite mode: typing 'X' at pos 1 in 'abcde' should replace 'b', got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 2);
    }
}

#[test]
fn textfield_overwrite_mode_at_end_inserts() {
    // "abc" with overwrite mode, cursor at 3 (end) → type 'X' → "abcX"
    // C++: OverwriteMode && CursorIndex < GetRowEndIndex → false at end, so insert
    let (mut h, tf_ref) = setup_nav_harness("abc", 3);
    tf_ref.borrow_mut().set_overwrite_mode(true);

    h.press_char('X');
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "abcX",
            "Overwrite mode at end should insert, got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 4);
    }
}

// ---------------------------------------------------------------------------
// Non-editable TextField rejects all editing operations
// (C++ IsEditable() guard on editing block)
// ---------------------------------------------------------------------------

#[test]
fn textfield_non_editable_rejects_backspace() {
    let (mut h, tf_ref) = setup_nav_harness("hello", 5);
    tf_ref.borrow_mut().set_editable(false);

    h.press_key(InputKey::Backspace);
    assert_eq!(
        tf_ref.borrow().text(),
        "hello",
        "Non-editable TextField should reject Backspace"
    );
}

#[test]
fn textfield_non_editable_rejects_delete() {
    let (mut h, tf_ref) = setup_nav_harness("hello", 2);
    tf_ref.borrow_mut().set_editable(false);

    h.press_key(InputKey::Delete);
    assert_eq!(
        tf_ref.borrow().text(),
        "hello",
        "Non-editable TextField should reject Delete"
    );
}

#[test]
fn textfield_non_editable_rejects_ctrl_backspace() {
    let (mut h, tf_ref) = setup_nav_harness("hello world", 5);
    tf_ref.borrow_mut().set_editable(false);

    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Backspace);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(
        tf_ref.borrow().text(),
        "hello world",
        "Non-editable TextField should reject Ctrl+Backspace"
    );
}

#[test]
fn textfield_non_editable_rejects_ctrl_delete() {
    let (mut h, tf_ref) = setup_nav_harness("hello world", 5);
    tf_ref.borrow_mut().set_editable(false);

    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Delete);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(
        tf_ref.borrow().text(),
        "hello world",
        "Non-editable TextField should reject Ctrl+Delete"
    );
}

#[test]
fn textfield_non_editable_allows_insert_toggle() {
    // C++ Insert key toggle is NOT guarded by IsEditable — it's in the
    // non-editable block. So overwrite mode toggles even when not editable.
    let (mut h, tf_ref) = setup_nav_harness("hello", 0);
    tf_ref.borrow_mut().set_editable(false);

    assert!(!tf_ref.borrow().is_overwrite_mode());
    h.press_key(InputKey::Insert);
    assert!(
        tf_ref.borrow().is_overwrite_mode(),
        "Insert toggle should work even when non-editable (C++ ref: emTextField.cpp:661)"
    );
}

// ---------------------------------------------------------------------------
// Ctrl+Backspace with selection deletes selection (not word)
// C++ ref: emTextField.cpp:741-752 — selection check before word delete
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_backspace_with_selection_deletes_selection() {
    // "foo bar baz" with selection [4,7) → Ctrl+Backspace → "foo baz"
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 7);
    tf_ref.borrow_mut().select(4, 7);
    tf_ref.borrow_mut().set_cursor_index(7);

    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Backspace);
    h.input_state.release(InputKey::Ctrl);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            "foo  baz",
            "Ctrl+Backspace with selection should delete selection, got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 4);
        assert!(tf.is_selection_empty());
    }
}

// ---------------------------------------------------------------------------
// Ctrl+Delete with selection deletes selection (not word)
// C++ ref: emTextField.cpp:757-770 — selection check before word delete
// ---------------------------------------------------------------------------

#[test]
fn textfield_ctrl_delete_with_selection_deletes_selection() {
    // "foo bar baz" with selection [0,3) → Ctrl+Delete → " bar baz"
    let (mut h, tf_ref) = setup_nav_harness("foo bar baz", 3);
    tf_ref.borrow_mut().select(0, 3);
    tf_ref.borrow_mut().set_cursor_index(3);

    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Delete);
    h.input_state.release(InputKey::Ctrl);
    {
        let tf = tf_ref.borrow();
        assert_eq!(
            tf.text(),
            " bar baz",
            "Ctrl+Delete with selection should delete selection, got '{}'",
            tf.text()
        );
        assert_eq!(tf.cursor_pos(), 0);
        assert!(tf.is_selection_empty());
    }
}
