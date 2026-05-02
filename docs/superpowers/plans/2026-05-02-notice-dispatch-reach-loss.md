# Notice-dispatch PanelCtx reach loss Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the GUI panic at `emFileLinkPanel::AutoExpand` and the four latent silent-degradation sites in `emView::handle_notice_one` by threading the three missing scheduler-reach handles through `HandleNotice` / `handle_notice_one`, replacing the five `PanelCtx::with_scheduler + 2/5 override` blocks with `with_sched_reach`, then deleting `PanelCtx::with_scheduler` after all callers migrate.

**Architecture:** F013 mirror at a different construction site. Three coordinated signature changes: `HandleNotice` and `handle_notice_one` gain three handle params (`framework_actions`, `framework_clipboard`, `pending_actions`); the five dispatch sites in `handle_notice_one` switch from `PanelCtx::with_scheduler` (1/5 reach) to `PanelCtx::with_sched_reach` (5/5 reach) with `view_context` set after; `PanelCtx::with_scheduler` deleted post-migration.

**Tech Stack:** Rust, cargo, cargo-nextest. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-05-02-notice-dispatch-reach-loss-design.md`
**Investigation:** `docs/debug/investigations/notice-dispatch-reach-loss.md`
**Precedent:** F013 regression test at `crates/emcore/src/emPanelTree.rs:3197-3247`

---

## Task 1: Failing regression test (TDD red)

**Files:**
- Create: `crates/emcore/tests/notice_dispatch_reach.rs`

This task adds the regression test against the **current** `HandleNotice` signature (4 params). The probe records `as_sched_ctx().is_some()` in each of `notice`, `AutoExpand`, `AutoShrink`, `LayoutChildren`. With the current code the assertions all fail because three handles are dropped. Task 2 will extend the signatures and update the test's `HandleNotice` call to the 7-param shape, turning the test green.

- [ ] **Step 1: Read F013 regression test for the probe pattern**

Run: `sed -n '3196,3247p' crates/emcore/src/emPanelTree.rs`
Expected: see the `SchedReachProbe` struct definition. Note: it implements `PanelBehavior`, records into `Rc<Cell<bool>>`, the assertion compares `had_reach.get()`.

- [ ] **Step 2: Write the failing test file**

```rust
//! Regression test for notice-dispatch PanelCtx reach loss.
//!
//! Spec: docs/superpowers/specs/2026-05-02-notice-dispatch-reach-loss-design.md
//! Investigation: docs/debug/investigations/notice-dispatch-reach-loss.md
//!
//! Asserts that the per-callback `PanelCtx` built inside
//! `emView::handle_notice_one` carries full scheduler reach
//! (`as_sched_ctx().is_some()`) for all five behavior dispatch sites:
//! `notice`, `AutoExpand`, `AutoShrink` (Phase-1 + Phase-3),
//! `LayoutChildren`. Before the fix all five were `false`; the
//! `AutoExpand` site additionally panicked in `cfg(not(test))` binaries
//! at `emFileLinkPanel::AutoExpand` (the original repro).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use emcore::emEngineCtx::PanelCtx;
use emcore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use emcore::emPanelTree::PanelTree;
use emcore::emView::emView;

#[derive(Default)]
struct ReachLog {
    notice: Cell<bool>,
    auto_expand: Cell<bool>,
    auto_shrink: Cell<bool>,
    layout_children: Cell<bool>,
}

struct ReachProbe(Rc<ReachLog>);

impl PanelBehavior for ReachProbe {
    fn notice(
        &mut self,
        _flags: NoticeFlags,
        _state: &PanelState,
        ctx: &mut PanelCtx,
    ) {
        self.0.notice.set(ctx.as_sched_ctx().is_some());
    }

    fn AutoExpand(&mut self, ctx: &mut PanelCtx) {
        self.0.auto_expand.set(ctx.as_sched_ctx().is_some());
    }

    fn AutoShrink(&mut self, ctx: &mut PanelCtx) {
        self.0.auto_shrink.set(ctx.as_sched_ctx().is_some());
    }

    fn LayoutChildren(&mut self, ctx: &mut PanelCtx) {
        self.0.layout_children.set(ctx.as_sched_ctx().is_some());
    }
}

#[test]
fn handle_notice_dispatch_sites_carry_full_reach() {
    let log = Rc::new(ReachLog::default());
    let mut tree = PanelTree::new();
    let root = tree.create_root_deferred_view("root");
    tree.set_behavior(root, Box::new(ReachProbe(log.clone())));

    let mut view = emView::NewForTest(&tree, root);

    let mut sched = emcore::emScheduler::EngineScheduler::new();
    let mut fw_actions: Vec<emcore::emEngineCtx::DeferredAction> = Vec::new();
    let root_ctx = emcore::emContext::emContext::NewRoot();
    let fw_cb: RefCell<Option<Box<dyn emcore::emClipboard::emClipboard>>> =
        RefCell::new(None);
    let pa: Rc<RefCell<Vec<emcore::emEngineCtx::FrameworkDeferredAction>>> =
        Rc::new(RefCell::new(Vec::new()));

    // Drive each dispatch path by setting the corresponding invalidation
    // bit on the panel and calling HandleNotice. Bits and queue_notice
    // map to the five sites at emView.rs:4033 (Phase-1 AS), 4100 (notice),
    // 4139 (AutoExpand), 4165 (Phase-3 AS), 4194 (LayoutChildren).
    tree.queue_notice(root, NoticeFlags::SOUGHT_NAME_CHANGED, None);
    if let Some(p) = tree.panels.get_mut(root) {
        p.ae_decision_invalid = true;
        p.children_layout_invalid = true;
    }
    // Force AE threshold met so AutoExpand runs.
    tree.set_seek_target(Some(root));

    view.HandleNotice(
        &mut tree,
        &mut sched,
        Some(&root_ctx),
        None,
        // Task 2 will add the next 3 args. Until then the call has
        // 4 args and the test fails because three handles are dropped.
    );

    // Force AS path: clear seek target, mark AE-invalid + ae_expanded.
    tree.set_seek_target(None);
    if let Some(p) = tree.panels.get_mut(root) {
        p.ae_invalid = true;
        p.ae_expanded = true;
    }
    view.HandleNotice(&mut tree, &mut sched, Some(&root_ctx), None);

    assert!(log.notice.get(), "notice dispatch must carry full reach");
    assert!(
        log.auto_expand.get(),
        "AutoExpand dispatch must carry full reach"
    );
    assert!(
        log.auto_shrink.get(),
        "AutoShrink dispatch must carry full reach"
    );
    assert!(
        log.layout_children.get(),
        "LayoutChildren dispatch must carry full reach"
    );

    // Suppress unused-binding warnings until Task 2 wires the handles.
    let _ = (&mut fw_actions, &fw_cb, &pa);
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo nextest run -p emcore --test notice_dispatch_reach`
Expected: FAIL — assertions on `log.notice.get()`, `log.auto_expand.get()`, `log.auto_shrink.get()`, `log.layout_children.get()` all return `false` because `as_sched_ctx()` returns `None` at all five dispatch sites.

If the test panics on `AutoExpand` instead (because `emFileLinkPanel`-style production-mode panic was added on this code path): that also counts as red. Move to Task 2.

If the test does not compile (e.g., `emView::NewForTest` does not exist or has a different signature): adapt the constructor call to whatever the existing `emView` test constructor is — read `emView` test modules for examples (e.g., `emView.rs:7921` test fixture). Do **not** invent helpers; use what is already there.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/tests/notice_dispatch_reach.rs
git commit -m "$(cat <<'EOF'
test: failing regression for notice-dispatch reach loss

Probe behavior records as_sched_ctx().is_some() in notice, AutoExpand,
AutoShrink, LayoutChildren callbacks. All four assertions fail because
emView::handle_notice_one builds the per-callback PanelCtx via
with_scheduler (1/5 reach handles set) instead of with_sched_reach.
This is the exact shape that panics at emFileLinkPanel::AutoExpand in
production binaries.

Task 2 extends HandleNotice / handle_notice_one signatures with the 3
missing handles and switches the 5 dispatch sites to with_sched_reach,
turning this test green.

Spec: docs/superpowers/specs/2026-05-02-notice-dispatch-reach-loss-design.md

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Extend signatures + fix 5 dispatch sites (TDD green)

**Files:**
- Modify: `crates/emcore/src/emView.rs` — `HandleNotice` (around line 3903) and `handle_notice_one` (around line 4007) signatures; 5 dispatch sites at lines 4033, 4100, 4139, 4165, 4194.
- Modify: `crates/emcore/tests/notice_dispatch_reach.rs` — pass the 3 new args.

This task is TDD-green for the test from Task 1. Production callers still pass the old 4-arg shape after this task; Task 3 migrates them. To keep this task's diff buildable, the 3 new params are added at the **end** of the parameter list — old call sites compile-fail loudly, which Task 3 fixes one at a time.

- [ ] **Step 1: Extend `HandleNotice` signature**

Read current at `crates/emcore/src/emView.rs:3903-3909`:

```rust
pub fn HandleNotice(
    &mut self,
    tree: &mut PanelTree,
    sched: &mut crate::emScheduler::EngineScheduler,
    root_context: Option<&Rc<crate::emContext::emContext>>,
    view_context: Option<&Rc<crate::emContext::emContext>>,
) -> bool {
```

Replace with:

```rust
pub fn HandleNotice(
    &mut self,
    tree: &mut PanelTree,
    sched: &mut crate::emScheduler::EngineScheduler,
    root_context: Option<&Rc<crate::emContext::emContext>>,
    view_context: Option<&Rc<crate::emContext::emContext>>,
    framework_actions: &mut Vec<crate::emEngineCtx::DeferredAction>,
    framework_clipboard: &RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>>,
    pending_actions: &Rc<RefCell<Vec<crate::emEngineCtx::FrameworkDeferredAction>>>,
) -> bool {
```

Inside `HandleNotice`, every call to `self.handle_notice_one(...)` must thread the 3 new args through. The function has at least one `handle_notice_one` call inside the drain loop — read the function body to find them. Each call gains `, framework_actions, framework_clipboard, pending_actions` at the end of its argument list.

- [ ] **Step 2: Extend `handle_notice_one` signature**

Read current at `crates/emcore/src/emView.rs:4007-4014`:

```rust
fn handle_notice_one(
    &mut self,
    tree: &mut PanelTree,
    id: PanelId,
    sched: &mut crate::emScheduler::EngineScheduler,
    root_context: Option<&Rc<crate::emContext::emContext>>,
    view_context: Option<&Rc<crate::emContext::emContext>>,
) {
```

Replace with:

```rust
fn handle_notice_one(
    &mut self,
    tree: &mut PanelTree,
    id: PanelId,
    sched: &mut crate::emScheduler::EngineScheduler,
    root_context: Option<&Rc<crate::emContext::emContext>>,
    view_context: Option<&Rc<crate::emContext::emContext>>,
    framework_actions: &mut Vec<crate::emEngineCtx::DeferredAction>,
    framework_clipboard: &RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>>,
    pending_actions: &Rc<RefCell<Vec<crate::emEngineCtx::FrameworkDeferredAction>>>,
) {
```

- [ ] **Step 3: Replace dispatch site at emView.rs:4033 (Phase-1 AutoShrink)**

Find the block:

```rust
if let Some(mut behavior) = tree.take_behavior(id) {
    let mut ctx = PanelCtx::with_scheduler(tree, id, pixel_tallness, sched);
    ctx.root_context = root_context;
    ctx.view_context = view_context;
    behavior.AutoShrink(&mut ctx);
    if tree.panels.contains_key(id) {
        tree.put_behavior(id, behavior);
    }
}
```

Replace with:

```rust
if let Some(mut behavior) = tree.take_behavior(id) {
    let root_ctx_some = root_context.expect("root_context must be Some when handle_notice_one runs in production");
    let mut ctx = PanelCtx::with_sched_reach(
        tree,
        id,
        pixel_tallness,
        sched,
        framework_actions,
        root_ctx_some,
        framework_clipboard,
        pending_actions,
    );
    ctx.view_context = view_context;
    behavior.AutoShrink(&mut ctx);
    if tree.panels.contains_key(id) {
        tree.put_behavior(id, behavior);
    }
}
```

Note: `with_sched_reach` requires `&Rc<emContext>` (not `Option<...>`) for `root_context`. Production callers always have `Some`; the `expect` here documents the invariant. Test-only callers that pass `None` for `root_context` will not reach this branch in well-formed test setups.

- [ ] **Step 4: Replace dispatch site at emView.rs:4100 (notice)**

Apply the same transformation as Step 3 to the `notice`-dispatch block (the one that calls `behavior.notice(flags, &state, &mut ctx)`).

```rust
if let Some(mut behavior) = tree.take_behavior(id) {
    let state = tree.build_panel_state(id, window_focused, pixel_tallness);
    let root_ctx_some = root_context.expect("root_context must be Some when handle_notice_one runs in production");
    let mut ctx = PanelCtx::with_sched_reach(
        tree,
        id,
        pixel_tallness,
        sched,
        framework_actions,
        root_ctx_some,
        framework_clipboard,
        pending_actions,
    );
    ctx.view_context = view_context;
    behavior.notice(flags, &state, &mut ctx);
    if tree.panels.contains_key(id) {
        tree.put_behavior(id, behavior);
    }
}
```

- [ ] **Step 5: Replace dispatch site at emView.rs:4139 (AutoExpand) — the panic site**

Apply the same transformation to the `AutoExpand` block.

```rust
if let Some(mut behavior) = tree.take_behavior(id) {
    let root_ctx_some = root_context.expect("root_context must be Some when handle_notice_one runs in production");
    let mut ctx = PanelCtx::with_sched_reach(
        tree,
        id,
        pixel_tallness,
        sched,
        framework_actions,
        root_ctx_some,
        framework_clipboard,
        pending_actions,
    );
    ctx.view_context = view_context;
    behavior.AutoExpand(&mut ctx);
    if tree.panels.contains_key(id) {
        tree.put_behavior(id, behavior);
    }
}
```

- [ ] **Step 6: Replace dispatch site at emView.rs:4165 (Phase-3 AutoShrink)**

```rust
if let Some(mut behavior) = tree.take_behavior(id) {
    let root_ctx_some = root_context.expect("root_context must be Some when handle_notice_one runs in production");
    let mut ctx = PanelCtx::with_sched_reach(
        tree,
        id,
        pixel_tallness,
        sched,
        framework_actions,
        root_ctx_some,
        framework_clipboard,
        pending_actions,
    );
    ctx.view_context = view_context;
    behavior.AutoShrink(&mut ctx);
    if tree.panels.contains_key(id) {
        tree.put_behavior(id, behavior);
    }
}
```

- [ ] **Step 7: Replace dispatch site at emView.rs:4194 (LayoutChildren)**

```rust
if let Some(mut behavior) = tree.take_behavior(id) {
    let root_ctx_some = root_context.expect("root_context must be Some when handle_notice_one runs in production");
    let mut ctx = PanelCtx::with_sched_reach(
        tree,
        id,
        pixel_tallness,
        sched,
        framework_actions,
        root_ctx_some,
        framework_clipboard,
        pending_actions,
    );
    ctx.view_context = view_context;
    behavior.LayoutChildren(&mut ctx);
    if tree.panels.contains_key(id) {
        tree.put_behavior(id, behavior);
    }
}
```

- [ ] **Step 8: Update the regression test to pass the new args**

In `crates/emcore/tests/notice_dispatch_reach.rs`, change both `view.HandleNotice(...)` calls from 4 args to 7 args:

```rust
view.HandleNotice(
    &mut tree,
    &mut sched,
    Some(&root_ctx),
    None,
    &mut fw_actions,
    &fw_cb,
    &pa,
);
```

Apply to both call sites in the test. Remove the trailing `let _ = (&mut fw_actions, &fw_cb, &pa);` suppress line (no longer unused).

- [ ] **Step 9: Run the test to verify it passes**

Run: `cargo nextest run -p emcore --test notice_dispatch_reach`
Expected: PASS — all four assertions return `true`.

- [ ] **Step 10: Run cargo check on the workspace to surface caller breakage**

Run: `cargo check --workspace`
Expected: failures at the production callers — `emSubViewPanel.rs:518`, `emView.rs:2646`, `emTestPanel.rs:3968` — and at the test-only callers `emPanelTree.rs:3123`, `emView.rs:7921`, `emView.rs:7933`. These are fixed in Tasks 3 and 4.

- [ ] **Step 11: Commit (workspace not buildable; emcore lib + the new test build)**

`cargo check -p emcore --lib` and `cargo nextest run -p emcore --test notice_dispatch_reach` must both succeed before committing.

```bash
git add crates/emcore/src/emView.rs crates/emcore/tests/notice_dispatch_reach.rs
git commit -m "$(cat <<'EOF'
fix: thread 5 sched-reach handles through emView::HandleNotice

emView::HandleNotice and handle_notice_one previously dropped 3 of 5
scheduler-reach handles (framework_actions, framework_clipboard,
pending_actions) at the function boundary. The 5 PanelCtx construction
sites in handle_notice_one used PanelCtx::with_scheduler (1/5 reach)
plus 2/5 manual override, leaving as_sched_ctx() returning None for
all behavior dispatches.

Effect: emFileLinkPanel::AutoExpand panicked in production (the
trigger); 4 other dispatch sites (notice, AutoShrink x2 phases,
LayoutChildren) silently degraded — callbacks using
`if let Some(sc) = ctx.as_sched_ctx()` skipped their bodies.

Fix mirrors F013 (create_control_panel_in at f8fac99a): extend the
function signatures with the 3 missing handles, replace the 5
with_scheduler+override blocks with with_sched_reach, set view_context
manually after construction.

Workspace does not yet build — production callers
(emSubViewPanel.rs:518, emView.rs:2646, emTestPanel.rs:3968) and
test-only HandleNotice callers (emPanelTree.rs:3123, emView.rs:7921,
emView.rs:7933) migrate in Tasks 3-4.

Spec: docs/superpowers/specs/2026-05-02-notice-dispatch-reach-loss-design.md

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Migrate production HandleNotice callers

**Files:**
- Modify: `crates/emcore/src/emSubViewPanel.rs:518` (production)
- Modify: `crates/emcore/src/emView.rs:2646` (internal self-call)
- Modify: `crates/emtest/src/emTestPanel.rs:3968` (now production-reachable per F019)

Each caller must split-borrow the three new handles (`framework_actions`, `framework_clipboard`, `pending_actions`) from its surrounding `EngineCtx` and pass them through.

- [ ] **Step 1: Migrate `emSubViewPanel.rs:518`**

Read the current call. The surrounding scope holds `ectx: &mut EngineCtx<'_>`. The three new fields are `ectx.framework_actions`, `ectx.framework_clipboard`, `ectx.pending_actions`.

Replace:

```rust
self.sub_view.HandleNotice(
    /* existing args */,
);
```

with the same call extended:

```rust
self.sub_view.HandleNotice(
    /* existing args */,
    ectx.framework_actions,
    ectx.framework_clipboard,
    ectx.pending_actions,
);
```

Specific arg shapes (`&mut Vec<...>` vs `&Vec<...>`, `&RefCell<...>` vs `&Rc<RefCell<...>>`) follow the field types from `EngineCtx` definition at `emEngineCtx.rs`. If a borrow conflict arises (existing borrows already aliasing these fields), use the split-borrow pattern documented at `emSubViewPanel.rs:448` (the existing `as_sched_ctx()` site).

- [ ] **Step 2: Migrate `emView.rs:2646`**

Read the current call. The surrounding scope is the `Update` entry point or similar — it holds an `EngineCtx`. Pass `ectx.framework_actions`, `ectx.framework_clipboard`, `ectx.pending_actions` after the existing args.

- [ ] **Step 3: Migrate `emTestPanel.rs:3968`**

Read the current call:

```rust
ts.with(|sc| view.HandleNotice(tree, sc.scheduler, Some(ctx), None));
```

`sc` is a `SchedCtx`. Its fields include `framework_actions`, `framework_clipboard`, `pending_actions`. Extend:

```rust
ts.with(|sc| {
    view.HandleNotice(
        tree,
        sc.scheduler,
        Some(ctx),
        None,
        sc.framework_actions,
        sc.framework_clipboard,
        sc.pending_actions,
    )
});
```

If `SchedCtx` field names differ, adapt to match — read `emEngineCtx.rs::SchedCtx` for the canonical field list.

- [ ] **Step 4: Run cargo check**

Run: `cargo check --workspace`
Expected: only the test-only callers (`emPanelTree.rs:3123`, `emView.rs:7921`, `emView.rs:7933`) and the test-fn `with_scheduler` sites still error. Production lib targets compile clean.

- [ ] **Step 5: Run nextest for the migrated crates**

Run: `cargo nextest run -p emcore -p emtest`
Expected: emcore lib tests pass (including `notice_dispatch_reach`); emtest lib tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emSubViewPanel.rs crates/emcore/src/emView.rs crates/emtest/src/emTestPanel.rs
git commit -m "$(cat <<'EOF'
fix: migrate production HandleNotice callers to 7-arg signature

emSubViewPanel::Cycle, emView's internal self-call, and emTestPanel's
HandleNotice site now thread framework_actions, framework_clipboard,
and pending_actions from their surrounding EngineCtx / SchedCtx into
HandleNotice. This unblocks the per-callback PanelCtx in
handle_notice_one from receiving full scheduler reach.

Test-only HandleNotice callers and test-fn with_scheduler sites
migrate in Task 4.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Migrate test-only callers

**Files:**
- Modify: `crates/emcore/src/emPanelTree.rs:3123` (test-only HandleNotice caller)
- Modify: `crates/emcore/src/emView.rs:7921, 7933` (test-only HandleNotice callers)
- Modify: `crates/emmain/src/emMainPanel.rs:1517, 1635, 1759, 1839` (test-fn `with_scheduler`)
- Modify: `crates/emmain/src/emAutoplayControlPanel.rs:1114` (test-fn `with_scheduler`)
- Modify: `crates/emmain/src/emMainControlPanel.rs:1357, 1508, 1633, 1698` (test-fn `with_scheduler`)
- Modify: `crates/emmain/src/emVirtualCosmos.rs:1408, 1462, 1566` (test-fn `with_scheduler`)
- Modify: `crates/emfileman/src/emDirPanel.rs:827, 882, 1978` (test-fn `with_scheduler`)

Test-only sites do not have `EngineCtx` available in scope. Each migrates per a per-site triage:

- **HandleNotice test callers** (3 sites): construct local `framework_actions` / `framework_clipboard` / `pending_actions` shims (mirror the new regression test in Task 1) and pass them through.
- **test-fn `with_scheduler` sites** (16 sites): triage into one of two outcomes:
  - **Reach wanted** — replace `PanelCtx::with_scheduler(tree, id, tallness, sched)` with `PanelCtx::with_sched_reach(tree, id, tallness, sched, &mut fw_actions, &root_ctx, &fw_cb, &pa)` and add the four supporting locals if absent.
  - **Reach not wanted** — replace with `PanelCtx::new(tree, id, tallness)` if the test does not depend on engine wakeups.

The triage rule: if the test's panel behavior calls `as_sched_ctx().expect(...)` or relies on engine wakeup, choose reach-wanted. Otherwise reach-not-wanted (`PanelCtx::new`).

- [ ] **Step 1: Migrate `emPanelTree.rs:3123`**

Read context. It currently reads:

```rust
view.HandleNotice(&mut t, &mut _dummy_sched, None, None);
```

The test does not appear to exercise sched-reach behavior. Triage: reach-not-wanted is impossible here (HandleNotice now requires the 3 args). Construct local shims:

```rust
let mut _dummy_fw: Vec<crate::emEngineCtx::DeferredAction> = Vec::new();
let _dummy_fw_cb: std::cell::RefCell<Option<Box<dyn crate::emClipboard::emClipboard>>> =
    std::cell::RefCell::new(None);
let _dummy_pa: std::rc::Rc<std::cell::RefCell<Vec<crate::emEngineCtx::FrameworkDeferredAction>>> =
    std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
view.HandleNotice(
    &mut t,
    &mut _dummy_sched,
    None,
    None,
    &mut _dummy_fw,
    &_dummy_fw_cb,
    &_dummy_pa,
);
```

- [ ] **Step 2: Migrate `emView.rs:7921` and `emView.rs:7933`**

Same pattern as Step 1. Read the surrounding test (`SwapViewPorts` shape from line 7857 onward) — `h` already provides `scheduler`. Add shims for the 3 new args alongside `h.scheduler`.

- [ ] **Step 3: Migrate emmain test-fn `with_scheduler` sites (12 sites)**

For each site listed in the file header, read the test it lives in. Decide reach-wanted vs reach-not-wanted by inspecting whether the test panel's behavior calls `as_sched_ctx().expect(...)` or relies on signal fires reaching the scheduler.

Per-site replacement:

- **Reach wanted:**
  ```rust
  let mut pctx = PanelCtx::with_sched_reach(
      &mut tree,
      root_id,
      1.0,
      unsafe { &mut *sched_ptr },
      &mut fw_actions,
      &root_ctx,
      &fw_cb,
      &pa,
  );
  ```
  (where `fw_actions`, `root_ctx`, `fw_cb`, `pa` are the test's existing locals — most emmain test-fns already have them per the B-006 click-through pattern; just thread through).

- **Reach not wanted:**
  ```rust
  let mut pctx = PanelCtx::new(&mut tree, root_id, 1.0);
  ```
  Drops the `unsafe { sched_ptr }` line if it was only used for `with_scheduler`.

If a site is ambiguous, default to **reach-wanted** — false positives on reach are silent (no harm); false negatives cause regressions when the test was relying on engine wakeup.

- [ ] **Step 4: Migrate `emfileman/src/emDirPanel.rs:827, 882, 1978`**

Per the file header these are inside test modules. Same triage as Step 3.

- [ ] **Step 5: Run cargo check on the full workspace**

Run: `cargo check --workspace --all-targets`
Expected: clean. Any remaining errors mean a `with_scheduler` site was missed — re-run `grep -rn "PanelCtx::with_scheduler" crates/` and triage.

- [ ] **Step 6: Run full nextest**

Run: `cargo nextest run --workspace`
Expected: all tests pass. If a test in emmain or emfileman fails, the per-site triage in Step 3-4 was wrong — re-inspect that test and flip reach-wanted/not-wanted.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emPanelTree.rs crates/emcore/src/emView.rs \
        crates/emmain/src/emMainPanel.rs \
        crates/emmain/src/emAutoplayControlPanel.rs \
        crates/emmain/src/emMainControlPanel.rs \
        crates/emmain/src/emVirtualCosmos.rs \
        crates/emfileman/src/emDirPanel.rs
git commit -m "$(cat <<'EOF'
test: migrate test-only callers to with_sched_reach / PanelCtx::new

3 test-only HandleNotice callers gain framework_actions /
framework_clipboard / pending_actions shims. 15 test-fn
with_scheduler+unsafe { sched_ptr } sites migrate per-site to either
with_sched_reach (reach-wanted) or PanelCtx::new (reach-not-wanted).

Workspace builds and tests pass.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Delete `PanelCtx::with_scheduler`

**Files:**
- Modify: `crates/emcore/src/emEngineCtx.rs:641-658` (delete)

After Task 4 the constructor has zero callers. Delete it; verify the audit invariant holds (grep clean); manual GUI verification.

- [ ] **Step 1: Verify zero callers remain**

Run: `grep -rn "PanelCtx::with_scheduler\b" crates/ --include="*.rs"`
Expected: no matches in production or test code (only docs and comments may remain — those are removed in this task too if they reference the constructor).

If matches remain: do not proceed; return to Task 4 and migrate them.

- [ ] **Step 2: Delete the constructor**

Read `crates/emcore/src/emEngineCtx.rs:640-658`:

```rust
/// Create a context with a scheduler so engine wakeups are propagated.
pub fn with_scheduler(
    tree: &'a mut PanelTree,
    id: PanelId,
    current_pixel_tallness: f64,
    scheduler: &'a mut EngineScheduler,
) -> Self {
    Self {
        tree,
        id,
        current_pixel_tallness,
        scheduler: Some(scheduler),
        framework_clipboard: None,
        framework_actions: None,
        root_context: None,
        view_context: None,
        pending_actions: None,
    }
}
```

Delete the entire block (including the doc-comment line above).

- [ ] **Step 3: Remove stale doc comments referencing `with_scheduler`**

Run: `grep -rn "with_scheduler" crates/ --include="*.rs"`
Expected: any remaining hits are doc-comments inside other constructors / functions (e.g., `with_clipboard` says "chain after `with_scheduler`"). Update those to reference `with_sched_reach` instead, or rephrase.

Specific known sites to inspect:
- `emEngineCtx.rs:662` — `with_clipboard` doc comment ("chain after `with_scheduler`"). Update to "chain after `with_sched_reach`".
- `emEngineCtx.rs:673` — `with_pending_actions` doc comment. Same update.
- `emfileman/src/emDirPanel.rs:787, 795` — comments referencing `with_scheduler` test pattern. Reword to describe the new structure.

- [ ] **Step 4: Final workspace verification**

Run: `cargo check --workspace --all-targets`
Expected: clean.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

Run: `cargo nextest run --workspace`
Expected: clean.

Run: `cargo xtask annotations`
Expected: clean.

- [ ] **Step 5: Manual GUI verification of the panic site**

Run: `cargo run -p eaglemode`
Drive the app: zoom into a file system panel, then into a file link entry (the original repro). The panic at `emFileLinkPanel::AutoExpand requires scheduler reach in production` must not fire.

If the panic fires: the fix is incomplete — re-run Task 1's regression test and verify it still passes; then inspect the dispatch path actually taken. The probe-test covers all 5 dispatch sites; if it passes but the GUI panics, a sixth dispatch path exists that the audit missed.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emEngineCtx.rs crates/emfileman/src/emDirPanel.rs
git commit -m "$(cat <<'EOF'
refactor: delete PanelCtx::with_scheduler

Zero callers remain after Task 4 migration. The partial-reach
constructor (1/5 sched-reach handles set) was the construction-site
footgun that produced the notice-dispatch reach loss bug. Removing it
forces future production code to choose explicitly between PanelCtx::new
(no reach) and PanelCtx::with_sched_reach (full reach).

Updates the with_clipboard / with_pending_actions doc comments to
reference with_sched_reach. Removes stale references in
emfileman/emDirPanel.rs test-pattern comments.

GUI verified: zooming into a file link no longer panics at
emFileLinkPanel::AutoExpand.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Acceptance (post-Task 5)

- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo nextest run --workspace` clean (all tests pass, including new `notice_dispatch_reach`).
- `cargo xtask annotations` clean.
- `grep -rn "PanelCtx::with_scheduler\b" crates/` returns zero hits.
- Manual: `cargo run -p eaglemode` and zoom into a file link panel — no panic.
