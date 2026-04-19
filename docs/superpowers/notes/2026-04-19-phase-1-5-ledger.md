# Phase 1.5 — Keystone Migration — Ledger

**Started:** 2026-04-19 16:53 local
**Branch:** port-rewrite/phase-1-5
**Baseline:** see `2026-04-19-phase-1-5-baseline.md`
**Spec sections:** §2 P1/P6, §3.1, §3.3, §3.7, §4 D4.1–D4.11
**JSON entries to close:** E002, E003, E004, E005, E007, E008, E009, E010, E011

## Predecessor context (B4 sanctioned-PARTIAL)

- Phase 1 closeout: `2026-04-19-phase-1-closeout.md` status line
  `PARTIAL — Chunks 1+2 complete; Chunks 3+4 (keystone) deferred to Phase 1.5`.
- Tag: `port-rewrite-phase-1-partial-complete`.
- Nine JSON entries carry `status: carry-forward-phase-1.5` from Phase 1:
  E002, E003, E004, E005, E007, E008, E009, E010, E011.
- Plan `2026-04-19-port-rewrite-phase-1-5-keystone-migration.md` Entry
  precondition explicitly accepts the PARTIAL status; B4 deviation
  recorded here, no halt.

## Carry-ins from Phase 1

These items were identified in Phase 1 / the silent-drift audit
(85946f1 "close silent-drift workarounds") and are scheduled to be
addressed inside Phase 1.5 tasks. Recorded here so they are not
dropped silently.

- **W1 (pending_inputs field restoration).** Chunk 2 speculatively
  deleted `App.pending_inputs: Vec<(WindowId, emInputEvent)>`
  (spec §3.1 + §4 D4.9 mandate the field). Task 1 step 1g restores
  it. Invariant I-P15-pending-inputs witnesses closure
  (`rg -n 'pub(crate)?\s+pending_inputs' crates/emcore/src/emGUIFramework.rs`
  returns ≥ 1).

- **W3 (engine_id reconciliation / spec §3.1.1).** Silent-drift commit
  85946f1 reconciled spec §3.1.1 to the engine_id shape. No Phase 1.5
  code change required beyond consuming the reconciled spec; record that
  Task 1's ctx threading follows §3.1.1 as reconciled, not the pre-drift
  language. Phase 2 owns any further engine_id work.

- **W4 regression guard (framework_actions workaround not to be
  reintroduced).** Chunk 1's scheduler-owned framework_actions workaround
  was removed in Chunk 2 @ 0e68a1f. Invariant I-P15-W4-regression
  witnesses: `rg 'mem::take.*framework_actions|drain_framework_actions'
  crates/emcore/src/emScheduler.rs` returns zero. Closeout C4 re-runs.

- **Pending-inputs consumer (Phase 3).** Field restored here is consumed
  by `InputDispatchEngine` in Phase 3. Phase 1.5 expects a single
  `dead_code` carry-forward warning on the field; plan sanctions
  `--no-verify` on the final Phase 1.5 commit if clippy warns, with a
  note here. Re-verify removal when Phase 3 lands.

## Task log

<empty — tasks append here as they complete>
