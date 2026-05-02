# Notice-dispatch PanelCtx reach loss

**Date:** 2026-05-02
**Trigger:** GUI panic — `emFileLinkPanel::AutoExpand requires scheduler reach in production` (`emFileLinkPanel.rs:424`).
**Precedent:** F013 (`docs/debug/investigations/F013.md`) — same shape (`PanelCtx::new` no-reach in `create_control_panel_in`); fix threaded the five handles through and built via `with_sched_reach`.

## Invariant being audited

Every callback receiving a `PanelCtx` that may need scheduler reach either (a) gets reach, (b) panics loudly when it's missing, or (c) is annotated as reach-optional with rationale.

## Root cause

`emView::HandleNotice` and its helper `emView::handle_notice_one` accept only `sched`, `root_context`, `view_context` — three of the five handles `as_sched_ctx()` requires. `framework_actions`, `framework_clipboard`, `pending_actions` are dropped at the function boundary.

Inside `handle_notice_one`, all five behavior dispatch sites build the per-callback `PanelCtx` via `PanelCtx::with_scheduler(tree, id, pixel_tallness, sched)` then override only `root_context` and `view_context`. The remaining three handles stay `None`, so `as_sched_ctx()` returns `None` for any callback dispatched through this path.

Caller `emSubViewPanel::Cycle` (backtrace frame 19) has `EngineCtx<'_>` with all five handles — they're available, just not threaded through `HandleNotice`'s signature.

## Audit space (production behavior-dispatch construction sites)

| File:Line | Constructor | Callback dispatched | Status |
|---|---|---|---|
| `emcore/src/emView.rs:4033` | `with_scheduler` + 2/5 override | `AutoShrink` (Phase-1, `ae_invalid && ae_expanded`) | broken: silent partial-reach |
| `emcore/src/emView.rs:4100` | `with_scheduler` + 2/5 override | `notice` | broken: silent partial-reach |
| `emcore/src/emView.rs:4139` | `with_scheduler` + 2/5 override | `AutoExpand` | broken: **PANIC** at `emFileLinkPanel.rs:424` |
| `emcore/src/emView.rs:4165` | `with_scheduler` + 2/5 override | `AutoShrink` (Phase-3) | broken: silent partial-reach |
| `emcore/src/emView.rs:4194` | `with_scheduler` + 2/5 override | `LayoutChildren` | broken: silent partial-reach |

All five sites share the same defect; only one panicked because only `emFileLinkPanel::AutoExpand` has `#[cfg(not(test))] panic!` on the no-reach branch (`emFileLinkPanel.rs:418-424`).

## Why the other four are silent (latent bug)

Most callback bodies use `if let Some(mut sc) = ctx.as_sched_ctx() { ... }` (D-007 pattern) — a no-reach `PanelCtx` skips the body silently. Examples sampled during enumeration: `emColorField`, `emCheckBox`, `emCheckButton`, `emRadioButton`, `emButton`, `emStocksFilePanel`, `emFileLinkPanel::Cycle` itself. These would never panic — they'd silently drop signal fires, deferred actions, and clipboard operations during notice/AE/AS/LayoutChildren dispatch.

This matches the D-009 "polling intermediaries" anti-shape but at the construction layer rather than the use layer: **the audit failures are in the construction site, not the use site.** The use-site D-007 audit (FU-004) would not have caught these because the use sites are correctly written — they're just being fed a broken context.

## Out of audit space (verified clean by static enumeration)

- `emPanelCycleEngine::Cycle` (lines 112, 192) — uses `with_sched_reach`. Correct.
- `emWindow.rs:1191` — `with_sched_reach`. Correct.
- `emPanelTree::create_control_panel_in` (line 2213) — `with_sched_reach` after F013 fix. Correct.
- All `emcore/src/em*.rs:N` test-module `with_scheduler` calls — test-only.
- `emmain/src/emMainPanel.rs`, `emMainControlPanel.rs`, `emVirtualCosmos.rs`, `emAutoplayControlPanel.rs` `with_scheduler(... unsafe { &mut *sched_ptr })` calls — test-fn scaffolding (B-006 click-through pattern).

## Remediation pattern (from F013)

1. Extend `emView::HandleNotice` signature to accept `framework_actions`, `framework_clipboard`, `pending_actions` alongside the existing three.
2. Extend `emView::handle_notice_one` to thread the same.
3. Replace the 5 `PanelCtx::with_scheduler(...) + 2/5 override` blocks with `PanelCtx::with_sched_reach(... five handles ..., view_context_set_after)` (or an equivalent that sets all six fields including `view_context`).
4. Update `emSubViewPanel::Cycle` (backtrace frame 19) and any other `HandleNotice` callers to split-borrow the three new handles from `EngineCtx` and pass them through.
5. Regression test: probe behavior whose `notice`/`AutoExpand`/`AutoShrink`/`LayoutChildren` records `ctx.as_sched_ctx().is_some()`. Asserts `true` after fix; would assert `false` before (would also panic if the AutoExpand variant panics).

C++ parity: every emPanel implicitly carries its emContext (with scheduler, framework actions, clipboard); Rust conveys the same reach through five explicit handles. Notice dispatch is in the C++ "always has full context" regime — the partial-reach Rust constructor at this layer is a fidelity bug, not a forced divergence.

## Caller chain reference (from panic backtrace)

```
winit ApplicationHandler::about_to_wait
  → EngineScheduler::DoTimeSlice
    → PanelCycleEngine::Cycle (has full sched reach via with_sched_reach)
      → emSubViewPanel::Cycle (has full sched reach via EngineCtx)
        → emView::HandleNotice (drops 3 of 5 handles ← BUG)
          → emView::handle_notice_one (drops 3 of 5 handles ← BUG)
            → PanelCtx::with_scheduler (only 1/5 reach handle set)
              + override root_context, view_context (now 3/5)
              → behavior.AutoExpand(&mut ctx)
                → emFileLinkPanel::AutoExpand
                  → ctx.as_sched_ctx() returns None (needs 5/5)
                    → panic! (the observed bug)
```

## Recommendation

Fix all 5 emView dispatch sites in one pass; this is the F013 pattern at a different construction site. Implementation phase should:

- Brainstorm a spec covering the signature change + caller migration.
- Plan should mirror F013's three-step structure: extend `HandleNotice`/`handle_notice_one` signatures; update dispatch construction sites; update callers.
- Regression test at the framework level (probe behavior with notice/AE/AS/LayoutChildren callbacks asserting `as_sched_ctx().is_some()`).
- Verify the panic site (`emFileLinkPanel::AutoExpand`) no longer fires by running the binary and zooming into a file link.

The four silent-degradation sites (notice, AS phases 1+3, LayoutChildren) are latent bugs of the same shape; the fix lifts all five together because they share the construction code.
