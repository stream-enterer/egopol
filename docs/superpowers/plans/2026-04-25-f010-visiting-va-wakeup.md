# F010 — VisitingVA Activation WakeUp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `emView::Visit*` actually drive the visiting view animator by waking the registered `VisitingVAEngineClass` after `va.Activate()`, so programmatic visits (StartupEngine `VisitFullsized(":")`, control-channel `visit`/`visit-fullsized`/`seek-to`, focus-follow zoom) advance the inner view's framing rectangle. Closes the F010 root cause (see `docs/debug/investigations/F010-root-cause.md`).

**Architecture:** C++ `emViewAnimator::Activate` (emViewAnimator.cpp:81) calls `WakeUp()` because in C++ the animator IS-A engine. The Rust port splits the animator (`emVisitingViewAnimator` in `crates/emcore/src/emViewAnimator.rs`) from the wrapper engine (`VisitingVAEngineClass` in `crates/emcore/src/emView.rs:267-353`) for ownership reasons, so `Activate()` cannot reach the scheduler from within the animator. The fix plumbs `&mut SchedCtx<'_>` through the **six** Visit-family methods (`Visit`, `VisitByIdentity`, `VisitFullsized`, `VisitFullsizedByIdentity`, `VisitPanel`, `VisitByIdentityBare`) and **eleven** navigation helpers on `emView`, and adds a new helper `wake_visiting_va_engine(&mut self, &mut SchedCtx<'_>)` (mirror of the existing `WakeUpUpdateEngine`, which uses `&mut self`) that the Visit-family methods call after `va.Activate()`.

**Tech Stack:** Rust 2021, single-crate `emcore`, `cargo-nextest` for tests, `cargo clippy -D warnings` lint gate. Pre-commit hook runs `cargo fmt`, `clippy`, `cargo-nextest ntr`. C++ reference at `~/Projects/eaglemode-0.96.4/`.

**Rejected alternatives** (do NOT pursue):
- *Always-poll the wrapper engine* (return `true` from `Cycle` even when `!is_active`) — breaks `EngineScheduler::is_idle()` (`emScheduler.rs:734-737`), which `emCtrlSocket::wait_idle` and integration tests depend on.
- *Wake from `UpdateEngineClass::Cycle`* — `UpdateEngine` is itself only woken on demand (notices/signals); after `Activate` it may not run for arbitrarily long, defeating the wake.
- *Store `engine_id` on the animator and wake from inside `Activate`* — animator still needs scheduler access, so callers still need to plumb `SchedCtx`.

---

## File Structure

**Modified:**

- `crates/emcore/src/emView.rs` — add `wake_visiting_va_engine`; thread `&mut SchedCtx<'_>` through the 6 Visit-family methods (`Visit`, `VisitByIdentity`, `VisitFullsized`, `VisitFullsizedByIdentity`, `VisitPanel`, `VisitByIdentityBare`) and the 11 navigation helpers (`VisitNext`, `VisitPrev`, `VisitFirst`, `VisitLast`, `VisitLeft`, `VisitRight`, `VisitUp`, `VisitDown`, `VisitIn`, `VisitOut`, `VisitNeighbour`); update internal nav-request drain in `Update` (line 2660) and ensure the trailing `self.VisitPanel(tree, current, true)` at line 2936 *inside* `VisitNeighbour` is updated alongside the helper itself; update tests.
- `crates/emcore/src/emWindow.rs` — pass `ctx` through `HandleInput` keyboard navigation block (lines 1111, 1113, 1235-1258).
- `crates/emcore/src/emCtrlSocket.rs` — change `resolve_target` closure type to accept `&mut SchedCtx<'_>` and construct it from `app` fields with split borrows; update `handle_visit` (488), `handle_visit_fullsized` (497), `handle_seek_to` (518), and add an `_ctx` placeholder to `handle_set_focus` (509) since the closure type changed.
- `crates/emcore/src/emViewInputFilter.rs` — pass `ctx` through the two `VisitFullsized` call sites (lines 1656, 1679). Enclosing `do_gesture` already takes `ctx: &mut SchedCtx<'_>`.
- `crates/emcore/src/emSubViewPanel.rs` — `visit_by_identity` (line 155, call at line 165, snake_case) gains a trailing `ctx: &mut SchedCtx<'_>` parameter.
- `crates/emmain/src/emMainWindow.rs` — caller of `emSubViewPanel::visit_by_identity` at line 1339 (in `RecreateContentPanels`); thread `ctx` through.
- `crates/eaglemode/tests/support/pipeline.rs` — pipeline test harness already has the SchedCtx fields (`scheduler`, `framework_actions`, `root_context`, `framework_clipboard`, `pending_actions`); construct a local `SchedCtx` in each Visit-call branch.
- `crates/eaglemode/tests/golden/interaction.rs` — update ~15 call sites; the file already imports `TestSched` from `crates/eaglemode/tests/golden/common.rs:9`, which exposes a `with(|sc| ...)` helper — wrap each `Visit*` call accordingly. No new helper needed.
- `crates/emcore/src/emView.rs` (test module) — update existing test `visiting_va_cycles_when_activated` to remove the manual `wake_up(visiting_id)` workaround and validate the new `Activate→WakeUp` path via the public `Visit*` API.

**No new files.**

---

## Pre-flight verification

- [ ] **Step 0a: Confirm clean tree on `main` at HEAD with F010 status `root-cause-found`**

  Run: `git status && git log -1 --oneline && python3 -c "import json; print([i['status'] for i in json.load(open('docs/debug/ISSUES.json'))['issues'] if i['id']=='F010'][0])"`

  Expected: working tree clean, status `root-cause-found`.

- [ ] **Step 0b: Confirm baseline cargo health**

  Run: `cargo check --workspace 2>&1 | tail -5 && cargo-nextest ntr 2>&1 | tail -5`

  Expected: clean check, all tests passing. (If anything is already red, stop and surface to the user before proceeding — the plan assumes a green baseline.)

---

### Task 1 — Failing test that proves the bug

**Files:**
- Modify: `crates/emcore/src/emView.rs` (test module near `visiting_va_cycles_when_activated` ~ line 7398; place new test directly after it)

This test must fail at HEAD. It exercises the public `VisitByIdentityBare` API (which production callers use) and runs `DoTimeSlice` once, expecting the animator to have been cycled — proving Activate's wake-up path.

- [ ] **Step 1.1: Append the failing test to `crates/emcore/src/emView.rs`**

  Insert immediately after the closing brace of `visiting_va_cycles_when_activated` (closing brace around line 7494, before the next `#[test]`). NOTE: `use crate::emViewAnimator::emViewAnimator as _;` is already imported at the top of `mod tests` (line 5246) — do NOT re-import. The pattern below mirrors the construction style in the existing `visiting_va_cycles_when_activated` test (which uses `Rc<RefCell<EngineScheduler>>` so the scheduler can be re-borrowed for assertions after the visit). The local `TestSched::with` helper at line 5263 is *not* reusable here because we need to inspect `sched.has_awake_engines()` *after* the closure returns.

  ```rust
  /// F010: `VisitByIdentityBare` must wake the wrapper engine so the
  /// animator advances on the next time slice without a manual
  /// `scheduler.wake_up` call. Mirrors C++ `emViewAnimator::Activate →
  /// WakeUp()` (emViewAnimator.cpp:81). At baseline this test FAILS
  /// because `Activate()` only flips a flag.
  #[test]
  fn visit_by_identity_bare_wakes_wrapper_engine() {
      let mut tree = PanelTree::new();
      let root = tree.create_root_deferred_view("root");
      let view_rc = Rc::new(RefCell::new(emView::new(
          crate::emContext::emContext::NewRoot(),
          root,
          800.0,
          600.0,
      )));
      let sched = Rc::new(RefCell::new(EngineScheduler::new()));
      let scope = crate::emPanelScope::PanelScope::Toplevel(winit::window::WindowId::dummy());
      let pa: std::rc::Rc<
          std::cell::RefCell<Vec<crate::emEngineCtx::FrameworkDeferredAction>>,
      > = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
      let cb: std::cell::RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>> =
          std::cell::RefCell::new(None);

      // Register engines (sleeps the visiting engine).
      {
          let mut v = view_rc.borrow_mut();
          let root_ctx = v.Context.GetRootContext();
          let mut fw: Vec<crate::emEngineCtx::DeferredAction> = Vec::new();
          let mut s = sched.borrow_mut();
          let mut sc = crate::emEngineCtx::SchedCtx {
              scheduler: &mut s,
              framework_actions: &mut fw,
              root_context: &root_ctx,
              framework_clipboard: &cb,
              current_engine: None,
              pending_actions: &pa,
          };
          v.RegisterEngines(&mut sc, &mut tree, scope);
      }

      // Drive a programmatic visit through the public API. NO manual
      // wake_up — Activate must take care of it.
      {
          let mut v = view_rc.borrow_mut();
          let root_ctx = v.Context.GetRootContext();
          let mut fw: Vec<crate::emEngineCtx::DeferredAction> = Vec::new();
          let mut s = sched.borrow_mut();
          let mut sc = crate::emEngineCtx::SchedCtx {
              scheduler: &mut s,
              framework_actions: &mut fw,
              root_context: &root_ctx,
              framework_clipboard: &cb,
              current_engine: None,
              pending_actions: &pa,
          };
          v.VisitByIdentityBare("root", false, "test-subject", &mut sc);
      }

      // Animator must be active and queued for the next slice.
      assert!(
          view_rc.borrow().VisitingVA.borrow().is_active(),
          "animator should be active after VisitByIdentityBare",
      );

      // Critical: the wrapper engine must be in the scheduler's wake
      // queue. The bug at HEAD is precisely that it is NOT.
      let visiting_id = view_rc
          .borrow()
          .visiting_va_engine_id
          .expect("RegisterEngines must register VisitingVAEngineClass");
      assert!(
          sched.borrow().has_awake_engines(),
          "scheduler must have at least one awake engine after VisitByIdentityBare \
           (the visiting wrapper); F010: Activate fails to wake it",
      );

      // Direct check: scheduler reports this specific engine as awake.
      // (We use is_idle() which checks all wake queues are empty;
      // because we just woke one engine, idle must be false.)
      assert!(
          !sched.borrow().is_idle(),
          "scheduler::is_idle() must be false: visiting engine {:?} should be queued",
          visiting_id,
      );

      // Cleanup: tear down registered engines so the EngineScheduler's
      // Drop assertions don't trip. Mirrors cleanup in
      // `visiting_va_cycles_when_activated`.
      let mut v = view_rc.borrow_mut();
      if let Some(id) = v.update_engine_id.take() {
          sched.borrow_mut().remove_engine(id);
      }
      if let Some(id) = v.eoi_engine_id.take() {
          sched.borrow_mut().remove_engine(id);
      }
      if let Some(id) = v.visiting_va_engine_id.take() {
          sched.borrow_mut().remove_engine(id);
      }
      if let Some(sig) = v.EOISignal.take() {
          sched.borrow_mut().remove_signal(sig);
      }
  }
  ```

  Note: this test asserts the specific architectural property (engine queued) rather than running `DoTimeSlice` and inspecting downstream state. Reason: `DoTimeSlice` against a deferred-view root with no real panel may early-return; queue-membership is the cleanest mechanical assertion.

- [ ] **Step 1.2: Run the new test and confirm it FAILS**

  Run: `cargo-nextest run -p emcore --test-threads=1 visit_by_identity_bare_wakes_wrapper_engine 2>&1 | tail -20`

  Expected: failure with message indicating `scheduler::is_idle()` returned true (wrapper engine not woken). If the test errors for any other reason (compile error, panic in setup), STOP and fix the test — it must fail for the *right* reason before moving on.

- [ ] **Step 1.3: Commit the failing test**

  ```bash
  git add crates/emcore/src/emView.rs
  git commit -m "test(F010): failing test — VisitByIdentityBare must wake wrapper engine"
  ```

---

### Task 2 — Add `wake_visiting_va_engine` helper

**Files:**
- Modify: `crates/emcore/src/emView.rs` (add helper near existing `WakeUpUpdateEngine`, around line 3312)

- [ ] **Step 2.1: Insert the helper directly below `WakeUpUpdateEngine`**

  Find the existing block:
  ```rust
  pub fn WakeUpUpdateEngine(&mut self, ctx: &mut crate::emEngineCtx::SchedCtx<'_>) {
      if let Some(id) = self.update_engine_id {
          ctx.wake_up(id);
      }
  }
  ```

  Insert immediately after its closing brace. Use `&mut self` to mirror `WakeUpUpdateEngine` exactly (callers already have `&mut self` available in every Visit-family method, so this costs nothing and preserves File and Name Correspondence with the C++ shape):
  ```rust
  /// Wake the scheduler-registered `VisitingVAEngineClass` so its
  /// `Cycle` runs in the current/next time slice and observes
  /// `VisitingVA.is_active()`. Mirror of `WakeUpUpdateEngine`.
  ///
  /// Port of the `WakeUp()` call inside C++ `emViewAnimator::Activate`
  /// (emViewAnimator.cpp:81). The Rust port splits the animator from
  /// its engine, so the wake cannot live inside `Activate()` itself —
  /// the Visit-family methods (`emView::Visit*`) own the wake.
  pub fn wake_visiting_va_engine(&mut self, ctx: &mut crate::emEngineCtx::SchedCtx<'_>) {
      if let Some(id) = self.visiting_va_engine_id {
          ctx.wake_up(id);
      }
  }
  ```

- [ ] **Step 2.2: Verify it compiles**

  Run: `cargo check -p emcore 2>&1 | tail -5`

  Expected: clean compile (helper is unused so far — that's fine; clippy `dead_code` is `warn` not `deny`, and the helper is `pub` so it won't trigger).

- [ ] **Step 2.3: Commit**

  ```bash
  git add crates/emcore/src/emView.rs
  git commit -m "feat(emView): add wake_visiting_va_engine helper (mirror of WakeUpUpdateEngine)"
  ```

---

### Task 3 — Plumb `SchedCtx` through the three base Visit methods

These are the methods that actually call `va.Activate()`. Each grows one parameter (`ctx: &mut SchedCtx<'_>`) and gains one trailing call to `self.wake_visiting_va_engine(ctx)`.

**Files:**
- Modify: `crates/emcore/src/emView.rs` lines 1078-1093, 1113-1126, 1144-1151

- [ ] **Step 3.1: Modify `VisitByIdentity` (lines 1078-1093)**

  Replace:
  ```rust
  pub fn VisitByIdentity(
      &mut self,
      identity: &str,
      rel_x: f64,
      rel_y: f64,
      rel_a: f64,
      adherent: bool,
      subject: &str,
  ) {
      let cfg = self.CoreConfig.borrow();
      let cfg = cfg.GetRec();
      let mut va = self.VisitingVA.borrow_mut();
      va.SetAnimParamsByCoreConfig(cfg);
      va.SetGoalCoords(identity, rel_x, rel_y, rel_a, adherent, subject);
      va.Activate();
  }
  ```

  With:
  ```rust
  pub fn VisitByIdentity(
      &mut self,
      identity: &str,
      rel_x: f64,
      rel_y: f64,
      rel_a: f64,
      adherent: bool,
      subject: &str,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      {
          let cfg = self.CoreConfig.borrow();
          let cfg = cfg.GetRec();
          let mut va = self.VisitingVA.borrow_mut();
          va.SetAnimParamsByCoreConfig(cfg);
          va.SetGoalCoords(identity, rel_x, rel_y, rel_a, adherent, subject);
          va.Activate();
      }
      // F010: mirror C++ emViewAnimator::Activate's WakeUp() call
      // (emViewAnimator.cpp:81) — the Rust animator-engine split moves
      // this wake to the Visit-family methods.
      self.wake_visiting_va_engine(ctx);
  }
  ```

- [ ] **Step 3.2: Modify `VisitFullsizedByIdentity` (lines 1113-1126)**

  Replace:
  ```rust
  pub fn VisitFullsizedByIdentity(
      &mut self,
      identity: &str,
      adherent: bool,
      utilize_view: bool,
      subject: &str,
  ) {
      let cfg = self.CoreConfig.borrow();
      let cfg = cfg.GetRec();
      let mut va = self.VisitingVA.borrow_mut();
      va.SetAnimParamsByCoreConfig(cfg);
      va.SetGoalFullsized(identity, adherent, utilize_view, subject);
      va.Activate();
  }
  ```

  With:
  ```rust
  pub fn VisitFullsizedByIdentity(
      &mut self,
      identity: &str,
      adherent: bool,
      utilize_view: bool,
      subject: &str,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      {
          let cfg = self.CoreConfig.borrow();
          let cfg = cfg.GetRec();
          let mut va = self.VisitingVA.borrow_mut();
          va.SetAnimParamsByCoreConfig(cfg);
          va.SetGoalFullsized(identity, adherent, utilize_view, subject);
          va.Activate();
      }
      self.wake_visiting_va_engine(ctx);
  }
  ```

- [ ] **Step 3.3: Modify `VisitByIdentityBare` (lines 1144-1151)**

  Replace:
  ```rust
  pub fn VisitByIdentityBare(&mut self, identity: &str, adherent: bool, subject: &str) {
      let cfg = self.CoreConfig.borrow();
      let cfg = cfg.GetRec();
      let mut va = self.VisitingVA.borrow_mut();
      va.SetAnimParamsByCoreConfig(cfg);
      va.SetGoal(identity, adherent, subject);
      va.Activate();
  }
  ```

  With:
  ```rust
  pub fn VisitByIdentityBare(
      &mut self,
      identity: &str,
      adherent: bool,
      subject: &str,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      {
          let cfg = self.CoreConfig.borrow();
          let cfg = cfg.GetRec();
          let mut va = self.VisitingVA.borrow_mut();
          va.SetAnimParamsByCoreConfig(cfg);
          va.SetGoal(identity, adherent, subject);
          va.Activate();
      }
      self.wake_visiting_va_engine(ctx);
  }
  ```

- [ ] **Step 3.4: Do not run cargo check yet** — this will break delegators in Task 4. Continue.

---

### Task 4 — Plumb `SchedCtx` through the three thin delegator Visit methods

`Visit`, `VisitFullsized`, `VisitPanel` are pure delegators to the Task 3 base methods. Each grows `ctx`.

**Files:**
- Modify: `crates/emcore/src/emView.rs` lines 1059-1071, 1096-1106, 1133-1137

- [ ] **Step 4.1: Modify `Visit` (lines 1059-1071)**

  Replace:
  ```rust
  pub fn Visit(
      &mut self,
      tree: &PanelTree,
      panel: PanelId,
      rel_x: f64,
      rel_y: f64,
      rel_a: f64,
      adherent: bool,
  ) {
      let identity = tree.GetIdentity(panel);
      let subject = tree.get_title(panel);
      self.VisitByIdentity(&identity, rel_x, rel_y, rel_a, adherent, &subject);
  }
  ```

  With:
  ```rust
  pub fn Visit(
      &mut self,
      tree: &PanelTree,
      panel: PanelId,
      rel_x: f64,
      rel_y: f64,
      rel_a: f64,
      adherent: bool,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      let identity = tree.GetIdentity(panel);
      let subject = tree.get_title(panel);
      self.VisitByIdentity(&identity, rel_x, rel_y, rel_a, adherent, &subject, ctx);
  }
  ```

- [ ] **Step 4.2: Modify `VisitFullsized` (lines 1096-1106)**

  Replace:
  ```rust
  pub fn VisitFullsized(
      &mut self,
      tree: &PanelTree,
      panel: PanelId,
      adherent: bool,
      utilize_view: bool,
  ) {
      let identity = tree.GetIdentity(panel);
      let subject = tree.get_title(panel);
      self.VisitFullsizedByIdentity(&identity, adherent, utilize_view, &subject);
  }
  ```

  With:
  ```rust
  pub fn VisitFullsized(
      &mut self,
      tree: &PanelTree,
      panel: PanelId,
      adherent: bool,
      utilize_view: bool,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      let identity = tree.GetIdentity(panel);
      let subject = tree.get_title(panel);
      self.VisitFullsizedByIdentity(&identity, adherent, utilize_view, &subject, ctx);
  }
  ```

- [ ] **Step 4.3: Modify `VisitPanel` (lines 1133-1137)**

  Replace:
  ```rust
  pub fn VisitPanel(&mut self, tree: &PanelTree, panel: PanelId, adherent: bool) {
      let identity = tree.GetIdentity(panel);
      let subject = tree.get_title(panel);
      self.VisitByIdentityBare(&identity, adherent, &subject);
  }
  ```

  With:
  ```rust
  pub fn VisitPanel(
      &mut self,
      tree: &PanelTree,
      panel: PanelId,
      adherent: bool,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      let identity = tree.GetIdentity(panel);
      let subject = tree.get_title(panel);
      self.VisitByIdentityBare(&identity, adherent, &subject, ctx);
  }
  ```

---

### Task 5 — Plumb `SchedCtx` through 11 navigation helpers

`VisitNext`, `VisitPrev`, `VisitFirst`, `VisitLast`, `VisitLeft`, `VisitRight`, `VisitUp`, `VisitDown`, `VisitIn`, `VisitOut`, `VisitNeighbour` all currently take `(&mut self, &mut PanelTree[, i32])`. Each grows a trailing `ctx: &mut SchedCtx<'_>` and forwards it.

**Files:**
- Modify: `crates/emcore/src/emView.rs` lines ~2674-2780

- [ ] **Step 5.1: Modify each navigation helper signature and forward `ctx`**

  Apply the same pattern to all eleven methods. Example for `VisitNext`:

  Replace:
  ```rust
  pub fn VisitNext(&mut self, tree: &mut PanelTree) {
      let Some(active) = self.active else { return };
      let mut p = tree.GetFocusableNext(active);
      // ... unchanged ...
      if let Some(target) = p {
          self.VisitPanel(tree, target, true);
      }
  }
  ```

  With:
  ```rust
  pub fn VisitNext(&mut self, tree: &mut PanelTree, ctx: &mut crate::emEngineCtx::SchedCtx<'_>) {
      let Some(active) = self.active else { return };
      let mut p = tree.GetFocusableNext(active);
      // ... unchanged ...
      if let Some(target) = p {
          self.VisitPanel(tree, target, true, ctx);
      }
  }
  ```

  Apply the equivalent transformation to:
  - `VisitPrev` — adds `ctx`, forwards to `VisitPanel(...., ctx)`
  - `VisitFirst` — adds `ctx`, forwards to `VisitPanel(...., ctx)`
  - `VisitLast` — adds `ctx`, forwards to `VisitPanel(...., ctx)`
  - `VisitLeft` — adds `ctx`, forwards to `VisitNeighbour(tree, 2, ctx)`
  - `VisitRight` — adds `ctx`, forwards to `VisitNeighbour(tree, 0, ctx)`
  - `VisitUp` — adds `ctx`, forwards to `VisitNeighbour(tree, 3, ctx)`
  - `VisitDown` — adds `ctx`, forwards to `VisitNeighbour(tree, 1, ctx)`
  - `VisitIn` — adds `ctx`, forwards to both `VisitPanel(...., ctx)` and `VisitFullsized(...., ctx)`
  - `VisitOut` — adds `ctx`, forwards to `VisitPanel(...., ctx)` and `Visit(tree, root, 0.0, 0.0, rel_a, true, ctx)`
  - `VisitNeighbour` — signature becomes `(&mut self, tree: &mut PanelTree, direction: i32, ctx: &mut crate::emEngineCtx::SchedCtx<'_>)`; the inner `self.VisitPanel(...)` and `self.VisitFullsized(...)` calls (search for them within the function body) gain `, ctx` as the last argument.

- [ ] **Step 5.2: Internal `Update` nav-request drain (line 2660)**

  Find:
  ```rust
  let nav_requests = tree.drain_navigation_requests();
  for target in nav_requests {
      self.VisitFullsized(tree, target, false, false);
  }
  ```

  Replace with:
  ```rust
  let nav_requests = tree.drain_navigation_requests();
  for target in nav_requests {
      self.VisitFullsized(tree, target, false, false, ctx);
  }
  ```

  (`Update` already takes `ctx: &mut SchedCtx<'_>`.)

  Note: `VisitNeighbour` (the function spanning ~lines 2781-2937) contains *two* internal `self.VisitPanel`/`self.VisitFullsized` calls — the trailing `self.VisitPanel(tree, current, true)` at line 2936 is the second one and must also be updated to forward `ctx`. Step 5.1's "search for them within the function body" instruction covers it; flagged here so the executing agent does not stop at the first occurrence.

- [ ] **Step 5.3: Verify the file compiles in isolation**

  Run: `cargo check -p emcore 2>&1 | tail -30`

  Expected: errors only at *external* call sites (other crates and other emcore files that call `Visit*`). The emView.rs file itself should be internally consistent. If any error originates *inside* emView.rs, fix it (likely a missed delegate) before proceeding.

---

### Task 6 — Update `emWindow.rs` keyboard navigation callers

**Files:**
- Modify: `crates/emcore/src/emWindow.rs` lines 1111, 1113, 1235-1258

The surrounding method `HandleInput` already has a `ctx` parameter (search the function signature; if the local name is `ctx` keep as is, else rename `ctx` to match local convention).

- [ ] **Step 6.1: Update Tab/Shift+Tab (lines 1111-1113)**

  Replace:
  ```rust
  if state.GetShift() {
      self.view.VisitPrev(tree);
  } else {
      self.view.VisitNext(tree);
  }
  ```

  With:
  ```rust
  if state.GetShift() {
      self.view.VisitPrev(tree, ctx);
  } else {
      self.view.VisitNext(tree, ctx);
  }
  ```

- [ ] **Step 6.2: Update arrow/Home/End/PageUp/PageDown block (lines 1235-1258)**

  Replace:
  ```rust
  match event.key {
      InputKey::ArrowLeft if state.IsNoMod() => self.view.VisitLeft(tree),
      InputKey::ArrowRight if state.IsNoMod() => self.view.VisitRight(tree),
      InputKey::ArrowUp if state.IsNoMod() => self.view.VisitUp(tree),
      InputKey::ArrowDown if state.IsNoMod() => self.view.VisitDown(tree),

      InputKey::Home if state.IsNoMod() => self.view.VisitFirst(tree),
      InputKey::Home if state.IsAltMod() => {
          if let Some(p) = self.view.GetActivePanel() {
              let adherent = self.view.IsActivationAdherent();
              self.view.VisitFullsized(tree, p, adherent, false);
          }
      }
      InputKey::Home if state.IsShiftAltMod() => {
          if let Some(p) = self.view.GetActivePanel() {
              let adherent = self.view.IsActivationAdherent();
              self.view.VisitFullsized(tree, p, adherent, true);
          }
      }

      InputKey::End if state.IsNoMod() => self.view.VisitLast(tree),
      InputKey::PageUp if state.IsNoMod() => self.view.VisitOut(tree),
      InputKey::PageDown if state.IsNoMod() => self.view.VisitIn(tree),

      _ => {}
  }
  ```

  With:
  ```rust
  match event.key {
      InputKey::ArrowLeft if state.IsNoMod() => self.view.VisitLeft(tree, ctx),
      InputKey::ArrowRight if state.IsNoMod() => self.view.VisitRight(tree, ctx),
      InputKey::ArrowUp if state.IsNoMod() => self.view.VisitUp(tree, ctx),
      InputKey::ArrowDown if state.IsNoMod() => self.view.VisitDown(tree, ctx),

      InputKey::Home if state.IsNoMod() => self.view.VisitFirst(tree, ctx),
      InputKey::Home if state.IsAltMod() => {
          if let Some(p) = self.view.GetActivePanel() {
              let adherent = self.view.IsActivationAdherent();
              self.view.VisitFullsized(tree, p, adherent, false, ctx);
          }
      }
      InputKey::Home if state.IsShiftAltMod() => {
          if let Some(p) = self.view.GetActivePanel() {
              let adherent = self.view.IsActivationAdherent();
              self.view.VisitFullsized(tree, p, adherent, true, ctx);
          }
      }

      InputKey::End if state.IsNoMod() => self.view.VisitLast(tree, ctx),
      InputKey::PageUp if state.IsNoMod() => self.view.VisitOut(tree, ctx),
      InputKey::PageDown if state.IsNoMod() => self.view.VisitIn(tree, ctx),

      _ => {}
  }
  ```

- [ ] **Step 6.3: Verify**

  Run: `cargo check -p emcore 2>&1 | tail -30`

  Expected: emWindow.rs is now consistent; remaining errors should be confined to `emCtrlSocket.rs`, `emViewInputFilter.rs`, `emSubViewPanel.rs`, and tests.

---

### Task 7 — Update `emViewInputFilter.rs` callers

**Files:**
- Modify: `crates/emcore/src/emViewInputFilter.rs` lines 1656, 1679

The enclosing function already has `ctx: &mut SchedCtx<'_>` in scope (verify with `grep -nB30 'view.VisitFullsized' crates/emcore/src/emViewInputFilter.rs | grep -E 'fn |ctx:' | head -5`).

- [ ] **Step 7.1: Update both call sites**

  Replace `view.VisitFullsized(tree, panel, true, false);` (line 1656) with `view.VisitFullsized(tree, panel, true, false, ctx);`.

  Replace `view.VisitFullsized(tree, panel, true, true);` (line 1679) with `view.VisitFullsized(tree, panel, true, true, ctx);`.

  If the enclosing function does NOT have `ctx` in scope, propagate it through the call chain — search upward (`grep -nB5 'fn ' crates/emcore/src/emViewInputFilter.rs | grep -B1 -A1 <enclosing-fn-name>`) and add `ctx: &mut crate::emEngineCtx::SchedCtx<'_>` as the trailing parameter, then update its callers. (Most VIF methods already take `ctx`.)

- [ ] **Step 7.2: Verify**

  Run: `cargo check -p emcore 2>&1 | tail -20`

---

### Task 8 — Update `emSubViewPanel.rs` caller and propagate to `emmain`

**Files:**
- Modify: `crates/emcore/src/emSubViewPanel.rs` lines 155-166 (method `visit_by_identity`)
- Modify: `crates/emmain/src/emMainWindow.rs` line 1339 (only external caller of `visit_by_identity`)

The enclosing method is named **`visit_by_identity`** (snake_case `RUST_ONLY:` API on `emSubViewPanel`, *not* the C++-named `VisitByIdentity`). Its signature does not currently accept `ctx`; we must add the parameter and propagate to its sole external caller in `emmain/src/emMainWindow.rs:1339`.

- [ ] **Step 8.1: Add `ctx` parameter to `visit_by_identity` and forward**

  Replace (lines 155-166):
  ```rust
  pub fn visit_by_identity(
      &mut self,
      identity: &str,
      rel_x: f64,
      rel_y: f64,
      rel_a: f64,
      adherent: bool,
      subject: &str,
  ) {
      self.sub_view
          .VisitByIdentity(identity, rel_x, rel_y, rel_a, adherent, subject);
  }
  ```

  With:
  ```rust
  pub fn visit_by_identity(
      &mut self,
      identity: &str,
      rel_x: f64,
      rel_y: f64,
      rel_a: f64,
      adherent: bool,
      subject: &str,
      ctx: &mut crate::emEngineCtx::SchedCtx<'_>,
  ) {
      self.sub_view
          .VisitByIdentity(identity, rel_x, rel_y, rel_a, adherent, subject, ctx);
  }
  ```

- [ ] **Step 8.2: Update the `emmain` caller at `emMainWindow.rs:1339`**

  The call lives inside a `with_behavior_as::<emSubViewPanel, _>(svp_id, |svp| { ... })` closure inside `RecreateContentPanels`. Inspect the enclosing function — search backwards from line 1339 for the `fn ` keyword. If the enclosing function takes `ctx: &mut SchedCtx<'_>`, pass it through; otherwise propagate the parameter up the call chain (likely one or two levels) until you reach a frame that already has `SchedCtx` in scope.

  Run: `awk 'NR>=1280 && NR<=1345' crates/emmain/src/emMainWindow.rs` to inspect.

  Replace `svp.visit_by_identity(&identity, rel_x, rel_y, rel_a, adherent, &title);` with `svp.visit_by_identity(&identity, rel_x, rel_y, rel_a, adherent, &title, ctx);` (substitute the actual binding name if it differs).

- [ ] **Step 8.3: Verify**

  Run: `cargo check --workspace 2>&1 | tail -30`

  Expected: errors confined to `emCtrlSocket.rs` and tests. If `emmain` still has errors, propagation up the call chain is incomplete — extend `RecreateContentPanels` (and any intermediates) to take `ctx`.

---

### Task 9 — Update `emCtrlSocket.rs` `resolve_target` and three handlers

**Files:**
- Modify: `crates/emcore/src/emCtrlSocket.rs` lines 92-153, 488-507, 518-530

The `resolve_target` closure type does not currently include `&mut SchedCtx<'_>`. We extend the closure signature and construct the `SchedCtx` inside `resolve_target` using split borrows on the `App` struct.

- [ ] **Step 9.1: Inspect `App` struct fields**

  Run: `grep -n "pub.*scheduler\|pub.*framework_actions\|pub.*context\|pub.*clipboard\|pub.*pending_actions\|pub.*windows\|pub.*home_window_id" crates/emcore/src/emGUIFramework.rs | head -15`

  Confirm these fields exist on `App`: `scheduler`, `framework_actions`, `context` (root context), `clipboard`, `pending_actions`, `windows`, `home_window_id` (verified at audit time: lines 158, 159, 166, 172, 196, 214, 224 in `emGUIFramework.rs`). The exact pattern to mirror is already used in `emGUIFramework.rs:482-511`:
  ```rust
  let App {
      scheduler,
      framework_actions,
      windows,
      clipboard,
      pending_actions,
      ..
  } = self;
  ```
  Note: type `framework_actions: Vec<FrameworkDeferredAction>` in `emGUIFramework.rs` is `pub use crate::emEngineCtx::DeferredAction as FrameworkDeferredAction` (the engine-level enum). It matches `SchedCtx::framework_actions: &mut Vec<DeferredAction>` exactly. `pending_actions: Rc<RefCell<Vec<DeferredAction>>>` (the boxed-closure type) matches `SchedCtx::pending_actions: &Rc<RefCell<Vec<FrameworkDeferredAction>>>` (where SchedCtx's `FrameworkDeferredAction` is the boxed closure). The naming is confusing but the types compose correctly.

- [ ] **Step 9.2: Modify `resolve_target` signature and body**

  Replace the function body (lines 92-153) — keep the existing structure but extend the closure type and construct `SchedCtx`:

  ```rust
  pub(crate) fn resolve_target<R>(
      app: &mut App,
      view_sel: &str,
      identity: &str,
      f: impl FnOnce(&mut crate::emView::emView, &mut PanelTree, PanelId, &mut crate::emEngineCtx::SchedCtx<'_>) -> R,
  ) -> Result<R, String> {
      let home_id = app
          .home_window_id
          .ok_or_else(|| "home window not initialized".to_string())?;

      // Split-borrow the App: scheduler/framework_actions/context/etc.
      // are independent of `windows`. We construct a SchedCtx that
      // outlives the closure call below.
      let App {
          windows,
          scheduler,
          framework_actions,
          context,
          clipboard,
          pending_actions,
          ..
      } = app;
      let mut sc = crate::emEngineCtx::SchedCtx {
          scheduler,
          framework_actions,
          root_context: context,
          framework_clipboard: clipboard,
          current_engine: None,
          pending_actions,
      };

      let win = windows
          .get_mut(&home_id)
          .ok_or_else(|| "home window missing".to_string())?;

      if view_sel.is_empty() {
          let tree = &mut win.tree;
          let view = &mut win.view;
          let root = tree
              .GetRootPanel()
              .ok_or_else(|| "no root panel".to_string())?;
          let target = resolve_identity(tree, root, identity)?;
          return Ok(f(view, tree, target, &mut sc));
      }

      let svp_id = {
          let outer_root = win
              .tree
              .GetRootPanel()
              .ok_or_else(|| "no root panel".to_string())?;
          resolve_identity(&win.tree, outer_root, view_sel)?
      };
      let svp_name = win
          .tree
          .name(svp_id)
          .unwrap_or("<unnamed>")
          .to_string();

      let result = win
          .tree
          .with_behavior_as::<crate::emSubViewPanel::emSubViewPanel, _>(svp_id, |svp| {
              let (sub_view, sub_tree) = svp.sub_view_and_tree_mut();
              let sub_root = sub_tree
                  .GetRootPanel()
                  .ok_or_else(|| "sub-view has no root panel".to_string())?;
              let inner_target = resolve_identity(sub_tree, sub_root, identity)?;
              Ok::<R, String>(f(sub_view, sub_tree, inner_target, &mut sc))
          })
          .ok_or_else(|| {
              format!(
                  "view selector '{}' resolved to panel '{}' which is not a sub-view panel",
                  view_sel, svp_name
              )
          })?;
      result
  }
  ```

  Note: the field names `context`/`clipboard` in the destructure must match the actual `App` struct field names verified in Step 9.1. Adjust if e.g. the App field is `gui_clipboard` instead of `clipboard`. Reference: `emGUIFramework.rs:1036-1043` shows the mapping used in `WindowEvent::Touch`:
  - `scheduler: &mut self.scheduler`
  - `framework_actions: &mut self.framework_actions`
  - `root_context: &self.context`
  - `framework_clipboard: &self.clipboard`
  - `pending_actions: &self.pending_actions`

  Field names match the mapping above by inspection — but verify before editing.

- [ ] **Step 9.3: Update `handle_visit` (line 488)**

  Replace:
  ```rust
  fn handle_visit(app: &mut App, view_sel: &str, identity: &str, adherent: bool) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, tree, target| {
          view.VisitPanel(tree, target, adherent);
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

  With:
  ```rust
  fn handle_visit(app: &mut App, view_sel: &str, identity: &str, adherent: bool) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, tree, target, ctx| {
          view.VisitPanel(tree, target, adherent, ctx);
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

- [ ] **Step 9.4: Update `handle_visit_fullsized` (line 497)**

  Replace:
  ```rust
  fn handle_visit_fullsized(app: &mut App, view_sel: &str, identity: &str) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, tree, target| {
          view.VisitFullsized(tree, target, false, false);
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

  With:
  ```rust
  fn handle_visit_fullsized(app: &mut App, view_sel: &str, identity: &str) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, tree, target, ctx| {
          view.VisitFullsized(tree, target, false, false, ctx);
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

- [ ] **Step 9.5: Update `handle_seek_to` (line 518)**

  Replace:
  ```rust
  fn handle_seek_to(app: &mut App, view_sel: &str, identity: &str) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, tree, target| {
          view.VisitPanel(tree, target, false);
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

  With:
  ```rust
  fn handle_seek_to(app: &mut App, view_sel: &str, identity: &str) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, tree, target, ctx| {
          view.VisitPanel(tree, target, false, ctx);
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

- [ ] **Step 9.6: Update `handle_set_focus` if needed (line 509)**

  This handler does not currently call `Visit*` (it calls `view.set_focus(Some(target))`). However, the closure type changed, so the closure needs the extra `_ctx` parameter (prefixed with `_` since unused):

  Replace:
  ```rust
  fn handle_set_focus(app: &mut App, view_sel: &str, identity: &str) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, _tree, target| {
          view.set_focus(Some(target));
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

  With:
  ```rust
  fn handle_set_focus(app: &mut App, view_sel: &str, identity: &str) -> CtrlReply {
      match resolve_target(app, view_sel, identity, |view, _tree, target, _ctx| {
          view.set_focus(Some(target));
      }) {
          Ok(()) => CtrlReply::ok(),
          Err(e) => CtrlReply::err(e),
      }
  }
  ```

- [ ] **Step 9.7: Verify**

  Run: `cargo check -p emcore 2>&1 | tail -30`

  Expected: emCtrlSocket.rs internally consistent; remaining errors confined to test crates.

---

### Task 10 — Commit production-side changes (intermediate)

- [ ] **Step 10.1: Commit cumulative production-side changes**

  ```bash
  git add crates/emcore/src/emView.rs crates/emcore/src/emWindow.rs \
          crates/emcore/src/emCtrlSocket.rs crates/emcore/src/emViewInputFilter.rs \
          crates/emcore/src/emSubViewPanel.rs
  git commit -m "fix(F010): plumb SchedCtx through Visit* and wake visiting engine"
  ```

  Note: the workspace will not yet build — golden tests and unit tests still need updates (Tasks 11-13). This is an intermediate commit; the commit message does NOT claim "fix" alone but pairs with the failing test from Task 1 that still fails until Task 14.

---

### Task 11 — Update `tests/support/pipeline.rs`

**Files:**
- Modify: `crates/eaglemode/tests/support/pipeline.rs` lines 348-350, 443-466

This test harness drives keyboard navigation. The `pipeline` struct already has a scheduler in scope (or can construct a SchedCtx the way `emWindow` does). Inspect the surrounding methods first.

- [ ] **Step 11.1: Inspect the surrounding methods**

  Run: `awk 'NR==340,NR==470' crates/eaglemode/tests/support/pipeline.rs`

  Identify how `self.tree` / `self.view` / `self.scheduler` are exposed. Locate where a `SchedCtx` is currently built (search for `SchedCtx`).

- [ ] **Step 11.2: Construct a `SchedCtx` once per call site**

  Around lines 348-350 and 443-466, before each `view.Visit*(...)` call, construct a local `SchedCtx`:

  ```rust
  let mut sc = crate::emcore::emEngineCtx::SchedCtx {
      scheduler: &mut self.scheduler,           // adjust field path as needed
      framework_actions: &mut self.framework_actions,
      root_context: &self.root_context,
      framework_clipboard: &self.clipboard,
      current_engine: None,
      pending_actions: &self.pending_actions,
  };
  ```

  Then update each `Visit*` call to pass `&mut sc` as its trailing arg. Example:

  Replace:
  ```rust
  InputKey::ArrowLeft if st.IsNoMod() => self.view.VisitLeft(&mut self.tree),
  ```

  With:
  ```rust
  InputKey::ArrowLeft if st.IsNoMod() => self.view.VisitLeft(&mut self.tree, &mut sc),
  ```

  If the pipeline harness lacks one of the fields (e.g. no `framework_actions`), check existing tests in `crates/emcore/src/emView.rs` (around line 5263, see `with` helper there) for the established pattern of constructing a stub `SchedCtx` from local owned values.

  *Pattern: read the existing helper at `emView.rs:5263-5310` for a reusable stub-SchedCtx construction; inline the same pattern in pipeline.rs if no shared constructor exists.*

- [ ] **Step 11.3: Verify**

  Run: `cargo check -p eaglemode --tests 2>&1 | tail -30`

---

### Task 12 — Update `tests/golden/interaction.rs`

**Files:**
- Modify: `crates/eaglemode/tests/golden/interaction.rs` ~15 call sites

These are unit-style tests that build a view and call `view.Visit*(...)` in sequence. Each call site needs a `SchedCtx`.

- [ ] **Step 12.1: Inspect the test setup**

  Run: `awk 'NR==180,NR==240' crates/eaglemode/tests/golden/interaction.rs`

  Determine the standing pattern (single shared scheduler? per-test scheduler? helper function?).

- [ ] **Step 12.2: Reuse the existing `TestSched` helper**

  At audit time, `interaction.rs` already imports `TestSched` from `crates/eaglemode/tests/golden/common.rs:9` (via `use super::common::*;` at line 4) and calls `ts.with(|sc| view.Update(&mut tree, sc));`. Reuse that helper for every `Visit*` call: each test that calls `Visit*` either already constructs `let mut ts = TestSched::new();` (e.g. `three_panel_tree`) or trivially can. **Do not** add a duplicate helper. Pattern:

  ```rust
  ts.with(|sc| view.VisitNext(&mut tree, sc));
  ```

  Skip the rest of this Step. The fallback helper definition below is kept only as a contingency if `TestSched`'s API is found insufficient (e.g. needs a different `EngineScheduler` lifetime):

  ```rust
  /// Build a throwaway SchedCtx for unit-test sequences that drive
  /// `view.Visit*` directly. Mirrors the pattern in
  /// `crates/emcore/src/emView.rs::tests::with`.
  fn with_sched_ctx<R>(
      scheduler: &mut emcore::emScheduler::EngineScheduler,
      f: impl FnOnce(&mut emcore::emEngineCtx::SchedCtx<'_>) -> R,
  ) -> R {
      let root_ctx = emcore::emContext::emContext::NewRoot();
      let mut fw: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
      let cb: std::cell::RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> =
          std::cell::RefCell::new(None);
      let pa: std::rc::Rc<
          std::cell::RefCell<Vec<emcore::emEngineCtx::FrameworkDeferredAction>>,
      > = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
      let mut sc = emcore::emEngineCtx::SchedCtx {
          scheduler,
          framework_actions: &mut fw,
          root_context: &root_ctx,
          framework_clipboard: &cb,
          current_engine: None,
          pending_actions: &pa,
      };
      f(&mut sc)
  }
  ```

  (Adjust crate path prefix to match how `emcore` is imported in this file — likely `eaglemode::...` or similar; check the existing imports.)

- [ ] **Step 12.3: Wrap each `view.Visit*(...)` call with `ts.with(...)`**

  Pattern: replace `view.VisitNext(&mut tree);` with `ts.with(|sc| view.VisitNext(&mut tree, sc));` (using the existing `TestSched` binding `ts` already constructed in each test).

  Apply to all 15 call sites at lines 199, 225, 265, 305, 345, 372, 411, 451, 491, 531, 571, 611, 750, 791, 818.

  Each test currently must have (or trivially construct) a `let mut ts = TestSched::new();` binding — the file's existing tests already follow this pattern. If a particular test does not have a `ts` in scope, add one at its top.

- [ ] **Step 12.4: Verify**

  Run: `cargo check -p eaglemode --tests 2>&1 | tail -30`

---

### Task 13 — Update `emView.rs` unit tests

**Files:**
- Modify: `crates/emcore/src/emView.rs` lines 5400, 5404, 5422, 5426, 5456, 5460, 6351, 6356, **6379**, 6403, 7569

These are inline tests inside emView.rs's `#[cfg(test)] mod tests`. Most already construct a local `SchedCtx` via the `with` helper (at line 5263 — `TestSched::with(|sc| ...)`) or similar. Each `view.Visit*` call needs the matching `ctx`.

NOTE on the audit: line **6379** (`view.Visit(&tree, child, 0.25, 0.5, 2.0, false);`) is the *six-arg* base `Visit` method and was missing from the original line list. Eleven call sites total in emView.rs tests, not ten. Also confirm whether call sites at 5400/5404 etc. use `ts.with(|sc| ...)` wrapping or have a free `sc`/`ctx` already in scope — some tests use `TestSched::new()` and call `.with(|sc| view.Update(...))`; for `Visit*` calls outside such a closure the call must be wrapped.

- [ ] **Step 13.1: Locate each call and confirm a `sc` / `ctx` is already in scope**

  Run: `for ln in 5400 5404 5422 5426 5456 5460 6351 6356 6379 6403 7569; do echo "=== line $ln ==="; awk -v ln=$ln 'NR>=ln-15 && NR<=ln+2' crates/emcore/src/emView.rs; done`

  For each: identify the locally-available `SchedCtx` binding (likely `sc` or `ctx`). If absent, wrap in `ts.with(|sc| view.Visit*(...., sc))`.

- [ ] **Step 13.2: Append the binding to each call**

  Apply: `view.VisitNext(&mut tree)` → `view.VisitNext(&mut tree, &mut sc)` (or whatever the binding name is) — or wrap in `ts.with(|sc| ...)` if no SchedCtx is currently in scope.

  Repeat for all eleven call sites.

- [ ] **Step 13.3: Verify**

  Run: `cargo check -p emcore --tests 2>&1 | tail -20`

---

### Task 14 — Verify failing test now passes; remove manual wake_up workaround

**Files:**
- Modify: `crates/emcore/src/emView.rs` line 7454 (inside `visiting_va_cycles_when_activated`)

- [ ] **Step 14.1: Run the formerly failing test from Task 1**

  Run: `cargo-nextest run -p emcore --test-threads=1 visit_by_identity_bare_wakes_wrapper_engine 2>&1 | tail -10`

  Expected: PASS. If it fails, the wake helper isn't being called — re-trace Task 3 changes.

- [ ] **Step 14.2: Update the existing test `visiting_va_cycles_when_activated`**

  Find (lines 7437-7454):
  ```rust
      // Activate the animator — SetGoal + Activate, matching the
      // delegation shape Visit-family methods will use in Phase 3.
      {
          let view = view_rc.borrow();
          let mut va = view.VisitingVA.borrow_mut();
          va.SetGoal("root", false, "");
          va.Activate();
      }
      assert!(
          view_rc.borrow().VisitingVA.borrow().is_active(),
          "animator should be active after SetGoal + Activate"
      );

      // Tick the scheduler. With view-direct weak (SP8 Phase 1), Cycle
      // upgrades the weak successfully and calls va.animate. The animator
      // either progresses (remaining active) or cleanly deactivates.
      sched.borrow_mut().wake_up(visiting_id);
  ```

  Replace with (NOTE the audit finding: at HEAD, the outer `__cb` declared at line 7419 dies at the closing brace of the `RegisterEngines` block at line 7430. The new block must declare its own `__cb`. `__pa` *is* still alive at outer scope — declared at line 7411 — so `&__pa` works):
  ```rust
      // Drive the animator through the public `Visit*` API — F010
      // moved the wake from raw `va.Activate()` to
      // `emView::wake_visiting_va_engine`, which the Visit-family
      // methods invoke automatically.
      {
          let mut v = view_rc.borrow_mut();
          let root_ctx = v.Context.GetRootContext();
          let mut fw: Vec<crate::emEngineCtx::DeferredAction> = Vec::new();
          let mut s = sched.borrow_mut();
          let __cb_visit: std::cell::RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>> =
              std::cell::RefCell::new(None);
          let mut sc = crate::emEngineCtx::SchedCtx {
              scheduler: &mut s,
              framework_actions: &mut fw,
              root_context: &root_ctx,
              framework_clipboard: &__cb_visit,
              current_engine: None,
              pending_actions: &__pa,
          };
          v.VisitByIdentityBare("root", false, "", &mut sc);
      }
      assert!(
          view_rc.borrow().VisitingVA.borrow().is_active(),
          "animator should be active after VisitByIdentityBare"
      );
      // Engine should already be queued — no manual wake_up needed.
      // Drop the unused visiting_id binding (was used only for the
      // removed manual wake_up); silence the warning by prefixing:
      let _ = visiting_id;
  ```

  After the replacement, the existing trailing block (lines 7455-7494) that calls `DoTimeSlice` and cleans up engines is untouched — it still references `__pa`, `__cb` (re-declared at 7461 inside that block), `sched`, etc. as before.

- [ ] **Step 14.3: Run the existing test**

  Run: `cargo-nextest run -p emcore --test-threads=1 visiting_va_cycles_when_activated 2>&1 | tail -10`

  Expected: PASS.

- [ ] **Step 14.4: Commit the test updates**

  ```bash
  git add crates/emcore/src/emView.rs crates/eaglemode/tests/golden/interaction.rs \
          crates/eaglemode/tests/support/pipeline.rs
  git commit -m "test(F010): update tests for new SchedCtx-bearing Visit* API; drop manual wake_up workaround"
  ```

---

### Task 15 — Full workspace verification

- [ ] **Step 15.1: `cargo check`**

  Run: `cargo check --workspace 2>&1 | tail -10`

  Expected: clean.

- [ ] **Step 15.2: `cargo clippy -- -D warnings`**

  Run: `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`

  Expected: clean. If any clippy warning fires (likely `clippy::too_many_arguments` on the now-8-arg `VisitByIdentity`), per CLAUDE.md "Do NOT" section the *too-many-arguments* warning is allowed — confirm this is the only suppression class triggered, and add `#[allow(clippy::too_many_arguments)]` on the offending methods only if needed.

- [ ] **Step 15.3: `cargo-nextest ntr`**

  Run: `cargo-nextest ntr 2>&1 | tail -20`

  Expected: all tests pass, including the new `visit_by_identity_bare_wakes_wrapper_engine` and the updated `visiting_va_cycles_when_activated`.

- [ ] **Step 15.4: Golden tests**

  Run: `cargo test --test golden -- --test-threads=1 2>&1 | tail -10`

  Expected: pass (or unchanged divergence count). Run `python3 scripts/divergence_report.py --diff` to confirm no regressions in pixel parity.

- [ ] **Step 15.5: Commit if any post-fix touch-ups landed**

  If `clippy --fix` or any small adjustment was needed:
  ```bash
  git add -u
  git commit -m "fix(F010): post-verification touch-ups (clippy, fmt)"
  ```

---

### Task 16 — Manual F010 verification (runtime)

This task confirms the user-visible symptom resolves. It is *not* automatable from this plan — the executing agent should hand off to the user with explicit instructions.

- [ ] **Step 16.1: Print the verification recipe for the human**

  Output to the user:

  ```
  F010 fix is committed. Manual verification needed:

  1. Launch the app: `cargo run --release --bin eaglemode`
  2. Run the canonical capture sequence via the control channel:

     # Baseline
     emctrl dump > /tmp/F010_baseline.emTreeDump

     # Visit cosmos via the inner content sub-view
     emctrl visit view='root:content view' identity=':'
     emctrl wait_idle

     # Capture again
     emctrl dump > /tmp/F010_after_cosmos.emTreeDump

  3. Diff the two dumps. Expect:
     - Inner content view's `Current XYWH` ≠ `Home XYWH`.
     - Cosmos panel: `Viewed: yes`, `PaintCount > 0`.

  4. With the GUI window: zoom into a directory. Expect entries to
     render (no extended blank phase after "Loading NN%").

  5. If verification passes, close F010:
     - Set status to `closed` in docs/debug/ISSUES.json.
     - Populate `fixed_in_commit` and `fixed_date` (today).
     - Otherwise, reopen with the new symptom in `fix_note`.
  ```

- [ ] **Step 16.2: Stop**

  Do not modify ISSUES.json — the human owns the close transition for `needs-manual-verification`-style fixes.

---

## Self-Review Checklist (executed by plan author at write time)

**Spec coverage:** Each Visit-family method (6) + each navigation helper (11) is touched in Tasks 3-5. Each production caller (emWindow ×2 sites, emCtrlSocket ×3+1 closure-shape, emViewInputFilter ×2, emSubViewPanel ×1, emmain ×1) is touched in Tasks 6-9. Each test caller location (interaction.rs ×15, pipeline.rs ~10, emView.rs unit tests ×11 — including the previously-missed line 6379) is touched in Tasks 11-13. The TDD failing test is Task 1, the passing assertion is Task 14. The manual repro is Task 16. ✓

**Placeholder scan:** No "TBD"/"add appropriate handling"/"similar to Task N" without code. The Task 5 navigation-helper transformation lists each method with its exact forwarding call. The Task 11 pipeline.rs section relies on inspecting the file (Step 11.1) — flagged because the harness has historically diverged in field names; the agent will read the file and adapt rather than blindly editing. ✓

**Type consistency:** New helper signature `wake_visiting_va_engine(&mut self, &mut SchedCtx<'_>)` — mirrors `WakeUpUpdateEngine`'s `&mut self` shape; used in Task 3 with the same name. The new parameter `ctx: &mut crate::emEngineCtx::SchedCtx<'_>` is identical across Tasks 3-9. The closure type in `resolve_target` (`FnOnce(&mut emView, &mut PanelTree, PanelId, &mut SchedCtx) -> R`) is consistent across the three handlers + `handle_set_focus`. ✓

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-25-f010-visiting-va-wakeup.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. Particularly useful here because Tasks 11-13 require file-inspection-then-adapt patterns.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
