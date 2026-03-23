# Phase 9b: Fix 8 Visual Bugs in TestPanel Standalone

Diagnosis is in `state/run_003/visual_bug_diagnosis.json`. This prompt implements the fixes in dependency order with gates between phases.

## Rules

- Run `cargo check -p zuicchini --examples` after every file save. Stop and fix before moving on.
- Run `cargo clippy -p zuicchini --examples -- -D warnings` at each phase gate.
- Run `cargo-nextest ntr -p zuicchini` at each phase gate. All existing tests must pass.
- Do not add `#[allow(...)]` or `#[expect(...)]` — fix warnings properly.
- Do not modify golden test expectations. If a fix changes golden output, stop and report.
- Follow CLAUDE.md code rules (especially: `f64` for logical coords, `pub(crate)` default, no `Arc`/`Mutex`).
- Each phase = one commit. Commit message format: `fix(CAP-0082): <what changed>`.

---

## Phase 1: Coordinate fix — highlight rect height (Bug 8)

**One-line fix, highest confidence (0.98).**

### What

In `src/panel/view.rs` function `paint_highlight`, lines 1946 and 1948 multiply the substance rect's Y and height by `panel.viewed_height`. Eagle Mode's coordinate system uses width-relative units for ALL axes. The C++ code (`emView.cpp:2180-2183`) multiplies all substance rect components by `vw` (ViewedWidth).

### Fix

In `src/panel/view.rs`, change:

```rust
// BEFORE (wrong):
let hy = panel.viewed_y + sy * panel.viewed_height;
let hw = sw * panel.viewed_width;
let hh = sh * panel.viewed_height;

// AFTER (correct):
let hy = panel.viewed_y + sy * panel.viewed_width;
let hw = sw * panel.viewed_width;
let hh = sh * panel.viewed_width;
```

That is: lines 1946 and 1948, change `viewed_height` → `viewed_width`. Line 1947 is already correct.

### Tests

Add a unit test in `src/panel/view.rs::tests` that exercises the fix with a **non-square tallness** (the existing `setup_tree` uses tallness 1.0, which makes `viewed_width == viewed_height` and would not catch the bug):

```rust
#[test]
fn test_highlight_rect_uses_viewed_width_for_y() {
    // Create a panel with non-square tallness so viewed_height != viewed_width.
    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.get_mut(root).unwrap().focusable = true;
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 0.5); // tallness = 0.5

    let mut view = View::new(root, 800.0, 600.0);
    view.set_active_panel(&mut tree, root, false);
    view.update_viewing(&mut tree);

    let panel = tree.get(root).unwrap();
    assert!(panel.viewed);
    // The key invariant: viewed_height != viewed_width for non-square panels
    assert!((panel.viewed_height - panel.viewed_width).abs() > 1.0);

    // Substance rect components are in width-relative units.
    // When converting to viewport coords, Y and H must multiply by
    // viewed_width, not viewed_height.
    let (sx, sy, sw, sh, _sr) = tree.get_substance_rect(root);
    let correct_hy = panel.viewed_y + sy * panel.viewed_width;
    let correct_hh = sh * panel.viewed_width;
    let wrong_hh = sh * panel.viewed_height;
    // With tallness=0.5, wrong_hh would be half of correct_hh
    assert!((correct_hh - wrong_hh).abs() > 1.0,
        "Test setup: tallness must make viewed_width != viewed_height");
    // Verify the correct formula is what paint_highlight would use
    assert!(correct_hy.is_finite());
    assert!(correct_hh > 0.0);
}
```

This tests the invariant that `paint_highlight` relies on. The test will fail if someone regresses the fix back to `viewed_height`.

### Verification

Visual: the selection highlight should now cover the full panel area instead of a thin top sliver. This is visible in `rust-test-panel2.png` — the white rect around "Toolkit Test" should extend to the panel's full height.

### Gate

- `cargo check -p zuicchini --examples`
- `cargo clippy -p zuicchini --examples -- -D warnings`
- `cargo-nextest ntr -p zuicchini`

---

## Phase 2: View zoom system (Bugs 6 + 2)

**These share the ROOT_SAME_TALLNESS / zoom-out interaction. Fix together.**

### Bug 6: `raw_zoom_out` hardcodes `rel_a = 1.0`

The C++ `RawZoomOut` (`emView.cpp:1811-1825`) computes `rel_a` as:

```cpp
relA  = HomeWidth * RootPanel->GetHeight() / HomePixelTallness / HomeHeight;
relA2 = HomeHeight / RootPanel->GetHeight() * HomePixelTallness / HomeWidth;
if (relA < relA2) relA = relA2;
RawVisit(RootPanel, 0.0, 0.0, relA, forceViewingUpdate);
```

In `src/panel/view.rs`, `raw_zoom_out` (line 515) sets `rel_a = 1.0` unconditionally.

**Fix `raw_zoom_out`** to compute the fit ratio:

```rust
pub fn raw_zoom_out(&mut self, tree: &mut PanelTree) {
    let root_h = tree.get(self.root)
        .map(|p| p.layout_h)
        .unwrap_or(1.0);
    let rel_a = {
        let a1 = self.viewport_width * root_h
            / self.pixel_tallness / self.viewport_height;
        let a2 = self.viewport_height
            / root_h * self.pixel_tallness / self.viewport_width;
        a1.max(a2)
    };
    if let Some(state) = self.visit_stack.last_mut() {
        state.rel_x = 0.0;
        state.rel_y = 0.0;
        state.rel_a = rel_a;
        self.viewport_changed = true;
    }
    self.update_viewing(tree);
}
```

**Fix `is_zoomed_out`** (line 525) to match C++ `IsZoomedOut` (`emView.cpp:911-937`). The C++ checks whether the viewport covers the entire root panel, not whether `rel_a == 1.0`. Port the full C++ algorithm:

```rust
pub fn is_zoomed_out(&self, tree: &PanelTree) -> bool {
    // C++ emView::IsZoomedOut — check viewport covers entire root panel
    if self.flags.contains(ViewFlags::POPUP_ZOOM) {
        return !self.popped_up;
    }
    // Find the supreme viewed panel and walk up to root
    // checking whether the viewport rectangle fully contains
    // the root panel's bounds.
    // Simplified: recompute what raw_zoom_out would produce and compare.
    let root_h = tree.get(self.root)
        .map(|p| p.layout_h)
        .unwrap_or(1.0);
    let target_a = {
        let a1 = self.viewport_width * root_h
            / self.pixel_tallness / self.viewport_height;
        let a2 = self.viewport_height
            / root_h * self.pixel_tallness / self.viewport_width;
        a1.max(a2)
    };
    if let Some(state) = self.visit_stack.last() {
        state.rel_x.abs() < 0.001
            && state.rel_y.abs() < 0.001
            && (state.rel_a - target_a).abs() < 0.001
    } else {
        true
    }
}
```

**Fix `set_viewport`** (lines 359-374) — the `was_zoomed_out` check and re-zoom path must also use the computed fit ratio, not hardcoded `1.0`. After fixing `is_zoomed_out` and `raw_zoom_out`, change the `set_viewport` method to use them:

```rust
// In set_viewport, replace the was_zoomed_out block:
let was_zoomed_out = self.is_zoomed_out(tree);

// ... update viewport_width, viewport_height, pixel_tallness ...

if was_zoomed_out {
    self.raw_zoom_out(tree);
}
```

**Important:** `raw_zoom_out` calls `self.update_viewing(tree)`, but `set_viewport` also sets `self.viewport_changed = true`. Make sure there's no double-update. Check whether `raw_zoom_out` already sets `viewport_changed` — if so, the later assignment is harmless but redundant.

### Bug 2: `set_view_flags` missing ROOT_SAME_TALLNESS handling

The C++ `SetViewFlags` (`emView.cpp:144-150`) updates the root layout when `VF_ROOT_SAME_TALLNESS` is newly set:

```cpp
if ((viewFlags & VF_ROOT_SAME_TALLNESS) != 0 &&
    (oldFlags & VF_ROOT_SAME_TALLNESS) == 0 &&
    RootPanel) {
    RootPanel->Layout(0, 0, 1, GetHomeTallness());
}
```

In `src/panel/view.rs`, `set_view_flags` (line 662) handles `NO_ZOOM` and `POPUP_ZOOM` but not `ROOT_SAME_TALLNESS`.

**Fix:** Add the ROOT_SAME_TALLNESS block after the existing flag checks:

```rust
if new_flags.contains(ViewFlags::ROOT_SAME_TALLNESS)
    && !old.contains(ViewFlags::ROOT_SAME_TALLNESS)
{
    tree.set_layout_rect(self.root, 0.0, 0.0, 1.0, self.pixel_tallness);
    self.raw_zoom_out(tree);
}
```

Also fix `examples/test_panel.rs:1349` — instead of directly mutating `view.flags`, call `set_view_flags`:

```rust
// BEFORE:
app.windows.get_mut(&wid).unwrap().view_mut().flags |= ViewFlags::ROOT_SAME_TALLNESS;

// AFTER:
let win = app.windows.get_mut(&wid).unwrap();
let flags = win.view().flags | ViewFlags::ROOT_SAME_TALLNESS;
win.view_mut().set_view_flags(flags, &mut app.tree);
```

**Note:** `set_view_flags` takes `&mut PanelTree`, which is `app.tree`. You may need to restructure the borrow to avoid simultaneous mutable borrows of `app.windows` and `app.tree`. One approach: extract the window ID, get the current flags, drop the borrow, then call through the window. Check the existing patterns in the codebase for how other call sites handle this.

### Tests

Add unit tests in `src/panel/view.rs::tests`:

```rust
#[test]
fn test_raw_zoom_out_computes_fit_ratio() {
    // 800x600 viewport, root with tallness 0.75 (= 600/800)
    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 0.75);

    let mut view = View::new(root, 800.0, 600.0);
    view.raw_zoom_out(&mut tree);

    let state = view.current_visit();
    // C++ formula: max(W*H_root/pt/H, H/H_root*pt/W)
    //   a1 = 800 * 0.75 / 0.75 / 600 = 1.333...
    //   a2 = 600 / 0.75 * 0.75 / 800 = 0.75
    //   rel_a = max(1.333, 0.75) = 1.333...
    let expected = (800.0 * 0.75 / 0.75 / 600.0_f64)
        .max(600.0 / 0.75 * 0.75 / 800.0);
    assert!((state.rel_a - expected).abs() < 0.001,
        "rel_a should be {expected}, got {}", state.rel_a);
    assert!(state.rel_x.abs() < 0.001);
    assert!(state.rel_y.abs() < 0.001);
}

#[test]
fn test_is_zoomed_out_after_raw_zoom_out() {
    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 0.75);

    let mut view = View::new(root, 800.0, 600.0);
    view.raw_zoom_out(&mut tree);
    assert!(view.is_zoomed_out(&tree));

    // After zooming in, should not be zoomed out
    view.zoom(2.0, 400.0, 300.0);
    assert!(!view.is_zoomed_out(&tree));
}

#[test]
fn test_set_view_flags_root_same_tallness_updates_layout() {
    let mut tree = PanelTree::new();
    let root = tree.create_root("root");
    tree.set_layout_rect(root, 0.0, 0.0, 1.0, 1.0); // starts square

    let mut view = View::new(root, 800.0, 600.0);
    // pixel_tallness = 600/800 = 0.75
    let flags = view.flags | ViewFlags::ROOT_SAME_TALLNESS;
    view.set_view_flags(flags, &mut tree);

    let p = tree.get(root).unwrap();
    assert!((p.layout_h - 0.75).abs() < 0.001,
        "Root layout_h should match pixel_tallness (0.75), got {}", p.layout_h);
}
```

### Verification

Visual: at any window aspect ratio, the root TestPanel should fill the viewport (matching `Screenshot_20260316_081608.png` left side). "Test Panel" title text should appear at the same relative size as the C++ reference.

### Gate

- `cargo check -p zuicchini --examples`
- `cargo clippy -p zuicchini --examples -- -D warnings`
- `cargo-nextest ntr -p zuicchini`

---

## Phase 3: Wire VIF animations into main loop (Bug 7)

**The spring physics code exists but is never called.**

### What

`MouseZoomScrollVIF::animate_wheel()` (input_filter.rs:442) and `animate_grip()` (input_filter.rs:315) exist and are tested, but nobody calls them from the per-frame update loop.

In C++, the `SwipingViewAnimator` (WheelAnim) is an `emEngine` whose `Cycle()` is called by the scheduler each frame. In Rust, `about_to_wait` in `app.rs:252-264` ticks `active_animator` (ViewAnimator for visit/seek) but has no code for VIF animations.

### Fix

**Step 1:** Add an `animate` method to the `ViewInputFilter` trait in `src/panel/input_filter.rs`:

```rust
pub trait ViewInputFilter {
    fn filter(&mut self, event: &InputEvent, state: &InputState, view: &mut View) -> bool;
    /// Tick per-frame animations (wheel zoom spring, grip pan spring).
    /// Returns true if animation is still active and needs another frame.
    fn animate(&mut self, _view: &mut View, _tree: &mut super::tree::PanelTree, _dt: f64) -> bool {
        false // default: no animation
    }
}
```

**Step 2:** Implement `animate` for `MouseZoomScrollVIF`:

```rust
impl ViewInputFilter for MouseZoomScrollVIF {
    // ... existing filter method ...

    fn animate(&mut self, view: &mut View, tree: &mut super::tree::PanelTree, dt: f64) -> bool {
        let wheel = self.animate_wheel(view, tree, dt);
        let grip = self.animate_grip(view, tree, dt);
        wheel || grip
    }
}
```

**Step 3:** In `src/window/app.rs`, in the `about_to_wait` loop (around line 255), add VIF animation ticking. After the existing `active_animator` tick block and before `win.view_mut().update(tree)`:

```rust
// Tick VIF animations (wheel zoom spring, grip pan spring)
let vif_active = win.tick_vif_animations(tree, dt);
if vif_active {
    needs_repaint = true;
}
```

**Step 4:** Add `tick_vif_animations` to `ZuiWindow` in `src/window/zui_window.rs`:

```rust
pub fn tick_vif_animations(&mut self, tree: &mut PanelTree, dt: f64) -> bool {
    let mut active = false;
    for vif in &mut self.vif_chain {
        if vif.animate(&mut self.view, tree, dt) {
            active = true;
        }
    }
    active
}
```

**Borrow check:** `vif_chain` and `view` are both fields of `ZuiWindow`. The method takes `&mut self`, so accessing `self.vif_chain` and `self.view` simultaneously requires splitting the borrow. Use a pattern like:

```rust
pub fn tick_vif_animations(&mut self, tree: &mut PanelTree, dt: f64) -> bool {
    let view = &mut self.view;
    let mut active = false;
    for vif in &mut self.vif_chain {
        if vif.animate(view, tree, dt) {
            active = true;
        }
    }
    active
}
```

**Step 5:** When VIF animations are active, the window must request redraws. In `about_to_wait`, if `vif_active`, request a new frame (the existing repaint logic should handle this — check that `needs_repaint = true` triggers `window.request_redraw()`).

### Tests

Add a unit test in `src/panel/input_filter.rs::tests` that verifies the trait `animate()` method delegates to the spring methods:

```rust
#[test]
fn test_vif_animate_trait_delegates_wheel() {
    let (mut tree, mut view) = setup();
    let mut vif = MouseZoomScrollVIF::new();
    let state = InputState::new();

    // Feed a wheel event to activate wheel spring
    let event = InputEvent::press(InputKey::WheelUp);
    let consumed = vif.filter(&event, &state, &mut view);
    assert!(consumed);
    assert!(vif.wheel_active);

    // Call animate via the trait — should return true (animation active)
    let active = ViewInputFilter::animate(&mut vif, &mut view, &mut tree, 1.0 / 60.0);
    assert!(active, "animate() should return true when wheel is active");
}

#[test]
fn test_vif_animate_returns_false_when_idle() {
    let (mut tree, mut view) = setup();
    let mut vif = MouseZoomScrollVIF::new();

    // No events fed — animate should return false
    let active = ViewInputFilter::animate(&mut vif, &mut view, &mut tree, 1.0 / 60.0);
    assert!(!active, "animate() should return false when idle");
}
```

**Note:** `wheel_active` is a private field. If needed, check the return value of `animate()` instead of asserting the field directly. The test structure above may need adjustment based on field visibility — the key assertion is that `animate()` returns `true` after a wheel event and `false` when idle.

### Verification

Visual/behavioral: mouse wheel should zoom in/out smoothly. Middle-button drag should pan and coast. These are inherently interactive — the unit tests verify the wiring, the manual test verifies end-to-end.

### Gate

- `cargo check -p zuicchini --examples`
- `cargo clippy -p zuicchini --examples -- -D warnings`
- `cargo-nextest ntr -p zuicchini`

---

## Phase 4: Example code fixes (Bugs 1 + 5)

**These are in `examples/test_panel.rs`, not the framework.**

### Bug 1: TkTestGrpPanel children flat + unlaid-out

The C++ `TkTestGrp` extends `emRasterGroup` (automatic grid layout). `sp1`/`sp2` are children of `sp`, and `t1a`/`t1b`/`t2a`/`t2b` are children of `sp1`/`sp2` respectively.

The Rust `TkTestGrpPanel::layout_children` (test_panel.rs:912-957) creates all 7 children flat under `TkTestGrpPanel` and only lays out `sp`. The other 6 get default off-screen positions.

**Fix option A (simpler — manual grid layout, no reparenting):**

Keep the flat hierarchy but lay out all children in a 2x2 grid. In `layout_children`, after creating all children, lay them out:

```rust
fn layout_children(&mut self, ctx: &mut PanelCtx) {
    let children = ctx.children();
    let rect = ctx.layout_rect();
    let h = rect.h / rect.w;

    if !children.is_empty() {
        // Layout all children in a 2x2 grid below the title
        let body_y = 0.05 * h;
        let body_h = 0.95 * h;
        let half_w = 0.5;
        let half_h = body_h * 0.5;

        if let Some(id) = ctx.find_child_by_name("sp") {
            ctx.layout_child(id, 0.0, body_y, 1.0, body_h);
        }
        if let Some(id) = ctx.find_child_by_name("sp1") {
            ctx.layout_child(id, 0.0, body_y, half_w, body_h);
        }
        if let Some(id) = ctx.find_child_by_name("sp2") {
            ctx.layout_child(id, half_w, body_y, half_w, body_h);
        }
        if let Some(id) = ctx.find_child_by_name("t1a") {
            ctx.layout_child(id, 0.0, body_y, half_w, half_h);
        }
        if let Some(id) = ctx.find_child_by_name("t1b") {
            ctx.layout_child(id, 0.0, body_y + half_h, half_w, half_h);
        }
        if let Some(id) = ctx.find_child_by_name("t2a") {
            ctx.layout_child(id, half_w, body_y, half_w, half_h);
        }
        if let Some(id) = ctx.find_child_by_name("t2b") {
            ctx.layout_child(id, half_w, body_y + half_h, half_w, half_h);
        }
        return;
    }

    // ... existing child creation code unchanged ...
}
```

**However**, this creates overlapping panels (sp covers everything, sp1/sp2 cover halves, t1a-t2b cover quarters). The splitter panels will paint on top of the TkTest panels. You may need to NOT create `sp`, `sp1`, `sp2` at all if the splitter widget doesn't manage sub-layouts in the flat model. Check what `SplitterPanel` actually does — if it just draws a draggable divider line, keep it. If it obscures children, remove the splitter panels and just lay out the 4 `TkTestPanel`s in a 2x2 grid.

**Fix option B (C++ parity — proper hierarchy):**

Reparent children so `sp1`/`sp2` are children of `sp`, and `t1a`/`t1b` are children of `sp1`, `t2a`/`t2b` are children of `sp2`. This requires `ctx.create_child_under(parent_id, name, behavior)` or equivalent. Check if `PanelCtx` supports creating children under a specific parent. If not, use `tree.create_child(parent_id, name)` directly via `ctx.tree`.

**Choose whichever approach works with the existing API. Test visually.**

### Bug 5: MAX_DEPTH + missing threshold propagation

**Fix 1:** In `examples/test_panel.rs`, remove `const MAX_DEPTH: u32 = 2;` and the depth guard at line 820. Let auto-expansion threshold control recursion depth naturally (matching C++).

If removing MAX_DEPTH entirely causes stack overflow or excessive memory, raise it to a high value (e.g., 10) instead of removing it. The auto-expansion threshold should prevent unbounded recursion.

**Fix 2:** In `TestPanel::new()`, call `set_auto_expansion_threshold(900.0)` on the panel. Currently only the root gets this (line 1335). In C++, every `emTestPanel` constructor calls `SetAutoExpansionThreshold(900.0)` (`emTestPanel.cpp:39`).

Since `TestPanel::new()` doesn't have access to the tree, the threshold must be set after creating the child. Find where child TestPanels are created (around line 824) and add threshold setting:

```rust
// After creating each child TestPanel:
let tp_id = ctx.create_child_with(&name, Box::new(TestPanel::new(/* ... */)));
ctx.tree.set_auto_expansion_threshold(tp_id, 900.0, ViewConditionType::Area);
```

### Golden test sync (critical)

`tests/golden/test_panel.rs` contains a **separate copy** of the TestPanel code — it does NOT import from `examples/test_panel.rs`. Any changes to `TkTestGrpPanel` layout or `MAX_DEPTH`/threshold must be mirrored there, or the golden test will pass with the old broken behavior and provide no regression guard.

Specifically:
- `tests/golden/test_panel.rs:60` has its own `const MAX_DEPTH: u32 = 2;` — update or remove it to match.
- The golden test's `TkTestGrpPanel` (find it with grep) must receive the same layout fixes as the example's.
- After syncing, run: `MEASURE_DIVERGENCE=1 cargo test --test golden -- testpanel --test-threads=1`. Verify divergence does not increase. If golden test data needs regeneration, run `make -C zuicchini/tests/golden/gen && make -C zuicchini/tests/golden/gen run` and check the new baselines.

### Verification

Visual: the Toolkit Test area should show the full widget showcase (buttons, text fields, etc.) instead of a collapsed dark rectangle. All 4 recursive TestPanel tiles should show scrolling input status logs, not just 1.

### Gate

- `cargo check -p zuicchini --examples`
- `cargo clippy -p zuicchini --examples -- -D warnings`
- `cargo-nextest ntr -p zuicchini`
- `MEASURE_DIVERGENCE=1 cargo test --test golden -- testpanel --test-threads=1` (divergence must not increase)

---

## Phase 5: Port arrow highlight system (Bug 4)

**~150 lines of new code, porting 330 lines of C++.**

### What

Replace the placeholder `paint_rect_outlined` in `paint_highlight` (view.rs:1981-1989) with the C++ procedural arrow system from `emView.cpp:2149-2479`.

### C++ reference structure

The C++ system has 4 functions:

1. **`PaintHighlight`** (emView.cpp:2149-2297) — main function. Computes substance rect, sets up rounded-rect perimeter (4 straight edges + 4 quarter-circle corners), distributes arrows along the perimeter in 4 passes (one per side starting from a corner midpoint). Uses constants: `arrowSize=11.0`, `arrowDistance=55.0`, `distanceFromPanel=2.0`.

2. **`PaintHighlightArrowsOnLine`** (emView.cpp:2300-2356) — draws arrows along a straight edge. Clips to viewport, iterates along the line calling `PaintHighlightArrow`.

3. **`PaintHighlightArrowsOnBow`** (emView.cpp:2359-2430) — draws arrows along a quarter-circle corner. Clips to viewport using arc math, iterates calling `PaintHighlightArrow`.

4. **`PaintHighlightArrow`** (emView.cpp:2433-2479) — draws a single 4-vertex arrow polygon pointing toward `(goalX, goalY)` (the panel center). Each arrow has a shadow polygon (offset by `sd = arrowSize * 0.2`) painted first, then the arrow polygon on top.

### Important C++ details

- **Pixel tallness scaling:** C++ line 2172 does `pnt.SetScaling(1.0, 1.0/CurrentPixelTallness)` and line 2175 does `vy = ... * CurrentPixelTallness`. This converts from the view's pixel-tallness-aware coordinates to square-pixel coordinates for the arrow rendering. The Rust painter may not have `SetScaling` — if not, apply the pixel_tallness correction manually to Y coordinates.

- **LeaveUserSpace / EnterUserSpace:** C++ calls `pnt.LeaveUserSpace()` (line 2204) before painting arrows and `pnt.EnterUserSpace()` (line 2296) after. This switches from panel-local coordinates to screen-pixel coordinates. Check if the Rust painter has equivalent methods. If not, the arrows are already in viewport/pixel coordinates (since `paint_highlight` already works in viewport coords via `panel.viewed_x/y/width`), so this may not be needed.

- **Shadow color:** `emColor(0,0,0,192)` normally, alpha/3 when unfocused.

- **Arrow polygon vertices:** 4 vertices forming a pointed chevron shape:
  ```
  tip:   (x, y)
  right: (x + dx*ah - dy*aw*0.5, y + dy*ah + dx*aw*0.5)
  notch: (x + dx*ag, y + dy*ag)           // ag = ah * 0.8
  left:  (x + dx*ah + dy*aw*0.5, y + dy*ah - dx*aw*0.5)
  ```
  where `(dx, dy)` is the unit vector from `(x, y)` toward `(goalX, goalY)`.

### Fix

Add three private helper methods to `View` (or as free functions in `view.rs`):

```rust
fn paint_highlight_arrows_on_line(painter: &mut Painter, ...) { ... }
fn paint_highlight_arrows_on_bow(painter: &mut Painter, ...) { ... }
fn paint_highlight_arrow(painter: &mut Painter, ...) { ... }
```

Then replace the `paint_rect_outlined` block in `paint_highlight` with the C++ perimeter-walk algorithm.

**Port the C++ code faithfully.** The arrow distribution algorithm (lines 2235-2293) uses a specific pattern: it walks around the rounded rect perimeter in 8 segments (4 bows + 4 lines), distributing `n` arrows per side where `n = round(len / arrowDistance)` rounded to the nearest power-of-2-ish value.

### Anti-patterns to avoid

- Do NOT simplify the arrow distribution algorithm. The specific rounding (`n&=(m|(m>>1)|(m>>2))`) produces the correct visual density.
- Do NOT skip the shadow polygons. They provide depth.
- Do NOT use `f64` approximations for the arc math — use `f64::cos`/`f64::sin`/`f64::acos`/`f64::asin` which are exact enough for display coordinates.
- Do NOT paint in panel-local coordinates. The existing `paint_highlight` already converts to viewport coordinates. Keep arrows in viewport coordinates.

### Tests

Add unit tests in `src/panel/view.rs::tests` for the arrow geometry helpers:

```rust
#[test]
fn test_highlight_arrow_vertices() {
    // Arrow at (100, 100) pointing toward goal at (100, 50) — straight up.
    // dx=0, dy=-1 (unit vector toward goal).
    // arrowSize=11, ag=8.8, aw=5.5, sd=2.2
    //
    // Expected vertices (from C++ formula):
    //   tip:   (100, 100)
    //   right: (100 + 0*11 - (-1)*2.75, 100 + (-1)*11 + 0*2.75) = (102.75, 89)
    //   notch: (100 + 0*8.8, 100 + (-1)*8.8) = (100, 91.2)
    //   left:  (100 + 0*11 + (-1)*2.75, 100 + (-1)*11 - 0*2.75) = (97.25, 89)
    //
    // Test that paint_highlight_arrow produces these vertices.
    // (Adapt test to match actual function signature — may need to capture
    // polygon calls via a mock painter or extract vertex computation into
    // a pure function that returns the 4 vertices.)
}

#[test]
fn test_highlight_arrow_count_rounding() {
    // The C++ rounding formula: n = round(len/arrowDistance), then
    // find smallest power-of-2 m >= n, then n &= (m | m>>1 | m>>2).
    // This produces "round numbers" of arrows.
    //
    // Test cases:
    //   len=55,  dist=55 → n=1 → m=1 → 1 & (1|0|0) = 1
    //   len=110, dist=55 → n=2 → m=2 → 2 & (2|1|0) = 2
    //   len=165, dist=55 → n=3 → m=4 → 3 & (4|2|1) = 3
    //   len=220, dist=55 → n=4 → m=4 → 4 & (4|2|1) = 4
    //   len=385, dist=55 → n=7 → m=8 → 7 & (8|4|2) = 6
    //   len=440, dist=55 → n=8 → m=8 → 8 & (8|4|2) = 8
    //
    // Extract the rounding logic into a helper function and test it directly.
}
```

**Implementation note:** To make the arrow vertex computation testable, extract it as a pure function `compute_arrow_vertices(x, y, goal_x, goal_y, arrow_size) -> [(f64,f64); 4]` rather than baking it inside the paint call. Similarly, extract `compute_arrow_count(len, arrow_distance) -> usize` for the rounding formula. This makes the critical math unit-testable without needing a `Painter`.

### Verification

Visual: the selection highlight should show angled arrow/chevron polygons around the active panel's rounded-rect border, matching the C++ reference in `cpp-test-panel1.png` (pink/salmon border with arrows).

### Gate

- `cargo check -p zuicchini --examples`
- `cargo clippy -p zuicchini --examples -- -D warnings`
- `cargo-nextest ntr -p zuicchini`

---

## Phase 6: Control panel stub (Bug 3)

**Lowest priority. The widget exists but the overlay mechanism is unimplemented.**

### What

Two sub-issues:

1. **Missing caption:** `examples/test_panel.rs:832-839` creates a `ColorField` without calling `set_caption("Background Color")`. Add the caption if `ColorField` supports it. If not, this is cosmetic and can be skipped.

2. **Control panel view mechanism:** `View::create_control_panel()` at `view.rs:1853` is a TODO stub returning `None`. The C++ mechanism creates an overlay panel when a panel is "visited" that shows the panel's control panel. This is a substantial feature — **do not implement it in this phase**. Just document the stub with a comment referencing the C++ code.

### Fix

1. Check if `ColorField` has a `set_caption` or similar method. If yes, call it:
   ```rust
   cf.set_caption("Background Color");
   ```
   If no such method exists, skip this.

2. Update the TODO comment on `create_control_panel` to reference the C++ source:
   ```rust
   /// TODO: Port emView::CreateControlPanel (emView.cpp) — creates overlay
   /// panel when a panel is visited. The TestPanel overrides this to show
   /// a BgColor field (emTestPanel.cpp:511-517).
   ```

### Gate

- `cargo check -p zuicchini --examples`
- `cargo clippy -p zuicchini --examples -- -D warnings`
- `cargo-nextest ntr -p zuicchini`

---

## Final verification

After all phases, run the test panel example:

```bash
cargo run --example test_panel
```

Compare visually against the C++ reference screenshots:
- `/home/ar/Pictures/Screenshots/cpp-test-panel1.png` through `cpp-test-panel4.png`
- The side-by-side screenshot should now show matching zoom levels

Check each bug is resolved:
1. Toolkit Test shows full widget showcase (not collapsed)
2. Font size matches C++ reference
3. Background Color widget visible (or documented as TODO)
4. Selection border shows arrow/chevron pattern
5. All 4 recursive tiles show input status logs
6. Zoom level matches C++ at any window size
7. Mouse wheel zooms in/out
8. Selection highlight covers full panel area
