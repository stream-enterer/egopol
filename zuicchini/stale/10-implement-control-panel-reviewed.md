# Implement Control Panel Mechanism (Reviewed)

## What this is

The control panel is an overlay widget that appears when a panel is "visited" (actively focused). In C++, it shows panel-specific controls — the TestPanel uses it to display a label with identity and color info. The mechanism delegates upward through the panel tree: each panel's `create_control_panel` override can return a custom widget, or delegate to its parent.

## Architecture: separate control tree (matching C++ intent)

> **Why not a single tree?** The original plan proposed putting control panels as children of root in the content tree. Review found this is a dead end: active-path pollution (auto-focus selecting the control panel), layout injection (no behavior positions it), `delete_all_children` destroying it silently, and no migration path to the real C++ architecture. Every one of these requires a workaround that future contributors won't know about.
>
> **Why not SubViewPanel-in-tree?** `PanelBehavior` has no `as_any()` downcasting, so `about_to_wait` can't reach into a SubViewPanel's sub_tree to manage control panel lifecycle. Adding downcasting is possible but invasive. The C++ uses `emMainPanel` (application-layer, not core) with two SubViewPanels — that's the right path for the full app, but overkill for the standalone test binary.
>
> **This plan:** ZuiWindow owns a separate `PanelTree` + `View` for the control region. `about_to_wait` has direct access to both the content view (to check invalidation) and the control tree (to create/destroy panels). This matches the C++ architectural property — two independent view/tree pairs — without requiring the full `emMainPanel` apparatus. When `emMainPanel` is built later, the control tree moves into a SubViewPanel naturally.

### The cross-tree creation trick

C++ `emMainControlPanel::RecreateContentControlPanel()` calls `ContentView.CreateControlPanel(*this, "context")` — this walks the **content** view's active panel chain to find the behavior override, but creates the new panel as a child of `*this`, which is in the **control** tree. This works because C++ `emPanel(parent, name)` binds the new panel to whichever tree the parent lives in.

In Rust, the same separation is achieved by a new `PanelTree` method:

```rust
/// Walk this tree's parent chain from `id`, but create the control panel
/// in `target_tree` as a child of `parent_arg`.
pub fn create_control_panel_in(
    &mut self,
    id: PanelId,
    target_tree: &mut PanelTree,
    parent_arg: PanelId,
    name: &str,
) -> Option<PanelId>
```

This takes behaviors out of the content tree (via `take_behavior`), hands them a `PanelCtx` pointing at the control tree, lets them create children there, then puts the behavior back. The only change from the existing `create_control_panel` is `PanelCtx::new(target_tree, parent_arg)` instead of `PanelCtx::new(self, parent_arg)`.

## What already exists in Rust

| Component | File | Status |
|-----------|------|--------|
| `SubViewPanel` | `src/panel/sub_view_panel.rs` | Fully implemented (no input forwarding yet — not needed for this plan) |
| `PanelBehavior::create_control_panel()` | `src/panel/behavior.rs:197` | Trait method exists, default returns None |
| `PanelTree::create_control_panel()` delegation | `src/panel/tree.rs:1333-1354` | Walks parent chain, creates in same tree |
| `View::invalidate_control_panel()` | `src/panel/view.rs:1582-1585` | Sets `control_panel_invalid = true` |
| `View::is_control_panel_invalid()` | `src/panel/view.rs:1610-1612` | Reads flag |
| `View::clear_control_panel_invalid()` | `src/panel/view.rs:1615-1616` | Clears flag |
| `control_panel_invalid` field | `src/panel/view.rs:76` | Exists |
| `View::create_control_panel()` | `src/panel/view.rs:1882-1884` | **STUB — returns None, wrong signature** |
| `TestPanel::create_control_panel()` | `examples/test_panel.rs:832-841` | **Already implemented** (creates LabelPanel matching C++) |

## What's missing

Six pieces, in implementation order:

### 1. Fix `set_active_panel` to invalidate the control panel (1 line)

> **[Critical — without this, the entire mechanism is inert.]** In C++ (`emView.cpp:308`), `SetActivePanel` fires `Signal(ControlPanelSignal)`. In Rust, `set_active_panel` (`view.rs:707-753`) never sets the flag. The control panel will never update when the user clicks a different panel.

At `view.rs:752`, after `self.activation_adherent = adherent;`, add:

```rust
self.control_panel_invalid = true;
```

This must be unconditional (not guarded by `in_active_path`) because it fires on every active-panel change, matching C++ `SetActivePanel` which always signals.

### 2. Add `PanelTree::create_control_panel_in` (cross-tree creation)

New method on `PanelTree`, adjacent to the existing `create_control_panel` at `tree.rs:1333`. The body is nearly identical — the only change is the `PanelCtx` target:

```rust
pub fn create_control_panel_in(
    &mut self,
    id: PanelId,
    target_tree: &mut PanelTree,
    parent_arg: PanelId,
    name: &str,
) -> Option<PanelId> {
    let mut cur = id;
    loop {
        if let Some(mut behavior) = self.take_behavior(cur) {
            let mut ctx = PanelCtx::new(target_tree, parent_arg);
            let result = behavior.create_control_panel(&mut ctx, name);
            self.put_behavior(cur, behavior);
            if result.is_some() {
                return result;
            }
        }
        match self.panels.get(cur).and_then(|p| p.parent) {
            Some(parent) => cur = parent,
            None => return None,
        }
    }
}
```

This walks `self` (content tree) to find the behavior override, but creates the panel in `target_tree` (control tree).

> **Note on `PanelCtx.id`:** The `PanelCtx` gets `id = parent_arg` (the control tree's root), not the behavior's own panel ID. The existing `TestPanel::create_control_panel` at `test_panel.rs:833` calls `ctx.tree.get_identity(ctx.id)`, which will return the control root's identity, not TestPanel's. Verify this matches C++ behavior — C++ uses `parent.GetIdentity()` in the same position, so this may be correct for parity.

### 3. Fix `View::create_control_panel` signature and implementation (~5 lines)

The stub at `view.rs:1882` has the wrong signature:
```rust
// Current (broken):
pub fn create_control_panel(&self, _tree: &PanelTree) -> Option<PanelId>
```

Change to:
```rust
pub fn create_control_panel(
    &self,
    content_tree: &mut PanelTree,
    control_tree: &mut PanelTree,
    parent: PanelId,
    name: &str,
) -> Option<PanelId> {
    let active = self.active?;
    content_tree.create_control_panel_in(active, control_tree, parent, name)
}
```

This reads the active panel from `self`, then delegates to the cross-tree creation method.

### 4. Add control tree fields to `ZuiWindow` (~20 lines)

Add to `ZuiWindow` struct (`zui_window.rs:32`):

```rust
control_tree: PanelTree,
control_view: View,
control_panel_id: Option<PanelId>,
control_strip_height: u32,  // 0 when no control panel, CONTROL_STRIP_PX when active
```

Initialize in `ZuiWindow::create`:
- Create a `PanelTree` with a root panel named `"control_root"`
- Set the root's layout rect to `(0.0, 0.0, 1.0, 1.0)`
- Create a `View` pointing to that root, with initial viewport `(w, 0)` (hidden — no control panel yet)
- Set `control_panel_id = None`, `control_strip_height = 0`

### 5. Control region geometry

> **C++ reference:** In C++, `emMainPanel` uses a slider-controlled split with `ControlTallness = 0.0538` (aspect ratio). The control region is always in the tree (even minimized at height 1E-5). Default slider position 0.515 gives the control region ~3-5% of window height. A `spaceFac = 1.015` gap separates control from content.
>
> **For the standalone binary:** No slider. Use a fixed-height strip.

**Spec:**

```rust
const CONTROL_STRIP_PX: u32 = 32;
```

- When `control_panel_id.is_some()`: the control strip occupies the **bottom** `CONTROL_STRIP_PX` pixels. The content viewport shrinks to `h - CONTROL_STRIP_PX`.
- When `control_panel_id.is_none()`: strip height is 0, content gets the full viewport.
- On strip show/hide: call `win.view_mut().set_viewport(&mut tree, w, content_h)` to resize the content viewport. This triggers layout recalculation (new `pixel_tallness`) and a full repaint. Also call `win.control_view.set_viewport(&mut win.control_tree, w, CONTROL_STRIP_PX)`.
- On window resize: recompute the split. Content gets `h - control_strip_height`, control gets `control_strip_height`.
- The control view's `pixel_tallness` = `CONTROL_STRIP_PX as f64 / w as f64`. Set this when updating the control viewport.

Add helper methods to `ZuiWindow`:
```rust
fn content_height(&self) -> u32 {
    self.surface_config.height - self.control_strip_height
}

fn show_control_strip(&mut self, tree: &mut PanelTree) {
    if self.control_strip_height == 0 {
        self.control_strip_height = CONTROL_STRIP_PX;
        let w = self.surface_config.width;
        let ch = self.content_height();
        self.view.set_viewport(tree, w as f64, ch as f64);
        self.control_view.set_viewport(
            &mut self.control_tree,
            w as f64,
            CONTROL_STRIP_PX as f64,
        );
        self.invalidate();
    }
}

fn hide_control_strip(&mut self, tree: &mut PanelTree) {
    if self.control_strip_height > 0 {
        self.control_strip_height = 0;
        let w = self.surface_config.width;
        let h = self.surface_config.height;
        self.view.set_viewport(tree, w as f64, h as f64);
        self.invalidate();
    }
}
```

### 6. Implement lifecycle management in `about_to_wait` (~30-40 lines)

> **Borrow choreography:** `about_to_wait` iterates `self.windows.values_mut()` while having `let tree = &mut self.tree`. The content view is on `ZuiWindow`, the content tree is `self.tree`, and the control tree is on `ZuiWindow`. All three are in separate struct fields, so the borrows don't conflict. This is simpler than the single-tree approach.

In the `about_to_wait` loop, after `win.view_mut().update(tree)` (line 272), add:

```rust
// Control panel lifecycle
if win.view().is_control_panel_invalid() {
    // Destroy old control panel
    if let Some(old_id) = win.control_panel_id.take() {
        win.control_tree.remove(old_id);
    }

    // Create new control panel in the control tree
    let control_root = win.control_view.root();
    let new_id = win.view().create_control_panel(
        tree,                    // content tree (for delegation walk)
        &mut win.control_tree,   // control tree (for panel creation)
        control_root,
        "context",
    );
    win.control_panel_id = new_id;

    // Show or hide the control strip based on whether a panel was created
    if new_id.is_some() {
        win.show_control_strip(tree);
    } else {
        win.hide_control_strip(tree);
    }

    win.view_mut().clear_control_panel_invalid();
    needs_full_repaint = true;
}

// Deliver notices for control tree (layout, children changed, etc.)
if win.control_strip_height > 0 {
    win.control_tree.deliver_notices(window_focused, pixel_tallness);
    win.control_view.update(&mut win.control_tree);
}
```

### 7. Rendering the control region (~20 lines)

> **How the existing pipeline works:** `ZuiWindow::render` has three paths based on dirty tile count: (1) >50% dirty → single-pass into `viewport_buffer`, copy chunks to tiles; (2) >1 dirty + threads → parallel display-list; (3) few dirty → per-tile painting. All paths use the same tile cache grid covering the entire window viewport. The compositor draws tiles as GPU quads in NDC space.

**The control region paints into the same tile grid.** No compositor changes needed. Tiles in the bottom strip simply contain control panel pixels instead of content pixels.

**Implementation — add a second paint call in each render path:**

1. **Viewport-buffer path** (>50% dirty): After `self.view.paint(tree, &mut painter)` paints the content tree into the viewport buffer, add:
   ```rust
   if self.control_strip_height > 0 {
       let content_h = self.content_height();
       // Paint control tree into the bottom strip of the same viewport buffer.
       // The painter's clip already covers the full viewport.
       // The control view's panels have viewed_y relative to (0,0) of the
       // control viewport, so offset by content_height to place them at the bottom.
       let base_offset = (0.0, content_h as f64);
       let bg = self.control_view.background_color();
       let control_root = self.control_view.root();
       self.control_view.paint_sub_tree(
           &mut self.control_tree, &mut painter, control_root, base_offset, bg,
       );
   }
   ```
   Then tile copy proceeds as before — bottom tiles now contain control pixels.

2. **Per-tile path** (few dirty): After painting the content tree into a tile, if the tile overlaps the control strip (i.e. `row * TILE_SIZE + TILE_SIZE > content_height`), also call:
   ```rust
   let base_offset = (-(col * ts) as f64, -(row * ts) as f64 + content_h as f64);
   self.control_view.paint_sub_tree(
       &mut self.control_tree, &mut painter, control_root, base_offset, bg,
   );
   ```

3. **Display-list path**: Same as per-tile — record control tree draw ops after content tree ops.

**Background:** Fill the control strip area with the control view's background color before painting panels. The `paint_sub_tree` call handles this via the `background` parameter, which fills unpainted areas.

### 8. Input dispatch for the control region (~25 lines)

> **C++ reference:** Input routing is spatial. `emView::RecurseInput` checks `IsPointInSubstanceRect(mx,my)` for each panel. `emSubViewPanel::Input` forwards events to the sub-view via `SubViewPort->InputToView(event, state)`, which enters the sub-view's own VIF chain. Mouse coordinates are NOT transformed — the sub-view's geometry matches the panel's absolute pixel position. Each sub-view has its own independent VIF chain (zoom, scroll, keyboard) and active panel tracking. Clicking the control region does NOT affect the content view's active panel.

**Implementation in `ZuiWindow::dispatch_input`:**

At the top of `dispatch_input`, before the VIF chain, check mouse Y:

```rust
let content_h = self.content_height() as f64;

// Route to control region if mouse is below the content viewport
if event.mouse_y >= content_h && self.control_strip_height > 0 {
    // Transform mouse Y into control-view space
    let mut ctrl_event = event.clone();
    ctrl_event.mouse_y -= content_h;

    // For mouse press: hit-test and set active panel in the CONTROL view
    if ctrl_event.variant == InputVariant::Press
        && matches!(ctrl_event.key,
            InputKey::MouseLeft | InputKey::MouseRight | InputKey::MouseMiddle)
    {
        let panel = self.control_view
            .get_focusable_panel_at(&self.control_tree, ctrl_event.mouse_x, ctrl_event.mouse_y)
            .unwrap_or_else(|| self.control_view.root());
        self.control_view.set_active_panel(&mut self.control_tree, panel, false);
    }

    // Dispatch to control tree panels
    let ctrl_ev = ctrl_event.with_modifiers(state);
    let wf = self.view.window_focused();
    let viewed = self.control_tree.viewed_panels_dfs();
    for panel_id in viewed {
        let mut panel_ev = ctrl_ev.clone();
        panel_ev.mouse_x = self.control_tree.view_to_panel_x(panel_id, ctrl_ev.mouse_x);
        panel_ev.mouse_y = self.control_tree.view_to_panel_y(panel_id, ctrl_ev.mouse_y);
        if let Some(mut behavior) = self.control_tree.take_behavior(panel_id) {
            let panel_state = self.control_tree.build_panel_state(
                panel_id, wf, self.control_view.pixel_tallness(),
            );
            let consumed = behavior.input(&panel_ev, &panel_state, state);
            self.control_tree.put_behavior(panel_id, behavior);
            if consumed { break; }
        }
    }
    return; // Do NOT fall through to content dispatch
}
```

**No VIF chain for the control view.** The control strip is a flat label region — no zoom/scroll behavior. The C++ control view gets its own VIFs with `VF_POPUP_ZOOM`, but that's for the full application where the control panel is zoomable. For the standalone binary, direct panel dispatch is sufficient. If zoom support is needed later, add a VIF chain to `ZuiWindow` for the control view.

### 7. TestPanel::create_control_panel — already done

> **The original plan incorrectly says TestPanel should create a ColorField.** C++ `emTestPanel::CreateControlPanel` (`emTestPanel.cpp:515`) creates an `emLabel`, not an `emColorField`. The existing Rust override at `examples/test_panel.rs:832-841` already correctly creates a `LabelPanel`, matching C++. **Do not replace it.**

The existing implementation creates a LabelPanel showing the panel identity and background color. It works as-is once the cross-tree creation is wired up.

## Implementation steps

1. **Read all existing infrastructure** listed in the table above. Pay special attention to:
   - `PanelTree::create_control_panel` at `tree.rs:1333-1354` (the delegation walk you'll duplicate)
   - `PanelCtx::new` at the ctx module (what `ctx.id` means — it determines `create_child_with`'s parent)
   - The `about_to_wait` loop at `app.rs:221-305` (borrow patterns, notice delivery, repaint)
   - `View::paint_sub_tree` (how a sub-view renders — used by `SubViewPanel::paint`)
   - The three render paths in `ZuiWindow::render` (viewport-buffer, display-list, per-tile)
   - `TestPanel::create_control_panel` at `examples/test_panel.rs:832-841` (already implemented)

2. **Fix `set_active_panel`** (Section 1 above). Without this, nothing else is testable.

3. **Add `create_control_panel_in`** (Section 2 above). Key new method enabling cross-tree creation.

4. **Fix `View::create_control_panel`** (Section 3 above). Change signature, forward via cross-tree method.

5. **Add control tree fields to `ZuiWindow`** (Section 4 above). Initialize in `create`. Add `content_height()`, `show_control_strip()`, `hide_control_strip()` helpers. Update `resize()` to recompute the content/control split.

6. **Implement lifecycle management in `about_to_wait`** (Section 6 above). Check flag, destroy old, create new, show/hide strip, deliver control tree notices.

7. **Implement rendering** (Section 7 above). Add second `paint_sub_tree` call in all three render paths with `base_offset.y = content_height`. Bottom tiles get control pixels.

8. **Implement input routing** (Section 8 above). Check `mouse_y >= content_height`, transform coords, dispatch to control tree. Return early — do NOT fall through to content dispatch.

9. **Gate:** `cargo check --workspace` + `cargo clippy --workspace -- -D warnings`. Fix any issues before proceeding.

10. **Verify visually:**
    - `cargo-nextest ntr --workspace` — zero regressions
    - Run the standalone binary. Visit the TestPanel (click on it). The label with identity/color info should appear in a strip at the bottom.
    - Navigate to a different panel — the control panel should update.
    - Click on the control panel — the content panel should NOT change.
    - Resize the window — the strip should stay at the bottom, content should relayout.

11. **Commit:** `feat: implement control panel mechanism with separate control tree`

## Evolution path to full emMainPanel

This plan gives you: two independent trees, cross-tree panel creation, window-level lifecycle management.

When emMainPanel is needed (for the full application, not the standalone test binary):
1. Move the content tree into a `SubViewPanel` child of the main tree (requires `SubViewPanel::input()` forwarding — not yet implemented).
2. Move the control tree into a second `SubViewPanel` child.
3. Add `MainPanel` behavior that creates both SubViewPanels and handles the slider split.
4. Move lifecycle management from `about_to_wait` into a `ControlPanelManager` behavior in the control tree, communicating with the content view via a shared `Rc<Cell<bool>>` signal.

This evolution is incremental — each step adds capability without rewriting prior work. This is the key difference from the single-tree approach, which was a dead end.

## Self-contained concerns (noted, not blockers)

- **Dirty-rect integration:** Creating/destroying panels in the control tree triggers its own notice delivery. The control tree's `deliver_notices` call in `about_to_wait` handles this. The content tree's dirty rects are unaffected.
- **Multiple windows:** Each `ZuiWindow` owns its own control tree. No cross-window interference. This is better than the single-tree approach where multiple windows shared one tree.
- **Bidirectional data binding:** TestPanel uses a read-only label. Future interactive control panels (ColorField) will need a callback from control-tree behaviors to content-tree behaviors. Cross that bridge when it comes — Rc<Cell<T>> shared state is the likely mechanism.
- **SubViewPanel::input():** Not needed for this plan (control input is dispatched directly by the window). Will be needed when SubViewPanel is used as a panel-in-tree for the full emMainPanel.

## C++ reference files

- `~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp:193-198` — CreateControlPanel
- `~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp:302-308` — SetActivePanel signals ControlPanelSignal
- `~/.local/git/eaglemode-0.96.4/src/emCore/emPanel.cpp:1244-1246` — Default delegation up parent chain
- `~/.local/git/eaglemode-0.96.4/src/emCore/emPanel.cpp:1323-1325` — InvalidateControlPanel
- `~/.local/git/eaglemode-0.96.4/src/emTest/emTestPanel.cpp:511-535` — TestPanel override creating **emLabel** (not emColorField)
- `~/.local/git/eaglemode-0.96.4/src/emMain/emMainControlPanel.cpp:317-324` — Lifecycle: RecreateContentControlPanel
- `~/.local/git/eaglemode-0.96.4/src/emMain/emMainControlPanel.h:61-62` — Holds `emView & ContentView` reference
- `~/.local/git/eaglemode-0.96.4/src/emMain/emMainPanel.cpp` — Two-SubViewPanel window split
- `~/.local/git/eaglemode-0.96.4/src/emCore/emSubViewPanel.cpp` — SubViewPanel (already ported)

## Rules

- Do not rewrite SubViewPanel. It's already ported and working.
- Do not restructure the content panel tree. The content tree stays in `App.tree` as-is.
- Do not replace TestPanel's LabelPanel with a ColorField — the existing implementation matches C++.
- Do not put control panels in the content tree. They belong in `ZuiWindow.control_tree`.
- The control panel must appear when any panel is visited, not just TestPanel — the mechanism is generic.
- `cargo test --workspace` must pass before committing.
- The control region must not affect the content view's active panel tracking.
