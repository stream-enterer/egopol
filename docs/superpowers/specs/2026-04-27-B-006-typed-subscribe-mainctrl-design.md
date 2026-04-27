# B-006-typed-subscribe-mainctrl — Design

**Date:** 2026-04-27
**Status:** Approved (brainstorm)
**Bucket:** `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/buckets/B-006-typed-subscribe-mainctrl.md`
**Pattern:** P-002-no-subscribe-accessor-present
**Scope:** emmain:emMainControlPanel, 3 rows
**Mechanical-vs-judgement:** mechanical-heavy

## Goal & scope

Wire the three missing P-002 subscriptions in `emMainControlPanel` to mirror C++ `emMainControlPanel.cpp:217-219`, applying the **D-006-subscribe-shape** first-Cycle init pattern established in B-005. The three rows together cover the panel's full reactive surface: content-panel changes (217), window-flag changes (218), and main-config changes (219).

The bucket also covers the seven button click signals (C++ 220–226) that today route through the `Rc<Cell<bool>>` `ClickFlags` shim. Those rows are NOT in B-006's row set (they are P-004 rc-shim rows owned by another bucket) — this design touches them only to keep the Cycle body coherent and flags this in §"Cross-bucket prereq edges".

## Decisions cited

- **D-006-subscribe-shape** — primary citation. First-Cycle init block + `IsSignaled` checks at top of Cycle. Same shape applied verbatim from B-005's reference design (`docs/superpowers/specs/2026-04-27-B-005-typed-subscribe-emfileman-design.md`). No per-row deviation required.
- **D-003-gap-blocked-fill-vs-stub** — listed by the bucket sketch for row 218 on the assumption `GetWindowFlagsSignal` was missing. **The assumption is wrong**: `emcore::emWindow::GetWindowFlagsSignal` already exists at `crates/emcore/src/emWindow.rs:1279`. Row 218 is not gap-blocked. D-003 does not apply. See §"Row anomalies" in the report-back.

No other D-### decisions apply (D-001, D-002, D-004, D-005 do not touch this scope).

## Per-row design

### Row 217 — `ContentView.GetControlPanelSignal()`

**C++ ref:** `src/emMain/emMainControlPanel.cpp:217` (subscribe), `:249-251` (Cycle reaction → `RecreateContentControlPanel()`).

**Rust target:** `crates/emmain/src/emMainControlPanel.rs:287` (Cycle).

**Current Rust state:** The reaction is performed by `ControlPanelBridge` (`crates/emmain/src/emMainWindow.rs:825`), a Framework-scope engine. Its existence is justified by an in-place `DIVERGED: (language-forced)` annotation (`emMainWindow.rs:819-824`) explaining that `emMainControlPanel::Cycle` runs at SubView scope and cannot reach the content sub-view to call `CreateControlPanel`.

**Design choice:** **Keep `ControlPanelBridge`; do NOT relocate the subscribe.**

**Reasoning:** The bucket-sketch's open question (a) "move the subscribe back into emMainControlPanel to mirror C++ structurally" was correctly listed as the Port-Ideology default, but the existing `DIVERGED` annotation already documents the language-forced category and the underlying constraint (cross-tree access from a SubView-scope engine). Reopening this requires either (i) inverting SubView/Framework scope ordering — large blast radius, no payoff over the current shape, mirrors D-006 option C's rejection rationale — or (ii) creating a panel-side subscribe whose reaction is "ask Framework to do it," which is just `ControlPanelBridge` re-spelt. Per Port Ideology §"Forced divergence" rule 2 (dependency-forced — the framework's panel-tree lifetime), the existing divergence stands.

**Action:** Mark row 217 in `inventory-enriched.json` as **resolved-by-existing-divergence** (not a remediation target). The `DIVERGED` block on `ControlPanelBridge` already covers it; no code change. Working-memory session updates the row to point to the existing annotation.

### Row 218 — `MainWin.GetWindowFlagsSignal()`

**C++ ref:** `src/emMain/emMainControlPanel.cpp:218` (subscribe), `:253-255` (Cycle reaction → `BtFullscreen->SetChecked((MainWin.GetWindowFlags()&WF_FULLSCREEN)!=0)`).

**Rust target:** `crates/emmain/src/emMainControlPanel.rs:287` (Cycle).

**Accessor status:** **Present** at `crates/emcore/src/emWindow.rs:1279` (`pub fn GetWindowFlagsSignal(&self) -> SignalId`). Row 218 is **NOT gap-blocked** despite the bucket sketch's assertion. D-003 does not apply.

**Wiring:** Standard D-006 first-Cycle init connect + Cycle `IsSignaled` branch.

**Reaction:** Set the `BtFullscreen` check-button's checked state from `WindowFlags::FULLSCREEN`. The Rust `BtFullscreen` widget today lives inside `LMainPanel` (the child built in `create_children`). The reaction needs a handle. Two options:

- **A. Promote `bt_fullscreen` to an `emMainControlPanel` field** (`Rc<RefCell<emCheckButton>>` or similar) so Cycle can mutate it. Mirrors C++ where `BtFullscreen` is a direct member of `emMainControlPanel`.
- **B. Forward the request through `LMainPanel`** via a small `set_fullscreen_checked(bool)` helper.

**Choice:** **A**. C++ holds `BtFullscreen` as a direct member; per File and Name Correspondence the Rust field carries the same name. The current confinement to `LMainPanel` is an implementation detail of `create_children`, not a load-bearing structural choice. Hoist the `Rc<RefCell<>>` handle into `emMainControlPanel` and have `LMainPanel` use the same handle. The `Rc<RefCell<>>` justification is (b) context-registry-style shared widget handle, which is the canonical Rust shape for this Rust-port pattern (it already appears across emcore widget panels).

**Window-flag access:** Cycle needs `MainWin.GetWindowFlags()`. The Rust panel currently accesses MainWin via `crate::emMainWindow::with_main_window(|mw| ...)`. Use the same accessor.

### Row 219 — `MainConfig->GetChangeSignal()`

**C++ ref:** `src/emMain/emMainControlPanel.cpp:219` (subscribe), `:257-260` (Cycle reaction → set `BtAutoHideControlView` and `BtAutoHideSlider` checked from config booleans).

**Rust target:** `crates/emmain/src/emMainControlPanel.rs:287`.

**Accessor status:** Present (`emMainConfig::GetChangeSignal` returns `SignalId` per `crates/emmain/src/emMainConfig.rs:106`).

**Wiring:** Standard D-006 connect + branch.

**Reaction:** Set `BtAutoHideControlView.SetChecked(MainConfig.AutoHideControlView)` and `BtAutoHideSlider.SetChecked(MainConfig.AutoHideSlider)`. Same field-promotion choice as row 218 applies (`bt_auto_hide_control_view`, `bt_auto_hide_slider` are hoisted to `emMainControlPanel` fields per File and Name Correspondence).

**Config field access:** `self._config: Rc<RefCell<emMainConfig>>` is already a panel field; the leading underscore (currently signalling unused) is dropped and the field is renamed `config` to match C++ `MainConfig`.

## Wiring-shape application (D-006)

The first-Cycle init block in `Cycle()`:

```rust
fn Cycle(
    &mut self,
    ectx: &mut emcore::emEngineCtx::EngineCtx<'_>,
    pctx: &mut PanelCtx,
) -> bool {
    if !self.subscribed_init {
        let eid = ectx.id();
        // Row 218: window flags signal (accessor at emWindow.rs:1279).
        crate::emMainWindow::with_main_window(|mw| {
            ectx.connect(mw.GetWindowFlagsSignal(), eid);
        });
        // Row 219: config change signal.
        ectx.connect(self.config.borrow().GetChangeSignal(), eid);
        // Row 217 NOT subscribed here — handled by ControlPanelBridge.
        // (See DIVERGED block in emMainWindow.rs:819-824.)
        self.subscribed_init = true;
    }

    // Row 218 reaction (mirrors C++ Cycle 253-255).
    let fullscreen_signal = crate::emMainWindow::with_main_window(|mw| mw.GetWindowFlagsSignal());
    if ectx.IsSignaled(fullscreen_signal) {
        let is_fs = crate::emMainWindow::with_main_window(|mw| {
            mw.GetWindowFlags().contains(WindowFlags::FULLSCREEN)
        });
        self.bt_fullscreen.borrow_mut().SetChecked(is_fs);
    }

    // Row 219 reaction (mirrors C++ Cycle 257-260).
    if ectx.IsSignaled(self.config.borrow().GetChangeSignal()) {
        let cfg = self.config.borrow();
        self.bt_auto_hide_control_view.borrow_mut().SetChecked(cfg.AutoHideControlView);
        self.bt_auto_hide_slider.borrow_mut().SetChecked(cfg.AutoHideSlider);
    }

    // Existing click-flag polling for the seven button reactions stays as-is —
    // those are P-004 rc-shim rows owned by another bucket.
    // ... (existing flags.new_window / fullscreen / etc. block unchanged) ...
    false
}
```

Per-row deviations from D-006: **none**. Row 217 is excluded entirely (preserved-divergence per existing annotation), not a deviation in shape. Rows 218 and 219 are mechanical D-006 applications.

The `with_main_window(|mw| ...)` borrow shape is slightly awkward for the `IsSignaled` check (because we need the `SignalId` outside the closure to call `ectx.IsSignaled`). Implementer may add a `with_main_window_signal()` helper or simply read the signal id once at the top of Cycle. This is below-surface adaptation — no annotation needed.

## Verification strategy

C++ → Rust observable contract: the three signals fire and the documented Cycle reaction runs.

**Pre-fix observable behavior:**
- Row 218: User toggles fullscreen (e.g., via Alt+Enter or window manager); `BtFullscreen` check state does NOT update. C++ updates it via the signal-driven `SetChecked`.
- Row 219: External edit to `~/.eaglemode/emMain/config.rec` triggers config reload; `BtAutoHideControlView`/`BtAutoHideSlider` check states do NOT update. C++ updates them.
- Row 217: Already covered by `ControlPanelBridge`; behavior matches C++.

**Post-fix observable behavior:** all three reactions fire on signal arrival.

**New test file:** `crates/emmain/tests/typed_subscribe_b006.rs` (RUST_ONLY: dependency-forced — no C++ test analogue, mirrors B-005's test rationale).

Test pattern per row:

```rust
// Row 218
let mut h = Harness::new();
let panel = h.create_main_control_panel();
h.window_mut().SetWindowFlags(WindowFlags::FULLSCREEN);
h.run_cycle();
assert!(panel.bt_fullscreen.borrow().IsChecked());

// Row 219
let mut h = Harness::new();
let panel = h.create_main_control_panel();
h.config_mut().AutoHideControlView = true;
h.config_mut().fire_change_signal();
h.run_cycle();
assert!(panel.bt_auto_hide_control_view.borrow().IsChecked());
```

Row 217 is verified by existing `ControlPanelBridge` coverage in emMainWindow tests; B-006 adds no new test for 217.

**Four-question audit-trail evidence per row:** (1) signal connected? — D-006 init block. (2) Cycle observes? — `IsSignaled` branch. (3) reaction fires documented mutator? — assertions above. (4) C++ branch order preserved? — code review against C++ Cycle 247-293.

## Implementation sequencing

1. Hoist `bt_fullscreen`, `bt_auto_hide_control_view`, `bt_auto_hide_slider` from `LMainPanel`-local construction into `Rc<RefCell<emCheckButton>>` fields on `emMainControlPanel`. Rename `_config` → `config`. Add `subscribed_init: bool`. Land as one mechanical refactor commit; verify no behavior change.
2. Add the D-006 first-Cycle init block and the two `IsSignaled` branches for rows 218 and 219. Land as a single commit.
3. Add `tests/typed_subscribe_b006.rs` covering both rows. Land in the same commit as step 2 or immediately after.
4. Working-memory reconciliation: mark row 217 as resolved-by-existing-divergence in `inventory-enriched.json`; update bucket sketch to remove the D-003 citation and the gap-blocked claim for row 218.

The three rows have no inter-row ordering constraint within steps 2–3.

## Cross-bucket prereq edges

- **The seven `BtNewWindow..BtQuit` click reactions (C++ 220–226 / 262–290)** are P-004 rc-shim rows owned by a different bucket (the emmain rc-shim bucket). B-006 leaves them untouched. When that bucket lands, the existing `click_flags`-polling block in this Cycle is replaced with `IsSignaled(button.click_signal)` branches in the same D-006 shape. **No B-006 prereq blocks**: the panel is observable-correct after B-006 even if the click rows haven't migrated.

## Open questions for the implementer

1. **`bt_fullscreen` ownership shape.** `Rc<RefCell<emCheckButton>>` shared between `emMainControlPanel` and `LMainPanel` is the proposed shape. If the existing `LMainPanel` construction makes this awkward (e.g., the button is consumed into a `MainCheckButtonPanel` adapter via move), the implementer may need to wrap the inner `emCheckButton` in `Rc<RefCell<>>` at adapter level too. Pick whichever shape minimises adapter churn; document if non-obvious.
2. **`with_main_window_signal()` helper.** Whether to add a small typed accessor or inline the borrow dance. Implementer's call; below-surface.
3. **Row 217 bookkeeping.** Confirm with the working-memory session that "resolved-by-existing-divergence" is an acceptable inventory state distinct from "remediated." If not, propose a new state — but do not redesign the resolution.
