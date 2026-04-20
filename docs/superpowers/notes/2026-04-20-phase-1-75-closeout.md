# Phase 1.75 — Unified Scheduler Dispatch + ctx Cleanup — Closeout

**Branch:** `port-rewrite/phase-1-75`
**Commits:** `702c6dc` (bootstrap) … final closeout commits (this note + JSON sweep commits).
**Status:** COMPLETE — all C1–C11 invariants SAT; Phase 1.76 opened for `PanelBehavior::Input` scheduler threading (tracked as JSON entry E039 with status `deferred-phase-1-76`).

## Summary

Phase 1.75 closes the Tasks 2–5 deferred from Phase 1.5 PARTIAL and delivers the
first COMPLETE phase in the port-rewrite series. The per-sub-view
`EngineScheduler` (SP8) is deleted; every engine — outer and sub-tree — is
now dispatched by the single outer `EngineScheduler` via a per-engine
`TreeLocation` (Outer / SubView { outer_panel_id, rest }) walked at
`DoTimeSlice` time through `take_behavior` + `as_sub_view_panel_mut`. The
`register_pending_engines` post-slice catch-up sweep and both `try_borrow`
deferral sites at `emPanelTree.rs` are deleted; registration is now
synchronous at `create_child` time. `emPanelCtx.rs` is deleted and
`PanelCtx` absorbed into `emEngineCtx.rs`. Popup signals are allocated
inline at use sites in `emView::RawVisitAbs` (audit confirmed; no
pre-allocation block present). A new timing fixture encodes the
`delta == 0` invariant. Spec §3.3 is clarified to describe the cross-tree
dispatch mechanism (observable invariant preserved verbatim). One
deliberate residual: `PanelBehavior::Input` still builds a throwaway
`EngineScheduler` locally — threading a scheduler into that trait method
requires ~100+ impls to change and is out of Phase 1.75 scope; opened as
Phase 1.76 (JSON entry E039, status `deferred-phase-1-76`). Goldens held
237/6 identical failure set across all seven tasks; nextest 2454/0/9.

## Delta from baseline

| metric              | baseline | exit | delta |
|---------------------|---------:|-----:|------:|
| nextest passed      |     2455 | 2454 |    −1 |
| nextest failed      |        0 |    0 |     0 |
| nextest skipped     |        9 |    9 |     0 |
| goldens passed      |      237 |  237 |     0 |
| goldens failed      |        6 |    6 |     0 |
| rc_refcell_total    |      282 |  283 |    +1 |
| diverged_total      |      177 |  176 |    −1 |
| rust_only_total     |       17 |   17 |     0 |
| idiom_total         |        0 |    0 |     0 |
| try_borrow_total    |        4 |    0 |    −4 |

See `2026-04-20-phase-1-75-exit.md` for per-metric notes.

## JSON entries closed

- **E005** — sub_scheduler DIVERGED @ `a5efee6f` (Task 4 keystone)
- **E008** — register_pending_engines + try_borrow @ `eb5ed94b` (Task 5 cont)
- **E011** — popup pre-allocation @ `a7a2482c` (Task 6 audit)
- **E003, E004, E007, E010** — retroactively marked `resolved-phase-1-5` (closeout table already cited SAT; JSON status was stale).

New entry:
- **E039** — PanelBehavior::Input throwaway EngineScheduler. Status: `deferred-phase-1-76`.

## Spec sections implemented

- §3.3 (clarification; invariant preserved), §3.6, §4 D4.1 / D4.6 / D4.7 / D4.11, §10 Phase-1 invariant list (Phase-1.75 delivery annotations).

## Invariants verified

Phase 1.75 plan invariants: **all SAT** — I1, I1c, I1d, I-Y3-dispatch, I-T3a, I-T3b, I-T3c, I-Spec-3.3-clarified, Task-10, Task-11.

Phase 1 carry-forwards: **all SAT** — I1a (SchedOp eliminated), I1b (close_signal_pending / pending_sched_ops eliminated), I6 (Golden tests 237/6 preserved, NewRootWithScheduler gone).

Evidence per invariant in `2026-04-20-phase-1-75-exit.md`.

## Known residual / Phase-1.76 deferral

`PanelBehavior::Input` at `emSubViewPanel.rs:301` constructs a local
throwaway `EngineScheduler` to satisfy `SchedCtx` lifetime during mouse /
touch input dispatch. Wakes emitted inside the Input path land on the
dropped scheduler (observational no-op). Goldens held 237/6 across every
task in Phase 1.75, confirming no visible regression. Tracked as JSON entry
**E039**, status `deferred-phase-1-76`. Phase 1.76 plan will widen
`PanelBehavior::Input` (and sibling trait methods if needed) to thread a
scheduler handle; scope is ~100+ impls and is a mechanical cascade.

This residual is a deliberate, user-accepted scope decision per the Task 5
escape clause ("If that requires signature changes on `PanelBehavior::Input`
or similar observable-API changes, STOP and escalate"). It is NOT a
closeout failure; it IS a known divergence surfaced explicitly on the
status line above and in the exit metrics doc.

## Next phase

Phase 2 — view/window composition and back-ref migration. See
`docs/superpowers/plans/` for the Phase 2 plan once written. Phase 2's B4
predecessor check should accept `port-rewrite-phase-1-75-complete` as the
first COMPLETE predecessor in the chain (prior tags
`port-rewrite-phase-1-complete` and
`port-rewrite-phase-1-5-partial-complete` were the partial predecessors
Phase 1.75 completed).

Phase 1.76 — dedicated focused plan for E039 (Input throwaway elimination).
