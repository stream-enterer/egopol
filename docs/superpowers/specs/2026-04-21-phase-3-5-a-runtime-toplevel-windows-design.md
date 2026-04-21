# Phase 3.5.A — Runtime Top-Level Windows + Per-emWindow PanelTree

**Date:** 2026-04-21
**Branch:** `port-rewrite/phase-3-5-a-runtime-toplevel-windows` (off `1e393d2f` on Phase 3.5, tagged `port-rewrite/phase-3-5-partial-checkpoint-before-3-5-a`)
**Prereq for:** Phase 3.5 Task 5 (`emDialog` reshape).
**JSON entries:** none opened/closed directly; unblocks E024 path via Phase 3.5. Opens E026 only if popup migration must be deferred (contingency — see §Risks).

## Problem

Phase 3.5 Task 5's `emDialog = emWindow + DlgPanel-root + DialogPrivateEngine` shape requires three capabilities the codebase currently lacks:

1. **Multiple `PanelTree`s.** `App::tree: PanelTree` is a single tree; `PanelTree::create_root` asserts exactly one root (emPanelTree.rs:479-483).
2. **Runtime top-level-window install.** `App::windows: HashMap<WindowId, emWindow>` is only populated at startup. `DeferredAction` (emEngineCtx.rs:33-42) has `CloseWindow` + `MaterializePopup` only; no runtime-add path.
3. **Top-level-window constructor callable mid-Cycle.** `emWindow::new_popup_pending` exists for popups; no analog for top-level windows. `emWindow::create` requires `&ActiveEventLoop`, unavailable inside engine Cycles.

Investigation at commit `1e393d2f` confirmed all three tight.

## Scope

Unblock Phase 3.5 Task 5 in one atomic sub-phase. Every `emWindow` (home + popup + dialog) owns its `PanelTree` as a field. `App::tree` is deleted.

Explicitly out of scope: `emDialog` reshape itself (Phase 3.5 Task 5 consumer), multi-monitor, `emSubViewPanel` structural changes (already correct).

## Design principles — precedent audit

Every decision below maps to an existing in-codebase pattern. The spec composes established infrastructure rather than inventing new shapes.

| Concern | Existing precedent | 3.5.A reuse |
|---|---|---|
| "Container owns its tree" | `emSubViewPanel::sub_tree: PanelTree` (emSubViewPanel.rs:23) | `emWindow::tree: PanelTree` |
| "Dispatch detaches a tree slot, walks, restores" | `dispatch_with_resolved_tree` SubView path (emScheduler.rs:138-169) | `mem::take` on `emWindow::tree` at dispatch entry, restore on exit |
| "Sentinel empty tree as placeholder during re-entrant dispatch" | `PanelCycleEngine::Cycle` allocates a `dummy_tree = PanelTree::new()` per cycle (emPanelCycleEngine.rs:82) | `PanelTree: Default` returning `PanelTree::new()`; used by scheduler take/put |
| "Engine's view-scope identity" | `PanelScope::Toplevel(WindowId) \| SubView(PanelId)` (emPanelScope.rs:10-13), used by `UpdateEngineClass`, `VisitingVAEngineClass`, `emWindowStateSaver`, `PanelCycleEngine` | Extend with `Framework` variant; make scheduler branch on PanelScope at dispatch |
| "Ctx variants per role" | `EngineCtx` (full) / `SchedCtx` (no tree, no windows) / `InitCtx` (no fire/connect) — Phase 1.5 (emEngineCtx.rs) | `EngineCtx::tree: Option<&mut PanelTree>` — `None` for Framework engines, `Some` for window-scoped. No new ctx type. |
| "Stable id handles resolved via App lookup" | `EngineId`, `PanelId`, `SignalId`, `TimerId` throughout | New `DialogId(u64)` for pending-dialog correlation |
| "Pending → materialize via closure queue" | Popup `new_popup_pending` + `pending_framework_actions` + `materialize_pending_popup` (emGUIFramework.rs:250-357, emView.rs:1936-1975) | Top-level analog: `new_top_level_pending` + same closure queue + `install_pending_top_level` drain |

No new Rc/RefCell, no Any, no Cow. Existing aliased-borrow via unsafe pointer casts (emPanelCycleEngine.rs:93-109, emPanelScope.rs:79-84) is not worsened — 3.5.A preserves those sites as-is.

## Architecture

### 1. Per-emWindow PanelTree

`emWindow` gains field `tree: PanelTree`. Every ctor (`create`, `new_popup_pending`, new `new_top_level_pending`) constructs its own tree via `PanelTree::new_with_location(...)`, calls `create_root` on it, and stores as `self.tree`. `root_panel: PanelId` remains — now a key into `self.tree`, not `App::tree`.

`App::tree` field deleted entirely. `App::windows: HashMap<WindowId, emWindow>` becomes the sole tree storage, keyed by WindowId.

Helper on `emWindow`:
```rust
pub(crate) fn take_tree(&mut self) -> PanelTree { std::mem::take(&mut self.tree) }
pub(crate) fn put_tree(&mut self, t: PanelTree) { self.tree = t; }
```

`PanelTree: Default` added — returns `PanelTree::new()` (empty SlotMaps). This is consistent with the `dummy_tree = PanelTree::new()` sentinel pattern already used by `PanelCycleEngine::Cycle` at emPanelCycleEngine.rs:82.

### 2. `PanelScope` as unified engine-location carrier

Current `PanelScope`:
```rust
pub enum PanelScope {
    Toplevel(WindowId),
    SubView(PanelId),  // pid in App::tree
}
```

Extended:
```rust
pub enum PanelScope {
    Framework,                                     // spans windows; no own tree
    Toplevel(WindowId),                            // root of windows[wid].tree
    SubView { window_id: WindowId, outer_panel_id: PanelId, rest: Box<PanelScope> },
}
```

Semantic shifts:
- `Framework` — new. Engines that observe / route across windows (InputDispatchEngine, MiniIpcEngine, PriSchedEngine, emWindowStateSaver). At Cycle dispatch, **no tree is detached**; `ctx.tree` is `None`. The engine reaches windows + trees directly via `ctx.windows[wid].tree`.
- `Toplevel(wid)` — unchanged meaning; scheduler detaches `windows[wid].tree` for the Cycle.
- `SubView { window_id, outer_panel_id, rest }` — now carries explicit WindowId (previously implicit as "App::tree's containing window"). Recursion unchanged in shape.

`PanelScope::resolve_view` extended with the Framework case (returns `None` — framework engines don't have a view-scope; callers use windows directly).

### 3. `TreeLocation` retirement

`TreeLocation` (emEngine.rs:24-30) is retired. Its two responsibilities split:

- **"Which window does this engine's tree belong to"** → absorbed into `PanelScope`.
- **"How to walk within a tree to reach a sub-view"** → absorbed into `PanelScope::SubView`'s `rest` chain.

Scheduler bookkeeping: `engine_locations: SecondaryMap<EngineId, TreeLocation>` replaced by `engine_scopes: SecondaryMap<EngineId, PanelScope>`, populated at `register_engine` time.

`register_engine` signature:
```rust
pub fn register_engine(
    &mut self,
    behavior: Box<dyn emEngine>,
    pri: Priority,
    scope: PanelScope,                    // was: tree_location: TreeLocation
) -> EngineId
```

`ConstructCtx::register_engine` similarly updated. Actual call-site count via `rg`: `register_engine(` appears ~124 times across 41 files (~80 in code, rest in docs/plans); `TreeLocation::Outer|SubView` appears ~123 times across 26 files (25 in emScheduler.rs tests alone, ~90 total in code). Migration is wide but the type system guarantees every site is touched at compile time — a missed site is a compile error, not a runtime panic.

Test-site migration cost: mechanical but wide. Every test that builds a scheduler + a panel tree + registers an engine must supply a `WindowId`. Tests without a real window use `WindowId::dummy()` (already exists — emPanelScope.rs:98 test precedent).

### 4. Scheduler dispatch — take/put via scope

`DoTimeSlice` signature drops the standalone `tree: &mut PanelTree` parameter; windows carry the trees:

```rust
pub fn DoTimeSlice(
    &mut self,
    windows: &mut HashMap<WindowId, emWindow>,
    root_context: &Rc<emContext>,
    framework_actions: &mut Vec<DeferredAction>,
    pending_inputs: &mut Vec<(WindowId, emInputEvent)>,
    input_state: &mut emInputState,
    framework_clipboard: &RefCell<Option<Box<dyn emClipboard>>>,
)
```

Per-engine dispatch (replacing emScheduler.rs:~543-573):

```rust
let scope = engine_scopes.get(engine_id).cloned().expect("populated at register");
let stay_awake = match scope {
    PanelScope::Framework => {
        // No tree detachment. ctx.tree = None.
        let mut ctx = EngineCtx {
            scheduler: self, tree: None, windows, root_context,
            framework_actions, pending_inputs, input_state,
            framework_clipboard, engine_id,
        };
        behavior.Cycle(&mut ctx)
    }
    PanelScope::Toplevel(wid) => {
        let Some(win) = windows.get_mut(&wid) else { /* window gone — sleep */ return /* ... */ };
        let mut tree = win.take_tree();
        let result = {
            let mut ctx = EngineCtx {
                scheduler: self, tree: Some(&mut tree), windows,
                root_context, framework_actions, pending_inputs,
                input_state, framework_clipboard, engine_id,
            };
            behavior.Cycle(&mut ctx)
        };
        windows.get_mut(&wid).expect("window still present").put_tree(tree);
        result
    }
    PanelScope::SubView { window_id, outer_panel_id, rest } => {
        let Some(win) = windows.get_mut(&window_id) else { return /* sleep */ };
        let mut tree = win.take_tree();
        let result = dispatch_sub_resolved(&mut tree, &*rest, |resolved| {
            let mut ctx = EngineCtx {
                scheduler: self, tree: Some(resolved), windows, ...
            };
            behavior.Cycle(&mut ctx)
        });
        windows.get_mut(&window_id).unwrap().put_tree(tree);
        result
    }
};
```

`dispatch_sub_resolved` is `dispatch_with_resolved_tree`'s SubView-chain portion with the outer `Outer → f(tree)` base case unchanged; the top-level `Toplevel(wid)` → take/put wrap is the new outer layer.

### 5. `EngineCtx::tree` becomes `Option<&mut PanelTree>`

```rust
pub struct EngineCtx<'a> {
    pub scheduler: &'a mut EngineScheduler,
    pub tree: Option<&'a mut PanelTree>,              // was: &'a mut PanelTree
    pub windows: &'a mut HashMap<WindowId, emWindow>,
    // ... unchanged
}
```

**Migration rule for every existing engine's Cycle body:**

- Window-scoped engines (PanelCycleEngine, UpdateEngineClass, VisitingVAEngineClass, DialogPrivateEngine, the various test engines in emScheduler.rs, every emSubViewPanel-nested engine): add `let tree = ctx.tree.as_deref_mut().expect("window-scoped engine: tree is Some");` at the top of Cycle. Replace all `ctx.tree.foo(...)` with `tree.foo(...)`.
- Framework engines (InputDispatchEngine, MiniIpcEngine, PriSchedEngine, emWindowStateSaver): Cycle body does not read `ctx.tree`. They access per-window trees via `ctx.windows.get_mut(&wid).tree`.

Both patterns are spelled out. The `.expect` in the window-scoped path is a runtime invariant check — equivalent to today's `engine_locations.get(...).expect("register_engine always populates")` pattern (emScheduler.rs:550-553).

Alternative considered and rejected: two Cycle trait methods (one per scope). Rejected because it requires splitting the `emEngine` trait and every test engine declaration migrates. The `Option<_>.expect()` pattern costs one line per Cycle with no type proliferation.

### 6. Engine audit (migration classification)

All engines found via `rg "impl emEngine for" crates/emcore/src`:

| Engine | Current location (if any) | Post-3.5.A `PanelScope` |
|---|---|---|
| `PanelCycleEngine` (emPanelCycleEngine.rs:42) | `TreeLocation::Outer` or `SubView` | `Toplevel(wid)` or `SubView { wid, ... }` — panel's window |
| `InputDispatchEngine` (emInputDispatchEngine.rs:18) | `TreeLocation::Outer` | **`Framework`** (spans windows) |
| `MiniIpcEngine` (emMiniIpc.rs:322) | `TreeLocation::Outer` | **`Framework`** (FIFO polling, no view) |
| `emWindowStateSaver` (emWindowStateSaver.rs:237) | registered per-window | **`Framework`** (observes window via `ctx.windows.get(&self.window_id)`; doesn't touch trees) |
| `PriSchedEngine` (emPriSchedAgent.rs:41) | `TreeLocation::Outer` | **`Framework`** (Cycle doesn't touch tree/windows; purely model-internal) |
| `DialogPrivateEngine` (emDialog.rs:362) | Phase 3.5 Task 4 registers at `TreeLocation::Outer` | `Toplevel(dialog_window_id)` — dialog's own window |
| `UpdateEngineClass` (emView.rs:199) | carries `PanelScope` already | unchanged — `Toplevel(wid)` or `SubView{...}` |
| `VisitingVAEngineClass` (emView.rs:268) | carries `PanelScope` already | unchanged |
| `EOIEngineClass` (emView.rs:355) | registered from emView; `TreeLocation::Outer` today | `Toplevel(wid)` — per-view EOI countdown |
| Test engines in emScheduler.rs (`CountingEngine`, `PollingEngine`, `OrderEngine`, `CheckSignalEngine`, `FiringEngine`, `ReceivingEngine`, `HighEngine`, `ProbePointerEngine`) | `TreeLocation::Outer` | `Framework` — they don't touch trees; purely scheduler-behavior tests |
| Test engines in emPanelTree.rs (`ChildSpawnEngine`, `SpawnEngineWithProbe`) | Outer | `Toplevel(WindowId::dummy())` — they touch trees |
| Test engines in emSubViewPanel.rs, emEngineCtx.rs (`NoopEngine` ×2), emDialog.rs (`FinishProbe`) | various | classify per-test — most `Framework` |

Audit deliverable in Task 3.5.A-Task-2 (see Migration Tasks below): enumerate every `impl emEngine` site, classify, write migration diff.

### 7. Popup migration

`emWindow::new_popup_pending` stops taking `root_panel: PanelId` from caller (emWindow.rs:320). Instead:

1. Constructs its own `PanelTree::new_with_location(...)`.
2. Calls `tree.create_root("popup", has_view=true)` to get `root_panel`.
3. Stores both as `self.tree` and `self.root_panel`.

`emView::RawVisitAbs` popup-enter code (emView.rs:1936-1949) drops the `self.root` argument. Signature of `new_popup_pending` simplifies.

Popup viewport, render, input, and teardown all route through `windows[popup_wid].tree` post-migration. Popup's panels and sub-views live in the popup's own tree, independent of the launching view's tree. Closes a long-standing implicit divergence (popup sharing launching-view tree) that C++ never had (each emWindow has its own emView::RootPanel).

### 8. Top-level install path

New ctor `emWindow::new_top_level_pending(...)`. Signature mirrors `new_popup_pending` but:
- No `root_panel: PanelId` argument (new behaviour matches migrated popup path).
- Takes top-level `WindowFlags` (not popup-specific).
- Takes a `title: String`, `parent_context: Rc<emContext>`, signals, and `look_bg_color: emColor` — identical shape to `new_popup_pending`'s post-migration signature.
- Constructs own `PanelTree`, root panel, `emView` in `OsSurface::Pending` state. Returns `emWindow`.

New App fields:

```rust
pub struct App {
    // ... existing fields except: pub tree: PanelTree  ← DELETED
    pub windows: HashMap<WindowId, emWindow>,
    pub pending_top_level: Vec<PendingTopLevel>,
    pub dialog_windows: HashMap<DialogId, WindowId>,
    pub next_dialog_id: u64,
    // ...
}

pub(crate) struct PendingTopLevel {
    pub dialog_id: DialogId,
    pub window: emWindow,   // tree lives inside
    pub close_signal: SignalId,  // cached for early-cancel deregister
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DialogId(pub u64);
```

Materialization is enqueued by the caller (e.g., `emDialog::new`) onto the existing `pending_framework_actions` queue (emGUIFramework.rs:~122 — the `Rc<RefCell<Vec<Box<dyn FnOnce(&mut App, &ActiveEventLoop)>>>>`). The closure invokes `App::install_pending_top_level(event_loop)`, which mirrors `materialize_pending_popup`:

1. Pop the first `PendingTopLevel` off the queue.
2. Build `winit::window::WindowAttributes` from the window's `PendingSurface` state (flags, caption, requested_pos_size).
3. `event_loop.create_window(attrs)` → `winit::window::Window`, grab its `WindowId`.
4. Build `MaterializedSurface::build(gpu, arc_window)`.
5. On the emWindow: swap `os_surface` to `Materialized`, call `wire_viewport_window_id(wid)`, call `view_mut().SetGeometry(&mut self.tree, 0.0, 0.0, w, h, 1.0, &mut sc)` (via a destructured SchedCtx).
6. Move the emWindow into `self.windows.insert(wid, emwindow)`.
7. Record `self.dialog_windows.insert(dialog_id, wid)`.

Multiple pending entries drain in FIFO order within the same tick. If a caller enqueues the materialize closure N times for N dialogs constructed in the same slice, each closure drains one entry — no coordination needed.

### 9. Dialog handle resolution

Consumer (`emDialog` façade, Phase 3.5 Task 5) holds `dialog_id: DialogId`. Façade ops resolve via:

1. Matured: `App::dialog_windows.get(&did).and_then(|wid| App::windows.get_mut(wid))`.
2. Pending (pre-materialize): scan `App::pending_top_level` for matching `dialog_id`; op is either allowed on the pending emWindow (e.g., `Finish` → sets `pending_result` on DlgPanel root in the still-embedded tree) or deferred until materialized.

Helper on `App`:
```rust
pub(crate) enum DialogWindow<'a> {
    Pending { idx: usize, window: &'a mut emWindow },
    Materialized { window_id: WindowId, window: &'a mut emWindow },
}

impl App {
    pub(crate) fn dialog_window_mut(&mut self, did: DialogId) -> Option<DialogWindow<'_>> { ... }
}
```

### 10. Teardown

`emDialog::deregister` routes to one of:

- **Pending (pre-materialize):** find index in `App::pending_top_level`, `swap_remove` it. Deregister the private engine (signals free, scheduler.remove_engine). Tree + emWindow drop locally. This handles the same-slice-construct-and-destroy case (mirrors popup same-frame-enter-exit cancellation).
- **Materialized:** push `DeferredAction::CloseWindow(wid)`. Deregister the private engine. On next framework drain, the emWindow (with its tree) is removed from `App::windows` and `App::dialog_windows`. Scheduler `remove_engine` for any per-window engines attached to this window.

Auto-delete flow (C++ `FinishState` countdown) is unchanged from Phase 3.5 Task 4: `DialogPrivateEngine::Cycle` increments state, at threshold emits `DeferredAction::CloseWindow(wid)`. Framework drain handles removal.

## Data flow

### Dialog open (caller inside an engine Cycle, e.g., button-click callback)

1. Caller invokes `emDialog::new(parent_ctx, title, look, scheduler, pending_top_level, pending_framework_actions, root_context)`.
2. Ctor:
   - Allocates signals: `close`, `flags`, `focus`, `geometry`, `finish`.
   - Allocates `dialog_id = pending_top_level.len() + app.next_dialog_id.fetch_add(1)` (or via an `App::allocate_dialog_id` helper — exact plumbing decided by the writing-plans output).
   - Builds `emWindow::new_top_level_pending(...)` with its own `PanelTree` and root. Installs `DlgPanel` as root behavior.
   - Registers `DialogPrivateEngine` at `Priority::High`, `PanelScope::Toplevel(placeholder_wid?)` — **open question, see §Risks**. Connects `close_signal`.
   - Pushes `PendingTopLevel { dialog_id, window, close_signal }` onto `pending_top_level`.
   - Enqueues closure `|fw, el| fw.install_pending_top_level(el)` onto `pending_framework_actions`.
3. Returns façade `emDialog { dialog_id, root_panel_id, private_engine_id, finish_signal, close_signal, ... }`.
4. Current slice completes. `App::about_to_wait` fires; closure drains one pending entry → winit surface created → emWindow moved into `windows[wid]` → `dialog_windows[did] = wid`.
5. Subsequent winit events addressed to `wid` route normally through per-window dispatch.

### Dialog close (user clicks X)

1. winit `CloseRequested` → `close_signal` fires on the dialog's emWindow.
2. Next slice: `DialogPrivateEngine::Cycle` observes `close_signal`, sets `pending_result = Cancel`, finalizes, fires `finish_signal`, invokes `on_finish` / `on_finished`.
3. If `auto_delete`: countdown runs (Phase 3.5 Task 4 state machine already implements this). At threshold, pushes `DeferredAction::CloseWindow(wid)`.
4. Framework drain removes emWindow; `dialog_windows.remove(&did)`; scheduler cleans up any associated engines.

### Popup open (post-migration)

1. `emView::RawVisitAbs` detects outside_home, no existing popup, creates `emWindow::new_popup_pending(...)` — **now constructs own tree + root internally**. No `self.root` pass-through.
2. Stores in `self.PopupWindow`.
3. Enqueues existing popup materialization closure. Unchanged thereafter — `materialize_pending_popup` finds the Pending popup, creates winit surface, installs. Popup's view, inputs, paint all route through `self.PopupWindow.tree`.

## Engine-registration `WindowId` chicken-and-egg

`DialogPrivateEngine` is registered inside `emDialog::new`, BEFORE the winit surface exists — i.e., before `WindowId` is assigned. But `PanelScope::Toplevel(wid)` needs a WindowId at register time.

Three options (design choice during implementation — writing-plans will pick):

**(a) Defer engine registration** until after materialize drain. `emDialog::new` stores the private-engine behavior in `PendingTopLevel`; `install_pending_top_level` registers it post-materialize with the real WindowId. Downside: pending dialogs can't fire their private engine (can't respond to close_signal) until materialized — OK because they don't need to: pre-materialize, there's no OS window, and close_signal can't fire.

**(b) `PanelScope::PendingWindow(DialogId)`** — new variant, resolved at dispatch time by scanning `pending_top_level`. Fragile; adds complexity for the rare case of pre-materialize engine Cycle.

**(c) Synthetic pre-materialize WindowId**: reserve a sentinel WindowId for each pending dialog, stored in `dialog_windows` as `DialogId → Option<WindowId>` (None = pending). Scheduler treats sentinel as "engine is asleep pending materialization." Clunky.

Recommendation: **(a) defer registration.** Matches the popup precedent (popup's `close_signal` is created at construction but its view registrations complete post-materialize — see emView.rs:1940-1950 for the pattern). Close-signal handling: pending dialog's `emDialog::deregister` checks materialization state; close-signal response is only required once the OS window exists.

## Error handling

- `emDialog::new` compile-error without scheduler access (positional argument).
- Ops on a deregistered `DialogId` — both lookup paths miss, return default (matches current emDialog contract).
- Pending dialog whose caller drops it pre-materialize: pending vec retains the emWindow (benign leak — mirrors popup same-frame cancellation). `emDialog::deregister` in the façade's Drop impl cleans up (explicit deregister always required per D3; enforce via ledger-documented invariant).
- Materialize failure (e.g., winit `create_window` fails): `install_pending_top_level` drains the entry but does NOT insert into `windows`, records failure in a log path, fires `close_signal` immediately so the private engine (once eventually registered — option (a) above means never, in this failure path) doesn't hang. OR: drop the entry entirely and let the façade time out. Exact behaviour decided during implementation.
- Scheduler dispatch against a `PanelScope::Toplevel(wid)` whose window was removed this slice: `windows.get_mut(&wid)` returns None → engine sleeps this slice; scheduler skips cleanly (existing precedent in dispatch bail-out at emScheduler.rs:~540).

## Testing strategy

### Unit

1. `emWindow::new_top_level_pending` — constructs in `OsSurface::Pending`, has a valid non-empty tree with exactly one root (distinct `PanelId` from any App-level id).
2. `emWindow::take_tree` / `put_tree` roundtrip — content preserved; post-`take_tree` the field is `PanelTree::default()` (empty).
3. `PanelTree: Default` — produces an empty tree with `root.is_none()`; construction cost low.
4. `App::install_pending_top_level` drain — seed one pending; confirm post-drain `windows` grew by 1, `dialog_windows[did] == wid`, `pending_top_level` empty.
5. Multi-pending-in-one-slice — seed 3 pending, drain each closure, confirm all 3 land in `windows` with distinct WindowIds.
6. Scheduler dispatch — Toplevel-scoped engine Cycle receives `ctx.tree.unwrap()` pointing at its registered window's tree. Post-Cycle, `windows[wid].tree` is restored (no sentinel leaked).
7. Scheduler dispatch — Framework-scoped engine Cycle receives `ctx.tree == None`; can access `ctx.windows[any_wid].tree` directly (all windows' trees present, none in sentinel state).
8. Scheduler dispatch — SubView-scoped engine Cycle receives `ctx.tree.unwrap()` pointing at the right sub-tree, reached by take-behavior chain.
9. `PanelScope` enum completeness — build each variant, confirm scheduler dispatches each correctly.

### Integration

10. Two concurrent dialogs: open two via `emDialog::new` in the same slice, confirm independent WindowIds, independent trees, independent close lifecycles. Close one, the other still responds.
11. Popup alongside dialog: existing popup test (e.g., popup_materialization) with a dialog open simultaneously. Both materialize, both work, neither interferes.
12. Cross-window engine read (Framework scope): in a test, register a Framework engine that reads `ctx.windows.values()` tree contents. All trees present, no sentinels during its Cycle.
13. Take/put invariant: register a Toplevel-scoped engine in window W; during its Cycle, `ctx.windows[W].tree` is in sentinel state (empty), `ctx.windows[other_w].tree` is intact. Engine that *violates* this by indexing own window must panic (the invariant is tested via a violating test case confirming the panic).

### Regression

14. Phase 3.5 Tasks 2-4 (DlgPanel / DlgButton / DialogPrivateEngine) — unchanged behaviourally; re-verify after Task 4's `TreeLocation::Outer` registration updates to `PanelScope::Toplevel(_)`.
15. All existing golden tests (237 pass / 6 fail baseline) — no paint-path changes; preserve 237/6.
16. Full nextest — baseline 2483/0/9 at branch start; 3.5.A exit target `(2483 + N new tests) / 0 / 9`.

## Migration tasks (writing-plans will formalize)

1. **`PanelTree: Default` + `emWindow::{tree, take_tree, put_tree}`** — add field, ctor plumbing, helpers. Every existing emWindow ctor builds its own tree.
2. **Engine audit + classification sheet** — enumerate every `impl emEngine` + every `register_engine` call site; produce a diff mapping each to new `PanelScope` variant. Output committed as `docs/superpowers/notes/2026-04-22-phase-3-5-a-engine-classification.md`.
3. **`PanelScope::Framework` variant + `PanelScope::SubView` WindowId extension** — enum rewrite; update `resolve_view`.
4. **`TreeLocation` → `PanelScope` migration, `register_engine` signature change** — scheduler and all ~150 call sites. Follows the engine audit.
5. **Scheduler dispatch rewrite** — `DoTimeSlice` signature, `engine_scopes` map, take/put branch on scope. Replace `engine_locations`.
6. **`EngineCtx::tree: Option<&mut PanelTree>`** + per-engine Cycle body migration — add `.expect(...)` in window-scoped bodies; drop tree access from Framework bodies.
7. **Home window: `App::tree` deletion, home tree migration** — home emWindow built at startup with its own tree; all App-level `self.tree` call sites → `self.windows[home_id].tree` or take/put through destructured match.
8. **Popup migration** — `new_popup_pending` owns its tree; `emView::RawVisitAbs` drops `self.root`. Popup tests verified.
9. **Top-level install path** — `emWindow::new_top_level_pending`, `App::pending_top_level`, `App::dialog_windows`, `App::next_dialog_id`, `App::install_pending_top_level`, `DialogId` type, `App::dialog_window_mut`.
10. **Test-site migration sweep** — every `DoTimeSlice` call-site (~20) updates signature; every `register_engine` call-site (~150) updates arg.
11. **Phase 3.5 Task 4 re-registration fix** — `DialogPrivateEngine` was registered with `TreeLocation::Outer` in commit `1e393d2f`. Post-3.5.A it needs `PanelScope::Toplevel(dialog_window_id)`, with the deferred-registration pattern from §"Engine-registration chicken-and-egg" option (a).
12. **Full gate** — clippy + nextest + goldens.

## Exit criteria (scriptable)

- `rg -n 'pub tree: PanelTree' crates/emcore/src/emGUIFramework.rs` → 0 matches (`App::tree` deleted).
- `rg -n 'tree: PanelTree' crates/emcore/src/emWindow.rs` → ≥1 match.
- `rg -n 'impl Default for PanelTree' crates/emcore/src/emPanelTree.rs` → 1.
- `rg -n 'PanelScope::Framework' crates/emcore/src/` → ≥1.
- `rg -n 'TreeLocation' crates/emcore/src/` → 0 matches in source files (retired). Test-migration ledger confirms every site migrated.
- `rg -n 'pending_top_level' crates/emcore/src/emGUIFramework.rs` → ≥1.
- `rg -n 'install_pending_top_level' crates/emcore/src/emGUIFramework.rs` → ≥1.
- `rg -n 'dialog_windows' crates/emcore/src/emGUIFramework.rs` → ≥1.
- nextest: 2483 + N new / 0 / 9; goldens 237 / 6 preserved.

## Risks

### High

**R1. Scheduler dispatch rewrite ripple — borrow-checker fights.** The take/put pattern at Toplevel(wid) requires `windows.get_mut(&wid).unwrap().take_tree()` then building `EngineCtx { windows, ... }` with the `windows` map still borrowed. The same map from which the tree was just extracted is passed into ctx — Rust's borrow checker must accept that the extracted `PanelTree` (moved, now owned locally) is disjoint from the remaining `&mut HashMap`. This is structurally legal (move-out makes the source invalid, but we didn't move the emWindow, just its tree field). Needs first-cut implementation to confirm; may require destructuring emWindow via `let emWindow { tree, .. } = windows.get_mut(&wid).unwrap()` style split.

**Mitigation:** implementation's first scheduler-rewrite sub-task is a spike — get dispatch compiling end-to-end before extending. If destructuring doesn't work, fallback to `unsafe` pointer aliasing (existing precedent at emPanelCycleEngine.rs:93-109, emPanelScope.rs:79-84) — adds one more annotated `unsafe` site, justified by single-threaded + field-disjoint invariant.

### Medium

**R2. `PanelTree: Default` surprises.** If `PanelTree::new` does any load-bearing init (beyond SlotMap defaults), `Default` may not replicate it. Must be a cheap empty-but-valid tree; never surfaced to user-level callers.

**Mitigation:** Task 1 probe: read `PanelTree::new` body, verify `Default` = `new` or a cheap sentinel is correct. If `new` is expensive, define `Default::default = empty sentinel` (separate from `new`) and document invariant "the sentinel is never read by correct code."

**R3. `register_engine` site count is ~80 in code + ~40 in tests (~124 total).** Large mechanical migration. All sites must migrate atomically in one commit/PR to keep the codebase compileable. A missed site is a compile error (not a runtime panic) because the signature change forces the touch.

**Mitigation:** Rust's type system is the safety net — `register_engine` signature change forces every call-site to be touched at compile time. Migration commit that builds green guarantees all sites are addressed.

**R4. Popup migration behavioural regression.** Live code path with golden tests (popup_materialization, popup_cancel_before_materialize, etc.). Any shift in popup's tree identity or root_panel identity could change observable behaviour.

**Mitigation:** popup migration is its own sub-task with its own gate checkpoint. Full popup test suite must be green before proceeding to top-level install path. If a regression surfaces, bisect within the popup sub-task.

### Low

**R5. Engine-registration chicken-and-egg.** Covered in §"Engine-registration WindowId chicken-and-egg" — option (a) deferred registration is the chosen path; it's straightforward but requires `PendingTopLevel` to carry the not-yet-registered engine behaviour.

**R6. Framework-engine audit misclassification.** If a "Framework"-labeled engine actually needs its own tree (e.g., some state machine we missed), 3.5.A could leave it broken. The classification table (§6) is a best-effort read; definitive audit happens in Task 2.

**Mitigation:** audit deliverable is reviewable before migration begins (Task 2 gates Task 3+). Each engine's Cycle body is spot-read during the audit to confirm what it touches.

### Deferred (contingency)

**R7. If popup migration proves too disruptive mid-3.5.A,** split: 3.5.A ships dialog-only with `emWindow::tree: Option<PanelTree>` (Some for dialogs/home, None for popup), popup keeps sharing launching-view tree, and a follow-up Phase 3.5.B retires the Option. Open E026 at split. Prefer avoiding — unified scope is the clean exit. This branch is only activated if R4 bisect produces irreconcilable behavioural drift.

## Precedent compliance audit

| Principle | 3.5.A compliance |
|---|---|
| Port Ideology: C++ > goldens > Rust idiom | C++ `emView::RootPanel` per-view matched; popups gain their own tree (C++-correct); no idiom-driven deviation |
| File/Name Correspondence | `emWindow.rs` still 1:1 with `emWindow.h`; `emDialog.rs` unchanged this phase (3.5 Task 5 is consumer); no renames |
| `#[allow(...)]` banned | not used; `.expect(...)` used in place of `Option::unwrap` per CLAUDE.md |
| `Rc<RefCell>` banned | not introduced; `Rc<emContext>` pre-existing |
| `Arc`, `Mutex`, `Cow`, Any | not used |
| `unsafe` for convenience | not added; pre-existing sites (emPanelCycleEngine, emPanelScope) preserved with existing justifications |
| Single-threaded UI tree invariant | preserved |
| F64 in blend/coverage paths | no paint-path touched; goldens preserved |

## Open issues (punted to writing-plans)

- **`DialogId` allocation mechanism** — thread counter through ctor args vs `App::allocate_dialog_id(&mut self)` vs atomic on App. Decided during Task 9 per borrow constraints.
- **Deferred-registration of `DialogPrivateEngine`** — `PendingTopLevel` carries the not-yet-registered boxed behaviour. Exact field name + retrieval API per Task 11 implementation.
- **`PanelScope::SubView` extension semantics** — does the existing `SubView(PanelId)` without a WindowId migrate to `SubView { window_id: home_wid, outer_panel_id: pid, rest: Outer }`, or does SubView always require explicit WindowId? Picked during Task 3 scope refactor.
- **Materialize-failure handling** — graceful degradation path if `event_loop.create_window` fails. Design during Task 9.

## References

- C++ ground truth:
  - `~/git/eaglemode-0.96.4/include/emCore/emWindow.h:74-100` (ctor taking parentContext + flags)
  - `~/git/eaglemode-0.96.4/include/emCore/emView.h:59` (`class emView : public emContext`)
  - `~/git/eaglemode-0.96.4/include/emCore/emView.h:676` (`emPanel * RootPanel;` — per-view root ownership)
- Current Rust infrastructure:
  - `crates/emcore/src/emSubViewPanel.rs:23-105` — `sub_tree: PanelTree` precedent
  - `crates/emcore/src/emScheduler.rs:138-169` — `dispatch_with_resolved_tree` take/put precedent
  - `crates/emcore/src/emPanelCycleEngine.rs:82-110` — `dummy_tree` sentinel precedent
  - `crates/emcore/src/emPanelScope.rs` — PanelScope precedent
  - `crates/emcore/src/emEngineCtx.rs:44-82` — three-tier ctx precedent
  - `crates/emcore/src/emGUIFramework.rs:250-357` — popup materialize precedent
  - `crates/emcore/src/emView.rs:1926-1975` — popup pending-install enqueue precedent
- Design origin: 2026-04-19 port-ownership-rewrite (`docs/superpowers/specs/2026-04-19-port-ownership-rewrite-design.md` §3.1-§3.2)
- Phase-3 closeout baseline: `docs/superpowers/notes/2026-04-19-phase-3-closeout.md` (nextest 2476/0/9, goldens 237/6). Phase 3.5 interim: nextest 2483/0/9 at `1e393d2f`.
