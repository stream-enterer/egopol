# Phase 4c — Task 7 Gate Report

**Captured:** 2026-04-22
**Branch:** port-rewrite/phase-4c
**Tip at verification:** 973f181a (post Task 6 + fixup)
**Baseline:** `2026-04-19-phase-4c-baseline.md`

## Exit metrics

| Metric              | Baseline | Exit  | Delta      | Note                                           |
|---------------------|----------|-------|------------|------------------------------------------------|
| nextest tests       | 2562     | 2613  | +51        | +51 tests added across Tasks 1–6 (all pass)    |
| nextest failed      | 0        | 0     | 0          |                                                |
| nextest skipped     | 9        | 9     | 0          |                                                |
| golden passed       | 237      | 237   | 0          |                                                |
| golden failed       | 6        | 6     | 0          | Same 6 pre-existing failures (see list below)  |
| clippy              | 0 warn   | 0 warn| 0          | `--all-targets --all-features -D warnings`     |
| cargo fmt --check   | clean    | clean | –          | No output (clean)                              |
| rc_refcell_total    | 351      | 290   | −61        | Decreased → I4c-8 constraint "does not increase" |
| try_borrow_total    | 0        | 0     | 0          | Still zero                                     |
| new unsafe blocks   | —        | 0     | 0          | `git diff main..HEAD` has 0 new `unsafe {` openings (three textual matches are in docstrings explicitly documenting the *avoidance* of `unsafe`). |

Pre-existing golden failures carried forward unchanged:
- `composition::composition_tktest_1x`
- `composition::composition_tktest_2x`
- `notice::notice_window_resize`
- `test_panel::testpanel_expanded`
- `test_panel::testpanel_root`
- `widget::widget_file_selection_box`

## Per-invariant verdicts

- **I4c-1 PASS** — All 8 primitives (emBoolRec, emIntRec, emDoubleRec, emEnumRec, emStringRec, emFlagsRec, emColorRec, emAlignmentRec) carry `aggregate_signals: Vec<SignalId>` (12 file matches including compounds that also hold the vec). Every primitive's `SetValue` has the `for sig in &self.aggregate_signals { sched.fire(*sig) }` loop confirmed (8/8 present, ADR-cited).
- **I4c-2 PASS** — `fn register_aggregate(&mut self, sig: SignalId)` declared on `emRecNode` trait (emRecNode.rs) and implemented on all 12 concrete rec types (8 primitives + 4 compounds).
- **I4c-3 PASS** — `emRecListener.rs`, `emStructRec.rs`, `emUnionRec.rs`, `emArrayRec.rs`, `emTArrayRec.rs` all present with required surface (see ledger Tasks 2–5).
- **I4c-4 PASS** — Task 3 composition test (Person with 3 primitives) + Task 6 integration test `person_array_listener_fires_on_nested_mutation` (all three primitive field mutations verified after the Task 6 fixup added the `male` mutation path).
- **I4c-5 PASS** — Task 3 `PersonWithAddr` multi-level test (root listener fires on deep-leaf zip mutation via nested `Address` struct).
- **I4c-6 PASS** — Task 4 SpyRec splice-target-new-instance test proves `SetVariant` fires the aggregate exactly once on tag change and that the old child instance is dropped (new register_aggregate targets fresh instance; stale listener inert). Same-tag no-op verified.
- **I4c-7 PASS** — `emRecListener::SetListenedRec(None)` detaches cleanly (detached listener does not fire on subsequent mutations); `SetListenedRec(Some(other))` re-targets. Tests added under Task 2 + Task 2 fixup.
- **I4c-8 PASS** — `try_borrow_total` = 0 (unchanged). `rc_refcell_total` = 290 (decreased from 351). 0 new `unsafe` blocks in the phase-4c diff.
- **I4c-9 PASS** — Golden suite: 237 passed, 6 failed — identical pass/fail partition to baseline. No new regressions.
- **I4c-10 PASS** — DIVERGED comments verified:
  - Per-primitive `SetValue` loop (8/8): `// DIVERGED: ... ADR 2026-04-21-phase-4b-listener-tree-adr.md` above the loop.
  - `emStructRec::AddMember`, `emUnionRec::SetVariant`, `emArrayRec::SetCount`, `emTArrayRec::SetCount`: DIVERGED comment citing `emRec::Changed()` (emRec.h:243 inline / emRec.cpp:217 ChildChanged) near the `register_aggregate` splice.

## Conclusion

All I4c invariants verified. No fixups required. Gate report committed at tip.
