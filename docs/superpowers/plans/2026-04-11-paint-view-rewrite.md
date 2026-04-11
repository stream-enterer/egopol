# Paint/paint_panel_recursive Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite emView::Paint and the child-painting loop to structurally match C++ PaintView, fixing 10 divergences that cause golden test failures.

**Architecture:** The C++ Paint function uses an iterative DFS with manual clip clamping against a render region, a copied painter for child painting, and conditional background clear based on SVP opacity/coverage. Rust will match this structure using push/pop state instead of painter copy, adding missing clip/origin accessors to emPainter, and replacing the recursive paint_panel_recursive with an inline iterative DFS in Paint.

**Tech Stack:** Rust, emPainter state management, PanelTree traversal

---

### Task 1: Add Clip and Origin Accessors to emPainter

**Files:**
- Modify: `crates/emcore/src/emPainter.rs:478-480` (rename GetClipX1)
- Modify: `crates/emcore/src/emView.rs:2454` (caller of GetClipX1)

These accessors are needed by the rewritten Paint function to compute the render region (C++ lines 1066-1071: `ox=painter.GetOriginX(); rx1=painter.GetClipX1()-ox;`).

- [ ] **Step 1: Rename `GetClipX1() -> bool` to `IsClipEmpty() -> bool`**

In `crates/emcore/src/emPainter.rs`, change:
```rust
// OLD (line 478):
pub fn GetClipX1(&self) -> bool {
    self.state.clip.IsEmpty()
}

// NEW:
/// Returns true if the current clip region has zero area.
pub fn IsClipEmpty(&self) -> bool {
    self.state.clip.IsEmpty()
}
```

- [ ] **Step 2: Update the single caller of `GetClipX1() -> bool`**

In `crates/emcore/src/emView.rs` line 2454, change:
```rust
// OLD:
if painter.GetClipX1() {

// NEW:
if painter.IsClipEmpty() {
```

- [ ] **Step 3: Add C++-matching clip and origin accessors**

In `crates/emcore/src/emPainter.rs`, after the `IsClipEmpty` function, add:

```rust
/// Get clipping rectangle X1 in pixel coordinates.
/// Corresponds to C++ `emPainter::GetClipX1`.
pub fn GetClipX1(&self) -> f64 {
    self.state.clip.x1
}

/// Get clipping rectangle Y1 in pixel coordinates.
/// Corresponds to C++ `emPainter::GetClipY1`.
pub fn GetClipY1(&self) -> f64 {
    self.state.clip.y1
}

/// Get clipping rectangle X2 in pixel coordinates.
/// Corresponds to C++ `emPainter::GetClipX2`.
pub fn GetClipX2(&self) -> f64 {
    self.state.clip.x2
}

/// Get clipping rectangle Y2 in pixel coordinates.
/// Corresponds to C++ `emPainter::GetClipY2`.
pub fn GetClipY2(&self) -> f64 {
    self.state.clip.y2
}

/// Get origin X in pixel coordinates.
/// Corresponds to C++ `emPainter::GetOriginX`.
pub fn GetOriginX(&self) -> f64 {
    self.state.offset_x
}

/// Get origin Y in pixel coordinates.
/// Corresponds to C++ `emPainter::GetOriginY`.
pub fn GetOriginY(&self) -> f64 {
    self.state.offset_y
}

/// Get X scale factor.
/// Corresponds to C++ `emPainter::GetScaleX`.
pub fn GetScaleX(&self) -> f64 {
    self.state.scale_x
}

/// Get Y scale factor.
/// Corresponds to C++ `emPainter::GetScaleY`.
pub fn GetScaleY(&self) -> f64 {
    self.state.scale_y
}
```

- [ ] **Step 4: Add `SetClippingAbsolute` for pixel-coord clip setting**

C++ `SetClipping(x1,y1,x2,y2)` sets clip directly in pixel coords with no intersection. Rust's existing `SetClipping` intersects in user coords. Add the C++ equivalent:

```rust
/// Set clip rectangle directly in pixel coordinates, no intersection.
/// Matches C++ `emPainter::SetClipping(clipX1, clipY1, clipX2, clipY2)`.
pub fn SetClippingAbsolute(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
    // Record as user-space coords for DrawOp compatibility
    let ux = (x1 - self.state.offset_x) / self.state.scale_x;
    let uy = (y1 - self.state.offset_y) / self.state.scale_y;
    let uw = (x2 - x1) / self.state.scale_x;
    let uh = (y2 - y1) / self.state.scale_y;
    self.record_state(DrawOp::ClipRect { x: ux, y: uy, w: uw, h: uh });
    self.state.clip = ClipRect { x1, y1, x2, y2 };
}
```

- [ ] **Step 5: Add `ClearWithCanvas` method**

C++ `painter.Clear(ncc, canvasColor)` paints a rect over the full clip region in user coords. Add:

```rust
/// Fill the entire clip region with a color, respecting canvas color for blending.
/// Corresponds to C++ `emPainter::Clear(texture, canvasColor)`.
pub fn ClearWithCanvas(&mut self, color: emColor, canvas_color: emColor) {
    let sx = self.state.scale_x;
    let sy = self.state.scale_y;
    let ox = self.state.offset_x;
    let oy = self.state.offset_y;
    self.PaintRect(
        (self.state.clip.x1 - ox) / sx,
        (self.state.clip.y1 - oy) / sy,
        (self.state.clip.x2 - self.state.clip.x1) / sx,
        (self.state.clip.y2 - self.state.clip.y1) / sy,
        color,
        canvas_color,
    );
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo clippy -- -D warnings`
Expected: PASS (no errors, no warnings)

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr`
Expected: PASS (no regressions — only added/renamed methods)

- [ ] **Step 8: Commit**

```bash
git add crates/emcore/src/emPainter.rs crates/emcore/src/emView.rs
git commit -m "refactor: add C++-matching clip/origin accessors to emPainter"
```

---

### Task 2: Add `IsOpaque` Accessor to PanelTree

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs` (add method)

The rewritten Paint function needs to check `!p->IsOpaque()` on the SVP (C++ line 1073). Currently PanelTree has no way to query IsOpaque without taking ownership of the behavior.

- [ ] **Step 1: Add `IsOpaque` method to PanelTree**

Add after the `with_behavior_as` method (around line 1048):

```rust
/// Check if a panel's behavior reports as opaque.
/// Corresponds to C++ `emPanel::IsOpaque()`.
pub fn IsOpaque(&mut self, id: PanelId) -> bool {
    match self.take_behavior(id) {
        Some(behavior) => {
            let opaque = behavior.IsOpaque();
            self.put_behavior(id, behavior);
            opaque
        }
        None => false,
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emPanelTree.rs
git commit -m "feat: add IsOpaque accessor to PanelTree"
```

---

### Task 3: Rewrite `emView::Paint` to Match C++

**Files:**
- Modify: `crates/emcore/src/emView.rs:2225-2250` (Paint function)

This is the core rewrite. The new Paint must match C++ `emView::Paint` (lines 1048-1146) block by block.

**C++ reference** (`~/git/eaglemode-0.96.4/src/emCore/emView.cpp:1048-1146`):
```
1. Assert scale == 1.0
2. EnterUserSpace (skip — single-threaded)
3. If no SVP: Clear(BackgroundColor, canvasColor) — done
4. ox, oy = origin; rx1..ry2 = clip - origin (render region)
5. If !SVP.IsOpaque || SVP doesn't cover render region:
   ncc = SVP.CanvasColor; if !opaque: ncc = BackgroundColor
   Clear(ncc, canvasColor); canvasColor = ncc
6. Clamp SVP clip to render region
7. If clip valid: copy painter state, SetClipping, SetTransformation, SVP.Paint
8. Iterative DFS over children
9. PaintHighlight (only when SVP exists)
10. ActiveAnimator.Paint (TODO)
11. StressTest overlay
```

- [ ] **Step 1: Replace the Paint function body**

Replace the entire `pub fn Paint(&self, tree: &mut PanelTree, painter: &mut emPainter)` body in `crates/emcore/src/emView.rs` with:

```rust
pub fn Paint(&self, tree: &mut PanelTree, painter: &mut emPainter) {
    // C++ line 1056: assert scale == 1.0
    debug_assert!(
        painter.GetScaleX() == 1.0 && painter.GetScaleY() == 1.0,
        "emView::Paint: Scaling not possible."
    );

    // C++ lines 1060, 1145: EnterUserSpace/LeaveUserSpace — no-op (single-threaded)

    let svp_id = match self.svp {
        Some(id) => id,
        None => {
            // C++ line 1063: painter.Clear(BackgroundColor, canvasColor)
            // canvasColor is TRANSPARENT at the top level (first paint).
            painter.ClearWithCanvas(self.background_color, emColor::TRANSPARENT);
            // StressTest overlay
            if let Some(st) = &self.stress_test {
                st.paint_info(painter, self.viewport_width, self.viewport_height);
            }
            return;
        }
    };

    // C++ lines 1066-1071: compute render region from painter clip/origin
    let ox = painter.GetOriginX();
    let oy = painter.GetOriginY();
    let rx1 = painter.GetClipX1() - ox;
    let ry1 = painter.GetClipY1() - oy;
    let rx2 = painter.GetClipX2() - ox;
    let ry2 = painter.GetClipY2() - oy;

    let svp = match tree.GetRec(svp_id) {
        Some(p) => p,
        None => return,
    };

    // C++ lines 1073-1084: conditional background clear
    let mut canvas_color = emColor::TRANSPARENT;
    if !tree.IsOpaque(svp_id)
        || svp.viewed_x > rx1
        || svp.viewed_x + svp.viewed_width < rx2
        || svp.viewed_y > ry1
        || svp.viewed_y + svp.viewed_height < ry2
    {
        let mut ncc = svp.canvas_color;
        if !ncc.IsOpaque() {
            ncc = self.background_color;
        }
        painter.ClearWithCanvas(ncc, canvas_color);
        canvas_color = ncc;
    }

    // C++ lines 1085-1088: clamp SVP clip to render region
    // PanelData stores clip as (x, y, w, h), convert to (x1, y1, x2, y2)
    let svp_clip_x2 = svp.clip_x + svp.clip_w;
    let svp_clip_y2 = svp.clip_y + svp.clip_h;
    let mut cx1 = svp.clip_x;
    if cx1 < rx1 { cx1 = rx1; }
    let mut cx2 = svp_clip_x2;
    if cx2 > rx2 { cx2 = rx2; }
    let mut cy1 = svp.clip_y;
    if cy1 < ry1 { cy1 = ry1; }
    let mut cy2 = svp_clip_y2;
    if cy2 > ry2 { cy2 = ry2; }

    // Cache SVP fields before mutable borrow
    let svp_vx = svp.viewed_x;
    let svp_vy = svp.viewed_y;
    let svp_vw = svp.viewed_width;
    let svp_layout_rect = svp.layout_rect;

    if cx1 < cx2 && cy1 < cy2 {
        // C++ lines 1090-1098: set clip, transform, paint SVP
        painter.push_state();
        painter.SetClippingAbsolute(cx1 + ox, cy1 + oy, cx2 + ox, cy2 + oy);
        painter.SetTransformation(svp_vx + ox, svp_vy + oy, svp_vw, svp_vw);

        // Set canvas color for the SVP Paint call
        painter.SetCanvasColor(canvas_color);

        // Paint SVP behavior
        if let Some(mut behavior) = tree.take_behavior(svp_id) {
            let mut state =
                tree.build_panel_state(svp_id, self.window_focused, self.pixel_tallness);
            state.priority = tree.GetUpdatePriority(
                svp_id,
                self.viewport_width,
                self.viewport_height,
                self.window_focused,
            );
            const DEFAULT_MEMORY_LIMIT: u64 = 2_048_000_000;
            state.memory_limit = tree.GetMemoryLimit(
                svp_id,
                self.viewport_width,
                self.viewport_height,
                DEFAULT_MEMORY_LIMIT,
                self.seek_pos_panel,
            );
            let tallness = if svp_layout_rect.w > 0.0 {
                svp_layout_rect.h / svp_layout_rect.w
            } else {
                1.0
            };
            behavior.Paint(painter, 1.0, tallness, &state);
            tree.put_behavior(svp_id, behavior);
        }
        painter.pop_state();

        // C++ lines 1099-1135: iterative DFS over children
        // C++ does LeaveUserSpace before the loop, EnterUserSpace around each
        // child Paint, then EnterUserSpace after the loop. We skip the mutex
        // calls but match the painter state lifecycle: each child gets its own
        // push/pop scope with fresh clip and transform.
        if let Some(first_child) = tree.GetFirstChild(svp_id) {
            let mut p = first_child;
            loop {
                let panel = match tree.GetRec(p) {
                    Some(rec) => rec,
                    None => break,
                };

                if panel.viewed {
                    // C++ lines 1104-1108: clamp child clip to render region
                    let p_clip_x2 = panel.clip_x + panel.clip_w;
                    let p_clip_y2 = panel.clip_y + panel.clip_h;
                    let mut cx1 = panel.clip_x;
                    if cx1 < rx1 { cx1 = rx1; }
                    let mut cx2 = p_clip_x2;
                    if cx2 > rx2 { cx2 = rx2; }
                    if cx1 < cx2 {
                        let mut cy1 = panel.clip_y;
                        if cy1 < ry1 { cy1 = ry1; }
                        let mut cy2 = p_clip_y2;
                        if cy2 > ry2 { cy2 = ry2; }
                        if cy1 < cy2 {
                            // C++ lines 1110-1118: set clip, transform, paint child
                            let p_vx = panel.viewed_x;
                            let p_vy = panel.viewed_y;
                            let p_vw = panel.viewed_width;
                            let p_canvas = panel.canvas_color;
                            let p_layout = panel.layout_rect;

                            painter.push_state();
                            painter.SetClippingAbsolute(
                                cx1 + ox, cy1 + oy, cx2 + ox, cy2 + oy,
                            );
                            painter.SetTransformation(
                                p_vx + ox, p_vy + oy, p_vw, p_vw,
                            );

                            // C++ line 1118: p->Paint(pnt, p->CanvasColor)
                            painter.SetCanvasColor(p_canvas);

                            if let Some(mut behavior) = tree.take_behavior(p) {
                                let mut state = tree.build_panel_state(
                                    p,
                                    self.window_focused,
                                    self.pixel_tallness,
                                );
                                state.priority = tree.GetUpdatePriority(
                                    p,
                                    self.viewport_width,
                                    self.viewport_height,
                                    self.window_focused,
                                );
                                const DEFAULT_MEMORY_LIMIT: u64 = 2_048_000_000;
                                state.memory_limit = tree.GetMemoryLimit(
                                    p,
                                    self.viewport_width,
                                    self.viewport_height,
                                    DEFAULT_MEMORY_LIMIT,
                                    self.seek_pos_panel,
                                );
                                let tallness = if p_layout.w > 0.0 {
                                    p_layout.h / p_layout.w
                                } else {
                                    1.0
                                };
                                behavior.Paint(painter, 1.0, tallness, &state);
                                tree.put_behavior(p, behavior);
                            }
                            painter.pop_state();

                            // C++ lines 1120-1123: descend to first child
                            if let Some(fc) = tree.GetFirstChild(p) {
                                p = fc;
                                continue;
                            }
                        }
                    }
                }

                // C++ lines 1127-1134: advance to next sibling or walk up
                if let Some(next) = tree.GetNext(p) {
                    p = next;
                } else {
                    loop {
                        p = match tree.GetParentContext(p) {
                            Some(parent) => parent,
                            None => break,
                        };
                        if p == svp_id {
                            break;
                        }
                        if let Some(next) = tree.GetNext(p) {
                            p = next;
                            break;
                        }
                    }
                    if p == svp_id || tree.GetParentContext(p).is_none() {
                        break;
                    }
                }
            }
        }
    }

    // C++ line 1139: PaintHighlight — only when SVP exists
    self.paint_highlight(tree, painter);

    // C++ line 1142: ActiveAnimator paint
    // TODO: ActiveAnimator not yet implemented

    // C++ line 1143: StressTest overlay
    if let Some(st) = &self.stress_test {
        st.paint_info(painter, self.viewport_width, self.viewport_height);
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo clippy -- -D warnings`
Expected: PASS. If there are unused variable warnings from the old code that was replaced, they should be gone. If `emColor::TRANSPARENT` is not available, check the import.

- [ ] **Step 3: Run unit tests**

Run: `cargo-nextest ntr`
Expected: PASS

- [ ] **Step 4: Run golden tests**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | grep -E 'FAILED|test result'`
Expected: Improvement from 229/243 baseline. Record result.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "refactor: rewrite emView::Paint to match C++ PaintView structure"
```

---

### Task 4: Fix `paint_panel_recursive` for `paint_sub_tree` callers

**Files:**
- Modify: `crates/emcore/src/emView.rs:2410-2521` (paint_panel_recursive)

`paint_sub_tree` still calls `paint_panel_recursive`. Fix its bugs to match C++ behavior: remove `visible` check, fix clip handling. This function is used by `emSubViewPanel` and `emWindow` control strip painting.

Note: `paint_panel_recursive` operates differently from the main `Paint` — it gets a `base_offset` and `parent_canvas` passed in. The `base_offset` corresponds to what C++ calls `ox, oy`. The function is recursive (fine for sub-views which have shallow trees).

- [ ] **Step 1: Remove the `visible` check**

In `crates/emcore/src/emView.rs`, in `paint_panel_recursive`, change the match guard:

```rust
// OLD (line 2420):
Some(p) if p.viewed && p.visible => (

// NEW:
Some(p) if p.viewed => (
```

- [ ] **Step 2: Remove the `effective_canvas` resolution logic**

Replace the canvas color section. Children should get their own `p->CanvasColor` directly, matching C++ line 1118:

```rust
// OLD (lines 2459-2474):
// C++ PaintView passes canvasColor differently...
let effective_canvas = if canvas_color.GetAlpha() > 0 {
    canvas_color
} else {
    parent_canvas
};
painter.SetCanvasColor(effective_canvas);

// NEW:
painter.SetCanvasColor(canvas_color);
```

- [ ] **Step 3: Fix child recursion canvas color**

Change the recursive call to pass the child's own canvas color instead of TRANSPARENT:

```rust
// OLD (lines 2507-2517):
// C++ line 1118: p->Paint(pnt, p->CanvasColor)...
self.paint_panel_recursive(
    tree,
    painter,
    child,
    base_offset,
    emColor::TRANSPARENT,
);

// NEW:
let child_canvas = tree.GetRec(child)
    .map(|c| c.canvas_color)
    .unwrap_or(emColor::TRANSPARENT);
self.paint_panel_recursive(
    tree,
    painter,
    child,
    base_offset,
    child_canvas,
);
```

- [ ] **Step 4: Verify compilation**

Run: `cargo clippy -- -D warnings`
Expected: PASS. The `parent_canvas` parameter may become unused — if so, remove it from the signature and update callers (`Paint` no longer calls this, only `paint_sub_tree` does).

- [ ] **Step 5: Clean up unused parameters**

Since `Paint` no longer calls `paint_panel_recursive`, check if `parent_canvas` is still needed. It IS still needed because `paint_sub_tree` passes a `background` color. But now the function uses `canvas_color` (the panel's own) directly, not `parent_canvas`. We should remove `parent_canvas` if it's unused.

Check: the function now only uses `canvas_color` from the panel data. The `parent_canvas` parameter is no longer referenced. Remove it:

In `paint_panel_recursive` signature:
```rust
// OLD:
fn paint_panel_recursive(
    &self,
    tree: &mut PanelTree,
    painter: &mut emPainter,
    id: PanelId,
    base_offset: (f64, f64),
    parent_canvas: emColor,
) {

// NEW:
fn paint_panel_recursive(
    &self,
    tree: &mut PanelTree,
    painter: &mut emPainter,
    id: PanelId,
    base_offset: (f64, f64),
) {
```

Update `paint_sub_tree`:
```rust
// OLD:
pub(crate) fn paint_sub_tree(
    &self,
    tree: &mut PanelTree,
    painter: &mut emPainter,
    root: PanelId,
    base_offset: (f64, f64),
    background: emColor,
) {
    self.paint_panel_recursive(tree, painter, root, base_offset, background);
}

// NEW:
pub(crate) fn paint_sub_tree(
    &self,
    tree: &mut PanelTree,
    painter: &mut emPainter,
    root: PanelId,
    base_offset: (f64, f64),
    _background: emColor,
) {
    self.paint_panel_recursive(tree, painter, root, base_offset);
}
```

Update recursive call:
```rust
// OLD:
self.paint_panel_recursive(
    tree,
    painter,
    child,
    base_offset,
    child_canvas,
);

// NEW:
self.paint_panel_recursive(
    tree,
    painter,
    child,
    base_offset,
);
```

- [ ] **Step 6: Verify compilation and tests**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: PASS

- [ ] **Step 7: Run golden tests**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | grep -E 'FAILED|test result'`
Expected: No regressions from Task 3 results.

- [ ] **Step 8: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "fix: remove visible check and fix canvas color in paint_panel_recursive"
```

---

### Task 5: Verify and Diagnose

**Files:** None modified — verification only.

- [ ] **Step 1: Run full golden test suite**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | tail -30`
Record the full result: how many pass/fail.

- [ ] **Step 2: If composition_tktest tests improved, record success**

The key targets are `composition_tktest_1x` and `composition_tktest_2x` which had zero DrawOp mismatches but 6.74% pixel diffs — suggesting the paint traversal was wrong.

- [ ] **Step 3: For any remaining failures, run DrawOp diff**

For each still-failing test:
```bash
DUMP_DRAW_OPS=1 cargo test --test golden <test_name> -- --test-threads=1
python3 scripts/diff_draw_ops.py <test_name>
```

This produces op-by-op comparison to identify remaining divergences.

- [ ] **Step 4: Record findings**

Document the before/after golden test results and any remaining divergence patterns for future work.

---

### Task 6: Fix Iterative DFS Walk-Up Bug

**Files:**
- Modify: `crates/emcore/src/emView.rs` (Paint function, DFS walk-up logic)

The iterative DFS walk-up logic in Task 3 has a subtle correctness risk: when walking up from a leaf, the C++ code does `do { p=p->Parent; } while (p!=SVP && !p->Next); if (p==SVP) break; p=p->Next;`. The Rust port must match this exactly. This task exists as a checkpoint to verify the walk-up is correct after Task 5 reveals any tree-traversal issues.

**Skip this task if Task 5 shows no tree-traversal issues.**

- [ ] **Step 1: Compare walk-up logic against C++**

C++ (lines 1127-1134):
```cpp
if (p->Next) p=p->Next;
else {
    do {
        p=p->Parent;
    } while (p!=SupremeViewedPanel && !p->Next);
    if (p==SupremeViewedPanel) break;
    p=p->Next;
}
```

Verify the Rust in Task 3's code matches this exactly. The key invariant: when we walk up past a node with no Next sibling, we keep walking up until we find a node with a Next sibling OR we hit SVP.

- [ ] **Step 2: Fix if needed, test, commit**

If the walk-up logic doesn't match, fix it, run `cargo clippy -- -D warnings && cargo-nextest ntr`, run golden tests, and commit.
