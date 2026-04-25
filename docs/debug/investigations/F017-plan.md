---
issue: F017
created: 2026-04-25
status: ready
phases: 4 (+ Phase 0 discovery, + Final verification)
order: instrument → tick-rate → loading-dispatch → verify
---

# F017 implementation plan — directory loading throughput parity with C++

F017: zooming into a directory shows a visible "Loading: NN%" overlay that crawls cell-by-cell. Same op in C++ Eagle Mode 0.96.4 is too fast to see the overlay. **Out of scope:** F018 (loading-state black background).

Per project ideology (CLAUDE.md §Port Ideology): observable behavior must match C++. Rust deviations from the C++ shape are presumed to be design-intent until proven forced. This plan ports two confirmed divergences verbatim from C++ and gates each behind a verification step.

## Phase 0 — Documentation Discovery (findings)

Verified facts. Do not re-derive in implementation phases; cite these.

### Confirmed divergences vs C++

**(α) Scheduler tick rate — 5× slower in Rust**
- C++ `src/emCore/emScheduler.cpp:160` — `SyncTime+=10` → `DoTimeSlice` runs every **10 ms**; `DeadlineTime=SyncTime+50` (line 162) gives engines a 50 ms deadline budget per tick.
- Rust `crates/emcore/src/emScheduler.rs:115` — `TIME_SLICE_DURATION = Duration::from_millis(50)`. Tick runs every **50 ms**.
- Effect: every Rust panel `Cycle` runs at 1/5 the rate of the C++ original. With per-`Cycle` work that does one filename or one `lstat` (see (β)), Rust loads 5× slower for the same directory.

**(β) Loading dispatch path — emDirPanel bypasses step_loading**
- C++ `src/emCore/emFilePanel.cpp:151-161` — `emFilePanel::Cycle` does NOT call `StepLoading`. It only checks `FileStateSignal` and re-paints. Loading is driven by `emPriSchedAgent` (a priority-scheduler engine, separate from any panel cycle).
- C++ `src/emCore/emFileModel.cpp:340` — `StepLoading` is the only entry point: it calls `TryContinueLoading` once **and** recomputes `MemoryNeed` via `CalcMemoryNeed` after each call.
- Rust `crates/emfileman/src/emDirPanel.rs:376` — `emDirPanel::Cycle` calls `dm.try_continue_loading()` **directly**, skipping `step_loading`. Consequence: `emFileModel::memory_need` is never recomputed during `emDirPanel`-driven loading.
- Rust `crates/emcore/src/emFileModel.rs:404` — `step_loading` is the only writer of `self.memory_need`. Stale value is read by `emFilePanel::compute_vir_file_state` (`emFilePanel.rs:113`).
- The DIVERGED comment at `crates/emfileman/src/emDirModel.rs:207-210` claims "language-forced (Rust generics constraint)" — this rationale needs revisit in Phase 3.

### Confirmed parities

- Per-call lstat cost: identical. Both ports do **one** entry's metadata load per `try_continue_loading` / `TryContinueLoading` call. C++ `emDirModel.cpp:149-162`; Rust `emDirModel.rs:118`.
- Per-cycle entry budget: identical. Both load one entry per call.
- Progress-update cadence (250 ms): identical between ports.

### Allowed APIs

| Symbol | File:Line | Notes |
|---|---|---|
| `emFileModel::step_loading` | `crates/emcore/src/emFileModel.rs:365-417` | The verbatim-C++ entry point. Calls `try_continue_loading` once, then `calc_memory_need`. |
| `emFileModel::try_continue_loading` | `emFileModel.rs` (delegates to ops) | Existing trait method on `FileModelOps`. |
| `emPriSchedAgent` | `crates/emcore/src/emPriSchedAgent.rs` | Already ported. Priority-scheduler agent that drives `step_loading` in C++. |

### Anti-patterns

- ❌ Do **not** "tune" `TIME_SLICE_DURATION` to 20 ms or 25 ms as a partial fix. Port the C++ value (10 ms) verbatim; if a benchmark shows it crosses a documented degradation threshold, that becomes a Phase 2-amendment with the benchmark attached.
- ❌ Do **not** add new throttle constants ("entries per cycle", "max time per call"). C++ has none beyond `IsTimeSliceAtEnd`; Rust must not invent any.
- ❌ Do **not** refactor `step_loading` to "match what `emDirPanel::Cycle` is currently doing". The direction is the opposite: bring the panel back onto `step_loading`.

---

## Phase 1 — Diagnostic instrumentation (read-only)

**Goal:** before changing any behavior, measure to confirm which divergence dominates. Defaults to expecting (α) and (β) both contribute; instrumentation either confirms or surfaces a third unknown cause.

### Tasks

1. Add `RUST_ONLY:` (performance-instrument) opt-in counters behind an env var (e.g. `EM_LOAD_TRACE=1`):
   - Increment per `emDirModel::try_continue_loading` call; record wall-clock duration of each call.
   - Increment per `emDirPanel::Cycle` invocation while `FileState::Loading`.
   - Log `(call_count, total_ms, avg_call_ms)` once per second to stderr.
2. Run the app under `EM_LOAD_TRACE=1`, navigate cosmos → File System → some moderately-sized directory (e.g. `/usr/bin`). Capture stderr.
3. Compare to C++ baseline by reading `~/Projects/eaglemode-0.96.4/src/emCore/emScheduler.cpp` and `emFileModel.cpp` to compute the expected C++ rate (10 ms tick, ~one entry per tick → ~100 entries/sec ceiling; subtract per-entry stat cost).

### Verification checklist

- [ ] Counter output shows actual `Cycle`-per-second rate. **Expected ≈ 20/sec** (1000/50). If higher, scheduler isn't the bottleneck.
- [ ] Counter output shows mean `try_continue_loading` cost. **Expected < 1 ms per call** for typical filesystem; if much higher, candidate (b) is in play.
- [ ] Findings written to `docs/debug/investigations/F017.md` (create the file) with exact numbers.

### Anti-pattern guards

- ❌ Don't ship the counters past Phase 1. They are diagnostic only; remove in Phase 4 or before final commit.
- ❌ Don't form a hypothesis from code reading alone. The plan presumes (α)+(β); the measurements either confirm that or override it.

### Gate

Phase 2 begins only after Phase 1 numbers are recorded in `F017.md`. If measurements contradict (α) or (β), update this plan before proceeding.

---

## Phase 2 — Port C++ scheduler tick rate verbatim (candidate α)

**Goal:** make `TIME_SLICE_DURATION` match C++ exactly.

### What to implement

- Change `crates/emcore/src/emScheduler.rs:115` from `Duration::from_millis(50)` to `Duration::from_millis(10)`.
- Verify the 50 ms deadline budget is preserved separately (it should be tracked via `IsTimeSliceAtEnd` semantics, not `TIME_SLICE_DURATION`). If the Rust scheduler conflates "tick rate" with "deadline budget", split them: tick = 10 ms, deadline = tick + 50 ms.

### Documentation references

- C++ `emScheduler.cpp:159-162` — exact source of the 10 ms / 50 ms split. Copy the structure, not a "Rust-friendly" rewrite.
- Rust `emScheduler.rs:464` — `self.inner.deadline = Instant::now() + TIME_SLICE_DURATION;`. After the split this line must use the deadline budget, not the tick.

### Verification checklist

- [ ] `TIME_SLICE_DURATION` (or its replacement) is 10 ms.
- [ ] Deadline budget equals 50 ms and is independent of tick rate.
- [ ] `cargo check`, `cargo clippy -- -D warnings`, `cargo-nextest ntr` all clean.
- [ ] No new throttle constants introduced.
- [ ] Phase 1 counter reruns show ~5× higher Cycle rate (≈100/sec) — measurement, not assumed.
- [ ] No golden test regressions (run `scripts/verify_golden.sh --report`).

### Anti-pattern guards

- ❌ Don't pick a value between 10 and 50 to "balance CPU". C++ chose 10; we copy 10.
- ❌ If the Rust scheduler uses `time_slice_counter` arithmetic that depends on the 50 ms value (e.g. animation tween rates), don't paper over with a multiplier — find and fix every site that conflates tick with budget.

### Gate

Phase 3 begins only if Phase 2 ships green and Phase 1 counters are re-run to confirm the new tick rate.

---

## Phase 3 — Port loading dispatch via step_loading (candidate β)

**Goal:** route `emDirPanel`-driven loading through `emFileModel::step_loading` so `memory_need` is recomputed each call and the dispatch shape matches C++.

### What to implement

1. **Revisit the DIVERGED claim** at `crates/emfileman/src/emDirModel.rs:207-210`. It asserts language-forced. Per CLAUDE.md §"Forced divergence" criterion 1: try writing the C++ shape under the project's canonical ownership model. If the C++ shape (panel observes signal; priority scheduler drives `step_loading`) compiles, the language-forced claim is invalidated and must be removed.
2. **Two acceptable shapes**, in order of preference:
   - **(β.1) Match C++ structure exactly:** wire `emPriSchedAgent` to drive `step_loading` for the dir model. `emDirPanel::Cycle` reverts to the C++ shape: observe `FileStateSignal`, no direct loading call. This is the highest-fidelity port. `crates/emcore/src/emPriSchedAgent.rs` already exists (`grep` confirms). Read C++ `emFileModel.cpp` to find where the priority-scheduler agent is registered, mirror it.
   - **(β.2) Minimal fix if (β.1) is genuinely blocked:** keep panel-driven loading but replace the direct `try_continue_loading` call at `emDirPanel.rs:376` with a call to `step_loading`. This still recomputes `memory_need`. The DIVERGED annotation is updated to describe the fallback and cite which forced-category criterion blocks (β.1).
3. **Either way:** delete the stale `progress` write at `emDirPanel.rs:373` if `step_loading` already updates progress; otherwise keep it but cite C++ behavior.

### Documentation references

- C++ `emFilePanel.cpp:151-161` — exact `Cycle` shape (observe-only).
- C++ `emFileModel.cpp:340-402` — exact `StepLoading` body; `MemoryNeed` recompute at line 388.
- C++ `emPriSchedAgent.h` / `.cpp` — agent registration pattern.
- Rust `crates/emcore/src/emFileModel.rs:365-417` — current `step_loading` implementation; verify it matches C++ before delegating to it.
- Rust `crates/emcore/src/emPriSchedAgent.rs` — confirm public API; consult `crates/eaglemode/tests/behavioral/pri_sched_agent.rs` for a usage example.

### Verification checklist

- [ ] `emDirPanel::Cycle` no longer calls `try_continue_loading` directly. (`grep -n 'try_continue_loading' crates/emfileman/src/emDirPanel.rs` returns no hits.)
- [ ] `emFileModel::memory_need` is updated during dir-model loading. Add a unit test that loads a fixture dir and asserts `memory_need > 0` before `Loaded`.
- [ ] If shape (β.1) shipped: `emDirPanel::Cycle` matches the structure of C++ `emFilePanel::Cycle` (observe `FileStateSignal` → invalidate paint). DIVERGED comment at `emDirModel.rs:207-210` is removed.
- [ ] If shape (β.2) shipped: DIVERGED comment is updated with one of the four forced categories and a cited blocker.
- [ ] `cargo check`, `cargo clippy -- -D warnings`, `cargo-nextest ntr`, `cargo xtask annotations` all clean.
- [ ] Phase 1 counters re-run: per-call cost unchanged, Cycle rate unchanged from Phase 2.

### Anti-pattern guards

- ❌ Don't accept the existing DIVERGED note at face value. Verify the language-forced claim by attempting (β.1) first. The original commit may have taken the easier route.
- ❌ Don't introduce a new wrapper just to make `step_loading` callable — if there's a real ownership issue, document it concretely.
- ❌ Don't change `step_loading`'s body to accommodate the panel — `step_loading` is the C++-faithful entry point and must stay observably equivalent to `emFileModel::StepLoading`.

### Gate

Final verification begins only after Phase 3 lands green.

---

## Phase 4 — Final verification

**Goal:** confirm F017 is observably resolved.

### Tasks

1. Remove all Phase 1 instrumentation counters (no `EM_LOAD_TRACE` traces left behind).
2. Build release binary; launch app.
3. Navigate cosmos → File System; zoom into `/usr/bin` (or any directory with ≥ 200 entries).
4. **Acceptance criterion:** the "Loading: NN%" overlay must not be visible long enough to read the percentage. If it flashes briefly, that matches C++; if any digit is readable, F017 is not closed.
5. Repeat with a smaller directory (`/etc`) and a larger one (`/usr/share`) to confirm scaling behavior.
6. Compare side-by-side with C++ Eagle Mode 0.96.4 launching the same paths if practical.

### Verification checklist

- [ ] No instrumentation code remaining. (`grep -ri EM_LOAD_TRACE crates/` returns nothing.)
- [ ] All workspace tests pass: `cargo-nextest ntr` clean (subject to known pre-existing `emcore::plugin_invocation` baseline; document any new failures).
- [ ] Annotation lint clean: `cargo xtask annotations`.
- [ ] Golden tests clean: `scripts/verify_golden.sh --report` shows no new divergences.
- [ ] Manual GUI confirmation recorded in `docs/debug/investigations/F017.md` with paths tested and observed behavior.
- [ ] `docs/debug/ISSUES.json` updated: F017 status `closed`, `fix_note` summarizing what landed in each phase, `fixed_in_commit` filled.

### Anti-pattern guards

- ❌ Don't close F017 on test pass alone. The defining symptom is visual; require manual GUI verification.
- ❌ Don't conflate F017 closure with F018 progress. F018 (loading-state black background) is a separate symptom and may still reproduce after F017 closes — that is expected.

---

## Phase ordering rationale

- **Diagnostic first** because the plan presumes (α)+(β) but does not measure. Two cheap counters cost ~30 lines and rule out a third unknown cause.
- **(α) before (β)** because (α) is a one-line change with broad effect; landing it first lets Phase 3 measure (β)'s contribution against the corrected baseline.
- **(β) requires care** — it touches a previously-DIVERGED area. Doing it after (α) means we can measure whether the panel-dispatch divergence still matters once tick rate is fixed; (α) alone may close F017 and reduce (β) to a code-quality fix.
- **Final verification is GUI-bound** — the symptom is visible, so the close gate is visible.
