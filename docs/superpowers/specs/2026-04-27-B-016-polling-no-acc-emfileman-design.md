# B-016-polling-no-acc-emfileman — Design

**Date:** 2026-04-27
**Status:** Approved (brainstorm)
**Bucket:** `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/buckets/B-016-polling-no-acc-emfileman.md`
**Pattern:** P-007-polling-accessor-missing
**Scope:** emfileman (`emDirPanel`, `emDirStatPanel`, `emFileLinkPanel`) — 3 rows
**Mechanical-vs-judgement:** mechanical-heavy once the soft prereq lands (B-004 G1 ports `emFilePanel::GetVirFileStateSignal`). The three rows reduce to one D-006 first-Cycle init + one or two `IsSignaled` branches per panel, mirroring the C++ ctors and `Cycle` bodies.

## Goal & scope

Replace per-`Cycle` `vir_file_state` polling in three sibling fileman panels with subscribe-driven Cycle reactions. All three rows apply **D-005-poll-replacement-shape** (direct subscribe; consumer reaction collapsed into the Cycle body) wired through **D-006-subscribe-shape** (first-Cycle init block + `IsSignaled` checks at the top of `Cycle`). The polling code already inside each `Cycle` becomes signal-gated (it runs only when the subscribed signal fires, instead of every frame).

The three rows:

| Row | C++ ref | Rust target | Subscribed signals (per C++ ctor) |
|---|---|---|---|
| `emDirPanel-37` | `emDirPanel.cpp:37` | `emDirPanel.rs:344` (`Cycle`) | `GetVirFileStateSignal()` (cpp:37), `Config->GetChangeSignal()` (cpp:38) |
| `emDirStatPanel-30` | `emDirStatPanel.cpp:30` | `emDirStatPanel.rs:109` (`Cycle`) | `GetVirFileStateSignal()` (cpp:30), `Config->GetChangeSignal()` (cpp:39) |
| `emFileLinkPanel-54` | `emFileLinkPanel.cpp:54` | `emFileLinkPanel.rs:175` (`Cycle`) | `UpdateSignalModel->Sig` (cpp:53), `GetVirFileStateSignal()` (cpp:54), `Config->GetChangeSignal()` (cpp:55), `Model->GetChangeSignal()` (cpp:56) — see scope note |

**Scope note on emFileLinkPanel-54.** The audit row is specifically the *vir-file-state* polling read at `emFileLinkPanel.rs:175` (`refresh_vir_file_state()`). The C++ ctor subscribes to four signals on the surrounding lines; only the `GetVirFileStateSignal` connection is in B-016's row scope. The other three (`UpdateSignalModel->Sig`, `Config->GetChangeSignal`, `Model->GetChangeSignal`) belong to other audit rows or future buckets — this design wires only what row `-54` covers, but documents the surrounding structure so the implementer's first-Cycle init block has the right shape and does not need to be rewritten when the other connections land.

## Decisions cited

- **D-005-poll-replacement-shape** — primary citation for the reaction model. Direct subscribe; the Rust `Cycle` body for each panel is invoked by the engine when the subscribed `vir_file_state_signal` (or `Config->GetChangeSignal`) fires, and re-reads `file_panel.GetVirFileState()` inline. Mirrors C++ `Cycle` shape exactly: subscribe in ctor, `IsSignaled` branches at top of `Cycle`, react.
- **D-006-subscribe-shape** — wiring shape for all three rows. First-Cycle init block calling `ectx.connect(sig, ectx.id())` for each subscribed signal, gated on `subscribed_init: bool`, then `IsSignaled` checks, then the existing reactions. The C++ `AddWakeUpSignal` calls live in the panel ctor; per D-006 §"why first-Cycle init mirrors C++," the Rust port issues the equivalent `connect` calls on the first `Cycle` invocation because `ConstructCtx` does not expose `connect`.

D-001, D-002, D-003, D-004 do not apply: no type-mismatched accessors in scope (the `Config->GetChangeSignal` call sites in this bucket *do* return `u64` per the existing emfileman convention, but B-016's three rows do not subscribe to that accessor — they only *read* it on the side, identical to the existing `last_config_gen` shim. Flipping that accessor's type is owned by the D-001 / B-005 family, not B-016.); no `Rc<RefCell>` shim consumers; no gap-blocked rows once B-004 G1 lands; no stocks panels.

## Soft prereq edge — B-004 G1

B-016's wire requires `emFilePanel::GetVirFileStateSignal() -> SignalId` to exist. **B-004 G1 ports exactly this accessor** (`docs/superpowers/specs/2026-04-27-B-004-no-wire-misc-design.md` §G1, design committed 7:06a 2026-04-27). G1 adds:

- `vir_file_state_signal: SignalId` field on `emFilePanel`
- `GetVirFileStateSignal(&self) -> SignalId` accessor
- Fires on every `last_vir_file_state` mutation (in `SetFileModel`, `set_custom_error`, `clear_custom_error`, and `cycle_inner`)
- Constructor signature change: `emFilePanel::new(cc: &mut C: ConstructCtx)`

**Edge:** **B-004 G1 → B-016 (all three rows).** B-016 cannot land until G1 is merged. This is the soft edge flagged in the bucket sketch and confirmed here. If G1 is delayed, B-016 implementer can stage the wiring to land in the same PR by including G1's diff inline; bucket sketcher's preference is to land G1 first (it has its own consumer in `emImageFile-117` and is independently shippable).

No other cross-bucket prereqs.

## Audit-data corrections

Re-validation against the actual Rust source plus the B-019 reconciliation note:

1. **The "emDirModel doesn't implement FileModelState" framing is false and is being struck (per bucket-sketch inbound note).** `emDirModel` *does* implement `FileModelState` — `GetFileStateSignal` is at `emDirModel.rs:413`, delegating to `self.file_model.GetFileStateSignal()`. The polling at `emDirPanel.rs:344` is therefore not justified by any missing-trait gap; it is plain drift caused by the absence of `emFilePanel::GetVirFileStateSignal`. B-016's design treats it as drift accordingly. The stale `cleanup-emDirPanel-117` annotation row removed by B-019 carried the false framing; B-016 does not preserve it.

2. **All three rows poll only `vir_file_state`.** Re-reading each `Cycle` body confirms:
   - `emDirPanel.rs:344` — calls `self.dir_model.borrow().get_file_state()` (which is the same data path as `file_panel.GetVirFileState`'s underlying source, consulted directly to drive `stay_awake`). Also reads `self.config.borrow().GetChangeSignal()` as a `u64` generation counter (not via subscribe).
   - `emDirStatPanel.rs:109` — calls `self.file_panel.refresh_vir_file_state()` then `update_statistics()`. No other polled signals in this row's `Cycle` body.
   - `emFileLinkPanel.rs:175` — calls `self.file_panel.refresh_vir_file_state()`. The other three C++ subscribes (`UpdateSignalModel->Sig`, `Config->GetChangeSignal`, `Model->GetChangeSignal`) are *not* exercised by the current Rust `Cycle` body; the panel does not react to them at all today. They are out of B-016's row scope.

   **Confirmation that none of the three rows requires multi-source subscribe in B-016 scope.** D-005's deferred multi-source open question (resolved for B-015) holds here: `emDirPanel` and `emDirStatPanel` do subscribe to two signals in C++ (vir-file-state + config change), but the bucket sketcher noted only the vir-file-state polling read; the config-change subscribe is functionally shimmed via the `last_config_gen` u64 generation counter (`emDirPanel.rs:331`), which is a separate audit concern owned by the emFileMan config-signal cluster. B-016's three rows are vir-file-state-only.

3. **No accessor reclassifications.** All three rows are correctly tagged `accessor missing` *as of audit time*. Once B-004 G1 lands, the accessor exists; B-016 then proceeds as a P-006-shaped fix (polling, accessor present) on top of the freshly-landed accessor. The bucket retains its P-007 tag because at audit time the accessor was missing and B-016 owns the consumer-side wire that completes the gap-fill.

4. **Sketch open-question §3 — PR staging.** Resolved: **separate PRs.** B-004 G1 lands first as a standalone commit (its own consumer is `emImageFile-117`, in B-004); B-016 lands as a follow-up commit subscribing the three sibling consumers. This matches the natural dependency boundary and avoids cross-bucket PR coupling.

5. **Sketch open-question §4 — `emDirStatPanel` external-wake dependency.** Resolved: **collapsing into subscribe is observably equivalent to C++.** C++ `emDirStatPanel::Cycle` (`emDirStatPanel.cpp:52-65`) is invoked by the C++ scheduler whenever `VirFileStateSignal` or `Config->GetChangeSignal` fires (because both are subscribed in the ctor); it has no other wake source. The Rust port's current "external wake" reliance is incidental — Cycle returns `false` and depends on whatever else schedules the panel. After the wire, Cycle is invoked exactly when one of the two subscribed signals fires, identical to C++. No observable behavior change beyond eliminating the per-frame redundant work.

These corrections do not move any rows out of B-016.

## Accessor groups

Following the B-001 / B-008 organising convention. One accessor group covers all three rows.

### G1 — `emFilePanel::GetVirFileStateSignal` (panel-side virtual-file-state broadcast)

**Status.** **Ported by B-004 G1** (soft prereq). B-016 consumes the accessor; it does not port it.

**Accessor surface (post-B-004 G1):**

```rust
impl emFilePanel {
    pub fn GetVirFileStateSignal(&self) -> SignalId { self.vir_file_state_signal }
}
```

**Fires on (per B-004 G1):** `SetFileModel`, `set_custom_error`, `clear_custom_error`, `cycle_inner` (when `last_vir_file_state` mutates). Mirrors C++ `Signal(VirFileStateSignal)` call sites at `emFilePanel.cpp:51,78,87,158,179`.

**Subscribers wired by B-016:** all three rows in the bucket.

## Wiring-shape application (D-006)

### `emDirPanel::Cycle` (row `emDirPanel-37`)

**C++ ref:** ctor `emDirPanel.cpp:37-38` (`AddWakeUpSignal(GetVirFileStateSignal())` + `AddWakeUpSignal(Config->GetChangeSignal())`); reaction `emDirPanel.cpp:71-86` (Cycle: `IsSignaled(GetVirFileStateSignal()) || IsSignaled(Config->GetChangeSignal()) → InvalidatePainting + UpdateChildren + InvalidateChildrenLayout`).

**Rust target:** `crates/emfileman/src/emDirPanel.rs:344` (existing polling site inside `Cycle`).

**B-016 row scope:** the `vir_file_state` subscribe (cpp:37) only. The `Config->GetChangeSignal` subscribe (cpp:38) is *not* in this row's scope; the existing `last_config_gen` u64 generation shim continues unchanged (audit-tracked under the emFileMan config-signal cluster, not B-016).

**Field changes:**

```rust
pub struct emDirPanel {
    // ... existing fields ...
    /// Cached SignalId from emFilePanel::GetVirFileStateSignal, captured
    /// at first-Cycle init time. None until subscribed_init flips true.
    vir_file_state_sig: Option<SignalId>,
    subscribed_init: bool,
}
```

**Cycle wiring (D-006 first-Cycle init):**

```rust
fn Cycle(&mut self, ectx: &mut EngineCtx<'_>, ctx: &mut PanelCtx) -> bool {
    // Lazy emDirModel registration is unchanged (existing block at :320-329).
    if self.dir_model.is_none() {
        let dm_rc = emDirModel::Acquire(&self.ctx, &self.path);
        emDirModel::ensure_engine_registered(&dm_rc, ectx.scheduler);
        self.file_panel
            .SetFileModel(/* ectx if B-004 G1's signature change is final */, Some(...));
        self.dir_model = Some(dm_rc);
        // ...
    }

    // First-Cycle init: subscribe to emFilePanel::GetVirFileStateSignal.
    if !self.subscribed_init {
        let sig = self.file_panel.GetVirFileStateSignal();
        ectx.connect(sig, ectx.id());
        self.vir_file_state_sig = Some(sig);
        self.subscribed_init = true;
    }

    // IsSignaled-gated reaction (C++ emDirPanel.cpp:75-82).
    let vfs_fired = self
        .vir_file_state_sig
        .map_or(false, |s| ectx.IsSignaled(s));

    // Existing config-change generation-counter shim (out of B-016 scope).
    let cfg_gen = self.config.borrow().GetChangeSignal();
    let cfg_changed = cfg_gen != self.last_config_gen;
    if cfg_changed {
        self.last_config_gen = cfg_gen;
        self.child_count = 0;
    }

    if vfs_fired || cfg_changed {
        // C++ Cycle reaction: InvalidatePainting + UpdateChildren + InvalidateChildrenLayout.
        // The existing observed_state match-arm body already handles the materialization
        // and stay_awake decision; preserve it. Add InvalidatePainting + InvalidateChildrenLayout
        // calls if the existing body does not already cover them (verify against current Rust
        // panel-invalidation API; if absent in Rust, the cycle body's existing behavior is
        // semantically equivalent).
    }

    // Existing observed_state match-arm body unchanged.
    // ... (Loaded → update_children, LoadError → set_custom_error, Loading/Waiting → stay_awake) ...
}
```

**Reaction:** existing `observed_state` match-arm body is preserved verbatim. The win is invocation timing: `Cycle` is now scheduled by the engine when `vir_file_state_signal` fires (i.e., when `emFilePanel::cycle_inner` mutates `last_vir_file_state`, which itself fires inside `emFilePanel::Cycle` per B-004 G1's mutator audit). Per-frame polling is eliminated; the body still re-reads via `dir_model.borrow().get_file_state()` (semantically equivalent to a poll, observably correct now that `Cycle` is signal-driven).

**No `stay_awake` removal.** The current code returns `true` while `Loading`/`Waiting`. Mirror C++: C++ `emFilePanel::Cycle` returns `busy=true` while loading is in progress (its own per-Cycle re-entry until Loaded). Rust's `stay_awake` mirrors this. Keep the existing return-value logic.

### `emDirStatPanel::Cycle` (row `emDirStatPanel-30`)

**C++ ref:** ctor `emDirStatPanel.cpp:30,39` (`AddWakeUpSignal(GetVirFileStateSignal())` + `AddWakeUpSignal(Config->GetChangeSignal())`); reaction `emDirStatPanel.cpp:52-66` (Cycle: `IsSignaled(GetVirFileStateSignal()) → UpdateStatistics + InvalidatePainting`; `IsSignaled(Config->GetChangeSignal()) → InvalidatePainting`).

**Rust target:** `crates/emfileman/src/emDirStatPanel.rs:109` (existing `Cycle` body calling `refresh_vir_file_state` + `update_statistics`).

**B-016 row scope:** the `vir_file_state` subscribe only. No config-signal handling in this row's scope.

**Field changes:**

```rust
pub struct emDirStatPanel {
    pub(crate) file_panel: emFilePanel,
    config: Rc<RefCell<emFileManViewConfig>>,
    stats: DirStatistics,
    /// Cached SignalId from emFilePanel::GetVirFileStateSignal.
    vir_file_state_sig: Option<SignalId>,
    subscribed_init: bool,
}
```

**Cycle wiring (D-006 first-Cycle init):**

```rust
fn Cycle(&mut self, ectx: &mut EngineCtx<'_>, _ctx: &mut PanelCtx) -> bool {
    if !self.subscribed_init {
        let sig = self.file_panel.GetVirFileStateSignal();
        ectx.connect(sig, ectx.id());
        self.vir_file_state_sig = Some(sig);
        self.subscribed_init = true;
    }

    let vfs_fired = self
        .vir_file_state_sig
        .map_or(false, |s| ectx.IsSignaled(s));

    if vfs_fired {
        // C++ emDirStatPanel.cpp:57-60: UpdateStatistics + InvalidatePainting.
        self.file_panel.refresh_vir_file_state();
        self.update_statistics();
    }

    false
}
```

**Reaction:** preserve the existing `refresh_vir_file_state` + `update_statistics` calls, but only on `vfs_fired`. The redundant per-frame invocation is eliminated. Removing `refresh_vir_file_state` from the unconditional path is safe because B-004 G1's `cycle_inner` is the canonical refresh site; this panel observes the broadcast and re-syncs locally.

### `emFileLinkPanel::Cycle` (row `emFileLinkPanel-54`)

**C++ ref:** ctor `emFileLinkPanel.cpp:53-56` (four subscribes); reaction `emFileLinkPanel.cpp:77-108` (per-signal `IsSignaled` branches → `doUpdate=true` → `UpdateDataAndChildPanel`).

**Rust target:** `crates/emfileman/src/emFileLinkPanel.rs:175` (existing `Cycle` calling `refresh_vir_file_state`).

**B-016 row scope:** the `GetVirFileStateSignal` subscribe (cpp:54) only. The other three C++ subscribes are out of scope — they are not exercised by the current Rust `Cycle` body. The first-Cycle init block in B-016 wires only the vir-file-state signal; the structure is shaped to admit future additions (other three connections added on the same `subscribed_init` flag) without rewrite.

**Field changes:**

```rust
pub struct emFileLinkPanel {
    // ... existing fields ...
    vir_file_state_sig: Option<SignalId>,
    subscribed_init: bool,
}
```

**Cycle wiring (D-006 first-Cycle init):**

```rust
fn Cycle(&mut self, ectx: &mut EngineCtx<'_>, _ctx: &mut PanelCtx) -> bool {
    if !self.subscribed_init {
        let sig = self.file_panel.GetVirFileStateSignal();
        ectx.connect(sig, ectx.id());
        self.vir_file_state_sig = Some(sig);
        // Future additions (out of B-016 scope, listed for documentation):
        //   - UpdateSignalModel->Sig (emFileLinkPanel.cpp:53)
        //   - Config->GetChangeSignal() (emFileLinkPanel.cpp:55)
        //   - Model->GetChangeSignal() (emFileLinkPanel.cpp:56)
        self.subscribed_init = true;
    }

    let vfs_fired = self
        .vir_file_state_sig
        .map_or(false, |s| ectx.IsSignaled(s));

    if vfs_fired {
        // C++ emFileLinkPanel.cpp:85-88: InvalidatePainting + doUpdate=true →
        // UpdateDataAndChildPanel. Rust shim:
        self.file_panel.refresh_vir_file_state();
        // The full UpdateDataAndChildPanel is invoked from the existing
        // AutoExpand path; in Cycle, refresh the cached state. When the
        // remaining three subscribes are wired in a future bucket, the
        // doUpdate path will be re-introduced here.
    }

    false
}
```

**Reaction:** `refresh_vir_file_state` only, signal-gated. The full C++ `UpdateDataAndChildPanel` cascade depends on the three out-of-scope subscribes; B-016 does not introduce that call here because the other inputs aren't observed yet, and the panel currently does not call it from Cycle.

## Implementation sequencing

1. **B-004 G1 lands first** (soft cross-bucket prereq). G1 ports `emFilePanel::vir_file_state_signal` + `GetVirFileStateSignal()` + the four mutator fires. B-004 lands its own consumer (`emImageFile-117`) in the same PR.
2. **B-016 row 1 — `emDirPanel-37`.** Add `vir_file_state_sig` + `subscribed_init` fields; add first-Cycle init block; gate the existing observed_state body on `vfs_fired || cfg_changed`. Leave `last_config_gen` shim untouched. Add a test (see Verification).
3. **B-016 row 2 — `emDirStatPanel-30`.** Add fields + init block; gate `refresh_vir_file_state` + `update_statistics` on `vfs_fired`.
4. **B-016 row 3 — `emFileLinkPanel-54`.** Add fields + init block; gate `refresh_vir_file_state` on `vfs_fired`. Document the three out-of-scope subscribes inline so the future bucket has a turnkey extension point.

Rows 2, 3, and 4 are independent of each other and can land in any order or as one combined PR. They all depend only on row 1 of the prereq (B-004 G1).

## Verification strategy

C++ → Rust observable contract: each polled re-read becomes a signal-driven Cycle invocation; observable behavior identical (state mutation matches; what changes is invocation timing — no longer per-frame).

**Pre-fix observable behavior:**
- All three panels rely on `Cycle` being invoked every frame to detect file-state transitions. With `Cycle` returning `false` and no other wake source on the panel, future scheduler optimizations could starve the polling. The drift is "live behavior accidentally works due to over-scheduling."

**Post-fix observable behavior:** all three panels fire their existing reactions on `vir_file_state_signal` arrival, independent of any other wake source. Matches C++ `Cycle` invocation cadence exactly.

**New test file:** additions to the existing emfileman test suite, or new `crates/emfileman/tests/polling_b016.rs`. RUST_ONLY: dependency-forced — no C++ test analogue, mirrors B-005/B-008/B-015 test rationale.

Test pattern per row:

```rust
// emDirPanel — fire VirFileStateSignal, assert Cycle reacts.
let mut h = Harness::new();
let panel = h.create_dir_panel("/tmp/test");
h.run_cycle(); // first-Cycle init wires the subscribe + lazy emDirModel
let model = panel.borrow().dir_model_for_test().unwrap();
model.borrow_mut().set_state(FileState::Loaded);
// SetFileModel/cycle_inner fires VirFileStateSignal per B-004 G1.
h.run_cycle();
assert_eq!(panel.borrow().child_count_for_test(), expected_count);

// emDirStatPanel — fire VirFileStateSignal, assert update_statistics ran.
let mut h = Harness::new();
let panel = h.create_dir_stat_panel(model);
h.run_cycle(); // init
model.borrow_mut().set_state(FileState::Loaded);
h.fire(panel.borrow().file_panel.GetVirFileStateSignal());
h.run_cycle();
assert_eq!(panel.borrow().stats_for_test().total_count, model_total);

// emFileLinkPanel — fire VirFileStateSignal, assert refresh_vir_file_state ran.
let mut h = Harness::new();
let panel = h.create_file_link_panel(model);
h.run_cycle();
model.borrow_mut().set_state(FileState::Loaded);
h.fire(panel.borrow().file_panel.GetVirFileStateSignal());
h.run_cycle();
assert!(matches!(panel.borrow().file_panel.GetVirFileState(), VirtualFileState::Loaded));
```

**Four-question audit-trail evidence per row:** (1) signal connected? — D-006 init block calls `ectx.connect(GetVirFileStateSignal(), ectx.id())`. (2) Cycle observes? — `IsSignaled` branch on `vir_file_state_sig`. (3) reaction fires documented mutator? — assertions above (`update_statistics`, `update_children`, `refresh_vir_file_state`). (4) C++ branch order preserved? — code review against C++ `emDirPanel::Cycle` (cpp:71-86), C++ `emDirStatPanel::Cycle` (cpp:52-66), C++ `emFileLinkPanel::Cycle` (cpp:77-108).

## Cross-bucket prereq edges

- **B-004 G1 → B-016 (all three rows). Soft, blocking.** B-016 cannot land until `emFilePanel::GetVirFileStateSignal` exists. B-004 G1 ports it and is independently shippable. If G1 slips, B-016 implementer either waits or merges G1's diff inline.
- **B-016 has no outbound prereq edges.** No other bucket depends on B-016 completing.

## Out-of-scope adjacency

- **`Config->GetChangeSignal` subscribes (cpp:38, cpp:39, cpp:55).** Not in B-016's row scope. The existing `last_config_gen` u64 shim in `emDirPanel.rs` is functionally correct (re-checks per Cycle; works because the panel is awake whenever its surroundings are). Replacing it with a true subscribe belongs to the emFileMan config-signal cluster (D-001 family).
- **`emFileLinkPanel`'s `UpdateSignalModel->Sig` and `Model->GetChangeSignal` subscribes (cpp:53, cpp:56).** Not in B-016's row scope. The Rust panel currently does not react to either; the future bucket that wires them adds two `connect` calls to the same `subscribed_init` block plus two `IsSignaled` branches in `Cycle`. B-016's structure is shaped to absorb these additions without rewrite.
- **`emDirPanel::stay_awake` semantics during Loading/Waiting.** Preserved as-is. Returning `true` from Cycle while loading is in progress mirrors C++ `busy=emFilePanel::Cycle()` returning `true` during loading. Below-surface; no annotation.

## Open questions for the implementer

1. **`emFilePanel::SetFileModel` signature.** B-004 G1's design flags an open question: does `SetFileModel` need to thread `&mut EngineCtx` to fire `VirFileStateSignal` synchronously at swap time, or queue the fire for next Cycle? The B-016 caller at `emDirPanel.rs:325` (the lazy registration block) must be updated to match whichever shape G1 lands. If G1 picks the sync-thread shape, `emDirPanel`'s `Cycle` already has `ectx` in hand — pass it through; if G1 picks the deferred-fire shape, no caller change. Confirm with G1's final commit before wiring.
2. **`InvalidatePainting` + `InvalidateChildrenLayout` parity.** C++ `emDirPanel::Cycle` calls `InvalidatePainting()` and `InvalidateChildrenLayout()` after a vir-file-state or config-change fire. Rust panel infra may or may not require explicit invalidation calls (some ports do this implicitly via the engine's dirty-tracking). Verify against the existing Rust panel invalidation API (look at `emPanel::InvalidatePainting` / `InvalidateChildrenLayout` if they exist; if absent, the engine's existing wake-on-Cycle dirty marking covers it). If absent in Rust, no annotation needed — below-surface.
3. **`vir_file_state_sig: Option<SignalId>` vs `SignalId`.** `Option` lets us defer subscription to first Cycle (the `subscribed_init` flag is the trigger). An alternative is to capture the SignalId at panel construction time (since `emFilePanel::new` returns the field) and store as a plain `SignalId`; the `subscribed_init` flag still gates the `connect` call. The `Option` form makes the "not yet subscribed" state explicit; either is correct. Pick whichever matches the prevailing style in the surrounding panels post-B-005/B-008.
4. **Test harness fire helpers.** B-005's harness exposes `Harness::fire(SignalId)`. Confirm the harness exposes a way to fire `vir_file_state_signal` indirectly via a model state mutation (the natural path: `model.set_state(...)` triggers `cycle_inner` which fires the signal per B-004 G1). The test pattern above assumes this; if the indirect path is awkward, fall back to direct `h.fire(panel.borrow().file_panel.GetVirFileStateSignal())`.

## Open items deferred to working-memory session

1. **No new D-### proposed.** This bucket reuses D-005 (reaction model: direct subscribe) and D-006 (wiring shape: first-Cycle init + IsSignaled at top of Cycle). The accessor port itself is owned by B-004 G1 (already designed) and falls under D-003 option A. No global decision is surfaced.
2. **D-005 open question §1 (subscribe-arity per consumer).** Confirmed: each B-016 consumer subscribes to a single `GetVirFileStateSignal` for its row scope, mirroring C++ exactly. C++ `emDirPanel`/`emDirStatPanel` ctors do also subscribe to `Config->GetChangeSignal` — out of B-016's row scope but documented above so the future bucket extends the same first-Cycle init block instead of rewriting it.
3. **B-019 inbound note honored.** The bucket-design does not preserve the "emDirModel doesn't implement FileModelState" framing (which was false; see Audit-Data Corrections §1). Working-memory session may strike that line from any remaining audit notes that still cite it.
4. **Cross-bucket prereq edge B-004 G1 → B-016 (all three rows).** Add to `work-order.md` DAG. B-016 cannot reach `merged` before B-004 G1's accessor is in tree. No row reclassifications.
5. **No row reclassifications.** All three rows verified `accessor missing` at audit time and `accessor present (post-G1)` at execution time. Bucket retains its P-007 tag because the audit-time accessor status defines the bucket; the post-G1 P-006 shape is the implementation reality.

## Success criteria

- `emDirPanel`, `emDirStatPanel`, and `emFileLinkPanel` each subscribe to `emFilePanel::GetVirFileStateSignal()` in a D-006 first-Cycle init block.
- Each panel's `Cycle` body runs its existing reaction (observed_state match-arm / `update_statistics` / `refresh_vir_file_state`) only when the subscribed signal fires (or, for `emDirPanel`, when the existing `last_config_gen` shim flips).
- `cargo clippy -D warnings` and `cargo-nextest ntr` pass.
- New tests cover: each of the three panels' subscribe + signal-driven reaction.
- B-016 status in `work-order.md` flips `pending → designed` (working-memory session reconciliation), and per-row commits flip to `merged` as they land (after B-004 G1 lands).
