# F019 — emFileModel lazy file_state_signal (FU-005 mirror)

**Date:** 2026-05-02
**Issue:** F019 (`docs/debug/ISSUES.json`)
**Predecessor pattern:** FU-005 emRecFileModel state-signal conflation fix (`e0e01500`)
**Sibling pattern:** B-015 emFilePanel D-006 Cycle-time subscribe (`emFilePanel.rs:514-541`)

## 1. Purpose

Close the model-side half of F019 by allocating `emFileModel<T>::file_state_signal` lazily, removing the constructor's signal-id parameter, and retiring the `emDirPanel::Cycle` `stay_awake`-while-loading workaround (F017 compensation, `emDirPanel.rs:393-421`).

The panel-side half (subscribe/disconnect on model swap) already landed via Tier-B B-015 D-006 Cycle-time subscribe and remains untouched here.

## 2. Background

`emDirModel::Acquire` constructs `emFileModel::new(path, SignalId::default())` because `Acquire`'s closure has no scheduler reach. The signal id is forever-null, so `emFilePanel::Cycle`'s `ectx.connect(target_sig, eid)` skip-on-null guard fires every cycle and the panel never wakes on state-change. `emDirPanel::Cycle` compensates with `stay_awake=true` while loading, ticking every 50 ms slice instead of only on FileStateSignal fires.

FU-005 just demonstrated the unblock pattern on the sibling `emRecFileModel`: constructor takes no signal arg; field is `Cell<SignalId>`; lazy accessor `GetFooSignal(&self, ectx: &mut impl SignalCtx) -> SignalId` allocates on first call. Three in-tree precedents (FU-005's `change_signal` and `file_state_signal`; B-004's `emFilePanel::vir_file_state_signal`) confirm the shape.

## 3. Architecture

Mirror FU-005 onto `emFileModel<T>`:

- Replace field `file_state_signal: SignalId` with `Cell<SignalId>`.
- Drop the `file_state_signal` parameter from `emFileModel::new`. Signature becomes `pub fn new(path: PathBuf) -> Self`.
- Replace `GetFileStateSignal(&self) -> SignalId` (currently returns the cached id, may be null) with `GetFileStateSignal(&self, ectx: &mut impl SignalCtx) -> SignalId` — lazy accessor; allocates via `ectx.create_signal()` on first call, returns cached id thereafter.
- The `FileModelState` trait method `GetFileStateSignal` adopts the new signature.
- All `emFileModel<T>` fire sites (currently `ctx.fire(self.file_state_signal)` at `emFileModel.rs:525`) become `ctx.fire(self.GetFileStateSignal(ctx))` so the model self-allocates before firing even when no panel has subscribed yet.

## 4. Components touched

| File | Change |
|---|---|
| `crates/emcore/src/emFileModel.rs` | Field → `Cell<SignalId>`; new lazy accessor; constructor signature change; fire sites use accessor; `FileModelState::GetFileStateSignal` trait signature changes |
| `crates/emcore/src/emImageFile.rs` | `emImageFileModel::new` drops `change_signal` parameter; `register` drops the upfront `ctx.create_signal()` call for file-state (data_change_signal stays eager — independent signal) |
| `crates/emcore/src/emFilePanel.rs` | `Cycle` subscribe site (lines 528-531) calls `m.borrow().GetFileStateSignal(ectx)`; test fixtures at lines 635 and 945 drop the eager-id arg |
| `crates/emfileman/src/emDirModel.rs` | `Acquire` (line 282) drops the `SignalId::default()` arg |
| `crates/emfileman/src/emDirPanel.rs` | `Cycle` drops `stay_awake` polling (lines 393-421); becomes pure observe-only; tests at 844, 904, 910, 1041, 1054, 1071, 1075 reframe from polling-assertions to fire-driven wake assertions |

## 5. Data flow

**Before:**

1. `emDirModel::Acquire` → `emFileModel::new(path, SignalId::default())` → forever-null signal.
2. `emFilePanel::Cycle` D-006 subscribe reads `null` → `connect` skipped.
3. `emFileModel::Cycle` fires on null → no-op.
4. Panel never wakes on state-change → `emDirPanel::Cycle` returns `stay_awake=true` while loading to compensate.

**After:**

1. `emDirModel::Acquire` constructs `file_model` with `Cell<SignalId>::new(null)`.
2. Two possible orderings, both correct:
   - *Model cycles first:* `emFileModel::Cycle` calls `self.GetFileStateSignal(ctx)` at fire time → lazy-allocates → fires (no subscribers yet, no observable effect). Panel's later first Cycle reads same cached id, connects, observes subsequent fires.
   - *Panel cycles first:* `emFilePanel::Cycle` calls `m.borrow().GetFileStateSignal(ectx)` → lazy-allocates → connects panel engine. Model's next Cycle reads same cached id and fires; panel wakes.
3. `emDirPanel::Cycle` returns `false` for observe-only paths.

## 6. Borrow ordering

At `emFilePanel.rs:528-531`, the panel reads the model's signal id inside a `m.borrow()` closure. Current shape:

```rust
let target_sig = self.model.as_ref()
    .map(|m| m.borrow().GetFileStateSignal())
    .unwrap_or_else(SignalId::null);
```

After:

```rust
let target_sig = self.model.as_ref()
    .map(|m| m.borrow().GetFileStateSignal(ectx))
    .unwrap_or_else(SignalId::null);
```

`m.borrow()` returns a `Ref` (immutable). The accessor takes `&self` and uses `Cell::set` for interior mutation; compatible with `Ref`. `ectx` is captured from the surrounding `Cycle` scope; `EngineCtx` implements `SignalCtx`. No borrow conflict.

## 7. Testing

**Unit tests added:**

- `emFileModel::new` without scheduler reach + first `Cycle` lazy-allocates and fires (mirrors FU-005 Phase 1's "null-until-ensured invariant" test).
- `GetFileStateSignal(ectx)` is idempotent: second call returns same id, does not invoke `create_signal` again.

**Tests updated:**

- `emFilePanel.rs:829` `set_file_model_connects_and_disconnects` — accessor signature change; behavioral expectations unchanged.
- `emFilePanel.rs:635`, `emFileModel.rs:945`, `emImageFile.rs` test fixtures drop the eager `SignalId` constructor arg.
- `emDirPanel.rs` `stay_awake` tests (sites at lines 844, 904, 910, 1041, 1054, 1071, 1075) reframe from "Cycle returns `true` while loading" to "FileStateSignal fire wakes the panel; Cycle returns `false`". The `/tmp` stay-awake assertion becomes a fire-driven wake assertion.

**Integration test (new):**

- emDirModel constructed via `Acquire` (no scheduler reach) → emDirPanel views it → directory load progresses via FileStateSignal fires → Cycle invocation count matches the C++ baseline (one wake per state change, not one per 50 ms slice).

**Acceptance:**

- `cargo-nextest ntr` clean.
- `cargo clippy -- -D warnings` clean.
- `cargo xtask annotations` clean.
- `docs/debug/ISSUES.json` F019 entry flips to `needs-manual-verification` with `fixed_in_commit` and `fixed_date` populated; fix_note records the residual lift and the polling retirement.
- Manual verification path: directory loading at parity with the F017 baseline (no observable regression in load speed or interactivity).

## 8. Out of scope (filed separately)

- Sweep for other "construction-time signal field set by no-reach caller" sites in emcore. Filed as a sibling follow-up bucket per the (c-3) decomposition; does not block F019.
- F020 (framework-scoped engine cleanup). Independent issue.

## 9. Risks

- **Test reframe surface area.** ≥ 7 emDirPanel test sites embed the `stay_awake`-while-loading invariant. Reframing each requires understanding what observable property each test was trying to check and rephrasing it in fire-driven terms. Risk of accidentally weakening assertions during the conversion. Mitigation: each reframed test must still fail under a regression that re-introduces the polling workaround.
- **Trait signature change.** `FileModelState::GetFileStateSignal` adopts a new signature. Any external impl outside the listed call sites breaks at compile time (compiler-enforced; no silent drift).
- **Ordering invariant.** Both panel-first and model-first orderings must work. Covered by unit tests; the lazy accessor's idempotency is the load-bearing invariant.
