# Phase 4e — emCoreConfig emRec Migration — Closeout

**Branch:** main (phase ran directly on main)
**Commits:** 74dd19a3..b112a845
**Status:** COMPLETE — all C1–C11 checks passed

## Summary

Phase 4e migrated `emCoreConfig` from 18 flat scalar fields to C++-faithful `emDoubleRec`/`emBoolRec`/`emIntRec` typed fields matching `emCoreConfig.h:51–89` exactly, including constructor-time bounds and defaults. `emCoreConfigPanel` was updated to write through `emRecNodeConfigModel::modify(|c, sc| c.Field.SetValue(val, sc), sched)` at all 19 write sites, and `emView` / `emViewAnimator` / `emPainter` were updated to read via `.GetValue()`. The central architectural challenge — `emContext::acquire`'s `FnOnce() -> T` closure has no `SchedCtx` parameter — was resolved with an orphaned private scheduler inside the closure, fully documented with a DIVERGED comment. All six phase-specific invariants pass, E026 and E027 are closed, and the golden test baseline is unchanged.

## Delta from baseline

| Metric | Baseline (4d exit) | Exit | Δ |
|---|---|---|---|
| nextest | 2681 | 2685 | +4 |
| goldens passed | 237 | 237 | 0 |
| goldens failed | 6 | 6 | 0 |
| rc_refcell_total | 444 | 461 | +17 |
| diverged_total | 251 | 253 | +2 |
| rust_only_total | 18 | 18 | 0 |
| idiom_total | 0 | 0 | 0 |
| try_borrow_total | 0 | 0 | 0 |

## JSON entries closed

- **E026**: 74dd19a3 — emCoreConfig emRec field migration complete; stale DIVERGED block and `VISIT_SPEED_MAX` removed.
- **E027**: c1a7cfb7 — emRef.no_rs mapping note updated; `Rc<RefCell<emConfigModel<Self>>>` reference corrected to `emRecNodeConfigModel`.

## Spec sections implemented

- `emCoreConfig.h:51–89` — all 18 typed field declarations
- `emCoreConfig.cpp:51–89` — constructor bounds and defaults
- `emCoreConfigPanel.cpp` — panel write-through pattern

## Invariants verified

- I4e-1 (VISIT_SPEED_MAX absent) — PASS
- I4e-2 (emRec fields present) — PASS
- I4e-3 (no Rc<RefCell<emConfigModel in production) — PASS
- I4e-4 (visit_speed_set_fires_signal) — PASS
- I4e-5 (emRef.no_rs updated) — PASS
- I4e-6 (Alignment drift chartered) — PASS

## Known Issues

**plugin_invocation tests require manual env setup.** `try_create_file_panel_loads_plugin` and `try_create_file_panel_missing_symbol_errors` fail under plain `cargo-nextest ntr` because the test file requires `cargo build -p test_plugin` and `LD_LIBRARY_PATH=target/debug` to be set before running. This is a pre-existing gap introduced in Phase 3 (ed6be6be) — not introduced in Phase 4e. The pre-commit hook does not set `LD_LIBRARY_PATH`, so these tests silently fail in hook context unless the developer runs the prerequisite build step. No fix landed in this phase; tracked as a known environment issue for a future cleanup pass.

## Next phase

Phase 5 — see `docs/superpowers/plans/` for the next plan file.
