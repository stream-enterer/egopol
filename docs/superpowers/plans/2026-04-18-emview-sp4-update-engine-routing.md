# SP4 Implementation Plan — emView::Update engine-only routing + scheduler-op deferral

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align Rust `emView::Update` dispatch with C++'s single-caller model, eliminate ALL re-entrant scheduler-borrow hazards across Update's call tree (not just the one known site) via a per-view deferred-ops queue, and promote `test_phase8_popup_close_signal_zooms_out` to a single-engine end-to-end run.

**Architecture:** Three C++ divergences drive today's latent re-entrant-borrow panics: (i) `emGUIFramework::about_to_wait:594` calls `view.update(tree)` directly every frame, bypassing `UpdateEngineClass::Cycle`; (ii) `attach_to_scheduler` omits the ctor-time `WakeUpUpdateEngine` C++ does at `emView.cpp:84`; (iii) ~10 `self.scheduler.borrow_mut()` sites in `emView.rs` are reached from Update's descendants, any of which would panic once Update runs inside `DoTimeSlice`. Fix (i)+(ii) to match C++ single-caller model. Fix (iii) by converting every scheduler-borrow site in `emView.rs` to a `queue_or_apply_sched_op` helper that uses `try_borrow_mut` to detect whether the scheduler is already held: if free (non-engine path), apply inline; if held (inside DoTimeSlice), push onto `emView::pending_sched_ops`. `UpdateEngineClass::Cycle` drains the queue via its `EngineCtx` immediately after `Update` returns — same time slice, observationally equivalent to C++'s inline scheduler writes.

**Tech Stack:** Rust 2021; `slotmap`, `winit`, `wgpu`; existing `emcore` + `eaglemode` crate split; `cargo-nextest` + `cargo test --test golden`.

**Spec:** `docs/superpowers/specs/2026-04-18-emview-sp4-update-engine-routing-design.md`.

---

## File map

- **Modify** `crates/emcore/src/emView.rs` — add `SchedOp` enum, `pending_sched_ops` field, `close_signal_pending` field, `queue_or_apply_sched_op` helper; migrate ~10 `sched.borrow_mut()` call sites; rewrite `UpdateEngineClass::Cycle`; replace popup-close probe in `Update`; add wake at end of `attach_to_scheduler`; append `SetActivePanelBestPossible(tree)` to `Scroll`/`Zoom`/`ZoomOut`; delete `update()` wrapper; rewrite Phase-8 test; add same-slice-propagation test.
- **Modify** `crates/emcore/src/emEngine.rs` — add `connect`, `disconnect`, `remove_signal` methods to `EngineCtx` (forwarding to `self.scheduler.X(...)`).
- **Modify** `crates/emcore/src/emGUIFramework.rs:594` — delete direct `win.view_mut().update(tree)` call.
- **Modify** `crates/emcore/src/emWindow.rs` — add `#[cfg(any(test, feature = "test-support"))] fn new_for_test(...)` constructor (no GPU/winit surface).

---

## Phase 0 — Baseline capture

Snapshot green state so regressions are attributable to SP4.

### Task 0.1: Capture baseline test counts

- [ ] **Step 1:** Run nextest.

  Run: `cargo-nextest ntr 2>&1 | tail -20`
  Expected: `2429 tests run: 2429 passed (9 skipped), 0 failed`.

- [ ] **Step 2:** Run golden.

  Run: `cargo test --test golden -- --test-threads=1 2>&1 | tail -5`
  Expected: `237 passed; 6 failed` (baseline, same pre-existing failures).

- [ ] **Step 3:** Smoke-run.

  Run: `timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"`
  Expected: `exit=124` or `exit=143`.

- [ ] **Step 4:** Record these three numbers in working notes. Phases 1–6 must match or improve each.

- [ ] **Step 5:** Enumerate scheduler-borrow sites — this is the Phase 2 migration checklist.

  Run: `grep -nE 'self\.scheduler.*borrow|sched(uler)?\.borrow' crates/emcore/src/emView.rs | grep -v '#\[cfg(test)\]' | grep -v 'fn attach_to_scheduler'`
  Expected: a list of ~10 lines. Copy into working notes as `[ ] <line>` for each. Each line becomes a Task 2.N checklist item.

---

## Phase 1 — Introduce `SchedOp`, `EngineCtx` extensions, and `close_signal_pending`

Pure additions; nothing is rewired yet. All existing tests must still pass.

### Task 1.1: Extend `EngineCtx` with `connect`, `disconnect`, `remove_signal`

**Files:**
- Modify: `crates/emcore/src/emEngine.rs`.

- [ ] **Step 1: Read `EngineCtx`'s existing methods.**

  Run: `sed -n '84,130p' crates/emcore/src/emEngine.rs`
  Expected: `fire`, `IsSignaled`, `IsTimeSliceAtEnd`, `wake_up`.

- [ ] **Step 2: Add three forwarding methods.**

  After `wake_up` (currently near `:120`), append:

  ```rust
  /// Connect a signal to an engine so the engine wakes whenever the
  /// signal is fired. Forwards to `EngineScheduler::connect`.
  pub fn connect(&mut self, signal: super::emSignal::SignalId, engine: EngineId) {
      self.scheduler.connect_inner(signal, engine);
  }

  /// Disconnect a signal→engine wake link. Forwards to
  /// `EngineScheduler::disconnect`.
  pub fn disconnect(&mut self, signal: super::emSignal::SignalId, engine: EngineId) {
      self.scheduler.disconnect_inner(signal, engine);
  }

  /// Remove a signal from the scheduler. Forwards to
  /// `EngineScheduler::remove_signal`.
  pub fn remove_signal(&mut self, signal: super::emSignal::SignalId) {
      self.scheduler.remove_signal_inner(signal);
  }
  ```

  **Implementer note:** the `_inner` method names assume they exist on `EngineCtxInner`. Read `EngineCtxInner` (in `emEngine.rs`) and the `EngineScheduler::{connect, disconnect, remove_signal}` implementations (in `emScheduler.rs`). If those operations are currently implemented on `EngineScheduler` directly (not on `EngineCtxInner`), extract the actual mutation bodies into `EngineCtxInner` methods (`pub(crate) fn connect_inner`, etc.) and make `EngineScheduler`'s public methods thin wrappers that call them. Do NOT duplicate logic.

- [ ] **Step 3: Build.**

  Run: `cargo check -p emcore 2>&1 | tail -10`
  Expected: clean.

- [ ] **Step 4: Run emcore tests.**

  Run: `cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: all pass; no new tests yet.

- [ ] **Step 5: Commit.**

  ```bash
  git add crates/emcore/src/emEngine.rs crates/emcore/src/emScheduler.rs
  git commit -m "sp4(1/n): EngineCtx gains connect/disconnect/remove_signal forwarders"
  ```

### Task 1.2: Add `SchedOp` enum with `apply_to` + `apply_via_ctx`

**Files:**
- Modify: `crates/emcore/src/emView.rs`.

- [ ] **Step 1: Locate a reasonable module-level location for the enum.**

  Run: `grep -n "^pub(crate) struct\|^pub(crate) enum\|^pub struct UpdateEngineClass" crates/emcore/src/emView.rs | head -5`
  Find the top of the file or just above `UpdateEngineClass` (~:179). The enum lives alongside the engine types that will use it.

- [ ] **Step 2: Insert the enum + impls.**

  Add (placement: just before `pub struct UpdateEngineClass` at ~:184):

  ```rust
  /// Deferred scheduler operation, issued from inside `emView::Update`'s
  /// reachable call tree when the scheduler is already `borrow_mut`'d by
  /// the enclosing `DoTimeSlice`. Drained by `UpdateEngineClass::Cycle`
  /// immediately after `Update` returns.
  ///
  /// IDIOM: C++ calls `Scheduler.X(...)` inline during Update because its
  /// scheduler has no aliasing restrictions. Rust's `RefCell<EngineScheduler>`
  /// forbids inner borrows while `DoTimeSlice` holds the outer borrow;
  /// deferral restores inline semantics within the same time slice
  /// without violating borrow rules.
  #[derive(Debug, Clone, Copy)]
  pub(crate) enum SchedOp {
      Fire(super::emSignal::SignalId),
      WakeUp(super::emEngine::EngineId),
      Connect(super::emSignal::SignalId, super::emEngine::EngineId),
      Disconnect(super::emSignal::SignalId, super::emEngine::EngineId),
      RemoveSignal(super::emSignal::SignalId),
  }

  impl SchedOp {
      /// Apply directly to an `&mut EngineScheduler`. Used on the
      /// non-engine path where `try_borrow_mut` succeeded.
      pub(crate) fn apply_to(self, sched: &mut super::emScheduler::EngineScheduler) {
          match self {
              SchedOp::Fire(s) => sched.fire(s),
              SchedOp::WakeUp(e) => sched.wake_up(e),
              SchedOp::Connect(s, e) => sched.connect(s, e),
              SchedOp::Disconnect(s, e) => sched.disconnect(s, e),
              SchedOp::RemoveSignal(s) => sched.remove_signal(s),
          }
      }

      /// Apply via an `EngineCtx` (drain-time path, inside `Cycle`).
      pub(crate) fn apply_via_ctx(self, ctx: &mut super::emEngine::EngineCtx<'_>) {
          match self {
              SchedOp::Fire(s) => ctx.fire(s),
              SchedOp::WakeUp(e) => ctx.wake_up(e),
              SchedOp::Connect(s, e) => ctx.connect(s, e),
              SchedOp::Disconnect(s, e) => ctx.disconnect(s, e),
              SchedOp::RemoveSignal(s) => ctx.remove_signal(s),
          }
      }
  }
  ```

- [ ] **Step 3: Build.**

  Run: `cargo check -p emcore 2>&1 | tail -10`
  Expected: clean.

- [ ] **Step 4: Commit.**

  ```bash
  git add crates/emcore/src/emView.rs
  git commit -m "sp4(2/n): SchedOp enum for deferred scheduler writes"
  ```

### Task 1.3: Add `pending_sched_ops`, `close_signal_pending` fields + `queue_or_apply_sched_op` helper

**Files:**
- Modify: `crates/emcore/src/emView.rs`.

- [ ] **Step 1: Add fields to `emView` struct.**

  Near the existing `pub(crate) pending_framework_actions: ...` field:

  ```rust
  /// Set by `UpdateEngineClass::Cycle` from `ctx.IsSignaled(close_signal)`
  /// before calling `Update`; read and cleared at the top of `Update`.
  /// See C++ `emView::Update` popup-close probe at `emView.cpp:1299`.
  ///
  /// DIVERGED: C++ emView is an emEngine (via emContext); Rust emView is
  /// not yet (tracked as SP7). UpdateEngine's clock substitutes for
  /// emView's own clock.
  pub(crate) close_signal_pending: bool,

  /// Queue of scheduler ops issued from inside Update's call tree when
  /// the scheduler is already borrow_mut'd. Drained by
  /// UpdateEngineClass::Cycle after Update returns. Invariant: only
  /// nonempty transiently, inside a `Cycle` invocation.
  pub(crate) pending_sched_ops: Vec<SchedOp>,
  ```

- [ ] **Step 2: Initialize in both constructors.**

  In `new(...)` and `new_for_test(...)`, add:
  ```rust
  close_signal_pending: false,
  pending_sched_ops: Vec::new(),
  ```

- [ ] **Step 3: Add the helper method on `impl emView`.**

  Locate the `impl emView` block (the main one starting near `:484`). Add:

  ```rust
  /// Apply a scheduler op: execute immediately if the scheduler is not
  /// currently borrowed (the common, non-engine-path case), otherwise
  /// enqueue for drain by `UpdateEngineClass::Cycle`.
  ///
  /// Used by every scheduler-write call site in `emView.rs` that is
  /// reachable from `Update`. Non-Update call sites hit the inline-apply
  /// arm and incur zero queue overhead.
  pub(crate) fn queue_or_apply_sched_op(&mut self, op: SchedOp) {
      let Some(sched_rc) = self.scheduler.as_ref() else {
          return; // Unit-test bare view: no scheduler, all ops no-op.
      };
      match sched_rc.try_borrow_mut() {
          Ok(mut sched) => op.apply_to(&mut *sched),
          Err(_) => self.pending_sched_ops.push(op),
      }
  }
  ```

- [ ] **Step 4: Build + test.**

  Run: `cargo check -p emcore && cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: clean; all existing tests pass.

- [ ] **Step 5: Commit.**

  ```bash
  git add crates/emcore/src/emView.rs
  git commit -m "sp4(3/n): emView gains pending_sched_ops + queue_or_apply helper"
  ```

---

## Phase 2 — Migrate scheduler-borrow call sites

Each of the ~10 `sched.borrow_mut().X(...)` sites from Task 0.1 Step 5 gets rewritten to `self.queue_or_apply_sched_op(SchedOp::X(...))`. Do NOT migrate `attach_to_scheduler:3050` or `#[cfg(test)]` sites. After each migration, run emcore tests — regressions here indicate the rewrite was non-equivalent.

### Task 2.1: Migrate one site at a time, committing after each

**Files:** `crates/emcore/src/emView.rs`.

For each line number in the Task 0.1 Step 5 checklist, perform Steps 1–4:

- [ ] **Step 1: Read 5 lines of context.**

  Run: `sed -n '<LINE-2>,<LINE+2>p' crates/emcore/src/emView.rs`

- [ ] **Step 2: Rewrite the block.**

  The four patterns to recognize:

  **Pattern A: single op inside `if let Some(sched) = &self.scheduler`.**

  Before:
  ```rust
  if let Some(sched) = &self.scheduler {
      sched.borrow_mut().fire(sig);
  }
  ```
  After:
  ```rust
  self.queue_or_apply_sched_op(SchedOp::Fire(sig));
  ```
  (The `None` case was a silent drop; `queue_or_apply_sched_op` does the same.)

  **Pattern B: single op with additional conditions.**

  Before:
  ```rust
  if let (Some(sched), Some(eng_id)) = (self.scheduler.as_ref(), self.update_engine_id) {
      sched.borrow_mut().connect(close_sig, eng_id);
  }
  ```
  After:
  ```rust
  if let Some(eng_id) = self.update_engine_id {
      self.queue_or_apply_sched_op(SchedOp::Connect(close_sig, eng_id));
  }
  ```

  **Pattern C: multiple ops sharing one `let mut s = sched.borrow_mut();`.**

  Before:
  ```rust
  let mut s = sched.borrow_mut();
  s.disconnect(close_sig, eng_id);
  s.remove_signal(close_sig);
  ```
  After:
  ```rust
  self.queue_or_apply_sched_op(SchedOp::Disconnect(close_sig, eng_id));
  self.queue_or_apply_sched_op(SchedOp::RemoveSignal(close_sig));
  ```

  **Pattern D: read-then-write (e.g., `create_signal` which returns an ID).**

  These must NOT be migrated — `create_signal` returns a value, which the queue cannot support. Verify by inspection: if the site needs a return value, leave it alone and mark in the commit message that the site was skipped because it is not reachable from `Update` (popup creation paths only).

  If a given site is at `:1723` or `:1757` (popup creation), leave it alone — those are `create_signal` and the returned ID is stored. Document in the commit message.

- [ ] **Step 3: Run emcore tests.**

  Run: `cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: same pass count as before this site's migration.

- [ ] **Step 4: Commit.**

  ```bash
  git add crates/emcore/src/emView.rs
  git commit -m "sp4(4.N/n): migrate sched.borrow_mut() at emView.rs:<LINE> → queue_or_apply"
  ```

  Check off the line in your working notes.

Repeat Steps 1–4 for every line in the checklist except `:2343` (covered in Phase 3 via the cached field) and popup-creation `create_signal` sites.

### Task 2.2: Migration completeness gate

- [ ] **Step 1: Confirm no scheduler borrows remain in emView.rs outside allowed zones.**

  Run:
  ```bash
  grep -nE 'self\.scheduler.*borrow|sched\.borrow' crates/emcore/src/emView.rs \
      | grep -v '#\[cfg(test)\]' \
      | grep -v 'fn attach_to_scheduler' \
      | grep -v ':2343' \
      | grep -v 'create_signal'
  ```
  Expected: empty output. If non-empty, each line is a missed migration; go back to Task 2.1 for it.

- [ ] **Step 2: Run the full workspace build + test.**

  Run: `cargo check && cargo-nextest run 2>&1 | tail -5`
  Expected: clean; baseline test count.

---

## Phase 3 — Rewire Update (cached-field popup probe + engine-only routing)

### Task 3.1: Add bridge path in `Update` for `close_signal_pending`

**Files:** `crates/emcore/src/emView.rs` (popup-close block at `:2318-2350`).

- [ ] **Step 1: Read the current block.**

  Run: `sed -n '2318,2352p' crates/emcore/src/emView.rs`

- [ ] **Step 2: Replace the `let popup_closed = { ... }` block with a bridge that prefers the cached field and keeps the legacy borrow as a fallback.**

  ```rust
  let popup_closed = {
      let cached = std::mem::take(&mut self.close_signal_pending);
      if cached {
          true
      } else if let (Some(popup), Some(sched), Some(eng_id)) = (
          self.PopupWindow.as_ref(),
          self.scheduler.as_ref(),
          self.update_engine_id,
      ) {
          let close_sig = popup.borrow().close_signal;
          sched.borrow().is_signaled_for_engine(close_sig, eng_id)
      } else {
          false
      }
  };
  ```

- [ ] **Step 3: Test.**

  Run: `cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: all pass.

- [ ] **Step 4: Commit.**

  ```bash
  git commit -am "sp4(5/n): Update popup-close probe: cached field + legacy bridge"
  ```

### Task 3.2: Write popup-close probe in `UpdateEngineClass::Cycle` + queue drain

**Files:** `crates/emcore/src/emView.rs` (`UpdateEngineClass::Cycle` at `:197-206`).

- [ ] **Step 1: Rewrite `Cycle`.**

  ```rust
  impl super::emEngine::emEngine for UpdateEngineClass {
      fn Cycle(&mut self, ctx: &mut super::emEngine::EngineCtx<'_>) -> bool {
          // C++ UpdateEngineClass::Cycle (emView.cpp:2521-2524).
          let Some(win_rc) = ctx.windows.get(&self.window_id) else {
              return false;
          };
          let win_rc = Rc::clone(win_rc);
          let mut win = win_rc.borrow_mut();
          let view = win.view_mut();

          // SP4 Part A: pre-compute the popup-close probe here (C++ emView.cpp:1299;
          // in C++ this is inside Update against emView's own engine clock, but
          // Rust emView is not an emEngine so we use UpdateEngine's clock via ctx).
          if let Some(popup) = view.PopupWindow.as_ref() {
              let close_sig = popup.borrow().close_signal;
              view.close_signal_pending = ctx.IsSignaled(close_sig);
          }

          view.Update(ctx.tree);

          // SP4 Part B: drain deferred scheduler ops queued by Update's call tree.
          let ops: Vec<SchedOp> = view.pending_sched_ops.drain(..).collect();
          for op in ops {
              op.apply_via_ctx(ctx);
          }
          false
      }
  }
  ```

  **Why the `.collect()`:** `apply_via_ctx(ctx)` borrows `ctx` mutably; during that call, `view` (which owns `pending_sched_ops`) is also borrowed mutably. Collect into a local `Vec` to release the `view.pending_sched_ops.drain(..)` borrow before iterating.

- [ ] **Step 2: Test.**

  Run: `cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: all pass. Any failure here is either a Phase 2 migration bug or a bug in Part A.

- [ ] **Step 3: Commit.**

  ```bash
  git commit -am "sp4(6/n): UpdateEngineClass::Cycle pre-probes close_signal, drains pending_sched_ops"
  ```

### Task 3.3: Remove the legacy bridge from `Update`

**Files:** `crates/emcore/src/emView.rs` (the block from Task 3.1).

- [ ] **Step 1: Replace the bridge with the final form.**

  ```rust
  // C++ emView.cpp:1299 popup-close probe. The IsSignaled call happens
  // one frame earlier in Rust, in UpdateEngineClass::Cycle — see SP4 spec
  // docs/superpowers/specs/2026-04-18-emview-sp4-update-engine-routing-design.md §2.3.
  let popup_closed = std::mem::take(&mut self.close_signal_pending);
  ```

  Delete the entire `BUG (tracked as ...)` comment block at `:2324-2335`.

- [ ] **Step 2: Verify no scheduler borrow remains at `:2343`.**

  Run: `sed -n '2318,2345p' crates/emcore/src/emView.rs`
  Expected: no `sched.borrow().is_signaled_for_engine` and no `self.scheduler.as_ref()` in this block.

- [ ] **Step 3: Full workspace test.**

  Run: `cargo-nextest run 2>&1 | tail -5`
  Expected: baseline.

- [ ] **Step 4: Commit.**

  ```bash
  git commit -am "sp4(7/n): remove legacy scheduler-borrow fallback from Update popup probe"
  ```

### Task 3.4: Migration completeness gate (final)

- [ ] **Step 1:**

  Run: `grep -nE 'self\.scheduler.*borrow|sched\.borrow' crates/emcore/src/emView.rs | grep -v '#\[cfg(test)\]' | grep -v 'fn attach_to_scheduler'`
  Expected: output is either empty or only `create_signal` popup-creation sites (at `:1723`-ish). Anything else is a missed migration.

- [ ] **Step 2: Confirm the BUG marker is gone.**

  Run: `grep -n "BUG (tracked as" crates/emcore/src/emView.rs`
  Expected: empty.

---

## Phase 4 — Engine-only routing: delete `:594`, add ctor wake, relocate `SetActivePanelBestPossible`

### Task 4.1: Append `SetActivePanelBestPossible` to `Scroll`, `Zoom`, `ZoomOut`

**Files:** `crates/emcore/src/emView.rs` (`Scroll` near `:1123`, `Zoom` near `:1086`, `ZoomOut` near `:1251`).

- [ ] **Step 1: Locate each function end and append.**

  Each function gets `self.SetActivePanelBestPossible(tree);` as its last statement, with a citing comment:
  - `Scroll`: `// C++ emView.cpp:780.`
  - `Zoom`: `// C++ emView.cpp:800.`
  - `ZoomOut`: `// C++ emView.cpp:901.`

- [ ] **Step 2: Test.**

  Run: `cargo check -p emcore && cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: clean; pass.

- [ ] **Step 3: Commit.**

  ```bash
  git commit -am "sp4(8/n): relocate SetActivePanelBestPossible to Scroll/Zoom/ZoomOut (C++ parity)"
  ```

### Task 4.2: Wake the update engine at `attach_to_scheduler`

**Files:** `crates/emcore/src/emView.rs:3044-3070`.

- [ ] **Step 1: Append to `attach_to_scheduler`.**

  Last line of the function body (after `self.visiting_va_engine_id = Some(visiting_va_engine_id);`):

  ```rust
  // C++ emView::emView at emView.cpp:84: UpdateEngine->WakeUp().
  self.WakeUpUpdateEngine();
  ```

- [ ] **Step 2: Fix `test_phase7_update_engine_wakeup_via_scheduler`.**

  Near `:5956`: delete `assert!(!sched.borrow().has_awake_engines());` (the engine is now awake after attach). Add comment:
  ```rust
  // SP4: attach_to_scheduler wakes the update engine (C++ emView.cpp:84).
  // The explicit WakeUpUpdateEngine() below verifies the re-wake API.
  ```

- [ ] **Step 3: Test.**

  Run: `cargo-nextest run -p emcore 2>&1 | tail -5`
  Expected: all pass.

- [ ] **Step 4: Commit.**

  ```bash
  git commit -am "sp4(9/n): wake update engine from attach_to_scheduler"
  ```

### Task 4.3: Delete `emGUIFramework.rs:594` direct call

**Files:** `crates/emcore/src/emGUIFramework.rs:593-595`.

- [ ] **Step 1: Replace the two-line block.**

  Delete:
  ```rust
              // Update view (recompute viewing coords, auto-select active)
              win.view_mut().update(tree);
  ```
  With:
  ```rust
              // SP4: Update runs only via UpdateEngineClass::Cycle now
              // (C++ single-caller model, emView.cpp:2523).
  ```

- [ ] **Step 2: Full workspace test + golden + smoke.**

  Run:
  ```bash
  cargo check && cargo-nextest run 2>&1 | tail -5
  cargo test --test golden -- --test-threads=1 2>&1 | tail -5
  timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
  ```
  Expected: all baseline or better. If tests regress here, they exercised a behavior that the `update()` wrapper provided but the engine path doesn't — investigate root cause before patching over.

- [ ] **Step 3: Commit.**

  ```bash
  git commit -am "sp4(10/n): remove direct emView::update call from about_to_wait"
  ```

### Task 4.4: Delete `emView::update()` wrapper

**Files:** `crates/emcore/src/emView.rs:3845-3859` and any stragglers.

- [ ] **Step 1: Find cross-crate callers.**

  Run: `grep -rn '\.update(&mut tree\|\.update(tree)' crates/ | grep -v 'Update('`
  Expected: empty (Task 4.3 deleted the only production site).

- [ ] **Step 2: Delete the wrapper.**

  Remove the `pub fn update(&mut self, tree: &mut PanelTree) { ... }` block.

- [ ] **Step 3: Full workspace test + golden + smoke.**

  Run: same as Task 4.3 Step 2.
  Expected: baseline.

- [ ] **Step 4: Commit.**

  ```bash
  git commit -am "sp4(11/n): delete emView::update() wrapper — single-caller model complete"
  ```

---

## Phase 5 — Test harness: bare-window ctor + Phase-8 rewrite + same-slice test

### Task 5.1: Add `emWindow::new_for_test`

**Files:** `crates/emcore/src/emWindow.rs`, `crates/emcore/Cargo.toml`.

- [ ] **Step 1: Verify `test-support` feature exists.**

  Run: `grep -n "test-support" crates/emcore/Cargo.toml`
  Expected: `test-support = []` entry. If absent, add it.

- [ ] **Step 2: Write failing test (in `emWindow.rs` tests module).**

  ```rust
  #[test]
  fn new_for_test_constructs_without_event_loop() {
      let mut tree = crate::emPanelTree::PanelTree::new();
      let root = tree.create_root("root");
      tree.Layout(root, 0.0, 0.0, 1.0, 1.0, 1.0);
      let win_id = winit::window::WindowId::dummy();
      let sched = std::rc::Rc::new(std::cell::RefCell::new(
          crate::emScheduler::EngineScheduler::new(),
      ));
      let win = emWindow::new_for_test(win_id, &sched, root, 640.0, 480.0);
      assert_eq!(win.borrow().id(), win_id);
      assert!(win.borrow().view().update_engine_id.is_some());
  }
  ```

- [ ] **Step 3: Run — fails.**

  Run: `cargo-nextest run -p emcore new_for_test_constructs_without_event_loop 2>&1 | tail -5`
  Expected: FAIL (method not defined).

- [ ] **Step 4: Implement `new_for_test`.**

  Read the `emWindow` struct and existing `new_popup_pending` constructor to understand every field. Mirror the `new_popup_pending` shape but skip anything GPU/winit-surface-related by using the `OsSurface::Pending` variant with a stub `PendingSurface` default.

  ```rust
  #[cfg(any(test, feature = "test-support"))]
  pub fn new_for_test(
      window_id: winit::window::WindowId,
      scheduler: &std::rc::Rc<std::cell::RefCell<crate::emScheduler::EngineScheduler>>,
      root: crate::emPanelTree::PanelId,
      width: f64,
      height: f64,
  ) -> std::rc::Rc<std::cell::RefCell<Self>> {
      let mut view = crate::emView::emView::new_for_test(root, width, height);
      view.attach_to_scheduler(scheduler.clone(), window_id);
      // ... fill remaining emWindow fields with OsSurface::Pending default
      // and test-neutral values for the rest. See existing new_popup_pending
      // for the exact struct shape.
      let win = Self {
          window_id,
          // ... (implementer: read struct, fill each field)
          view,
      };
      std::rc::Rc::new(std::cell::RefCell::new(win))
  }
  ```

  **Implementer note:** If `OsSurface::Pending` construction requires a `PendingSurface`, build the minimum one (per W3 closeout §3.2). If you find a field you cannot confidently default, STOP and read the existing `emWindow::create` and `new_popup_pending` for precedent. Do not guess.

- [ ] **Step 5: Test passes; commit.**

  Run: `cargo-nextest run -p emcore new_for_test_constructs_without_event_loop 2>&1 | tail -5`
  Expected: PASS.

  Commit:
  ```bash
  git add crates/emcore/src/emWindow.rs crates/emcore/Cargo.toml
  git commit -m "sp4(12/n): emWindow::new_for_test for single-engine integration tests"
  ```

### Task 5.2: Add same-slice-propagation test

**Files:** `crates/emcore/src/emView.rs` tests module.

This guards against the risk that drain-at-end-of-Cycle reorders signal wakes past the slice boundary.

- [ ] **Step 1: Write the test.**

  ```rust
  /// SP4: a signal fired from inside Update via queue_or_apply_sched_op
  /// must still wake a receiver engine that is connected to that signal
  /// and is at a lower priority than UpdateEngineClass — all in the same
  /// DoTimeSlice. Guards against "drain-too-late" bugs where queued wakes
  /// would be delayed past the current slice.
  #[test]
  fn sp4_signal_fired_from_update_reaches_receiver_same_slice() {
      use std::collections::HashMap;
      use crate::emEngine::{emEngine as EngineTrait, EngineCtx, Priority};

      let (mut tree, root, _, _) = setup_tree();
      let win_id = winit::window::WindowId::dummy();
      let sched = Rc::new(RefCell::new(EngineScheduler::new()));
      let win = crate::emWindow::emWindow::new_for_test(win_id, &sched, root, 640.0, 480.0);

      // Receiver engine at lower priority, woken by a signal fired from inside Update.
      struct Receiver { cycled: Rc<RefCell<bool>> }
      impl EngineTrait for Receiver {
          fn Cycle(&mut self, _ctx: &mut EngineCtx<'_>) -> bool {
              *self.cycled.borrow_mut() = true;
              false
          }
      }
      let cycled = Rc::new(RefCell::new(false));
      let recv_id = sched.borrow_mut().register_engine(
          Priority::Low,
          Box::new(Receiver { cycled: cycled.clone() }),
      );
      let trigger_sig = sched.borrow_mut().create_signal();
      sched.borrow_mut().connect(trigger_sig, recv_id);

      // Queue a Fire op onto the view from outside Update, using
      // queue_or_apply_sched_op with the scheduler already borrow_mut'd
      // (simulated by calling from inside a borrow we construct here).
      // Rather than simulate, drive a real cycle: cause control_panel_signal
      // (or a similar signal reliably fired from within Update's call tree)
      // to wake our receiver — but that requires threading. Simpler route:
      // fire an arbitrary signal through queue_or_apply_sched_op while the
      // outer borrow is held by DoTimeSlice.
      //
      // Strategy: borrow_mut the scheduler ourselves (simulating DoTimeSlice),
      // then have the view queue a Fire op, then release the borrow and
      // DoTimeSlice; the drain should happen via UpdateEngineClass::Cycle.
      //
      // Simpler deterministic strategy: use the already-existing
      // control_panel_signal. Call v.set_active_panel inside DoTimeSlice and
      // verify the Receiver (connected to control_panel_signal) cycles.
      //
      // Implementer: design the specific trigger carefully so the assertion
      // is load-bearing. Use the following template:
      {
          let mut w = win.borrow_mut();
          w.view_mut().Update(&mut tree); // priming
      }
      // Install Receiver as a listener of control_panel_signal.
      let cp_sig = win.borrow().view().control_panel_signal.expect("cp signal present");
      sched.borrow_mut().connect(cp_sig, recv_id);

      // Drive DoTimeSlice: UpdateEngineClass::Cycle runs Update. We need
      // Update's call tree to fire control_panel_signal. set_active_panel
      // does this, but it must be triggered during Update. Use
      // SVPChoiceInvalid + SetActivePanelBestPossible path: set
      // SVPChoiceInvalid=true before the slice, so Update's drain reaches
      // SetActivePanelBestPossible → set_active_panel → Fire(cp_sig).
      win.borrow_mut().view_mut().SVPChoiceInvalid = true;

      let mut windows: HashMap<_, _> = HashMap::new();
      windows.insert(win_id, Rc::clone(&win));
      sched.borrow_mut().DoTimeSlice(&mut tree, &mut windows);

      assert!(
          *cycled.borrow(),
          "Receiver at Low priority must cycle in the same slice as the Update-issued Fire"
      );

      // Cleanup
      let cycled_drop = cycled;
      drop(cycled_drop);
      let mut w = win.borrow_mut();
      let v = w.view_mut();
      if let Some(id) = v.update_engine_id.take() { sched.borrow_mut().remove_engine(id); }
      if let Some(id) = v.visiting_va_engine_id.take() { sched.borrow_mut().remove_engine(id); }
      if let Some(s) = v.EOISignal.take() { sched.borrow_mut().remove_signal(s); }
      sched.borrow_mut().disconnect(cp_sig, recv_id);
      sched.borrow_mut().remove_engine(recv_id);
      sched.borrow_mut().remove_signal(trigger_sig);
  }
  ```

  **Implementer note:** The test design hinges on an Update-call-tree path that fires a signal. `SVPChoiceInvalid → SetActivePanelBestPossible → set_active_panel → fire(control_panel_signal)` is the proposed path. Verify this path is real by reading `set_active_panel`'s body (near `:1418`) and `SetActivePanelBestPossible` (near `:1489`). If the path is not reliably triggered by `SVPChoiceInvalid=true`, choose an alternative signal fired from Update's tree — the test only needs *any* such signal; its purpose is to exercise drain-at-end-of-Cycle.

- [ ] **Step 2: Run — must pass.**

  Run: `cargo-nextest run -p emcore sp4_signal_fired_from_update_reaches_receiver_same_slice 2>&1 | tail -5`
  Expected: PASS. If it fails, the drain semantics differ from what the spec claims and need investigation before proceeding.

- [ ] **Step 3: Commit.**

  ```bash
  git commit -am "sp4(13/n): same-slice wake propagation test for queued Fire ops"
  ```

### Task 5.3: Rewrite Phase-8 test as single-engine

**Files:** `crates/emcore/src/emView.rs` (`test_phase8_popup_close_signal_zooms_out` near `:6098-6210`).

- [ ] **Step 1: Replace the entire test body and its multi-paragraph doc block.**

  ```rust
  /// SP4 Phase-8: popup's close_signal, when fired and processed through the
  /// scheduler, wakes UpdateEngineClass, which invokes emView::Update, which
  /// reads close_signal_pending and calls ZoomOut. ZoomOut's popup teardown
  /// enqueues a Disconnect + RemoveSignal + Fire(geometry_signal), drained
  /// at end of Cycle. The entire sequence runs in one DoTimeSlice.
  /// Supersedes the previous two-engine harness (NoopEngine swap), which
  /// was a compromise against the now-fixed scheduler re-entrant borrow.
  #[test]
  fn test_phase8_popup_close_signal_zooms_out() {
      use std::collections::HashMap;
      let (mut tree, root, child_a, _) = setup_tree();
      let win_id = winit::window::WindowId::dummy();
      let sched = Rc::new(RefCell::new(EngineScheduler::new()));
      let win = crate::emWindow::emWindow::new_for_test(win_id, &sched, root, 640.0, 480.0);

      // Prime Update (clear zoomed_out_before_sg), push a popup under POPUP_ZOOM.
      {
          let mut w = win.borrow_mut();
          let v = w.view_mut();
          v.Update(&mut tree);
          v.SetViewFlags(ViewFlags::POPUP_ZOOM, &mut tree);
          v.RawVisit(&mut tree, child_a, 0.0, 0.0, 0.1, true);
          assert!(v.PopupWindow.is_some());
      }

      // Fire close_signal.
      let close_sig = win.borrow().view().PopupWindow.as_ref().unwrap().borrow().close_signal;
      sched.borrow_mut().fire(close_sig);

      // One DoTimeSlice: signal processing advances sig.clock; Cycle
      // observes ctx.IsSignaled(close_sig) = true, stores close_signal_pending,
      // calls Update → ZoomOut → RawVisitAbs (popup teardown); drains queued
      // Disconnect/RemoveSignal/Fire ops; exits.
      let mut windows: HashMap<_, _> = HashMap::new();
      windows.insert(win_id, Rc::clone(&win));
      sched.borrow_mut().DoTimeSlice(&mut tree, &mut windows);

      assert!(win.borrow().view().PopupWindow.is_none(),
          "close_signal → ZoomOut must tear down PopupWindow in one time slice");
      assert!(!win.borrow().view().popped_up, "popped_up must be false after ZoomOut");

      // Cleanup for scheduler Drop debug_asserts.
      let mut w = win.borrow_mut();
      let v = w.view_mut();
      if let Some(id) = v.update_engine_id.take() { sched.borrow_mut().remove_engine(id); }
      if let Some(id) = v.visiting_va_engine_id.take() { sched.borrow_mut().remove_engine(id); }
      if let Some(s) = v.EOISignal.take() { sched.borrow_mut().remove_signal(s); }
  }
  ```

- [ ] **Step 2: Run full test suite.**

  Run: `cargo-nextest run 2>&1 | tail -5`
  Expected: baseline + 2 new tests = 2431/2431.

- [ ] **Step 3: Run golden + smoke.**

  Run:
  ```bash
  cargo test --test golden -- --test-threads=1 2>&1 | tail -5
  timeout 20 cargo run --release --bin eaglemode; echo "exit=$?"
  ```
  Expected: 237/243 baseline; smoke exit=124/143.

- [ ] **Step 4: Confirm single-engine structure.**

  Inspect the test body visually: exactly one `DoTimeSlice` call, no `NoopEngine`.

- [ ] **Step 5: Commit.**

  ```bash
  git commit -am "sp4(14/n): rewrite Phase-8 popup-close test as single-engine DoTimeSlice"
  ```

---

## Phase 6 — Closeout

### Task 6.1: Full verification

- [ ] **Step 1: Clippy.**

  Run: `cargo clippy --all-targets --features test-support -- -D warnings 2>&1 | tail -10`
  Expected: clean.

- [ ] **Step 2: Success-criteria gates from the spec §5.**

  Run each and confirm:
  - `grep -n "BUG (tracked as" crates/emcore/src/emView.rs` → empty.
  - `grep -n "win\.view_mut().update(tree)" crates/emcore/src/emGUIFramework.rs` → empty.
  - `grep -n "^    pub fn update\b" crates/emcore/src/emView.rs` → empty.
  - `grep -nE 'self\.scheduler.*borrow|sched\.borrow' crates/emcore/src/emView.rs | grep -v '#\[cfg(test)\]' | grep -v 'fn attach_to_scheduler' | grep -v 'create_signal'` → empty.
  - Task 5.2's test passes.
  - Phase-8 test runs one `DoTimeSlice`.

- [ ] **Step 3: Re-run nextest + golden + smoke once more.**

  Same as Phase 0.

### Task 6.2: Update closeout doc

**Files:** `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md`.

- [ ] **Step 1:** Mark SP4 complete in §8.0 table with commit SHA.
- [ ] **Step 2:** Mark §8.1 items 11 and 14 as CLOSED.
- [ ] **Step 3:** Mark §5.1 item 5 (Phase-8 two-engine test) as closed by SP4.
- [ ] **Step 4:** Update §1 "Status at a glance" — remove item 14 from residuals.
- [ ] **Step 5:** Commit.

  ```bash
  git commit -am "docs(closeout): mark SP4 complete"
  ```

### Task 6.3: Merge / PR

Out of scope — defer to project branch-finishing flow.

---

## Self-review

1. **Spec coverage:**
   - §2.1 engine-only routing → Phase 4. ✓
   - §2.2 ctor-time wake → Task 4.2. ✓
   - §2.3 Part A (cached close_signal_pending) → Tasks 1.3, 3.1, 3.2, 3.3. ✓
   - §2.3 Part B (queue + SchedOp + helper + drain) → Tasks 1.1, 1.2, 1.3, 3.2, Phase 2. ✓
   - §2.4 test-site stability → no test signature changes; achieved. ✓
   - §2.5 Phase-8 single-engine → Tasks 5.1, 5.3. ✓
   - §2.6 non-goals → no task touches `VisitingVAEngineClass` or notice dispatch. ✓
   - §5.10 same-slice-propagation test → Task 5.2. ✓

2. **Placeholder scan:**
   - Task 5.1 Step 4 leaves specific struct-field values to implementer, with explicit "do not guess, read the struct" instruction. Intentional: the `emWindow` struct shape is large and stable; listing every field here would immediately go stale. Acceptable.
   - Task 5.2 Step 1 notes the test hinges on a specific Update-call-tree signal path and tells the implementer to verify it. Intentional.
   - Phase 2 Task 2.1 lists patterns but not exact line-by-line rewrites because the file shifts between phases. Task 0.1 Step 5 and Task 2.2 Step 1 provide the exact grep to produce the live checklist. Acceptable.

3. **Type consistency:**
   - `SchedOp` (Task 1.2) used identically in `apply_to`, `apply_via_ctx`, `queue_or_apply_sched_op`, `UpdateEngineClass::Cycle` drain. ✓
   - `pending_sched_ops: Vec<SchedOp>` drained via `drain(..).collect()` into a local `Vec<SchedOp>` in Task 3.2 — releases view borrow before iterating. ✓
   - `close_signal_pending: bool` written by Cycle, consumed via `std::mem::take` in Update. ✓
   - `EngineCtx::{connect, disconnect, remove_signal}` defined in Task 1.1, used in `SchedOp::apply_via_ctx` (Task 1.2). ✓
   - `emWindow::new_for_test` signature consistent between Task 5.1 test + impl, Task 5.2 test, Task 5.3 Phase-8 rewrite. ✓

---

**End of plan.**
