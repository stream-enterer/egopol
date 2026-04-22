# Phase 4c — emRec Listener Tree + Structural Compounds — Closeout

**Branch:** port-rewrite/phase-4c
**Commits:** 30ef331f..27883ca2
**Status:** COMPLETE — all C1–C11 checks passed

## Summary

Phase 4c shipped the reified-chain listener-tree mechanism (per ADR
`2026-04-21-phase-4b-listener-tree-adr.md`, choice R5): every primitive emRec
now carries an `aggregate_signals: Vec<SignalId>` and fires every ancestor's
aggregate signal on mutation, eliminating the C++ parent-pointer virtual
`ChildChanged` walk. On top of that rep, Phase 4c ported the four structural
compounds `emStructRec`, `emUnionRec`, `emArrayRec`, `emTArrayRec<T>` (each
splicing `register_aggregate` across its subtree on `AddMember` / `SetVariant`
/ `SetCount`) plus `emRecListener`, which owns an engine and observes a single
`SignalId` with `SetListenedRec(Option<&R>)` re-targeting. End-to-end
composition tests (Rust analogue of the C++ `Person` example + array of
`Person`, multi-level nesting, union retargeting, listener detach) all pass
and confirm aggregate signals fire exactly once on any leaf mutation at any
depth. Gate: 2613/2613 nextest, 237/243 goldens (same 6 pre-existing failures
carried forward unchanged), clippy/fmt clean, `try_borrow_total` = 0,
`rc_refcell_total` decreased 351 → 290, no new `unsafe` blocks.

## Delta from baseline

| Metric              | Baseline | Exit  | Delta      |
|---------------------|----------|-------|------------|
| nextest tests       | 2562     | 2613  | +51        |
| nextest failed      | 0        | 0     | 0          |
| nextest skipped     | 9        | 9     | 0          |
| golden passed       | 237      | 237   | 0          |
| golden failed       | 6        | 6     | 0          |
| clippy warnings     | 0        | 0     | 0          |
| cargo fmt           | clean    | clean | –          |
| rc_refcell_total    | 351      | 290   | −61        |
| try_borrow_total    | 0        | 0     | 0          |
| new unsafe blocks   | —        | 0     | 0          |

## JSON entries closed

None — Phase 4c closes no entries (E026/E027 land at Phase 4e; persistence at
Phase 4d).

## Spec sections implemented

- §7 D7.1 (continued) — emRec listener tree (ADR R5 rep) + structural
  compounds (`emStructRec`, `emUnionRec`, `emArrayRec`, `emTArrayRec<T>`) +
  `emRecListener`.

## Invariants verified

All from Task 7 gate report (`2026-04-19-phase-4c-gate-report.md`):

- **I4c-1 PASS** — all 8 primitives carry `aggregate_signals: Vec<SignalId>`;
  each `SetValue` fires the full chain.
- **I4c-2 PASS** — `register_aggregate(&mut self, sig: SignalId)` on
  `emRecNode` trait, implemented on all 12 concrete rec types (8 primitives + 4
  compounds).
- **I4c-3 PASS** — `emRecListener.rs`, `emStructRec.rs`, `emUnionRec.rs`,
  `emArrayRec.rs`, `emTArrayRec.rs` all present with required surface.
- **I4c-4 PASS** — Person composition + `person_array_listener_fires_on_nested_mutation`
  integration tests green.
- **I4c-5 PASS** — `PersonWithAddr` multi-level deep-leaf mutation test green.
- **I4c-6 PASS** — `emUnionRec::SetVariant` splice fires aggregate once; stale
  listener on old child inert; same-tag no-op verified.
- **I4c-7 PASS** — `SetListenedRec(None)` detaches cleanly;
  `SetListenedRec(Some(other))` re-targets.
- **I4c-8 PASS** — `try_borrow_total = 0`; `rc_refcell_total` decreased
  (351 → 290); 0 new `unsafe` blocks in the phase diff.
- **I4c-9 PASS** — goldens 237 pass / 6 fail identical to baseline.
- **I4c-10 PASS** — `// DIVERGED:` comments in place on all 8 primitives'
  `SetValue` loops and on each compound's `register_aggregate` splice.

## Next phase

Phase 4d — see
`docs/superpowers/plans/2026-04-19-port-rewrite-phase-4d-emrec-persistence.md`.
