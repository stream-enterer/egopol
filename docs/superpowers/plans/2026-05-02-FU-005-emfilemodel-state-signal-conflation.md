# FU-005 — emFileModel state-signal conflation fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two bugs at the emFileModel/emRecFileModel boundary: (1) `emRecFileModel::GetFileStateSignal` returns null instead of a real signal, and (2) emRecFileModel state-mutating methods only fire `ChangeSignal`, never `FileStateSignal`.

**Architecture:** Phase 1 renames the misleadingly-named base-class field (`change_signal` → `file_state_signal`) and gives `emRecFileModel` its own lazy-allocated `file_state_signal` to return from the trait impl (plumbing-only — no fires yet, so behavior is unchanged from "always null" to "real but un-fired"). Phase 2 adds parallel `FileStateSignal` fires alongside every existing `signal_change(ectx)` call site in `emRecFileModel`, activating previously inert subscriptions. Phase 3 cleans up downstream UPSTREAM-GAP comments and tests. **Phase 1 must merge before Phase 2** so subscribers connect to a real signal id before fires start.

**Tech Stack:** Rust (port of C++ Eagle Mode 0.96.4 emCore). `SignalCtx`/`EngineCtx` typed contexts; `slotmap`-backed `SignalId` with null-safe `connect`/`fire`.

---

## Spec adaptation note (non-obvious decision recorded up front)

The spec proposes that `emRecFileModel::GetFileStateSignal` delegate via `self.file_model.GetFileStateSignal()`. **That call doesn't compile in this tree.** `emRecFileModel<T>` in `crates/emcore/src/emRecFileModel.rs:17-18` is a documented standalone port — it does **not** wrap `emFileModel<T>`. There is no `file_model` field to delegate through.

**Adapted design** (preserves the spec's intent — return a real, non-null signal — and matches existing patterns in the same file): give `emRecFileModel<T>` its own lazy-allocated `file_state_signal: Cell<SignalId>`, mirroring the existing `change_signal: Cell<SignalId>` pattern (line 32) and its `GetChangeSignal(&self, ectx)` accessor (lines 56-65) and `signal_change(&self, ectx)` fire helper (lines 76-81). The trait-level `GetFileStateSignal(&self) -> SignalId` (no `ectx`) returns the current cell value, which is null until first promoted via a new `ensure_file_state_signal(&self, ectx)` accessor that callers (subscribers) invoke at first-Cycle wake-up. This matches how `emFilePanel::ensure_vir_file_state_signal` already works.

This is **not a forced divergence from C++** — C++ has eager allocation of FileStateSignal. The lazy form here is **dependency-internal**: every other signal in `emRecFileModel` (the sibling `change_signal`) already lazy-allocates because `emRecFileModel::new()` has no `EngineCtx` reach (D-006). Eager allocation would require restructuring the constructor — a wider change than this fix warrants. The lazy form is observably equivalent: subscribers go through `ensure_*` at first Cycle and get a real id before any fires can happen, so the rename-then-fire ordering still holds.

The 5 base-class rename sites in `emFileModel<T>` (lines 64, 117, 137, 168, 525 per spec) **are** still rename-only; only the `emRecFileModel` side changes from "delegate to base" to "own lazy field".

---

## File Structure

**Modified files:**
- `crates/emcore/src/emFileModel.rs` — rename field `change_signal` → `file_state_signal` (5 internal sites; field is private so no external callers affected).
- `crates/emcore/src/emRecFileModel.rs` — add `file_state_signal: Cell<SignalId>` field, `ensure_file_state_signal(&self, ectx)` accessor, `signal_file_state(&self, ectx)` fire helper; replace null-return in trait impl with `self.file_state_signal.get()`; add 9 parallel fire sites in state-mutating methods.
- `crates/emstocks/src/emStocksFileModel.rs` — replace UPSTREAM-GAP block (lines ~146-166) with simple delegation doc-comment; add `ensure_file_state_signal(&self, ectx)` pass-through.
- `crates/emstocks/src/emStocksPricesFetcher.rs` — drop UPSTREAM-GAP block on field doc (lines ~75-83); update first-Cycle subscribe (lines ~421-435) to call `ensure_file_state_signal(ectx)` so the connect wires into the now-real signal.
- `crates/eaglemode/tests/behavioral/file_model.rs` (and any other tests touching `change_signal_for_test` or asserting null `GetFileStateSignal`) — adjust test names/assertions to match new semantics. Verify in Phase 1 Task 1.5.

**Test files (new behavioral coverage):**
- `crates/emcore/tests/fu005_file_state_signal.rs` (NEW) — three tests: (a) `emRecFileModel::GetFileStateSignal` returns null until `ensure_file_state_signal`, then non-null and stable; (b) state-mutating methods fire FileStateSignal alongside ChangeSignal; (c) Save error transition fires FileStateSignal.

---

## Phase 1 — Rename + delegate (plumbing-only, no behavior change)

**Phase 1 Gate (must pass before any Phase 2 task starts):**
- All Phase 1 tasks complete and committed.
- `cargo check -p emcore` clean.
- `cargo clippy -p emcore -- -D warnings` clean.
- Behavioral test `crates/eaglemode/tests/behavioral/file_model.rs` still passes (rename should be transparent).
- New test in `crates/emcore/tests/fu005_file_state_signal.rs` Phase 1 portion (`get_file_state_signal_returns_null_until_ensured`) passes.
- **Pre-flight to Phase 2:** confirm no caller anywhere else has begun depending on FileStateSignal fires. (Phase 2 changes observable behavior; Phase 1 must land first so subscribers wire to the real id before fires start.)

### Task 1.0: Pre-flight verification of rename surface

**Files:**
- Read-only: `crates/emcore/src/emFileModel.rs`, all of `crates/`.

- [ ] **Step 1: Confirm `change_signal` field on `emFileModel<T>` has no external readers**

Run: `rg -n '\.change_signal\b' crates/`
Expected output (only the 3 internal lines in emFileModel.rs plus the unrelated `emRecFileModel::change_signal` field references):
```
crates/emcore/src/emFileModel.rs:64:        self.change_signal
crates/emcore/src/emFileModel.rs:168:        self.change_signal
crates/emcore/src/emFileModel.rs:525:            ctx.fire(self.change_signal);
crates/emcore/src/emRecFileModel.rs:32:    change_signal: Cell<SignalId>,
crates/emcore/src/emRecFileModel.rs:48:            change_signal: Cell::new(SignalId::null()),
crates/emcore/src/emRecFileModel.rs:57:        let cur = self.change_signal.get();
crates/emcore/src/emRecFileModel.rs:60:            self.change_signal.set(new_id);
crates/emcore/src/emRecFileModel.rs:67:    /// Test-only accessor for the raw `change_signal` slot (without allocating).
crates/emcore/src/emRecFileModel.rs:69:    pub fn change_signal_for_test(&self) -> SignalId {
crates/emcore/src/emRecFileModel.rs:70:        self.change_signal.get()
crates/emcore/src/emRecFileModel.rs:74:    /// D-007. No-op when `change_signal` is null (matches C++ ...
crates/emcore/src/emRecFileModel.rs:77:        let s = self.change_signal.get();
```
If any other crate accesses `emFileModel<T>.change_signal` directly, stop and update this task with the additional sites. The `emRecFileModel` matches are a separate field and are out of scope for this task.

- [ ] **Step 2: Confirm signature of `emFileModel::new` constructor parameter**

Run: `rg -n 'pub fn new' crates/emcore/src/emFileModel.rs`
Read the constructor (line 131-149). Confirm parameter name is `signal_id: SignalId` and only the field initialization at line 137 references it. Note: parameter name will also be renamed to `file_state_signal: SignalId` in Task 1.1 to keep call sites self-documenting.

- [ ] **Step 3: List `emFileModel::new` callers**

Run: `rg -n 'emFileModel.*::new\(' crates/ tests/`
Expected: ~17 sites across `crates/eaglemode/tests/behavioral/file_model.rs`, `crates/emcore/tests/f018_iv3_svpchoice_invalidation.rs`, `crates/emfileman/src/emDirModel.rs`, `crates/eaglemode/tests/unit/model.rs`. All pass `signal_id`/`change`/`sig` positionally; renaming the parameter does not break call sites (positional). No call-site changes required.

- [ ] **Step 4: No commit — verification only**

### Task 1.1: Rename `emFileModel<T>.change_signal` → `file_state_signal`

**Files:**
- Modify: `crates/emcore/src/emFileModel.rs:64,117,131,137,168,525`

- [ ] **Step 1: Apply rename via Edit tool, 5 sites**

Site 1 (line 64) — trait impl body:
```rust
    fn GetFileStateSignal(&self) -> SignalId {
        self.file_state_signal
    }
```

Site 2 (line 117) — struct field:
```rust
    file_state_signal: SignalId,
```

Site 3 (line 131) — ctor parameter:
```rust
    pub fn new(path: PathBuf, file_state_signal: SignalId) -> Self {
```

Site 4 (line 137) — ctor body:
```rust
            file_state_signal,
```

Site 5 (line 168) — public accessor body:
```rust
    pub fn GetFileStateSignal(&self) -> SignalId {
        self.file_state_signal
    }
```

Site 6 (line 525) — load time-slice fire:
```rust
            ctx.fire(self.file_state_signal);
```

- [ ] **Step 2: Verify compile**

Run: `cargo check -p emcore`
Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Verify no surviving `change_signal` references in emFileModel.rs**

Run: `rg -n 'change_signal' crates/emcore/src/emFileModel.rs`
Expected: empty output.

- [ ] **Step 4: Run emcore lib tests**

Run: `cargo test -p emcore --lib`
Expected: PASS (pre-existing tests unchanged).

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emFileModel.rs
git commit -m "refactor(emFileModel): rename change_signal field to file_state_signal

Clarifies the C++ correspondence (emFileModel::FileStateSignal) and
disambiguates from the separately-named change_signal field on
emRecFileModel. Field is private; no call-site changes. Plumbing-only.

Phase 1.1 of FU-005."
```

### Task 1.2: Add `file_state_signal` cell, accessor, and fire helper to `emRecFileModel`

**Files:**
- Modify: `crates/emcore/src/emRecFileModel.rs:32,48,76-81`

- [ ] **Step 1: Add field at line 32 (alongside existing `change_signal`)**

After the existing `change_signal: Cell<SignalId>,` line 32, insert:
```rust
    /// Port of inherited C++ `emFileModel::FileStateSignal` (FU-005).
    /// Lazy-allocated on first `ensure_file_state_signal(ectx)` call; null
    /// until then. Mirrors the sibling `change_signal` lazy pattern (D-006:
    /// `emRecFileModel::new()` has no `EngineCtx` reach for eager alloc).
    file_state_signal: Cell<SignalId>,
```

- [ ] **Step 2: Add field initializer in `new()` at line 48**

After `change_signal: Cell::new(SignalId::null()),` add:
```rust
            file_state_signal: Cell::new(SignalId::null()),
```

- [ ] **Step 3: Add `ensure_file_state_signal` accessor and `signal_file_state` fire helper**

Insert after the existing `signal_change` method (line 81), before `pub fn GetFileState`:
```rust
    /// Port of inherited C++ `emFileModel::GetFileStateSignal()` with lazy
    /// allocation (FU-005). Allocates on first call; returns the live id
    /// thereafter. Subscribers call this at first-Cycle subscribe time so
    /// the connect wires into a real id before any fires can occur. Mirrors
    /// `GetChangeSignal` (line 56) and `emFilePanel::ensure_vir_file_state_signal`.
    pub fn ensure_file_state_signal(&self, ectx: &mut impl SignalCtx) -> SignalId {
        let cur = self.file_state_signal.get();
        if cur.is_null() {
            let new_id = ectx.create_signal();
            self.file_state_signal.set(new_id);
            new_id
        } else {
            cur
        }
    }

    /// Test-only accessor for the raw `file_state_signal` slot (without allocating).
    #[doc(hidden)]
    pub fn file_state_signal_for_test(&self) -> SignalId {
        self.file_state_signal.get()
    }

    /// Synchronous fire of `file_state_signal` (FU-005). No-op when null
    /// (matches C++ `emSignal::Signal()` with zero subscribers per D-007).
    /// Called alongside `signal_change` at every state-mutating site.
    pub fn signal_file_state(&self, ectx: &mut impl SignalCtx) {
        let s = self.file_state_signal.get();
        if !s.is_null() {
            ectx.fire(s);
        }
    }
```

- [ ] **Step 4: Replace null-return trait impl at line 366**

Replace the existing block (lines 358-368, the doc comment + `fn GetFileStateSignal`) with:
```rust
    /// Port of inherited C++ `emFileModel::GetFileStateSignal` (FU-005).
    /// Returns the lazy-allocated `file_state_signal` id, or null if no
    /// subscriber has called `ensure_file_state_signal(ectx)` yet. Once
    /// promoted, the id is stable for the lifetime of this model.
    fn GetFileStateSignal(&self) -> SignalId {
        self.file_state_signal.get()
    }
```

- [ ] **Step 5: Verify compile**

Run: `cargo check -p emcore`
Expected: 0 errors, 0 warnings.

- [ ] **Step 6: Run emcore tests**

Run: `cargo test -p emcore --lib`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emRecFileModel.rs
git commit -m "feat(emRecFileModel): add lazy file_state_signal field + accessors

Adds file_state_signal: Cell<SignalId> with ensure_file_state_signal
accessor and signal_file_state fire helper, mirroring the existing
change_signal lazy pattern. Replaces the null-returning trait impl
of GetFileStateSignal with a read of the cell.

Phase 1 plumbing only — no fires added yet. Subscribers that called
ensure_file_state_signal at first Cycle will now receive a real id
instead of null. Behavior change (fires) lands in Phase 2.

Phase 1.2 of FU-005."
```

### Task 1.3: Write Phase 1 behavioral test — null-until-ensured semantics

**Files:**
- Create: `crates/emcore/tests/fu005_file_state_signal.rs`

- [ ] **Step 1: Write the failing test**

```rust
//! FU-005: emRecFileModel file-state-signal lifecycle and fire coverage.
//!
//! Phase 1 establishes that `GetFileStateSignal` returns a real (non-null)
//! id after `ensure_file_state_signal` is called at first Cycle. Phase 2
//! adds the fire-side coverage (separate test functions below).

use std::path::PathBuf;

use emcore::emFileModel::FileModelState;
use emcore::emRecFileModel::emRecFileModel;
use emcore::emRecRecord::Record;
use emcore::emSignal::SignalId;
use emcore::test_support::{TestEngineCtx, TestScheduler};

#[derive(Default)]
struct DummyRec;
impl Record for DummyRec {
    fn from_rec(_r: &emcore::emRecParser::Rec) -> Result<Self, String> { Ok(Self) }
    fn to_rec(&self) -> emcore::emRecParser::Rec { emcore::emRecParser::Rec::default() }
    fn SetToDefault(&mut self) { *self = Self; }
}

#[test]
fn get_file_state_signal_returns_null_until_ensured() {
    let m: emRecFileModel<DummyRec> = emRecFileModel::new(PathBuf::from("/tmp/fu005.rec"));

    // Pre-ensure: trait impl returns null per the lazy invariant.
    assert_eq!(
        m.GetFileStateSignal(),
        SignalId::null(),
        "FileStateSignal must be null before ensure_file_state_signal is called"
    );
    assert!(
        m.file_state_signal_for_test().is_null(),
        "raw cell slot must be null pre-ensure"
    );

    // Promote via the lazy accessor.
    let mut sched = TestScheduler::new();
    let mut ectx = TestEngineCtx::new(&mut sched);
    let id1 = m.ensure_file_state_signal(&mut ectx);
    assert!(!id1.is_null(), "ensure_file_state_signal must return a real id");

    // Idempotent: second call returns the same id.
    let id2 = m.ensure_file_state_signal(&mut ectx);
    assert_eq!(id1, id2, "ensure_file_state_signal must be idempotent");

    // Trait impl now returns the real id.
    assert_eq!(
        m.GetFileStateSignal(),
        id1,
        "post-ensure, GetFileStateSignal must return the live id"
    );
}
```

**Note:** The exact path for `TestEngineCtx`/`TestScheduler` may differ — verify before writing by running `rg -n 'pub struct TestEngineCtx|pub struct TestScheduler' crates/emcore/`. If they live in a non-public path, use whatever the file-model behavioral test in `crates/eaglemode/tests/behavioral/file_model.rs` uses (read its `use` block and copy the import). If `Record::SetToDefault` signature doesn't match, mirror what `crates/emstocks/src/emStocksRec.rs` does for the `emStocksRec: Record` impl. Adapt the `DummyRec` impl to whatever the trait actually requires.

- [ ] **Step 2: Run the test — expect FAIL with compile or assertion errors initially**

Run: `cargo test -p emcore --test fu005_file_state_signal get_file_state_signal_returns_null_until_ensured`
Expected: PASS (Tasks 1.1 + 1.2 already implemented the support). If FAIL, fix before committing.

- [ ] **Step 3: Adjust import paths if test compile fails**

If the test won't compile, fix the imports based on the verified pub paths (Step 1 note). Re-run the test.

- [ ] **Step 4: Verify test passes**

Run: `cargo test -p emcore --test fu005_file_state_signal`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/tests/fu005_file_state_signal.rs
git commit -m "test(emRecFileModel): FU-005 Phase 1 — null-until-ensured invariant

Asserts GetFileStateSignal returns null before
ensure_file_state_signal is called, and a stable real id after.
Phase 1.3 of FU-005."
```

### Task 1.4: Audit existing tests for `GetFileStateSignal == null` assertions

**Files:**
- Read-only: all tests under `crates/`

- [ ] **Step 1: Search for tests asserting null FileStateSignal**

Run: `rg -n 'GetFileStateSignal' crates/ --type rust`
Inspect each call site. Tests that assert the result `is_null()` after Phase 1 should be re-classified:
- If the assertion happens **before** any `ensure_file_state_signal` call, the assertion remains true under the new semantics and stays as-is.
- If the assertion happens **after** subscriber wiring or expected the system-wide signal to always be null, it must be deleted or updated in Task 3.x.

- [ ] **Step 2: Search for `change_signal_for_test` references on emFileModel**

Run: `rg -n 'change_signal_for_test' crates/`
The only matches should be on `emRecFileModel` (existing helper) — `emFileModel<T>` does not have a `_for_test` accessor. If a test references one on `emFileModel<T>`, it doesn't exist and is unrelated.

- [ ] **Step 3: Document findings in commit message**

If any test will need updating in Phase 2, list them in the commit message of Task 2.4. No commit in this task; it's a verification-only audit.

### Task 1.5: Phase 1 gate — full test sweep

- [ ] **Step 1: Run emcore tests**

Run: `cargo test -p emcore`
Expected: all pass.

- [ ] **Step 2: Run dependent crate tests (touched compile surface only)**

Run: `cargo check -p emfileman -p emstocks -p eaglemode`
Expected: 0 errors. (No emstocks code changed yet — change happens in Phase 3.)

- [ ] **Step 3: Run clippy with deny warnings**

Run: `cargo clippy -p emcore -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 4: Confirm no behavior change observable**

Run the file-model behavioral suite:
`cargo test -p eaglemode --test behavioral`
Expected: all existing tests pass with no changes. Phase 1 is plumbing-only.

- [ ] **Step 5: No commit — gate verification only**

If any step fails, fix in a follow-up task before proceeding to Phase 2.

---

## Phase 2 — Wire FileStateSignal fires at state-transition sites (BEHAVIOR CHANGE)

**Phase 2 Gate (must pass before Phase 3):**
- All Phase 2 tasks complete and committed.
- New behavioral test `state_mutation_fires_both_signals` passes.
- Existing `cargo test -p emcore` and `cargo test -p eaglemode --test behavioral` pass (with documented assertion updates if any).
- `cargo clippy -p emcore -- -D warnings` clean.

**Important: Phase 1 MUST be merged before Phase 2 starts.** Subscribers connect at first-Cycle via `ensure_file_state_signal(ectx)`; if Phase 2 fires land before subscribers can wire to the real id, fires go to a dead signal. Phase 1 makes the wire real; Phase 2 makes it carry signal traffic.

### Task 2.0: Re-verify Phase 1.5 of spec — count `signal_change` sites

**Files:**
- Read-only: `crates/emcore/src/emRecFileModel.rs`

- [ ] **Step 1: Enumerate state-mutating call sites**

Run: `rg -n 'self\.signal_change\(ectx\)' crates/emcore/src/emRecFileModel.rs`
Expected: 9 sites (per spec): lines 138, 156, 163, 169, 184, 205, 224, 234, 248.
If count differs, update Task 2.2 with the actual line numbers and ensure each site receives a parallel fire.

- [ ] **Step 2: Verify no orphan state-mutation sites without `signal_change`**

Run: `rg -n 'self\.state =' crates/emcore/src/emRecFileModel.rs`
Inspect each match. Each should be either followed by a `signal_change(ectx)` call within the same method, or be in a private helper called from a method that issues `signal_change(ectx)` afterward (e.g., `do_step_loading` is private and its caller `TryLoad` issues `signal_change` after the loop). Flag any orphan to Task 2.3 for additional `signal_file_state` coverage.

- [ ] **Step 3: No commit — verification only**

### Task 2.1: Write the failing fire-coverage test

**Files:**
- Modify: `crates/emcore/tests/fu005_file_state_signal.rs`

- [ ] **Step 1: Append failing test**

Add at the bottom of the file:
```rust
#[test]
fn try_load_fires_file_state_signal() {
    use std::io::Write;

    // Set up a real on-disk rec file so TryLoad reaches Loaded state.
    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("fu005_load.rec");
    {
        let mut f = std::fs::File::create(&path).expect("create");
        // Empty rec — DummyRec::from_rec accepts the default Rec.
        writeln!(f, "").ok();
    }

    let mut sched = TestScheduler::new();
    let mut ectx = TestEngineCtx::new(&mut sched);

    let m: emRecFileModel<DummyRec> = emRecFileModel::new(path);
    // Promote both signals (mirrors first-Cycle subscriber wiring).
    let change_sig = m.GetChangeSignal(&mut ectx);
    let state_sig = m.ensure_file_state_signal(&mut ectx);
    assert!(!change_sig.is_null());
    assert!(!state_sig.is_null());

    // Reset the scheduler's fire log so we count only TryLoad's fires.
    sched.clear_fired();

    let mut m_mut = m;
    m_mut.TryLoad(&mut ectx);

    let fired = sched.fired_signals();
    assert!(
        fired.contains(&change_sig),
        "TryLoad must fire ChangeSignal (existing behavior)"
    );
    assert!(
        fired.contains(&state_sig),
        "TryLoad must fire FileStateSignal (FU-005 new behavior)"
    );
}
```

**Note on test scaffolding:** if `TestScheduler::clear_fired`/`fired_signals` accessors don't exist, use whatever introspection the existing emcore test scaffolding offers (read `crates/emcore/src/test_support.rs` or the `signaling.rs` test in the same dir). If no fire-log facility exists, build a minimal one inline by passing a closure-based `SignalCtx` mock or by using whatever capture pattern the `D-007` tests already use (search `rg -n 'fn cycle_fires|assert.*fired' crates/emcore/`). The test framework choice does not affect the implementation — only the assertion shape.

If `tempfile` isn't a dev-dep of `emcore`, either (a) add it (`cargo add --dev tempfile -p emcore`), or (b) write to `std::env::temp_dir()` with a UUID-suffixed filename and clean up at the end of the test.

- [ ] **Step 2: Run the test — expect FAIL**

Run: `cargo test -p emcore --test fu005_file_state_signal try_load_fires_file_state_signal`
Expected: FAIL — `TryLoad` currently fires only `ChangeSignal`, so `fired.contains(&state_sig)` fails.

- [ ] **Step 3: No commit yet — test stays failing until Task 2.2 lands the fix.**

### Task 2.2: Add parallel `signal_file_state` calls at all 9 sites

**Files:**
- Modify: `crates/emcore/src/emRecFileModel.rs:138,156,163,169,184,205,224,234,248`

- [ ] **Step 1: Apply the pattern at each site**

For each line listed in Task 2.0 Step 1, **immediately after** the existing `self.signal_change(ectx);` line, insert:
```rust
        self.signal_file_state(ectx);
```
Match the exact indentation of the preceding `signal_change` call.

Concretely (assuming spec line numbers hold; re-verify after Phase 1 commits may have shifted them):

- Line 138 (TryLoad final fire) — `self.signal_file_state(ectx);` after.
- Line 156 (Save: parent dir create_dir_all error) — same.
- Line 163 (Save: write error) — same.
- Line 169 (Save: try_fetch_date error) — same.
- Line 184 (Save: success/TooCostly trailing fire) — same.
- Line 205 (update: discriminant-changed transition) — same.
- Line 224 (hard_reset trailing fire) — same.
- Line 234 (clear_save_error: SaveError → Unsaved) — same.
- Line 248 (set_unsaved_state_internal: Loaded/SaveError → Unsaved) — same.

- [ ] **Step 2: Verify compile**

Run: `cargo check -p emcore`
Expected: 0 errors.

- [ ] **Step 3: Run the FU-005 fire-coverage test — expect PASS**

Run: `cargo test -p emcore --test fu005_file_state_signal try_load_fires_file_state_signal`
Expected: PASS.

- [ ] **Step 4: Run the broader emcore test suite to catch regressions**

Run: `cargo test -p emcore`
Expected: PASS. If any pre-existing test asserted "ChangeSignal fired exactly once" or "no other signals fired during X", update those assertions to match new (correct) behavior. Document each updated test in the commit message.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emRecFileModel.rs crates/emcore/tests/fu005_file_state_signal.rs
git commit -m "fix(emRecFileModel): fire FileStateSignal alongside ChangeSignal

Adds parallel signal_file_state(ectx) calls at all 9 existing
signal_change(ectx) call sites covering TryLoad, Save (success
+ 3 error paths), update, hard_reset, clear_save_error, and
set_unsaved_state_internal. Mirrors C++ fires of FileStateSignal
at every state transition. Subscribers that wired via
ensure_file_state_signal in Phase 1 now receive wake-ups.

Phase 2 of FU-005. Behavior change. See test_impact below.

Test impact: <list any updated assertions here>"
```

### Task 2.3: Cover orphan state-mutation sites if any

**Files:**
- Modify: `crates/emcore/src/emRecFileModel.rs` (only if Task 2.0 Step 2 found orphans)

- [ ] **Step 1: For each orphan site identified in Task 2.0 Step 2, add a paired fire**

If `do_step_loading` (a private helper without `ectx`) sets state but the caller `TryLoad` already issues `signal_change`+`signal_file_state` on completion, no action — Phase 2's site-2.2 covers it transitively.

If a truly orphan site exists (state mutated without any caller-issued `signal_change`), thread `ectx` into the site or move the mutation. Document the site, the threading approach, and the rationale in the commit. **Do not introduce new polling intermediaries** (per CLAUDE.md: a `Cell` set here and drained elsewhere is a one-tick drift bug).

- [ ] **Step 2: If no orphans, skip this task and note "no orphans found per Task 2.0" in the Phase 2 gate report.**

- [ ] **Step 3: Commit (if changes made)**

```bash
git add crates/emcore/src/emRecFileModel.rs
git commit -m "fix(emRecFileModel): cover orphan state-mutation site at <line>

<rationale>

Phase 2.3 of FU-005."
```

### Task 2.4: Phase 2 gate — full test sweep + clippy

- [ ] **Step 1: Full nextest run**

Run: `cargo-nextest ntr` (project alias for `cargo nextest run`)
Expected: all tests pass. If any fail with previously-correct assertions about "FileStateSignal subscription is inert" or "subscriber receives 0 wake-ups," update them to match the new fires; document each in the commit message. Per CLAUDE.md: "Tests updated to match new behavior; no test asserts the old null-subscription behavior."

- [ ] **Step 2: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 3: Annotation check**

Run: `cargo xtask annotations`
Expected: clean. (Phase 3 will remove the obsolete UPSTREAM-GAP markers; if `xtask annotations` complains about them being orphaned now, defer to Phase 3.)

- [ ] **Step 4: No commit — gate only.**

---

## Phase 3 — Downstream cleanup (no behavior change)

**Phase 3 Gate:**
- All UPSTREAM-GAP comments referencing the FU-005 null-signal behavior removed.
- emstocks first-Cycle subscribe wires through `ensure_file_state_signal` so the connect lands on the real id.
- `cargo-nextest ntr` green; `cargo clippy -- -D warnings` clean; `cargo xtask annotations` clean.

### Task 3.1: Update `emStocksFileModel::GetFileStateSignal` and add `ensure_file_state_signal` pass-through

**Files:**
- Modify: `crates/emstocks/src/emStocksFileModel.rs:140-166` (verify exact range; spec said 146-166 but composition with sibling FU-001 may have shifted lines)

- [ ] **Step 1: Re-read the file to find the current location of the UPSTREAM-GAP block**

Run: `rg -n 'UPSTREAM-GAP|GetFileStateSignal|GetChangeSignal' crates/emstocks/src/emStocksFileModel.rs`
Find the doc-comment block that describes the FU-005 null-signal collapse and the `pub fn GetFileStateSignal` method that follows it.

- [ ] **Step 2: Replace the UPSTREAM-GAP doc block + method with a clean delegation**

Replace:
```rust
    /// Port of inherited C++ `emFileModel::GetFileStateSignal`. Delegates to
    /// the composed `emRecFileModel<emStocksRec>`.
    ///
    /// UPSTREAM-GAP: ... <full block> ...
    pub fn GetFileStateSignal(&self) -> SignalId {
        use emcore::emFileModel::FileModelState as _;
        self.file_model.GetFileStateSignal()
    }
```

With:
```rust
    /// Port of inherited C++ `emFileModel::GetFileStateSignal` (FU-005).
    /// Delegates to the composed `emRecFileModel<emStocksRec>` lazy-allocated
    /// `file_state_signal`. Returns null until a subscriber has called
    /// `ensure_file_state_signal(ectx)` at first Cycle.
    pub fn GetFileStateSignal(&self) -> SignalId {
        use emcore::emFileModel::FileModelState as _;
        self.file_model.GetFileStateSignal()
    }

    /// Port of inherited C++ `emFileModel::GetFileStateSignal()` lazy
    /// allocator (FU-005). First-Cycle subscriber pass-through to the
    /// composed `emRecFileModel`. Mirrors `GetChangeSignal`.
    pub fn ensure_file_state_signal(&self, ectx: &mut impl emcore::emEngineCtx::SignalCtx) -> SignalId {
        self.file_model.ensure_file_state_signal(ectx)
    }
```

- [ ] **Step 3: Verify compile**

Run: `cargo check -p emstocks`
Expected: 0 errors. If the `SignalCtx` import path differs in this file, use whatever the existing `GetChangeSignal` method imports (see ~5 lines above).

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksFileModel.rs
git commit -m "refactor(emStocksFileModel): drop FU-005 UPSTREAM-GAP; add ensure_file_state_signal

Replaces the UPSTREAM-GAP comment block with a clean delegation
doc-comment now that emRecFileModel exposes a real (lazy-allocated)
FileStateSignal. Adds an ensure_file_state_signal pass-through for
emStocksPricesFetcher's first-Cycle subscribe.

Phase 3.1 of FU-005."
```

### Task 3.2: Update `emStocksPricesFetcher` first-Cycle subscribe to use `ensure_file_state_signal`

**Files:**
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs:75-83,420-435` (verify exact lines)

- [ ] **Step 1: Re-read the relevant sections**

Run: `rg -n 'UPSTREAM-GAP|file_model_state_sig|GetFileStateSignal' crates/emstocks/src/emStocksPricesFetcher.rs`

- [ ] **Step 2: Drop the UPSTREAM-GAP block from the field doc-comment (~lines 75-83)**

Replace the comment that describes `file_model_state_sig`:
```rust
    /// Cached `emStocksFileModel::GetFileStateSignal` id captured at first
    /// `cycle()`. `None` until `subscribed_init` flips. Mirrors C++ ctor
    /// `AddWakeUpSignal(FileModel->GetFileStateSignal())`
    /// (emStocksPricesFetcher.cpp:39). UPSTREAM-GAP: the underlying signal
    /// id is `SignalId::default()` (null) in the standalone-port
    /// `emRecFileModel`; the connect call below is a no-op for null but the
    /// subscribe site is preserved per the upstream-gap convention so a
    /// future emRecFileModel promotion plugs in without callsite changes.
    file_model_state_sig: Option<SignalId>,
```

With:
```rust
    /// Cached `emStocksFileModel::GetFileStateSignal` id captured at first
    /// `cycle()` via `ensure_file_state_signal(ectx)`. `None` until
    /// `subscribed_init` flips. Mirrors C++ ctor
    /// `AddWakeUpSignal(FileModel->GetFileStateSignal())`
    /// (emStocksPricesFetcher.cpp:39).
    file_model_state_sig: Option<SignalId>,
```

- [ ] **Step 3: Update the first-Cycle subscribe block (~lines 420-435)**

Replace the subscribe block:
```rust
        // First-Cycle subscribe — D-006 deferred init.
        if !self.subscribed_init {
            // GetChangeSignal lazily allocates if needed; capture into the
            // option slot. GetFileStateSignal currently delegates to a null
            // SignalId per the UPSTREAM-GAP on emRecFileModel; connect is
            // null-safe so we still preserve the subscribe site for future
            // promotion.
            let change_sig = file_model.borrow().GetChangeSignal(ectx);
            let state_sig = file_model.borrow().GetFileStateSignal();
            ectx.connect(change_sig, eid);
            ectx.connect(state_sig, eid);
            self.file_model_change_sig = Some(change_sig);
            self.file_model_state_sig = Some(state_sig);
            self.subscribed_init = true;
        }
```

With:
```rust
        // First-Cycle subscribe — D-006 deferred init. Both signals are
        // lazy-allocated in emRecFileModel; the `ensure_*` accessors promote
        // the cells to real ids on first call (FU-005).
        if !self.subscribed_init {
            let change_sig = file_model.borrow().GetChangeSignal(ectx);
            let state_sig = file_model.borrow().ensure_file_state_signal(ectx);
            ectx.connect(change_sig, eid);
            ectx.connect(state_sig, eid);
            self.file_model_change_sig = Some(change_sig);
            self.file_model_state_sig = Some(state_sig);
            self.subscribed_init = true;
        }
```

- [ ] **Step 4: Verify compile and run emstocks tests**

Run: `cargo check -p emstocks && cargo test -p emstocks`
Expected: 0 errors; tests pass. Some tests may have asserted that `file_model_state_sig` was null after subscribed_init flipped; those assertions must be updated to match the new (correct) "real id" behavior. Document any test updates in the commit message.

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksPricesFetcher.rs
git commit -m "refactor(emStocksPricesFetcher): drop FU-005 UPSTREAM-GAP; subscribe via ensure_file_state_signal

Updates the first-Cycle subscribe to call ensure_file_state_signal
(promoting the lazy cell to a real id) instead of the null-returning
GetFileStateSignal. The connect now lands on a live wire and the
fetcher Cycle wakes on FileStateSignal as in C++
(emStocksPricesFetcher.cpp:39).

Phase 3.2 of FU-005."
```

### Task 3.3: Final gate — full repo test + lint + annotation sweep

- [ ] **Step 1: Full nextest run**

Run: `cargo-nextest ntr`
Expected: all pass.

- [ ] **Step 2: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: 0 warnings.

- [ ] **Step 3: Annotations**

Run: `cargo xtask annotations`
Expected: clean. No surviving `UPSTREAM-GAP:` markers referencing FU-005.

- [ ] **Step 4: Confirm no production caller returns null `GetFileStateSignal`**

Run: `rg -n 'SignalId::default\(\)|SignalId::null\(\)' crates/emcore/src/emRecFileModel.rs crates/emstocks/src/`
Expected: only the field-init line (`Cell::new(SignalId::null())` in emRecFileModel.rs ctor) and unrelated SaveError-init scaffolding. No call site returns null from `GetFileStateSignal`.

- [ ] **Step 5: Update FU-005 bucket file with closure note**

Modify: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-005-emfilemodel-state-signal-conflation.md`

Append a closure section:
```markdown
## Closure (2026-05-02)

Resolved across three phased commits:
- Phase 1: emFileModel<T>.change_signal renamed to file_state_signal;
  emRecFileModel given lazy file_state_signal cell + ensure/fire helpers.
- Phase 2: parallel signal_file_state(ectx) fires added at all 9
  existing signal_change(ectx) sites in emRecFileModel.
- Phase 3: emstocks UPSTREAM-GAP comments removed;
  emStocksPricesFetcher first-Cycle subscribe wires through
  ensure_file_state_signal so the connect lands on a real id.

Adapted vs original spec: emRecFileModel does not compose emFileModel
in this port (standalone per file header), so delegation through
self.file_model.GetFileStateSignal() is not possible. Adapted: gave
emRecFileModel its own lazy-allocated file_state_signal cell mirroring
the existing change_signal lazy pattern. Observably equivalent.
```

- [ ] **Step 6: Commit**

```bash
git add docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-005-emfilemodel-state-signal-conflation.md
git commit -m "docs(FU-005): closure section — phased landing summary

Phase 3.3 final gate of FU-005."
```

---

## Acceptance criteria (mapped to spec)

- [x] `emFileModel<T>.change_signal` renamed to `file_state_signal` (5 internal sites). — Task 1.1.
- [x] `emRecFileModel::GetFileStateSignal` returns a real signal id post-ensure (lazy form per spec adaptation). — Task 1.2.
- [x] emRecFileModel state-mutation methods fire `FileStateSignal` alongside `ChangeSignal` at every existing `signal_change(ectx)` site. — Task 2.2.
- [x] 3 UPSTREAM-GAP markers removed (emStocksFileModel.rs, emStocksPricesFetcher.rs field doc, emStocksPricesFetcher.rs subscribe). — Tasks 3.1, 3.2.
- [x] No production caller of `GetFileStateSignal()` returns a forever-null id (subscribers wire via `ensure_file_state_signal`). — Tasks 1.2, 3.1, 3.2.
- [x] Tests updated to match new behavior; new fu005 test asserts correct fire and lifecycle. — Tasks 1.3, 2.1, 2.4, 3.2.
- [x] `cargo-nextest ntr`, `cargo clippy -- -D warnings`, `cargo xtask annotations` all clean. — Task 3.3.

## Out of scope (per spec)

- `emFileModel<T>` base-class state-mutation method ctx threading.
- `emRecFileModel.change_signal` rename (correct as-is — it IS the C++ ChangeSignal port).
- Per-state-transition fire granularity matching C++ exactly (multiple fires per method call). The Rust convention of "fire once per method via signal_change/signal_file_state pair" is preserved.
- Eager allocation of `file_state_signal` (lazy form is dependency-internal; matches existing sibling pattern).

## References

- Spec: `docs/superpowers/specs/2026-05-02-FU-005-emfilemodel-state-signal-conflation-design.md`
- Bucket: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-005-emfilemodel-state-signal-conflation.md`
- C++:
  - `~/Projects/eaglemode-0.96.4/include/emCore/emFileModel.h:75,295,313`
  - `~/Projects/eaglemode-0.96.4/src/emCore/emFileModel.cpp` lines 42, 55, 65, 76, 123, 143, 154, 188, 259, 267, 292, 514, 524, 532, 539
  - `~/Projects/eaglemode-0.96.4/src/emStocks/emStocksPricesFetcher.cpp:39`
- Rust:
  - `crates/emcore/src/emFileModel.rs:64,117,131,137,168,525`
  - `crates/emcore/src/emRecFileModel.rs:30-77, 138/156/163/169/184/205/224/234/248, 366`
  - `crates/emstocks/src/emStocksFileModel.rs:~146-166`
  - `crates/emstocks/src/emStocksPricesFetcher.rs:~75-83, ~420-435`
