# D4 — emRecListener Self-Update: Remove generation Counter

**Date:** 2026-04-29
**Audit item:** D-009 sighting #4 — `generation: Rc<Cell<u64>>` counter in `emCoreConfigPanel`
**Pattern:** D-009-polling-intermediary-replacement
**Cited decisions:** D-006, D-007, D-008, D-009
**C++ reference:** `emCoreConfigPanel.cpp`, `emRec.h:253-290` (`emRecListener`)

---

## Background

`ButtonsPanel::Cycle` in `emCoreConfigPanel.rs` bumps a `generation: Rc<Cell<u64>>` counter when the Reset button fires. Each sub-group's `LayoutChildren` polls `generation != last_generation` and destroys + rebuilds all children. This is a D-009 polling-intermediary: set in site A, polled in site B.

C++ avoids this entirely:
- Simple groups (`KBGroup`, `KineticGroup`, `MouseGroup`) have no group-level `OnRecChanged`. Their `FactorField` children inherit from both `emScalarField` and `emRecListener`, self-updating via `OnRecChanged() → UpdateValue() → SetValue()`.
- Complex groups (`MouseMiscGroup`, `PerformanceGroup`) implement `emRecListener(config)` at the group level, calling `UpdateOutput()` in `OnRecChanged()` to update non-scalar widgets (checkboxes) in place.

The Rust port has `emRecListener` already ported (`emRecListener.rs`) and `emCoreConfig` has aggregate signals fully wired via `emStructRec`. The fix mirrors C++ structure adapted to Rust's panel/behavior model.

---

## Design

### Overview

Three coordinated changes:

1. `ScalarFieldPanel` gains a per-field subscribe + value-read closure (equivalent to C++ `FactorField: emRecListener + UpdateValue()`).
2. Groups with non-scalar widgets (`MouseMiscGroup`, `CpuGroup`) gain a group-level config-aggregate subscribe + `update_output()` in Cycle.
3. Generation counter removed from all groups; child-rebuild dance in `LayoutChildren` deleted.

**Observable timing deviation:** C++ `OnRecChanged()` fires synchronously inside `modify()`. Rust signals fire on the next scheduler cycle — 1-cycle delay. Language-forced. Annotated at each subscribe site with `DIVERGED: language-forced`.

---

### Section 1: `ScalarFieldPanel` self-update

**Struct changes** (`emCoreConfigPanel.rs`, `ScalarFieldPanel`):

```rust
struct ScalarFieldPanel {
    scalar_field: emScalarField,
    // D-006 first-Cycle init subscribe to the specific field's value signal.
    // SignalId::null() = no config-update wiring (direct construction without
    // make_factor_field, or explicit opt-out).
    config_sig: SignalId,
    get_config_val: Option<Box<dyn Fn() -> f64>>,
    subscribed_to_config: bool,
}
```

**New `Cycle` implementation on `ScalarFieldPanel`:**

```rust
fn Cycle(&mut self, ectx: &mut EngineCtx<'_>, ctx: &mut PanelCtx<'_>) -> bool {
    // DIVERGED: language-forced 1-cycle delay vs C++ emRecListener::OnRecChanged
    // which fires synchronously inside emRec::Changed().
    if !self.subscribed_to_config && !self.config_sig.is_null() {
        ectx.connect(self.config_sig, ectx.id());
        self.subscribed_to_config = true;
    }
    if !self.config_sig.is_null() && ectx.IsSignaled(self.config_sig) {
        if let Some(ref get_val) = self.get_config_val {
            let new_val = get_val();
            self.scalar_field.SetValue(new_val, ctx);
        }
    }
    false
}
```

**`make_factor_field` signature change:**

Add two trailing parameters:
- `field_sig: SignalId` — the specific `emDoubleRec`'s value signal, obtained at `create_children` time via `c.SomeField.listened_signal()` on the borrowed rec.
- `get_val: Box<dyn Fn() -> f64 + 'static>` — closure that reads the current config value and converts it to the slider representation (same `factor_cfg_to_val` conversion used for the initial value).

The closure captures `Rc::clone(&self.config)` so it can borrow the config at reaction time.

**Construction at call sites** (inside `create_children` of each simple group):

```rust
let cfg = self.config.borrow();
let c = cfg.GetRec();
let field_sig = c.KeyboardZoomSpeed.listened_signal();
let config_clone = Rc::clone(&self.config);
let zoom = make_factor_field(
    ctx,
    "Keyboard zoom speed",
    "Speed of zooming by keyboard",
    self.look.clone(),
    0.25, 4.0,
    *c.KeyboardZoomSpeed.GetValue(),
    false,
    field_sig,
    Box::new(move || {
        let cfg = config_clone.borrow();
        factor_cfg_to_val(*cfg.GetRec().KeyboardZoomSpeed.GetValue(), 0.25, 4.0)
    }),
);
```

Callers that construct `ScalarFieldPanel` directly (not via `make_factor_field`) pass `SignalId::null()` and `None` to opt out.

---

### Section 2: Group-level subscribe for non-scalar widgets

**New accessor on `emRecNodeConfigModel<T>`** (`emRecNodeConfigModel.rs`):

```rust
/// C++ analogue: none (C++ `emRecListener` splices into the UpperNode chain
/// directly). Rust reifies the observable channel as a named accessor per D-008.
pub fn GetChangeSignal(&self) -> SignalId {
    self.value.listened_signal()
}
```

**`MouseMiscGroup` changes:**

Add two fields: `config_sig: SignalId`, `subscribed_to_config: bool`. Note: `subscribed_init: bool` already exists for the per-checkbox signal subscriptions (B-010); `subscribed_to_config` is a separate flag for the config-aggregate subscribe and must not be merged with it.

Set `config_sig = config.borrow().GetChangeSignal()` at `new()` time (signal is stable across borrows — it's a `SignalId` copy).

Augment existing `Cycle`:

```rust
// First-Cycle init: subscribe to config aggregate for update_output.
// Distinct from subscribed_init which gates per-checkbox wakeup subscriptions.
if !self.subscribed_to_config {
    ectx.connect(self.config_sig, ectx.id());
    self.subscribed_to_config = true;
}
// Config changed (e.g. Reset): update checkbox display.
if ectx.IsSignaled(self.config_sig) {
    self.update_output(ctx);
}
// ... existing checkbox IsSignaled branches unchanged ...
```

Add `update_output` method. Uses `set_checked_silent` (display-only, no signal fired) to avoid two problems: (1) `ctx.tree.with_behavior_as` borrows `ctx`, making it unavailable to pass into the closure; (2) firing the check signal from a programmatic display-sync would create a feedback loop. Read all config values before entering `with_behavior_as` so the config borrow is dropped first:

```rust
fn update_output(&self, ctx: &mut PanelCtx) {
    let (stick_val, emu_val, pan_val) = {
        let cfg = self.config.borrow();
        let c = cfg.GetRec();
        (
            self.stick_possible && *c.StickMouseWhenNavigating.GetValue(),
            *c.EmulateMiddleButton.GetValue(),
            *c.PanFunction.GetValue(),
        )
    };
    if let Some(id) = self.stick_id {
        ctx.tree.with_behavior_as::<CheckBoxPanel, _>(id, |p| {
            p.check_box.set_checked_silent(stick_val);
        });
    }
    if let Some(id) = self.emu_id {
        ctx.tree.with_behavior_as::<CheckBoxPanel, _>(id, |p| {
            p.check_box.set_checked_silent(emu_val);
        });
    }
    if let Some(id) = self.pan_id {
        ctx.tree.with_behavior_as::<CheckBoxPanel, _>(id, |p| {
            p.check_box.set_checked_silent(pan_val);
        });
    }
}
```

`set_checked_silent(&mut self, val: bool)` is a new method on `emCheckBox` that updates display state without firing `check_signal` — analogous to `set_checked_for_test` but non-gated and for production use. Add it alongside the existing test accessor.

**`CpuGroup` changes:** Same pattern. `update_output` handles the AllowSIMD checkbox. The MaxRenderThreads `ScalarFieldPanel` self-updates; `update_output` only touches the checkbox.

**Groups with only scalar fields (`KBGroup`, `KineticGroup`, `PerformanceGroup`, `MemFieldLayoutPanel`):** No group-level subscribe. Their `ScalarFieldPanel` children self-update via Section 1.

---

### Section 3: Generation counter removal

Remove from every group that holds it:
- `generation: Rc<Cell<u64>>` field
- `last_generation: u64` field
- `generation` parameter from `new()` signatures
- `generation` allocation site and threading through parent constructors

Remove from `ButtonsPanel::Cycle`:
```rust
self.generation.set(self.generation.get() + 1);  // DELETE
```

Remove from each group's `LayoutChildren`:
```rust
let gen = self.generation.get();
if gen != self.last_generation && ctx.child_count() > 0 {
    for id in ctx.children() { ctx.delete_child(id); }
    self.last_generation = gen;
}
// DELETE the above block entirely
```

The `if ctx.child_count() == 0 { self.create_children(ctx); }` guard remains — children are still created lazily on first expand, just never force-rebuilt on Reset.

---

### Section 4: Tests

New file: `crates/emcore/tests/rec_listener_b_d4.rs` (RUST_ONLY: dependency-forced).

**Test 1 — `scalar_field_panel_self_updates_on_field_change`:**
Construct a `ScalarFieldPanel` wired to a live `emDoubleRec` (not through full `emCoreConfigPanel`). Mutate the rec via `SetValue`. Advance scheduler. Assert `scalar_field.GetValue()` reflects the new converted value. Verifies the per-field self-update path in isolation.

**Test 2 — `mouse_misc_group_update_output_on_config_change`:**
Construct a `MouseMiscGroup` with an `emRecNodeConfigModel<emCoreConfig>`. Fire a config mutation changing `StickMouseWhenNavigating`. Advance scheduler. Assert the checkbox child reflects the new value via `with_behavior_as`. Verifies group-level subscribe + `update_output()`.

**Test 3 — `reset_button_updates_in_place_no_rebuild`:**
Construct a `ButtonsPanel` + sibling `KBGroup` wired to shared config. Fire the reset-button click signal. Advance scheduler. Assert: (a) `KBGroup`'s scalar field child shows the default slider value, and (b) the child was not destroyed and recreated (verify via child count stability or a sentinel ID). Regression guard for D-009 fix.

---

## Affected files

| File | Change |
|---|---|
| `crates/emcore/src/emCoreConfigPanel.rs` | `ScalarFieldPanel`: 3 new fields + `Cycle`; `make_factor_field`: 2 new params + call sites; `MouseMiscGroup`/`CpuGroup`: config subscribe + `update_output()`; all groups: generation counter removed |
| `crates/emcore/src/emRecNodeConfigModel.rs` | Add `GetChangeSignal(&self) -> SignalId` |
| `crates/emcore/src/emCheckBox.rs` | Add `set_checked_silent(&mut self, val: bool)` |
| `crates/emcore/tests/rec_listener_b_d4.rs` | New — 3 tests |

## End-of-bucket gate

1. `cargo check --workspace`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo-nextest ntr` (expect 2892 + 3 new tests)
4. `cargo xtask annotations`
5. `rg -n 'generation.*Cell\|Rc<Cell<u64>>' crates/emcore/src/emCoreConfigPanel.rs` → expect zero hits
6. Combined-reviewer at end of bucket.
