# SP4 — emView::Update engine-only routing + Phase-8 test promotion

**Date:** 2026-04-18
**Sub-project:** SP4 of the emView subsystem closeout (see `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md` §8.0).
**Closes:** §8.1 item 14 (scheduler re-entrant borrow — full call-tree, not just line 2343) and item 11 (Phase-8 test promotion). Item 14 blocks item 11; one combined spec.
**Scope boundary:** align `emView::Update` dispatch with C++'s single-caller model, eliminate ALL re-entrant scheduler-borrow hazards on Update's reachable call tree (not just the popup-close probe), and promote `test_phase8_popup_close_signal_zooms_out` to a single-engine end-to-end run. Does **not** touch notice dispatch (SP5) or `emContext` threading (SP7).

---

## 1. Background

### 1.1 C++ reference (ground truth)

- `emView::Update()` is called from exactly one site: `UpdateEngineClass::Cycle()` at `emView.cpp:2523`.
- Every other code path that needs `Update` to run schedules it via `UpdateEngine->WakeUp()` (`emView.cpp:84, 173, 307, 1013, 1288, 1805`, and the ctor at `:84`).
- `emView`'s ctor wakes the engine immediately (`emView.cpp:84`) so the first frame runs.

### 1.2 Rust drift (items to fix)

1. **Direct non-engine call.** `emGUIFramework::about_to_wait:594` calls `win.view_mut().update(tree)` unconditionally every frame, bypassing the scheduler. C++ has no such path. The Rust wrapper `emView::update()` at `emView.rs:3845` exists *only* to serve this site; it wraps `self.Update(tree)` + a post-hoc `SetActivePanelBestPossible` fixup.
2. **Missing ctor-time wake.** `attach_to_scheduler` (`emView.rs:3044`) registers `UpdateEngineClass` but does not wake it. C++ `emView::emView` does (`emView.cpp:84`).
3. **Re-entrant scheduler borrows throughout Update's call tree.** The popup-close probe at `emView.rs:2343` is one instance; it is not the only one. Audit of `emView.rs` identifies ~10 `self.scheduler.borrow_mut()` / `self.scheduler.borrow()` call sites (lines 1023, 1476, 1723, 1757, 1848, 1875, 1876, 2343, 2967, 3075, 3261, 3318). Several are on Update's reachable call tree:
   - `:1848 + :1875` — popup teardown (`RawVisitAbs`): reachable via `Update → ZoomOut → RawZoomOut → RawVisit → RawVisitAbs`.
   - `:1476` — `set_active_panel` fires `control_panel_signal`: reachable via `Update's SVPChoiceInvalid drain → SetActivePanelBestPossible → set_active_panel`.
   - `:1023, :2967, :3261, :3075` — additional `fire` / `WakeUpUpdateEngine` sites within the drain-loop.
   Each of these panics re-entrantly if `Update` runs inside `DoTimeSlice` (because `emGUIFramework.rs:490` holds the outer `sched.borrow_mut()` across the whole slice). Today they are latent because `Update` runs *outside* `DoTimeSlice` via item 1 — but as soon as we unify on the engine path, **the `:2343` panic gets relocated, not eliminated**. A narrow fix of `:2343` alone leaves the rest of Update's call tree broken.
4. **Bare-view test unreachable from engine.** `UpdateEngineClass::Cycle` resolves the view via `ctx.windows.get(&self.window_id)`; bare-view tests have no window registered, so a principled single-engine promotion of `test_phase8_popup_close_signal_zooms_out` can't receive the cycle call.

Items 1, 2, and 3 are a single defect viewed from three angles: Rust runs `Update` outside its one C++ caller, and its one correct caller has a whole call-tree of scheduler-borrow hazards. Fix the caller model AND the hazard category uniformly.

### 1.3 Classification under CLAUDE.md

- Item 1 (`:594` direct call): **Rust-inertia drift**. Not forced — the engine infrastructure works. Not preserved design intent — C++ does the opposite. Must be removed.
- Item 2 (missing ctor wake): **silent drift from C++ ctor**. Must be added.
- Item 3 (re-entrant borrow hazard across the call tree): **idiom adaptation forced by `RefCell<EngineScheduler>`**. C++ has no RefCell — `Scheduler.Fire(sig)` during Update just works. Rust's RefCell wrapping is an ownership-model necessity (Rust can't aliasingly mutate the scheduler while inside `DoTimeSlice`'s top-level borrow). The adaptation — deferring scheduler-writes issued during Update to a queue drained immediately after `Update` returns in the same `Cycle` — preserves observable "what fires when" (all queued ops land in the same time slice, visible to engines cycling later in that slice) while honoring Rust's borrow rules. Under CLAUDE.md's classification: idiom adaptation, not a divergence. Still marked at the queue definition for discoverability.
- Item 4 (bare-view test): **forced, for tests only.** Production `UpdateEngineClass::Cycle` correctly requires a window; fixing this at the engine level would diverge from C++. Resolution lands in the test, not the engine.

---

## 2. Design decisions

### 2.1 Route `emView::Update` through the engine only

Delete `emGUIFramework::about_to_wait:594`'s `win.view_mut().update(tree)` call. `Update` runs only via `UpdateEngineClass::Cycle`, which already runs inside `DoTimeSlice` at `emGUIFramework.rs:491`. Every C++ `WakeUp()` site already has a Rust `WakeUpUpdateEngine()` counterpart (`emView.rs:1757, 1845, 2088, 3372, 3790`), so mutation paths already schedule the engine correctly.

The `emView::update()` wrapper (`emView.rs:3845-3859`) goes away. Its two pieces:
- `self.Update(tree)` — now reached via engine only.
- Post-hoc `SetActivePanelBestPossible(tree)` gated on active-panel invariants — this is drift. C++ calls `SetActivePanelBestPossible()` at three sites (`emView.cpp:780, 800, 901`: end of `Scroll`, `Zoom`, `ZoomOut`), not after `Update`. Resolution: append `SetActivePanelBestPossible(tree)` to the end of Rust's `Scroll` (`emView.rs:1123`), `Zoom` (`:1086`), and `ZoomOut` (`:1251`), each with a cited C++ line. The Rust-only `need_reselect`/`viewport_changed` gates in the wrapper drop; C++ has no such guard, the method is cheap, and the guard masks inherited drift.

### 2.2 Wake the update engine at attach time

Append `self.WakeUpUpdateEngine();` to the end of `attach_to_scheduler` (after `self.update_engine_id = Some(engine_id);` at `emView.rs:3067`). Matches C++ `emView::emView:84`. Ensures the first `DoTimeSlice` cycles `Update` at least once.

### 2.3 Defer scheduler-writes via a per-view queue; probe popup-close signal in `UpdateEngineClass::Cycle`

**C++ context.** C++ `emView` is itself an `emEngine` (via `class emView : public emContext` and `class emContext : public emEngine`, `emContext.h:44`). `emView::Update` at `emView.cpp:1299` calls `IsSignaled(close_signal)` against `emView`'s own engine clock. Within Update's call tree, C++ code freely calls `Scheduler.Fire(sig)`, `Engine.WakeUp()`, `Scheduler.Connect(sig, engine)`, etc. — all work because C++ has no `RefCell` wrapping the scheduler.

**Rust constraint.** Rust's `emView` holds `Option<Rc<RefCell<EngineScheduler>>>`. During `DoTimeSlice`, `emGUIFramework.rs:490` holds the outer `sched.borrow_mut()` across the entire slice. Any inner `self.scheduler.borrow_mut()` inside Update's call tree panics re-entrantly.

**Resolution — two parts.**

**Part A: Popup-close probe moves to `UpdateEngineClass::Cycle`.** Cycle holds `&mut EngineCtx` directly (no RefCell borrow needed) and exposes `ctx.IsSignaled(sig)`. We pre-compute the probe there and stash it in a transient `emView::close_signal_pending` field. Update reads and clears it with `std::mem::take`. No `Update` signature change.

Field added to `emView`:
```rust
/// Set by `UpdateEngineClass::Cycle` from `ctx.IsSignaled(close_signal)`
/// before calling `Update`; read and cleared at the top of `Update`.
/// Stands in for C++ `IsSignaled(PopupWindow->GetCloseSignal())` in
/// `emView::Update` (emView.cpp:1299). DIVERGED: C++ emView inherits
/// from emEngine (via emContext), so the IsSignaled call there is
/// against emView's own clock. Rust emView is not an emEngine (SP7
/// will revisit); the nearest correct clock is UpdateEngine's, and
/// UpdateEngineClass::Cycle is the natural site to observe it.
pub(crate) close_signal_pending: bool,
```

**Part B: Deferred-scheduler-ops queue for writes issued from inside Update.** Introduce a `SchedOp` enum covering the five operations actually used in emView's scheduler-borrow sites:

```rust
/// Deferred scheduler operation. Variants cover exactly the scheduler
/// writes emView issues from inside its Update-reachable call tree —
/// no generic dispatch, no data; every variant carries only the IDs the
/// target scheduler method needs. IDIOM: C++ calls `Scheduler.X(...)`
/// inline during Update; Rust cannot because `emGUIFramework::about_to_wait`
/// holds `sched.borrow_mut()` across `DoTimeSlice`. Deferring queued ops
/// to immediately after `Update` returns (in the same `Cycle`, same time
/// slice) preserves observable "what fires when": all fired signals,
/// wakes, and (dis)connects land in the same slice, visible to any
/// engine that cycles after UpdateEngineClass in this slice.
pub(crate) enum SchedOp {
    Fire(SignalId),
    WakeUp(EngineId),
    Connect(SignalId, EngineId),
    Disconnect(SignalId, EngineId),
    RemoveSignal(SignalId),
}
```

Field added to `emView`:
```rust
/// Queue of scheduler operations issued from inside Update's call tree.
/// Non-empty only between a Cycle entry and its post-Update drain.
/// See SchedOp docs.
pub(crate) pending_sched_ops: Vec<SchedOp>,
```

Helper on `emView`:
```rust
/// Apply a scheduler op: execute immediately if the scheduler is not
/// currently borrowed (the common, non-engine-path case — callers
/// outside DoTimeSlice), otherwise enqueue for drain by
/// UpdateEngineClass::Cycle. Uses try_borrow_mut to detect the engine
/// path without an explicit "inside Update" flag.
pub(crate) fn queue_or_apply_sched_op(&mut self, op: SchedOp) {
    if let Some(sched_rc) = self.scheduler.as_ref() {
        match sched_rc.try_borrow_mut() {
            Ok(mut sched) => op.apply_to(&mut *sched),
            Err(_) => self.pending_sched_ops.push(op),
        }
    }
    // No scheduler attached (unit-test bare view): silently drop.
    // All SchedOp variants are no-ops without a scheduler.
}
```

`UpdateEngineClass::Cycle` body:
```rust
fn Cycle(&mut self, ctx: &mut super::emEngine::EngineCtx<'_>) -> bool {
    if let Some(win_rc) = ctx.windows.get(&self.window_id) {
        let win_rc = Rc::clone(win_rc);
        let mut win = win_rc.borrow_mut();
        let view = win.view_mut();
        // Part A: popup-close probe (C++ emView.cpp:1299).
        if let Some(popup) = view.PopupWindow.as_ref() {
            let close_sig = popup.borrow().close_signal;
            view.close_signal_pending = ctx.IsSignaled(close_sig);
        }
        // Run Update; any scheduler writes it issues enqueue onto
        // view.pending_sched_ops via queue_or_apply_sched_op.
        view.Update(ctx.tree);
        // Drain deferred ops through ctx (direct &mut access, no
        // RefCell borrow needed).
        for op in view.pending_sched_ops.drain(..) {
            op.apply_via_ctx(ctx);
        }
    }
    false
}
```

`SchedOp::apply_to` and `SchedOp::apply_via_ctx` are two small dispatchers that wrap `EngineScheduler::{fire, wake_up, connect, disconnect, remove_signal}` and the equivalent `EngineCtx` methods. `EngineCtx` today exposes `fire` and `wake_up`; add `connect`, `disconnect`, `remove_signal` to match (each a 3–5 line forward to `self.scheduler.X(...)`).

**Call-site migration.** Every `self.scheduler.as_ref().borrow_mut().X(...)` expression in `emView.rs` is rewritten:

- `sched.borrow_mut().fire(sig)` → `self.queue_or_apply_sched_op(SchedOp::Fire(sig))`
- `sched.borrow_mut().wake_up(id)` → `self.queue_or_apply_sched_op(SchedOp::WakeUp(id))`
- `sched.borrow_mut().connect(sig, id)` → `self.queue_or_apply_sched_op(SchedOp::Connect(sig, id))`
- `sched.borrow_mut().disconnect(sig, id)` → `self.queue_or_apply_sched_op(SchedOp::Disconnect(sig, id))`
- `sched.borrow_mut().remove_signal(sig)` → `self.queue_or_apply_sched_op(SchedOp::RemoveSignal(sig))`

Expected sites touched (from the audit, excluding purely non-Update paths and the popup-close probe at `:2343` which is eliminated in Part A):

| Line | Op | Reachable from Update? |
|---|---|---|
| 1023 | Fire | Possibly — in SetGeometry-adjacent code |
| 1476 | Fire (control_panel_signal) | Yes — via SetActivePanelBestPossible |
| 1848 | Disconnect + RemoveSignal | Yes — popup teardown in RawVisitAbs |
| 1875/1876 | Fire (geometry_signal) | Yes — popup teardown |
| 2967 | Fire | Possibly |
| 3075 | WakeUp (inside `WakeUpUpdateEngine`) | Yes — many mutators reached from Update |
| 3261 | Fire (geometry_signal) | Possibly |
| 3318 | Signal(s) in EOI path | Possibly |

**Uniform migration, not per-site reachability analysis.** Rather than audit each site to decide "reachable or not," convert *every* scheduler-borrow site in `emView.rs` — including those at `:1723, :1757` (popup creation, not reachable from Update) — to the queue-or-apply path. `try_borrow_mut` will succeed immediately on those non-engine-path sites, applying inline with zero queue overhead. Benefits: one uniform pattern, no reachability claims to maintain as code evolves, no latent hazards hiding in newly-added Update descendants.

Sites left untouched by this spec:
- `attach_to_scheduler` (`:3050`) — setup-only, never reached from inside a time slice by construction; uses `borrow_mut` on a local `Rc` parameter before storing in `self.scheduler`.
- Scheduler-borrows in `#[cfg(test)]` test-harness code — those tests own their scheduler lifecycle explicitly; no change.

**Observational-equivalence argument.** C++ fires/wakes/connects happen inline during Update; receivers see the effects in their subsequent Cycles (never in the middle of Update itself, because C++ is single-threaded and the scheduler doesn't re-enter engines mid-Cycle). Rust deferred-to-end-of-Cycle fires/wakes/connects land in the same time slice. Any engine that cycles after UpdateEngineClass in the same priority wave (or in lower priorities) sees the effects — exactly as in C++. Same-slice wake-into-same-priority (via `current_awake_idx` bump-up in `EngineCtxInner`) behavior: C++ triggers bump-up inline; Rust triggers it at end of Cycle; both re-scan from the affected priority in the remainder of the current slice. Observationally identical within the time-slice granularity the scheduler exposes.

**Drain site.** `UpdateEngineClass::Cycle` is the sole draining site. Other emEngine `Cycle` impls (e.g., `VisitingVAEngineClass::Cycle`, `EOIEngineClass::Cycle`) do not invoke `emView::Update` and so do not queue ops — no drain needed there. If a future engine does call `Update`, its `Cycle` must drain `pending_sched_ops` too; document this at the queue field.

Rejected alternatives:
- *Add `ctx: &mut EngineCtx` to `Update` signature.* Cascades through 141 `view.Update(&mut tree)` call sites across 20+ files. All would need `Option<&mut ctx>` plumbing. The queue approach stays local to `emView.rs`.
- *Single-purpose `close_signal_pending` field only (the option from pre-review spec v1).* Leaves `:1476`, `:1848`, `:1875`, `:1023`, `:2967`, `:3075`, `:3261`, `:3318` as re-entrant-borrow landmines once Update runs inside `DoTimeSlice`. Phase-8 single-engine test would panic on the first popup teardown via the engine path.
- *Keep `:594` direct call, accept that engine path is aspirational.* Spec §1.3's "Rust is defective by default" rule says no.

### 2.4 Test call sites unchanged

With §2.3's cached-field approach, `Update`'s signature does not change. All 141 `.Update(&mut tree)` call sites across tests and production remain as-is. Tests that do not attach a scheduler will simply never observe `close_signal_pending == true` (no engine ever writes it), which is correct — those tests don't exercise popup close.

### 2.5 Phase-8 test promotion

`test_phase8_popup_close_signal_zooms_out` currently asserts across two engines (Half A + Half B with a dummy engine), per closeout doc §5.1 item 5. Rewrite as a single run:

1. Build a scheduler + minimal `emWindow` (test harness — C++ needs one too) + `emView` attached.
2. Push a popup (sets `PopupWindow`, connects `close_signal` → update engine via `SwapViewPorts`).
3. Fire the popup's `close_signal`.
4. Call `scheduler.DoTimeSlice(&mut tree, &mut windows)` once.
5. Assert `popped_up == false` and the zoom state matches the post-`ZoomOut` expectation.

The "minimal `emWindow`" requirement is item 4 from §1.2. Add a test-only `emWindow::new_for_test(scheduler, ...)` constructor under the existing `test-support` feature. It produces a window with no GPU/winit surface — just enough to satisfy `ctx.windows.get(&window_id)` in `UpdateEngineClass::Cycle` and expose `view_mut()`. The integration test owns the HashMap it passes into `DoTimeSlice`.

The current inline test at `emView.rs` is replaced, not extended. Delete Half A + Half B; the new single-engine test supersedes both.

### 2.6 Scope non-goals

- Do not touch `VisitingVAEngineClass::Cycle` (same window-lookup pattern; not blocking anything in SP4).
- Do not add non-`emEngine` wake paths.
- Do not restructure `SetActivePanelBestPossible`'s call semantics beyond the relocation into `Scroll`/`Zoom`/`ZoomOut` specified in §2.1.

---

## 3. Blast radius

| Touch | Count | Nature |
|---|---|---|
| `emGUIFramework::about_to_wait:594` | 1 line | Delete direct `update()` call |
| `emView::update` wrapper | 1 method | Delete after audit |
| `emView::Update` signature | unchanged | Cached-field + queue approach keeps signature stable |
| `emView::close_signal_pending` field | new | 1 line + doc |
| `emView::pending_sched_ops` field + `SchedOp` enum | new | ~30 lines |
| `emView::queue_or_apply_sched_op` helper | new | ~10 lines |
| `SchedOp::apply_to` + `apply_via_ctx` | new | ~30 lines |
| `EngineCtx::connect` / `disconnect` / `remove_signal` | new | ~15 lines (forward to `self.scheduler.X(...)`) |
| `UpdateEngineClass::Cycle` | ~15 lines | Pre-probe close_signal, run Update, drain pending_sched_ops |
| `attach_to_scheduler` | 1 line | Add `self.WakeUpUpdateEngine()` |
| Popup-close probe in `Update` | ~12 lines → 4 lines | Replace scheduler borrow with `mem::take(&mut self.close_signal_pending)` |
| Scheduler-borrow call-site migration in `emView.rs` | ~10 sites | Mechanical: `sched.borrow_mut().X(...)` → `self.queue_or_apply_sched_op(SchedOp::X(...))` |
| `emWindow::new_for_test` | new test-only ctor | ~30 lines |
| Phase-8 test rewrite | 1 test | ~50 lines net |
| `SetActivePanelBestPossible` relocation into `Scroll`/`Zoom`/`ZoomOut` | 3 lines | Mechanical append per C++ `emView.cpp:780, 800, 901` |

Expected total: ~200 lines changed, mostly additive (enum + impls) plus ~10 mechanical call-site rewrites. One `BUG` comment removed (`:2324-2335`). One new `DIVERGED:` comment on `close_signal_pending`. One `IDIOM:` comment on `SchedOp` documenting the RefCell-forced adaptation.

---

## 4. Risks

| Risk | Mitigation |
|---|---|
| Moving `SetActivePanelBestPossible` from post-`Update` to end-of-`Scroll`/`Zoom`/`ZoomOut` changes ordering (notice dispatch vs. active-panel reselection) in a way a test/golden depends on | C++ itself uses end-of-mutator ordering; any divergence exposed is Rust drift being closed. Phase 4 runs nextest + golden; investigate any new failure under that frame |
| Tests that relied on `emView::update()` wrapper's post-hoc `SetActivePanelBestPossible` fail once the wrapper is deleted | Mitigated by §2.1 relocation of that call into `Scroll`/`Zoom`/`ZoomOut`; any remaining failure means the test was exercising drift and should be rewritten to match C++ ordering |
| Same-slice wake-propagation timing differs (queued wake drained at end of Cycle vs. inline during Update) such that a receiver engine that would have cycled mid-wave in C++ cycles one wave later in Rust | The scheduler's `current_awake_idx` bump-up happens equivalently at drain time; lower-priority engines still see the wake in the remaining slice. Mitigation: add a targeted test (see §5) that fires a signal from inside Update and asserts a lower-priority receiver cycles the same slice |
| A future non-Update code path in `emView.rs` calls `queue_or_apply_sched_op` but is not a descendant of `UpdateEngineClass::Cycle`, leaving ops permanently queued | Non-engine paths take the `try_borrow_mut().Ok(_)` arm and execute inline. The queue is only populated under an outer `borrow_mut()`, which outside `DoTimeSlice` is contradictory (nothing else holds the scheduler borrow across a method return). Document the invariant at the queue field |
| Some `self.scheduler.borrow*` site in `emView.rs` is missed during migration and continues to panic | Phase 2 ends with `grep -n "self\\.scheduler\\.\\(as_ref()\\)\\?\\.\\?borrow" crates/emcore/src/emView.rs` — empty result outside `attach_to_scheduler` and `#[cfg(test)]` blocks is a blocking gate |
| Waking the engine at `attach_to_scheduler` changes observable frame-one behavior | C++ does this at ctor; any observable difference is a pre-existing divergence we're closing, not introducing |
| Popup teardown path in production depends on engine not being woken | The W3 closeout verified popup teardown runs end-to-end through the engine (§3.6 R5); the wake is already expected |

---

## 5. Success criteria

1. `cargo check` + `cargo clippy -- -D warnings` clean.
2. `cargo-nextest ntr` — 2429/2429 (baseline) or higher (SP4 adds one test, replaces two halves of another — net +0 or +1).
3. `cargo test --test golden -- --test-threads=1` — 237/243 (baseline parity; same 6 pre-existing failures).
4. `timeout 20 cargo run --release --bin eaglemode` exits 124/143 (stayed alive).
5. `grep -n "BUG (tracked as" crates/emcore/src/emView.rs` returns nothing (§8.1 item 14 marker deleted).
6. `grep -n "win.view_mut().update(tree)" crates/emcore/src/emGUIFramework.rs` returns nothing.
7. `grep -n "fn update\b" crates/emcore/src/emView.rs` returns nothing (wrapper deleted).
8. `test_phase8_popup_close_signal_zooms_out` runs `DoTimeSlice` exactly once and is documented as single-engine.
9. `grep -nE 'self\.scheduler.*borrow' crates/emcore/src/emView.rs` returns only `attach_to_scheduler` and `#[cfg(test)]` lines (call-site migration complete; no unmigrated borrows remain).
10. New test `sp4_signal_fired_from_update_reaches_receiver_same_slice`: a receiver engine with `AddWakeUpSignal(sig)` at lower priority than `UpdateEngineClass` cycles in the same `DoTimeSlice` call that the Update-issued `Fire(sig)` ran in. Guards against the same-slice-propagation risk.

---

## 6. Out of scope — deferred to successor sub-projects

- **Notice dispatch per-view** (SP5): `emGUIFramework.rs:517-522`'s `pixel_tallness` single-window shortcut stays.
- **emContext threading** (SP7): `emView::new` signature untouched.
- **W3 surface de-dup** (SP6): unchanged.

End of SP4 design.
