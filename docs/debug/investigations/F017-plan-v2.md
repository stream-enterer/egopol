---
issue: F017
created: 2026-04-25
status: ready
supersedes: F017-plan.md
phases: 3 (one coherent structural change)
---

# F017 implementation plan v2 — restore the C++ loading-loop architecture

F017: zooming into a directory shows a visible "Loading: NN%" overlay that crawls cell-by-cell. C++ Eagle Mode 0.96.4 is too fast to read the overlay. **Out of scope:** F018 (loading-state black background).

This plan supersedes `F017-plan.md`, which fragmented the work into independent diagnostic and fix phases. The two traces in `F017-trace.md` show the divergence is **one architectural shape**, not a bag of independent symptoms. This plan ports that shape in three sequenced steps that must land together.

## Phase 0 — Findings

Two parallel traces (C++ at `~/Projects/eaglemode-0.96.4/`, Rust at this repo) of the same scenario — user zooms into a directory, first entry's `lstat` completes — produced row-aligned call graphs. The full traces and diff tables are in `F017-trace.md`. The architectural picture:

### C++ shape

- **`emFilePanel::Cycle`** (`src/emCore/emFilePanel.cpp:151-161`) is observe-only: checks `FileStateSignal`, calls `InvalidatePainting()`. **No loading dispatch from any panel.**
- **`emFileModel::Cycle`** (`src/emCore/emFileModel.cpp:243-260`) owns the loading loop:
  ```cpp
  do {
      if (StepLoading()) stateChanged=true;
  } while (State==FS_LOADING && !IsTimeSliceAtEnd());
  ```
  Calls `StepLoading` repeatedly within one Cycle invocation, until the model finishes loading or the 50 ms deadline budget is hit.
- **`emPriSchedAgent`** (`src/emCore/emPriSchedAgent.cpp:98-123`) gates CPU access. `emFileModel::StartPSAgent` (`emFileModel.cpp:570-575`) enrolls the model when memory allows. The agent's `GotAccess` callback (`emFileModel.cpp:593-596`) re-wakes the model so its `Cycle` can resume the inner loop on the next tick.

Net effect: C++ loads as many entries as fit in 50 ms per scheduler tick — in practice hundreds.

### Rust shape (current)

- **`emDirPanel::Cycle`** (`crates/emfileman/src/emDirPanel.rs:307-403`) drives loading directly. Calls `dm.try_continue_loading()` once at line 376, then returns `true` to be re-cycled.
- **`emFileModel::Cycle` does not exist.** `emFileModel::step_loading` (`crates/emcore/src/emFileModel.rs:365-417`) is single-call with no inner loop.
- **`emPriSchedAgent` is bypassed.** It exists (`crates/emcore/src/emPriSchedAgent.rs`) and has tests, but the production loading path does not enroll.
- **DIVERGED note at `crates/emfileman/src/emDirModel.rs:207-210`** claims language-forced. The trace shows the C++ shape is buildable; the claim is wrong and must be removed in Step 3.

Net effect: one entry per Cycle invocation, regardless of how much deadline budget remains.

### What is **not** the problem

- **Scheduler tick rate.** `TIME_SLICE_DURATION = 50ms` (`emScheduler.rs:115`) vs. C++'s 10 ms tick (`emScheduler.cpp:160`) is a real difference but is **secondary**. Rust's `run()` loop has no sleep; effective tick rate is paced by winit, not by `TIME_SLICE_DURATION`. Once the inner loop exists, throughput is gated by the deadline budget per tick, not by tick rate. Do **not** change `TIME_SLICE_DURATION` as part of this fix.
- **Per-call `lstat` cost.** Identical between ports (one `lstat` per `try_continue_loading`).
- **`memory_need` recomputation.** The bypassed-`step_loading` issue from the v1 plan is real but is a *consequence* of the wrong loop owner, not an independent fix. Step 2 fixes it by routing through `step_loading`.

### Allowed APIs (verified)

| Symbol | File:Line | Notes |
|---|---|---|
| `emFileModel::step_loading` | `emFileModel.rs:365-417` | The body is correct as-is; needs a Cycle wrapper. |
| `emPriSchedAgent` | `emcore/src/emPriSchedAgent.rs` | Already ported. Behavioral test at `crates/eaglemode/tests/behavioral/pri_sched_agent.rs` shows usage. |
| Engine context | `emEngine.rs`, `emEngineCtx.rs` | Step 1 adds `is_time_slice_at_end()` here. |

### Anti-patterns this plan rejects

- ❌ "Instrument first to confirm." The two traces are the instrumentation. Confirming with counters before fixing only delays.
- ❌ "Tune `TIME_SLICE_DURATION` as a partial fix." It's not the bottleneck, and tuning it leaves the structural divergence in place.
- ❌ "Land Step 1 alone, see if it helps." It can't — `is_time_slice_at_end` does nothing without a loop calling it. The three steps land together or not at all.
- ❌ "Keep the old DIVERGED note as a fallback." If Step 2 ships, the language-forced claim is disproven; remove the note.
- ❌ "Add a new throttle constant." C++ has none beyond the 50 ms deadline. Rust must not invent any.

---

## Step 1 — Add `is_time_slice_at_end` to the engine context

**Goal:** provide the deadline-check primitive the loop body in Step 2 needs. Mechanical, isolated, no behavior change on its own.

### What to implement

- Add `is_time_slice_at_end(&self) -> bool` on whatever type backs `EngineCtx`. Implementation: `Instant::now() >= self.inner.deadline` (the deadline field already exists at `emScheduler.rs:464`).
- Port the C++ name verbatim: `IsTimeSliceAtEnd` (per CLAUDE.md File and Name Correspondence).
- Mirror C++ visibility: public on the context type (`emScheduler.cpp:171` makes it public via `emScheduler::IsTimeSliceAtEnd`).

### Documentation references

- C++ `emScheduler.cpp:171` — `return emGetClockMS()>=DeadlineTime;` — exact body to mirror.
- C++ `emScheduler.h` — declaration shape for visibility.
- Rust `emScheduler.rs:464` — `deadline` field already maintained per tick.

### Verification

- [ ] Method exists on `EngineCtx` with the C++ name.
- [ ] Unit test: construct a context, set the deadline to `Instant::now() - 1ms`, assert `is_time_slice_at_end()` returns `true`. Set deadline to `Instant::now() + 100ms`, assert `false`.
- [ ] `cargo check`, `cargo clippy -- -D warnings`, `cargo-nextest ntr` clean.
- [ ] No callers yet — Step 2 adds the only caller. (`grep -n is_time_slice_at_end` returns one definition, no usages.)

### Anti-pattern guards

- ❌ Don't add a wrapper method with a different name "for Rust idiom." The C++ name is part of name correspondence.
- ❌ Don't expose the raw `deadline` field as a public method instead. C++ exposes the boolean predicate only.

---

## Step 2 — Port `emFileModel::Cycle` (loading loop owner) and `StartPSAgent`

**Goal:** make `emFileModel` the loading-loop owner, matching C++. This is the fix; Steps 1 and 3 are plumbing for it.

### What to implement

1. **`emFileModel::Cycle`** — port the C++ shape verbatim from `emFileModel.cpp:243-260`:
   ```rust
   fn Cycle(&mut self, ctx: &mut EngineCtx, ops: &mut O) -> bool {
       // ... PSAgent::HasAccess() check (matches cpp:243-249) ...
       let mut state_changed = false;
       loop {
           if self.step_loading(ops) { state_changed = true; }
           if !matches!(self.state, FileState::Loading { .. }) { break; }
           if ctx.is_time_slice_at_end() { break; }
       }
       // ... UpdateFileProgress, Signal(FileStateSignal), return true if still Loading ...
       matches!(self.state, FileState::Loading { .. })
   }
   ```
   The Rust loop is `loop { ...; if cond { break; } }` because Rust has no `do-while`; it preserves the C++ "execute body at least once" semantic.

2. **`emFileModel::StartPSAgent`** — port `emFileModel.cpp:570-575`. Enroll a `PriSchedAgent` named `"cpu"` when transitioning from `Waiting` to `Loading` (the C++ entry path is `UpdateMemoryLimit` → `StartPSAgent` at `emFileModel.cpp:538`).

3. **`emFileModel::PSAgentClass::GotAccess`** — port `emFileModel.cpp:593-596`. Calls `self.wake_up()` on the model.

4. **Register `emFileModel` as an engine.** Currently `emFileModel` is not a `Cycle`-bearing engine in Rust. Ports the C++ `emEngine` inheritance: `emFileModel` derives `emEngine` at `emFileModel.h`. In Rust, that means `emFileModel` registers itself in the scheduler (`register_engine`) when constructed. Use `Priority::Default` (matches C++ `DEFAULT_PRIORITY` at `emFileModel.cpp:194`).

5. **Update `memory_need` and `file_progress` in the Cycle body** — both already happen inside `step_loading` (line 404) and need an `UpdateFileProgress` equivalent at the loop tail (mirror `emFileModel.cpp:258`). Port `UpdateFileProgress` from `emFileModel.cpp:462-490` if not already present (calls `CalcFileProgress`, signals on change).

### Documentation references

- C++ `emFileModel.cpp:220-272` — full `Cycle` body. Read in full; copy structure.
- C++ `emFileModel.cpp:340-402` — `StepLoading` body (Rust `step_loading` already matches; verify no drift).
- C++ `emFileModel.cpp:462-490` — `UpdateFileProgress`.
- C++ `emFileModel.cpp:570-596` — `StartPSAgent`, `PSAgentClass::GotAccess`.
- C++ `emPriSchedAgent.cpp:47-59` — `RequestAccess` shape; behavioral test at `crates/eaglemode/tests/behavioral/pri_sched_agent.rs` shows the Rust API.

### Verification

- [ ] `emFileModel::Cycle` exists and contains a `loop`/`break` body that calls `step_loading` until either state leaves `Loading` or `ctx.is_time_slice_at_end()` returns `true`.
- [ ] `emFileModel` registers as an engine on construction; awake when `state == Loading`; sleeps when `Loaded`/`LoadError`.
- [ ] `StartPSAgent` enrolls a `PriSchedAgent` and `GotAccess` re-wakes the model.
- [ ] Unit test: construct an `emDirModel` for a temp dir of N entries (e.g. N=200). Drive a single `Cycle` call with a context whose deadline is 100 ms in the future. Assert that **multiple** entries are loaded in that one call (`entry_count > 1`). This is the test that proves the inner loop works.
- [ ] Unit test: same setup, but deadline is 0 ms in the past. Assert exactly one `step_loading` is called (the do-while "execute at least once" semantic).
- [ ] `memory_need` is updated each iteration (already true via `step_loading`).
- [ ] `Signal(FileStateSignal)` fires when `state_changed`.
- [ ] `cargo check`, `cargo clippy -- -D warnings`, `cargo-nextest ntr`, `cargo xtask annotations` all clean.
- [ ] No new throttle constants. No `TIME_SLICE_DURATION` change.

### Anti-pattern guards

- ❌ Don't write `while State==Loading && !is_time_slice_at_end()` as a head-checked loop — C++ uses do-while ("execute at least once even if deadline already passed"). Use `loop { body; if exit { break; } }`.
- ❌ Don't change `step_loading`'s body to accommodate the new caller. `step_loading` is the C++-faithful inner step; keep it stable.
- ❌ Don't make the `PriSchedAgent` enrollment conditional on a feature flag or "phase 2." It's part of the same change.

---

## Step 3 — Revert `emDirPanel::Cycle` to observe-only

**Goal:** match C++ `emFilePanel::Cycle` exactly. Loading is no longer the panel's job.

### What to implement

1. **Remove the loading dispatch from `emDirPanel::Cycle`.** Delete lines 341-378 of `emDirPanel.rs` — the `try_start_loading` / `try_continue_loading` block. The new body matches C++ `emFilePanel::Cycle` (`emFilePanel.cpp:151-161`): observe `FileStateSignal`, call `refresh_vir_file_state()`, signal `VirFileStateSignal` to dependents, return `false` (sleep).
2. **Move `update_children` to fire on `FileStateSignal` transition to `Loaded`.** In C++ this is `emDirPanel::Notice(NF_FILE_STATE_CHANGED)` at `emDirPanel.cpp:77-82`. Port the Notice handler if not already present.
3. **Remove `loading_done` and `loading_error` fields from `emDirPanel`.** They duplicate state that now lives entirely on `emFileModel`. Replace reads with calls into the model.
4. **Remove the DIVERGED comment at `emDirModel.rs:207-210`.** The language-forced claim is disproven by Step 2 building successfully. Confirm by searching for any other DIVERGED notes that referenced this one and update them.

### Documentation references

- C++ `emFilePanel.cpp:151-161` — the new body shape.
- C++ `emDirPanel.cpp:71-88` — the `Notice` handler that consumes `NF_FILE_STATE_CHANGED`.
- Rust `emDirPanel.rs:307-403` — current Cycle to be replaced.

### Verification

- [ ] `grep -n 'try_continue_loading\|try_start_loading' crates/emfileman/src/emDirPanel.rs` returns no hits.
- [ ] `emDirPanel::Cycle` body is observe-only and returns `false` to sleep.
- [ ] `update_children` fires when the model state transitions to `Loaded` (driven by `FileStateSignal`, not by polling in `Cycle`).
- [ ] DIVERGED note at `emDirModel.rs:207-210` is removed. `cargo xtask annotations` clean.
- [ ] All emfileman tests pass (167+ at last count).
- [ ] No regressions in golden tests: `scripts/verify_golden.sh --report` clean.
- [ ] `cargo check`, `cargo clippy -- -D warnings`, `cargo-nextest ntr` clean.

### Anti-pattern guards

- ❌ Don't keep `loading_done` "for now" with a TODO. Remove it; the model owns loading state.
- ❌ Don't gate the revert behind a feature flag. The new `emFileModel::Cycle` is the authoritative loading driver; the old panel-driven path must not coexist.
- ❌ Don't preserve the direct `try_continue_loading` call as a "fallback" — same reason.

---

## Final verification — GUI

The symptom is visual; the verification must be visual.

1. Build release binary.
2. Launch app. Navigate cosmos → File System.
3. Zoom into `/usr/bin` (≈3000 entries). **Pass criterion:** the "Loading: NN%" overlay must not be visible long enough to read the percentage. A brief flash matches C++; any readable digit fails.
4. Repeat with `/etc` (small) and `/usr/share` (large) to confirm scaling.
5. If feasible, side-by-side with C++ Eagle Mode 0.96.4 launching the same paths.
6. Update `docs/debug/ISSUES.json`: F017 status `closed`, `fix_note` summarizing the structural port, `fixed_in_commit` filled.

### Anti-pattern guards

- ❌ Don't close F017 on test pass alone. The defining symptom is visual.
- ❌ Don't conflate F017 closure with F018 progress. F018 may still reproduce after this fix; that is expected.

---

## Why these three steps are one change

- Step 1 alone: dead code (no callers).
- Step 2 alone: panic / wrong behavior — `emDirPanel::Cycle` still dispatches loading directly; both paths race or one wins arbitrarily.
- Step 3 alone: nothing drives loading at all; `Loading: NN%` overlay would freeze.

Land them in the same commit (or contiguous commits in a single PR with no intermediate green state). The three-step ordering is for cognitive structure during implementation, not for landing strategy.
