# Startup Architecture Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the engine/scheduler architecture so engines have full access to PanelTree and windows during Cycle(), then rewrite StartupEngine to directly create panels and drive animations — matching C++ Eagle Mode.

**Architecture:** Widen `EngineCtx` to carry `&mut PanelTree` and `&mut HashMap<WindowId, ZuiWindow>`. Rewrite `StartupEngine` to manipulate panels directly in `Cycle()` instead of updating shared state. Add event loop pumping so `about_to_wait()` fires continuously when engines are awake.

**Tech Stack:** Rust, winit 0.30, wgpu, slotmap

---

### Task 1: Widen EngineCtx and update all Cycle signatures

This is one atomic change: the trait signature, the scheduler, the call site, and ALL engine implementations must update together. The change is mechanical for all engines except StartupEngine (which is rewritten in Task 3).

**Files:**
- Modify: `crates/emcore/src/emEngine.rs`
- Modify: `crates/emcore/src/emScheduler.rs`
- Modify: `crates/emcore/src/emGUIFramework.rs`
- Modify: `crates/emcore/src/emPriSchedAgent.rs`
- Modify: `crates/emcore/src/emMiniIpc.rs`
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs`
- Modify: `crates/emcore/src/emFileModel.rs` (test only)
- Modify: `crates/emmain/src/emMainWindow.rs`
- Modify: `crates/eaglemode/tests/unit/scheduler.rs`
- Modify: `crates/eaglemode/tests/integration/lifecycle.rs`
- Modify: `crates/eaglemode/tests/integration/signals.rs`
- Modify: `crates/eaglemode/tests/golden/scheduler.rs`
- Modify: `crates/eaglemode/tests/behavioral/mini_ipc.rs`
- Modify: `crates/eaglemode/tests/behavioral/pri_sched_agent.rs`
- Modify: `crates/eaglemode/tests/support/mod.rs`
- Modify: `crates/eaglemode/tests/support/pipeline.rs`
- Modify: `examples/signal_timer_demo.rs`

- [ ] **Step 1: Update EngineCtx and emEngine trait**

In `crates/emcore/src/emEngine.rs`, add `tree` and `windows` fields to `EngineCtx`:

```rust
use std::collections::HashMap;
use winit::window::WindowId;
use crate::emPanelTree::PanelTree;
use crate::emWindow::ZuiWindow;

/// Context passed to `emEngine::Cycle()`, providing scheduler operations
/// and access to the panel tree and windows.
///
/// This matches the C++ design where engines have full access to the
/// application context — not just the scheduler.
pub struct EngineCtx<'a> {
    /// The ID of the engine currently being cycled.
    pub(crate) engine_id: EngineId,
    pub(crate) scheduler: &'a mut EngineCtxInner,
    /// The panel tree. Engines can create, modify, and query panels.
    pub tree: &'a mut PanelTree,
    /// All open windows, keyed by winit WindowId.
    pub windows: &'a mut HashMap<WindowId, ZuiWindow>,
}
```

The `emEngine` trait signature does NOT change — `Cycle` already takes `&mut EngineCtx<'_>`.

- [ ] **Step 2: Update DoTimeSlice to accept tree and windows**

In `crates/emcore/src/emScheduler.rs`, change the `DoTimeSlice` signature and pass the new fields through to `EngineCtx`:

```rust
pub fn DoTimeSlice(
    &mut self,
    tree: &mut PanelTree,
    windows: &mut HashMap<WindowId, ZuiWindow>,
) {
    // ... existing timer phase unchanged ...

    // ... existing scheduling loop, but change EngineCtx construction:
    let stay_awake = {
        let mut ctx = EngineCtx {
            engine_id,
            scheduler: &mut self.inner,
            tree,
            windows,
        };
        behavior.Cycle(&mut ctx)
    };

    // ... rest unchanged ...
}
```

Add the necessary imports at the top of `emScheduler.rs`:

```rust
use std::collections::HashMap;
use winit::window::WindowId;
use super::emPanelTree::PanelTree;
use super::emWindow::ZuiWindow;
```

Also update `run()` (line 385-390) which calls `DoTimeSlice()`. Since `run()` is the blocking scheduler loop (unused in the GUI path), it needs dummy tree/windows:

```rust
pub fn run(&mut self) {
    let mut tree = PanelTree::new();
    let mut windows = HashMap::new();
    self.terminated = false;
    while !self.terminated {
        self.DoTimeSlice(&mut tree, &mut windows);
    }
}
```

Add `has_awake_engines()` method (needed by Task 2):

```rust
/// Check if any engines are currently awake (queued in any wake list).
pub fn has_awake_engines(&self) -> bool {
    self.inner.wake_queues.iter().any(|q| !q.is_empty())
}
```

- [ ] **Step 3: Update production call site in emGUIFramework**

In `crates/emcore/src/emGUIFramework.rs`, line 311, change:

```rust
// Old:
self.scheduler.borrow_mut().DoTimeSlice();

// New:
self.scheduler.borrow_mut().DoTimeSlice(&mut self.tree, &mut self.windows);
```

- [ ] **Step 4: Update production engine Cycle signatures**

These engines don't use tree or windows — just update the parameter name to `_ctx` where appropriate.

In `crates/emstocks/src/emStocksPricesFetcher.rs` line 507, the signature already uses `_ctx`:
```rust
fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
```
No change needed (EngineCtx is imported by name, widening it doesn't change the call site).

Same for `crates/emcore/src/emPriSchedAgent.rs` line 40 and `crates/emcore/src/emMiniIpc.rs` line 317 — verify they compile with the widened EngineCtx. If they import specific EngineCtx fields, update those imports.

- [ ] **Step 5: Update StartupEngine Cycle signature (temporary)**

In `crates/emmain/src/emMainWindow.rs`, the current `StartupEngine::Cycle` at line 356 already takes `ctx: &mut EngineCtx<'_>`. Since EngineCtx is widened, it compiles as-is. The full rewrite comes in Task 3.

- [ ] **Step 6: Update inline scheduler tests**

In `crates/emcore/src/emScheduler.rs` tests (lines 445-823), every `DoTimeSlice()` call needs tree and windows arguments. Add a helper at the top of the `tests` module and update all calls:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use winit::window::WindowId;
    use crate::emPanelTree::PanelTree;
    use crate::emWindow::ZuiWindow;

    /// Helper: run one time slice with empty tree/windows (for scheduler-only tests).
    fn slice(sched: &mut EngineScheduler) {
        let mut tree = PanelTree::new();
        let mut windows: HashMap<WindowId, ZuiWindow> = HashMap::new();
        sched.DoTimeSlice(&mut tree, &mut windows);
    }
```

Then replace every `sched.DoTimeSlice()` with `slice(&mut sched)`.

The `CountingEngine`, `PollingEngine`, and all other test engines in this file already take `_ctx: &mut EngineCtx<'_>` — they compile unchanged.

- [ ] **Step 7: Update external test files**

Apply the same `slice()` helper pattern to each test file that calls `DoTimeSlice()`:

**`crates/emcore/src/emPriSchedAgent.rs`** (inline tests): Add `slice()` helper, replace `sched.DoTimeSlice()` calls.

**`crates/emcore/src/emFileModel.rs`** (inline test): Same pattern.

**`crates/eaglemode/tests/unit/scheduler.rs`**: Add `slice()` helper, replace calls.

**`crates/eaglemode/tests/integration/lifecycle.rs`**: Update `DummyEngine` Cycle signature (line 58), add `slice()` helper.

**`crates/eaglemode/tests/integration/signals.rs`**: Update `SignalFiringEngine`, `CounterEngine`, `FlagEngine` Cycle signatures.

**`crates/eaglemode/tests/golden/scheduler.rs`**: Add `slice()` helper, replace calls. Update `RecordingEngine`, `MultiSigEngine` Cycle signatures.

**`crates/eaglemode/tests/behavioral/mini_ipc.rs`**: Add `slice()` helper, replace calls.

**`crates/eaglemode/tests/behavioral/pri_sched_agent.rs`**: Add `slice()` helper, replace calls.

**`crates/eaglemode/tests/support/mod.rs`** (line 72) and **`crates/eaglemode/tests/support/pipeline.rs`** (line 73): These likely have a `TestHarness` struct with tree already. Update `DoTimeSlice()` to pass `&mut self.tree` and an empty windows HashMap (or add a windows field to the harness).

**`examples/signal_timer_demo.rs`** (line 42): Update `CounterEngine` Cycle signature.

- [ ] **Step 8: Run tests and verify**

```bash
cargo clippy -- -D warnings
cargo-nextest ntr
```

Expected: All tests pass, no clippy warnings.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: widen EngineCtx to include PanelTree and windows

Match C++ Eagle Mode design where engines have full access to the
application context during Cycle(), not just the scheduler.

Add has_awake_engines() to EngineScheduler for event loop pumping."
```

---

### Task 2: Add event loop pumping

After `DoTimeSlice()`, if any engines are still awake, request redraws so `about_to_wait()` fires again. This ensures the startup engine (and any other active engine) gets continuous cycling even with `ControlFlow::Wait`.

**Files:**
- Modify: `crates/emcore/src/emGUIFramework.rs`

- [ ] **Step 1: Add pumping logic after DoTimeSlice**

In `crates/emcore/src/emGUIFramework.rs`, in `about_to_wait()`, after the `DoTimeSlice` call (line ~311), add:

```rust
// Run one scheduler time slice
self.scheduler.borrow_mut().DoTimeSlice(&mut self.tree, &mut self.windows);

// Keep event loop pumping while engines are active.
// C++ runs a tight 10ms loop; Rust uses event-driven winit with
// ControlFlow::Wait which only fires about_to_wait on OS events.
// Requesting redraws ensures continuous cycling during startup,
// animations, and any other engine activity.
if self.scheduler.borrow().has_awake_engines() {
    for win in self.windows.values() {
        win.request_redraw();
    }
}
```

Note: `request_redraw` is called on `&self` (shared ref) since it goes through winit's `Window::request_redraw()` which takes `&self`. If the current `request_redraw` takes `&mut self`, change to iterate window IDs and call on each.

- [ ] **Step 2: Verify request_redraw signature**

Check `crates/emcore/src/emWindow.rs` for `request_redraw`. If it takes `&mut self`, change the pumping to collect window IDs first:

```rust
if self.scheduler.borrow().has_awake_engines() {
    let ids: Vec<_> = self.windows.keys().copied().collect();
    for id in ids {
        if let Some(win) = self.windows.get(&id) {
            win.request_redraw();
        }
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo clippy -- -D warnings
cargo-nextest ntr
```

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emGUIFramework.rs
git commit -m "feat: pump event loop while engines are awake

Request redraws after DoTimeSlice when engines are still queued,
ensuring continuous cycling with ControlFlow::Wait."
```

---

### Task 3: Rewrite StartupEngine to directly manipulate panels

Delete the shared-state IPC pattern. Rewrite `StartupEngine` to access the panel tree and windows directly via `ctx.tree` and `ctx.windows`, matching C++ `StartupEngineClass::Cycle()` state-by-state.

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

**Reference:** C++ source at `~/git/eaglemode-0.96.4/src/emMain/emMainWindow.cpp:362-485`

- [ ] **Step 1: Delete shared-state IPC**

Remove from `crates/emmain/src/emMainWindow.rs`:

1. Delete the `StartupState` struct (lines 31-34)
2. Delete `cycle_startup()` method (lines 108-151)
3. Remove `startup_state` field from `emMainWindow` struct (line 66)
4. Remove `startup_state` initialization from `emMainWindow::new()` (line 89)
5. Remove shared state creation and assignment from `create_main_window()` (lines 475-480)
6. Update `handle_input()` line 229: change `self.startup_state.is_some()` to `self.startup_engine_id.is_some()`
7. Update `GetTitle()` line 197: change `self.startup_state.is_none()` to `self.startup_engine_id.is_none()`
8. Remove the `use std::cell::RefCell; use std::rc::Rc;` imports if no longer needed elsewhere in the file (Rc is still needed for `_ctx`).

- [ ] **Step 2: Rewrite StartupEngine struct**

Replace the current `StartupEngine` (lines 337-432) with:

```rust
/// Startup engine registered with the scheduler.
///
/// Port of C++ `emMainWindow::StartupEngineClass` (emMainWindow.cpp:330-485).
/// Directly manipulates the panel tree and windows during Cycle(),
/// matching C++ where the engine holds `emMainWindow& MainWin`.
pub(crate) struct StartupEngine {
    state: u8,
    main_panel_id: PanelId,
    window_id: winit::window::WindowId,
    self_engine_id: EngineId,
    visit_valid: bool,
    visit_identity: String,
    visit_rel_x: f64,
    visit_rel_y: f64,
    visit_rel_a: f64,
    visit_adherent: bool,
    visit_subject: String,
    clock: std::time::Instant,
}

impl StartupEngine {
    pub(crate) fn new(
        main_panel_id: PanelId,
        window_id: winit::window::WindowId,
        self_engine_id: EngineId,
        visit_identity: Option<String>,
    ) -> Self {
        let visit_valid = visit_identity.is_some();
        Self {
            state: 0,
            main_panel_id,
            window_id,
            self_engine_id,
            visit_valid,
            visit_identity: visit_identity.unwrap_or_default(),
            visit_rel_x: 0.0,
            visit_rel_y: 0.0,
            visit_rel_a: 0.0,
            visit_adherent: false,
            visit_subject: String::new(),
            clock: std::time::Instant::now(),
        }
    }
}
```

- [ ] **Step 3: Implement Cycle states 0-6**

```rust
impl emEngine for StartupEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        match self.state {
            // States 0-2: idle wake-ups (C++ emMainWindow.cpp:367-375).
            0..=2 => {
                self.state += 1;
                true
            }
            // State 3: Set startup overlay, configure autoplay
            // (C++ emMainWindow.cpp:377-389).
            // DIVERGED: C++ creates MainPanel here; Rust creates it in
            // create_main_window() before the engine starts.
            3 => {
                ctx.tree
                    .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
                        mp.SetStartupOverlay(true);
                    });
                // TODO(Task 4): set autoplay config path here
                self.state += 1;
                true
            }
            // State 4: Load bookmarks, find start location
            // (C++ emMainWindow.cpp:392-406).
            4 => {
                // BookmarksModel integration is in Task 4.
                // For now, advance without bookmark search.
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 5: Create control panel (C++ emMainWindow.cpp:407-415).
            5 => {
                ctx.tree
                    .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
                        mp.advance_creation_stage();
                    });
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 6: Create content panel (C++ emMainWindow.cpp:416-422).
            6 => {
                ctx.tree
                    .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
                        mp.advance_creation_stage();
                    });
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
```

- [ ] **Step 4: Implement Cycle states 7-11 (animation + cleanup)**

Continue the match block:

```rust
            // State 7: Create visiting animator, zoom to ":"
            // (C++ emMainWindow.cpp:423-432).
            7 => {
                if let Some(win) = ctx.windows.get_mut(&self.window_id) {
                    let mut animator = emVisitingViewAnimator::new(0.0, 0.0, 0.0, 1.0);
                    animator.SetGoalFullsized(":", false);
                    // Activate: set as window's active animator
                    win.active_animator = Some(Box::new(animator));
                }
                self.clock = std::time::Instant::now();
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 8: Wait up to 2 seconds for root zoom
            // (C++ emMainWindow.cpp:433-438).
            8 => {
                let still_active = ctx
                    .windows
                    .get(&self.window_id)
                    .and_then(|w| w.active_animator.as_ref())
                    .map(|a| a.is_active())
                    .unwrap_or(false);
                if self.clock.elapsed().as_millis() < 2000 && still_active {
                    true // keep waiting
                } else {
                    self.state += 1;
                    true
                }
            }
            // State 9: Deactivate animator, set visit goal, reactivate
            // (C++ emMainWindow.cpp:439-454).
            9 => {
                if let Some(win) = ctx.windows.get_mut(&self.window_id) {
                    // Stop current animation
                    if let Some(ref mut anim) = win.active_animator {
                        anim.stop();
                    }
                    // Set new goal if visit is valid
                    if self.visit_valid {
                        let mut animator = emVisitingViewAnimator::new(0.0, 0.0, 0.0, 1.0);
                        animator.set_goal_rel(
                            &self.visit_identity,
                            self.visit_rel_x,
                            self.visit_rel_y,
                            self.visit_rel_a,
                            self.visit_adherent,
                            &self.visit_subject,
                        );
                        win.active_animator = Some(Box::new(animator));
                    }
                }
                self.clock = std::time::Instant::now();
                self.state += 1;
                !ctx.IsTimeSliceAtEnd()
            }
            // State 10: Wait for visit animation, then clean up
            // (C++ emMainWindow.cpp:455-465).
            10 => {
                let still_active = ctx
                    .windows
                    .get(&self.window_id)
                    .and_then(|w| w.active_animator.as_ref())
                    .map(|a| a.is_active())
                    .unwrap_or(false);
                if self.clock.elapsed().as_millis() < 2000 && still_active {
                    return true;
                }
                // Clean up: remove animator, zoom out, set active panel, remove overlay
                if let Some(win) = ctx.windows.get_mut(&self.window_id) {
                    win.active_animator = None;
                    win.view_mut().RawZoomOut(ctx.tree);
                    // Set active panel to content panel root
                    // (C++ uses MainWin.ContentPanel which is the cosmos panel)
                }
                ctx.tree
                    .with_behavior_as::<emMainPanel, _>(self.main_panel_id, |mp| {
                        mp.SetStartupOverlay(false);
                    });
                self.clock = std::time::Instant::now();
                self.state += 1;
                true
            }
            // State 11+: Final visit, self-delete
            // (C++ emMainWindow.cpp:466-483).
            11 => {
                if self.clock.elapsed().as_millis() < 100 {
                    return true;
                }
                if self.visit_valid {
                    if let Some(win) = ctx.windows.get_mut(&self.window_id) {
                        // Find panel by identity and visit it
                        if let Some(panel_id) =
                            ctx.tree.find_panel_by_identity(&self.visit_identity)
                        {
                            win.view_mut().Visit(
                                panel_id,
                                self.visit_rel_x,
                                self.visit_rel_y,
                                self.visit_rel_a,
                            );
                        }
                    }
                }
                // Self-delete: remove engine from scheduler
                // (C++ emMainWindow.cpp:481-483: delete this; return false)
                ctx.scheduler.remove_engine(self.self_engine_id);
                false
            }
            _ => false,
        }
    }
}
```

**Note:** The `emVisitingViewAnimator::new()` constructor and `set_goal_rel()` signatures need verification against `crates/emcore/src/emViewAnimator.rs:679` and `752`. The implementer should read the actual signatures and adjust. Also verify `ctx.tree.find_panel_by_identity()` exists — if not, the Visit in state 11 may need to use the view's Visit-by-identity path instead.

**Note:** `ctx.scheduler.remove_engine()` is called on `EngineCtxInner` — verify this method exists there (it may only be on `EngineScheduler`). If not available, the engine can signal self-deletion via a shared flag that `create_main_window` checks, or the engine can simply return `false` and let the scheduler clean it up on next slice.

- [ ] **Step 5: Update create_main_window**

In `create_main_window()` (line 439-494), update engine creation to pass the new fields and remove shared-state code:

```rust
// Register StartupEngine with the scheduler
let engine_id = {
    let mut sched = app.scheduler.borrow_mut();
    let id = sched.register_engine(Priority::Low, Box::new(
        // Placeholder — we need engine_id to create the engine, but
        // register_engine returns the id. Use a two-phase approach:
        // register with a dummy, then replace.
        StartupEngine::new(root_id, window_id, EngineId::default(), mw.config.visit.clone())
    ));
    // Patch the engine's self_engine_id
    // (The engine needs its own ID for self-deletion in state 11)
    id
};
// The engine_id patching is tricky. Alternative: store engine_id on
// emMainWindow and have the engine remove itself by signaling done,
// then the window removes it from the scheduler.
app.scheduler.borrow_mut().wake_up(engine_id);
mw.startup_engine_id = Some(engine_id);
```

**Self-deletion design note:** C++ uses `delete this` inside Cycle(). In Rust, the engine can't remove itself from the scheduler during Cycle() because the scheduler owns it. Options:
1. Store `self_engine_id` and call `ctx.scheduler.remove_engine()` — but EngineCtxInner may not expose this.
2. Return `false` from Cycle(), which makes the engine sleep. Then `create_main_window` stores the engine_id and the emMainWindow removes it later.
3. Add a `self_remove` flag to EngineCtx that the scheduler checks after Cycle().

The simplest: just return `false`. The engine sleeps permanently. The `emMainWindow` can check `startup_engine_id` and remove the engine on next frame. Or simply leave the sleeping engine registered — it consumes negligible memory.

The implementer should choose the simplest working approach. The C++ self-deletion is not essential for correctness — what matters is that the engine stops cycling and the startup_engine_id is cleared.

- [ ] **Step 6: Update tests**

Update `test_startup_engine_initial_state` (line 593) — it creates a `StartupEngine` with old args. Update to match new constructor:

```rust
#[test]
fn test_startup_engine_initial_state() {
    let panel_id = PanelId::from(KeyData::from_ffi(0x0100_0000_0000_0000));
    let engine_id = EngineId::from(KeyData::from_ffi(0x0100_0000_0000_0000));
    let window_id = unsafe { winit::window::WindowId::dummy() };
    let engine = StartupEngine::new(panel_id, window_id, engine_id, None);

    assert_eq!(engine.state, 0);
    assert_eq!(engine.main_panel_id, panel_id);
    assert!(!engine.visit_valid);
}
```

Also remove tests that reference `StartupState` (`test_startup_state_debug`, `test_startup_state_done`).

- [ ] **Step 7: Run tests**

```bash
cargo clippy -- -D warnings
cargo-nextest ntr
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: rewrite StartupEngine to directly manipulate panels

Match C++ StartupEngineClass::Cycle() — the engine accesses the panel
tree and windows directly via EngineCtx, creating panels and driving
animations inline. Delete shared-state IPC (StartupState, cycle_startup)."
```

---

### Task 4: BookmarksModel integration in startup

Wire `emBookmarksModel` into StartupEngine state 4 (search start location) and into `emMainWindow::handle_input` (bookmark hotkeys).

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`
- Modify: `crates/emmain/src/emBookmarks.rs` (verify APIs exist)

**Reference:** C++ `emMainWindow.cpp:392-406` (state 4) and `emMainWindow.cpp:247-260` (hotkeys)

- [ ] **Step 1: Verify BookmarksModel APIs**

Read `crates/emmain/src/emBookmarks.rs` and confirm:
- `emBookmarksModel::Acquire(ctx: &Rc<emContext>) -> Rc<RefCell<Self>>` exists (line ~458)
- `SearchStartLocation(&self) -> Option<&emBookmarkRec>` exists (line ~304)
- `SearchBookmarkByHotkey(&self, hotkey: &str) -> Option<&emBookmarkRec>` exists (line ~311)
- `emBookmarkRec` has fields: `LocationIdentity`, `LocationRelX`, `LocationRelY`, `LocationRelA`, `Name`

- [ ] **Step 2: Add BookmarksModel to StartupEngine**

Add a `bookmarks` field to `StartupEngine`:

```rust
pub(crate) struct StartupEngine {
    // ... existing fields ...
    bookmarks: Option<Rc<RefCell<emBookmarksModel>>>,
    context: Rc<emContext>,
}
```

Update the constructor to accept `context`:

```rust
pub(crate) fn new(
    main_panel_id: PanelId,
    window_id: winit::window::WindowId,
    self_engine_id: EngineId,
    visit_identity: Option<String>,
    context: Rc<emContext>,
) -> Self {
    // ... existing fields ...
    Self {
        // ...
        bookmarks: None,
        context,
    }
}
```

- [ ] **Step 3: Implement state 4 bookmark search**

Replace the state 4 placeholder in Cycle():

```rust
4 => {
    // Acquire BookmarksModel (C++ emMainWindow.cpp:392).
    self.bookmarks = Some(emBookmarksModel::Acquire(&self.context));

    // If no explicit visit identity, search bookmarks for start location
    // (C++ emMainWindow.cpp:393-405).
    if !self.visit_valid {
        if let Some(ref bm) = self.bookmarks {
            if let Some(rec) = bm.borrow().SearchStartLocation() {
                self.visit_valid = true;
                self.visit_identity = rec.LocationIdentity.clone();
                self.visit_rel_x = rec.LocationRelX;
                self.visit_rel_y = rec.LocationRelY;
                self.visit_rel_a = rec.LocationRelA;
                self.visit_adherent = true;
                self.visit_subject = rec.entry.Name.clone();
            }
        }
    }
    self.state += 1;
    !ctx.IsTimeSliceAtEnd()
}
```

- [ ] **Step 4: Add BookmarksModel to emMainWindow**

Add a `bookmarks_model` field to `emMainWindow`:

```rust
pub struct emMainWindow {
    // ... existing fields ...
    pub(crate) bookmarks_model: Option<Rc<RefCell<crate::emBookmarks::emBookmarksModel>>>,
}
```

Initialize to `None` in `new()`. After startup completes (when `startup_engine_id` is cleared), the bookmarks model should be transferred from the engine or re-acquired.

Alternatively, acquire it independently in `create_main_window`:

```rust
mw.bookmarks_model = Some(crate::emBookmarks::emBookmarksModel::Acquire(&app.context));
```

- [ ] **Step 5: Wire bookmark hotkeys in handle_input**

Replace the DIVERGED comment at line 302-305 with actual bookmark hotkey handling:

```rust
// Bookmark hotkeys (C++ emMainWindow.cpp:247-260).
if let Some(ref bm_model) = self.bookmarks_model {
    // Build hotkey string from the event
    let hotkey = format_hotkey(event, input_state);
    if !hotkey.is_empty() {
        if let Some(rec) = bm_model.borrow().SearchBookmarkByHotkey(&hotkey) {
            // Visit the bookmark location
            if let Some(main_id) = self.main_panel_id {
                if let Some(panel_id) = app.tree.find_panel_by_identity(&rec.LocationIdentity) {
                    if let Some(win) = self.window_id.and_then(|id| app.windows.get_mut(&id)) {
                        win.view_mut().Visit(
                            panel_id,
                            rec.LocationRelX,
                            rec.LocationRelY,
                            rec.LocationRelA,
                        );
                    }
                }
            }
            return true;
        }
    }
}
```

**Note:** The `format_hotkey` function needs to convert `emInputEvent` + `emInputState` into the hotkey string format that `SearchBookmarkByHotkey` expects. Check the C++ `emInputHotkey(event, state)` for the format. The implementer should read `crates/emcore/src/emInput.rs` for the Rust equivalent.

If `format_hotkey` / `emInputHotkey` doesn't exist in Rust, it needs to be implemented. C++ format is typically `"Ctrl+Shift+F1"` etc. Check what format the bookmark config files use for hotkeys and match it.

- [ ] **Step 6: Run tests**

```bash
cargo clippy -- -D warnings
cargo-nextest ntr
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: wire BookmarksModel into startup and hotkeys

State 4 loads bookmarks and searches for start location.
Bookmark hotkeys in handle_input navigate to saved locations."
```

---

### Task 5: Make emMainWindow an engine

Register `emMainWindow` logic as a scheduler engine, matching C++ where `emMainWindow` derives from `emEngine` (emMainWindow.cpp:174-190).

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`

**Reference:** C++ `emMainWindow.cpp:174-190` (`Cycle`), lines 28-73 (constructor, signal wiring)

- [ ] **Step 1: Create MainWindowEngine struct**

Add a new engine struct that handles title signal and close signal:

```rust
/// Engine for emMainWindow, matching C++ emMainWindow::Cycle()
/// (emMainWindow.cpp:174-190).
///
/// Checks title signal (from content view) and close signal each cycle.
pub(crate) struct MainWindowEngine {
    window_id: winit::window::WindowId,
    main_panel_id: PanelId,
    close_signal: SignalId,
    title_signal: Option<SignalId>,
}

impl emEngine for MainWindowEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        // Check title signal → update window title
        // (C++ emMainWindow.cpp:176-178)
        if let Some(title_sig) = self.title_signal {
            if ctx.IsSignaled(title_sig) {
                if let Some(win) = ctx.windows.get_mut(&self.window_id) {
                    let title = win.view().GetTitle();
                    let full_title = format!("Eagle Mode - {title}");
                    win.winit_window.set_title(&full_title);
                }
            }
        }

        // Check close signal → remove window
        // (C++ emMainWindow.cpp:180-181)
        if ctx.IsSignaled(self.close_signal) {
            // Mark for close — actual removal happens in event loop
            with_main_window(|mw| {
                mw.to_close = true;
            });
        }

        // Self-delete if to_close (C++ emMainWindow.cpp:184-187)
        let to_close = with_main_window(|mw| mw.to_close).unwrap_or(false);
        if to_close {
            return false; // Stop cycling; event loop handles actual window removal
        }

        false // Sleep until signaled
    }
}
```

- [ ] **Step 2: Register MainWindowEngine in create_main_window**

In `create_main_window()`, after creating the window and signals:

```rust
// Register MainWindowEngine (C++ emMainWindow derives from emEngine).
let mw_engine = MainWindowEngine {
    window_id,
    main_panel_id: root_id,
    close_signal,
    title_signal: None, // TODO: wire content view title signal when available
};
let mw_engine_id = app
    .scheduler
    .borrow_mut()
    .register_engine(Priority::Low, Box::new(mw_engine));
// Connect close signal to this engine
app.scheduler.borrow_mut().connect(close_signal, mw_engine_id);
app.scheduler.borrow_mut().wake_up(mw_engine_id);
```

- [ ] **Step 3: Update GetTitle to be dynamic**

In `emMainWindow::GetTitle()`, replace the static title with a dynamic one:

```rust
pub fn GetTitle(&self, app: &App) -> String {
    if self.main_panel_id.is_some() && self.startup_engine_id.is_none() {
        if let Some(win) = self.window_id.and_then(|id| app.windows.get(&id)) {
            let title = win.view().GetTitle();
            if !title.is_empty() {
                return format!("Eagle Mode - {title}");
            }
        }
    }
    "Eagle Mode".to_string()
}
```

- [ ] **Step 4: Run tests**

```bash
cargo clippy -- -D warnings
cargo-nextest ntr
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: register emMainWindow as scheduler engine

Match C++ emMainWindow::Cycle() — handles title signal for dynamic
window titles and close signal for clean shutdown."
```

---

### Task 6: Final verification

Run all tests, clippy, and manual launch to verify everything works end-to-end.

**Files:** None (verification only)

- [ ] **Step 1: Run golden tests**

```bash
cargo test --test golden -- --test-threads=1
```

Expected: 239 pass, 4 fail (unchanged baseline).

- [ ] **Step 2: Run full test suite**

```bash
cargo-nextest ntr
```

- [ ] **Step 3: Run clippy**

```bash
cargo clippy -- -D warnings
```

- [ ] **Step 4: Manual launch verification**

```bash
cargo run
```

Verify:
- Window opens with "Loading..." overlay
- After ~4 seconds, overlay clears
- Starfield background visible
- 3 cosmos items visible (Home, Root, Stocks1)
- Control panel with bookmarks visible
- Keyboard shortcuts work (F11 fullscreen, Escape toggle control)
- Zooming into items works

- [ ] **Step 5: Commit any final fixes**

If any adjustments were needed, commit them:

```bash
git add -A
git commit -m "fix: final adjustments for startup architecture rewrite"
```
