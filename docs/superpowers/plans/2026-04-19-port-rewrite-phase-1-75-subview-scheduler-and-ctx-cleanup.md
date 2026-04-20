# Phase 1.75 — Unified Scheduler Dispatch + ctx Cleanup (Phase-1.5 deferreds)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. This plan closes the Tasks 2–5 deferred from Phase 1.5 PARTIAL by **unifying engine dispatch across outer and sub-view trees** — preserving the C++ observable invariant that all engines share one priority queue (§3.3). An earlier draft (Option G) proposed chartering `sub_scheduler` and weakening §3.3; that direction was rejected as drift. See `notes/2026-04-20-phase-1-75-brainstorm.md` for the option analysis.

**Goal.** Eliminate `emSubViewPanel::sub_scheduler` outright. All engines — outer and sub-tree — register with the single outer `EngineScheduler`, which dispatches them via a per-engine `TreeLocation` that tells the dispatcher how to reach the engine's tree. Plus: thread ctx through `register_engine_for`, delete `register_pending_engines`, delete `emPanelCtx.rs`, inline popup signals, timing fixtures `delta==0`.

Goal stated as invariants:

- **I1 (full).** `rg 'Rc<RefCell<EngineScheduler>>' crates/` returns zero matches.
- **I1c (full, original wording from spec).** `rg -w 'sub_scheduler' crates/` returns zero matches. No per-sub-view scheduler exists anywhere.
- **I1d (full).** `rg 'try_borrow(_mut)?\(\)' crates/emcore/src/emView.rs crates/emcore/src/emPanelTree.rs crates/emcore/src/emSubViewPanel.rs` returns zero matches.
- **I-Y3-dispatch.** `EngineScheduler::DoTimeSlice` walks a single priority queue containing all engines (outer + every sub-view's); outer priority-P and sub-view priority-P engines fire interleaved within one slice, preserving C++ §3.3 observability.
- **I-T3a.** `rg -w 'register_pending_engines' crates/` returns zero matches.
- **I-T3b.** `test -e crates/emcore/src/emPanelCtx.rs` returns non-zero (deleted). `rg 'pub mod emPanelCtx' crates/emcore/src/lib.rs` returns zero.
- **I-T3c.** `rg 'emPanelCtx::' crates/` returns zero matches outside git-history noise.
- **Task-10.** No pre-allocated popup signals — `ctx.create_signal()` inline at the 4 popup-signal use sites (spec §4 D4.7).
- **Task-11.** `sp4_5_fix_1_timing_panel_reinit_baseline_slices.rs`, `_sched_drain_baseline_slices.rs`, `_subview_reinit_baseline_slices.rs` all assert `delta == 0` (spec §4 D4.6 / D4.11).
- **I-Spec-3.3-clarified.** Spec §3.3 updated to *describe* the cross-tree dispatch mechanism (the spec is currently silent on how `PanelCycleEngine` routes to sub-tree panels; this clarification makes the implementation knowable from the spec). §3.3's "shared scheduler" observable invariant is **preserved**, not weakened.

**Tech stack:** unchanged.

**Architecture.**

```
EngineScheduler
├── engines: SlotMap<EngineId, Box<dyn emEngine>>           (unchanged)
├── engine_locations: SecondaryMap<EngineId, TreeLocation>  (NEW — Phase 1.75)
├── wake_queues[priority*2+parity]: Vec<EngineId>           (unchanged; one queue per priority, cross-tree)
└── DoTimeSlice(outer_tree, ...):                            (MODIFIED)
      for each awake engine E (priority order):
          resolve ctx.tree = walk(outer_tree, engine_locations[E])
          dispatch E.Cycle(ctx)

enum TreeLocation {
    Outer,
    SubView { outer_panel_id: PanelId, rest: Box<TreeLocation> },
}

trait PanelBehavior {
    fn as_sub_view_panel_mut(&mut self) -> Option<&mut emSubViewPanel> { None }
    // ... unchanged methods
}
impl PanelBehavior for emSubViewPanel {
    fn as_sub_view_panel_mut(&mut self) -> Option<&mut emSubViewPanel> { Some(self) }
    // ...
}
```

The `as_sub_view_panel_mut` trait method with `None` default is the only per-trait surface change — no `Any`, no downcasting, no boilerplate on the ~50 non-sub-view `PanelBehavior` impls.

**Cross-tree dispatch walk** (called by `DoTimeSlice` per engine):

```rust
fn resolve<'t>(outer: &'t mut PanelTree, loc: &TreeLocation) -> ResolvedTree<'t> {
    match loc {
        TreeLocation::Outer => ResolvedTree::Direct(outer),
        TreeLocation::SubView { outer_panel_id, rest } => {
            // take_behavior mid-walk; put back on return path
            let taken = outer.take_behavior(*outer_panel_id)?;
            ResolvedTree::Nested { outer, panel_id: *outer_panel_id, taken, inner: /* walk rest via taken.as_sub_view_panel_mut().sub_tree */ }
        }
    }
}
```

The dispatch takes/puts the owner `emSubViewPanel`'s behavior around each sub-tree-engine call. Cost: one behavior-slot swap per sub-tree engine dispatch (matches the existing take/put cost profile for outer panels).

**Why sub_tree stays on `emSubViewPanel` (vs. side-slot on `PanelData`).** Keeps `emSubViewPanel` self-contained (its fields — sub_view, animator, sub_tree — all belong together). The `take_behavior`-with-downcast path is not a new pattern; it's how `PanelCycleEngine::Cycle` already reaches sub-view state during sub-dispatch today (via `sub_scheduler.borrow_mut().DoTimeSlice(&mut self.sub_tree, ...)`). Phase 1.75 just moves the reach from `self.sub_scheduler` (which goes away) to `ctx.scheduler` (single outer) and from `self.sub_tree` direct to `self.sub_tree` via the taken behavior.

**Companion documents:**
- Spec: `docs/superpowers/specs/2026-04-19-port-ownership-rewrite-design.md` §3.3 (clarified by this phase), §3.6, §4 D4.1/D4.6/D4.7/D4.11, §10 Phase 1 invariant list.
- Phase 1.5 plan (superseded for Tasks 2–5): `docs/superpowers/plans/2026-04-19-port-rewrite-phase-1-5-keystone-migration.md`.
- Phase 1.5 closeout: `docs/superpowers/notes/2026-04-19-phase-1-5-closeout.md`.
- Phase 1.5 ledger: `docs/superpowers/notes/2026-04-19-phase-1-5-ledger.md`.
- Brainstorm note (to write in B10): `docs/superpowers/notes/2026-04-20-phase-1-75-brainstorm.md` — captures the G/Y/Y2/Y3 option analysis and why Y3 was picked.
- Bootstrap/closeout ritual: `docs/superpowers/plans/2026-04-19-port-rewrite-bootstrap-ritual.md`.

**Entry precondition.** Phase 1.5 closeout `PARTIAL — Task 1 complete; Tasks 2–5 deferred`. Branch tagged `port-rewrite-phase-1-5-partial-complete`. Main at or ahead of that tag (currently `5060b9b` merged + `c41baff` plan-v1 + this plan overwrite commit). Working tree clean.

Sanctioned PARTIAL predecessor: Phase 1.75 exists precisely to close Phase 1.5's deferred tasks. At B4, record and do not halt.

**Baseline.** Phase 1.5 exit metrics: nextest 2455/0/9, goldens 237/6, `rc_refcell_total=282`, `diverged_total=177`, `rust_only_total=17`, `idiom_total=0`, `try_borrow_total=5`.

**JSON entries closed:** enumerated at Closeout C5.

---

## Bootstrap (per shared ritual)

Run B1–B12 with `<N>` = `1-75`.

Deviations:
- **B4.** Phase 1.5 closeout reads `PARTIAL — Task 1 complete; Tasks 2–5 deferred`. Sanctioned predecessor — this plan exists to close those deferred tasks. Record, do not halt.
- **B7.** Baseline = Phase 1.5 exit state.
- **B9.** Branch: `port-rewrite/phase-1-75`.
- **B10 (extended).** In addition to the standard ledger, write `docs/superpowers/notes/2026-04-20-phase-1-75-brainstorm.md` capturing: (a) the Phase-1.5 Task-2 blocker, (b) options G/Y/Y2/Y3 considered, (c) why Y3 was picked (spec-pure + smallest), (d) rejection rationale for each other option. This document exists so a future session can audit the reasoning without re-deriving it.
- **B11.** Commit: `phase-1-75: bootstrap — baseline captured, ledger + brainstorm opened`.

---

## File Structure

**Files modified:**
- `crates/emcore/src/emScheduler.rs` — add `engine_locations: SecondaryMap<EngineId, TreeLocation>` field; `register_engine` gains `tree_location` param; `DoTimeSlice` dispatch walks TreeLocation per engine (take/put behavior on path); `remove_engine` clears the map entry.
- `crates/emcore/src/emEngine.rs` — introduce `TreeLocation` enum.
- `crates/emcore/src/emPanel.rs` — add `PanelBehavior::as_sub_view_panel_mut(&mut self) -> Option<&mut emSubViewPanel>` with `None` default body. Only one other impl in the codebase overrides it (on `emSubViewPanel`).
- `crates/emcore/src/emSubViewPanel.rs` — **delete** `sub_scheduler` field + all Rc/RefCell usage around it; delete the sub-slice drive in `Cycle` (outer scheduler handles sub-tree engines natively); reduce `Cycle` body to animator tick + wake-status return; override `as_sub_view_panel_mut`; `new` signature gains `outer_panel_id: PanelId` so sub-tree engines register with valid `TreeLocation::SubView(outer_panel_id, Box::new(Outer))` immediately.
- `crates/emcore/src/emPanelTree.rs` — `register_engine_for` signature `fn<C: ConstructCtx>(&mut self, PanelId, tree_loc: TreeLocation, ctx: &mut C)`; `register_pending_engines` + its backing queue deleted; `create_child` and `init_panel_view` take `tree_loc` (outer callers pass `Outer`; sub-view callers pass `SubView(outer_id, Outer)`).
- `crates/emcore/src/emEngineCtx.rs` — absorb `PanelCtx` struct + impl from `emPanelCtx.rs`; introduce `trait ConstructCtx` with impls for `SchedCtx<'_>`, `EngineCtx<'_>`, new `BareSchedCtx<'_>`.
- `crates/emcore/src/lib.rs` — remove `pub mod emPanelCtx;`; add `pub use emEngineCtx::PanelCtx;`.
- `crates/emcore/src/emView.rs` — popup-signal pre-allocation block (near `RawVisitAbs`) replaced by inline `ctx.create_signal()` at 4 use sites.

**Files deleted:**
- `crates/emcore/src/emPanelCtx.rs` — PanelCtx absorbed into emEngineCtx.rs.

**Files heavily touched by import updates (bulk sed):**
- Every file with `use crate::emPanelCtx::PanelCtx;` → `use crate::emEngineCtx::PanelCtx;`. ~40 sites.

**Callers of `emSubViewPanel::new`:**
- Grep-verify all callers. Each must provide the caller's outer_panel_id. Mechanical in production (only a few call sites); test harnesses need light rewiring.

**Test files modified:**
- `crates/emcore/tests/sp4_5_fix_1_timing_*_baseline_slices.rs` — `delta == 1` → `delta == 0`.

**DIVERGED blocks:**
- `emSubViewPanel.rs:34-42` — **deleted entirely** (sub_scheduler gone; no divergence to document).

---

## Task sequencing

**Task 1: Plumbing — `TreeLocation` + `as_sub_view_panel_mut` + `ConstructCtx` (no behavior change).**
Introduce types/traits; existing code still uses `Option<&mut EngineScheduler>` and `sub_scheduler`. Intermediate `--no-verify` commit OK. End state: new types exist and compile.

**Task 2: Scheduler dispatch rewrite.**
`EngineScheduler::register_engine` gains `TreeLocation` param (default call sites pass `Outer`). `engine_locations: SecondaryMap` populated. `DoTimeSlice` walks the location per engine using `take_behavior`/`put_behavior` through `as_sub_view_panel_mut().sub_tree`. Outer-only codepaths still work (all engines register as `Outer` for now). Gate green.

**Task 3: Move sub-view panel engine registration from `sub_scheduler` to outer scheduler.**
`emSubViewPanel::new(parent_context, outer_panel_id)` — added outer_panel_id param. Inside `new`, sub-view's `RegisterEngines` call and `sub_tree` engine registrations now route through the **outer** scheduler handed in via ctx, tagging each with `TreeLocation::SubView(outer_panel_id, Box::new(Outer))`. Update callers of `emSubViewPanel::new` to pass outer_panel_id (caller must `create_child` the outer slot first, then pass the returned id). `sub_scheduler` field still present but unused; deletion in Task 4.

**Task 4: Delete `sub_scheduler` + reduce `emSubViewPanel::Cycle`.**
Delete the `sub_scheduler` field. `emSubViewPanel::Cycle` body reduces to: animator tick (unchanged) + return `animator_active || /* outer scheduler tracks sub-view engine awake state natively */ false`. The sub-tree sub-slice drive is gone — outer `DoTimeSlice` now dispatches sub-tree engines in the same priority pass as outer engines. Goldens must preserve 237/6 (priority invariant now C++-correct, which should match or improve pixel parity). Clippy clean. Gate green. **This is the keystone step of the phase** — plan for checkpoint `--no-verify` commits if it halts mid-migration; final commit must be green.

**Task 5: `register_engine_for(ctx)` + delete `register_pending_engines` + delete `emPanelCtx.rs`.**
Rewrite signature per File Structure. Delete `register_pending_engines` + backing queue. Verify: with TreeLocation always known at registration time (every caller has a concrete tree_loc), the prior deferral mechanism has no remaining use case. The `try_borrow` deferral at `emPanelTree.rs:595-597` (view_rc already borrowed by Update): **verify under new ctx shape whether this case still exists** — if `Update` no longer holds `view_rc.borrow_mut()` across child-creation points post-Phase-1, the deferral is obsolete; if it still exists, widen Task 5 scope to fix the upstream borrow or keep a minimal deferral (ledger decision point). Delete `emPanelCtx.rs`; absorb `PanelCtx` into `emEngineCtx.rs`; bulk-update imports. Gate green.

**Task 6: Popup-signal inline + timing-fixture delta=0.**
Replace `emView::RawVisitAbs` popup pre-allocation with inline `ctx.create_signal()` at use sites. Rewrite three `sp4_5_fix_1_timing_*_baseline_slices.rs` fixtures: `assert_eq!(delta, 1)` → `assert_eq!(delta, 0)`. If a fixture fails at `delta==0`, the underlying scheduler/view timing needs a real fix — do NOT lower assertion back; record and address. Gate green.

**Task 7: Spec §3.3 clarification (docs).**
Update §3.3 to *describe* the cross-tree dispatch mechanism (previously silent): name `TreeLocation`, explain the take/put walk, reference `as_sub_view_panel_mut`. The observational invariant ("outer and sub-view engines at priority P interleave in priority order") is **preserved verbatim** — this task makes the spec match what the code does, not weaken the spec. Also update §10 Phase-1 invariant bullets whose wording presumed `sub_scheduler` deletion would be Phase-1-Chunk-3: rewrite to cite Phase 1.75. Commit: `phase-1-75 task-7: clarify spec §3.3 cross-tree dispatch mechanism`.

**Task 8: Closeout prep.**
Full gate. All phase-1.75 invariants verified. Append Phase 2's B4 deviation (accept `port-rewrite-phase-1-75-complete` as the first COMPLETE predecessor in the chain). Proceed to Closeout.

---

## Closeout (per shared ritual)

Run C1–C11 with `<N>` = `1-75`.

Specific requirements:
- **C4.** Verify I1, I1c, I1d, I-Y3-dispatch, I-T3a, I-T3b, I-T3c, I-Spec-3.3-clarified, Task-10, Task-11. Verify Phase-1 carry-forwards (I1a, I1b, I6) remain SAT.
- **C5.** Sweep `2026-04-19-port-divergence-raw-material.json` for entries tied to `sub_scheduler`, `register_pending_engines`, `emPanelCtx.rs`, popup pre-allocation, timing-fixture delta. Cite Phase 1.75 commits.
- **C6.** Mark closed entries `status: resolved-phase-1-75`. Commit: `phase-1-75: mark JSON entries <list> resolved`.
- **C7.** Closeout status: `COMPLETE — all C1–C11 checks passed`. (First COMPLETE in the port-rewrite series.)
- **C10.** Tag: `port-rewrite-phase-1-75-complete`.

---

## Risk + halt-recovery

**Task 4 is the load-bearing step.** Expect 400-700 lines of diff concentrated there. If the subagent halts mid-Task-4, the branch carries `--no-verify` red commits from which the next session can resume — same pattern Phase 1.5 Task 1 used successfully.

**Priority-interleave observable change.** Task 4 changes WHEN sub-view engines fire relative to outer engines — today they fire atomically inside `emSubViewPanel::Cycle`; post-Task-4 they interleave by priority across trees. This is the *goal* (matches C++), but it may shift deterministic ordering in ways that perturb timing-fixture baselines. The three `sp4_5_fix_1_timing_*` fixtures in Task 6 are assertion-updated to `delta==0`, which should capture the new (C++-aligned) timing. Golden tests compare pixel output — priority reordering within a slice should not shift pixels unless a priority-sensitive engine was producing wrong-ordered writes (which would be a latent bug Task 4 exposes, not a Task-4 regression).

**Nested sub-views.** TreeLocation supports arbitrary nesting (`SubView(outer_id, Box<SubView(inner_id, Outer)>)`). Test explicitly during Task 2 with a depth-2 fixture before Task 3.

---

## Self-review checklist (before Closeout)

- [ ] `rg 'Rc<RefCell<EngineScheduler>>' crates/` empty.
- [ ] `rg -w 'sub_scheduler' crates/` empty.
- [ ] `rg 'try_borrow(_mut)?\(\)' crates/emcore/src/emSubViewPanel.rs crates/emcore/src/emView.rs crates/emcore/src/emPanelTree.rs` empty.
- [ ] `rg -w 'register_pending_engines' crates/` empty.
- [ ] `! test -e crates/emcore/src/emPanelCtx.rs`; `rg 'pub mod emPanelCtx' crates/emcore/src/lib.rs` empty; `rg 'emPanelCtx::' crates/` empty.
- [ ] Every `EngineScheduler::register_engine` call site passes a `TreeLocation`.
- [ ] Every `PanelBehavior` impl either inherits the `None` default or returns `Some(self)` (only emSubViewPanel overrides).
- [ ] `emSubViewPanel::Cycle` body contains no reference to a scheduler (outer or sub) — just animator tick.
- [ ] Spec §3.3 describes the cross-tree dispatch mechanism; observational invariant unchanged.
- [ ] Popup pre-allocation block absent; 4 inline `ctx.create_signal()` sites.
- [ ] Three timing fixtures assert `delta == 0`.
- [ ] Goldens 237/6 or better. Nextest ≥ 2455. Clippy clean.
- [ ] No `#[allow(...)]` introduced outside CLAUDE.md whitelist.
- [ ] No `Rc<RefCell<PanelTree>>` introduced.
- [ ] No nested `DoTimeSlice` call anywhere.
