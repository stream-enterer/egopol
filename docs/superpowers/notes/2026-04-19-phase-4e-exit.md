# Phase 4e — emCoreConfig emRec Migration — Exit Metrics

**Captured:** 2026-04-22
**Commits:** 74dd19a3..b112a845

## Gate

- `cargo fmt --check` — clean (b112a845 applied fmt fixes from prior session before capture)
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo-nextest ntr` — 2685 passed, 9 skipped, 0 failed (requires `LD_LIBRARY_PATH=target/debug cargo build -p test_plugin` — see Known Issues)
- `cargo test --test golden -- --test-threads=1` — 237 passed, 6 failed (baseline; no regressions)

## Counts

- nextest: 2685
- goldens passed: 237
- goldens failed: 6
- rc_refcell_total: 461
- diverged_total: 253
- rust_only_total: 18
- idiom_total: 0
- try_borrow_total: 0

## Delta from baseline (Phase 4d exit)

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

**nextest Δ +4**: `visit_speed_set_fires_signal` (field signal + aggregate signal assertions) plus updated behavioral tests in `crates/eaglemode/tests/behavioral/core_config.rs`.

**rc_refcell_total Δ +17**: emCoreConfig::Acquire creates a private scheduler context inside the closure (the "orphaned scheduler" pattern); the SchedCtx frame carries `Rc<RefCell<...>>` references from emScheduler internals. This is the same pattern as emRecNodeConfigModel — no new Rc<RefCell> in the config values themselves.

**diverged_total Δ +2**: one DIVERGED annotation on `emCoreConfig::inner` (C++ multiple inheritance → Rust field composition) and one on `emTiling::Alignment` (I4e-6, alignment drift chartered).

**goldens** unchanged — Phase 4e touches no rendering pipeline.

## Known Issues

**plugin_invocation tests require manual env setup.** `try_create_file_panel_loads_plugin` and `try_create_file_panel_missing_symbol_errors` fail under plain `cargo-nextest ntr` because `libtest_plugin.so` is not on the default library path. The test file documents this: `cargo build -p test_plugin && LD_LIBRARY_PATH=target/debug cargo-nextest ntr`. This is a pre-existing gap (tests added in Phase 3, ed6be6be) — not introduced in Phase 4e. The pre-commit hook does not set LD_LIBRARY_PATH, so these tests silently fail in hook context. Tracked as a known environment issue; no fix landed in this phase.

## JSON entries closed

- **E026**: 74dd19a3 — `emCoreConfig` now uses `emDoubleRec`/`emBoolRec`/`emIntRec` typed fields matching C++ `emCoreConfig.h:51–89`. The stale DIVERGED block at ~lines 237–252 and `VISIT_SPEED_MAX` are removed. Phases 4a–4e together complete the emRec infrastructure build-out (emRec types → persistence → emCoreConfig migration).
- **E027**: c1a7cfb7 — `emRef.no_rs` updated: `emRef<T> → Rc<T>` default with chartered exceptions (a) winit/wgpu cross-closure refs, (b) context-registry singletons, (c) shared mutable sibling state. Stale `Rc<RefCell<emConfigModel<Self>>>` reference replaced by `Rc<RefCell<emRecNodeConfigModel<Self>>>`.

## Invariants (C4)

- **I4e-1** — `VISIT_SPEED_MAX` and `VisitSpeed_GetMaxValue` absent; stale DIVERGED block removed — **PASS** (`rg 'VISIT_SPEED_MAX|VisitSpeed_GetMaxValue' crates/emcore/src/` → no matches)
- **I4e-2** — emRec typed fields present in `emCoreConfig.rs` — **PASS** (`grep 'VisitSpeed:.*emDoubleRec'` → match)
- **I4e-3** — zero `Rc<RefCell<emConfigModel` in production code — **PASS** (`rg 'Rc<RefCell<emConfigModel' crates/ --glob '!*/tests/*'` → no matches)
- **I4e-4** — `visit_speed_set_fires_signal` passes, asserting both field signal and aggregate signal fire on `SetValue` — **PASS**
- **I4e-5** — `emRef.no_rs` mapping note updated with `emRef<T> → Rc<T>` default + chartered exceptions — **PASS**
- **I4e-6** — `emTiling::Alignment` DIVERGED annotation records C++ u8 bitmask vs Rust single-axis enum mismatch — **PASS**
