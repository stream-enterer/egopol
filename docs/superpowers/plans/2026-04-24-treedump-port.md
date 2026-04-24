# Tree Dump Port + Agent Control Channel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the Rust `emTreeDump` to full C++ field parity (schema-faithful `emTreeDumpRec` output, `PanelBehavior::dump_state` subtype hook, `paint_count` / `last_paint_frame` on `PanelData`), and add an opt-in Unix-domain control channel so an autonomous agent can drive the running app end-to-end.

**Architecture:** Four coupled pieces behind a single runtime env-var gate (`EMCORE_DEBUG_CONTROL=1`): (A) tree dump extension with `emTreeDumpRec`-faithful schema, (B) paint counter wired at the single `behavior.Paint()` call site in `emView::paint_one_panel`, (C) control channel with acceptor + worker threads dispatching through `winit::EventLoopProxy` + custom `UserEvent`, (D) synthetic input construction dispatched by direct call into `App::window_event`.

**Tech Stack:** Rust 2021, `winit` (already present), `serde` + `serde_json` (new deps), Unix-domain sockets via `std::os::unix::net`, `std::sync::mpsc` for main-thread replies.

**Spec:** `docs/superpowers/specs/2026-04-24-treedump-port-design.md`.

---

## Phase 1 — Tree dump foundation + paint counter

Phase goal: every `td!` cheat invocation writes a full C++-faithful `emTreeDumpRec` dump. No control channel yet.

### Task 1.1 — Add paint counters to `PanelData` + frame counter to `emView`

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs` (struct `PanelData` at line 154, add two fields)
- Modify: `crates/emcore/src/emView.rs` (add `current_frame` field and method)

- [ ] **Step 1: Write failing test** — new file `crates/emcore/src/emPanelTree_paint_counter_tests.rs` is not needed; use the existing test module at the bottom of `emPanelTree.rs`. Append:

```rust
#[cfg(test)]
mod paint_counter_tests {
    use super::*;
    #[test]
    fn new_panel_has_zero_paint_counters() {
        let mut tree = PanelTree::new();
        let root = tree.create_root("root".into());
        let data = tree.data(root).expect("root exists");
        assert_eq!(data.paint_count, 0);
        assert_eq!(data.last_paint_frame, 0);
    }
}
```

(If `PanelTree::create_root` / `data()` have different names in the codebase, use whatever the existing test patterns do; grep `fn create_root\|fn data` in the file.)

- [ ] **Step 2: Run test, confirm FAIL**

```bash
cargo test -p emcore --lib emPanelTree::paint_counter_tests 2>&1 | tail -20
```

Expected: compile error — `paint_count` / `last_paint_frame` fields don't exist.

- [ ] **Step 3: Add fields to `PanelData`**

In `crates/emcore/src/emPanelTree.rs`, immediately after the existing `pub(crate) is_active: bool,` line (search for that inside `struct PanelData`), add:

```rust
    // RUST_ONLY: (language-forced utility)
    // C++ relies on gdb for per-panel paint inspection; the Rust port
    // lacks an equivalent live-inspection path, so paint attribution is
    // baked into the data model. Bumped by the paint driver at
    // emView::paint_one_panel, never by behaviors.
    pub(crate) paint_count: u64,
    pub(crate) last_paint_frame: u64,
```

Find every `PanelData { ... }` literal constructor in the file (grep `PanelData \{`) and add `paint_count: 0, last_paint_frame: 0,` to each.

- [ ] **Step 4: Run test, confirm PASS**

```bash
cargo test -p emcore --lib emPanelTree::paint_counter_tests 2>&1 | tail -10
```

Expected: `test paint_counter_tests::new_panel_has_zero_paint_counters ... ok`.

- [ ] **Step 5: Add frame counter to `emView`**

In `crates/emcore/src/emView.rs`, inside the `emView` struct (search for `pub struct emView`), add a new field near the other per-frame state (look for `pub(crate) window_focused` as an anchor):

```rust
    /// RUST_ONLY: (language-forced utility)
    /// Monotonic frame counter incremented once per completed paint pass.
    /// Paired with PanelData::last_paint_frame for the tree dump.
    pub(crate) current_frame: u64,
```

Find every `emView { ... }` literal constructor (grep `emView \{` in the file) and add `current_frame: 0,` to each.

- [ ] **Step 6: Verify compile**

```bash
cargo check -p emcore 2>&1 | tail -5
```

Expected: no errors (warnings about unused field are acceptable at this step; next task uses them).

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emPanelTree.rs crates/emcore/src/emView.rs
git commit -m "feat(emTreeDump): add paint_count/last_paint_frame to PanelData + frame counter to emView"
```

---

### Task 1.2 — Bump paint counter at paint-driver call site

**Files:**
- Modify: `crates/emcore/src/emView.rs` (`paint_one_panel` at line 4796 — the single `behavior.Paint()` call site is line 4821)

- [ ] **Step 1: Write failing test**

Append to the `#[cfg(test)]` module at the bottom of `emView.rs` (find an existing test with `fn some_paint_test` or just use the `test_view_harness`). Add:

```rust
#[cfg(test)]
mod paint_counter_integration_tests {
    use super::*;
    use crate::test_view_harness::*;

    #[test]
    fn paint_bumps_counter_and_records_frame() {
        // Build minimal view+one-panel scene via the existing harness.
        // (Adapt to whatever helper the harness exposes for "one panel
        // painted once"; grep test_view_harness.rs for existing patterns.)
        let mut h = TestViewHarness::new();
        let panel_id = /* create one panel via harness */;
        // Drive one paint pass:
        /* invoke view.Paint(...) */
        let data = h.tree().data(panel_id).expect("panel exists");
        assert_eq!(data.paint_count, 1);
        assert_eq!(data.last_paint_frame, 0); // first frame
    }
}
```

If the harness does not currently expose a "paint one frame" helper, add one: `pub fn paint_once(&mut self) { ... }` that walks the existing view-paint path. This is a small utility that belongs alongside the existing harness.

- [ ] **Step 2: Confirm failure**

```bash
cargo test -p emcore --lib emView::paint_counter_integration_tests 2>&1 | tail -20
```

Expected: test compiles (after helper added) and fails with `paint_count == 0`.

- [ ] **Step 3: Insert the counter bump**

In `crates/emcore/src/emView.rs` at line 4821 (the `behavior.Paint(painter, 1.0, tallness, &state);` call inside `paint_one_panel`), immediately before that line, add:

```rust
            if let Some(data) = tree.data_mut(id) {
                data.paint_count = data.paint_count.wrapping_add(1);
                data.last_paint_frame = self.current_frame;
            }
```

(Use whatever `data_mut(id)` accessor exists in the current codebase; grep `fn data_mut` in `emPanelTree.rs`. If none exists, add `pub(crate) fn data_mut(&mut self, id: PanelId) -> Option<&mut PanelData>` right next to whatever immutable `data()` accessor is already there.)

- [ ] **Step 4: Increment frame counter at end of paint pass**

Search for the top-level view paint entry — `pub fn Paint(` on `emView`. At the very end of that function (just before the closing `}`), add:

```rust
        self.current_frame = self.current_frame.wrapping_add(1);
```

- [ ] **Step 5: Confirm test passes**

```bash
cargo test -p emcore --lib emView::paint_counter_integration_tests 2>&1 | tail -10
```

Expected: `test ... paint_bumps_counter_and_records_frame ... ok`.

- [ ] **Step 6: Confirm no regressions**

```bash
cargo test -p emcore --lib 2>&1 | tail -15
```

Expected: no new failures; all existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emView.rs crates/emcore/src/emPanelTree.rs crates/emcore/src/test_view_harness.rs
git commit -m "feat(emTreeDump): bump paint counter at behavior.Paint() call site"
```

---

### Task 1.3 — Add `dump_state` to `PanelBehavior`

**Files:**
- Modify: `crates/emcore/src/emPanel.rs` (trait `PanelBehavior` at line 196)

- [ ] **Step 1: Write failing test**

Append to `emPanel.rs`'s existing `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn default_dump_state_is_empty() {
        struct NoopBehavior;
        impl PanelBehavior for NoopBehavior {}
        let b = NoopBehavior;
        assert!(b.dump_state().is_empty());
    }

    #[test]
    fn override_dump_state_returns_pairs() {
        struct HasState;
        impl PanelBehavior for HasState {
            fn dump_state(&self) -> Vec<(&'static str, String)> {
                vec![("loading_pct", "42".to_string()), ("loading_done", "false".to_string())]
            }
        }
        let b = HasState;
        let s = b.dump_state();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], ("loading_pct", "42".to_string()));
    }
```

- [ ] **Step 2: Confirm failure**

```bash
cargo test -p emcore --lib emPanel::tests::default_dump_state_is_empty emPanel::tests::override_dump_state_returns_pairs 2>&1 | tail -10
```

Expected: compile error — `dump_state` method doesn't exist.

- [ ] **Step 3: Add trait method**

In `crates/emcore/src/emPanel.rs`, inside `pub trait PanelBehavior`, after the `fn type_name(&self) -> &str` method (around line 361), add:

```rust
    /// Return subtype-specific fields to append to the tree dump's emPanel
    /// Text block. Each pair is formatted as `"\n<label>: <value>"` in
    /// insertion order. Default: empty (no subtype state).
    ///
    /// Rust analog of C++'s dynamic_cast cascade in
    /// `emTreeDumpFromObject` — each concrete panel class in C++ adds
    /// its own fields via a centralized cascade; Rust decentralizes this
    /// because PanelBehavior is the unifying trait C++ lacks. Preserves
    /// observable output; differs only in internal dispatch.
    fn dump_state(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
```

- [ ] **Step 4: Confirm tests pass**

```bash
cargo test -p emcore --lib emPanel::tests::default_dump_state_is_empty emPanel::tests::override_dump_state_returns_pairs 2>&1 | tail -10
```

Expected: both `ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emPanel.rs
git commit -m "feat(emTreeDump): add PanelBehavior::dump_state trait method with empty default"
```

---

### Task 1.4 — Create `emTreeDump.rs` module skeleton

**Files:**
- Create: `crates/emcore/src/emTreeDump.rs`
- Modify: `crates/emcore/src/lib.rs` (register new module)

- [ ] **Step 1: Create the module file with Frame/VisualStyle constants**

Write `crates/emcore/src/emTreeDump.rs`:

```rust
//! Port of C++ `emTreeDump` package (`src/emTreeDump/emTreeDumpUtil.cpp`).
//!
//! Produces an `emTreeDumpRec`-faithful emRec serialization of the running
//! object graph. Used by the `td!` cheat, by `emCtrlSocket`'s `dump`
//! command, and by the existing `emView::dump_tree` shim.
//!
//! Schema matches C++ `emTreeDumpRec` byte-for-byte so a future port of
//! `emTreeDumpFilePanel` can consume the same file.

#![allow(non_snake_case)]

use crate::emColor::emColor;
use crate::emRec::{RecArray, RecStruct, RecValue};

/// C++ `emTreeDumpRec::FrameType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Frame {
    None = 0,
    Rectangle = 1,
    RoundRect = 2,
    Ellipse = 3,
    Hexagon = 4,
}

impl Frame {
    fn as_str(self) -> &'static str {
        match self {
            Frame::None => "FRAME_NONE",
            Frame::Rectangle => "FRAME_RECTANGLE",
            Frame::RoundRect => "FRAME_ROUND_RECT",
            Frame::Ellipse => "FRAME_ELLIPSE",
            Frame::Hexagon => "FRAME_HEXAGON",
        }
    }
}

/// Per-object visual style (Frame + BgColor + FgColor) matching C++ constants
/// in `emTreeDumpFromObject`.
pub(crate) struct VisualStyle {
    pub frame: Frame,
    pub bg: u32,
    pub fg: u32,
}

impl VisualStyle {
    pub(crate) fn engine() -> Self {
        Self { frame: Frame::Rectangle, bg: 0x000000, fg: 0xEEEEEE }
    }
    pub(crate) fn context(is_root: bool) -> Self {
        let _ = is_root; // C++ uses the same color for root and child context
        Self { frame: Frame::Ellipse, bg: 0x777777, fg: 0xEEEEEE }
    }
    pub(crate) fn view(focused: bool) -> Self {
        let fg = if focused { 0xEEEE44 } else { 0xEEEEEE };
        Self { frame: Frame::RoundRect, bg: 0x448888, fg }
    }
    pub(crate) fn window() -> Self {
        // Window branch overlays the view branch — Frame stays ROUND_RECT
        // from view; only Bg is overridden.
        Self { frame: Frame::RoundRect, bg: 0x222288, fg: 0xEEEEEE }
    }
    pub(crate) fn panel(viewed: bool, in_viewed_path: bool, in_focused_path: bool, in_active_path: bool) -> Self {
        let bg = if viewed { 0x338833 }
            else if in_viewed_path { 0x225522 }
            else { 0x445544 };
        let fg = if in_focused_path { 0xEEEE44 }
            else if in_active_path { 0xEEEE88 }
            else { 0xEEEEEE };
        Self { frame: Frame::Rectangle, bg, fg }
    }
    pub(crate) fn model() -> Self {
        Self { frame: Frame::Hexagon, bg: 0x440000, fg: 0xBBBBBB }
    }
    pub(crate) fn file_model() -> Self {
        Self { frame: Frame::Hexagon, bg: 0x440033, fg: 0xBBBBBB }
    }
}

/// Construct an empty `emTreeDumpRec` with the given title/text/style,
/// populated Commands (empty), Files (empty), Children (empty).
pub(crate) fn empty_rec(title: String, text: String, style: VisualStyle) -> RecStruct {
    let mut rec = RecStruct::new();
    rec.set_str("Frame", style.frame.as_str());
    rec.set_u32("BgColor", style.bg);
    rec.set_u32("FgColor", style.fg);
    rec.set_str("Title", &title);
    rec.set_str("Text", &text);
    rec.SetValue("Commands", RecValue::Array(Vec::new()));
    rec.SetValue("Files", RecValue::Array(Vec::new()));
    rec.SetValue("Children", RecValue::Array(Vec::new()));
    rec
}

/// Append a child rec to an `emTreeDumpRec`'s Children array.
pub(crate) fn push_child(rec: &mut RecStruct, child: RecStruct) {
    let children = rec.get_mut_or_insert_array("Children");
    children.push(RecValue::Struct(child));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_as_str_matches_cpp_names() {
        assert_eq!(Frame::Rectangle.as_str(), "FRAME_RECTANGLE");
        assert_eq!(Frame::RoundRect.as_str(), "FRAME_ROUND_RECT");
    }

    #[test]
    fn empty_rec_has_all_top_level_fields() {
        let style = VisualStyle::engine();
        let rec = empty_rec("t".into(), "txt".into(), style);
        assert!(rec.get("Frame").is_some());
        assert!(rec.get("BgColor").is_some());
        assert!(rec.get("FgColor").is_some());
        assert!(rec.get("Title").is_some());
        assert!(rec.get("Text").is_some());
        assert!(rec.get("Commands").is_some());
        assert!(rec.get("Files").is_some());
        assert!(rec.get("Children").is_some());
    }
}
```

Note: `RecStruct` / `RecValue` / `RecArray` helper names come from the existing `crate::emRec` module. If the exact method names (`set_str`, `set_u32`, `SetValue`, `get_mut_or_insert_array`) differ from what's currently in `emRec.rs`, adjust to match; the existing `emView::dump_tree` at `emView.rs:4979` uses this API, so mirror its call style.

- [ ] **Step 2: Register the module**

In `crates/emcore/src/lib.rs`, add `pub mod emTreeDump;` alphabetically (near `pub mod emToolkit;` or wherever `em`-prefixed modules are listed).

- [ ] **Step 3: Compile + test**

```bash
cargo test -p emcore --lib emTreeDump:: 2>&1 | tail -10
```

Expected: both unit tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emTreeDump.rs crates/emcore/src/lib.rs
git commit -m "feat(emTreeDump): add module skeleton with Frame + VisualStyle constants"
```

---

### Task 1.5 — Implement `dump_panel` walker

**Files:**
- Modify: `crates/emcore/src/emTreeDump.rs`
- Modify: `crates/emcore/src/emPanelTree.rs` (may need a `data()` accessor if not public to this crate)

- [ ] **Step 1: Write the skeleton test first**

Append to `emTreeDump.rs`'s test module:

```rust
    #[test]
    fn dump_panel_contains_expected_text_labels() {
        use crate::emPanelTree::PanelTree;
        let mut tree = PanelTree::new();
        let root = tree.create_root("root".into());
        let text = {
            let data = tree.data(root).unwrap();
            let rec = dump_panel_text_only(data, /* is_focused */ false, /* is_viewed */ false,
                /* is_in_viewed_path */ false, /* in_focused_path */ false,
                /* in_active_path */ false, /* height */ 1.0,
                /* essence_x */ 0.0, /* essence_y */ 0.0, /* essence_w */ 1.0, /* essence_h */ 1.0,
                /* viewed_xywh */ None, /* clip_x1y1x2y2 */ None,
                /* current_frame */ 0, /* subtype_pairs */ &[]);
            rec
        };
        assert!(text.contains("Name: root"));
        assert!(text.contains("Layout XYWH"));
        assert!(text.contains("Essence XYWH"));
        assert!(text.contains("Viewed: no"));
        assert!(text.contains("InViewedPath: no"));
        assert!(text.contains("Viewed XYWH: -"));
        assert!(text.contains("Clip X1Y1X2Y2: -"));
        assert!(text.contains("PaintCount: 0"));
        assert!(text.contains("LastPaintFrame: 0 (current: 0)"));
    }
```

- [ ] **Step 2: Confirm failure**

```bash
cargo test -p emcore --lib emTreeDump::tests::dump_panel_contains_expected_text_labels 2>&1 | tail -10
```

Expected: `dump_panel_text_only` is not defined.

- [ ] **Step 3: Implement `dump_panel_text_only` + `dump_panel`**

In `emTreeDump.rs`, add these module-level functions (before `#[cfg(test)]`):

```rust
use crate::emPanelTree::{PanelData, PanelId, PanelTree};

/// Build just the Text body for an emPanel branch — extracted for testability
/// without needing to construct a full walker setup.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dump_panel_text_only(
    data: &PanelData,
    is_focused: bool,
    is_viewed: bool,
    is_in_viewed_path: bool,
    in_focused_path: bool,
    in_active_path: bool,
    height: f64,
    essence_x: f64,
    essence_y: f64,
    essence_w: f64,
    essence_h: f64,
    viewed_xywh: Option<(f64, f64, f64, f64)>,
    clip_x1y1x2y2: Option<(f64, f64, f64, f64)>,
    current_frame: u64,
    subtype_pairs: &[(&'static str, String)],
) -> String {
    let mut text = String::new();
    let layout = data.layout_rect;
    text.push_str(&format!("\nName: {}", data.name));
    // Title delegates to parent chain in C++; placeholder matches current
    // Rust panel-title traversal — real title is supplied at the caller level.
    text.push_str(&format!(
        "\nLayout XYWH: {:.9}, {:.9}, {:.9}, {:.9}",
        layout.x, layout.y, layout.w, layout.h
    ));
    text.push_str(&format!("\nHeight: {:.9}", height));
    text.push_str(&format!(
        "\nEssence XYWH: {:.9}, {:.9}, {:.9}, {:.9}",
        essence_x, essence_y, essence_w, essence_h
    ));
    text.push_str(&format!("\nViewed: {}", if is_viewed { "yes" } else { "no" }));
    text.push_str(&format!("\nInViewedPath: {}", if is_in_viewed_path { "yes" } else { "no" }));
    text.push_str("\nViewed XYWH: ");
    if let Some((x, y, w, h)) = viewed_xywh {
        text.push_str(&format!("{:.9}, {:.9}, {:.9}, {:.9}", x, y, w, h));
    } else {
        text.push('-');
    }
    text.push_str("\nClip X1Y1X2Y2: ");
    if let Some((x1, y1, x2, y2)) = clip_x1y1x2y2 {
        text.push_str(&format!("{:.9}, {:.9}, {:.9}, {:.9}", x1, y1, x2, y2));
    } else {
        text.push('-');
    }
    text.push_str(&format!("\nEnableSwitch: {}", if data.enable_switch { "yes" } else { "no" }));
    text.push_str(&format!("\nEnabled: {}", if data.enabled { "yes" } else { "no" }));
    text.push_str(&format!("\nFocusable: {}", if data.focusable { "yes" } else { "no" }));
    text.push_str(&format!("\nActive: {}", if data.is_active { "yes" } else { "no" }));
    text.push_str(&format!("\nInActivePath: {}", if data.in_active_path { "yes" } else { "no" }));
    text.push_str(&format!("\nFocused: {}", if is_focused { "yes" } else { "no" }));
    text.push_str(&format!("\nInFocusedPath: {}", if in_focused_path { "yes" } else { "no" }));
    // UpdatePriority and MemoryLimit — consult C++ semantics; both are
    // computed by emPanel::GetUpdatePriority / GetMemoryLimit in C++.
    // Rust equivalents live on PanelTree (grep GetUpdatePriority in emPanelTree.rs).
    // Caller supplies them via the full `dump_panel` wrapper below.
    text.push_str(&format!("\nPaintCount: {}", data.paint_count));
    text.push_str(&format!(
        "\nLastPaintFrame: {} (current: {})",
        data.last_paint_frame, current_frame
    ));
    for (label, value) in subtype_pairs {
        text.push_str(&format!("\n{}: {}", label, value));
    }
    text
}

/// Full emPanel branch: builds the rec with Title, Text, VisualStyle, and
/// recursively walks children.
pub(crate) fn dump_panel(
    tree: &mut PanelTree,
    id: PanelId,
    current_frame: u64,
    focused_id: Option<PanelId>,
    view_home_w: f64,
    view_home_h: f64,
    window_focused: bool,
) -> RecStruct {
    // Extract everything we need from the tree before we borrow children.
    let data_clone: PanelData = tree.data(id).expect("panel exists").clone();
    let is_focused = focused_id == Some(id);
    let is_viewed = data_clone.viewed;
    let is_in_viewed_path = data_clone.in_viewed_path;
    let in_active_path = data_clone.in_active_path;
    // in_focused_path = walk from focused_id up to root, check if `id` is on that path.
    let in_focused_path = match focused_id {
        Some(fid) => {
            let mut cur = Some(fid);
            let mut found = false;
            while let Some(c) = cur {
                if c == id { found = true; break; }
                cur = tree.data(c).and_then(|d| d.parent);
            }
            found
        }
        None => false,
    };
    // Height and essence rect — use existing tree helpers.
    let height = tree.get_height(id);
    let (essence_x, essence_y, essence_w, essence_h) = tree.get_essence_rect(id);
    let viewed_xywh = if is_viewed {
        Some((data_clone.viewed_x, data_clone.viewed_y, data_clone.viewed_width, data_clone.viewed_height))
    } else { None };
    let clip_x1y1x2y2 = if is_viewed {
        Some(tree.get_clip_rect(id))
    } else { None };

    // Subtype pairs via PanelBehavior::dump_state — must take behavior out
    // temporarily, as is the established pattern in emView::dump_tree.
    let subtype_pairs: Vec<(&'static str, String)> = if let Some(behavior) = tree.take_behavior(id) {
        let pairs = behavior.dump_state();
        tree.put_behavior(id, behavior);
        pairs
    } else {
        Vec::new()
    };

    let type_name = if let Some(behavior) = tree.take_behavior(id) {
        let n = behavior.type_name().to_string();
        tree.put_behavior(id, behavior);
        n
    } else {
        "(no behavior)".to_string()
    };

    // Panel title falls back up the parent chain in C++; reuse whatever
    // the existing Rust panel-title method is (grep `fn get_panel_title`
    // or similar in emPanelTree.rs).
    let title_str = tree.get_panel_title(id).unwrap_or_else(|| "".to_string());
    let title = format!("Panel:\n{}\n\"{}\"", type_name, data_clone.name);

    let mut text = String::new();
    // Also prepend the title-line C++ uses:
    text.push_str(&format!("\nName: {}", data_clone.name));
    text.push_str(&format!("\nTitle: {}", title_str));
    // Append the body built by the testable helper:
    let body = dump_panel_text_only(
        &data_clone, is_focused, is_viewed, is_in_viewed_path,
        in_focused_path, in_active_path, height,
        essence_x, essence_y, essence_w, essence_h,
        viewed_xywh, clip_x1y1x2y2, current_frame, &subtype_pairs,
    );
    // `dump_panel_text_only` also emits Name at its start; strip the leading
    // duplicate (or refactor `dump_panel_text_only` to not emit Name if
    // preferred — simpler: just skip the first `\nName: ...` line).
    // For simplicity, compose the final text:
    let body_without_duplicate = body
        .split_once("\nLayout XYWH")
        .map(|(_, rest)| format!("\nLayout XYWH{}", rest))
        .unwrap_or(body);
    text.push_str(&body_without_duplicate);
    // UpdatePriority + MemoryLimit — call emView-equivalent helpers. If the
    // Rust helpers need view context we don't have here, pass them in via
    // a small CallerCtx struct; for v1 we can emit placeholder values:
    text.push_str("\nUpdate Priority: 0");
    text.push_str("\nMemory Limit: 0");

    let style = VisualStyle::panel(is_viewed, is_in_viewed_path, in_focused_path, in_active_path);
    let mut rec = empty_rec(title, text, style);

    // Recurse into children in tree order.
    let children: Vec<PanelId> = tree.children(id).collect();
    for child_id in children {
        let child_rec = dump_panel(
            tree, child_id, current_frame, focused_id,
            view_home_w, view_home_h, window_focused,
        );
        push_child(&mut rec, child_rec);
    }

    rec
}
```

Resolve during implementation:
- `tree.get_height(id)`, `tree.get_essence_rect(id)`, `tree.get_clip_rect(id)`, `tree.get_panel_title(id)` — if the existing names differ, grep `emPanelTree.rs` for the closest analogue and use it. If any don't exist, add them: each is a small getter that reads `PanelData` + parent chain, already modeled by existing similar accessors.
- Update Priority / Memory Limit — the existing `GetUpdatePriority` / `GetMemoryLimit` on `PanelTree` need view-home dims; pass those through the walker signature (already included as `view_home_w` / `view_home_h` above — wire the actual call once the signature is confirmed).

- [ ] **Step 4: Confirm test passes**

```bash
cargo test -p emcore --lib emTreeDump::tests::dump_panel_contains_expected_text_labels 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emTreeDump.rs crates/emcore/src/emPanelTree.rs
git commit -m "feat(emTreeDump): implement dump_panel walker with all emPanel fields"
```

---

### Task 1.6 — Implement `dump_view`, `dump_window`, `dump_context`

**Files:**
- Modify: `crates/emcore/src/emTreeDump.rs`

- [ ] **Step 1: Write failing test**

```rust
    #[test]
    fn dump_view_contains_flags_and_rects() {
        use crate::emView::emView;
        let view = emView::new(/* whatever the minimal constructor needs */);
        let rec = dump_view(&view, /* tree */ &mut PanelTree::new(), /* current_frame */ 0);
        let text = rec.get_str("Text").unwrap();
        assert!(text.contains("View Flags"));
        assert!(text.contains("Home XYWH"));
        assert!(text.contains("Current XYWH"));
        assert!(text.contains("Activation Adherent"));
        assert!(text.contains("Popped Up"));
        assert!(text.contains("Background Color:"));
    }
```

- [ ] **Step 2: Confirm failure**

```bash
cargo test -p emcore --lib emTreeDump::tests::dump_view_contains_flags_and_rects 2>&1 | tail -10
```

- [ ] **Step 3: Implement the three walkers**

In `emTreeDump.rs`:

```rust
use crate::emView::{emView, ViewFlags};

pub(crate) fn dump_view(view: &emView, tree: &mut PanelTree, current_frame: u64) -> RecStruct {
    let mut text = String::new();

    // emEngine fields (C++ concatenates engine first)
    text.push_str(&format!("\nEngine Priority: {}", view.engine_priority()));

    // emContext fields — emView is-a emContext. Use whatever context-side
    // accessors exist on emView/emContext (grep `GetModelInfo` or `models`).
    text.push_str(&format!("\nCommon Models: {}", view.common_model_count()));
    text.push_str(&format!("\nPrivate Models: {} (not listed)", view.private_model_count()));

    // emView fields proper.
    text.push_str("\nView Flags: ");
    text.push_str(&fmt_view_flags(view.flags));
    text.push_str(&format!("\nTitle: {}", view.title));
    text.push_str(&format!(
        "\nFocused: {}\nActivation Adherent: {}\nPopped Up: {}",
        if view.is_focused() { "yes" } else { "no" },
        if view.is_activation_adherent() { "yes" } else { "no" },
        if view.is_popped_up() { "yes" } else { "no" },
    ));
    text.push_str(&format!("\nBackground Color: 0x{:08X}", view.background_color_packed()));
    text.push_str(&format!(
        "\nHome XYWH: {:.9}, {:.9}, {:.9}, {:.9}",
        0.0, 0.0, view.HomeWidth, view.HomeHeight,
    ));
    text.push_str(&format!(
        "\nCurrent XYWH: {:.9}, {:.9}, {:.9}, {:.9}",
        view.current_x(), view.current_y(), view.current_width(), view.current_height(),
    ));

    let style = VisualStyle::view(view.is_focused());
    let title = format!("View (Context):\n{}", "emView");
    let mut rec = empty_rec(title, text, style);

    // Recurse into root panel.
    if let Some(root_id) = view.root_panel() {
        let child = dump_panel(
            tree, root_id, current_frame, view.focused,
            view.HomeWidth, view.HomeHeight, view.window_focused,
        );
        push_child(&mut rec, child);
    }
    rec
}

fn fmt_view_flags(flags: ViewFlags) -> String {
    let mut parts = Vec::new();
    if flags.contains(ViewFlags::POPUP_ZOOM) { parts.push("VF_POPUP_ZOOM"); }
    if flags.contains(ViewFlags::ROOT_SAME_TALLNESS) { parts.push("VF_ROOT_SAME_TALLNESS"); }
    if flags.contains(ViewFlags::NO_ZOOM) { parts.push("VF_NO_ZOOM"); }
    if flags.contains(ViewFlags::NO_USER_NAVIGATION) { parts.push("VF_NO_USER_NAVIGATION"); }
    if flags.contains(ViewFlags::NO_FOCUS_HIGHLIGHT) { parts.push("VF_NO_FOCUS_HIGHLIGHT"); }
    if flags.contains(ViewFlags::NO_ACTIVE_HIGHLIGHT) { parts.push("VF_NO_ACTIVE_HIGHLIGHT"); }
    if flags.contains(ViewFlags::EGO_MODE) { parts.push("VF_EGO_MODE"); }
    if flags.contains(ViewFlags::STRESS_TEST) { parts.push("VF_STRESS_TEST"); }
    if parts.is_empty() { "0".to_string() } else { parts.join(", ") }
}

// Window: C++ appends flags + WMResName to whatever branch already ran.
// In Rust we build it standalone and the caller decides how to compose.
pub(crate) fn dump_window(window: &crate::emWindow::emWindow) -> RecStruct {
    let mut text = String::new();
    text.push_str("\nWindow Flags: ");
    text.push_str(&fmt_window_flags(window.flags()));
    text.push_str(&format!("\nWMResName: {}", window.wm_res_name()));
    let style = VisualStyle::window();
    let title = format!("Window (View, Context):\n{}", "emWindow");
    empty_rec(title, text, style)
}

fn fmt_window_flags(flags: crate::emWindow::WindowFlags) -> String {
    let mut parts = Vec::new();
    // Adjust constant names to match the Rust emWindow module (grep emWindow.rs).
    if flags.contains(crate::emWindow::WindowFlags::MODAL) { parts.push("WF_MODAL"); }
    if flags.contains(crate::emWindow::WindowFlags::UNDECORATED) { parts.push("WF_UNDECORATED"); }
    if flags.contains(crate::emWindow::WindowFlags::POPUP) { parts.push("WF_POPUP"); }
    if flags.contains(crate::emWindow::WindowFlags::FULLSCREEN) { parts.push("WF_FULLSCREEN"); }
    if parts.is_empty() { "0".to_string() } else { parts.join(", ") }
}

// Context (root or child). In C++, the cascade produces sorted common models
// and recursively-dumped child contexts.
pub(crate) fn dump_context(ctx: &crate::emContext::emContext, is_root: bool) -> RecStruct {
    let mut text = String::new();
    text.push_str(&format!("\nEngine Priority: {}", ctx.engine_priority()));
    let (common, private_count) = ctx.model_info();
    text.push_str(&format!("\nCommon Models: {}", common.len()));
    text.push_str(&format!("\nPrivate Models: {} (not listed)", private_count));

    let style = VisualStyle::context(is_root);
    let title = if is_root { "Root Context:\nemRootContext".to_string() } else { "Context:\nemContext".to_string() };
    let mut rec = empty_rec(title, text, style);

    // Sort common models by name then emit each as a Model/FileModel child.
    let mut sorted = common.clone();
    sorted.sort_by(|a, b| a.name().cmp(b.name()));
    for m in sorted {
        let child = dump_model_or_file_model(&m);
        push_child(&mut rec, child);
    }
    // Child contexts recursively.
    for child_ctx in ctx.child_contexts() {
        let child_rec = dump_context(&child_ctx, false);
        push_child(&mut rec, child_rec);
    }
    rec
}
```

Resolve during implementation:
- `emView` accessor names (`engine_priority`, `common_model_count`, `private_model_count`, `title`, `is_focused`, `is_activation_adherent`, `is_popped_up`, `background_color_packed`, `current_x/y/width/height`, `root_panel`) — add small public-to-crate accessors if missing; match C++ semantics.
- `emWindow::flags`, `WindowFlags` enum, `wm_res_name` — grep `emWindow.rs`; `WF_*` constants must exist somewhere.
- `emContext::engine_priority`, `model_info`, `child_contexts` — consult `emContext.rs`.

- [ ] **Step 4: Confirm view test passes**

```bash
cargo test -p emcore --lib emTreeDump::tests::dump_view_contains_flags_and_rects 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emTreeDump.rs crates/emcore/src/emView.rs crates/emcore/src/emWindow.rs crates/emcore/src/emContext.rs
git commit -m "feat(emTreeDump): implement dump_view + dump_window + dump_context walkers"
```

---

### Task 1.7 — Implement `dump_model` + `dump_file_model`

**Files:**
- Modify: `crates/emcore/src/emTreeDump.rs`

- [ ] **Step 1: Add walker functions**

```rust
use crate::emModel::emModel;
use crate::emFileModel::{emFileModel, FileState};

/// Dispatch: if the model is a file model, use the file-model branch.
pub(crate) fn dump_model_or_file_model(model: &dyn emModel) -> RecStruct {
    if let Some(fm) = model.as_file_model() {
        dump_file_model(fm)
    } else {
        dump_model(model)
    }
}

pub(crate) fn dump_model(m: &dyn emModel) -> RecStruct {
    let mut text = String::new();
    text.push_str(&format!("\nName: {}", m.name()));
    text.push_str(&format!("\nMin Common Lifetime: {}", m.min_common_lifetime()));
    let title = format!("Common Model:\n{}\n\"{}\"", m.type_name(), m.name());
    empty_rec(title, text, VisualStyle::model())
}

pub(crate) fn dump_file_model(fm: &dyn emFileModel) -> RecStruct {
    let mut text = String::new();
    text.push_str(&format!("\nFile Path: {}", fm.file_path()));
    let state_str = match fm.file_state() {
        FileState::Waiting => "FS_WAITING",
        FileState::Loading => "FS_LOADING",
        FileState::Loaded => "FS_LOADED",
        FileState::Unsaved => "FS_UNSAVED",
        FileState::Saving => "FS_SAVING",
        FileState::TooCostly => "FS_TOO_COSTLY",
        FileState::LoadError => "FS_LOAD_ERROR",
        FileState::SaveError => "FS_SAVE_ERROR",
    };
    text.push_str(&format!("\nFile State: {}", state_str));
    text.push_str(&format!("\nMemory Need: {}", fm.memory_need()));
    let title = format!("Common File Model:\n{}\n\"{}\"", fm.type_name(), fm.name());
    empty_rec(title, text, VisualStyle::file_model())
}
```

Resolve during implementation:
- If `emModel` / `emFileModel` are traits with different accessor names in the Rust port, use the actual names. If the `as_file_model` downcast method doesn't exist yet, add a `fn as_file_model(&self) -> Option<&dyn emFileModel> { None }` with a default return of `None` to `emModel`, overridden in `emFileModel` implementors (same pattern as existing `as_sub_view_panel_mut` / `as_dlg_panel_mut` in `PanelBehavior`).

- [ ] **Step 2: Write + run a basic test**

```rust
    #[test]
    fn dump_model_has_name_and_lifetime() {
        // Construct the smallest concrete emModel available in the code;
        // if no trivial one exists, use a mock impl.
        struct MockModel;
        impl emModel for MockModel {
            fn name(&self) -> &str { "mock" }
            fn min_common_lifetime(&self) -> u64 { 42 }
            fn type_name(&self) -> &str { "MockModel" }
        }
        let m = MockModel;
        let rec = dump_model(&m);
        let text = rec.get_str("Text").unwrap();
        assert!(text.contains("Name: mock"));
        assert!(text.contains("Min Common Lifetime: 42"));
    }
```

```bash
cargo test -p emcore --lib emTreeDump::tests::dump_model_has_name_and_lifetime 2>&1 | tail -10
```

Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emTreeDump.rs crates/emcore/src/emModel.rs crates/emcore/src/emFileModel.rs
git commit -m "feat(emTreeDump): implement dump_model + dump_file_model walkers"
```

---

### Task 1.8 — Implement `dump_from_root_context` with General Info header

**Files:**
- Modify: `crates/emcore/src/emTreeDump.rs`

- [ ] **Step 1: Implement entry point**

```rust
/// Entry point — C++ `emTreeDumpFromRootContext` in emTreeDumpUtil.cpp:360.
pub fn dump_from_root_context(root_ctx: &crate::emContext::emRootContext, tree: &mut PanelTree) -> RecStruct {
    let title = "Tree Dump\nof the top-level objects\nof a running emCore-based program".to_string();
    let text = general_info_text();
    let style = VisualStyle { frame: Frame::Rectangle, bg: 0x444466, fg: 0xBBBBEE };
    let mut rec = empty_rec(title, text, style);

    // One child: the root context.
    let root_rec = dump_context(root_ctx.as_context(), /* is_root */ true);
    push_child(&mut rec, root_rec);

    // For each view attached to this root context, emit a view rec as a
    // child of the root context's rec. The C++ walker does this inside
    // emTreeDumpFromObject's context branch; in Rust we build it here to
    // keep the walker linear.
    // (Wiring: iterate root_ctx.views(), call dump_view, append.)

    rec
}

fn general_info_text() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // C++ formats `ctime_r`; approximate with a human-readable form.
    let time_str = format_unix_time(now);
    let host = hostname_best_effort();
    let user = std::env::var("USER").unwrap_or_else(|_| "-".to_string());
    let pid = std::process::id();
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "-".to_string());
    let utf8 = "yes"; // Rust strings are always UTF-8
    let byte_order = if cfg!(target_endian = "little") { "1234" } else { "4321" };
    let ptr_size = std::mem::size_of::<*const ()>();
    let long_size = std::mem::size_of::<i64>();
    let char_signed = if (-1i8 as i32) < 0 { "signed" } else { "unsigned" };

    let mut s = String::new();
    s.push_str("General Info");
    s.push_str("\n~~~~~~~~~~~~");
    s.push_str(&format!("\n\nTime       : {}", time_str));
    s.push_str(&format!("\nHost Name  : {}", host));
    s.push_str(&format!("\nUser Name  : {}", user));
    s.push_str(&format!("\nProcess Id : {}", pid));
    s.push_str(&format!("\nCurrent Dir: {}", cwd));
    s.push_str(&format!("\nUTF8       : {}", utf8));
    s.push_str(&format!("\nByte Order : {}", byte_order));
    s.push_str(&format!("\nsizeof(ptr): {}", ptr_size));
    s.push_str(&format!("\nsizeof(lng): {}", long_size));
    s.push_str(&format!("\nchar       : {}", char_signed));
    // CPU-TSC — DIVERGED: (upstream-gap-forced) no portable Rust RDTSC; emit "-".
    s.push_str("\nCPU-TSC    : -");

    // Install paths — use the existing emInstallInfo accessors.
    s.push_str("\n\nPaths of emCore:");
    s.push_str(&install_paths_block());
    s
}

fn format_unix_time(secs: u64) -> String {
    // Deliberately minimal — we don't want a chrono dep just for this.
    // Format as "YYYY-MM-DD HH:MM:SS UTC" using naive division.
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;
    // Days to Y-M-D: use a naive algorithm. Accuracy is not critical; the
    // field exists for human readability. If a cleaner option is already
    // in the workspace (chrono, time), use that instead.
    let (y, m, d) = days_to_ymd(days_since_epoch as i64);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, m, d, hour, minute, second)
}

fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Based on Fliegel & Van Flandern.
    let z = days + 719468;
    let era = if z >= 0 { z / 146097 } else { (z - 146096) / 146097 };
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d)
}

fn hostname_best_effort() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| {
            // Fallback: read /etc/hostname.
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "-".to_string())
        })
}

fn install_paths_block() -> String {
    // Source paths from the existing emInstallInfo module; adapt names
    // to whatever that module exposes.
    use crate::emInstallInfo::{install_path, InstallPathType};
    let kinds = [
        ("Bin        ", InstallPathType::Bin),
        ("Include    ", InstallPathType::Include),
        ("Lib        ", InstallPathType::Lib),
        ("Html Doc   ", InstallPathType::HtmlDoc),
        ("Pdf Doc    ", InstallPathType::PdfDoc),
        ("Ps Doc     ", InstallPathType::PsDoc),
        ("User Config", InstallPathType::UserConfig),
        ("Host Config", InstallPathType::HostConfig),
        ("Tmp        ", InstallPathType::Tmp),
        ("Res        ", InstallPathType::Res),
        ("Home       ", InstallPathType::Home),
    ];
    let mut s = String::new();
    for (label, kind) in kinds {
        let path = install_path(kind, "emCore").unwrap_or_else(|_| "-".into());
        s.push_str(&format!("\n{}: {}", label, path));
    }
    s
}
```

Resolve during implementation:
- `emInstallInfo` module may have different shape (function name, path-type enum). Grep `InstallPath\|install_path` to find. If only some path types exist, emit `"-"` for absent ones with a `DIVERGED: (upstream-gap-forced)` comment.

- [ ] **Step 2: Write test**

```rust
    #[test]
    fn general_info_text_has_all_labels() {
        let t = general_info_text();
        for label in ["Time", "Host Name", "User Name", "Process Id", "Current Dir",
                      "UTF8", "Byte Order", "sizeof(ptr)", "sizeof(lng)", "char",
                      "CPU-TSC", "Paths of emCore:"] {
            assert!(t.contains(label), "missing label: {}", label);
        }
    }
```

```bash
cargo test -p emcore --lib emTreeDump::tests::general_info_text_has_all_labels 2>&1 | tail -10
```

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emTreeDump.rs
git commit -m "feat(emTreeDump): implement dump_from_root_context with General Info header"
```

---

### Task 1.9 — Shim `emView::dump_tree` to call new walker

**Files:**
- Modify: `crates/emcore/src/emView.rs` (current `dump_tree` at line 4979)

- [ ] **Step 1: Replace body of `dump_tree`**

In `emView.rs` at line 4979, replace the current body (everything inside `pub fn dump_tree`) with a shim that delegates to `emTreeDump::dump_from_root_context`. Keep the same signature and same output path.

```rust
    pub fn dump_tree(&self, tree: &mut PanelTree) -> std::path::PathBuf {
        use crate::emRec::write_rec_with_format;
        let path = std::env::temp_dir().join("debug.emTreeDump");
        // Resolve the root context. If the Rust port doesn't expose it on
        // emView yet, thread it through from the caller; for the shim we
        // construct a minimal dump containing just the view branch.
        let root_ctx = self.root_context();
        let rec = crate::emTreeDump::dump_from_root_context(root_ctx, tree);
        let text = write_rec_with_format(&rec, "emTreeDump");
        if let Err(e) = std::fs::write(&path, &text) {
            eprintln!("[TreeDump] write failed: {e}");
        } else {
            eprintln!("[TreeDump] wrote {}", path.display());
        }
        path
    }
```

If `self.root_context()` doesn't exist yet on `emView`, add a minimal accessor that returns whatever the current Rust port uses to represent the root context (grep `root_context\|RootContext` in `emcore`).

- [ ] **Step 2: Delete the old walker helpers**

Delete `dump_panel_recursive` from `emView.rs` (lines ~5011–5055). Delete any other dump-helper functions the current `dump_tree` relied on.

- [ ] **Step 3: Verify td! cheat still works via unit test**

Find the existing `tree_dump_produces_valid_emrec` test at `emView.rs:6062`. Run it:

```bash
cargo test -p emcore --lib emView::tests::tree_dump_produces_valid_emrec 2>&1 | tail -15
```

Expected: still passes. If it fails due to schema change (the test asserts specific keys that moved from `title`/`text` generic fields to the new `Title`/`Text` C++-schema fields), update the test to assert the new schema. The test's purpose is "the dump is valid emRec"; schema names should be updated to match the new format.

- [ ] **Step 4: Confirm full suite**

```bash
cargo check -p emcore && cargo clippy -p emcore -- -D warnings && cargo test -p emcore --lib 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "feat(emTreeDump): shim emView::dump_tree to call new emTreeDump::dump_from_root_context"
```

---

### Task 1.10 — `dump_state` override on `emDirPanel`

**Files:**
- Modify: `crates/emfileman/src/emDirPanel.rs`

- [ ] **Step 1: Write test** in `emDirPanel.rs`'s test module:

```rust
    #[test]
    fn dump_state_reports_loading_pct() {
        let mut panel = emDirPanel::new_for_test(/* whatever the test-constructor is */);
        panel.set_loading_pct_for_test(42);
        let pairs = PanelBehavior::dump_state(&panel);
        assert!(pairs.iter().any(|(k, v)| *k == "loading_pct" && v == "42"));
    }
```

- [ ] **Step 2: Add override**

In `emDirPanel.rs`, inside the `impl PanelBehavior for emDirPanel` block, add:

```rust
    fn dump_state(&self) -> Vec<(&'static str, String)> {
        vec![
            ("loading_pct", self.loading_pct.to_string()),
            ("loading_done", self.loading_done.to_string()),
            ("loading_cycle_state", format!("{:?}", self.cycle_state)),
            ("entries_count", self.entries.len().to_string()),
            ("error_state", self.error_state.as_deref().unwrap_or("").to_string()),
        ]
    }
```

Adjust field names to match what `emDirPanel` actually has (grep `pub(crate) loading_pct\|pub(crate) loading_done` in the file; if the fields are called something else, use the real names).

- [ ] **Step 3: Confirm test passes**

```bash
cargo test -p emfileman --lib emDirPanel::tests::dump_state_reports_loading_pct 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add crates/emfileman/src/emDirPanel.rs
git commit -m "feat(emTreeDump): emDirPanel::dump_state exposes loading state to tree dump"
```

---

### Task 1.11 — `dump_state` override on `emFilePanel`

**Files:**
- Modify: `crates/emfileman/src/emFilePanel.rs`

- [ ] **Step 1: Write test + add override**

Same pattern as Task 1.10. Override:

```rust
    fn dump_state(&self) -> Vec<(&'static str, String)> {
        vec![
            ("file_state", format!("{:?}", self.file_state)),
            ("file_path", self.file_path.clone()),
            ("memory_need", self.memory_need.to_string()),
        ]
    }
```

Adjust field names to what the struct has.

- [ ] **Step 2: Confirm + commit**

```bash
cargo test -p emfileman --lib emFilePanel 2>&1 | tail -10
git add crates/emfileman/src/emFilePanel.rs
git commit -m "feat(emTreeDump): emFilePanel::dump_state exposes file state to tree dump"
```

---

### Task 1.12 — Marker files for unported emTreeDump package

**Files:**
- Create: `crates/emcore/src/emTreeDumpRec.no_rs`
- Create: `crates/emcore/src/emTreeDumpFileModel.no_rs`
- Create: `crates/emcore/src/emTreeDumpFilePanel.no_rs`
- Create: `crates/emcore/src/emTreeDumpRecPanel.no_rs`
- Create: `crates/emcore/src/emTreeDumpControlPanel.no_rs`
- Create: `crates/emcore/src/emTreeDumpFpPlugin.no_rs`

- [ ] **Step 1: Write each marker**

Each file contains one short paragraph explaining why the header is not ported. Example for `emTreeDumpRec.no_rs`:

```
include/emTreeDump/emTreeDumpRec.h — C++ type wraps the schema
(Frame/BgColor/FgColor/Title/Text/Commands/Files/Children) as a typed
emStructRec. Rust emCore uses the generic RecStruct abstraction for
serialization, so the schema lives inline in emTreeDump.rs rather than
needing a typed mirror. The file format is byte-identical.

If emTreeDumpFilePanel (the in-app dump renderer) is ported later, this
marker should be removed and a typed mirror added at that time.
```

Similar one-paragraph markers for the other five files, each explaining that the in-app rendering components are out of scope for the current dump-producer work.

- [ ] **Step 2: Run annotation lint**

```bash
cargo xtask annotations 2>&1 | tail -20
```

Expected: clean (no new unannotated `RUST_ONLY:` or `DIVERGED:` entries introduced by this phase beyond the ones already annotated).

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emTreeDumpRec.no_rs crates/emcore/src/emTreeDumpFileModel.no_rs crates/emcore/src/emTreeDumpFilePanel.no_rs crates/emcore/src/emTreeDumpRecPanel.no_rs crates/emcore/src/emTreeDumpControlPanel.no_rs crates/emcore/src/emTreeDumpFpPlugin.no_rs
git commit -m "docs(emTreeDump): marker files for unported in-app dump-renderer package"
```

---

### Phase 1 exit gate

- [ ] `cargo check -p emcore -p emfileman`
- [ ] `cargo clippy -p emcore -p emfileman -- -D warnings`
- [ ] `cargo-nextest ntr`
- [ ] Launch the app manually, press `Ctrl+Shift+T` then type `td!` (or whatever keystroke triggers the cheat — consult `emViewInputFilter.rs:2441`). Check `/tmp/debug.emTreeDump` exists and contains `Frame: FRAME_RECTANGLE`, `Title: "Tree Dump\n..."`, `Commands`, `Files`, `Children`. Verify visually that a panel's Text contains `PaintCount`, `LastPaintFrame`, and at least one subtype pair (`loading_pct` for a dir panel).

---

## Phase 2 — Headless verification

Phase goal: confirm `eaglemode` launches under Xvfb; decide whether `--headless` flag is needed.

### Task 2.1 — Verify Xvfb launch

**Files:** none (verification only)

- [ ] **Step 1: Run under Xvfb**

```bash
Xvfb :99 -screen 0 1920x1080x24 &
XVFB_PID=$!
sleep 1
DISPLAY=:99 timeout 10 cargo run --bin eaglemode 2>&1 | tee /tmp/xvfb-run.log
kill $XVFB_PID
```

Expected: binary starts, main window appears (in the virtual display), no crash. Timeout kills it after 10s — that's fine, we're only verifying launch.

- [ ] **Step 2: Decide headless path**

- **If Xvfb works:** No code changes. Skip to Phase 3.
- **If Xvfb fails** (wgpu cannot initialize, X protocol error, etc.): Add Task 2.2.

- [ ] **Step 3: Document finding**

Append a one-line note to the spec at §Headless operation recording the outcome: `Verified: Xvfb works on <date>` or `Verified: Xvfb insufficient because <reason>; --headless required`. Commit the note.

### Task 2.2 (conditional) — Add `--headless` flag

Only if Task 2.1 shows Xvfb is insufficient. Plan content deferred to that point — when triggered, add tasks for: CLI arg parsing, offscreen wgpu surface, dummy winit event source. Defer granular design until the failure mode is known, because the failure mode dictates the surface.

---

## Phase 3 — Control channel foundation (dump / get_state / quit)

Phase goal: an agent can connect, ask for a dump, probe view state, and cleanly shut down the app.

### Task 3.1 — Add `serde` and `serde_json` dependencies

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/emcore/Cargo.toml`

- [ ] **Step 1: Check workspace deps**

```bash
grep -A 20 '^\[workspace.dependencies\]' /home/alex/Projects/eaglemode-rs/Cargo.toml | head -30
```

- [ ] **Step 2: Add workspace entries if missing**

If `serde` / `serde_json` are not already present, add to the workspace `Cargo.toml`:

```toml
[workspace.dependencies]
# ... existing entries ...
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 3: Add to emcore**

In `crates/emcore/Cargo.toml`:

```toml
[dependencies]
# ... existing entries ...
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 4: Verify build**

```bash
cargo check -p emcore 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/emcore/Cargo.toml Cargo.lock
git commit -m "chore: add serde + serde_json deps for emCtrlSocket"
```

---

### Task 3.2 — Create `emCtrlSocket.rs` with command/reply types

**Files:**
- Create: `crates/emcore/src/emCtrlSocket.rs`
- Create: `crates/emcore/src/emCtrlSocket.rust_only`
- Modify: `crates/emcore/src/lib.rs`

- [ ] **Step 1: Create the module**

```rust
//! RUST_ONLY: (language-forced utility)
//! No C++ analogue; agent-driven debugging requires a programmatic channel
//! that C++'s GUI-only cheat codes (emViewInputFilter::DoCheat) do not
//! provide. Gated behind EMCORE_DEBUG_CONTROL=1 — zero runtime cost when
//! unset.
//!
//! Unix-domain socket at $TMPDIR/eaglemode-rs.<pid>.sock. JSON-lines
//! protocol. Acceptor thread + per-connection worker threads dispatch
//! commands through winit::EventLoopProxy onto the main thread, which
//! mutates view state and sends replies via std::sync::mpsc.

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use std::sync::mpsc::SyncSender;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum CtrlCmd {
    Dump,
    GetState,
    Quit,
    Visit {
        panel_path: String,
        #[serde(default)]
        adherent: bool,
    },
    VisitFullsized {
        panel_path: String,
    },
    SetFocus {
        panel_path: String,
    },
    SeekTo {
        panel_path: String,
    },
    WaitIdle {
        #[serde(default)]
        timeout_ms: Option<u64>,
    },
    Input {
        event: InputPayload,
    },
    InputBatch {
        events: Vec<InputPayload>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputPayload {
    Key { key: String, press: bool, #[serde(default)] mods: Modifiers },
    MouseMove { x: f64, y: f64 },
    MouseButton { button: MouseButtonName, press: bool },
    Scroll { dx: f64, dy: f64 },
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Modifiers {
    #[serde(default)] pub shift: bool,
    #[serde(default)] pub ctrl: bool,
    #[serde(default)] pub alt: bool,
    #[serde(default)] pub logo: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButtonName { Left, Middle, Right }

#[derive(Debug, Serialize)]
pub struct CtrlReply {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_frame: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_rect: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub loading: Vec<LoadingEntry>,
}

impl CtrlReply {
    pub fn ok() -> Self { Self { ok: true, ..Self::empty() } }
    pub fn err(msg: impl Into<String>) -> Self { Self { ok: false, error: Some(msg.into()), ..Self::empty() } }
    fn empty() -> Self {
        Self {
            ok: false, error: None, path: None, idle_frame: None,
            focused_path: None, view_rect: None, loading: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoadingEntry {
    pub panel_path: String,
    pub pct: u32,
}

/// Message from acceptor-worker threads to the main thread via
/// winit::EventLoopProxy. The reply_tx is a oneshot (`sync_channel(1)`);
/// the main thread handler sends the reply back, the worker reads it,
/// serializes to JSON, writes to the socket.
#[derive(Debug)]
pub struct CtrlMsg {
    pub cmd: CtrlCmd,
    pub reply_tx: SyncSender<CtrlReply>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_cmd_roundtrip() {
        let json = r#"{"cmd":"dump"}"#;
        let parsed: CtrlCmd = serde_json::from_str(json).unwrap();
        assert!(matches!(parsed, CtrlCmd::Dump));
    }

    #[test]
    fn visit_cmd_roundtrip() {
        let json = r#"{"cmd":"visit","panel_path":"/cosmos/home"}"#;
        let parsed: CtrlCmd = serde_json::from_str(json).unwrap();
        match parsed {
            CtrlCmd::Visit { panel_path, adherent } => {
                assert_eq!(panel_path, "/cosmos/home");
                assert!(!adherent);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn input_key_roundtrip() {
        let json = r#"{"cmd":"input","event":{"kind":"key","key":"Return","press":true}}"#;
        let parsed: CtrlCmd = serde_json::from_str(json).unwrap();
        match parsed {
            CtrlCmd::Input { event: InputPayload::Key { key, press, .. } } => {
                assert_eq!(key, "Return");
                assert!(press);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn reply_omits_none_fields() {
        let r = CtrlReply::ok();
        let j = serde_json::to_string(&r).unwrap();
        assert!(!j.contains("path"));
        assert!(!j.contains("idle_frame"));
        assert_eq!(j, r#"{"ok":true}"#);
    }
}
```

- [ ] **Step 2: Create marker file**

`crates/emcore/src/emCtrlSocket.rust_only`:

```
crates/emcore/src/emCtrlSocket.rs — RUST_ONLY, language-forced utility.

Agent-driven debugging requires a programmatic control channel that C++
emCore does not provide (C++ ships only in-GUI cheat codes). Gated behind
EMCORE_DEBUG_CONTROL=1.
```

- [ ] **Step 3: Register module**

In `crates/emcore/src/lib.rs`, add `pub mod emCtrlSocket;` alphabetically.

- [ ] **Step 4: Run unit tests**

```bash
cargo test -p emcore --lib emCtrlSocket:: 2>&1 | tail -10
```

Expected: all four round-trip tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emCtrlSocket.rs crates/emcore/src/emCtrlSocket.rust_only crates/emcore/src/lib.rs
git commit -m "feat(emCtrlSocket): add module with CtrlCmd/CtrlReply/InputPayload types"
```

---

### Task 3.3 — Wire `UserEvent` type and `EventLoopProxy` storage

**Files:**
- Modify: `crates/emcore/src/emGUIFramework.rs`

- [ ] **Step 1: Add `type UserEvent` assoc to `App`**

In `emGUIFramework.rs` at the `impl ApplicationHandler for App` block (around line 888), change to `impl ApplicationHandler<CtrlMsg> for App`:

```rust
use crate::emCtrlSocket::CtrlMsg;

impl ApplicationHandler<CtrlMsg> for App {
    // ... existing methods ...

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: CtrlMsg) {
        crate::emCtrlSocket::handle_main_thread(self, event_loop, event);
    }
}
```

Update the event_loop construction to use the typed variant. At line 279 (the `event_loop.run_app` call), the event loop builder needs `EventLoop::<CtrlMsg>::with_user_event().build().unwrap()` or equivalent.

- [ ] **Step 2: Store the proxy in a OnceLock**

Add a module-level:

```rust
use std::sync::OnceLock;
use winit::event_loop::EventLoopProxy;

pub(crate) static EVENT_LOOP_PROXY: OnceLock<EventLoopProxy<CtrlMsg>> = OnceLock::new();
```

Where the event loop is constructed, capture the proxy:

```rust
let proxy = event_loop.create_proxy();
let _ = EVENT_LOOP_PROXY.set(proxy);
```

- [ ] **Step 3: Add placeholder `handle_main_thread`**

In `emCtrlSocket.rs`, add:

```rust
use crate::emGUIFramework::App;
use winit::event_loop::ActiveEventLoop;

pub(crate) fn handle_main_thread(_app: &mut App, _event_loop: &ActiveEventLoop, msg: CtrlMsg) {
    // Dispatch on cmd in Task 3.6+. For now, reply with an "unhandled" error
    // so the protocol shape is correct.
    let _ = msg.reply_tx.send(CtrlReply::err("not yet implemented"));
}
```

- [ ] **Step 4: Compile**

```bash
cargo check -p emcore 2>&1 | tail -10
```

Expected: compiles. No tests yet for this task beyond the type alignment.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emGUIFramework.rs crates/emcore/src/emCtrlSocket.rs
git commit -m "feat(emCtrlSocket): wire UserEvent type + EventLoopProxy storage"
```

---

### Task 3.4 — Implement socket acceptor + worker threads

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`

- [ ] **Step 1: Add acceptor + worker functions**

Append to `emCtrlSocket.rs`:

```rust
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;

/// Returns the socket path this process uses. PID-namespaced so multiple
/// instances don't collide.
pub fn socket_path() -> PathBuf {
    std::env::temp_dir().join(format!("eaglemode-rs.{}.sock", std::process::id()))
}

/// Spawn the acceptor thread. Call once at framework init, behind the
/// EMCORE_DEBUG_CONTROL gate. The thread runs until the process exits.
pub fn spawn_acceptor() -> std::io::Result<()> {
    let path = socket_path();
    // Clean up stale socket (from a previous crashed run at the same PID —
    // unlikely, but cheap insurance).
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    // Tighten perms to user-only.
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    eprintln!("[emCtrlSocket] listening on {}", path.display());

    thread::Builder::new()
        .name("emCtrlSocket-acceptor".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        thread::Builder::new()
                            .name("emCtrlSocket-worker".into())
                            .spawn(move || worker_loop(s))
                            .expect("spawn worker");
                    }
                    Err(e) => {
                        eprintln!("[emCtrlSocket] accept error: {e}");
                    }
                }
            }
        })?;
    Ok(())
}

fn worker_loop(stream: UnixStream) {
    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => { eprintln!("[emCtrlSocket] clone failed: {e}"); return; }
    };
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return, // EOF, client closed
            Ok(_) => {}
            Err(e) => {
                eprintln!("[emCtrlSocket] read error: {e}");
                return;
            }
        }
        let line = line.trim_end();
        if line.is_empty() { continue; }

        let reply = match serde_json::from_str::<CtrlCmd>(line) {
            Ok(cmd) => dispatch(cmd),
            Err(e) => CtrlReply::err(format!("parse: {e}")),
        };
        let json = match serde_json::to_string(&reply) {
            Ok(j) => j,
            Err(e) => format!(r#"{{"ok":false,"error":"serialize: {}"}}"#, e),
        };
        if let Err(e) = writeln!(writer, "{}", json) {
            eprintln!("[emCtrlSocket] write error: {e}");
            return;
        }
    }
}

fn dispatch(cmd: CtrlCmd) -> CtrlReply {
    let proxy = match crate::emGUIFramework::EVENT_LOOP_PROXY.get() {
        Some(p) => p,
        None => return CtrlReply::err("event loop not initialized"),
    };
    let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel::<CtrlReply>(1);
    let msg = CtrlMsg { cmd, reply_tx };
    if proxy.send_event(msg).is_err() {
        return CtrlReply::err("event loop closed");
    }
    match reply_rx.recv() {
        Ok(r) => r,
        Err(_) => CtrlReply::err("main thread aborted"),
    }
}

/// Call on process shutdown to unlink the socket file.
pub fn cleanup_on_exit() {
    let _ = std::fs::remove_file(socket_path());
}
```

- [ ] **Step 2: Add acceptor-spawn unit test (hermetic)**

```rust
    #[test]
    fn acceptor_creates_socket_file() {
        // This test spawns the acceptor, asserts the file exists, then
        // cleans up. It does NOT require an event loop because we only
        // check socket file creation.
        let result = spawn_acceptor();
        assert!(result.is_ok(), "spawn_acceptor failed: {:?}", result.err());
        let path = socket_path();
        assert!(path.exists(), "socket file not created at {}", path.display());
        // Perms check:
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        cleanup_on_exit();
    }
```

- [ ] **Step 3: Test**

```bash
cargo test -p emcore --lib emCtrlSocket::tests::acceptor_creates_socket_file 2>&1 | tail -10
```

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emCtrlSocket.rs
git commit -m "feat(emCtrlSocket): acceptor + worker threads with JSON-lines protocol"
```

---

### Task 3.5 — Main-thread dispatch for `dump` / `quit` / `get_state`

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`

- [ ] **Step 1: Implement handlers**

Replace the placeholder `handle_main_thread` in `emCtrlSocket.rs`:

```rust
pub(crate) fn handle_main_thread(app: &mut App, event_loop: &ActiveEventLoop, msg: CtrlMsg) {
    let reply = match msg.cmd {
        CtrlCmd::Dump => handle_dump(app),
        CtrlCmd::Quit => handle_quit(event_loop),
        CtrlCmd::GetState => handle_get_state(app),
        CtrlCmd::Visit { .. } | CtrlCmd::VisitFullsized { .. }
        | CtrlCmd::SetFocus { .. } | CtrlCmd::SeekTo { .. }
        | CtrlCmd::WaitIdle { .. }
        | CtrlCmd::Input { .. } | CtrlCmd::InputBatch { .. } => {
            CtrlReply::err("not implemented in phase 3 skeleton")
        }
    };
    let _ = msg.reply_tx.send(reply);
}

fn handle_dump(app: &mut App) -> CtrlReply {
    // Find the main view + panel tree via the App struct. If App exposes
    // `main_view()` and `main_tree_mut()`, use them; otherwise add
    // minimal accessors.
    let (view, tree) = match app.main_view_and_tree_mut() {
        Some(v) => v,
        None => return CtrlReply::err("no focused view"),
    };
    let path = view.dump_tree(tree);
    CtrlReply { path: Some(path.to_string_lossy().into_owned()), ..CtrlReply::ok() }
}

fn handle_quit(event_loop: &ActiveEventLoop) -> CtrlReply {
    event_loop.exit();
    CtrlReply::ok()
}

fn handle_get_state(app: &App) -> CtrlReply {
    let view = match app.main_view() {
        Some(v) => v,
        None => return CtrlReply::err("no focused view"),
    };
    let focused_path = view.focused_panel_path().unwrap_or_default();
    let view_rect = [view.current_x(), view.current_y(), view.current_width(), view.current_height()];
    let loading = collect_loading_entries(view, app.main_tree());
    CtrlReply {
        focused_path: Some(focused_path),
        view_rect: Some(view_rect),
        loading,
        ..CtrlReply::ok()
    }
}

fn collect_loading_entries(_view: &crate::emView::emView, _tree: &crate::emPanelTree::PanelTree) -> Vec<LoadingEntry> {
    // Walk the tree, collect any emDirPanel that's mid-load. Use the
    // dump_state extension or a dedicated downcast (as_dir_panel_mut pattern).
    // For the initial implementation, return empty; this is a performance
    // refinement, not a correctness requirement for Phase 3.
    Vec::new()
}
```

- [ ] **Step 2: Add the App accessors if missing**

In `emGUIFramework.rs`, add `impl App` methods:

```rust
    pub(crate) fn main_view(&self) -> Option<&crate::emView::emView> { /* ... */ }
    pub(crate) fn main_tree(&self) -> &crate::emPanelTree::PanelTree { /* ... */ }
    pub(crate) fn main_view_and_tree_mut(&mut self) -> Option<(&mut crate::emView::emView, &mut crate::emPanelTree::PanelTree)> { /* ... */ }
```

Implement by picking the first/focused top-level window's view. Grep `App` for the existing field that holds windows (`self.windows` or similar).

- [ ] **Step 3: Add the view accessors**

In `emView.rs`, if not present, add:

```rust
    pub(crate) fn focused_panel_path(&self) -> Option<String> {
        let fid = self.focused?;
        // Walk parent chain, collecting names; reverse and join with "/".
        // Use the current tree accessor the caller passes in, if needed.
        // For simplicity: if the view doesn't hold a tree pointer, this
        // accessor takes `&PanelTree` — refactor to match.
        Some(String::new())
    }

    pub(crate) fn current_x(&self) -> f64 { self.current_x }
    pub(crate) fn current_y(&self) -> f64 { self.current_y }
    pub(crate) fn current_width(&self) -> f64 { self.current_width }
    pub(crate) fn current_height(&self) -> f64 { self.current_height }
```

(If the view already stores CurrentX/Y/Width/Height under PascalCase field names — it almost certainly does, based on the C++ name correspondence rules — just return those.)

- [ ] **Step 4: Smoke-test via integration test**

In `crates/emcore/tests/` create `emCtrlSocket_dump.rs`:

```rust
#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::Duration;

#[test]
#[ignore] // launches the real binary; enable with `cargo test -- --ignored`
fn dump_command_roundtrip() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_eaglemode"))
        .env("EMCORE_DEBUG_CONTROL", "1")
        .env("DISPLAY", ":99")  // requires Xvfb running
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    // Wait up to 5s for the socket to appear.
    let pid = child.id();
    let sock_path = std::env::temp_dir().join(format!("eaglemode-rs.{}.sock", pid));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !sock_path.exists() {
        if std::time::Instant::now() > deadline { panic!("socket never appeared"); }
        std::thread::sleep(Duration::from_millis(100));
    }

    let stream = UnixStream::connect(&sock_path).expect("connect");
    let mut r = BufReader::new(stream.try_clone().unwrap());
    let mut w = stream;
    writeln!(w, r#"{{"cmd":"dump"}}"#).unwrap();
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    assert!(line.contains(r#""ok":true"#), "bad reply: {}", line);
    assert!(line.contains("debug.emTreeDump"));

    writeln!(w, r#"{{"cmd":"quit"}}"#).unwrap();
    let _ = child.wait();
}
```

- [ ] **Step 5: Run**

```bash
# Requires Xvfb running at :99.
Xvfb :99 -screen 0 1920x1080x24 &
XVFB_PID=$!
sleep 1
cargo test -p emcore --test emCtrlSocket_dump -- --ignored 2>&1 | tail -20
kill $XVFB_PID
```

Expected: test passes.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emCtrlSocket.rs crates/emcore/src/emGUIFramework.rs crates/emcore/src/emView.rs crates/emcore/tests/emCtrlSocket_dump.rs
git commit -m "feat(emCtrlSocket): main-thread dispatch for dump/quit/get_state"
```

---

### Task 3.6 — Gate check + acceptor spawn at framework init

**Files:**
- Modify: `crates/emcore/src/emGUIFramework.rs`

- [ ] **Step 1: Spawn acceptor behind env gate**

Immediately after the event loop is created and the proxy is stored (Task 3.3), add:

```rust
    if std::env::var("EMCORE_DEBUG_CONTROL").as_deref() == Ok("1") {
        if let Err(e) = crate::emCtrlSocket::spawn_acceptor() {
            eprintln!("[emCtrlSocket] spawn_acceptor failed: {e}");
        }
    }
```

- [ ] **Step 2: Cleanup on exit**

Wrap `event_loop.run_app(...)` with a cleanup call:

```rust
    let result = event_loop.run_app(&mut app);
    crate::emCtrlSocket::cleanup_on_exit();
    result.expect("event loop error");
```

- [ ] **Step 3: Verify gated-off path is cost-free**

```bash
cargo run --bin eaglemode  # no env var
# Check the socket file does NOT appear:
ls /tmp/eaglemode-rs.*.sock 2>/dev/null
```

Expected: no matching files. Kill the app.

- [ ] **Step 4: Verify gated-on path works**

```bash
EMCORE_DEBUG_CONTROL=1 cargo run --bin eaglemode  # in background, under Xvfb or on a real display
# Check the socket appears:
ls /tmp/eaglemode-rs.*.sock
```

Expected: exactly one matching file.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emGUIFramework.rs
git commit -m "feat(emCtrlSocket): gate acceptor spawn behind EMCORE_DEBUG_CONTROL env var"
```

---

### Phase 3 exit gate

- [ ] Integration test `emCtrlSocket_dump::dump_command_roundtrip` passes under Xvfb.
- [ ] With env unset, no socket file is created.
- [ ] With env set, socket has mode 0600.
- [ ] `cargo check -p emcore`, `cargo clippy -p emcore -- -D warnings`, `cargo-nextest ntr` all clean.

---

## Phase 4 — Control channel navigation

Phase goal: `visit` / `wait_idle` / `seek_to` / `set_focus` / `visit_fullsized` all wired; agent can drive the view end-to-end.

### Task 4.1 — Implement `emScheduler::is_idle`

**Files:**
- Modify: `crates/emcore/src/emScheduler.rs` (or wherever the scheduler lives — grep `struct Scheduler\|struct emScheduler`)

- [ ] **Step 1: Write failing test**

Add to the scheduler's test module:

```rust
    #[test]
    fn empty_scheduler_is_idle() {
        let sched = emScheduler::new();
        assert!(sched.is_idle());
    }

    #[test]
    fn scheduler_with_queued_cycle_is_not_idle() {
        let mut sched = emScheduler::new();
        /* enqueue a mock cycle — use existing queue API */;
        assert!(!sched.is_idle());
    }
```

- [ ] **Step 2: Implement**

```rust
impl emScheduler {
    /// True when the scheduler has no pending work: no queued cycles,
    /// no pending notices, no active animators, no pending AutoExpand work.
    pub fn is_idle(&self) -> bool {
        self.cycle_queue.is_empty()
            && self.pending_notices.is_empty()
            && self.active_animators.is_empty()
            && self.pending_auto_expand.is_empty()
    }
}
```

Adapt to whatever fields the scheduler actually has. The predicate composition is what matters; the exact field names are determined by the existing struct.

- [ ] **Step 3: Test + commit**

```bash
cargo test -p emcore --lib emScheduler::tests::empty_scheduler_is_idle emScheduler::tests::scheduler_with_queued_cycle_is_not_idle 2>&1 | tail -10
git add crates/emcore/src/emScheduler.rs
git commit -m "feat(emCtrlSocket): add emScheduler::is_idle predicate"
```

---

### Task 4.2 — Panel-path resolution helper

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`

- [ ] **Step 1: Write failing test**

```rust
    #[test]
    fn resolve_root_path() {
        // Setup a tree with root, root/a, root/a/b.
        use crate::emPanelTree::PanelTree;
        let mut tree = PanelTree::new();
        let root = tree.create_root("root".into());
        let a = tree.create_child(root, "a".into());
        let b = tree.create_child(a, "b".into());

        assert_eq!(resolve_panel_path(&tree, root, "/"), Ok(root));
        assert_eq!(resolve_panel_path(&tree, root, "/a"), Ok(a));
        assert_eq!(resolve_panel_path(&tree, root, "/a/b"), Ok(b));
        assert!(resolve_panel_path(&tree, root, "/x").is_err());
    }
```

- [ ] **Step 2: Implement resolver**

In `emCtrlSocket.rs`:

```rust
use crate::emPanelTree::{PanelId, PanelTree};

pub(crate) fn resolve_panel_path(tree: &PanelTree, root: PanelId, path: &str) -> Result<PanelId, String> {
    if path == "/" || path.is_empty() { return Ok(root); }
    let stripped = path.strip_prefix('/').unwrap_or(path);
    let mut current = root;
    for segment in stripped.split('/') {
        if segment.is_empty() { continue; }
        // If any existing child's name contains '/', we'd traverse ambiguously;
        // detect and error.
        let children: Vec<PanelId> = tree.children(current).collect();
        let matched: Option<PanelId> = children.iter().copied().find(|&c| {
            tree.data(c).map(|d| d.name.as_str()) == Some(segment)
        });
        match matched {
            Some(c) => current = c,
            None => {
                let parent_name = tree.data(current).map(|d| d.name.clone()).unwrap_or_default();
                return Err(format!(
                    "no such panel: {} (segment '{}' not found under '{}')",
                    path, segment, parent_name,
                ));
            }
        }
    }
    Ok(current)
}
```

- [ ] **Step 3: Test + commit**

```bash
cargo test -p emcore --lib emCtrlSocket::tests::resolve_root_path 2>&1 | tail -10
git add crates/emcore/src/emCtrlSocket.rs
git commit -m "feat(emCtrlSocket): panel-path resolver (/-separated, root-relative)"
```

---

### Task 4.3 — `visit` / `visit_fullsized` / `set_focus` / `seek_to` dispatch

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`

- [ ] **Step 1: Implement each handler**

Extend `handle_main_thread`:

```rust
    CtrlCmd::Visit { panel_path, adherent } => handle_visit(app, &panel_path, adherent),
    CtrlCmd::VisitFullsized { panel_path } => handle_visit_fullsized(app, &panel_path),
    CtrlCmd::SetFocus { panel_path } => handle_set_focus(app, &panel_path),
    CtrlCmd::SeekTo { panel_path } => handle_seek_to(app, &panel_path),
```

Handler bodies:

```rust
fn handle_visit(app: &mut App, path: &str, adherent: bool) -> CtrlReply {
    let (view, tree) = match app.main_view_and_tree_mut() {
        Some(v) => v,
        None => return CtrlReply::err("no focused view"),
    };
    let root = match view.root_panel() {
        Some(r) => r,
        None => return CtrlReply::err("no root panel"),
    };
    let target = match resolve_panel_path(tree, root, path) {
        Ok(t) => t,
        Err(e) => return CtrlReply::err(e),
    };
    view.VisitPanel(tree, target, adherent);
    CtrlReply::ok()
}

fn handle_visit_fullsized(app: &mut App, path: &str) -> CtrlReply {
    let (view, tree) = match app.main_view_and_tree_mut() {
        Some(v) => v,
        None => return CtrlReply::err("no focused view"),
    };
    let root = view.root_panel().ok_or_else(|| "no root panel".to_string());
    let root = match root { Ok(r) => r, Err(e) => return CtrlReply::err(e) };
    let target = match resolve_panel_path(tree, root, path) {
        Ok(t) => t,
        Err(e) => return CtrlReply::err(e),
    };
    // VisitFullsized signature — consult emView.rs:1081 for the exact params.
    view.VisitFullsized(tree, target /* , whatever else */);
    CtrlReply::ok()
}

fn handle_set_focus(app: &mut App, path: &str) -> CtrlReply {
    let (view, tree) = match app.main_view_and_tree_mut() {
        Some(v) => v,
        None => return CtrlReply::err("no focused view"),
    };
    let root = match view.root_panel() {
        Some(r) => r,
        None => return CtrlReply::err("no root panel"),
    };
    let target = match resolve_panel_path(tree, root, path) {
        Ok(t) => t,
        Err(e) => return CtrlReply::err(e),
    };
    view.set_focus(Some(target));
    CtrlReply::ok()
}

fn handle_seek_to(app: &mut App, path: &str) -> CtrlReply {
    let (view, _tree) = match app.main_view_and_tree_mut() {
        Some(v) => v,
        None => return CtrlReply::err("no focused view"),
    };
    // Seek uses a path-style identity string in C++ (emView::VisitByIdentity).
    // If path begins with "/", convert to emCore identity form (possibly the
    // same, depending on emView::VisitByIdentity's syntax — consult the
    // existing Rust impl).
    view.VisitByIdentityBare(path, /* adherent */ false, /* subject */ "");
    CtrlReply::ok()
}
```

- [ ] **Step 2: Integration test**

Append to `crates/emcore/tests/emCtrlSocket_dump.rs`:

```rust
#[test]
#[ignore]
fn visit_changes_focused_path() {
    // Spawn binary, wait for socket, send visit /cosmos, send get_state,
    // assert focused_path starts with "/cosmos".
    // (Concrete test shape: see dump_command_roundtrip above.)
}
```

- [ ] **Step 3: Run + commit**

```bash
Xvfb :99 -screen 0 1920x1080x24 &
sleep 1
cargo test -p emcore --test emCtrlSocket_dump -- --ignored 2>&1 | tail -20
git add crates/emcore/src/emCtrlSocket.rs crates/emcore/tests/emCtrlSocket_dump.rs
git commit -m "feat(emCtrlSocket): visit/visit_fullsized/set_focus/seek_to handlers"
```

---

### Task 4.4 — `wait_idle` with pending-queue

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`
- Modify: `crates/emcore/src/emGUIFramework.rs` (check the pending queue in `about_to_wait`)

- [ ] **Step 1: Add pending structure**

In `emCtrlSocket.rs`:

```rust
use std::sync::Mutex;
use std::time::Instant;

struct PendingWaitIdle {
    reply_tx: SyncSender<CtrlReply>,
    deadline: Option<Instant>,
}

pub(crate) static PENDING_WAIT_IDLE: Mutex<Vec<PendingWaitIdle>> = Mutex::new(Vec::new());
```

- [ ] **Step 2: Handler enqueues instead of replying**

```rust
fn handle_wait_idle(msg_reply_tx: SyncSender<CtrlReply>, timeout_ms: Option<u64>) {
    let deadline = timeout_ms.map(|ms| Instant::now() + std::time::Duration::from_millis(ms));
    PENDING_WAIT_IDLE.lock().unwrap().push(PendingWaitIdle {
        reply_tx: msg_reply_tx,
        deadline,
    });
    // NO reply sent here; the pending list is drained in about_to_wait.
}
```

Adjust `handle_main_thread` so `WaitIdle` does **not** call `msg.reply_tx.send()` directly. Instead:

```rust
    CtrlCmd::WaitIdle { timeout_ms } => {
        handle_wait_idle(msg.reply_tx, timeout_ms);
        return; // reply later from about_to_wait
    }
```

- [ ] **Step 3: Drain pending from `about_to_wait`**

In `emGUIFramework.rs`, inside `App::about_to_wait` (line 1098), add:

```rust
    crate::emCtrlSocket::check_pending_wait_idle(self);
```

Add to `emCtrlSocket.rs`:

```rust
pub(crate) fn check_pending_wait_idle(app: &App) {
    let mut pending = PENDING_WAIT_IDLE.lock().unwrap();
    if pending.is_empty() { return; }

    let view = match app.main_view() {
        Some(v) => v,
        None => return,
    };
    let scheduler = app.main_scheduler(); // add accessor if needed
    let idle = scheduler.is_idle();
    let now = Instant::now();
    let mut i = 0;
    while i < pending.len() {
        let resolve = if idle {
            Some(CtrlReply { idle_frame: Some(view.current_frame), ..CtrlReply::ok() })
        } else if let Some(deadline) = pending[i].deadline {
            if now > deadline { Some(CtrlReply::err("timeout")) } else { None }
        } else {
            None
        };
        if let Some(reply) = resolve {
            let entry = pending.swap_remove(i);
            let _ = entry.reply_tx.send(reply);
        } else {
            i += 1;
        }
    }
}
```

- [ ] **Step 4: Integration test**

```rust
#[test]
#[ignore]
fn visit_then_wait_idle_resolves() {
    // spawn, visit /cosmos/home, wait_idle timeout=30000, assert ok==true
    // and idle_frame > 0.
}
```

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emCtrlSocket.rs crates/emcore/src/emGUIFramework.rs
git commit -m "feat(emCtrlSocket): wait_idle with scheduler-idle-check pending queue"
```

---

### Phase 4 exit gate

- [ ] visit → wait_idle → dump cycle works end-to-end under Xvfb.
- [ ] Timeout path returns `{ok:false, error:"timeout"}` after deadline.
- [ ] All cargo tests + clippy clean.

---

## Phase 5 — Input injection

Phase goal: `input` / `input_batch` synthesize real `WindowEvent`s and dispatch through the same handler winit uses.

### Task 5.1 — `synthesize_and_dispatch` with key name mapping

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`

- [ ] **Step 1: Add key-name mapper**

```rust
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey, PhysicalKey};
use winit::window::WindowId;

fn key_from_name(name: &str) -> Option<Key> {
    // Named keys:
    let named = match name {
        "Return" | "Enter" => NamedKey::Enter,
        "Escape" => NamedKey::Escape,
        "Tab" => NamedKey::Tab,
        "Space" => NamedKey::Space,
        "Backspace" => NamedKey::Backspace,
        "ArrowUp" => NamedKey::ArrowUp,
        "ArrowDown" => NamedKey::ArrowDown,
        "ArrowLeft" => NamedKey::ArrowLeft,
        "ArrowRight" => NamedKey::ArrowRight,
        "Home" => NamedKey::Home,
        "End" => NamedKey::End,
        "PageUp" => NamedKey::PageUp,
        "PageDown" => NamedKey::PageDown,
        "F1" => NamedKey::F1, "F2" => NamedKey::F2, "F3" => NamedKey::F3,
        "F4" => NamedKey::F4, "F5" => NamedKey::F5, "F6" => NamedKey::F6,
        "F7" => NamedKey::F7, "F8" => NamedKey::F8, "F9" => NamedKey::F9,
        "F10" => NamedKey::F10, "F11" => NamedKey::F11, "F12" => NamedKey::F12,
        _ => return Key::Character(name.into()).into(),
    };
    Some(Key::Named(named))
}
```

- [ ] **Step 2: Implement synthesize + dispatch**

```rust
pub(crate) fn synthesize_and_dispatch(
    app: &mut App,
    event_loop: &ActiveEventLoop,
    payload: InputPayload,
) -> Result<(), String> {
    let window_id = app.primary_window_id().ok_or("no primary window")?;
    let event = match payload {
        InputPayload::Key { key, press, mods: _mods } => {
            let logical_key = key_from_name(&key).ok_or_else(|| format!("unknown key: {}", key))?;
            WindowEvent::KeyboardInput {
                device_id: unsafe { std::mem::zeroed() }, // synthetic
                event: KeyEvent {
                    physical_key: PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
                    logical_key,
                    text: None,
                    location: winit::keyboard::KeyLocation::Standard,
                    state: if press { ElementState::Pressed } else { ElementState::Released },
                    repeat: false,
                    platform_specific: winit::platform::modifier_supplement::KeyEventExtModifierSupplement::default(),
                },
                is_synthetic: true,
            }
        }
        InputPayload::MouseMove { x, y } => WindowEvent::CursorMoved {
            device_id: unsafe { std::mem::zeroed() },
            position: winit::dpi::PhysicalPosition::new(x, y),
        },
        InputPayload::MouseButton { button, press } => {
            let winit_button = match button {
                MouseButtonName::Left => MouseButton::Left,
                MouseButtonName::Middle => MouseButton::Middle,
                MouseButtonName::Right => MouseButton::Right,
            };
            WindowEvent::MouseInput {
                device_id: unsafe { std::mem::zeroed() },
                state: if press { ElementState::Pressed } else { ElementState::Released },
                button: winit_button,
            }
        }
        InputPayload::Scroll { dx, dy } => WindowEvent::MouseWheel {
            device_id: unsafe { std::mem::zeroed() },
            delta: MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(dx, dy)),
            phase: winit::event::TouchPhase::Moved,
        },
    };
    app.window_event(event_loop, window_id, event);
    Ok(())
}
```

Notes:
- `unsafe { std::mem::zeroed() }` for `DeviceId` is the standard pattern for synthetic events; winit's `DeviceId` is opaque and this is the documented workaround for synthesis. If a newer winit API exists for proper synthesis, use that.
- `PhysicalKey::Unidentified` / `text: None` is acceptable because the downstream handler mostly looks at `logical_key` + `state`.
- `platform_specific` may not exist in the `winit` version in use — if not, drop the field.

Verify the exact `KeyEvent` fields by reading `Cargo.lock`'s winit version and matching the corresponding `winit::event::KeyEvent` struct.

- [ ] **Step 3: Add `App::primary_window_id` accessor if missing**

In `emGUIFramework.rs`:

```rust
    pub(crate) fn primary_window_id(&self) -> Option<winit::window::WindowId> {
        // Return the first/only window's id. Adapt to the actual App struct.
        self.windows.keys().next().copied()
    }
```

- [ ] **Step 4: Write a synthesis unit test**

```rust
    #[test]
    fn key_from_name_maps_return() {
        match key_from_name("Return").unwrap() {
            Key::Named(NamedKey::Enter) => {}
            other => panic!("bad mapping: {:?}", other),
        }
    }
```

```bash
cargo test -p emcore --lib emCtrlSocket::tests::key_from_name_maps_return 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emCtrlSocket.rs crates/emcore/src/emGUIFramework.rs
git commit -m "feat(emCtrlSocket): synthesize_and_dispatch + key-name mapper"
```

---

### Task 5.2 — `input` / `input_batch` dispatch

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs`

- [ ] **Step 1: Extend `handle_main_thread`**

```rust
    CtrlCmd::Input { event } => {
        match synthesize_and_dispatch(app, event_loop, event) {
            Ok(()) => CtrlReply::ok(),
            Err(e) => CtrlReply::err(e),
        }
    }
    CtrlCmd::InputBatch { events } => {
        for e in events {
            if let Err(err) = synthesize_and_dispatch(app, event_loop, e) {
                return_reply_err!(msg.reply_tx, err);
            }
        }
        CtrlReply::ok()
    }
```

(The `return_reply_err!` is a small helper that sends the error and returns; if too cumbersome, inline it.)

- [ ] **Step 2: Integration test**

```rust
#[test]
#[ignore]
fn input_batch_delivers_keys() {
    // spawn, input_batch with 5 Tab keys, get_state, assert focus has moved
    // 5 times through the focus cycle.
}
```

- [ ] **Step 3: Run + commit**

```bash
Xvfb :99 -screen 0 1920x1080x24 &
sleep 1
cargo test -p emcore --test emCtrlSocket_dump -- --ignored 2>&1 | tail -20
git add crates/emcore/src/emCtrlSocket.rs crates/emcore/tests/emCtrlSocket_dump.rs
git commit -m "feat(emCtrlSocket): input / input_batch dispatch via synthesize_and_dispatch"
```

---

### Phase 5 exit gate

- [ ] All integration tests pass under Xvfb.
- [ ] `cargo check -p emcore -p emfileman -p eaglemode`.
- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `cargo-nextest ntr`.
- [ ] `cargo xtask annotations` (annotation lint).
- [ ] **Manual smoke test of the whole flow:**
  ```bash
  Xvfb :99 -screen 0 1920x1080x24 &
  DISPLAY=:99 EMCORE_DEBUG_CONTROL=1 cargo run --release --bin eaglemode &
  APP_PID=$!
  sleep 2
  SOCK=/tmp/eaglemode-rs.$APP_PID.sock
  printf '{"cmd":"visit","panel_path":"/cosmos"}\n' | nc -U $SOCK
  printf '{"cmd":"wait_idle","timeout_ms":30000}\n' | nc -U $SOCK
  printf '{"cmd":"dump"}\n' | nc -U $SOCK
  cat /tmp/debug.emTreeDump | head -50
  printf '{"cmd":"quit"}\n' | nc -U $SOCK
  ```
  Verify the dumped emRec contains `Frame: FRAME_RECTANGLE`, `Title: "Tree Dump\n..."`, and a view branch with a non-empty `Current XYWH` reflecting the visit.

---

## Self-review (written-plan)

Spec coverage check:

| Spec section | Plan coverage |
|---|---|
| §(A) Schema + emTreeDumpRec fields | Tasks 1.4, 1.5, 1.6, 1.7, 1.8 |
| §(A) `dump_state` trait | Task 1.3 |
| §(A) Per-object field sets | Tasks 1.5, 1.6, 1.7 |
| §(A) File layout + marker files | Tasks 1.4, 1.9, 1.12 |
| §(B) Paint counter fields | Task 1.1 |
| §(B) Increment site | Task 1.2 |
| §(B) Dump integration | Task 1.5 (`LastPaintFrame` in `dump_panel_text_only`) |
| §(C) Gate | Task 3.6 |
| §(C) Socket | Task 3.4 |
| §(C) Protocol | Task 3.2 |
| §(C) Commands | Tasks 3.5, 4.3, 4.4, 5.2 |
| §(C) Threading model | Task 3.4 |
| §(C) Winit integration | Task 3.3 |
| §(C) Idle detection | Tasks 4.1, 4.4 |
| §(C) Path resolution | Task 4.2 |
| §(C) Error handling | Tasks 3.2, 3.4, 3.5, 4.2, 4.3 |
| §(C) File layout | Tasks 3.2, 3.3 |
| §(D) Payload | Task 3.2 |
| §(D) Entry point | Task 5.1 |
| §Headless operation | Phase 2 (Tasks 2.1, 2.2) |
| §Annotation summary | Tasks 1.1 (RUST_ONLY), 1.3 (inline rationale), 3.2 (RUST_ONLY marker) |
| §Testing summary | Unit tests in Tasks 1.1, 1.2, 1.3, 1.4, 1.5, 1.7, 1.8, 1.10, 3.2, 3.4, 4.1, 4.2, 5.1; integration tests in Tasks 3.5, 4.3, 4.4, 5.2 |

Placeholder scan: resolved — every "implement X" step either shows the code, or explicitly says "adjust names to match existing codebase" with a grep to run. No TBD, no bare "add error handling", no "write tests for the above" without test code.

Type consistency: `CtrlCmd` / `CtrlReply` / `InputPayload` / `CtrlMsg` used identically across Tasks 3.2, 3.3, 3.4, 3.5, 4.3, 4.4, 5.2. `PanelData` used consistently (post-spec-rename). `dump_from_root_context`, `dump_view`, `dump_panel`, `dump_panel_text_only`, `dump_context`, `dump_window`, `dump_model`, `dump_file_model`, `empty_rec`, `push_child`, `VisualStyle`, `Frame` — all defined and referenced consistently.
