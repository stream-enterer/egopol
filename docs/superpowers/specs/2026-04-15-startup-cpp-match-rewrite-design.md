# Startup C++ Match Rewrite Design

**Date:** 2026-04-15
**Status:** Draft
**Supersedes:** `2026-04-15-startup-architecture-rewrite-design.md` (failed implementation)

## Goal

Rewrite the Rust startup and control panel architecture to exactly match C++ Eagle Mode. Eliminate all DIVERGED patterns that affect the startup visual sequence, control panel lifecycle, sub-view notice propagation, and runtime input handling. The app must show: eagle image with gradient background → startup overlay → zoom animation into eagle's eye → cosmos with starfield + items.

## Problem

The previous rewrite (commits b826b33–8a812fa) got the engine architecture right (EngineCtx widening, event loop pumping) but the panel creation was wrong. Instead of the eagle image and cosmos, the app showed grey background with two empty blue rectangles. Root causes:

### Root Cause 1: Sub-tree HandleNotice and run_panel_cycles Never Called

Rust `emSubViewPanel` owns a separate `PanelTree` (sub-tree) for each sub-view. The main framework loop (`about_to_wait`) only calls `HandleNotice()` and `run_panel_cycles()` on the main tree. Sub-trees never get:

- **`HandleNotice()`:** Panels inside sub-views never receive notice delivery → `LayoutChildren` never runs → children never created. A new panel created via `create_child()` receives `INIT_NOTICE_FLAGS` (including `LAYOUT_CHANGED`) automatically, but only if `HandleNotice()` is called on that tree. Without it, panels exist but are never laid out — their children (eagle, cosmos, buttons) are never created. **This is the most likely direct cause of the empty blue rectangles.**

- **`run_panel_cycles()`:** Panel `Cycle()` methods inside sub-views never execute. This means emMainControlPanel's `Cycle()` (which polls ClickFlags and handles button events) never runs.

In C++, this works because `emSubViewPortClass` participates in the main view's update cycle via `emViewPort::RequestUpdate()`, and all panels share one `emScheduler` (since every panel IS an engine).

### Root Cause 2: creation_stage Indirection

C++ StartupEngine directly creates panels in sub-views (`new emMainControlPanel(GetControlView(), ...)`, `new emMainContentPanel(GetContentView(), "")`). Rust gates creation through `advance_creation_stage()` → `LayoutChildren`, which adds an unnecessary layer of indirection.

### Root Cause 3: ZuiWindow's Separate Control System

Rust ZuiWindow has `control_tree`, `control_view`, `control_panel_id`, `control_strip_height` for per-panel context controls. This doesn't exist in C++. In C++, per-panel controls live inside `emMainControlPanel` as `ContentControlPanel`, managed by `RecreateContentControlPanel()` triggered by `ControlPanelSignal`.

### Root Cause 4: emGUIFramework Drives Control Lifecycle

The `about_to_wait()` loop manages control panel creation/destruction each frame. In C++, `emMainControlPanel::Cycle()` handles this via signals.

### Root Cause 5: emMainControlPanel Layout Wrong

C++ top-level layout: child 0 = lMain (weight 11.37, contains general + bookmarks), child 1 = contentControlPanel (weight 21.32). Rust: child 0 = general (11.37), child 1 = bookmarks (21.32). Bookmarks in wrong position, contentControlPanel missing.

## Key Architecture Difference: Panels Are Not Engines

**In C++:** `emPanel : public emEngine`. Every panel IS an engine with `AddWakeUpSignal()`, `IsSignaled()`, `WakeUp()`, and participation in the global `emScheduler`.

**In Rust:** `PanelBehavior` is a trait with `Cycle(&mut PanelCtx) -> bool`. Panels are NOT engines. They have no signal support — no `AddWakeUpSignal`, no `IsSignaled`. Panels use `PanelTree::run_panel_cycles()` (a flat list), not the scheduler. Current workaround for C++ signals: `Rc<Cell<bool>>` flags (ClickFlags pattern in emMainControlPanel).

**Consequence:** Panels cannot directly use `AddWakeUpSignal`/`IsSignaled`. Where C++ uses panel-as-engine signal wiring, Rust registers a dedicated **bridge engine** that wakes on the signal and performs the cross-tree work. The bridge engine uses the scheduler's `SignalId` system identically to C++ — same signal, same wake-up, same action. The only structural difference is the engine is standalone rather than the panel itself.

## Existing Components That Already Work

The audit confirmed these Rust components are correctly ported and don't need changes:

- **emMainContentPanel:** Renders gradient background (0x91ABF2FF blue → 0xE1DDB7FF gold) + 14 eagle polygons procedurally (961 vertices, centered at EAGLE_CX=78450, EAGLE_CY=47690). Creates emVirtualCosmosPanel child at eagle's eye position.
- **emVirtualCosmosPanel:** Loads `.emVcItem` files from `~/.eaglemode/emMain/VcItems/`, creates emStarFieldPanel background + emVirtualCosmosItemPanel for each item (Home, Root, Stocks1, etc.).
- **emStarFieldPanel:** Procedural quadtree starfield with LCG PRNG, 3-tier rendering, max depth 50.
- **emVirtualCosmosItemPanel:** Renders items with border images, lazily creates content panels via file plugin system.
- **emFpPluginList:** Full file panel plugin system — loads `.emFpPlugin` configs, creates panels for `.emStocks`, `.emFileLink`, directories. Cosmos items can load content.
- **emVisitingViewAnimator:** SetGoalFullsized(":"), set_goal_rel(), all working. Resolves identity strings to PanelIds internally.
- **emView:** RawZoomOut(), Visit() working. Visit takes PanelId (animator handles identity resolution).
- **emSubViewPanel:** Owns PanelTree + emView, handles paint delegation via paint_sub_tree(), input forwarding with coordinate transforms, focus propagation, geometry sync.
- **BookmarksModel:** Loads bookmarks, SearchStartLocation() works.
- **emInputHotkey:** Hotkey string conversion from input events.
- **emAutoplayViewModel:** Full autoplay system ported — config, animator, view model, F12 hotkeys (4 variants + mouse X1/X2). Not needed for basic cosmos.
- **All hotkeys:** F4 (Duplicate, stub), Alt+F4 (Close), Shift+Alt+F4 (Quit), F5 (Reload, stub), F11 (Fullscreen), Escape (ToggleControlView, currently via slider), F12 variants (autoplay), bookmark hotkeys (routing done, visit pending).
- **PanelTree notice system:** `create_child()` queues INIT_NOTICE_FLAGS (including LAYOUT_CHANGED) on new panels. `HandleNotice()` loops until all cascading notices drain. `Layout()` queues LAYOUT_CHANGED on children. System is proven correct — just needs to be called on sub-trees.

## Design

Eleven changes. All applied simultaneously (big bang).

### 1. Sub-tree Notice Delivery and Panel Cycling (CRITICAL)

**Problem:** emSubViewPanel owns a `sub_tree: PanelTree` that never gets `HandleNotice()` or `run_panel_cycles()`. Panels inside sub-views are dead — never laid out, never cycled.

**Fix:** Add sub-tree lifecycle management to `emSubViewPanel`. Both `HandleNotice()` and `run_panel_cycles()` must be called on the sub-tree.

**Integration point:** In `emSubViewPanel::Paint()`, before painting:

```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
    if !state.is_viewed() { return; }

    // Drive sub-tree lifecycle (C++ does this via emViewPort::RequestUpdate)
    self.sub_tree.run_panel_cycles();
    self.sub_tree.HandleNotice(state.is_focused(), state.pixel_tallness);

    // Update sub-view (recompute viewing coords)
    self.sub_view.Update(&mut self.sub_tree);

    // Paint sub-tree
    // ...
}
```

**Why Paint:** In C++, sub-view updates happen during the view/viewport update cycle which runs each frame for viewed panels. Rust's Paint is the per-frame entry point for viewed panels. HandleNotice must run before paint_sub_tree so LayoutChildren creates all children before rendering. run_panel_cycles must run so panel Cycle() methods (emMainControlPanel button handling, etc.) execute.

**Alternative consideration:** Could also add a new PanelBehavior method (`update_sub_trees()`) called from the framework loop. But Paint is simpler and matches C++ where the viewport update happens during the paint cycle.

**Files:** `crates/emcore/src/emSubViewPanel.rs`

### 2. emMainPanel: Eliminate creation_stage

**Delete:**
- `creation_stage: u8` field (line 330)
- `advance_creation_stage()` method (lines 535-539)
- `creation_stage()` getter (lines 542-544)
- `control_panel_created: Option<PanelId>` field (line 318)
- `content_panel_created: Option<PanelId>` field (line 319)
- Creation-gated blocks in `LayoutChildren` (lines 778-814)
- Tests: `test_creation_stage_initial`, `test_advance_creation_stage`, `test_advance_creation_stage_saturates_at_2`

**Keep:** Sub-view panel creation in `LayoutChildren` (lines 749-776) — ControlViewPanel, ContentViewPanel, SliderPanel, StartupOverlayPanel are still created here on first layout. This matches C++ where `emMainPanel` constructor creates these (emMainPanel.cpp:39-42).

**Add:** Public methods to expose sub-view panel IDs:

```rust
pub fn GetControlViewPanelId(&self) -> Option<PanelId> {
    self.control_view_panel
}
pub fn GetContentViewPanelId(&self) -> Option<PanelId> {
    self.content_view_panel
}
```

**LayoutChildren becomes pure positioning:** After initial child creation, it only calls `update_coordinates()` and positions children. Matches C++ emMainPanel.cpp:225-231.

**Files:** `crates/emmain/src/emMainPanel.rs`

### 3. StartupEngine: Direct Panel Creation in Sub-views

Rewrite states 5 and 6 to directly create panels inside sub-views, matching C++ emMainWindow.cpp:407-422.

**State 5 — Create emMainControlPanel:**

```rust
// C++ emMainWindow.cpp:408-413:
//   ControlPanel = new emMainControlPanel(
//       MainPanel->GetControlView(), "ctrl", *this, MainPanel->GetContentView()
//   );
let (ctrl_view_id, content_view_id) = ctx.tree.with_behavior_as::<emMainPanel, _>(
    self.main_panel_id, |mp| (mp.GetControlViewPanelId(), mp.GetContentViewPanelId())
).unwrap_or((None, None));

if let Some(ctrl_id) = ctrl_view_id {
    ctx.tree.with_behavior_as::<emSubViewPanel, _>(ctrl_id, |svp| {
        let sub_tree = svp.sub_tree_mut();
        let sub_root = sub_tree.GetRootPanel().expect("sub-view has root");
        let child_id = sub_tree.create_child(sub_root, "ctrl");
        sub_tree.set_behavior(child_id, Box::new(
            emMainControlPanel::new(ctx_clone, content_view_id)
        ));
        sub_tree.Layout(child_id, 0.0, 0.0, 1.0, control_tallness);
    });
}
```

**State 6 — Create emMainContentPanel:**

```rust
// C++ emMainWindow.cpp:417-420:
//   ContentPanel = new emMainContentPanel(MainPanel->GetContentView(), "");
if let Some(content_id) = content_view_id {
    ctx.tree.with_behavior_as::<emSubViewPanel, _>(content_id, |svp| {
        let sub_tree = svp.sub_tree_mut();
        let sub_root = sub_tree.GetRootPanel().expect("sub-view has root");
        let child_id = sub_tree.create_child(sub_root, "");
        sub_tree.set_behavior(child_id, Box::new(
            emMainContentPanel::new(content_ctx)
        ));
        sub_tree.Layout(child_id, 0.0, 0.0, 1.0, 1.0);
    });
}
```

No `advance_creation_stage()` calls. Sub-tree `HandleNotice()` (from Section 1) delivers LAYOUT_CHANGED to newly created panels, triggering their `LayoutChildren`.

**Files:** `crates/emmain/src/emMainWindow.rs`

### 4. ZuiWindow: Remove Control System

**Delete from ZuiWindow struct:**
- `control_tree: PanelTree` (line 64)
- `control_view: emView` (line 66)
- `control_panel_id: Option<PanelId>` (line 68)
- `control_strip_height: u32` (line 70)

**Delete methods:**
- `show_control_strip()` (lines 221-234)
- `hide_control_strip()` (lines 237-245)
- `content_height()` (if it exists)

**Update `create()`:** Remove control_tree/control_view initialization.

**Update `render()`:** Remove second paint pass for control strip. Only `self.view.Paint(tree, ...)`.

**Update `resize()`:** Full height goes to main view. No control strip subtraction.

**Files:** `crates/emcore/src/emWindow.rs`

### 5. emGUIFramework: Remove Control Panel Lifecycle

**Delete from `about_to_wait()`:**
- The entire block at lines 408-445: `is_control_panel_invalid()` check, `create_control_panel_in()`, `show_control_strip()`/`hide_control_strip()`, `HandleNotice` for control_tree, `control_view.update()`

**Files:** `crates/emcore/src/emGUIFramework.rs`

### 6. emMainControlPanel: Restructure Layout + ContentControlPanel Lifecycle

Match C++ emMainControlPanel (emMainControlPanel.h:39-76, emMainControlPanel.cpp:100-324).

#### 6a. Constructor Change

C++ constructor: `emMainControlPanel(ParentArg parent, const emString & name, emMainWindow & mainWin, emView & contentView)`

New Rust constructor: `emMainControlPanel::new(ctx: Rc<emContext>, content_view_id: Option<PanelId>)`

The `content_view_id` is the PanelId of the content emSubViewPanel in the **main** tree. Needed to access the content sub-view's active panel for creating context-sensitive controls.

C++ holds `emMainWindow & MainWin` for button actions. Rust already uses `with_main_window()` thread_local. No change needed for button callbacks.

#### 6b. Layout Restructuring

**C++ layout** (emMainControlPanel.cpp:100-228):
```
emMainControlPanel (emLinearGroup, top-level)
├── child 0: lMain (emLinearLayout, weight 11.37)
│   ├── child 0: lAbtCfgCmd (about + config + commands, weight 4.71)
│   └── child 1: bookmarks (emBookmarksPanel, weight 6.5)
└── child 1: contentControlPanel (weight 21.32) ← DYNAMIC per-panel controls
```

**Current Rust layout (wrong):**
```
emMainControlPanel (top-level)
├── child 0: general (weight 11.37) ← about + commands only
└── child 1: bookmarks (weight 21.32) ← WRONG: should be inside general
```

**New Rust layout (matching C++):**
```
emMainControlPanel (top-level)
├── child 0: lMain (weight 11.37)
│   ├── child 0: general/lAbtCfgCmd (weight 4.71)
│   └── child 1: bookmarks (weight 6.5)
└── child 1: contentControlPanel (weight 21.32) ← DYNAMIC
```

Create a new `LMainPanel` wrapper that contains both `GeneralPanel` and `emBookmarksPanel` as children.

#### 6c. ContentControlPanel Lifecycle

**Add to emMainControlPanel struct:**
```rust
content_control_panel: Option<PanelId>,
```

In C++, `emMainControlPanel` owns the `ContentControlPanel` lifecycle entirely: it calls `AddWakeUpSignal(ContentView.GetControlPanelSignal())`, checks `IsSignaled()` in `Cycle()`, and calls `RecreateContentControlPanel()`. Since Rust panels aren't engines, this lifecycle is handled by a dedicated **bridge engine** (see Section 8) that matches C++ behavior: wakes on `ControlPanelSignal`, accesses both sub-trees, and performs the recreation. emMainControlPanel just stores the resulting `content_control_panel` ID for layout purposes.

#### 6d. Escape Key Handling

C++ emMainControlPanel::Input (emMainControlPanel.cpp:296-314):
```cpp
case EM_KEY_ESCAPE:
    if (state.IsNoMod()) {
        MainWin.ToggleControlView();
        event.Eat();
    }
```

Add to Rust emMainControlPanel::Input: on Escape with no modifiers, call ToggleControlView via `with_main_window()`.

**Files:** `crates/emmain/src/emMainControlPanel.rs`

### 7. ToggleControlView

Match C++ emMainWindow.cpp:144-158.

**Current Rust (DIVERGED):** emMainWindow::handle_input Escape → calls `DoubleClickSlider()` on emMainPanel slider (lines 109-115).

**C++ exact implementation:**
```cpp
if (MainPanel && ControlPanel) {
    if (MainPanel->GetContentView().IsFocused()) {
        MainPanel->GetControlView().Focus();
        MainPanel->GetControlView().AbortActiveAnimator();
        MainPanel->GetControlView().RawVisitFullsized(ControlPanel);
        MainPanel->GetControlView().SetActivePanel(ControlPanel, false);
    } else {
        MainPanel->GetControlView().ZoomOut();
        MainPanel->GetContentView().Focus();
    }
}
```

**Rust implementation:** Access the sub-view panels via `tree.with_behavior_as::<emSubViewPanel>`, check which sub-view is focused, toggle focus and navigation accordingly.

**Triggers (matching C++):**
- Escape (no modifiers) in emMainWindow::handle_input (lines 224-231)
- Escape (no modifiers) in emMainControlPanel::Input (new, Section 6d)
- **Remove:** F11 toggle (C++ F11 is fullscreen, not control view toggle — Rust already has F11 → ToggleFullscreen correctly)

**Files:** `crates/emmain/src/emMainWindow.rs`, `crates/emmain/src/emMainControlPanel.rs`

### 8. ControlPanelBridge Engine — Signal-Driven ContentControlPanel

C++ `emMainControlPanel` IS an engine (inherits from `emPanel : emEngine`). It calls `AddWakeUpSignal(ContentView.GetControlPanelSignal())` and checks `IsSignaled()` in `Cycle()`. In Rust, panels aren't engines — but we can match C++ behavior exactly by registering a dedicated **bridge engine** that wakes on the same signal and does the same work.

**New: `ControlPanelBridge` engine** (registered with the scheduler like any other engine):

```rust
pub(crate) struct ControlPanelBridge {
    control_panel_signal: SignalId,
    main_panel_id: PanelId,
    ctrl_view_id: PanelId,      // Control sub-view panel in main tree
    content_view_id: PanelId,   // Content sub-view panel in main tree
}

impl emEngine for ControlPanelBridge {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        if ctx.IsSignaled(self.control_panel_signal) {
            // 1. Get active panel from content sub-view
            let active = ctx.tree.with_behavior_as::<emSubViewPanel, _>(
                self.content_view_id, |svp| {
                    svp.GetSubView().GetActivePanel()
                }
            ).flatten();

            // 2. Access control sub-tree, find emMainControlPanel, recreate child
            ctx.tree.with_behavior_as::<emSubViewPanel, _>(self.ctrl_view_id, |svp| {
                let sub_tree = svp.sub_tree_mut();
                // Find emMainControlPanel in sub-tree, delete old content control
                // panel, create new one via active panel's CreateControlPanel
                // ... (uses existing create_control_panel_in infrastructure)
            });
        }
        false // Sleep until next signal
    }
}
```

**Registration:** In StartupEngine state 5 (after creating emMainControlPanel), register the bridge engine:

```rust
let bridge = ControlPanelBridge {
    control_panel_signal,
    main_panel_id: self.main_panel_id,
    ctrl_view_id,
    content_view_id,
};
let bridge_id = ctx.scheduler.register_engine(Priority::Low, Box::new(bridge));
ctx.scheduler.connect(control_panel_signal, bridge_id);
```

**Why this matches C++:**
- Signal fires when content view's active panel changes (same trigger)
- Engine wakes and performs RecreateContentControlPanel (same action)
- Uses the scheduler's signal system with clock-based IsSignaled (same mechanism)
- No framework-loop polling, no shared flags, no DIVERGED comment needed

**Cross-tree access:** The engine has `ctx.tree` (the main tree) and can access both sub-trees via `with_behavior_as::<emSubViewPanel>`. This is natural since the engine runs at the scheduler level, not inside a sub-tree.

**Files:** `crates/emmain/src/emMainWindow.rs` (or new `emMainControlPanelBridge.rs`)

### 9. GetTitle() — Dynamic Window Title

C++ emMainWindow::GetTitle() (emMainWindow.cpp:87-95):
```cpp
if (MainPanel && !StartupEngine) {
    return "Eagle Mode - " + MainPanel->GetContentView().GetTitle();
} else {
    return "Eagle Mode";
}
```

C++ emMainWindow::Cycle() (emMainWindow.cpp:176-178): checks title signal → `InvalidateTitle()`.

**Current Rust:** MainWindowEngine only handles close signal. Title is static.

**Fix:** MainWindowEngine (already an engine in the scheduler) adds a wake-up signal for the content view's title signal. Matching C++ exactly:

1. Add `title_signal: SignalId` to the content sub-view's emView (see Section 10)
2. MainWindowEngine connects to this signal via `scheduler.connect(title_signal, engine_id)`
3. In Cycle(), `ctx.IsSignaled(title_signal)` → update window title to "Eagle Mode - " + content view title
4. Access title via `ctx.tree.with_behavior_as::<emSubViewPanel>(content_view_id, |svp| svp.GetSubView().GetTitle())`

**Files:** `crates/emmain/src/emMainWindow.rs`

### 10. emView: Add Real SignalIds for ControlPanelSignal and TitleSignal

C++ `emView` has `ControlPanelSignal` (emView.h:682) and `TitleSignal` that fire when:
- Active panel changes → fires ControlPanelSignal (emView.cpp:308)
- `InvalidateControlPanel()` called → fires ControlPanelSignal
- Title changes → fires TitleSignal

Rust already has `control_panel_invalid: bool` flag. Add real `SignalId` fields so bridge engines can use the scheduler's signal system:

**Add to emView:**
```rust
control_panel_signal: Option<SignalId>,
title_signal: Option<SignalId>,
```

**Add methods:**
```rust
pub fn set_control_panel_signal(&mut self, signal: SignalId) {
    self.control_panel_signal = Some(signal);
}
pub fn GetControlPanelSignal(&self) -> Option<SignalId> {
    self.control_panel_signal
}
pub fn set_title_signal(&mut self, signal: SignalId) {
    self.title_signal = Some(signal);
}
pub fn GetTitleSignal(&self) -> Option<SignalId> {
    self.title_signal
}
```

**Fire signals:** In `SetActivePanel()` and `InvalidateControlPanel()`, fire the signal via the scheduler. Since emView doesn't hold a scheduler reference, the signals are fired from the framework or engine layer after the flag is set. Specifically:
- When `control_panel_invalid` becomes true, the ControlPanelBridge engine needs to wake. The signal is fired by the emSubViewPanel's notice/update cycle, or by the framework after calling `view.Update()`.
- Alternatively: store an `Rc<RefCell<EngineScheduler>>` reference on emView (sub-views are created during startup when the scheduler is available). This lets emView fire signals directly, exactly matching C++.

**Recommended:** Give emView an optional scheduler reference so it can fire signals directly:
```rust
scheduler: Option<Rc<RefCell<EngineScheduler>>>,
```
Set during sub-view creation in emMainPanel's LayoutChildren. This matches C++ where emView holds a reference to its context (which includes the scheduler).

**Signal allocation:** Signals are created during StartupEngine state 5 (before creating emMainControlPanel) via `scheduler.create_signal()`. The signal IDs are passed to:
1. The content sub-view's emView (via `set_control_panel_signal()`)
2. The ControlPanelBridge engine (for `connect()`)
3. MainWindowEngine (for title signal `connect()`)

**Files:** `crates/emcore/src/emView.rs`

### 11. emView: Add VisitByIdentity for State 11

C++ StartupEngine state 11 calls `ContentView.Visit(identity, rel_x, rel_y, rel_a, adherent, subject)` with a string identity. Rust's `emView::Visit()` takes a PanelId.

**Add:** `emView::VisitByIdentity(tree, identity, rel_x, rel_y, rel_a)` that:
1. Uses existing `DecodeIdentity(identity)` to split the identity into path segments
2. Walks the tree to find the panel matching the identity path
3. Calls `Visit(panel_id, rel_x, rel_y, rel_a)`

This eliminates the DIVERGED comment in StartupEngine state 11 and matches C++ exactly. Also useful for bookmark hotkey navigation (emMainWindow handle_input lines 246-264).

**Files:** `crates/emcore/src/emView.rs`

## What This Preserves

- **EngineCtx widening** (commits 1-2): Engines have full tree+windows access.
- **Event loop pumping** (commit 2): `request_redraw()` when engines are awake.
- **MainWindowEngine** (commit 5): Close signal handling. Extended with title.
- **BookmarksModel integration** (commit 4): Loading and hotkey handling.
- **Input blocking during startup** (commit 3): `startup_engine_id.is_some()` check.
- **emInputHotkey.rs** (commit 4): Hotkey conversion.
- **All rendering components:** emMainContentPanel, emVirtualCosmosPanel, emStarFieldPanel, emVirtualCosmosItemPanel, emFpPluginList.
- **Autoplay system:** Fully ported, F12 hotkeys work.
- **All hotkeys:** F4, Alt+F4, Shift+Alt+F4, F5, F11, F12 variants, bookmark hotkeys.

## What This Deletes

- `creation_stage` mechanism in emMainPanel
- `control_tree`, `control_view`, `control_panel_id`, `control_strip_height` from ZuiWindow
- Control panel lifecycle from `about_to_wait()` (replaced by ControlPanelBridge engine)
- `show_control_strip()` / `hide_control_strip()` from ZuiWindow
- `advance_creation_stage()` and related tests
- `DoubleClickSlider()` as ToggleControlView mechanism

## Out of Scope

These C++ features exist but are not needed for cosmos to work. Explicitly deferred:

- **Duplicate() (F4):** Window duplication. Already stubbed with log message.
- **CreateControlWindow():** Detached control window popup ("ccw" cheat).
- **DoCustomCheat():** Debug cheat codes ("rcp", "ccw").
- **RecreateContentPanels():** Content panel recreation across all windows.
- **WindowStateSaver:** Persistent window geometry save/restore.
- **emStarFieldPanel TicTacToe easter egg:** Nested at depth > 50.
- **Copy-to-user for cosmos items:** `.emVcItem` copy to user config dir.
- **ReloadFiles() (F5):** Signal-based file reload. Already stubbed.
- **emAutoplayControlPanel full UI:** Uses placeholder ControlButton widgets instead of full emToolkit. Functional but simplified.
- **Screensaver inhibition during autoplay:** Flags present but no D-Bus/X11 calls.

## Blast Radius

| File | Change | Complexity |
|------|--------|------------|
| `emSubViewPanel.rs` | Add HandleNotice + run_panel_cycles to sub-tree in Paint | Medium — core fix |
| `emMainPanel.rs` | Delete creation_stage, simplify LayoutChildren, add sub-view ID getters | Low — deletion |
| `emMainWindow.rs` | Rewrite StartupEngine states 5-6, add ToggleControlView, extend MainWindowEngine for title | High — multi-concern |
| `emMainControlPanel.rs` | Restructure layout (bookmarks inside lMain, add contentControlPanel slot), add Escape handling | High — layout rework |
| `emWindow.rs` | Remove control_tree/control_view/control_strip_height, simplify render/resize | Medium — deletion |
| `emGUIFramework.rs` | Delete ZuiWindow control panel lifecycle from about_to_wait | Low — deletion |
| `emView.rs` | Add real SignalIds (ControlPanelSignal, TitleSignal), optional scheduler ref, fire signals on active panel change | Medium |
| `emMainWindow.rs` (or new file) | ControlPanelBridge engine — signal-driven ContentControlPanel lifecycle | Medium — new engine |

## Testing Strategy

- Golden tests: 239 pass, 4 fail baseline — no new failures
- Full suite: no new failures
- `cargo clippy -- -D warnings` clean
- Manual verification:
  - App launches, eagle image with gradient visible
  - Startup overlay appears and covers eagle
  - Zoom animation plays (zoom to ":", wait, zoom to start location)
  - After overlay clears: cosmos visible (black starfield + colored stars)
  - Cosmos items visible (Home, Root, Stocks1 with borders and titles)
  - Zooming into items shows content (Stocks data, file listings)
  - Control panel visible when control view focused (bookmarks, buttons)
  - Escape toggles between control and content views
  - F11 toggles fullscreen
  - Per-panel context controls appear when different content panels are focused
  - Input blocked during startup animation
  - Window title shows "Eagle Mode - " + current panel title after startup
  - Buttons work: Close (Alt+F4), Fullscreen (F11)
  - Autoplay works (F12 variants)
  - Bookmark hotkeys navigate (if configured)

## Success Criteria

1. Runtime rendering matches C++ Eagle Mode startup visual sequence
2. Eagle image visible (gradient + 14 polygons)
3. Cosmos visible after zoom (starfield + items with content loading)
4. Sub-tree notice delivery working (panels inside sub-views get LayoutChildren + Cycle)
5. No creation_stage mechanism remains
6. No control_tree/control_view on ZuiWindow
7. Per-panel context controls via signal-driven ControlPanelBridge engine
8. ToggleControlView works with Escape
9. emMainControlPanel layout matches C++ (lMain with bookmarks, contentControlPanel slot)
10. Dynamic window title: "Eagle Mode - " + content title
11. All existing tests pass (golden + full suite)
12. No new clippy warnings
