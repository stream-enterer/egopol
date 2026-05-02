# Notice-dispatch PanelCtx reach loss — F013-mirror fix

**Date:** 2026-05-02
**Trigger:** GUI panic — `emFileLinkPanel::AutoExpand requires scheduler reach in production` (`emFileLinkPanel.rs:424`).
**Investigation:** `docs/debug/investigations/notice-dispatch-reach-loss.md`
**Precedent:** F013 (`docs/debug/investigations/F013.md`) — same shape (`PanelCtx::new` partial-reach in `create_control_panel_in`); fix threaded the five handles through and built via `with_sched_reach`.

## 1. Purpose

Close the construction-site reach loss in `emView::HandleNotice` / `handle_notice_one`. Three of the five scheduler-reach handles (`framework_actions`, `framework_clipboard`, `pending_actions`) are dropped at the function boundary, so all five behavior dispatch sites build a partial-reach `PanelCtx`. One callback (`emFileLinkPanel::AutoExpand`) panics on the no-reach branch; four others silently degrade.

Mirror F013: thread the missing handles through, replace `PanelCtx::with_scheduler + 2/5 override` with `PanelCtx::with_sched_reach + view_context override`, delete `PanelCtx::with_scheduler` after migration.

## 2. Background

`PanelCtx::as_sched_ctx()` requires five handles (`scheduler`, `framework_actions`, `root_context`, `framework_clipboard`, `pending_actions`) — any `None` returns `None`. The five `PanelCtx` constructors set:

| Constructor | scheduler | framework_actions | root_context | framework_clipboard | pending_actions |
|---|---|---|---|---|---|
| `PanelCtx::new` | None | None | None | None | None |
| `PanelCtx::with_scheduler` | Some | None | None | None | None |
| `PanelCtx::with_sched_reach` | Some | Some | Some | Some | Some |

`handle_notice_one` uses `with_scheduler` + manual `ctx.root_context = ...` + `ctx.view_context = ...` overrides. Three handles stay `None`; `as_sched_ctx()` returns `None`.

F013 fixed the analogous bug in `PanelTree::create_control_panel_in` by extending the function signature with the missing handles and switching to `with_sched_reach`. This spec applies the same shape to `HandleNotice` / `handle_notice_one`.

## 3. Architecture

Three coordinated signature changes in `crates/emcore/src/emView.rs`:

1. `pub fn HandleNotice(&mut self, tree, sched, root_context, view_context, framework_actions, framework_clipboard, pending_actions) -> bool` — adds 3 handle params after the existing 4.
2. `fn handle_notice_one(&mut self, tree, id, sched, root_context, view_context, framework_actions, framework_clipboard, pending_actions)` — same 3 params threaded through.
3. The 5 `PanelCtx::with_scheduler(...) + 2/5 override` blocks at lines 4033, 4100, 4139, 4165, 4194 collapse to `PanelCtx::with_sched_reach(...)` plus `ctx.view_context = view_context;` (since `with_sched_reach` does not currently take `view_context`; the field stays manually set).

`PanelCtx::with_scheduler` deleted from `emEngineCtx.rs` after all callers migrate.

## 4. Components touched

| File | Change |
|---|---|
| `crates/emcore/src/emView.rs` | `HandleNotice` and `handle_notice_one` signatures gain 3 params. 5 dispatch sites at lines 4033, 4100, 4139, 4165, 4194 replace `with_scheduler + 2/5 override` with `with_sched_reach + view_context override`. |
| `crates/emcore/src/emEngineCtx.rs` | Delete `PanelCtx::with_scheduler` (lines 641-658). |
| `crates/emcore/src/emSubViewPanel.rs:518` | Split-borrow 3 new handles from the surrounding `EngineCtx` and pass them through. |
| `crates/emcore/src/emView.rs:2646` | Internal self-call — plumb the 3 handles from the surrounding entry point's `EngineCtx`. |
| `crates/emtest/src/emTestPanel.rs:3968` | Plumb the 3 handles from the test-shim's available state. |
| `crates/emmain/src/emMainPanel.rs` (4 sites), `emAutoplayControlPanel.rs` (1), `emMainControlPanel.rs` (4), `emVirtualCosmos.rs` (3) | 12 test-fn `with_scheduler(... unsafe { sched_ptr })` sites migrate to `with_sched_reach` (if reach is wanted) or `PanelCtx::new` (if not). Triage per site. |
| `crates/emfileman/src/emDirPanel.rs` (3 sites) | Test-module `with_scheduler` sites migrate similarly. |
| `crates/emcore/src/emPanelTree.rs:3123`, `emView.rs:7921, 7933` | Test-only `HandleNotice` callers pass test-shim defaults for the new params. |

## 5. Data flow

**Before:**

```
emSubViewPanel::Cycle (EngineCtx with all 5 handles)
  → emView::HandleNotice(tree, sched, root_context, view_context)
    [3 handles dropped]
    → handle_notice_one(... same 3 dropped ...)
      → PanelCtx::with_scheduler(tree, id, tallness, sched)  // 1/5
        + ctx.root_context = root_context                    // 2/5
        + ctx.view_context = view_context                    // (separate)
        → behavior.AutoExpand(&mut ctx)
          → ctx.as_sched_ctx() → None
            → panic! (or silently skips body)
```

**After:**

```
emSubViewPanel::Cycle (EngineCtx with all 5 handles)
  → emView::HandleNotice(tree, sched, root_context, view_context,
                          framework_actions, framework_clipboard, pending_actions)
    → handle_notice_one(... all 5 threaded ...)
      → PanelCtx::with_sched_reach(tree, id, tallness, sched,
                                    framework_actions, root_context,
                                    framework_clipboard, pending_actions)
        + ctx.view_context = view_context
        → behavior.AutoExpand(&mut ctx)
          → ctx.as_sched_ctx() → Some(SchedCtx { ... })
            → AutoExpand body runs with full reach
```

## 6. Testing

**Unit tests added:** new file `crates/emcore/tests/notice_dispatch_reach.rs`. Probe behavior records `ctx.as_sched_ctx().is_some()` in each of `notice`, `AutoExpand`, `AutoShrink`, `LayoutChildren`. Test drives the panel through:

1. Notice fire → asserts `notice` saw reach.
2. AE-invalid + threshold-met → asserts `AutoExpand` saw reach.
3. AE-invalid + threshold-not-met (after expansion) → asserts `AutoShrink` saw reach.
4. Children-layout-invalid → asserts `LayoutChildren` saw reach.
5. Phase-1 AE-invalid-shrink site (line 4033) → asserts that path also saw reach.

All five booleans must be `true`. Before the fix all five would be `false`; the `AutoExpand` step would also panic in `cfg(not(test))` binaries.

**Tests updated:** test-only `HandleNotice` callers in `emPanelTree.rs:3123`, `emView.rs:7921`, `emView.rs:7933` adopt the new signature. Test-fn `with_scheduler` sites in `emmain/*` and `emfileman/src/emDirPanel.rs` migrate per-site (reach-wanted → `with_sched_reach`; reach-not-wanted → `PanelCtx::new`).

## 7. Acceptance

- `cargo-nextest ntr` clean.
- `cargo clippy -- -D warnings` clean.
- `cargo xtask annotations` clean.
- `cargo run -p eaglemode` — manual: zoom into a file link panel; no panic.
- New regression test passes.
- `grep -n "PanelCtx::with_scheduler" crates/` returns zero hits.

## 8. Risks

- **Caller migration surface area.** ~20 call sites across 8 files; mostly mechanical but each `unsafe { sched_ptr }` test-fn site needs human triage on whether the test wants reach. Mitigation: per-site decision with the constructor names self-documenting which choice was made.
- **`emtest::emTestPanel` plumbing.** The test shim may not have all 3 handles available. Two outcomes acceptable at implementation time: (a) shim synthesizes defaults, (b) spec narrows to "production-only fix; emtest stays on a sibling helper until a follow-up." Decision deferred to implementation; either is consistent with the audit invariant.
- **Test-fn `unsafe { sched_ptr }` aliasing.** The 12 emmain test-fn sites use `unsafe { &mut *sched_ptr }` to alias the scheduler with a sibling `EngineCtx`. After migration to `with_sched_reach`, the same aliasing pattern continues for those that still want reach. Not a new risk; preserved unchanged.

## 9. Out of scope

- Sweeping `PanelCtx::new` call sites elsewhere in production. Static enumeration in the investigation verified none flow through behavior dispatch with reach-required callbacks.
- Changing F013-fixed `create_control_panel_in`. Already correct.
- D-007 use-side signal-fire patterns. Unaffected by this construction-layer fix.
- Adding `view_context` as a `with_sched_reach` parameter. Current signature has 7 params; adding an 8th is over the `clippy::too_many_arguments` threshold (already suppressed) and offers no observable benefit since the caller sets the field one line later anyway.
