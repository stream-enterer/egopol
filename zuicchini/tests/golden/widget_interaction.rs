use std::cell::Cell;
use std::rc::Rc;

use zuicchini::emCore::rect::Rect;
use zuicchini::emCore::emCursor::emCursor;
use zuicchini::emCore::emInput::{emInputEvent, InputKey, InputVariant};
use zuicchini::emCore::emInputState::emInputState;
use zuicchini::emCore::emLinearGroup::emLinearGroup;
use zuicchini::emCore::emTiling::Orientation;
use zuicchini::emCore::emPanel::{PanelBehavior, PanelState};
use zuicchini::emCore::emPanelCtx::PanelCtx;
use zuicchini::emCore::emPanelTree::PanelTree;
use zuicchini::emCore::emView::{emView, ViewFlags};
use zuicchini::emCore::emPainter::emPainter;
use zuicchini::emCore::emViewRenderer::SoftwareCompositor;
use zuicchini::emCore::emBorder::{emBorder, InnerBorderType, OuterBorderType};

use zuicchini::emCore::emButton::emButton;

use zuicchini::emCore::emCheckBox::emCheckBox;

use zuicchini::emCore::emCheckButton::emCheckButton;

use zuicchini::emCore::emListBox::{emListBox, SelectionMode};

use zuicchini::emCore::emLook::emLook;

use zuicchini::emCore::emRadioButton::{emRadioButton, RadioGroup};

use zuicchini::emCore::emScalarField::emScalarField;

use zuicchini::emCore::emSplitter::emSplitter;

use zuicchini::emCore::emTextField::emTextField;

use super::common::*;

fn default_panel_state() -> PanelState {
    PanelState::default_for_test()
}

fn default_input_state() -> emInputState {
    emInputState::new()
}

/// Skip test if golden data hasn't been generated yet.
macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden/ directory not found — run `make -C golden_gen run` first");
            return;
        }
    };
}

/// Load a widget state golden file as raw bytes.
fn load_widget_state_golden(name: &str) -> Vec<u8> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("data")
        .join("widget_state")
        .join(format!("{name}.widget_state.golden"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()))
}

// ─── Test 1: widget_checkbox_toggle ──────────────────────────────

#[test]
fn widget_checkbox_toggle() {
    require_golden!();
    let golden = load_widget_state_golden("widget_checkbox_toggle");
    assert_eq!(golden.len(), 3, "unexpected golden file size");

    let look = emLook::new();
    let mut cb = emCheckBox::new("Check Option", look);
    let ps = default_panel_state();
    let is = default_input_state();

    // Initial state
    assert_eq!(
        cb.IsChecked() as u8,
        golden[0],
        "initial checked state mismatch"
    );

    // After first activation (Enter is instant — no release needed)
    cb.Input(&emInputEvent::press(InputKey::Enter), &ps, &is);
    assert_eq!(cb.IsChecked() as u8, golden[1], "after 1st click mismatch");

    // After second activation
    cb.Input(&emInputEvent::press(InputKey::Enter), &ps, &is);
    assert_eq!(cb.IsChecked() as u8, golden[2], "after 2nd click mismatch");
}

// ─── Test 1b: widget_checkbutton_toggle ──────────────────────────

#[test]
fn widget_checkbutton_toggle() {
    require_golden!();
    let golden = load_widget_state_golden("widget_checkbutton_toggle");
    assert_eq!(golden.len(), 3, "unexpected golden file size");

    let look = emLook::new();
    let mut cb = emCheckButton::new("Toggle Option", look);
    let ps = default_panel_state();
    let is = default_input_state();

    // Initial state
    assert_eq!(
        cb.IsChecked() as u8,
        golden[0],
        "initial checked state mismatch"
    );

    // After first activation (Enter is instant — no release needed)
    cb.Input(&emInputEvent::press(InputKey::Enter), &ps, &is);
    assert_eq!(cb.IsChecked() as u8, golden[1], "after 1st click mismatch");

    // After second activation
    cb.Input(&emInputEvent::press(InputKey::Enter), &ps, &is);
    assert_eq!(cb.IsChecked() as u8, golden[2], "after 2nd click mismatch");
}

// ─── Test 2: widget_radiobutton_switch ───────────────────────────

#[test]
fn widget_radiobutton_switch() {
    require_golden!();
    let golden = load_widget_state_golden("widget_radiobutton_switch");
    assert_eq!(golden.len(), 8, "unexpected golden file size");

    let look = emLook::new();
    let group = RadioGroup::new();
    let _rb_a = emRadioButton::new("Option A", look.clone(), group.clone(), 0);
    let mut rb_b = emRadioButton::new("Option B", look.clone(), group.clone(), 1);
    let _rb_c = emRadioButton::new("Option C", look, group.clone(), 2);

    // Initial: A checked
    group.borrow_mut().Select(0);
    let initial = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    assert_eq!(
        group.borrow().GetChecked(),
        Some(initial),
        "initial radio check mismatch"
    );

    // Activate B (Enter is instant — no release needed)
    let ps = default_panel_state();
    let is = default_input_state();
    rb_b.Input(&emInputEvent::press(InputKey::Enter), &ps, &is);
    let after = u32::from_le_bytes(golden[4..8].try_into().unwrap()) as usize;
    assert_eq!(
        group.borrow().GetChecked(),
        Some(after),
        "after switch mismatch"
    );
}

// ─── Test 3: widget_listbox_select ───────────────────────────────

#[test]
fn widget_listbox_select() {
    require_golden!();
    let golden = load_widget_state_golden("widget_listbox_select");
    // Golden format: [u32 GetCount][u32 * GetCount indices]. Single GetMode → GetCount=1, 1 index = 8 bytes.
    assert_eq!(golden.len(), 8, "golden file size mismatch (expected count + 1 index = 8 bytes)");

    let look = emLook::new();
    let mut lb = emListBox::new(look);
    lb.SetSelectionType(SelectionMode::Single);
    lb.AddItem("item0".to_string(), "Alpha".to_string());
    lb.AddItem("item1".to_string(), "Beta".to_string());
    lb.AddItem("item2".to_string(), "Gamma".to_string());
    lb.AddItem("item3".to_string(), "Delta".to_string());
    lb.AddItem("item4".to_string(), "Epsilon".to_string());

    // Select 2, then 4 (single GetMode should replace)
    lb.Select(2, true);
    lb.Select(4, true);

    // Parse golden: [u32 GetCount][u32 * GetCount indices]
    let GetCount = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    let mut expected_indices: Vec<usize> = Vec::new();
    for i in 0..GetCount {
        let off = 4 + i * 4;
        expected_indices
            .push(u32::from_le_bytes(golden[off..off + 4].try_into().unwrap()) as usize);
    }

    assert_eq!(
        lb.GetSelectedIndices(),
        &expected_indices,
        "listbox selection mismatch"
    );
}

// ─── Test 4: widget_splitter_setpos ──────────────────────────────

#[test]
fn widget_splitter_setpos() {
    require_golden!();
    let golden = load_widget_state_golden("widget_splitter_setpos");
    assert_eq!(golden.len(), 24, "unexpected golden file size");

    let look = emLook::new();
    let mut sp = emSplitter::new(Orientation::Horizontal, look);
    sp.SetMinMaxPos(0.0, 1.0);

    let eps = 1e-9;

    // Normal GetValue
    sp.SetPos(0.7);
    let expected_1 = f64::from_le_bytes(golden[0..8].try_into().unwrap());
    assert!(
        (sp.GetPos() - expected_1).abs() < eps,
        "pos 0.7: actual={} expected={}",
        sp.GetPos(),
        expected_1
    );

    // Above max — should clamp
    sp.SetPos(1.5);
    let expected_2 = f64::from_le_bytes(golden[8..16].try_into().unwrap());
    assert!(
        (sp.GetPos() - expected_2).abs() < eps,
        "pos 1.5 clamped: actual={} expected={}",
        sp.GetPos(),
        expected_2
    );

    // Below min — should clamp
    sp.SetPos(-0.5);
    let expected_3 = f64::from_le_bytes(golden[16..24].try_into().unwrap());
    assert!(
        (sp.GetPos() - expected_3).abs() < eps,
        "pos -0.5 clamped: actual={} expected={}",
        sp.GetPos(),
        expected_3
    );
}

// ─── Test 5: widget_textfield_type ───────────────────────────────

#[test]
fn widget_textfield_type() {
    require_golden!();
    let golden = load_widget_state_golden("widget_textfield_type");
    assert!(golden.len() >= 8, "golden file too short");

    let look = emLook::new();
    let mut tf = emTextField::new(look);
    tf.SetEditable(true);
    let ps = default_panel_state();
    let is = default_input_state();

    // Type "abc"
    for ch in ['a', 'b', 'c'] {
        let event = emInputEvent::press(InputKey::Key(ch)).with_chars(&ch.to_string());
        tf.Input(&event, &ps, &is);
    }

    // Parse golden: [u32 text_len][text_bytes][u32 cursor_pos]
    let text_len = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    let text = std::str::from_utf8(&golden[4..4 + text_len]).expect("invalid UTF-8 in golden");
    let cursor_off = 4 + text_len;
    let cursor =
        u32::from_le_bytes(golden[cursor_off..cursor_off + 4].try_into().unwrap()) as usize;

    assert_eq!(tf.text(), text, "text mismatch");
    assert_eq!(tf.GetCursorIndex(), cursor, "cursor mismatch");
}

// ─── Test 6: widget_textfield_backspace ──────────────────────────

#[test]
fn widget_textfield_backspace() {
    require_golden!();
    let golden = load_widget_state_golden("widget_textfield_backspace");
    assert!(golden.len() >= 8, "golden file too short");

    let look = emLook::new();
    let mut tf = emTextField::new(look);
    tf.SetEditable(true);
    let ps = default_panel_state();
    let is = default_input_state();

    // Type "abc"
    for ch in ['a', 'b', 'c'] {
        let event = emInputEvent::press(InputKey::Key(ch)).with_chars(&ch.to_string());
        tf.Input(&event, &ps, &is);
    }

    // Backspace
    tf.Input(&emInputEvent::press(InputKey::Backspace), &ps, &is);

    // Parse golden: [u32 text_len][text_bytes][u32 cursor_pos]
    let text_len = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    let text = std::str::from_utf8(&golden[4..4 + text_len]).expect("invalid UTF-8 in golden");
    let cursor_off = 4 + text_len;
    let cursor =
        u32::from_le_bytes(golden[cursor_off..cursor_off + 4].try_into().unwrap()) as usize;

    assert_eq!(tf.text(), text, "text mismatch");
    assert_eq!(tf.GetCursorIndex(), cursor, "cursor mismatch");
}

// ─── Test 7: widget_textfield_select ────────────────────────────

#[test]
fn widget_textfield_select() {
    require_golden!();
    let golden = load_widget_state_golden("widget_textfield_select");
    assert_eq!(golden.len(), 12, "unexpected golden file size");

    let look = emLook::new();
    let mut tf = emTextField::new(look);
    tf.SetEditable(true);
    let ps = default_panel_state();
    let is = default_input_state();

    // Type "abcdef"
    for ch in ['a', 'b', 'c', 'd', 'e', 'f'] {
        let event = emInputEvent::press(InputKey::Key(ch)).with_chars(&ch.to_string());
        tf.Input(&event, &ps, &is);
    }

    // Shift+ArrowLeft × 3 to select last 3 chars
    for _ in 0..3 {
        tf.Input(&emInputEvent::press(InputKey::ArrowLeft).with_shift(), &ps, &is);
    }

    // Parse golden: [u32 sel_start][u32 sel_end][u32 cursor]
    let sel_start = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    let sel_end = u32::from_le_bytes(golden[4..8].try_into().unwrap()) as usize;
    let cursor = u32::from_le_bytes(golden[8..12].try_into().unwrap()) as usize;

    assert_eq!(tf.GetSelectionStartIndex(), sel_start, "sel_start mismatch");
    assert_eq!(tf.GetSelectionEndIndex(), sel_end, "sel_end mismatch");
    assert_eq!(tf.GetCursorIndex(), cursor, "cursor mismatch");
}

// ─── Test 8: widget_scalarfield_inc ─────────────────────────────

#[test]
fn widget_scalarfield_inc() {
    require_golden!();
    let golden = load_widget_state_golden("widget_scalarfield_inc");
    assert_eq!(golden.len(), 16, "unexpected golden file size");

    let look = emLook::new();
    let mut sf = emScalarField::new(0.0, 100.0, look);
    sf.SetValue(50.0);
    let ps = default_panel_state();
    let is = default_input_state();

    let eps = 1e-9;

    // Press "+" to increment
    sf.Input(&emInputEvent::press(InputKey::Key('+')), &ps, &is);
    let expected_inc = f64::from_le_bytes(golden[0..8].try_into().unwrap());
    assert!(
        (sf.GetValue() - expected_inc).abs() < eps,
        "after +: actual={} expected={}",
        sf.GetValue(),
        expected_inc
    );

    // Press "-" to decrement
    sf.Input(&emInputEvent::press(InputKey::Key('-')), &ps, &is);
    let expected_dec = f64::from_le_bytes(golden[8..16].try_into().unwrap());
    assert!(
        (sf.GetValue() - expected_dec).abs() < eps,
        "after -: actual={} expected={}",
        sf.GetValue(),
        expected_dec
    );
}

// ─── Test 9: widget_button_click ────────────────────────────────

#[test]
fn widget_button_click() {
    require_golden!();
    let golden = load_widget_state_golden("widget_button_click");
    assert_eq!(golden.len(), 3, "unexpected golden file size");

    let look = emLook::new();
    let mut btn = emButton::new("Click Me", look);

    // Track on_click callback invocations to verify side effects.
    let click_count = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let cc = click_count.clone();
    btn.on_click = Some(Box::new(move || {
        cc.Set(cc.GetRec() + 1);
    }));

    // Initial state: not pressed, callback not fired
    assert_eq!(
        btn.IsPressed() as u8,
        golden[0],
        "initial pressed state mismatch"
    );
    assert_eq!(click_count.GetRec(), 0, "on_click should not fire before any click");

    // After programmatic Click(): pressed state unchanged (Click is instantaneous)
    btn.Click();
    assert_eq!(
        btn.IsPressed() as u8,
        golden[1],
        "after 1st click pressed mismatch"
    );
    assert_eq!(click_count.GetRec(), 1, "on_click should fire exactly once after 1st click");

    // After second Click
    btn.Click();
    assert_eq!(
        btn.IsPressed() as u8,
        golden[2],
        "after 2nd click pressed mismatch"
    );
    assert_eq!(click_count.GetRec(), 2, "on_click should fire exactly twice after 2nd click");
}

// ─── Test 10: widget_listbox_multi ──────────────────────────────

#[test]
fn widget_listbox_multi() {
    require_golden!();
    let golden = load_widget_state_golden("widget_listbox_multi");
    // Golden format: [u32 GetCount][u32 * GetCount indices]. Multi select 2 items → GetCount=2, 2 indices = 12 bytes.
    assert_eq!(golden.len(), 12, "golden file size mismatch (expected count + 2 indices = 12 bytes)");

    let look = emLook::new();
    let mut lb = emListBox::new(look);
    lb.SetSelectionType(SelectionMode::Multi);
    lb.AddItem("item0".to_string(), "Alpha".to_string());
    lb.AddItem("item1".to_string(), "Beta".to_string());
    lb.AddItem("item2".to_string(), "Gamma".to_string());
    lb.AddItem("item3".to_string(), "Delta".to_string());
    lb.AddItem("item4".to_string(), "Epsilon".to_string());

    // Select items 1 and 3 additively
    lb.Select(1, false);
    lb.Select(3, false);

    // Parse golden: [u32 GetCount][u32*GetCount indices]
    let GetCount = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    let mut expected_indices: Vec<usize> = Vec::new();
    for i in 0..GetCount {
        let off = 4 + i * 4;
        expected_indices
            .push(u32::from_le_bytes(golden[off..off + 4].try_into().unwrap()) as usize);
    }

    assert_eq!(
        lb.GetSelectedIndices(),
        &expected_indices,
        "listbox multi-selection mismatch"
    );
}

// ─── Test 11: widget_listbox_toggle ─────────────────────────────

#[test]
fn widget_listbox_toggle() {
    require_golden!();
    let golden = load_widget_state_golden("widget_listbox_toggle");
    // Golden format: two snapshots. Snap 1: [GetCount=1][1 index] = 8 bytes. Snap 2: [GetCount=0] = 4 bytes. Total = 12.
    assert_eq!(golden.len(), 12, "golden file size mismatch (expected 2 snapshots = 12 bytes)");

    let look = emLook::new();
    let mut lb = emListBox::new(look);
    lb.SetSelectionType(SelectionMode::Toggle);
    lb.AddItem("item0".to_string(), "Alpha".to_string());
    lb.AddItem("item1".to_string(), "Beta".to_string());
    lb.AddItem("item2".to_string(), "Gamma".to_string());
    lb.AddItem("item3".to_string(), "Delta".to_string());
    lb.AddItem("item4".to_string(), "Epsilon".to_string());

    // Toggle item 2 on — first snapshot
    lb.ToggleSelection(2);

    let count1 = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    let mut expected1: Vec<usize> = Vec::new();
    let mut off = 4;
    for _ in 0..count1 {
        expected1.push(u32::from_le_bytes(golden[off..off + 4].try_into().unwrap()) as usize);
        off += 4;
    }
    assert_eq!(
        lb.GetSelectedIndices(),
        &expected1,
        "after toggle-on mismatch"
    );

    // Toggle item 2 off — second snapshot
    lb.ToggleSelection(2);

    let count2 = u32::from_le_bytes(golden[off..off + 4].try_into().unwrap()) as usize;
    off += 4;
    let mut expected2: Vec<usize> = Vec::new();
    for _ in 0..count2 {
        expected2.push(u32::from_le_bytes(golden[off..off + 4].try_into().unwrap()) as usize);
        off += 4;
    }
    assert_eq!(
        lb.GetSelectedIndices(),
        &expected2,
        "after toggle-off mismatch"
    );
}

// ─── Test 12: widget_textfield_cursor_nav ───────────────────────

#[test]
fn widget_textfield_cursor_nav() {
    require_golden!();
    let golden = load_widget_state_golden("widget_textfield_cursor_nav");
    assert_eq!(golden.len(), 8, "unexpected golden file size");

    let look = emLook::new();
    let mut tf = emTextField::new(look);
    tf.SetEditable(true);
    tf.SetMultiLineMode(true);
    tf.SetText("abc\ndef");
    tf.SetCursorIndex(7); // End of "abc\ndef"
    let ps = default_panel_state();
    let is = default_input_state();

    let cursor_before = u32::from_le_bytes(golden[0..4].try_into().unwrap()) as usize;
    assert_eq!(
        tf.GetCursorIndex(),
        cursor_before,
        "cursor before ArrowUp mismatch"
    );

    // ArrowUp
    tf.Input(&emInputEvent::press(InputKey::ArrowUp), &ps, &is);

    let cursor_after = u32::from_le_bytes(golden[4..8].try_into().unwrap()) as usize;
    assert_eq!(
        tf.GetCursorIndex(),
        cursor_after,
        "cursor after ArrowUp mismatch"
    );
}

// ─── Test 13: widget_splitter_drag ──────────────────────────────

#[test]
fn widget_splitter_drag() {
    require_golden!();
    let golden = load_widget_state_golden("widget_splitter_drag");
    assert_eq!(golden.len(), 16, "unexpected golden file size");

    let look = emLook::new();
    let mut sp = emSplitter::new(Orientation::Horizontal, look);
    sp.SetMinMaxPos(0.0, 1.0);
    sp.SetPos(0.5);

    let eps = 1e-9;

    let expected_before = f64::from_le_bytes(golden[0..8].try_into().unwrap());
    assert!(
        (sp.GetPos() - expected_before).abs() < eps,
        "pos before: actual={} expected={}",
        sp.GetPos(),
        expected_before
    );

    // Set GetPos to 0.7 (matching C++ SetPos(0.7))
    sp.SetPos(0.7);
    let expected_after = f64::from_le_bytes(golden[8..16].try_into().unwrap());
    assert!(
        (sp.GetPos() - expected_after).abs() < eps,
        "pos after: actual={} expected={}",
        sp.GetPos(),
        expected_after
    );
}

// ─── Test 14: splitter_layout_h ─────────────────────────────────

/// Wraps a emSplitter as PanelBehavior for layout testing.
struct SplitterLayoutBehavior {
    splitter: emSplitter,
}

impl PanelBehavior for SplitterLayoutBehavior {
    fn PaintContent(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        self.splitter.PaintContent(painter, w, h, _state.enabled);
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        let rect = ctx.layout_rect();
        self.splitter.LayoutChildren(ctx, rect.w, rect.h);
    }
}

/// Parse splitter layout golden: [u32 steps][steps * 9 f64s]
/// Each step: (pos, c0_x, c0_y, c0_w, c0_h, c1_x, c1_y, c1_w, c1_h)
fn parse_splitter_layout_golden(data: &[u8]) -> Vec<[f64; 9]> {
    let steps = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    assert_eq!(data.len(), 4 + steps * 72, "golden size mismatch");
    let mut GetResult = Vec::with_capacity(steps);
    for s in 0..steps {
        let base = 4 + s * 72;
        let mut vals = [0.0f64; 9];
        for i in 0..9 {
            let off = base + i * 8;
            vals[i] = f64::from_le_bytes(data[off..off + 8].try_into().unwrap());
        }
        GetResult.push(vals);
    }
    GetResult
}

/// Run splitter layout for a single GetPos, return [pos, c0_x, c0_y, c0_w, c0_h, c1_x, c1_y, c1_w, c1_h].
fn run_splitter_layout_step(
    orientation: Orientation,
    parent_rect: (f64, f64, f64, f64),
    pos: f64,
) -> [f64; 9] {
    let look = emLook::new();
    let mut sp = emSplitter::new(orientation, look);
    sp.SetMinMaxPos(0.0, 1.0);
    sp.SetPos(pos);
    let clamped_pos = sp.GetPos();

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.Layout(
        root,
        parent_rect.0,
        parent_rect.1,
        parent_rect.2,
        parent_rect.3,
    );
    let c0 = tree.create_child(root, "left");
    let c1 = tree.create_child(root, "right");

    tree.set_behavior(root, Box::new(SplitterLayoutBehavior { splitter: sp }));
    let mut behavior = tree.take_behavior(root).unwrap();
    {
        let mut ctx = PanelCtx::new(&mut tree, root);
        behavior.LayoutChildren(&mut ctx);
    }
    tree.put_behavior(root, behavior);

    let r0 = tree
        .layout_rect(c0)
        .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
    let r1 = tree
        .layout_rect(c1)
        .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));

    [clamped_pos, r0.x, r0.y, r0.w, r0.h, r1.x, r1.y, r1.w, r1.h]
}

#[test]
fn splitter_layout_h() {
    require_golden!();
    let golden = load_widget_state_golden("splitter_layout_h");
    let expected = parse_splitter_layout_golden(&golden);
    assert_eq!(expected.len(), 4);

    // C++ uses layout (0,0,1.0,0.75), positions: 0.5, 0.3, 0.8, 1.5 (clamped to 1.0)
    let positions = [0.5, 0.3, 0.8, 1.5];
    let GetParentContext = (0.0, 0.0, 1.0, 0.75);

    let eps = 1e-9;
    for (i, &pos) in positions.iter().enumerate() {
        let actual = run_splitter_layout_step(Orientation::Horizontal, GetParentContext, pos);
        for j in 0..9 {
            assert!(
                (actual[j] - expected[i][j]).abs() < eps,
                "step {i} field {j}: actual={:.6} expected={:.6}",
                actual[j],
                expected[i][j]
            );
        }
    }
}

#[test]
fn splitter_layout_v() {
    require_golden!();
    let golden = load_widget_state_golden("splitter_layout_v");
    let expected = parse_splitter_layout_golden(&golden);
    assert_eq!(expected.len(), 4);

    // C++ uses layout (0,0,1.0,1.0), positions: 0.5, 0.2, 0.7, 0.0 (at min)
    let positions = [0.5, 0.2, 0.7, 0.0];
    let GetParentContext = (0.0, 0.0, 1.0, 1.0);

    let eps = 1e-9;
    for (i, &pos) in positions.iter().enumerate() {
        let actual = run_splitter_layout_step(Orientation::Vertical, GetParentContext, pos);
        for j in 0..9 {
            assert!(
                (actual[j] - expected[i][j]).abs() < eps,
                "step {i} field {j}: actual={:.6} expected={:.6}",
                actual[j],
                expected[i][j]
            );
        }
    }
}

// ─── Test: composition_click_through_tree ────────────────────────

/// emButton wrapper that delegates Input handling (needed for mouse Click dispatch).
struct ClickableButtonPanel {
    widget: emButton,
}

impl PanelBehavior for ClickableButtonPanel {
    fn PaintContent(&mut self, p: &mut emPainter, w: f64, h: f64, s: &PanelState) {
        self.widget.PaintContent(p, w, h, s.enabled);
    }
    fn Input(&mut self, e: &emInputEvent, s: &PanelState, is: &emInputState) -> bool {
        self.widget.Input(e, s, is)
    }
    fn GetCursor(&self) -> emCursor {
        self.widget.GetCursor()
    }
    fn IsOpaque(&self) -> bool {
        true
    }
}

/// Dispatch a single Input event through the panel tree, replicating the
/// Input delivery logic from ZuiWindow::dispatch_input without needing a
/// window. Iterates viewed panels in post-order (children before parents),
/// transforms mouse coordinates to panel-local space, and stops on first
/// consumption.
fn dispatch_event(
    tree: &mut PanelTree,
    view: &mut emView,
    event: &emInputEvent,
    input_state: &emInputState,
) {
    // For mouse press: set active panel via hit test
    if event.variant == InputVariant::Press
        && Match!(
            event.key,
            InputKey::MouseLeft | InputKey::MouseRight | InputKey::MouseMiddle
        )
    {
        let panel = view
            .GetFocusablePanelAt(tree, event.mouse_x, event.mouse_y)
            .unwrap_or_else(|| view.GetRootPanel());
        view.set_active_panel(tree, panel, false);
    }

    let wf = view.IsFocused();
    let viewed = tree.viewed_panels_dfs();
    for panel_id in viewed {
        let mut panel_ev = event.clone();
        panel_ev.mouse_x = tree.ViewToPanelX(panel_id, event.mouse_x);
        panel_ev.mouse_y = tree.ViewToPanelY(panel_id, event.mouse_y, view.GetCurrentPixelTallness());

        if let Some(mut behavior) = tree.take_behavior(panel_id) {
            let panel_state = tree.build_panel_state(panel_id, wf, view.GetCurrentPixelTallness());
            // Suppress keyboard events for panels not in the active path
            if panel_ev.is_keyboard_event() && !panel_state.in_active_path {
                tree.put_behavior(panel_id, behavior);
                continue;
            }
            let consumed = behavior.Input(&panel_ev, &panel_state, input_state);
            tree.put_behavior(panel_id, behavior);
            if consumed {
                view.InvalidatePainting(tree, panel_id);
                break;
            }
        }
    }
}

/// Build a panel tree with nested borders and a button, simulate a mouse
/// Click on the button, and verify the Click reaches the button (the
/// on_click callback fires).
///
/// Hierarchy:
///   Root: emLinearGroup vertical (OBT_Rect, caption "Root")
///     Child: emLinearGroup vertical (OBT_Rect, caption "Container")
///       Grandchild: emButton ("Click Me")
#[test]
fn composition_click_through_tree() {
    let click_count = Rc::new(Cell::new(0u32));
    let clicked_clone = click_count.clone();

    let look = emLook::new();

    let mut tree = PanelTree::new();
    let root = tree.create_root("root");

    // Root: vertical emLinearGroup with OBT_Rect border
    let mut root_group = emLinearGroup::vertical();
    root_group.border = emBorder::new(OuterBorderType::Rect)
        .with_inner(InnerBorderType::None)
        .with_caption("Root");
    root_group.border.label_in_border = true;
    tree.Layout(root, 0.0, 0.0, 800.0 / 600.0, 1.0);

    // Container: vertical emLinearGroup with OBT_Rect border
    let container_id = tree.create_child(root, "container");
    let mut container_group = emLinearGroup::vertical();
    container_group.border = emBorder::new(OuterBorderType::Rect)
        .with_inner(InnerBorderType::None)
        .with_caption("Container");
    container_group.border.label_in_border = true;
    tree.set_behavior(container_id, Box::new(container_group));

    // emButton with on_click callback
    let button_id = tree.create_child(container_id, "button");
    let mut btn = emButton::new("Click Me", look);
    btn.on_click = Some(Box::new(move || {
        clicked_clone.Set(clicked_clone.GetRec() + 1);
    }));
    tree.set_behavior(button_id, Box::new(ClickableButtonPanel { widget: btn }));

    // Set root behavior last (after children are created)
    tree.set_behavior(root, Box::new(root_group));

    // Set up view and settle layout
    let mut view = emView::new(root, 800.0, 600.0);
    view.flags.insert(ViewFlags::NO_ACTIVE_HIGHLIGHT);
    for _ in 0..200 {
        tree.HandleNotice(view.IsFocused(), view.GetCurrentPixelTallness());
        view.Update(&mut tree);
    }

    // Render once so the button caches its PaintContent dimensions (last_w, last_h)
    // which are needed for mouse hit-testing.
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut tree, &view);

    // Click at the center of the viewport. The button should be laid out
    // within the nested borders and the center should fall inside it.
    let click_x = 400.0;
    let click_y = 300.0;
    let input_state = emInputState::new();

    // Mouse press
    let press = emInputEvent::press(InputKey::MouseLeft).with_mouse(click_x, click_y);
    dispatch_event(&mut tree, &mut view, &press, &input_state);

    // Mouse release at the same GetPos
    let release = emInputEvent::release(InputKey::MouseLeft).with_mouse(click_x, click_y);
    dispatch_event(&mut tree, &mut view, &release, &input_state);

    assert_eq!(
        click_count.GetRec(),
        1,
        "Button on_click callback should fire exactly once — click did not reach the button through the nested border tree"
    );
}
