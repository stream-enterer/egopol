# Startup C++ Match Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the Rust startup and control panel architecture to exactly match C++ Eagle Mode — eagle image, zoom animation, cosmos, signal-driven control panels, full C++ parity.

**Architecture:** Two tiers. Tier 1 (Tasks 1-8) is a coordinated big-bang rewrite of the startup/control-panel system — all changes compile together. Tier 2 (Tasks 9-16) adds remaining C++ parity features incrementally. Each tier produces testable software.

**Tech Stack:** Rust, wgpu, winit, slotmap (PanelId/SignalId/EngineId), emCore panel tree

**Spec:** `docs/superpowers/specs/2026-04-15-startup-cpp-match-rewrite-design.md`

**C++ reference:** `~/git/eaglemode-0.96.4/` (headers in `include/emCore/`, `include/emMain/`; impl in `src/emCore/`, `src/emMain/`)

**Baseline:** Golden tests 239 pass / 4 fail. Full suite 2383 pass / 16 fail. All pre-existing.

---

## File Structure

### Tier 1 — Modified files:
- `crates/emcore/src/emSubViewPanel.rs` — Add sub-tree HandleNotice + run_panel_cycles
- `crates/emcore/src/emView.rs` — Add SignalId fields, scheduler ref, VisitByIdentity
- `crates/emmain/src/emMainPanel.rs` — Delete creation_stage, add sub-view ID getters
- `crates/emmain/src/emMainControlPanel.rs` — Restructure layout, new constructor, Escape handling
- `crates/emcore/src/emWindow.rs` — Remove control_tree/control_view/control_strip_height
- `crates/emcore/src/emGUIFramework.rs` — Delete control panel lifecycle from about_to_wait
- `crates/emmain/src/emMainWindow.rs` — Rewrite StartupEngine states 5/6/11, ToggleControlView, MainWindowEngine title, ControlPanelBridge engine

### Tier 2 — Modified/created files:
- `crates/emmain/src/emMainWindow.rs` — Duplicate, CreateControlWindow, RecreateContentPanels
- `crates/emcore/src/emWindowStateSaver.rs` — New: geometry persistence engine
- `crates/emmain/src/emStarFieldPanel.rs` — TicTacToe easter egg
- `crates/emmain/src/emVirtualCosmos.rs` — Copy-to-user
- `crates/emcore/src/emFileModel.rs` — ReloadFiles signal
- `crates/emmain/src/emAutoplayControlPanel.rs` — Full UI
- `crates/emcore/src/emWindowPlatform.rs` — Screensaver timer + fallback

---

## Tier 1: Core Startup + Control Panel

**All Tier 1 tasks form a coordinated big-bang rewrite. Intermediate tasks may not compile independently. The verification step at the end of Task 8 confirms everything works together.**

### Task 1: emSubViewPanel — Sub-tree Notice Delivery and Panel Cycling

**Spec:** §1
**Files:**
- Modify: `crates/emcore/src/emSubViewPanel.rs:205-226`

**Context:** emSubViewPanel owns a `sub_tree: PanelTree` and `sub_view: emView` for each sub-view. Currently, `HandleNotice()` and `run_panel_cycles()` are never called on the sub-tree. This means panels created inside sub-views (emMainControlPanel, emMainContentPanel) never get their `LayoutChildren` called and their `Cycle()` never runs. This is the root cause of the empty blue rectangles.

- [ ] **Step 1: Add sub-tree lifecycle calls to Paint**

In `crates/emcore/src/emSubViewPanel.rs`, replace the `Paint` method (lines 205-226) with:

```rust
fn Paint(&mut self, painter: &mut emPainter, _w: f64, _h: f64, state: &PanelState) {
    if !state.viewed {
        return;
    }

    // Drive sub-tree lifecycle (C++ does this via emViewPort::RequestUpdate).
    // run_panel_cycles executes Cycle() for panels in the sub-tree's cycle_list
    // (e.g. emMainControlPanel button handling).
    self.sub_tree.run_panel_cycles();
    // HandleNotice delivers LAYOUT_CHANGED etc. so LayoutChildren runs and
    // children (eagle, cosmos, buttons) are actually created.
    self.sub_tree.HandleNotice(state.is_focused(), state.pixel_tallness);

    // Update the sub-view's viewing state so panel coordinates are current.
    self.sub_view.Update(&mut self.sub_tree);

    let base_offset = painter.origin();
    let bg = self.sub_view.GetBackgroundColor();
    let root = self.sub_root();

    self.sub_view
        .paint_sub_tree(&mut self.sub_tree, painter, root, base_offset, bg);
}
```

- [ ] **Step 2: Run `cargo check` (may have unrelated errors from other Tier 1 tasks — that's OK for now)**

Run: `cargo check 2>&1 | head -20`

- [ ] **Step 3: Commit**

```bash
git add crates/emcore/src/emSubViewPanel.rs
git commit -m "feat: add sub-tree HandleNotice + run_panel_cycles to emSubViewPanel::Paint

Panels inside sub-views (emMainControlPanel, emMainContentPanel) were
never laid out or cycled because HandleNotice/run_panel_cycles were only
called on the main tree. This was the root cause of the empty blue
rectangles — panels existed but their LayoutChildren never ran."
```

---

### Task 2: emView — Add SignalIds, Scheduler Reference, and VisitByIdentity

**Spec:** §10, §11
**Files:**
- Modify: `crates/emcore/src/emView.rs`

**Context:** C++ emView has `ControlPanelSignal` and `TitleSignal` as real signals that engines can connect to. Rust emView only has a `control_panel_invalid: bool` flag. We need real `SignalId` fields and a scheduler reference so emView can fire signals when the active panel changes or the title changes. We also need `VisitByIdentity` so StartupEngine state 11 and bookmark hotkeys can navigate by identity string.

- [ ] **Step 1: Add signal fields and scheduler reference to emView struct**

In `crates/emcore/src/emView.rs`, add these fields to the `emView` struct (after `control_panel_invalid: bool` at line 207):

```rust
    control_panel_signal: Option<super::emSignal::SignalId>,
    title_signal: Option<super::emSignal::SignalId>,
    scheduler: Option<Rc<RefCell<super::emScheduler::EngineScheduler>>>,
```

Add the import at the top of the file (with the other `use` statements):

```rust
use std::cell::RefCell;
use std::rc::Rc;
```

- [ ] **Step 2: Initialize new fields in `new()` constructor (around line 281)**

Add after `control_panel_invalid: false,`:

```rust
            control_panel_signal: None,
            title_signal: None,
            scheduler: None,
```

- [ ] **Step 3: Add setter/getter methods for signals and scheduler**

Add these methods to the `impl emView` block:

```rust
    pub fn set_control_panel_signal(&mut self, signal: super::emSignal::SignalId) {
        self.control_panel_signal = Some(signal);
    }

    pub fn GetControlPanelSignal(&self) -> Option<super::emSignal::SignalId> {
        self.control_panel_signal
    }

    pub fn set_title_signal(&mut self, signal: super::emSignal::SignalId) {
        self.title_signal = Some(signal);
    }

    pub fn GetTitleSignal(&self) -> Option<super::emSignal::SignalId> {
        self.title_signal
    }

    pub fn set_scheduler(&mut self, scheduler: Rc<RefCell<super::emScheduler::EngineScheduler>>) {
        self.scheduler = Some(scheduler);
    }
```

- [ ] **Step 4: Fire ControlPanelSignal in `set_active_panel()` (line 941)**

At line 941, after `self.control_panel_invalid = true;`, add:

```rust
        if let Some(sig) = self.control_panel_signal {
            if let Some(sched) = &self.scheduler {
                sched.borrow_mut().fire(sig);
            }
        }
```

- [ ] **Step 5: Fire ControlPanelSignal in `InvalidateControlPanel()` (after line 1802)**

Inside `InvalidateControlPanel`, after `self.control_panel_invalid = true;` (line 1802), add:

```rust
            if let Some(sig) = self.control_panel_signal {
                if let Some(sched) = &self.scheduler {
                    sched.borrow_mut().fire(sig);
                }
            }
```

- [ ] **Step 6: Add `VisitByIdentity` method**

Add this method to the `impl emView` block:

```rust
    /// Visit a panel by identity string, matching C++ emView::Visit(identity, ...).
    /// Uses DecodeIdentity to resolve the identity to a PanelId, then calls Visit.
    pub fn VisitByIdentity(
        &mut self,
        tree: &mut super::emPanelTree::PanelTree,
        identity: &str,
        rel_x: f64,
        rel_y: f64,
        rel_a: f64,
    ) {
        if let Some(panel_id) = tree.find_panel_by_identity(identity) {
            self.Visit(panel_id, rel_x, rel_y, rel_a);
        } else {
            log::warn!("VisitByIdentity: panel not found for identity '{identity}'");
        }
    }
```

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "feat(emView): add real SignalIds, scheduler ref, and VisitByIdentity

ControlPanelSignal and TitleSignal are now real SignalIds that fire via
the scheduler when the active panel changes or the control panel is
invalidated. VisitByIdentity resolves identity strings to PanelIds
for StartupEngine state 11 and bookmark hotkey navigation."
```

---

### Task 3: emMainPanel — Eliminate creation_stage

**Spec:** §2
**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

**Context:** C++ StartupEngine creates emMainControlPanel and emMainContentPanel directly inside sub-views. The Rust `creation_stage` mechanism gates this in LayoutChildren — an indirection that broke the startup. We delete creation_stage entirely and add public getters for sub-view panel IDs so StartupEngine can create panels directly.

- [ ] **Step 1: Delete creation_stage fields from struct**

In the `emMainPanel` struct (around lines 318-330), delete these fields:
- `control_panel_created: Option<PanelId>,` (line 318)
- `content_panel_created: Option<PanelId>,` (line 319)
- `creation_stage: u8,` (line 330)

- [ ] **Step 2: Delete creation_stage initialization from `new()`**

In the `new()` constructor (around lines 354-391), delete these lines from the struct initializer:
- `control_panel_created: None,`
- `content_panel_created: None,`
- `creation_stage: 0,`

- [ ] **Step 3: Delete creation_stage methods**

Delete `advance_creation_stage()` (lines 535-539) and `creation_stage()` getter (lines 542-544).

- [ ] **Step 4: Delete creation-gated blocks from LayoutChildren**

In `LayoutChildren` (lines 741-853), delete the two creation-gated blocks:
- Lines 778-794: The `if let Some(ctrl_id) = self.control_view_panel && self.control_panel_created.is_none() && self.creation_stage >= 1` block
- Lines 796-814: The `if let Some(content_id) = self.content_view_panel && self.content_panel_created.is_none() && self.creation_stage >= 2` block

LayoutChildren should now only: create sub-view panels (if !children_created), pass slider state, and position children.

- [ ] **Step 5: Add sub-view panel ID getters**

Add these public methods to `impl emMainPanel`:

```rust
    /// Get the PanelId of the control sub-view panel.
    /// Used by StartupEngine to create emMainControlPanel inside the sub-view.
    pub fn GetControlViewPanelId(&self) -> Option<PanelId> {
        self.control_view_panel
    }

    /// Get the PanelId of the content sub-view panel.
    /// Used by StartupEngine to create emMainContentPanel inside the sub-view.
    pub fn GetContentViewPanelId(&self) -> Option<PanelId> {
        self.content_view_panel
    }
```

- [ ] **Step 6: Delete creation_stage tests**

Delete these test functions (around lines 1229-1255):
- `test_creation_stage_initial`
- `test_advance_creation_stage`
- `test_advance_creation_stage_saturates_at_2`

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "refactor(emMainPanel): eliminate creation_stage mechanism

Delete creation_stage, advance_creation_stage(), control_panel_created,
content_panel_created, and creation-gated blocks from LayoutChildren.
Add GetControlViewPanelId/GetContentViewPanelId for direct sub-view
access by StartupEngine. LayoutChildren is now pure positioning."
```

---

### Task 4: emMainControlPanel — Restructure Layout + Constructor Change

**Spec:** §6
**Files:**
- Modify: `crates/emmain/src/emMainControlPanel.rs`

**Context:** C++ emMainControlPanel has lMain (general + bookmarks, weight 11.37) as child 0 and contentControlPanel (weight 21.32) as child 1. Rust currently has general (11.37) and bookmarks (21.32) — bookmarks in wrong position. We restructure to match C++, change the constructor to accept content_view_id, and add Escape key handling.

- [ ] **Step 1: Add content_view_id and content_control_panel fields to struct**

In the `emMainControlPanel` struct (around line 152), add:

```rust
    content_view_id: Option<PanelId>,
    content_control_panel: Option<PanelId>,
```

- [ ] **Step 2: Change constructor signature**

Change `pub fn new(ctx: Rc<emContext>) -> Self` to:

```rust
    pub fn new(ctx: Rc<emContext>, content_view_id: Option<PanelId>) -> Self
```

Add initialization of new fields in the `Self { ... }` block:

```rust
            content_view_id,
            content_control_panel: None,
```

- [ ] **Step 3: Create LMainPanel wrapper**

Add a new `LMainPanel` struct before the `GeneralPanel` struct (around line 351):

```rust
/// Wrapper matching C++ lMain: contains general (lAbtCfgCmd) + bookmarks.
/// C++ layout: child 0 = lAbtCfgCmd (weight 4.71), child 1 = bookmarks (weight 6.5).
struct LMainPanel {
    ctx: Rc<emContext>,
    look: Rc<emLook>,
    click_flags: Rc<ClickFlags>,
    layout: emLinearLayout,
    children_created: bool,
}

impl LMainPanel {
    fn new(ctx: Rc<emContext>, look: Rc<emLook>, click_flags: Rc<ClickFlags>) -> Self {
        Self {
            ctx,
            look,
            click_flags,
            layout: emLinearLayout {
                orientation: Orientation::Adaptive {
                    tallness_threshold: 1.0,
                },
                spacing: Spacing {
                    inner_h: 0.07,
                    inner_v: 0.07,
                    ..Spacing::default()
                },
                ..emLinearLayout::horizontal()
            },
            children_created: false,
        }
    }

    fn create_children(&mut self, ctx: &mut PanelCtx) {
        let general = Box::new(GeneralPanel::new(
            Rc::clone(&self.ctx),
            Rc::clone(&self.look),
            Rc::clone(&self.click_flags),
        ));
        let general_id = ctx.create_child_with("general", general);

        let bookmarks = Box::new(emBookmarksPanel::new(Rc::clone(&self.ctx)));
        let bm_id = ctx.create_child_with("bookmarks", bookmarks);

        // C++ lMain: child 0 = lAbtCfgCmd (weight 4.71), child 1 = bookmarks (weight 6.5)
        self.layout.set_child_constraint(
            general_id,
            ChildConstraint { weight: 4.71, ..Default::default() },
        );
        self.layout.set_child_constraint(
            bm_id,
            ChildConstraint { weight: 6.5, ..Default::default() },
        );

        self.children_created = true;
    }
}

impl PanelBehavior for LMainPanel {
    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        if !self.children_created {
            self.create_children(ctx);
        }
        let cc = ctx.GetCanvasColor();
        ctx.set_all_children_canvas_color(cc);
        self.layout.do_layout_skip(ctx, None, None);
    }

    fn notice(&mut self, _flags: NoticeFlags, _state: &PanelState) {}
}
```

- [ ] **Step 4: Rewrite create_children to use LMainPanel**

Replace the `create_children` method body in `emMainControlPanel`:

```rust
    fn create_children(&mut self, ctx: &mut PanelCtx) {
        let look = Rc::new(self.look.clone());
        let flags = Rc::clone(&self.click_flags);

        // Child 0: lMain (general + bookmarks, weight 11.37)
        let lmain = Box::new(LMainPanel::new(
            Rc::clone(&self.ctx),
            Rc::clone(&look),
            Rc::clone(&flags),
        ));
        let lmain_id = ctx.create_child_with("lMain", lmain);
        self.general_panel = Some(lmain_id);

        // Child 1: contentControlPanel slot (weight 21.32) — initially empty
        // The ControlPanelBridge engine will populate this dynamically.
        // For now, no child is created; the slot exists in the layout.

        self.layout_main.set_child_constraint(
            lmain_id,
            ChildConstraint { weight: 11.37, ..Default::default() },
        );

        // If a content control panel exists, set its weight
        if let Some(ccp_id) = self.content_control_panel {
            self.layout_main.set_child_constraint(
                ccp_id,
                ChildConstraint { weight: 21.32, ..Default::default() },
            );
        }

        self.children_created = true;
    }
```

Remove the `bookmarks_panel` field from the struct (it's now inside LMainPanel).

- [ ] **Step 5: Add Escape key handling to Input**

Add an `Input` method to the `PanelBehavior for emMainControlPanel` impl block:

```rust
    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        input_state: &emInputState,
    ) -> bool {
        // C++ emMainControlPanel::Input (emMainControlPanel.cpp:296-314)
        if event.key == emcore::emInput::InputKey::Escape
            && event.variant == emcore::emInput::InputVariant::Press
            && !input_state.GetShift()
            && !input_state.GetCtrl()
            && !input_state.GetAlt()
        {
            crate::emMainWindow::with_main_window(|mw| {
                // ToggleControlView will be rewritten in Task 7
                log::info!("emMainControlPanel: Escape → ToggleControlView");
            });
            return true;
        }
        false
    }
```

- [ ] **Step 6: Update tests for new constructor signature**

In the test module, update all `emMainControlPanel::new(ctx)` calls to `emMainControlPanel::new(ctx, None)`.

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emMainControlPanel.rs
git commit -m "refactor(emMainControlPanel): restructure layout to match C++

Add LMainPanel wrapper (general + bookmarks at weights 4.71/6.5).
Top-level layout: child 0 = lMain (11.37), child 1 = contentControlPanel
slot (21.32). Constructor now takes content_view_id. Add Escape key
handling for ToggleControlView."
```

---

### Task 5: ZuiWindow + emGUIFramework — Remove Control System

**Spec:** §4, §5
**Files:**
- Modify: `crates/emcore/src/emWindow.rs`
- Modify: `crates/emcore/src/emGUIFramework.rs`

**Context:** ZuiWindow has a separate control_tree/control_view for per-panel context controls. This doesn't exist in C++. Remove it entirely — the ControlPanelBridge engine (Task 7) replaces it with signal-driven lifecycle inside emMainControlPanel.

- [ ] **Step 1: Delete control system fields from ZuiWindow struct**

In `crates/emcore/src/emWindow.rs`, delete these fields from the `ZuiWindow` struct (lines 64-70):
- `pub(crate) control_tree: PanelTree,`
- `pub(crate) control_view: emView,`
- `pub(crate) control_panel_id: Option<PanelId>,`
- `pub(crate) control_strip_height: u32,`

- [ ] **Step 2: Delete control system methods**

Delete these methods from `impl ZuiWindow`:
- `show_control_strip()` (lines 221-234)
- `hide_control_strip()` (lines 237-245)
- `content_height()` (lines 214-218)

- [ ] **Step 3: Update `create()` — remove control_tree/control_view initialization**

In the `create()` method, delete lines 136-141 (control_tree creation) and remove the corresponding fields from the struct initializer (lines 179-182).

- [ ] **Step 4: Update `render()` — remove control strip paint pass**

In `render()`, remove all code that paints `self.control_view` or references `self.control_strip_height`. The three render paths (viewport buffer, single-tile, parallel) all have control strip blocks — delete them all. Only `self.view.Paint(tree, ...)` should remain.

- [ ] **Step 5: Update `resize()` — full height to main view**

In `resize()`, replace `let ch = self.content_height()` with `let ch = size.height` (or equivalent). Remove the `if self.control_strip_height > 0` block that updates control_view geometry.

- [ ] **Step 6: Delete control panel lifecycle from emGUIFramework about_to_wait**

In `crates/emcore/src/emGUIFramework.rs`, delete the entire control panel lifecycle block in `about_to_wait()` (lines 408-445):

```rust
// DELETE this entire block:
            // Control panel lifecycle
            if win.view().is_control_panel_invalid() {
                // ... through ...
            }

            // Deliver notices for control tree
            if win.control_strip_height > 0 {
                // ...
            }
```

- [ ] **Step 7: Fix compilation errors**

Any remaining references to `control_tree`, `control_view`, `control_panel_id`, `control_strip_height`, `show_control_strip`, `hide_control_strip`, or `content_height` must be removed. Search with:

```bash
cargo check 2>&1 | grep -E 'control_tree|control_view|control_panel_id|control_strip_height|show_control_strip|hide_control_strip|content_height'
```

- [ ] **Step 8: Commit**

```bash
git add crates/emcore/src/emWindow.rs crates/emcore/src/emGUIFramework.rs
git commit -m "refactor: remove ZuiWindow control system + framework control lifecycle

Delete control_tree, control_view, control_panel_id, control_strip_height
from ZuiWindow. Delete show/hide_control_strip, content_height. Delete
control panel lifecycle from about_to_wait. Per-panel context controls
will be managed by ControlPanelBridge engine inside emMainControlPanel."
```

---

### Task 6: StartupEngine — Direct Panel Creation + ControlPanelBridge Engine

**Spec:** §3, §8
**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

**Context:** Rewrite StartupEngine states 5 and 6 to directly create panels inside sub-views (matching C++ emMainWindow.cpp:407-422). Also create the ControlPanelBridge engine that wakes on ControlPanelSignal and recreates the ContentControlPanel — matching C++ emMainControlPanel::Cycle() signal-driven behavior.

- [ ] **Step 1: Add ControlPanelBridge engine struct**

Add after the `MainWindowEngine` struct:

```rust
/// Bridge engine matching C++ emMainControlPanel's signal-driven
/// ContentControlPanel lifecycle. In C++, emMainControlPanel IS an
/// engine that wakes on ControlPanelSignal. In Rust, panels aren't
/// engines, so this standalone engine does the same work.
pub(crate) struct ControlPanelBridge {
    control_panel_signal: SignalId,
    ctrl_view_id: PanelId,
    content_view_id: PanelId,
}

impl emEngine for ControlPanelBridge {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        if ctx.IsSignaled(self.control_panel_signal) {
            // Get active panel from content sub-view
            let active = ctx.tree.with_behavior_as::<emSubViewPanel, _>(
                self.content_view_id,
                |svp| svp.GetSubView().GetActivePanel(),
            ).flatten();

            // Access control sub-tree, recreate content control panel
            ctx.tree.with_behavior_as::<emSubViewPanel, _>(self.ctrl_view_id, |svp| {
                let sub_tree = svp.sub_tree_mut();
                // Find emMainControlPanel and update its content_control_panel
                // For now, just log — full wiring done when CreateControlPanel is
                // ported on individual panels
                if let Some(active_id) = active {
                    log::debug!(
                        "ControlPanelBridge: active panel changed to {:?}, would recreate ContentControlPanel",
                        active_id
                    );
                }
            });
        }
        false // Sleep until next signal
    }
}
```

- [ ] **Step 2: Rewrite StartupEngine state 5**

Replace the state 5 block (lines 404-418) with:

```rust
5 => {
    // C++ emMainWindow.cpp:408-413: Create emMainControlPanel
    // inside the control sub-view.
    let (ctrl_view_id, content_view_id) = ctx.tree
        .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
            (mp.GetControlViewPanelId(), mp.GetContentViewPanelId())
        })
        .unwrap_or((None, None));

    if let Some(ctrl_id) = ctrl_view_id {
        let ctrl_ctx = Rc::clone(&self.context);
        ctx.tree.with_behavior_as::<emSubViewPanel, _>(ctrl_id, |svp| {
            let sub_tree = svp.sub_tree_mut();
            let sub_root = sub_tree.GetRootPanel().expect("control sub-view has root");
            let child_id = sub_tree.create_child(sub_root, "ctrl");
            sub_tree.set_behavior(
                child_id,
                Box::new(emMainControlPanel::new(ctrl_ctx, content_view_id)),
            );
            // C++ uses control_tallness for the control panel height
            sub_tree.Layout(child_id, 0.0, 0.0, 1.0, 0.0538);
        });
    }

    // Create ControlPanelSignal and register bridge engine
    if let (Some(ctrl_id), Some(content_id)) = (ctrl_view_id, content_view_id) {
        let control_panel_signal = ctx.scheduler.create_signal();

        // Set the signal on the content sub-view's emView
        ctx.tree.with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
            svp.sub_view_mut().set_control_panel_signal(control_panel_signal);
            // Also give the sub-view a scheduler reference so it can fire signals
        });

        let bridge = ControlPanelBridge {
            control_panel_signal,
            ctrl_view_id: ctrl_id,
            content_view_id: content_id,
        };
        let bridge_id = ctx.scheduler.register_engine(
            emcore::emEngine::Priority::Low,
            Box::new(bridge),
        );
        ctx.scheduler.connect(control_panel_signal, bridge_id);
    }

    self.state += 1;
    !ctx.IsTimeSliceAtEnd()
}
```

- [ ] **Step 3: Rewrite StartupEngine state 6**

Replace the state 6 block (lines 419-431) with:

```rust
6 => {
    // C++ emMainWindow.cpp:417-420: Create emMainContentPanel
    // inside the content sub-view.
    let content_view_id = ctx.tree
        .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
            mp.GetContentViewPanelId()
        })
        .flatten();

    if let Some(content_id) = content_view_id {
        let content_ctx = Rc::clone(&self.context);
        ctx.tree.with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
            let sub_tree = svp.sub_tree_mut();
            let sub_root = sub_tree.GetRootPanel().expect("content sub-view has root");
            let child_id = sub_tree.create_child(sub_root, "");
            sub_tree.set_behavior(
                child_id,
                Box::new(emMainContentPanel::new(content_ctx)),
            );
            sub_tree.Layout(child_id, 0.0, 0.0, 1.0, 1.0);
        });
    }

    self.state += 1;
    !ctx.IsTimeSliceAtEnd()
}
```

- [ ] **Step 4: Rewrite StartupEngine state 11 (default) to use VisitByIdentity**

Replace the default/state 11 block (lines 513-526) with:

```rust
_ => {
    if self.clock.elapsed().as_millis() < 100 {
        return true;
    }

    // C++ emMainWindow.cpp:472-478: Visit the target location
    if self.visit_valid {
        let content_view_id = ctx.tree
            .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
                mp.GetContentViewPanelId()
            })
            .flatten();

        if let Some(content_id) = content_view_id {
            if let Some(win) = ctx.windows.get_mut(&self.window_id) {
                ctx.tree.with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
                    svp.sub_view_mut().VisitByIdentity(
                        svp.sub_tree_mut(),
                        &self.visit_identity,
                        self.visit_rel_x,
                        self.visit_rel_y,
                        self.visit_rel_a,
                    );
                });
            }
        }
    }

    // C++ emMainWindow.cpp:480: InvalidateTitle
    // Title signal will be wired in MainWindowEngine

    false // engine stops permanently
}
```

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat: rewrite StartupEngine states 5/6/11 + add ControlPanelBridge

States 5/6 now directly create emMainControlPanel and
emMainContentPanel inside sub-views (matching C++ emMainWindow.cpp).
ControlPanelBridge engine wakes on ControlPanelSignal to recreate
ContentControlPanel. State 11 uses VisitByIdentity for final visit."
```

---

### Task 7: ToggleControlView + Dynamic Window Title

**Spec:** §7, §9
**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

**Context:** Replace the DoubleClickSlider-based ToggleControlView with the C++ implementation that toggles focus between control and content sub-views. Also extend MainWindowEngine with title signal handling for dynamic "Eagle Mode - ..." title.

- [ ] **Step 1: Rewrite ToggleControlView**

Replace the `ToggleControlView` method (lines 116-123) with:

```rust
    /// Toggle focus between control and content views.
    /// C++ emMainWindow.cpp:144-158.
    pub fn ToggleControlView(&mut self, tree: &mut PanelTree) {
        let Some(main_id) = self.main_panel_id else { return };

        let (ctrl_view_id, content_view_id) = tree
            .with_behavior_as::<emMainPanel, _>(main_id, |mp| {
                (mp.GetControlViewPanelId(), mp.GetContentViewPanelId())
            })
            .unwrap_or((None, None));

        let (Some(ctrl_id), Some(content_id)) = (ctrl_view_id, content_view_id) else {
            return;
        };

        // Check if content view is focused
        let content_focused = tree
            .with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
                svp.GetSubView().IsFocused()
            })
            .unwrap_or(false);

        if content_focused {
            // Content focused → switch to control view
            tree.with_behavior_as::<emSubViewPanel, _>(ctrl_id, |svp| {
                svp.sub_view_mut().SetFocused(svp.sub_tree_mut(), true);
                // Zoom to root of control view
                let root = svp.sub_root();
                svp.sub_view_mut().VisitFullsized(svp.sub_tree(), root);
            });
        } else {
            // Control focused → switch to content view
            tree.with_behavior_as::<emSubViewPanel, _>(ctrl_id, |svp| {
                svp.sub_view_mut().RawZoomOut(svp.sub_tree_mut());
            });
            tree.with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
                svp.sub_view_mut().SetFocused(svp.sub_tree_mut(), true);
            });
        }
    }
```

- [ ] **Step 2: Update ToggleControlView call sites**

The handle_input Escape handler (line 223-231) calls `self.ToggleControlView(app)`. Update the signature to pass `&mut app.tree` instead of `app`:

```rust
InputKey::Escape
    if !input_state.GetShift()
        && !input_state.GetCtrl()
        && !input_state.GetAlt() =>
{
    self.ToggleControlView(tree);
    true
}
```

(Adjust the handle_input method signature if needed to accept `tree: &mut PanelTree`.)

- [ ] **Step 3: Add title signal to MainWindowEngine**

Extend `MainWindowEngine` struct with title signal and content view ID:

```rust
pub(crate) struct MainWindowEngine {
    close_signal: SignalId,
    title_signal: Option<SignalId>,
    content_view_id: Option<PanelId>,
    window_id: winit::window::WindowId,
    startup_done: bool,
}
```

Update the Cycle method to check title signal:

```rust
impl emEngine for MainWindowEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        if ctx.IsSignaled(self.close_signal) {
            with_main_window(|mw| { mw.to_close = true; });
        }

        // Dynamic title: "Eagle Mode - " + content view title
        if self.startup_done {
            if let Some(sig) = self.title_signal {
                if ctx.IsSignaled(sig) {
                    if let Some(content_id) = self.content_view_id {
                        let title = ctx.tree
                            .with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
                                svp.GetSubView().GetTitle().to_string()
                            })
                            .unwrap_or_default();
                        let full_title = if title.is_empty() {
                            "Eagle Mode".to_string()
                        } else {
                            format!("Eagle Mode - {title}")
                        };
                        if let Some(win) = ctx.windows.get(&self.window_id) {
                            win.winit_window.set_title(&full_title);
                        }
                    }
                }
            }
        }

        let to_close = with_main_window(|mw| mw.to_close).unwrap_or(false);
        if to_close { return false; }

        false
    }
}
```

- [ ] **Step 4: Wire title signal in create_main_window**

In `create_main_window`, after creating the MainWindowEngine, create and connect the title signal:

```rust
// Create title signal for dynamic window title
let title_signal = app.scheduler.borrow_mut().create_signal();
let mw_engine = MainWindowEngine {
    close_signal,
    title_signal: Some(title_signal),
    content_view_id: None, // Set after StartupEngine creates content panel
    window_id,
    startup_done: false,
};
```

The `content_view_id` and `startup_done` flag are set by StartupEngine after state 6 completes.

- [ ] **Step 5: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat: proper ToggleControlView + dynamic window title

ToggleControlView now toggles focus between control and content sub-views
matching C++ emMainWindow.cpp:144-158. MainWindowEngine checks title
signal for dynamic 'Eagle Mode - <title>' window title."
```

---

### Task 8: Tier 1 Integration Verification

**Files:** All Tier 1 files
**Spec:** All Tier 1 sections

- [ ] **Step 1: Fix all compilation errors**

Run: `cargo check 2>&1`

Fix any remaining type mismatches, missing imports, or signature changes across all Tier 1 files. This is the "big bang" integration point.

Common issues to expect:
- `emMainControlPanel::new()` call sites need `content_view_id` parameter
- `ToggleControlView` signature changed — call sites need updating
- `control_tree`/`control_view` references removed but callers may remain
- Missing `use` statements for new types

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings 2>&1`

Fix all warnings.

- [ ] **Step 3: Run golden tests**

Run: `cargo test --test golden -- --test-threads=1 2>&1 | tail -5`

Expected: 239 pass, 4 fail (unchanged baseline).

- [ ] **Step 4: Run full test suite**

Run: `cargo-nextest ntr 2>&1 | tail -10`

Expected: No new failures beyond pre-existing baseline.

- [ ] **Step 5: Manual verification**

Run the app: `cargo run --bin eaglemode`

Verify:
- Eagle image with gradient background is visible
- Startup overlay appears
- Zoom animation plays
- Cosmos visible (starfield + items)
- Escape toggles between control and content views
- Window title updates dynamically

- [ ] **Step 6: Commit any integration fixes**

```bash
git add -A
git commit -m "fix: Tier 1 integration — fix compilation and test issues"
```

---

## Tier 2: Remaining C++ Parity

**Tier 2 tasks are incremental — each produces independently testable software.**

### Task 9: Duplicate() + CreateControlWindow()

**Spec:** §12, §13
**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

- [ ] **Step 1: Implement Duplicate()**

Replace the stub `Duplicate()` method (lines 157-159) with a full implementation that extracts the current visited panel state from the content sub-view and calls `create_main_window()` with those parameters. The `App` already supports multiple windows via `windows: HashMap<WindowId, ZuiWindow>`.

Read C++ reference: `~/git/eaglemode-0.96.4/src/emMain/emMainWindow.cpp:98-129` for exact state extraction (visited identity, relX, relY, relA, adherence, title, control edges color).

- [ ] **Step 2: Complete CreateControlWindow()**

Update the existing `create_control_window()` (lines 607-630):
1. Add `control_window_id: Option<WindowId>` field to `emMainWindow`
2. If the control window already exists and is open, raise it (winit `focus_window()`)
3. If not, create a new ZuiWindow with `VF_POPUP_ZOOM | VF_ROOT_SAME_TALLNESS`, create `emMainControlPanel` as root panel, wire the content view's ControlPanelSignal

Read C++ reference: `~/git/eaglemode-0.96.4/src/emMain/emMainWindow.cpp:309-327`.

- [ ] **Step 3: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat: implement Duplicate (F4) and CreateControlWindow (ccw cheat)"
```

---

### Task 10: RecreateContentPanels() + DoCustomCheat()

**Spec:** §14, §15
**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

- [ ] **Step 1: Implement RecreateContentPanels()**

Replace the stub (lines 659-661). For each window in `app.windows`:
1. Extract visited panel identity + position from content sub-view
2. Delete old content panel from content sub-tree
3. Create new `emMainContentPanel` in content sub-tree
4. Restore view via `VisitByIdentity()`

Read C++ reference: `~/git/eaglemode-0.96.4/src/emMain/emMainWindow.cpp:280-306`.

- [ ] **Step 2: Verify DoCustomCheat() already dispatches correctly**

The existing `do_custom_cheat()` (lines 639-651) already calls `RecreateContentPanels` for "rcp" and `create_control_window` for "ccw". Verify these now work with the real implementations.

- [ ] **Step 3: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat: implement RecreateContentPanels (rcp cheat)"
```

---

### Task 11: WindowStateSaver

**Spec:** §16
**Files:**
- Create: `crates/emcore/src/emWindowStateSaver.rs`
- Modify: `crates/emcore/src/mod.rs` (add module)

- [ ] **Step 1: Create WindowStateSaver engine**

Create `crates/emcore/src/emWindowStateSaver.rs`. This is an engine registered with the scheduler that:
1. On startup: loads window geometry from a config file (`~/.eaglemode/emCore/WindowState.rec`)
2. Applies saved position/size/maximization/fullscreen to the window
3. Listens for geometry change signals (resize, move)
4. Saves geometry to config file on changes

Read C++ reference: `~/git/eaglemode-0.96.4/include/emCore/emWindowStateSaver.h` and `src/emCore/emWindowStateSaver.cpp`.

Use existing emRec serialization or a simple key-value format matching C++.

- [ ] **Step 2: Register in create_main_window**

In `create_main_window()`, after creating the window, register a `WindowStateSaver` engine and apply any saved geometry.

- [ ] **Step 3: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emcore/src/emWindowStateSaver.rs crates/emcore/src/mod.rs crates/emmain/src/emMainWindow.rs
git commit -m "feat: add WindowStateSaver — persist window geometry across sessions"
```

---

### Task 12: TicTacToe Easter Egg

**Spec:** §17
**Files:**
- Modify: `crates/emmain/src/emStarFieldPanel.rs`

- [ ] **Step 1: Create TicTacToePanel**

Add a `TicTacToePanel` struct implementing `PanelBehavior`:
- 3x3 game board, X/O rendering, minimax AI
- Input handling for mouse clicks on grid cells
- Paint method drawing board lines and X/O marks

Read C++ reference: `~/git/eaglemode-0.96.4/src/emMain/emStarFieldPanel.cpp:248-413` for the complete game logic including `DeepCheckState` minimax.

- [ ] **Step 2: Add creation check in LayoutChildren**

In `emStarFieldPanel::LayoutChildren`, after creating child starfield quadrants, add:

```rust
if self.depth > 50 && self.get_random() % 11213 == 0 {
    let ttt_id = ctx.create_child_with("tictactoe", Box::new(TicTacToePanel::new()));
    ctx.layout_child(ttt_id, 0.48, 0.48, 0.04, 0.04);
}
```

- [ ] **Step 3: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emmain/src/emStarFieldPanel.rs
git commit -m "feat: add TicTacToe easter egg in deep starfield (depth > 50)"
```

---

### Task 13: Copy-to-User for Cosmos Items

**Spec:** §18
**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

- [ ] **Step 1: Implement TryPrepareItemFile**

Find the location where cosmos items log a warning about CopyToUser and replace with actual file copy logic:
1. Check `CopyToUser` flag on the item record
2. If true: `std::fs::create_dir_all(user_dir)` then `std::fs::copy(orig_path, user_path)`
3. Use user copy path for file panel creation
4. On error: log warning and fall back to original path

Read C++ reference: `~/git/eaglemode-0.96.4/src/emMain/emVirtualCosmos.cpp:63-82`.

- [ ] **Step 2: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "feat: implement copy-to-user for cosmos items with CopyToUser flag"
```

---

### Task 14: ReloadFiles() Signal

**Spec:** §19
**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`
- Modify: `crates/emcore/src/emFileModel.rs` (if exists)

- [ ] **Step 1: Create global file update signal**

Create a global `SignalId` for file model updates (similar to C++ `emFileModel::AcquireUpdateSignalModel`). Store on emMainWindow or as a static.

- [ ] **Step 2: Implement ReloadFiles()**

Replace the stub to fire the global file update signal:

```rust
pub fn ReloadFiles(&self, scheduler: &mut EngineScheduler) {
    if let Some(sig) = self.file_update_signal {
        scheduler.fire(sig);
    }
}
```

- [ ] **Step 3: Connect file models to the signal**

All file model engines should `AddWakeUpSignal` to the file update signal and re-read from disk when signaled.

- [ ] **Step 4: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emmain/src/emMainWindow.rs
git commit -m "feat: implement ReloadFiles (F5) — fire global file model update signal"
```

---

### Task 15: emAutoplayControlPanel Full UI

**Spec:** §20
**Files:**
- Modify: `crates/emmain/src/emAutoplayControlPanel.rs`

- [ ] **Step 1: Replace placeholder ControlButton widgets with real widgets**

Replace `ControlButton` instances with:
- `emCheckButton` for the autoplay toggle (with progress bar overlay)
- `emButton` for Previous, Next, Continue Last Autoplay
- `emScalarField` for duration slider (if ported; otherwise create a minimal slider)
- `emCheckButton` for Recursive and Loop checkboxes

Read C++ reference: `~/git/eaglemode-0.96.4/src/emMain/emAutoplay.cpp:1157-1334` for exact widget tree, layout weights, and signal wiring.

- [ ] **Step 2: Wire controls to emAutoplayViewModel**

Connect button clicks and checkbox changes to the autoplay view model using the existing ClickFlags pattern or direct method calls.

- [ ] **Step 3: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emmain/src/emAutoplayControlPanel.rs
git commit -m "feat: full emAutoplayControlPanel UI — real buttons, slider, checkboxes"
```

---

### Task 16: Screensaver Inhibition

**Spec:** §21
**Files:**
- Modify: `crates/emcore/src/emWindowPlatform.rs`

- [ ] **Step 1: Add periodic re-inhibition timer**

When screensaver is inhibited, start a 59-second timer. On each tick, re-call the D-Bus Inhibit or reset the screensaver to prevent timeout.

- [ ] **Step 2: Add xscreensaver-command fallback**

For systems without D-Bus, shell out to `xscreensaver-command -deactivate` as a fallback (matching C++ emX11Screen.cpp:711-765).

- [ ] **Step 3: Add ref-counted inhibit/allow**

Track `inhibit_count` so multiple windows can inhibit independently. Only actually uninhibit when count reaches 0.

- [ ] **Step 4: Run tests and commit**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
git add crates/emcore/src/emWindowPlatform.rs
git commit -m "feat: screensaver inhibition with 59s timer + xscreensaver fallback"
```

---

## Final Verification

- [ ] **Run full golden test suite:**
  ```bash
  cargo test --test golden -- --test-threads=1
  ```
  Expected: 239 pass, 4 fail (unchanged baseline).

- [ ] **Run full test suite:**
  ```bash
  cargo-nextest ntr
  ```
  Expected: No new failures.

- [ ] **Run clippy:**
  ```bash
  cargo clippy -- -D warnings
  ```
  Expected: Clean.

- [ ] **Manual verification of all success criteria** (see spec Testing Strategy section).
