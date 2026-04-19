# SP4.5 — emPanel engine-registration port (`run_panel_cycles` elimination)

**Date:** 2026-04-19
**Supersedes / resolves:** Closeout §8.1 item 16 (`run_panel_cycles` Rust-only divergence).
**Predecessor:** SP5 (`PanelData::View: Weak<RefCell<emView>>`, `init_panel_view` init path, `Rc<RefCell<emView>>` at owners).
**Authority order applied:** C++ source → golden parity → Rust idiom → convenience. Not negotiated.

---

## 1. Goal

Close the structural divergence "`emPanel` is not an `emEngine`" in the Rust port. Make every panel register itself with the scheduler the way C++ does (`class emPanel : public emEngine`, `emPanel.h:33`), so panel cycling runs through the normal per-view engine loop instead of a Rust-only side-channel.

Observable consequence: multi-window cycling uses the correct per-view `CurrentPixelTallness` by construction, fixing the last "pick-first-window" shortcut in `emGUIFramework::about_to_wait`.

**This is a full port, not a polishing pass.** SP4.5 rejects Option A (move `cycle_list` from `PanelTree` to `emView`) because Option A preserves the Rust-only `cycle_list` construct and only hides the divergence behind per-view ownership. The structural debt would survive and drift. CLAUDE.md mandates matching C++ architecture unless strictly impossible; the impossibility here is narrow and localized (see §5).

## 2. Ground truth (C++)

- `emPanel.h:33` — `class emPanel : public emEngine { ... };`
- `emPanel.cpp:1110` — `bool emPanel::Cycle() { return false; }` (virtual override of `emEngine::Cycle`)
- Throughout `emPanel.cpp` — `View.UpdateEngine->WakeUp()` for view wake, and subclasses call `WakeUp()` (inherited from `emEngine`) on themselves to enter the cycle rotation.
- `emView.h:708` — `emOwnPtr<UpdateEngineClass> UpdateEngine;` (one per view); panels are separate engines, also attached to the view's context → same scheduler.

Net: one scheduler, many engines, one-of-them-per-panel. Cycling is uniform; there is no dedicated panel loop.

## 3. Current Rust state (defective by default)

- `emEngine` is a trait (`emcore::emEngine`). Scheduler stores `Box<dyn emEngine>` behaviors keyed by `EngineId`.
- `PanelBehavior` is a *separate* trait with its own `Cycle(&mut PanelCtx) -> bool` default. Panels are **not** engines.
- `PanelTree::cycle_list: Vec<PanelId>` carries panels that want cycling; `PanelTree::Cycle(id)` registers, `PanelTree::cancel_cycle(id)` unregisters, `PanelTree::run_panel_cycles(tallness)` drains the list once per frame.
- `emGUIFramework::about_to_wait` computes `panel_cycle_pixel_tallness` from `self.windows.values().next()` and calls `self.tree.run_panel_cycles(panel_cycle_pixel_tallness)` — outside the scheduler loop entirely.
- Call sites using the Rust-only `ctx.tree.Cycle(id)` API (to be migrated): `emFileSelectionBox.rs:1206`, `emDirEntryPanel.rs:208`, `emFileLinkPanel.rs:129`, `emVirtualCosmos.rs:624`.

What this leaves wrong:
1. **Structural divergence** — `emPanel` is not an engine; there is no C++-mirror slot for self-scheduling.
2. **Observable bug** — multi-window picks the wrong view's tallness for every view except whichever wins `HashMap::values().next()`.
3. **Surface drift risk** — a Rust-only `cycle_list` API on `PanelTree` invites future code to reach for it instead of the scheduler.

## 4. Design

### 4.1 Per-panel engine, eagerly registered

`PanelData` gains `engine_id: Option<EngineId>`. Allocated during `PanelTree::init_panel_view(id, weak_view)` — the same SP5-established chicken-and-egg slot — and propagated to descendants identically. Deregistered in the panel-removal path (where `PanelData` leaves the slot-map).

Priority: `Priority::Medium` (mirror `emEngine::NORMAL_PRIORITY`).

Registration is **eager**, not lazy on first `WakeUp`. C++ allocates at construction; Rust cannot because `View` is a `Weak` set post-construction, but `init_panel_view` is the authoritative "this panel is now fully attached" moment. Using it keeps the invariant "an attached panel is always an engine."

### 4.2 Adapter engine (the forced divergence, minimized)

```rust
// DIVERGED: C++ emPanel inherits from emEngine directly (emPanel.h:33).
// In Rust, PanelBehavior is taken/put on PanelTree during Cycle, so a
// PanelBehavior trait-object cannot simultaneously live in the scheduler's
// Box<dyn emEngine> slot-map. This adapter is the minimum concession: one
// engine per panel, registered with the scheduler, routing Cycle() to the
// panel's PanelBehavior::Cycle via the tree's take/put path.
pub(crate) struct PanelCycleEngine {
    panel_id: PanelId,
    view: Weak<RefCell<emView>>,
}

impl emEngine for PanelCycleEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx<'_>) -> bool {
        let Some(view_rc) = self.view.upgrade() else { return false; };
        let tallness = view_rc.borrow().GetCurrentPixelTallness();
        let Some(mut behavior) = ctx.tree.take_behavior(self.panel_id) else { return false; };
        let mut pctx = PanelCtx::new(ctx.tree, self.panel_id, tallness);
        let stay = behavior.Cycle(&mut pctx);
        if ctx.tree.panels.contains_key(self.panel_id) {
            ctx.tree.put_behavior(self.panel_id, behavior);
        }
        stay
    }
}
```

The adapter is strictly plumbing. The observable behavior — per-panel Cycle driven by the scheduler's priority/parity loop, using the correct view's tallness — is C++-faithful.

### 4.3 WakeUp / Sleep surface

Added to `PanelCtx`:
- `wake_up(&mut self)` — routes to `scheduler.wake_up(self.panel.engine_id.unwrap())`. C++ parity: `WakeUp()` inherited from `emEngine`, called from inside `emPanel::*`.
- `wake_up_panel(&mut self, id: PanelId)` — for sites that schedule a different panel (mirrors C++ `child->WakeUp()` pattern). Four current call sites.
- `sleep(&mut self)` — routes to `scheduler.sleep(self.panel.engine_id.unwrap())`. Optional: C++ engines sleep implicitly by returning `false` from `Cycle`; Rust parity is the same, so this may stay unused. Include only if there's a concrete caller; omit otherwise (YAGNI).

Deletions (atomic, no compatibility shims):
- `PanelTree::cycle_list` field.
- `PanelTree::Cycle(id)` method.
- `PanelTree::cancel_cycle(id)` method.
- `PanelTree::run_panel_cycles(tallness)` method.
- `emGUIFramework::about_to_wait` block computing `panel_cycle_pixel_tallness` and the `self.tree.run_panel_cycles(...)` call.

Call-site migration: the 4 `ctx.tree.Cycle(child_id)` / `ctx.tree.Cycle(ctx.id)` sites rewrite to `ctx.wake_up_panel(child_id)` / `ctx.wake_up()`.

### 4.4 Lifecycle

| Event | Action |
|---|---|
| Panel inserted into tree (no view yet) | `engine_id = None` |
| `init_panel_view(id, weak)` runs (SP5 path) | Register adapter engine with scheduler; store `engine_id = Some(eid)` |
| Panel Cycle body calls `ctx.wake_up()` | Scheduler wakes the engine; it enters the next slice |
| Panel `PanelBehavior::Cycle` returns `false` | Scheduler sleeps the engine (C++ parity) |
| Panel removed from tree | Scheduler `remove_engine(eid)`; `PanelData` drops |
| View dropped (Weak upgrade fails) | Adapter `Cycle` returns `false`; scheduler sleeps it; engine is removed with the panel normally |

Scheduler `Drop` already asserts "no dangling engines." That invariant is preserved — panel-removal deregistration is mandatory and tested.

### 4.5 Re-entrancy

Panel `Cycle` running can:
- Wake other panels → `ctx.wake_up_panel(id)` → `scheduler.wake_up(eid)`. Scheduler already supports mid-slice wakes (`current_awake_idx` bump in `emScheduler.rs:180`).
- Fire signals → `ctx.fire(sig)` through `EngineCtx`. Unchanged.
- Mutate tree (add/remove panels) → uses existing `PanelCtx` tree access. Unchanged from today.

No new `RefCell` borrow hazards: the adapter holds no long-lived `emView` borrow (tallness is read once and dropped before `behavior.Cycle`).

## 5. What stays DIVERGED after SP4.5

Exactly one marker, on `PanelCycleEngine`, citing `emPanel.h:33` and the slot-map/single-inheritance reason. No `PHASE-*-FOLLOWUP:` markers. No Rust-only "cycle list" concept anywhere in the tree.

## 6. Tests

### 6.1 New

1. `sp4_5_panel_cycle_uses_per_view_pixel_tallness` — two views with distinct `SetCurrentPixelTallness`; one panel per view, both registered for cycling. Drive a single `DoTimeSlice`. Assert each panel's `Cycle` saw its own view's tallness. Direct SP5 parity in shape.
2. `sp4_5_panel_engine_registered_at_init_panel_view` — build a panel, call `init_panel_view`, inspect `PanelData::engine_id.is_some()`. Build a descendant, assert the same.
3. `sp4_5_panel_engine_deregistered_on_panel_removal` — register, remove panel, assert scheduler no longer has the engine (and `Drop` on `EngineScheduler` post-teardown does not panic).
4. `sp4_5_wake_up_from_cycle_reschedules_same_slice` — panel `Cycle` calls `ctx.wake_up_panel(other_id)` on a sibling; both should run in the same `DoTimeSlice`. Tests the scheduler mid-slice wake path is exercised via the new API.

### 6.2 Migrated

No bulk test migration expected — the 4 production `ctx.tree.Cycle(id)` call sites are the only surface touched. Any test that constructs a panel and drives cycling via `run_panel_cycles` must switch to driving via `scheduler.do_time_slice(...)`; expected handful.

### 6.3 Targets

- Nextest: **2434 → 2438** (+4 new). Zero regressions.
- Golden: **237/6** unchanged (no pixel path touched).
- Smoke `timeout 20 cargo run --release --bin eaglemode` exits 124/143.

## 7. Out of scope

- Merging `PanelBehavior` and `emEngine` into one trait. (Separate trait is load-bearing for the take/put ownership model; merging it is orthogonal structural work.)
- `emSubViewPanel` / `emVisitingViewAnimator`: already direct engines — untouched.
- SP7 `emContext` threading: orthogonal. SP4.5 does not acquire panels' context via `emContext`; it reaches the scheduler through `Weak<RefCell<emView>>`, which is enough.
- `emGUIFramework` residual "pick first window" patterns not tied to `run_panel_cycles`. If any survive, SP4.5 classifies them in the closeout as remaining debt; it does not fix them here.

## 8. Rollout phases

| Phase | Scope | Gate |
|---|---|---|
| 1 | `PanelCycleEngine` adapter type + `PanelData::engine_id` field (unused) | `cargo check` green |
| 2 | Eager registration in `init_panel_view` + descendant propagation; deregistration in panel-removal path | 2 new lifecycle tests pass |
| 3 | Add `PanelCtx::wake_up` / `wake_up_panel`; migrate 4 call sites | Full suite green (still via old `run_panel_cycles`) |
| 4 | Delete `PanelTree::cycle_list`, `Cycle`, `cancel_cycle`, `run_panel_cycles` | Compiles only because phase 3 migrated all callers |
| 5 | Delete `emGUIFramework::about_to_wait` tallness block + `run_panel_cycles` call | Smoke + nextest + golden parity |
| 6 | Multi-view tallness test + mid-slice wake test | 2434 → 2438; close out |

Each phase is a single commit (or tightly coupled pair). No phase leaves the tree non-compiling.

## 9. Risks

| Risk | Mitigation |
|---|---|
| Adapter `take_behavior` during `Cycle` collides with another path also taking the behavior | Audit `take_behavior` call sites; all current callers finish synchronously before returning control. Add a debug-only "already taken" panic if any path re-enters. |
| Panel removal path misses `remove_engine(eid)` → scheduler-drop panic | Single centralized removal helper; covered by deregistration test. |
| `init_panel_view` invoked twice (e.g. re-parenting) double-registers | `init_panel_view` early-returns if `engine_id.is_some()`; documented. |
| Scheduler `wake_up` against a removed `EngineId` | `remove_engine` clears from `wake_queues`; `wake_up` on unknown id is a no-op in `EngineScheduler`. Safe. |
| Test-harness views without a scheduler | Adapter `Cycle` early-return on `view.upgrade() == None` handles the teardown case; harnesses that never register engines never trigger it. |

## 10. Discovery trail

- Closeout §8.1 item 16 (added 2026-04-18 during SP5 brainstorming) defined SP4.5 with Option A / Option B alternatives and a "recommended: Option A, default" note.
- 2026-04-19 directive (user): CLAUDE.md compliance is the sole priority; Option A is polishing and preserves structural divergence; SP4.5 must do the full port.
- Brainstorming 2026-04-19: confirmed B is feasible given SP5's `Weak<RefCell<emView>>` back-ref and the existing per-process scheduler; the only forced concession is the adapter (single-inheritance + slot-map ownership), documented at one site.

End of spec.
