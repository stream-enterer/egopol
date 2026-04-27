# B-002-no-wire-emfileman — Design

**Date:** 2026-04-27
**Status:** Approved (brainstorm)
**Bucket:** `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/buckets/B-002-no-wire-emfileman.md`
**Pattern:** P-001-no-subscribe-no-accessor
**Scope:** emfileman, 4 rows
**Mechanical-vs-judgement:** balanced — judgement concentrates in one accessor (G2: emRecFileModel change-signal infrastructure); the other three rows are mechanical once that accessor lands.

## Goal and scope

Wire the four missing P-001 sites in emfileman: both halves of the wire (model-side accessor + consumer-side subscribe). The four rows split across two accessor groups and one panel-local infrastructure port (emTimer wakeup):

1. **G1 — emTimer-driven idle wakeup on `emDirPanel`.** C++ uses an `emTimer` with `AddWakeUpSignal(timer.GetSignal())` to clear key-walk state after 1000 ms idle. Rust uses `Instant::now()` compared lazily on next Input. No accessor port needed (emTimer is already ported in emcore); the fix is panel-internal: add an `emTimer` field, wake on its signal, clear key-walk state when fired.
2. **G2 — `emRecFileModel<T>::GetChangeSignal()` accessor + signal infrastructure.** The C++ class hierarchy is `emRecFileModel : public emFileModel`, where `emFileModel` owns `change_signal`. The Rust port broke this: `emRecFileModel<T>` (`emcore/src/emRecFileModel.rs:15`) is a *standalone* port that does not embed `emFileModel<T>` ("Standalone Rust port… Does not wrap `emFileModel<T>` to avoid self-referential borrow-checker constraints"). It has no `change_signal` field. `emFileLinkModel` composes `emRecFileModel` and exposes only `GetFileStateSignal`. Fix is to add change-signal infrastructure to `emRecFileModel` and a delegating accessor on `emFileLinkModel`. Three of the four bucket rows depend on G2.

All four rows are wired using **D-006-subscribe-shape** (first-Cycle init + IsSignaled top-of-Cycle). The accessor add lands in scope per **D-003-gap-blocked-fill-vs-stub**.

## Cited decisions

- **D-006-subscribe-shape** — canonical wiring pattern (`subscribed_init` flag + `ectx.connect` in first Cycle + `IsSignaled` at top).
- **D-003-gap-blocked-fill-vs-stub** — fill the missing G2 accessor in this bucket; both halves live in emfileman+emcore scope. The emRec hierarchy is *partially ported* (the standalone `emRecFileModel<T>` is in tree); the fix adds the missing signal field, not a new model. Therefore D-003's "fill in scope" rule applies.
- **D-001** — *not* cited; G2 returns `SignalId`, not `u64`. No type-mismatch in this bucket.

## Audit-data anomalies (corrections)

The following audit observations are stale or under-specified. Rows remain in B-002; the design records the correction so the working-memory session can patch `inventory-enriched.json`.

1. **`emFileLinkPanel-56`** — audit notes "Rust exposes only `GetFileStateSignal`," which is correct, but classifies the missing accessor as `emRecFileModel::GetChangeSignal`. It is more precisely `emFileModel::GetChangeSignal` *as inherited via emRecFileModel*. In Rust, the standalone-port choice means the accessor must be added on `emRecFileModel<T>` directly (not delegated through a wrapped `emFileModel<T>`). This is the structural-divergence consequence flagged in this bucket's open question.

2. **`emFileLinkModel-accessor-model-change`** — audit notes "fix requires propagating `GetChangeSignal` up `emStructRec`-derived models (also affects emAutoplay, emVirtualCosmos)." Re-reading the source: the change signal does *not* live on `emStructRec` or `emRec` in C++ — it lives on `emFileModel` and `emRecFileModel` exposes it because it inherits. The `emRec`-side mutation tracking C++ uses is `emRecListener` (already ported in Rust at `emcore/src/emRecListener.rs`). The audit conflated two distinct mechanisms:
   - **emFileModel/emRecFileModel change signal** — fired on record load/save/explicit signal. This is what this bucket needs.
   - **emRec mutation listener** — `emRecListener` engine-callback when individual fields mutate. Already ported; orthogonal to this bucket.

   The correction: G2 is scoped to `emRecFileModel<T>`, not the broader emRec hierarchy. The downstream impact on emAutoplay and emVirtualCosmos is *that those models also extend `emRecFileModel`* (or its stand-in), and once `emRecFileModel<T>::GetChangeSignal` exists, those consumers can subscribe through the same delegating-accessor pattern. **No emRec-hierarchy port is needed for B-002.**

3. **`emFileLinkPanel-72`** — audit asks whether re-subscribe-on-`SetFileModel` needs a dedicated Rust setter or can be tied to model handle lifetime. Rust analog of `emFileLinkPanel::SetFileModel` is `set_link_model` (`emFileLinkPanel.rs:88`). Subscribing inside `set_link_model` is the natural port; the C++ remove-then-add pattern reduces in Rust to "the subscription is recorded in the engine when the panel is registered, and the engine handle is the same engine across model swaps, so the connect call inside `set_link_model` is sufficient." Removal is implicit: a dropped `SignalId` connection is harmless once the signal stops firing. (This is a per-panel call to `ectx.connect`, which is idempotent for the engine-id × signal-id pair — the engine ID is the panel's own ID, established at panel registration.)

These corrections do not move any rows out of B-002.

## Investigation: emRec hierarchy cross-bucket dependency

The bucket sketch flagged "emRec hierarchy cross-bucket dependency as the headline open question" — specifically whether the `emFileLinkModel-accessor-model-change` row blocks on out-of-bucket emRec infrastructure work, or is a within-bucket design concern.

**Verdict: within-bucket.**

Reasoning, from reading `emcore/src/emRecFileModel.rs` and `emcore/src/emFileModel.rs`:

- `emFileModel<T>` already owns `change_signal: SignalId` (line 117) and `GetChangeSignal()` returns it (line 64). It also fires it on file events (line 518).
- `emRecFileModel<T>` is a *separate* port that doesn't wrap `emFileModel<T>`. It owns load/save state (`FileState`) and rec parsing, but no signal field.
- The audit's framing — "emRec hierarchy lacks change-signal exposure" — is a misattribution. The signal lives on `emFileModel` in C++, and the Rust port simply forgot to add an analogous field to the standalone `emRecFileModel<T>`. There is no missing emRec-base-class infrastructure; the fix is a SignalId field plus accessor on `emRecFileModel<T>`, mirroring what `emFileModel<T>` already does.
- Other consumers of `emRecFileModel`-derived models (emAutoplay, emVirtualCosmos, emStocksFileModel) will *benefit* from the G2 add — once it lands, their delegating accessors become one-liners — but they are not blockers for B-002 and B-002 is not blocked by them. B-002 is independently completable.

**Operational consequence:** No cross-bucket prereq edge. B-001 and other emstocks/emfileman buckets can pick up the G2 accessor as a free win once B-002 lands; B-001 in particular can drop its "G1 delegating accessor" sketch in favor of inheriting from the now-signaled `emRecFileModel<T>`. Surface this to the working-memory session as an *opportunity*, not a prereq.

## Accessor groups

### G1 — `emTimer` wakeup signal on `emDirPanel` for idle key-walk clear

**C++ source.** `emDirPanel.cpp:432` (within key-walk handling): `AddWakeUpSignal(KeyWalkState->Timer.GetSignal())` is called when the timer is created; on signal, `ClearKeyWalkState()` runs. The timer is started with `Start(1000)` and re-started on each new key (`Stop(true); Start(1000)`).

**Rust state today.** `emDirPanel::key_walk` (line 178) creates / extends `key_walk_state: Option<KeyWalkState>` carrying `last_key_time: Instant`. The state is never proactively cleared — the next `Input` call compares `Instant::now()` against `last_key_time` and starts over if the gap is > 1 s. There is no Cycle-driven wake to clear it.

**Observable drift.** A user typing `abc` then idling for 5 s sees the panel in *internal* key-walk state for those 5 s, even though no UI update happens; the next keystroke, instead of starting a fresh search, would *also* be the first new lookup (correct behavior). The visible drift is narrow: `KeyWalkState->Timer.GetSignal()` in C++ has no rendered consequence by itself — `ClearKeyWalkState()` only resets internal state. **However**, on most windows the panel is also a candidate for cursor / focus visualization that depends on having an active key-walk state; verify in C++ whether `ClearKeyWalkState` updates anything observable (e.g., status-bar text). The implementer must confirm by reading `emDirPanel::ClearKeyWalkState`. If the only effect is internal-state reset, the Rust lazy approach is *behaviorally* equivalent on every observable trajectory; if it touches paint or invalidation, the C++ shape must be ported.

**Fix shape (assuming any observable consequence).** Add an `emTimer` field to `emDirPanel`:

```rust
pub struct emDirPanel {
    // ... existing
    /// Port of C++ `KeyWalkState->Timer`. Drives idle-clear of key_walk_state.
    key_walk_timer: Option<emcore::emTimer::emTimer>,
    /// First-Cycle init flag for D-006-subscribe-shape.
    subscribed_init: bool,
}
```

In `key_walk` (line 178), after a key match, `Stop` then `Start(1000)` the timer (creating it lazily on first key). In Cycle, on `IsSignaled(timer.GetSignal())`, call a `clear_key_walk_state()` helper that drops `key_walk_state`. Connect the timer signal in the first-Cycle init block (D-006).

Lazy timer creation in C++ is mirrored by Rust's `Option<emTimer>`: the connect-call must be deferred until the timer exists. Use a secondary `key_walk_timer_subscribed: bool` flag, reset to `false` whenever the timer is `None`, and run the connect on the first Cycle after the timer is `Some`.

**Rows depending on G1:**
- `emDirPanel-432` (only this row).

### G2 — `emRecFileModel<T>::GetChangeSignal()` (model-change broadcast)

**C++ source.** Inherited from `emFileModel`. `emRecFileModel.h:50` declares `const emSignal & GetChangeSignal() const;` "Signaled on every modification of the record." Fired by the `emRec` mutation hook that `emRecFileModel` installs at `PostConstruct`, plus on file load/save transitions inherited from `emFileModel::Signal()`.

**Rust state today.**
- `emcore::emFileModel<T>` already owns `change_signal` and exposes `GetChangeSignal()` returning `SignalId` (line 64). Fires it at `:518`.
- `emcore::emRecFileModel<T>` is structurally divergent from C++: standalone port that does not embed `emFileModel<T>` (per the explanatory comment on the struct itself, line 13–14). It owns its own `state: FileState`, `path`, `error_text`, etc. — but no `change_signal: SignalId`.
- `emfileman::emFileLinkModel` composes `emRecFileModel<emFileLinkData>` (line 8 import; data field not visible from the snippet but consistent with the FileModelState delegation at line 248–268). Exposes only `GetFileStateSignal` via `FileModelState` trait.

**Fix shape — port the `emFileModel`-side change-signal infrastructure to `emRecFileModel<T>`:**

```rust
pub struct emRecFileModel<T: Record + Default> {
    // ... existing
    /// Port of C++ inherited `emFileModel::ChangeSignal`. Fired on rec
    /// mutation and on load/save transitions.
    change_signal: SignalId,
}

impl<T: Record + Default> emRecFileModel<T> {
    /// Updated signature: takes a SignalId allocated by the caller.
    pub fn new(path: PathBuf, change_signal: SignalId) -> Self { ... }

    /// Port of inherited C++ `emFileModel::GetChangeSignal`.
    pub fn GetChangeSignal(&self) -> SignalId {
        self.change_signal
    }

    /// Port of C++ Signal-on-mutation. Call from every mutation site that
    /// transitions through `GetWritableMap` / `set_unsaved_state_internal` /
    /// load completion / save completion.
    pub fn signal_change(&self, ectx: &mut EngineCtx) {
        ectx.fire(self.change_signal);
    }
}
```

Mutation-site enumeration (must fire `change_signal`):
- `set_unsaved_state_internal` (called from `GetWritableMap`).
- Load completion (`FileState::Loaded` transition).
- Save completion (`FileState::Saved` / unsaved-cleared transition).
- Any explicit error transitions if C++ fires there (audit `emRecFileModel.cpp` for `Signal(GetChangeSignal())` call sites).

The signature change of `new(path) → new(path, change_signal)` ripples to every call site of `emRecFileModel::new`. Audit:

```bash
rg "emRecFileModel.*::new\(" crates/
```

Each caller threads in a `SignalId` from a `ConstructCtx`. Most callers are themselves model constructors (e.g., `emFileLinkModel::new`, `emAutoplay…`, `emVirtualCosmos…`, `emStocksFileModel`) and already have a `ConstructCtx`-equivalent.

**Delegating accessor on `emFileLinkModel`:**

```rust
impl emFileLinkModel {
    /// Port of inherited C++ `emFileModel::GetChangeSignal` via
    /// `emRecFileModel`.
    pub fn GetChangeSignal(&self) -> SignalId {
        self.rec_model.GetChangeSignal()
    }
}
```

**Rows depending on G2:**
- `emFileLinkPanel-56` (initial subscribe in panel constructor / first-Cycle).
- `emFileLinkPanel-72` (re-subscribe on `set_link_model`).
- `emFileLinkModel-accessor-model-change` (accessor add itself).

## Per-panel consumer wiring

### emDirPanel (1 row: -432)

Apply D-006 with the lazy-timer shape from G1. The connect-list is one entry (`key_walk_timer.GetSignal()`); IsSignaled branch calls `clear_key_walk_state()`. The first-Cycle init runs as soon as the timer is `Some`; before that, the panel has nothing to subscribe.

Verify by reading `emDirPanel::ClearKeyWalkState` in C++ that the only effects are `KeyWalkState=NULL; delete state` (internal cleanup). If yes, the Rust lazy-Instant scheme is observably equivalent; this row may be re-classified as a non-divergence and the timer port skipped. **Default: port the timer to match C++ structure.** Surface the equivalence question to the working-memory session.

### emFileLinkPanel (2 rows: -56, -72)

Apply D-006. The panel has both an outer `emFilePanel` (composition at `:59`) and an `Option<Rc<RefCell<emFileLinkModel>>>` at `:62`. The C++ Cycle does:

```cpp
if (Model && IsSignaled(Model->GetChangeSignal())) {
    DirEntryUpToDate = false;
    doUpdate = true;
}
```

Plus three other signal subscriptions (UpdateSignalModel, VirFileStateSignal, ViewConfig::ChangeSignal) that are *out of scope* for B-002 (those would be P-001 or P-002 rows in other buckets if drifted; the audit didn't catch them in this bucket, so leave them for future audit/bucket coverage — flag for working-memory session).

**Add to the panel struct:**
```rust
pub struct emFileLinkPanel {
    // ... existing
    /// First-Cycle init flag for D-006-subscribe-shape.
    subscribed_init: bool,
    /// Tracks whether the current `model` has been subscribed.
    /// Reset on `set_link_model` to re-run the connect for the new model.
    model_subscribed: bool,
}
```

**Connect flow:**
1. `set_link_model` (`:88`) sets `self.model = Some(...)` and *also* sets `self.model_subscribed = false`. (Row -72.)
2. First Cycle (and every Cycle until `model_subscribed` flips true): if `self.model.is_some() && !self.model_subscribed`, call `ectx.connect(model.borrow().GetChangeSignal(), ectx.id())`, set `model_subscribed = true`. (Row -56.)
3. Top-of-Cycle: `if let Some(m) = &self.model { if ectx.IsSignaled(m.borrow().GetChangeSignal()) { self.needs_update = true; /* port DirEntryUpToDate = false equivalent */ } }`.

The Rust panel already has `needs_update: bool` and `update_data_and_child_panel`. Tying the IsSignaled branch to `needs_update = true` is the cleanest port; the existing notice() handler also flips it on viewing change, so the integration is one extra branch.

**Why not subscribe directly inside `set_link_model`.** `set_link_model` does not hold an `EngineCtx`. In C++, `AddWakeUpSignal` runs through the engine via `this`. Our shape: defer to first-Cycle-after-set, gated on `model_subscribed`. This is essentially the D-006 deferred-queue at construction (option B in D-006), but localized to model-set rather than panel-construct. Document at the bucket level (this design doc) so the working-memory session can decide whether D-006 wants amendment.

## Sequencing

**Within the bucket:**

1. **Land G2 accessor add first** (emRecFileModel signal field + `signal_change` + delegating accessor on `emFileLinkModel`). This is a leaf change with mechanical ripple: every `emRecFileModel::new` caller takes one extra arg. Ship behind a behavioral test that fires `signal_change` and asserts a subscriber wakes. **Pre-merge audit:** confirm all `emRecFileModel::new` callers are caught (rg over the full tree).
2. **Land G1 emTimer port on emDirPanel** (or skip if the equivalence audit confirms behavioral equivalence; default: port). Independent of G2.
3. **Land emFileLinkPanel consumer wiring** (rows -56, -72). Depends on G2. One PR landing both rows together since they touch the same struct and would leave inconsistent state if split.

**Cross-bucket prereqs.** None outbound (B-002 has no upstream prereq). Outbound *opportunity*: B-001 (emstocks) can simplify its G1 delegating accessor for `emStocksFileModel::GetChangeSignal` by inheriting through the now-signaled `emRecFileModel<T>` once B-002 lands. Not a blocker for B-001, just a simplification. Flag for working-memory session reconciliation.

**Cross-bucket impact (downstream).** Other models composing `emRecFileModel<T>` (emAutoplay, emVirtualCosmos, emStocksFileModel, plus any not yet enumerated) gain `GetChangeSignal` through delegation once G2 lands. None of those are bucket-coupled to B-002; they consume the G2 accessor independently in their own buckets.

## Verification strategy

**Behavioral tests.** Two new files (or one combined):

- `crates/emcore/tests/emrecfilemodel_change_signal.rs` — fire `signal_change` on a fresh `emRecFileModel<TestRec>`; assert a subscriber wakes via the engine. Cover load/save/mutation paths.
- `crates/emfileman/tests/typed_subscribe_b002.rs` — for `emFileLinkPanel`, fire the model's change signal via `model.borrow_mut().rec_model.signal_change(ectx)`; run Cycle; assert `needs_update == true` (or `update_data_and_child_panel` was scheduled). For `emDirPanel`, simulate `key_walk` then advance the timer signal; assert `key_walk_state` is cleared.

For `emFileLinkPanel-72` (re-subscribe on model swap): construct panel with model A; fire A's signal; assert wake. Call `set_link_model(B)`; fire B's signal; assert wake. Fire A's signal *after* swap; assert no spurious wake (or accept idempotent waking — the C++ contract is: only the current model fires a meaningful event; the Cycle body's `if let Some(m) = &self.model` guard handles this regardless).

**No new pixel goldens.** The drift surface is signal flow; existing emfileman goldens (if any cover emFileLinkPanel) remain the regression backstop for paint output.

**Annotation checks.** The `emRecFileModel<T>` standalone-port comment becomes more nuanced (still standalone for state, but signal-aware). If the existing comment doesn't already carry an annotation, leave it as a prose comment (per the retired `IDIOM:` rule). If it does carry a `DIVERGED:` tag, update the body but the tag remains valid (the standalone-port choice was a language-forced divergence under the former emFileModel-wrapping scheme). Run `cargo xtask annotations` after edits.

## Open items deferred to working-memory session

1. **Reconcile audit-data corrections** into `inventory-enriched.json`:
   - `emFileLinkModel-accessor-model-change`: replace "emRec hierarchy lacks change-signal exposure" with "emRecFileModel<T> standalone port lacks change-signal field; fix is local to `emRecFileModel<T>` plus delegating accessor."
   - Cross-reference: drop the "(also affects emAutoplay, emVirtualCosmos)" note as a *prereq* — re-tag as a *downstream beneficiary* opportunity once G2 lands.
2. **`emDirPanel-432` observable-equivalence question.** Confirm by reading C++ `ClearKeyWalkState` whether the only effect is internal-state cleanup. If yes, this row may be re-classified as below-surface adaptation (no fix needed). Default for the implementer: port the timer to match C++ structure.
3. **`set_link_model`-driven subscribe (row -72) is a D-006 option-B style local override** (deferred-queue at model-set rather than panel-construct). This is a *new local pattern* — not yet a global override of D-006. If a second bucket rediscovers it, propose D-007. Not proposing now.
4. **emFileLinkPanel out-of-scope subscriptions.** The C++ panel subscribes to `UpdateSignalModel->Sig`, `GetVirFileStateSignal()`, and `Config->GetChangeSignal()` in addition to `Model->GetChangeSignal()`. The audit only flagged the model-change row in B-002; the other three may be in B-005 (P-002 emfileman) already — confirm. If not, surface as an audit gap.
5. **Other emRecFileModel composers (emAutoplay, emVirtualCosmos, emStocksFileModel) get `GetChangeSignal` for free once G2 lands.** B-001's G1 delegating accessor sketch can simplify post-B-002. Communicate this to the B-001 implementer.

## Proposed new D-### entries

**None.** All four rows fit existing decisions (D-003, D-006). The `set_link_model`-driven subscribe is a within-D-006 local variant, not a new global decision.

## Success criteria

- All 4 rows have a `connect(...)` call in their owning panel's first-Cycle init block (or the `set_link_model`-driven equivalent for row -72).
- All 3 G2-dependent rows have an `IsSignaled(...)` branch in `emFileLinkPanel::Cycle` reacting to `model.GetChangeSignal()`.
- `emRecFileModel<T>` carries a `change_signal: SignalId` field, a `GetChangeSignal()` accessor, and a `signal_change(&self, ectx)` mutator. All `emRecFileModel::new` callers thread a SignalId.
- `emFileLinkModel::GetChangeSignal()` exists as a one-line delegating accessor.
- `emDirPanel` either (a) gains an `emTimer` field driving `clear_key_walk_state` per C++, or (b) the equivalence audit confirms behavioral parity and the row is closed without code change (working-memory session decides).
- `cargo clippy -D warnings` and `cargo-nextest ntr` pass.
- New behavioral tests in `crates/emcore/tests/emrecfilemodel_change_signal.rs` and `crates/emfileman/tests/typed_subscribe_b002.rs` cover all 4 rows.
- B-002 status in `work-order.md` flips `pending → designed`.
