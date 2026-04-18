# W4 — emView Visit-State Restoration (Design)

**Date:** 2026-04-18
**Source:** `docs/superpowers/notes/2026-04-18-emview-followups-roadmap.md` (Wave 4)
**Origin:** `docs/superpowers/notes/2026-04-18-emview-followups-execution-debt.md` §1
**Scope:** Delete the accidental Rust-only visit-state scaffolding (`visit_stack`, `VisitState`, `pending_animated_visit`, `go_back`, `go_home`, `current_visit`, `animated_visit*`); restore `emView`'s ownership of `emVisitingViewAnimator` per C++ `emView.h:675`; route all `Visit`-family methods through it; port `GetVisitedPanel` and migrate readers; move `Home`-key handling from VIF to `emPanel::Input`; restore `factor=1.0` in `invariant_equilibrium_at_target`.

---

## Ideological frame

eaglemode-rs is an **observational port**. From any external vantage — user-visible behavior, event ordering, signals, focus transitions, emCore observers — the port must be indistinguishable from the C++ original. Below that surface:

1. **Forced divergence** — Rust's type system or an external crate genuinely cannot express the C++ shape. Accepted grudgingly, minimized.
2. **Author's design intent** — deliberate architectural choices by an exceptional engineer. Preserved.

Everything else — Rust idioms reached for because they felt natural, scaffolding accrued during iterative porting — is **accidental divergence**. Accidental divergence is removed, not redesigned.

The roadmap framed W4 as a three-way engineering choice ("thread `&PanelTree` vs. cache Viewed fields vs. delete-or-keep `Visit`/`go_back`/`go_home`"). Under this frame that framing is wrong. The correct question is: **which Rust visit-state types reflect author intent, and which are accidental?** The answer (research below) collapses the three-way choice.

## Problem

### The C++ author's visit-state model

From `include/emCore/emView.h:664-710` — emView's full inventory of visit-related state:

| C++ field | Role |
|---|---|
| `emPanel * ActivePanel` | currently-active panel |
| `emPanel * SupremeViewedPanel` | upper-most viewed panel |
| `emPanel * MinSVP, * MaxSVP` | SVP search bounds |
| `emOwnPtr<emVisitingViewAnimator> VisitingVA` | **the visit mechanism** |

**There is no `VisitStack`. No `VisitState`. No `PendingAnimatedVisit`. No `GoBack`/`GoHome`.**

C++ `Visit` (all four overloads at `emView.cpp:492-520`) is a three-line delegation:

```cpp
void emView::Visit(const char * identity, double relX, double relY, double relA,
                   bool adherent, const char * subject)
{
    VisitingVA->SetAnimParamsByCoreConfig(*CoreConfig);
    VisitingVA->SetGoal(identity, relX, relY, relA, adherent, subject);
    VisitingVA->Activate();
}
```

No state is stored on emView. When rel coords of the visited panel are needed later, the author calls `GetVisitedPanel(&relX, &relY, &relA)` (`emView.h:295`), which internally calls `CalcVisitCoords(panel, ...)` — derived on demand from `HomeX/Y/Width/Height` plus `panel->ViewedX/Y/Width/Height`.

`VisitNext/Prev/First/Last/In/Out/Up/Down/Left/Right/Neighbour` (`emView.cpp:564-756`) each compute a target, then call `Visit(identity, adherent, subject)`.

Home-key handling lives in `emPanel::Input` (`emPanel.cpp:1168-1198`):

```cpp
case EM_KEY_HOME:
    if (state.IsNoMod()) { View.VisitFirst(); event.Eat(); }
    else if (state.IsAltMod()) { View.VisitFullsized(this, View.IsActivationAdherent()); event.Eat(); }
    else if (state.IsShiftAltMod()) { View.VisitFullsized(this, View.IsActivationAdherent(), true); event.Eat(); }
    break;
```

Not in `emViewInputFilter`.

### The Rust drift

The Rust port accreted a parallel visit-state machine that has no C++ counterpart:

- `emView::visit_stack: Vec<VisitState>` — records (panel, rel_x, rel_y, rel_a) tuples on every `Visit` call.
- `emView::pending_animated_visit: Option<VisitState>` — a **dead pipeline**: `VisitNext/Prev/...` set it through `animated_visit`/`animated_visit_panel`; `take_pending_animated_visit` has **zero callers**. Keyboard navigation is silently broken in production today — the goal is recorded, no animator consumes it.
- `emView::go_back`, `go_home`, `current_visit` — no C++ counterpart.
- `emVisitingViewAnimator` (the C++ `VisitingVA` type) **is ported** (`emViewAnimator.rs:663`) but **not owned by `emView`**. Production code never constructs one; only tests do.
- VIF `Home` key → `view.go_home()` (`emViewInputFilter.rs:1261`) — double accidental: C++ handles Home in `emPanel::Input` and routes to `VisitFirst`, not via a "home" nav stack.

### Classification

| Rust item | Author intent? | Forced by Rust? | Verdict |
|---|---|---|---|
| `visit_stack`, `VisitState`, stack-push `Visit` body, `go_back`, `go_home`, `current_visit` | No — no C++ counterpart | No | Accidental. Remove. |
| `pending_animated_visit`, `animated_visit`, `animated_visit_panel`, `take_pending_animated_visit`, `has_pending_animated_visit` | No — C++ uses `VisitingVA` | No | Accidental dead pipeline. Remove. |
| Missing: `emView` ownership of `emVisitingViewAnimator` | Yes — C++ `emView.h:675` | — | Restore. |
| Missing: `GetVisitedPanel(tree, ...)` | Yes — C++ `emView.h:295` | — | Port. |
| VIF `Home` → `go_home` | No — C++ routes via `emPanel::Input` | No | Move to `emPanel::Input`; delete VIF handler. |
| `invariant_equilibrium_at_target` skips `factor=1.0` | — | — | Restore; the skip masked the dead-pipeline bug. |
| `emView::Visit(panel, rx, ry, ra)` — panel-form overload name | Name matches C++ | Rust lacks overloading ⇒ forced | Keep name; rewrite body. `VisitByIdentity` (Rust-disambiguated name, zero rename) becomes the canonical identity-form. |

## Target architecture

After W4, `emView`'s visit-related state matches C++ field-for-field within the Rust idiom:

```rust
pub struct emView {
    // preserved
    active: Option<PanelId>,                    // C++ ActivePanel
    // ... root, geometry, signals ...

    // NEW — C++ emView.h:675
    VisitingVA: Rc<RefCell<emVisitingViewAnimator>>,

    // DELETED
    // visit_stack: Vec<VisitState>,
    // pending_animated_visit: Option<VisitState>,
}
```

Rel-coord access is derived on demand via `CalcVisitCoords(tree, panel) → (rel_x, rel_y, rel_a)` (already exists, `emView.rs:863`).

`GetVisitedPanel(tree, &mut rel_x, &mut rel_y, &mut rel_a) → Option<PanelId>` mirrors `emView.cpp:471-489`: returns `ActivePanel` if viewed, else `SupremeViewedPanel`; fills rel coords via `CalcVisitCoords`. A Rust-idiomatic variant returning `Option<(PanelId, f64, f64, f64)>` is also provided for callers that prefer it.

`Visit(tree, panel, rel_x, rel_y, rel_a, adherent)` becomes a three-line delegation to `VisitingVA.SetAnimParamsByCoreConfig + SetGoal + Activate`. `VisitByIdentity` is the identity-form canonical body; the panel-form looks up identity + title from `tree` and calls it. All `VisitNext/Prev/First/Last/In/Out/Up/Down/Left/Right/Neighbour` bodies compute a target then call `VisitByIdentity`, matching `emView.cpp:564-756`.

`VisitingVA` is an `emEngine`. Its `CycleAnimation(dt)` (`emViewAnimator.rs` port of `emViewAnimator.cpp:1194`) writes directly to emView position/panel state each frame. `Activate()` engages `Cycle`; `Deactivate()` disengages — same as C++. No "pending" intermediary.

## Deletion inventory

### emView.rs

- `struct VisitState` (:37).
- `visit_stack: Vec<VisitState>` (:274) + initializer (:450).
- `pending_animated_visit: Option<VisitState>` (:309) + initializer (:468).
- `fn current_visit` (:669), `fn visit_stack` (:675), `fn visit_stack_mut` (:679).
- `fn animated_visit` (:843), `fn animated_visit_panel` (:862).
- `fn go_back` (:867), `fn go_home` (:879).
- `fn take_pending_animated_visit` (:3041), `fn has_pending_animated_visit` (:3046).
- `DIVERGED:` comment at :701.
- Stale comments at :5305, :5325 about "visit_stack mutation is not the intended path."
- Inline stack read at :1204 → migrated to `CalcVisitCoords`.
- Fallback at :3737 (`self.current_visit().panel`) → migrated to `self.SupremeViewedPanel(tree)` (C++ fallback, `emView.cpp:473`).

### emViewInputFilter.rs

- `InputKey::Home → view.go_home()` handler at :1261 — deleted outright.
- `view.Visit(panel, rx, ry, ra)` at :1676 — signature updated to `view.Visit(tree, panel, rx, ry, ra, adherent=false)`; semantics now route through `VisitingVA`.

### Tests

- `view_visit_and_back` (`tests/unit/panel.rs:140-165`) — deleted. Tests Rust-only API.
- `tests/integration/input.rs:143` comment "initial visit_stack with root" — deleted.
- All `current_visit()` reads in emView.rs tests (4610, 4612, 4965, 5075+, 5105+, 5274), `emViewInputFilter.rs` tests (2574+, 2737+, 2775+, 2824+, 2871+), `emViewAnimator.rs` tests (2738+, 2842+, 2911+, 3365+, 3379+), `golden/animator.rs:293`, `golden/input_filter.rs:80` — migrated to `GetVisitedPanel`.

## Additions

### `emView` — `VisitingVA` ownership

Field: `VisitingVA: Rc<RefCell<emVisitingViewAnimator>>`. Constructed in `emView::new`. Mirrors C++ `emView::emView` (`emView.cpp:82`).

New constructor on the animator: `emVisitingViewAnimator::new_for_view()` — no positional args, mirroring C++ `emVisitingViewAnimator(emView & view)`. Existing float-arg `new()` kept for standalone test callers. No back-ref to emView inside the animator; view identity is supplied by the Cycle caller, which already has `&mut emView` via the scheduler plumbing from Phase 7.

### `emView` — `Visit` method family

Names chosen by minimum-divergence from C++ under the Rust-has-no-overloading forced constraint:

```rust
// C++ emView.cpp:492
pub fn Visit(&mut self, tree: &PanelTree, panel: PanelId,
             rel_x: f64, rel_y: f64, rel_a: f64, adherent: bool) {
    let identity = tree.GetIdentity(panel);
    let subject = tree.GetTitle(panel);
    self.VisitByIdentity(tree, &identity, rel_x, rel_y, rel_a, adherent, &subject);
}

// C++ emView.cpp:500 — canonical identity-form
pub fn VisitByIdentity(&mut self, tree: &PanelTree, identity: &str,
                       rel_x: f64, rel_y: f64, rel_a: f64,
                       adherent: bool, subject: &str) {
    let mut va = self.VisitingVA.borrow_mut();
    va.SetAnimParamsByCoreConfig(&self.CoreConfig);
    va.SetGoal(identity, rel_x, rel_y, rel_a, adherent, subject);
    va.Activate();
}

// C++ emView.cpp:511 — short panel-form
pub fn VisitPanel(&mut self, tree: &PanelTree, panel: PanelId, adherent: bool) { ... }

// C++ emView.cpp:525 — fullsized
pub fn VisitFullsized(&mut self, tree: &PanelTree, panel: PanelId,
                      adherent: bool, utilize_view: bool) { ... }
```

The existing Rust `VisitByIdentity` at :2949 is not renamed; its body is replaced. Existing Rust `VisitFullsized` at :835 (currently delegates to the stack-push `Visit`) is rewritten to call `VisitingVA.SetGoalFullsized + Activate`.

### `emView` — `GetVisitedPanel`

```rust
pub fn GetVisitedPanel(&self, tree: &PanelTree,
                       rel_x: &mut f64, rel_y: &mut f64, rel_a: &mut f64)
                       -> Option<PanelId> {
    let p = self.active.filter(|&id| tree.is_viewed(id))
                        .or_else(|| self.SupremeViewedPanel(tree));
    if let Some(panel) = p {
        let (rx, ry, ra) = self.CalcVisitCoords(tree, panel);
        *rel_x = rx; *rel_y = ry; *rel_a = ra;
    } else {
        *rel_x = 0.0; *rel_y = 0.0; *rel_a = 0.0;
    }
    p
}
```

Rust-idiomatic companion `GetVisitedPanelIdiom(&self, tree: &PanelTree) -> Option<(PanelId, f64, f64, f64)>` provided for callers that prefer tuple returns. The out-param form exists to keep a straight C++ overlay.

### `emView` — `VisitNext/Prev/...` body rewrites

Each of the eleven methods (:2372, :2402, :2431, :2454, :2477, :2481, :2485, :2489, :2493, :2515 + `VisitNeighbour` internal helper if present) computes its target and calls `VisitByIdentity(tree, &identity, rel_x, rel_y, rel_a, adherent, &subject)` per `emView.cpp:564-756`. Current bodies end in `self.animated_visit_panel(tree, target, adherent)` — replaced by `VisitByIdentity`.

### `emPanel::Input` — key-routing port

Port the `EM_KEY_HOME`/`EM_KEY_END`/`EM_KEY_PAGE_UP`/`EM_KEY_PAGE_DOWN` block from `emPanel.cpp:1168-1198`:
- `Home` (no-mod) → `View.VisitFirst()`.
- `Home` (Alt) → `View.VisitFullsized(this, adherent)`.
- `Home` (Shift+Alt) → `View.VisitFullsized(this, adherent, true)`.
- `End` → `View.VisitLast()`.
- `PageUp` → `View.VisitOut()`.
- `PageDown` → `View.VisitIn()`.

If `emPanel::Input` does not yet dispatch key events in Rust, the minimum needed to accept these cases is added inside W4's Phase 4. Any such extension is minimum-necessary — no refactoring beyond what the key-routing case requires.

### Scheduler/Cycle integration for `VisitingVA`

`emVisitingViewAnimator` is an `emEngine`. Its constructor registers with the scheduler via the same path any `emEngine` uses (mirrors C++ `emEngine` base constructor behavior). `Activate()` causes subsequent time slices to invoke `Cycle → CycleAnimation`. `Deactivate()` disengages.

If the Phase-7 `UpdateEngineClass` / `is_signaled_for_engine` plumbing already covers animator engines, no change. If it needs extension, the extension is landed inside Phase 1 of W4 — not a separate spike or investigation.

## Caller migration

| Caller | From | To |
|---|---|---|
| `emMainWindow.rs:185` | `view.current_visit()` read | `view.GetVisitedPanel(tree, &mut rx, &mut ry, &mut ra)` |
| `emMainWindow.rs:1046` | same | same |
| `emViewInputFilter.rs:1261` | `view.go_home()` | Deleted; `Home` routes via `emPanel::Input` → `VisitFirst`. |
| `emViewInputFilter.rs:1676` | `view.Visit(panel, rx, ry, ra)` | `view.Visit(tree, panel, rx, ry, ra, false)` — routes through `VisitingVA`. |
| `emView.rs:2386, 2396, 2416, 2426, 2448, 2471, 2507, 2529, 2533, 2607` | `animated_visit_panel(...)` | `VisitByIdentity(...)` per C++. |
| `emView.rs:1204` | `visit_stack.last()` | `CalcVisitCoords(tree, ...)`. |
| `emView.rs:3737` | `self.current_visit().panel` | `self.SupremeViewedPanel(tree)` (C++ `emView.cpp:473`). |
| All `current_visit()` test readers | direct struct access | `GetVisitedPanel` out-params. Test intent preserved. |
| `tests/unit/panel.rs:140-165` | asserts on `visit_stack/go_back` | Deleted. |

## Phasing

Every phase commits green: `cargo fmt` + `cargo clippy -- -D warnings` + `cargo-nextest ntr` + golden baseline preserved.

### Phase 1 — `VisitingVA` ownership

Add `VisitingVA` field; construct in `emView::new`; wire engine registration. Add `new_for_view()` on the animator. Animator dormant (never activated yet).

*Acceptance:* emView holds `VisitingVA`; Activate→Cycle→Deactivate plumbing verified by unit test; zero behavior change to callers; nextest + golden baseline unchanged.

### Phase 2 — read-path port

Add `GetVisitedPanel` (out-param + idiomatic tuple companion). Migrate all production readers: `emMainWindow.rs:185`, `:1046`, the `:3737` fallback, the `:1204` inline stack read. Migrate all test `current_visit()` readers except those in tests slated for Phase 5 deletion.

*Acceptance:* grep shows zero non-test `current_visit()` calls; `current_visit` still compiles; nextest + golden baseline unchanged.

### Phase 3 — write-path rewrite

Rewrite `Visit`, `VisitByIdentity`, `VisitFullsized`, `VisitPanel` bodies to delegate to `VisitingVA`. Rewrite the eleven `VisitNext/Prev/First/Last/In/Out/Up/Down/Left/Right/Neighbour` bodies per `emView.cpp:564-756`. Update the VIF triple-tap call site's signature (`view.Visit(tree, panel, rx, ry, ra, false)`).

**Behavior change baked in.** Keyboard navigation now actually animates through `VisitingVA`. Phase-3 commit includes `scripts/verify_golden.sh --report` diff audit; any shifts are expected correctness restoration and documented in the commit message.

*Acceptance:* each rewritten body matches C++ line-for-line delegation shape (reviewable against `emView.cpp:492-756`); `pending_animated_visit` has zero writers; nextest passes; golden baseline 237/6 preserved or diff-audited and documented.

### Phase 4 — Home-key routing

Port `emPanel::Input` EM_KEY_{HOME,END,PAGE_UP,PAGE_DOWN} block from `emPanel.cpp:1168-1198`. Delete VIF `InputKey::Home` handler. `go_home` has zero callers.

*Acceptance:* integration test verifies Home key routes via `emPanel::Input → VisitFirst`; VIF has no Home handler; nextest passes.

### Phase 5 — accidental-divergence deletion

Delete in one commit:
- `struct VisitState`, `visit_stack`, `pending_animated_visit` fields + initializers.
- `current_visit`, `visit_stack`, `visit_stack_mut`, `go_back`, `go_home`.
- `animated_visit`, `animated_visit_panel`, `take_pending_animated_visit`, `has_pending_animated_visit`.
- `DIVERGED:` comment at :701; stale comments at :5305, :5325.
- `view_visit_and_back` test.
- `tests/integration/input.rs:143` comment.

Compile; fix any test reader missed in Phase 2.

*Acceptance:* `grep -nE 'visit_stack|pending_animated_visit|VisitState|\bgo_back\b|\bgo_home\b|\bcurrent_visit\b|animated_visit' crates/` → empty. Nextest passes.

### Phase 6 — invariant restoration + smoke

Restore `factor=1.0` in `invariant_equilibrium_at_target` (`emViewAnimator.rs:~3320`); remove stale `KNOWN GAP (TODO phase 8)` comment. Run full golden suite + smoke.

*Acceptance:* `invariant_equilibrium_at_target` passes at `factor=1.0`; nextest passes (count = current 2409 + 3 new − 1 deleted = 2411); golden baseline 237/6 preserved or documented; smoke (`timeout 20 cargo run --release --bin eaglemode`) returns 124 or 143.

## Testing strategy

### Preserved tests

Every `current_visit()`-reading test except `view_visit_and_back` is retained. Readers migrate to `GetVisitedPanel`. Test intent (observe rel coords before/after an operation) preserved.

### Deleted tests

- `view_visit_and_back` (`tests/unit/panel.rs:140-165`) — tests Rust-only API that no longer exists.

Net −1 test from deletion.

### New tests

1. `visiting_va_owned_by_view` — unit test in `emView.rs`: construct `emView`, assert `VisitingVA` present, assert Activate→Cycle→Deactivate fires engine-clock as expected.
2. `visit_routes_through_animator` — unit test: call `Visit(tree, panel, rx, ry, ra, adherent)`, observe `VisitingVA` state transitions to goal-set + active. Replaces the assertion surface previously provided by `visit_stack().len() == 2`.
3. `home_key_routes_through_empanel` — integration test: press `Home`, observe active-panel transition equivalent to `VisitFirst`. Covers the Phase-4 routing change.

Net +3 tests added.

### Golden tests

Baseline 237/6 preserved. Phase-3 commit runs `scripts/verify_golden.sh --report`; any shifts are expected-behavior-restoration and documented.

### Invariant restoration

`invariant_equilibrium_at_target` runs at `factor=1.0`. Phase-6 acceptance gate. The skip was a Rust-side test-debt workaround, not author intent.

### Smoke

`timeout 20 cargo run --release --bin eaglemode` returns 124 or 143 after Phase 6.

## Out of scope

- Wave 1 residuals (§3.1 re-entrancy doc comments, §3.2 geometry-signal double-fire comment, §4.8 `InvalidateHighlight` audit).
- Wave 3 popup architecture (already closed).
- Wave 5a Phase-8 test promotion; Wave 5b multi-window pixel tallness.
- Animator visual-quality tuning (curve constants, acceleration bias) — W4 restores the path, does not retune.
- Any emEngine-infrastructure refactoring beyond what `VisitingVA` registration requires.

## Acceptance

- `grep -nE 'visit_stack|pending_animated_visit|VisitState|\bgo_back\b|\bgo_home\b|\bcurrent_visit\b|animated_visit' crates/` → empty.
- `emView` owns `VisitingVA: Rc<RefCell<emVisitingViewAnimator>>`.
- Every `Visit*` method's body is a C++-shape delegation to `VisitingVA` (verifiable against `emView.cpp:492-756`).
- `GetVisitedPanel` ported; all production readers use it.
- `emPanel::Input` handles EM_KEY_{HOME,END,PAGE_UP,PAGE_DOWN} per `emPanel.cpp:1168-1198`; VIF does not.
- `invariant_equilibrium_at_target` covers `factor=1.0`.
- Golden baseline 237/6 preserved or diff-audited and documented.
- `cargo clippy -- -D warnings` clean.
- `cargo-nextest ntr` passes at 2411 (current 2409 + 3 new − 1 deleted).
- Smoke exits 124 or 143.
- No new `DIVERGED:` annotations.
- No `#[allow(...)]` / `#[expect(...)]` introduced.
