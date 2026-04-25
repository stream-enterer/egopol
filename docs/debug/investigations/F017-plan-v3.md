---
issue: F017
created: 2026-04-25
revised: 2026-04-25 (post-architectural investigation)
status: ready
supersedes: F017-plan-v2.md, F017-plan.md
---

# F017 implementation plan v3 — restore the C++ loading-loop architecture

This plan supersedes v2. v2 was insufficiently specified (no edit budget, no public-API contract, deferred architectural choice, per-crate verification, no stop-and-ask trigger). The result was an 800-line cross-crate refactor when the change should have been ~500 lines in three files. v3 closes those gaps and incorporates findings from a follow-up architectural investigation that surfaced three constraints v2 missed.

**Scope: one combined step.** v2 framed the work as Steps 2 and 3 (Step 1 turned out to already exist). The investigation showed they cannot be split — the public-field-as-mirror trick that would let Step 2 stand alone doesn't work, because a public field cannot mirror an inner state. Steps 2 and 3 land as one commit.

**PSAgent integration is deferred to a later issue, not in F017's scope.** The investigation showed Rust's `PriSchedModel` callback signature (`Box<dyn FnMut()>`) cannot reach a scheduler, so `GotAccess` cannot wake the file-model engine the way C++ does. Implementing it the C++ way requires extending `emPriSchedAgent.rs`'s API — outside this fix's edit budget. The do-while loop is what closes F017's visible symptom; CPU arbitration is C++'s fairness mechanism for multiple competing models and matters when more model types come online.

The technical premise (Phase 0 findings) is unchanged from v2. Trace evidence is in this conversation's two-subagent investigation.

---

## Phase 0 — Findings

### Why F017 reproduces

- C++ **`emFileModel::Cycle`** (`~/Projects/eaglemode-0.96.4/src/emCore/emFileModel.cpp:243-272`) owns the loading loop and contains an inner do-while:
  ```cpp
  do {
      if (StepLoading()) stateChanged=true;
  } while (State==FS_LOADING && !IsTimeSliceAtEnd());
  ```
  Loads as many entries as fit in the 50 ms deadline budget per scheduler tick.
- C++ **`emFilePanel::Cycle`** (`emFilePanel.cpp:151-161`) is observe-only — checks `FileStateSignal`, calls `InvalidatePainting`. **No loading dispatch from any panel.**
- Rust **`emDirPanel::Cycle`** (`crates/emfileman/src/emDirPanel.rs:307-403`) drives loading directly, with one `try_continue_loading` call per Cycle invocation (line 376). One entry per scheduler tick, regardless of remaining deadline budget.
- Rust **`emFileModel::Cycle` does not exist.** `emFileModel::step_loading` is single-call.
- Rust **`emDirModel`** (`emDirModel.rs:214`) does not compose `emFileModel<T>`; it's a parallel state machine with a DIVERGED note at line 207 claiming "language-forced (Rust generics constraint)." The note is wrong; the composition builds.

### What is **not** the problem

- Scheduler tick rate (`TIME_SLICE_DURATION = 50ms` at `emScheduler.rs:115`) is **not** the F017 bottleneck. Do not change it.
- Per-call `lstat` cost is identical between ports.
- `memory_need` recomputation is a *consequence* of the wrong loop owner, not an independent fix.

### Pre-existing facts

- **`EngineCtx::IsTimeSliceAtEnd` already exists** at `crates/emcore/src/emEngineCtx.rs:191-193` (forwards to `EngineScheduler::IsTimeSliceAtEnd` at `emScheduler.rs:684-687`). v2 listed adding this as Step 1; it was already done. **There is no Step 1.**
- Two pre-existing test failures (`emcore::plugin_invocation::try_create_file_panel_loads_plugin`, `…_missing_symbol_errors`) and one pre-existing clippy warning (`emFpPlugin.rs:201` `field-reassign-with-default`) are tolerated. Do not "fix" them.
- Current `emDirModel::Acquire` signature (verified, line 223): `pub fn Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<Self>>`. Acquire's closure has no scheduler access — engine registration cannot happen inside it.
- `emContext` does **not** hold a scheduler reference. Models cannot reach the scheduler through `ctx` alone.
- No Rust model currently combines `Acquire(ctx, name)` registry pattern with engine registration. This fix establishes the pattern.

### Architectural constraints discovered (and resolved below)

1. **Engine ownership puzzle**: `Acquire` returns `Rc<RefCell<emDirModel>>`, but the scheduler stores `Box<dyn emEngine>`. Same struct cannot live in both. → **Shim pattern**: separate `emDirModelEngine` type holds `Weak<RefCell<emDirModel>>` and forwards `Cycle`.

2. **Lazy engine registration**: Acquire's closure cannot reach the scheduler. → **Caller-side**: `emDirPanel` calls `emDirModel::ensure_engine_registered(model_rc, ctx, sched_ctx)` after `Acquire`, on first access. Acquire signature unchanged.

3. **PriSchedModel callback signature mismatch**: `Box<dyn FnMut()>` cannot reach the scheduler. → **PSAgent deferred to a later issue.** F017 ports the do-while loop and engine ownership, not CPU arbitration.

---

## Architecture (committed; field-level)

### `emDirModel` — composed type

```rust
pub struct emDirModel {
    file_model: emFileModel<()>,        // state machine: state, memory_need, file_progress, change_signal
    data: emDirModelData,               // loader workspace (renamed from current state-machine usage)
                                        // implements FileModelOps
    path: String,                       // unchanged
    engine_id: Option<EngineId>,        // None until ensure_engine_registered runs once
}
```

`emDirModelData` (the type currently used as a parallel state machine inside `emDirModel`) is reframed as the loader/ops object. It implements `FileModelOps` (existing trait at `emFileModel.rs:72-105`); the trait body is unchanged.

The current `pub state: FileState` field on `emDirModel` is **deleted**. The canonical state lives on `self.file_model.state`, accessed only via `get_file_state()`. All call sites in `emDirPanel.rs` that write `dm.state` (lines 344, 363, 372, 657) are deleted as part of this fix — those writes were the Rust port's panel-driven workaround for not having an engine-driven model.

### `emDirModelEngine` — engine shim (RUST_ONLY)

```rust
struct emDirModelEngine {
    model: Weak<RefCell<emDirModel>>,
}

impl emEngine for emDirModelEngine {
    fn Cycle(&mut self, ctx: &mut EngineCtx) -> bool {
        let Some(rc) = self.model.upgrade() else { return false; };
        rc.borrow_mut().cycle(ctx)
    }
}
```

Annotated `RUST_ONLY: (language-forced) Acquire pattern returns Rc<RefCell<>>; engine registration moves Box<dyn emEngine> into scheduler. Same struct cannot satisfy both. Shim holds Weak ref to allow the model to be both shared and registered.` The justification matches the `RUST_ONLY` charter in CLAUDE.md (language-forced utility).

### `emFileModel<T>` — new public methods

Two new public methods, both ports of C++ shapes:

- **`pub fn Cycle<O: FileModelOps>(&mut self, ctx: &mut EngineCtx, ops: &mut O) -> bool`** — port of C++ `emFileModel::Cycle` (`emFileModel.cpp:243-272`). Contains the do-while via Rust `loop { body; if exit { break; } }` to preserve "execute body at least once." Returns `true` if `state == Loading` (engine should stay awake).

  Body shape (committed; this is the exact Rust structure to write):
  ```rust
  pub fn Cycle<O: FileModelOps>(&mut self, ctx: &mut EngineCtx, ops: &mut O) -> bool {
      let mut state_changed = false;

      // DIVERGED: (upstream-gap-forced) C++ Cycle calls StartPSAgent and
      // UpdateMemoryLimit before the loop. PSAgent integration is deferred
      // from F017 scope (Rust PriSchedModel callback signature is
      // incompatible with C++ GotAccess→WakeUp; tracked separately).
      // UpdateMemoryLimit signals memory pressure that no panel currently
      // reads in the Rust port.

      if matches!(self.state, FileState::Loading { .. }) {
          loop {
              if self.step_loading(ops) { state_changed = true; }
              if !matches!(self.state, FileState::Loading { .. }) { break; }
              if ctx.IsTimeSliceAtEnd() { break; }
          }
      }

      if self.UpdateFileProgress(ops) { state_changed = true; }
      if state_changed { ctx.fire(self.change_signal); }

      matches!(self.state, FileState::Loading { .. })
  }
  ```

  The C++ FS_SAVING branch is omitted — saving uses `step_saving` and is panel-driven elsewhere in the Rust port; F017 ports loading only. If a future fix wants engine-driven saving, it extends this body.

- **`pub fn UpdateFileProgress<O: FileModelOps>(&mut self, ops: &O) -> bool`** — port of C++ `emFileModel::UpdateFileProgress` (`emFileModel.cpp:462-490`). 250 ms throttle. Returns `true` if progress changed (caller fires `change_signal`).

### `emDirModel` methods

```rust
impl emDirModel {
    // Existing — preserved unchanged
    pub fn Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<Self>> { ... }
    pub fn get_file_state(&self) -> FileState { self.file_model.GetFileState().clone() }
    pub fn try_start_loading(&mut self) -> Result<(), String> { self.data.try_start_loading_from(&self.path) }
    pub fn try_continue_loading(&mut self) -> Result<bool, String> { self.data.try_continue_loading() }
    pub fn quit_loading(&mut self) { self.data.quit_loading() }
    pub fn calc_file_progress(&self) -> f64 { self.data.calc_file_progress() }
    pub fn GetEntryCount(&self) -> usize { self.data.GetEntryCount() }
    // ... others delegate similarly

    // NEW — engine cycle (called by shim's Cycle)
    pub(crate) fn cycle(&mut self, ctx: &mut EngineCtx) -> bool {
        self.file_model.Cycle(ctx, &mut self.data)
    }

    // NEW — lazy engine registration; called by emDirPanel after Acquire
    pub fn ensure_engine_registered(
        model_rc: &Rc<RefCell<emDirModel>>,
        scheduler: &mut EngineScheduler,
    ) {
        if model_rc.borrow().engine_id.is_some() { return; }
        let weak = Rc::downgrade(model_rc);
        let engine = Box::new(emDirModelEngine { model: weak });
        let engine_id = scheduler.register_engine(
            engine,
            Priority::Default,
            PanelScope::Framework,
        );
        scheduler.wake_up(engine_id);  // initial wake so first Cycle runs
        model_rc.borrow_mut().engine_id = Some(engine_id);
    }
}
```

Signature resolved: `&mut EngineScheduler` directly. Both `Acquire` call sites in `emDirPanel.rs` have access:

- **In `Cycle` (line 318)**: pass `_ectx.scheduler` — the `EngineCtx::scheduler` field is `&'a mut EngineScheduler` (verified at `emEngineCtx.rs:52`).
- **In `notice` (line 446)**: pass `ctx.scheduler.as_deref_mut().expect("ensure_engine_registered requires full-reach panel context")` — `PanelCtx::scheduler` is `Option<&'a mut EngineScheduler>` (verified at `emEngineCtx.rs:445-446`); the `Option` is `None` only in layout-only test contexts that never run the model lifecycle. Production call sites set it via `PanelCycleEngine`.

The function is idempotent (early-returns if `engine_id.is_some()`), so calling it from both `Cycle` and `notice` is safe.

### `emDirPanel::Cycle` — new shape (port of C++ emFilePanel::Cycle)

```rust
fn Cycle(&mut self, ectx: &mut EngineCtx, ctx: &mut PanelCtx) -> bool {
    if self.dir_model.is_none() {
        // Acquire model on first interesting state; existing code at line 318
        let dm_rc = emDirModel::Acquire(&self.ctx, &self.path);
        emDirModel::ensure_engine_registered(&dm_rc, ectx.scheduler);
        self.file_panel.SetFileModel(Some(Rc::clone(&dm_rc) as Rc<RefCell<dyn FileModelState>>));
        self.dir_model = Some(dm_rc);
    }
    // C++ emFilePanel::Cycle equivalent: observe FileStateSignal, repaint
    if let Some(dm_rc) = &self.dir_model {
        let signal = dm_rc.borrow().GetFileStateSignal();
        if ectx.IsSignaled(signal) {
            self.file_panel.refresh_vir_file_state();
            // Trigger update_children on first observation of Loaded
            // (no transition cache needed — child_count == 0 means we
            // haven't built children yet, and we only run on signal)
            let state = dm_rc.borrow().get_file_state();
            if matches!(state, FileState::Loaded) && self.child_count == 0 {
                self.update_children(ctx);
            }
        }
    }
    false  // sleep; the engine drives loading, not the panel
}
```

`GetFileStateSignal` exists at `emFileModel.rs:169` (returns `SignalId`). `EngineCtx::IsSignaled(SignalId) -> bool` exists at `emEngineCtx.rs:180-183`. No new accessors needed.

The `update_children` placement stays in `Cycle` (not moved to `notice`). Rationale: post-revert, `Cycle` runs only when `FileStateSignal` fires (the panel returns `false` to sleep otherwise), so observing `state == Loaded && child_count == 0` is exactly the transition trigger — equivalent to a previous-state cache, simpler to write. The existing `notice` method at line 431-463 stays unchanged.

The `notice` method's existing `Acquire` call (line 446) gets the same `ensure_engine_registered` follow-up:
```rust
let dm_rc = emDirModel::Acquire(&self.ctx, &self.path);
if let Some(sched) = ctx.scheduler.as_deref_mut() {
    emDirModel::ensure_engine_registered(&dm_rc, sched);
}
```
The `if let Some(sched)` guard is required because `PanelCtx::scheduler` is `Option`. In production it's always `Some` (set by `PanelCycleEngine`); in layout-only tests it's `None` and the model is never loaded, so skipping registration is correct.

### `loading_done` and `loading_error` fields on `emDirPanel`

Removed. State lives entirely on `emFileModel`. Replacements:

- **`loading_done` readers** (e.g. `IsOpaque` at line 477) → `matches!(self.dir_model.as_ref().and_then(|m| Some(m.borrow().get_file_state())), Some(FileState::Loaded))`. Or shorter via a private helper `fn is_loaded(&self) -> bool` on the panel.
- **`loading_error` readers** (any error display logic) → match on `dm.get_file_state()`; the `FileState::LoadError(String)` variant carries the message. No new accessor on `emDirModel` needed.

Both fields' write sites (in current `Cycle` body and `notice`) are deleted along with the panel-driven loading dispatch.

### Why this shape, not Shape A (`T: FileModelOps + Default`)

Shape A would require `self.data: Option<T>` populated on Waiting→Loading transition. For `emDirModel`, the loader workspace (`emDirModelData`) needs to be available before loading starts (test fixtures, error reporting). `Option<T>` doesn't fit cleanly. Shape B keeps the loader as a non-optional sibling field. Below the observable surface; not a divergence per CLAUDE.md.

---

## Public API contract (immutable)

External callers (`emDirPanel.rs`, `emDirEntryPanel.rs`, `emFileManControlPanel.rs`, plus tests in `emDirModel.rs`) must continue to compile and pass without test edits, **except** for the explicitly listed deletions in `emDirPanel.rs`.

| Signature | Notes |
|---|---|
| `pub fn Acquire(ctx: &Rc<emContext>, name: &str) -> Rc<RefCell<emDirModel>>` | **Unchanged.** Engine registration happens lazily via `ensure_engine_registered`, called by callers after `Acquire`. |
| `pub fn get_file_state(&self) -> FileState` | Returns `self.file_model.GetFileState().clone()`. Behavior identical. |
| `pub fn try_start_loading(&mut self) -> Result<(), String>` | Delegates to `self.data` (now implements `FileModelOps`). Behavior identical. |
| `pub fn try_continue_loading(&mut self) -> Result<bool, String>` | Delegates to `self.data`. Behavior identical. |
| `pub fn quit_loading(&mut self)` | Delegates to `self.data`. |
| `pub fn calc_file_progress(&self) -> f64` | Delegates to `self.data`. |
| `pub fn GetEntryCount(&self) -> usize` | Delegates to `self.data`. |
| `impl FileModelState for emDirModel` | Trait impl signature unchanged. |
| `pub state: FileState` field | **REMOVED.** Writers in `emDirPanel.rs` (lines 344, 363, 372, 657) are deleted. Readers go through `get_file_state()`. This is the only contract-breaking change, and it's the symptom we're fixing — the writes only existed because the Rust port lacked engine-driven loading. |

Tests in `emDirModel.rs` itself drive loading via `while !done { try_continue_loading() }` loops (lines 340, 354, 368, 380, 429, 449, 471). These call `emDirModel`'s public delegating methods, **not** the inner `emFileModel::Cycle` — so they continue to work. They do not exercise the new engine path; the two new tests (below) cover that.

External callers of `dm.state` field: only `emDirPanel.rs` (4 sites). All are deleted by this fix.

---

## Edit budget (hard cap)

**Allowed files (whitelist):**

1. `crates/emcore/src/emFileModel.rs` — add `Cycle`, `UpdateFileProgress`.
2. `crates/emfileman/src/emDirModel.rs` — refactor to composition; add `emDirModelEngine` shim; add `ensure_engine_registered`; remove `pub state` field; add proof-of-fix tests.
3. `crates/emfileman/src/emDirPanel.rs` — replace `Cycle` body with observe-only shape; remove `loading_done`/`loading_error` fields; call `ensure_engine_registered` after `Acquire`; update the test at line 656.
4. `crates/emfileman/Cargo.toml` — only if `tempfile` dev-dep is needed for the proof-of-fix tests.

**Forbidden files** — every file not in the whitelist. Specifically, do **not** edit:

- `crates/emcore/src/emPriSchedAgent.rs` (PSAgent deferred from F017 scope)
- `crates/emcore/src/emScheduler.rs`
- `crates/emcore/src/emEngineCtx.rs` (`IsTimeSliceAtEnd` already exists)
- Any other file under `crates/emcore/src/`
- `crates/emfileman/src/emDirEntryPanel.rs`
- `crates/emfileman/src/emFileManControlPanel.rs`
- Any test file outside `emDirModel.rs`, `emFileModel.rs`, and `emDirPanel.rs`

**Diff-size cap:** combined `+` and `−` lines ≤ 700. Expected: ~500 lines.

**Stop-and-ask trigger:** if implementing this requires editing any forbidden file, **STOP. Do not edit. Report what you wanted to edit and why.** This is the most important rule in the plan. The v2 subagent edited 14 forbidden files; do not repeat that.

Cases where the implementer might think a forbidden edit is required:

- "I need to extend `PriSchedModel`'s callback signature." → Stop. PSAgent integration is out of scope; the plan deferred it.
- "I need to make a private item public in emcore." → Stop. The plan committed all needed accessors (`GetFileStateSignal`, `IsSignaled`, `EngineCtx::scheduler`); none are private. If something else looks private, surface it.
- "A test in another crate is calling an old `emDirModel` signature." → Stop. The contract above preserves all public signatures; if a test breaks, the contract is broken — surface it, don't edit the test.
- "Clippy is complaining about a file I didn't change." → Pre-existing failure. Document it; do not "fix" it.
- "I need to add a `loading_error` accessor on `emDirModel`." → Don't. The plan resolved this: `FileState::LoadError(String)` already carries the message; readers match on `get_file_state()`.

---

## Anti-patterns

- ❌ Don't change `step_loading`'s body. It's correct; the new `Cycle` calls it from a loop.
- ❌ Don't add new throttle constants. C++ has none beyond the 50 ms deadline.
- ❌ Don't add `#[allow(...)]` to silence warnings. Fix the cause.
- ❌ Don't keep the `DIVERGED` note at `emDirModel.rs:207-210` "as a fallback." If the refactor builds, the language-forced claim is disproven and the note must be deleted.
- ❌ Don't change `TIME_SLICE_DURATION`.
- ❌ Don't add PSAgent/PriSchedModel integration. It's deferred. The plan is explicit.
- ❌ Don't keep `loading_done` "for now" with a TODO. Remove it; the model owns loading state.
- ❌ Don't preserve the direct `try_continue_loading` call in `emDirPanel` as a "fallback." The new engine is the only authority.
- ❌ Don't paper over pre-existing failures. Document and proceed.

---

## Proof-of-fix tests (added in `emDirModel.rs`)

1. **`cycle_loads_multiple_entries_within_one_slice`** — build a 60-file `tempfile::TempDir`. Construct an `emDirModel`, register its engine via `ensure_engine_registered`, drive a single Cycle invocation with an `EngineCtx` whose deadline is 100 ms in the future. Assert `GetEntryCount() > 1` (ideally > 10). **Proves the inner do-while loop works.**

2. **`cycle_runs_at_least_one_step_when_deadline_passed`** — build a fixture, prime the model into `LoadingEntries` phase via repeated tight Cycle calls, then drive a single Cycle with deadline already in the past. Assert `entries_after > entries_before`. **Proves the do-while "execute at least once" semantic.**

Both tests construct an `EngineCtx` directly (or via test helpers); existing tests that drive `try_continue_loading` in a loop continue to pass unchanged because they don't go through the engine path.

---

## Verification (workspace-wide; runs once, after both pieces ship)

| Command | Pass criterion |
|---|---|
| `cargo build --workspace` | Clean build. **The load-bearing check.** |
| `cargo clippy --workspace --lib -- -D warnings` | Clean (the pre-existing `emFpPlugin.rs:201` failure is a `--all-targets` failure; lib-only must be clean). |
| `cargo-nextest ntr --no-fail-fast` | All tests pass except the 2 known `emcore::plugin_invocation` failures. |
| `cargo xtask annotations` | Clean (DIVERGED note removed, RUST_ONLY note on `emDirModelEngine` accepted). |
| `git diff --stat` | Only files in the whitelist appear. |
| Combined diff line count | Total `+` plus `−` ≤ 700. |
| `rg '^\s*pub state\b' crates/emfileman/src/emDirModel.rs` | No hits. (Confirms field removed.) |
| `rg 'try_continue_loading\|try_start_loading' crates/emfileman/src/emDirPanel.rs` | No hits. (Confirms panel reverted to observe-only.) |
| `rg 'loading_done\|loading_error' crates/emfileman/src/emDirPanel.rs` | No hits in field declarations. (Confirms duplicated state removed.) |

The two new proof-of-fix tests must pass.

**Sanity check during implementation:** run `cargo check` after the refactor compiles; do not run the full verification table until everything is in place. If `cargo check` reveals breakage in a forbidden file, STOP — that's a contract violation, not a fixable error.

**GUI verification (after the table passes):** orchestrator-driven. Build release binary; zoom into `/usr/bin`; the "Loading: NN%" overlay must not be readable. This closes F017.

---

## Reporting requirements

The implementation report must include:

1. **Diff stat output** (`git diff --stat`). If any file outside the whitelist appears: STOP, surface immediately, do not commit.
2. **For each allowed file**: lines added/removed, summary of change in 1–2 sentences.
3. **For each new test**: name, what it asserts, pass/fail.
4. **Verification results**: pass/fail for each command in the table; full output of `cargo build --workspace` if it failed.
5. **Public API contract check**: confirm the 8 preserved signatures are unchanged; confirm the `pub state` deletion is the only contract change.
6. **Architectural decisions made**: the exact ctx type chosen for `ensure_engine_registered`, where the engine registration happens in `emDirPanel` (which method, which line), and whether `update_children` was moved to `notice` or remained in `Cycle`.
7. **Anything noticed but not changed**: pre-existing issues, smells, or follow-ups for later issues. List, don't fix.

If any verification step fails, **stop**. Do not "fix forward" by editing files outside the whitelist or by silencing warnings.

---

## Why this plan is tighter than v2

| v2 gap | v3 closure |
|---|---|
| No edit budget | Whitelist of 4 files; explicit forbidden list with PSAgent and emEngineCtx named; 700-line cap; stop-and-ask trigger as "the most important rule." |
| No public-API contract | Table of every preserved signature; explicit `pub state` deletion as the only contract change; named caller sites and what happens to them. |
| Architecture deferred ("Shape A or B") | Shape committed: composition + shim + lazy registration. Field-level struct definition. Rationale for skipping Shape A documented. |
| Verification per-crate, not workspace | `cargo build --workspace` is the load-bearing check; `rg` assertions verify specific deletions. |
| No stop-and-ask trigger | Explicit triggers with concrete examples of cases that look like they need a forbidden edit but don't. |
| Three steps land together (claimed but not enforced) | One combined step. PSAgent (which v2 implicitly required) is explicitly deferred with rationale. |
| PSAgent integration glossed over | Investigated; deferred with rationale (Rust callback API incompatibility). |
| Engine ownership puzzle unaddressed | Shim pattern committed, with `RUST_ONLY` annotation rationale. |
| `pub state` mirror would have been impossible | Field deleted entirely; the writers in `emDirPanel.rs` are removed in the same change. |
