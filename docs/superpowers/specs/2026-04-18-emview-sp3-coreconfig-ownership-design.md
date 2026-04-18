# SP3 — CoreConfig ownership on emView

**Date:** 2026-04-18
**Sub-project:** SP3 of the emView subsystem closeout (see `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md` §8.0).
**Closes:** §8.1 item 10; §4.6 (3× `PHASE-W4-FOLLOWUP:` markers at `crates/emcore/src/emView.rs:877, 923, 947`).
**Scope boundary:** add `CoreConfig` ownership to `emView`; realign `emVisitingViewAnimator::SetAnimParamsByCoreConfig` to the C++ signature; add the `VisitSpeed` max-value getter needed to drive it. Does **not** thread `emContext` through `emView::new` — that is a separate, larger sub-project deferred ("SP-later" below).

---

## 1. Background

C++ `emView.h:664` holds `emRef<emCoreConfig> CoreConfig`, acquired at construction (`emView.cpp:35`: `CoreConfig = emCoreConfig::Acquire(GetRootContext())`). Three sites in `emView.cpp` (`:505, :519, :536`) pass `*CoreConfig` to `VisitingVA->SetAnimParamsByCoreConfig(const emCoreConfig &)`.

Rust `emView` has no `CoreConfig` field. The three corresponding sites (`emView.rs:892, 924, 948`) call `va.SetAnimParamsByCoreConfig(1.0, 10.0)` — a divergent two-`f64` signature hardcoding the schema defaults. `PHASE-W4-FOLLOWUP:` comments flag the gap.

A secondary, previously-unmarked divergence: `emVisitingViewAnimator::SetAnimParamsByCoreConfig` itself takes `(speed_factor: f64, max_speed_factor: f64)` instead of C++'s `(const emCoreConfig &)`. The Rust method body inlines what C++ computes from config fields, so the schema-backed `VisitSpeed.GetMaxValue()` has nowhere to land.

## 2. Design decisions

Answered under CLAUDE.md's Port Ideology (C++ = ground truth; "Rust is defective by default"; silent drift worse than verbose preservation).

### 2.1 CoreConfig acquisition at construction

`emView::new` takes `core_config: Rc<RefCell<emCoreConfig>>` as a required argument, appended at end.

Rejected alternatives:
- *Internally default-construct per view.* Collapses C++'s preserved design intent (context-shared singleton) into per-view defaults. Explicitly flagged as a failure mode by CLAUDE.md.
- *`Option<Rc<RefCell<emCoreConfig>>>` + `SetCoreConfig` setter.* Introduces a Rust-only "not yet wired" state with no C++ analogue. More drift, not less.

Position (end of arg list, not first) is a transitional choice: C++'s true first arg is `emContext &`, and SP-later will prepend that. Preserving end-position minimizes churn for now; the ordering gets corrected when context threading lands.

### 2.2 Animator signature realignment

`emVisitingViewAnimator::SetAnimParamsByCoreConfig(&mut self, core_config: &emCoreConfig)` — 1:1 with C++ `emViewAnimator.cpp:979`. Body mirrors C++ `:983-989`:

```rust
let f = core_config.visit_speed;
let f_max = core_config.VisitSpeed_GetMaxValue();
self.animated = f < f_max * 0.99999;
self.acceleration = 35.0 * f;
self.max_absolute_speed = 35.0 * f;
self.max_cusp_speed = self.max_absolute_speed * 0.5;
```

### 2.3 VisitSpeed max-value exposure

`emCoreConfig` gains:
- `pub const VISIT_SPEED_MAX: f64 = 10.0;` — cited from C++ `emCoreConfig.cpp:53`: `VisitSpeed(this,"VisitSpeed",1.0,0.1,10.0)`.
- `pub fn VisitSpeed_GetMaxValue(&self) -> f64 { Self::VISIT_SPEED_MAX }` — `DIVERGED:` comment at definition: C++ uses `VisitSpeed.GetMaxValue()` on an `emRec`-typed field (`emDoubleRec VisitSpeed;` at `emCoreConfig.h:51`); Rust flattens because the `emRec`-backed scalar-field infrastructure is not ported.

Rejected: hardcoding `10.0` inside the animator. Schema metadata belongs on the config, not the consumer.
Rejected (out of scope): converting all `emCoreConfig` scalars to `emRec`-backed wrappers — SP-later at earliest.

### 2.4 `emSubViewPanel` child view

`emSubViewPanel::new()` has no parent or context accessible; it cannot reach the parent's `CoreConfig`. It constructs `Rc::new(RefCell::new(emCoreConfig::default()))` internally and carries a `DIVERGED:` comment: C++ `emSubViewPanel` shares the parent context's `emCoreConfig` singleton; full fidelity requires context threading (SP-later).

### 2.5 Framework / test callers

`emGUIFramework` / `emWindow` / benches / tests each construct `Rc::new(RefCell::new(emCoreConfig::default()))` at their `emView::new` call site. This produces observable behavior identical to the current `1.0, 10.0` hardcoding (defaults match schema defaults), so no golden movement expected. SP-later flips these to `emCoreConfig::Acquire(ctx.GetRootContext())` without changing `emView::new`'s signature.

A `#[cfg(any(test, feature = "test-support"))]` helper `emView::new_for_test(root, w, h)` constructs the default config internally to keep test bodies flat. Tests that exercise non-default `visit_speed` call the full `emView::new`.

## 3. Surface changes

### 3.1 `crates/emcore/src/emCoreConfig.rs`

- Add `pub const VISIT_SPEED_MAX: f64 = 10.0;` (with C++ citation comment).
- Add `pub fn VisitSpeed_GetMaxValue(&self) -> f64`.
- Both carry `DIVERGED:` rationale per 2.3.

### 3.2 `crates/emcore/src/emViewAnimator.rs`

- `SetAnimParamsByCoreConfig` signature: `(&mut self, core_config: &emCoreConfig)`.
- Body rewritten per 2.2.
- Tests at `:2927, :2934` migrate: construct `emCoreConfig { visit_speed: 2.0, ..Default::default() }` / `{ visit_speed: 10.0, ..Default::default() }` and pass `&cfg`.

### 3.3 `crates/emcore/src/emView.rs`

- Add `pub CoreConfig: Rc<RefCell<emCoreConfig>>` (C++ field name).
- `new` takes `core_config: Rc<RefCell<emCoreConfig>>` as last arg; stores it.
- Three call sites (`:892, :924, :948`) become:
  ```rust
  va.SetAnimParamsByCoreConfig(&self.CoreConfig.borrow());
  ```
- `PHASE-W4-FOLLOWUP:` comment blocks at each of the three sites: removed.
- Add `#[cfg(any(test, feature = "test-support"))] pub fn new_for_test(root, w, h) -> Self` constructing default config.

### 3.4 `crates/emcore/src/emSubViewPanel.rs`

- Line 51 gains a constructed default `emCoreConfig` argument; `DIVERGED:` comment per 2.4.

### 3.5 `emView::new` call sites (approx. list)

Production:
- `crates/emcore/src/emSubViewPanel.rs:51` — internal-default (2.4).
- `crates/emcore/src/emWindow.rs` (popup + framework-created windows) — construct `Rc::new(RefCell::new(emCoreConfig::default()))` per window.

Tests / benches (migrate to `emView::new_for_test` unless the test exercises config behavior):
- `crates/eaglemode/tests/support/mod.rs:46`
- `crates/eaglemode/tests/support/pipeline.rs:46`
- `crates/eaglemode/tests/unit/panel.rs:148, :199`
- `crates/eaglemode/tests/unit/input_dispatch_chain.rs:23, :50`
- `crates/eaglemode/tests/unit/max_popup_rect_fallback.rs:15`
- `crates/eaglemode/tests/integration/input.rs:138`
- `crates/emcore/src/emView.rs:4674` (inline unit test in the file itself)
- `crates/eaglemode/benches/common/mod.rs:765`
- `crates/eaglemode/benches/common/scaled.rs:80`
- `examples/bench_interaction.rs:701`
- `examples/bench_zoom_depth.rs:118, :200`
- `examples/bench_zoom_animate.rs:135`
- `examples/profile_hotpaths.rs:97`
- `examples/profile_testpanel.rs:674`

Implementation should enumerate via `rg 'emView::new\b' -g '!docs/**'` immediately before editing to catch any sites added since this spec.

## 4. Tests added

- `visit_unit_speed_activates_animator` (in `emViewAnimator.rs` tests): build `emCoreConfig { visit_speed: 1.0, .. }`, call `SetAnimParamsByCoreConfig(&cfg)`, assert `animated=true, acceleration=35.0, max_absolute_speed=35.0, max_cusp_speed=17.5`.
- `visit_max_speed_disables_animation`: `visit_speed=10.0` → `animated=false` (exercises the `f < f_max * 0.99999` predicate boundary).
- `visit_sub_max_still_animates`: `visit_speed=9.9999` → `animated=true` (boundary from the other side).
- `view_owns_corecfg` (in `emView.rs` tests): `emView::new(..., Rc::clone(&cfg))` stores the `Rc`; mutation visible both ways.
- `visit_uses_view_corecfg`: view constructed with `visit_speed=10.0`; after `Visit(...)`, animator reports `animated=false`.

No golden expected to move (defaults unchanged). Full golden suite + nextest runs as a verification gate regardless.

## 5. Markers

Closed by SP3:
- `PHASE-W4-FOLLOWUP:` at `emView.rs:877, 923, 947` (3 total) — deleted.

Added by SP3:
- `DIVERGED:` at `emCoreConfig::VisitSpeed_GetMaxValue` — emRec-field flatten (pre-existing infra gap; getter matches C++ call shape).
- `DIVERGED:` at `emSubViewPanel::new` internal-default config — transitional; removed by SP-later when context threading lands.

## 6. Out of scope / SP-later

- Threading `emContext` through `emView::new` and replacing `Rc::new(RefCell::new(emCoreConfig::default()))` at call sites with `emCoreConfig::Acquire(ctx.GetRootContext())`. Affects more than `CoreConfig` (all context-acquired services), deserves its own sub-project.
- Rewriting `emCoreConfig` scalar fields as `emRec`-backed wrappers carrying schema bounds at runtime. Removes the `VisitSpeed_GetMaxValue` flatten divergence but is a full-config-surface refactor.
- Wiring real on-disk config values through to views at runtime. SP3 only establishes ownership; current framework code paths still default-construct.

## 7. Verification

- `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr` (pre-commit gate).
- `cargo test --test golden -- --test-threads=1` — expect no new failures vs. baseline (237/243).
- `git grep -n PHASE-W4-FOLLOWUP crates/` returns empty.
- `git grep -n 'SetAnimParamsByCoreConfig.*1\.0.*10\.0' crates/` returns empty (no hardcoded pair remains in production code).
