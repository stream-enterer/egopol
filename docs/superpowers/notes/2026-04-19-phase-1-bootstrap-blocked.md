# Phase 1 — Bootstrap Blocked

**Date:** 2026-04-19
**Branch:** `port-rewrite/phase-1` (created at B9; holds this note only)
**Blocker class:** plan/execution-model mismatch (NOT plan-contradicts-spec, NOT dirty tree)

## Status

BLOCKED at end of Bootstrap. B1–B10 complete; B11/B12 not executed. No task work dispatched.

## Bootstrap evidence captured

- **B1–B4:** spec + raw-material JSON + workaround ledger + CLAUDE.md read in full. Phase 1 is first phase; no predecessor closeout to check.
- **B5:** `git status --porcelain` empty. Clean.
- **B6:** `git rev-parse --abbrev-ref HEAD` = `main`. Clean.
- **B7 baseline (captured here; not written to a separate baseline.md since B11 was not reached):**
  - `cargo-nextest ntr`: `2451 tests run: 2451 passed, 9 skipped` — green.
  - `cargo test --test golden -- --test-threads=1`: `237 passed; 6 failed` — matches plan's expected baseline.
  - `cargo clippy --all-targets --all-features`: finishes clean.
  - `rc_refcell_total` (`rg -c 'Rc<RefCell<' crates/`): **284** (plan's §3.6 claims "current: 155" — plan is stale on this figure by ~130).
  - `diverged_total`: 177.
  - `rust_only_total`: 16.
  - `idiom_total`: 1 (matches plan — sole occurrence at `crates/emcore/src/emView.rs`).
  - `try_borrow_total`: 11.
- **B8:** baseline green; exactly 237/6 goldens as required.
- **B9:** phase branch `port-rewrite/phase-1` created from `main`.
- **B10:** not executed (see below).
- **B11/B12:** not executed.

## Root cause of halt — two compounding reasons

### Reason 1 — Plan's intermediate-red structure is incompatible with subagent-driven-development's per-task green gate.

The phase plan explicitly notes that intermediate tasks leave the tree uncompilable:

- Task 4 Step 9 → "Commit with Task 5. See Task 5 Step 7."
- Task 5 Step 7 → commits Tasks 4 + 5 together.
- Task 6 Step 5 → "Compile smoke. `cargo check -p emcore`. Expected: green (given Task 9's trait migration lands in the same phase)."
- Tasks 2, 3, 7, 8 each leave the tree in a broken state until later tasks land.

Under the subagent-driven-development model this driver is mandated to use, each implementer subagent is expected to produce a commit with its task's green gate (fmt + clippy + nextest). Tasks 2–8 cannot satisfy that gate by construction. The pre-commit hook (CLAUDE.md: "Runs `cargo fmt` then `clippy -D warnings` then `cargo-nextest ntr`. Do not skip with `--no-verify`") will reject every intermediate commit. There is no non-hacky execution path.

**The plan is coherent at phase granularity** — Closeout C1 gates the phase cliff green — but the plan was written for an executing-plans-style driver that can commit across uncompilable states, not subagent-driven-development which enforces per-task green.

### Reason 2 — Scope vs. single-session budget mismatch.

The workaround ledger `2026-04-19-scheduler-refcell-workaround-ledger.md` §6.4, from which this whole rewrite originates, sizes Option C (= this phase's plan): "Rough estimate: 300-600 LOC deleted, 100-200 LOC added, ~40 touched call sites. **A solid week of focused work with a good test safety net.**"

The plan understates downstream fan-out. Files outside `crates/emcore` that the plan does not enumerate but that its Task 2 + Task 8 deletions will break (verified by `rg -c 'scheduler\.borrow|scheduler\.borrow_mut|GetScheduler|NewRootWithScheduler' crates/`):

- `crates/emmain/src/emMainWindow.rs`: 17 references.
- `crates/emcore/src/emGUIFramework.rs`: 7 references.
- `crates/emcore/src/emContext.rs`: 11 references.
- `crates/emcore/src/emSubViewPanel.rs`: 14 references.
- `crates/emcore/src/emView.rs`: 9 references.
- `crates/eaglemode/tests/unit/popup_materialization.rs`: 4 references.
- `crates/eaglemode/tests/unit/popup_cancel_before_materialize.rs`: 4 references.

Total: 67 direct scheduler-access sites to migrate — higher than the ledger's "~40" estimate, and the plan's Task lists do not enumerate the emmain/tests call sites.

Additional unplanned fan-out: `crates/emmain/src/emMainWindow.rs` is a downstream consumer whose scheduler-touching code the plan does not describe task-by-task. `crates/eaglemode/tests/` contains ~20 integration/pipeline/unit tests (sample from earlier `rg`: `pipeline/check.rs:5`, `pipeline/splitter.rs:4`, `pipeline/radio.rs:3`, `pipeline/calibration.rs:3`, `unit/scheduler.rs:4`, `unit/image_file_panel.rs:2`, etc.) each with their own `Rc<RefCell<EngineScheduler>>` or `Rc<RefCell<emWindow>>` patterns that Tasks 2/4/6 will break.

Single-driver-session budget estimate: orchestrating ~12 implementer dispatches + ~24–36 reviewer dispatches, with review-loop retries, exceeds the context/time envelope of one unattended driver turn by a large multiple. The ledger's own week-of-focused-work sizing is for a human engineer with full IDE state; an agent driving through subagents with no state continuity across turns is slower, not faster.

## What this note is NOT claiming

- Not claiming the spec is wrong. §3.1–§3.7 of the spec are coherent and the destination state is well-defined.
- Not claiming the plan contradicts the spec. It implements §3.1 and §4 D4.1–D4.11 faithfully.
- Not claiming the plan is unimplementable. It is implementable — by a human engineer over a week, or by a sequence of driver sessions each tackling a sub-slice.
- Not claiming a regression exists. Baseline is clean.

## Recommendation (for the human or the next driver session)

Either (a) **re-plan as sub-phases with cliff-green commits**, each a self-contained executing-plans-style chunk (e.g., 1.1 = introduce EngineCtx scaffolding + Option B shim so tree stays green; 1.2 = migrate scheduler owner + Migrate View call sites under shim; 1.3 = delete shim + SchedOp; 1.4 = per-sub-view scheduler deletion; 1.5 = close gate), or (b) **execute the existing plan in a multi-session sequence with a custom driver that tolerates intermediate-red states across sessions**, with the phase branch living across sessions and closeout deferred to the final session.

The current single-shot subagent-driven-development mandate over a single-cliff plan is the combination this halt note documents as infeasible.

## Files touched by Bootstrap before halt

- Created branch `port-rewrite/phase-1` (B9).
- Created this note.
- No other writes. B11 commit skipped.
