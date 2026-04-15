# Startup Architecture Rewrite Design

**Date:** 2026-04-15
**Status:** Approved

## Goal

Rewrite the Rust startup architecture to match C++ Eagle Mode's design: engines have full access to the application context during `Cycle()`, and `StartupEngine` directly creates panels and drives animations — no shared-state IPC, no deferred consumption.

## Problem

The Rust port broke the C++ engine design by making `EngineCtx` narrow (scheduler-only). In C++, engines hold references to whatever they need and act on it directly during `Cycle()`. The Rust `StartupEngine` can only update a shared counter; a separate `cycle_startup()` function was supposed to consume it — but was never wired in. The app launches but shows "Loading..." forever.

The previous fix attempt (adding a per-frame callback) compiled but had no effect because `ControlFlow::Wait` doesn't pump the event loop when no OS events arrive.

## Design

Six changes, in dependency order.

### 1. Widen EngineCtx to include `&mut PanelTree`

```rust
pub struct EngineCtx<'a> {
    pub(crate) engine_id: EngineId,
    pub(crate) scheduler: &'a mut EngineCtxInner,
    pub tree: &'a mut PanelTree,
    pub windows: &'a mut HashMap<WindowId, ZuiWindow>,
}
```

`DoTimeSlice` signature becomes:
```rust
pub fn DoTimeSlice(&mut self, tree: &mut PanelTree, windows: &mut HashMap<WindowId, ZuiWindow>)
```

Call site in `about_to_wait()`:
```rust
self.scheduler.borrow_mut().DoTimeSlice(&mut self.tree, &mut self.windows);
```

This works because `scheduler` is behind `Rc<RefCell<>>` — borrowing it doesn't conflict with `&mut self.tree` or `&mut self.windows` (all separate fields of `App`).

All ~20 engine `Cycle` implementations get the wider `EngineCtx`. Most ignore `ctx.tree` and `ctx.windows` — mechanical signature change.

**Why windows too:** C++ `StartupEngine` accesses the view via `MainWin.MainPanel->GetContentView()`. The view is owned by `ZuiWindow`. The startup animation (`emVisitingViewAnimator`) is set as `win.active_animator` and needs `win.view_mut()` for `animate()` calls. Without window access, the engine cannot create, configure, or check the animator.

**Files:** `emEngine.rs`, `emScheduler.rs`, `emGUIFramework.rs`, all `impl emEngine` sites.

### 2. Make emMainWindow an engine

C++ `emMainWindow` derives from `emEngine` (emMainWindow.cpp:174-190). Its `Cycle()`:
- Checks title signal → `InvalidateTitle()`
- Checks close signal → `Close()`
- If `ToClose` → self-delete

Rust equivalent: register a `MainWindowEngine` with the scheduler that holds references to the main window's signal IDs and panel IDs. Each cycle it checks signals via `ctx.IsSignaled()` and acts on them via `ctx.tree`.

This replaces the current ad-hoc close handling. Title updates become dynamic (matching C++ `GetTitle()` which returns `"Eagle Mode - " + ContentView.GetTitle()`).

**Files:** `emMainWindow.rs`

### 3. Rewrite StartupEngine to directly manipulate panels

Match C++ `StartupEngineClass::Cycle()` (emMainWindow.cpp:362-485) state-by-state. The engine holds `main_panel_id: PanelId`, visit parameters, a clock, and an `emVisitingViewAnimator`.

| State | C++ action | Rust action |
|-------|-----------|-------------|
| 0-2 | No-op advance | Same |
| 3 | Create MainPanel, SetStartupOverlay(true), acquire AutoplayViewModel, set config path | MainPanel already exists (created in `create_main_window`). Call `SetStartupOverlay(true)` via `ctx.tree`. Set autoplay config path. |
| 4 | Acquire BookmarksModel, SearchStartLocation, populate visit params | Load BookmarksModel, call `SearchStartLocation()`, populate visit fields on self |
| 5 | Create ControlPanel (`advance_creation_stage`) | `ctx.tree.with_behavior_as::<emMainPanel, _>(id, \|mp\| mp.advance_creation_stage())` |
| 6 | Create ContentPanel (`advance_creation_stage`) | Same |
| 7 | Create emVisitingViewAnimator, SetGoalFullsized(":"), Activate | Create animator, configure, activate. Record clock. |
| 8 | Wait 2s or until animation inactive | Poll `clock.elapsed() < 2s && animator.IsActive()` |
| 9 | Deactivate animator, SetGoal to visit target, reactivate | Use visit params from state 4 |
| 10 | Wait 2s, Reset animator, RawZoomOut, SetActivePanel, SetStartupOverlay(false) | Same via `ctx.tree` and view methods |
| 11 | Wait 100ms, Visit() to target, InvalidateTitle, delete self | Visit via view, remove engine from scheduler |

Delete: `StartupState` struct, `cycle_startup()`, `startup_state` field on `emMainWindow`, all shared-state IPC.

The engine stores a `WindowId` and accesses the view via `ctx.windows.get_mut(&window_id)`. This matches C++ where the engine has `MainWin&` — full access to the window, its view, and its animator slot.

For the startup animation:
- State 7: Create `emVisitingViewAnimator`, call `SetGoalFullsized(":")`, set as `win.active_animator`
- State 8: Check `win.active_animator` is still active via `IsActive()`
- State 9: Take animator back, call `Deactivate()`, `SetGoal(visit_params)`, `Activate()`, put back
- State 10: Take animator, `Reset()`. Call `win.view_mut().RawZoomOut(tree)`, `SetActivePanel()`, and `SetStartupOverlay(false)` via tree
- The existing `about_to_wait()` code already ticks `win.active_animator` every frame (emGUIFramework.rs:343-348)

**Files:** `emMainWindow.rs`

### 4. Block input during startup

C++ `emMainWindow::Input()` (emMainWindow.cpp:193-201):
```cpp
if (StartupEngine) {
    event.Eat();
    emWindow::Input(event, state);
    return;
}
```

Add equivalent check in the Rust input dispatch path: if startup engine is active, eat the event and return.

**Files:** `emMainWindow.rs` or input dispatch in `emGUIFramework.rs`

### 5. BookmarksModel integration

**During startup (state 4):** Load `emBookmarksModel`, call `SearchStartLocation()`. If a start bookmark exists, populate the engine's visit parameters (identity, rel_x, rel_y, rel_a, adherent, subject).

**During input (post-startup):** In `emMainWindow::Input`, call `BookmarksModel->SearchBookmarkByHotkey()` for unhandled key events. If a bookmark matches, call `Visit()` on the content view.

**Files:** `emMainWindow.rs`, `emBookmarks.rs` (if `SearchStartLocation` or `SearchBookmarkByHotkey` need implementing)

### 6. Event loop pumping

After `DoTimeSlice()` in `about_to_wait()`, check if any engines are awake in the scheduler. If so, call `request_redraw()` on all windows to ensure `about_to_wait()` fires again next frame.

This gives continuous cycling when engines are active (startup, animations, fetchers) and idle behavior when all engines sleep. Matches C++ behavior where `emStandardScheduler::Run()` loops at 10ms unconditionally.

**Files:** `emGUIFramework.rs`, `emScheduler.rs` (expose `has_awake_engines()` query)

## What This Does NOT Include

- **Event loop cadence matching** (C++ 10ms vs Rust vsync): Functionally irrelevant. Not worth fighting winit.
- **Duplicate() (F4 no-mod)**: Window duplication is a feature, not startup wiring. Out of scope.
- **CreateControlWindow()**: Detached control window. Already noted as a DIVERGED in emMainWindow.rs. Out of scope.
- **DoCustomCheat()**: Debug cheat codes. Out of scope.
- **RecreateContentPanels()**: Content panel recreation across all windows. Out of scope.
- **WindowStateSaver**: Persistent window geometry. Out of scope.

## Blast Radius

| File | Change |
|------|--------|
| `emEngine.rs` | Trait signature: add `&mut PanelTree` to `EngineCtx` |
| `emScheduler.rs` | `DoTimeSlice` signature + EngineCtx construction + `has_awake_engines()` |
| `emGUIFramework.rs` | Call site change + event loop pumping logic |
| `emMainWindow.rs` | Major rewrite: engine registration, StartupEngine rewrite, input blocking, delete shared-state IPC |
| `emBookmarks.rs` | `SearchStartLocation()` and `SearchBookmarkByHotkey()` if not implemented |
| `emStocksPricesFetcher.rs` | Mechanical `Cycle` signature update |
| `emPriSchedAgent.rs` | Mechanical `Cycle` signature update |
| `emMiniIpc.rs` | Mechanical `Cycle` signature update |
| ~16 test files | Mechanical `Cycle` signature updates |
| `signal_timer_demo.rs` | Mechanical `Cycle` signature update |

## Testing Strategy

- Golden tests remain the authority: 239 pass, 4 fail baseline. No new failures.
- Existing scheduler unit/integration tests updated for new signature.
- Manual verification: app launches, starfield visible, 3 cosmos items appear, overlay clears, zoom animation plays.
- `cargo clippy -- -D warnings` clean.

## Success Criteria

- App launches and shows cosmos (starfield + Home/Root/Stocks1 items)
- Startup overlay appears then clears after ~4s animation
- Zoom animation plays (zoom to ":", wait, zoom to start location)
- Input blocked during startup
- Control panel with bookmarks visible after startup
- Bookmark hotkeys navigate to locations
- Golden tests: 239 pass, 4 fail (unchanged)
- No new clippy warnings
