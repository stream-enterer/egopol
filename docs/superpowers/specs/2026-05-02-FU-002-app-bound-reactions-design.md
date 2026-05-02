# FU-002 — App-bound reaction wiring (mainctrl)

**Bucket:** [FU-002](../../debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-002-app-bound-reactions.md)
**Date:** 2026-05-02
**Scope:** `emmain` only.
**Prereqs:** none.

## Summary

Three reaction bodies in `emMainControlPanel::Cycle` (rows 220 Duplicate, 221 Fullscreen, 226 Quit) are currently `TODO(B-012-followup)` stubs. The bucket file framed this as needing an architectural decision on App-threading from Cycle. **Research showed the decision is already made** — the `pending_actions` queue on `App` is the established pattern, already pervasive (`emBookmarks.rs:748` uses it; `emMainWindow::Duplicate` itself uses it), and `EngineCtx::pending_actions()` is the call-site accessor. The bucket reduces to one small helper plus three 1-line reaction wirings.

## Design intent (per Port Ideology)

The Rust `&mut App` parameter on `emMainWindow::Duplicate` / `ToggleFullscreen` / `Quit` is an idiom adaptation below the observable surface (per CLAUDE.md): C++ reaches `GetScheduler()` via panel-tree traversal; Rust threads App explicitly. This is **not** a divergence to revisit — observable behavior is identical, and the parameter is the right Rust shape. The reaction-body gap is purely about *getting* `&mut App` into Cycle's reach, which the existing `pending_actions` deferral mechanism solves.

The three "alternative" architectural options the bucket file enumerated (thread `&mut App` through `EngineCtx`, pending-action queue, `Rc<RefCell<App>>` registry) are obviated: the queue is option (b), and it already exists.

## Work units

Two units, one commit per unit (Phase 3 is documentation reconciliation):

### Unit 1 — `enqueue_main_window_action` helper (emmain)

**File:** `crates/emmain/src/emMainWindow.rs`.

**Change:**

Add a helper near the existing `with_main_window` accessor:

```rust
/// Enqueue a deferred action that runs with `&mut emMainWindow` and `&mut App`.
/// The closure executes during the next `pending_actions` drain (winit main
/// loop tick). Mirrors the pattern at `emBookmarks.rs:748` and the inline use
/// at `emMainWindow::Duplicate` line 233.
///
/// Use from `Cycle` reaction bodies that need to invoke MainWindow methods
/// requiring `&mut App` (Duplicate / ToggleFullscreen / Quit).
pub(crate) fn enqueue_main_window_action<F>(ectx: &mut EngineCtx<'_>, action: F)
where
    F: FnOnce(&mut emMainWindow, &mut App) + 'static,
{
    ectx.pending_actions()
        .borrow_mut()
        .push(Box::new(move |app, _event_loop| {
            with_main_window(|mw| action(mw, app));
        }));
}
```

No tests for the helper itself: it's a thin wrapper over an established pattern with no logic of its own. Coverage comes from the call sites in Unit 2.

### Unit 2 — Wire 3 reaction bodies (emmain)

**File:** `crates/emmain/src/emMainControlPanel.rs`.

**Changes — three sites in `Cycle`, all under "Reactions for rows 220-226":**

1. Row 220 (line ~561):
   ```rust
   if !self.bt_new_window_sig.is_null() && ectx.IsSignaled(self.bt_new_window_sig) {
       enqueue_main_window_action(ectx, |mw, app| mw.Duplicate(app));
   }
   ```
   Replaces the existing 5-line `TODO(B-012-followup)` block + `log::info!`.

2. Row 221 (line ~569):
   ```rust
   if !self.bt_fullscreen_sig.is_null() && ectx.IsSignaled(self.bt_fullscreen_sig) {
       enqueue_main_window_action(ectx, |mw, app| mw.ToggleFullscreen(app));
   }
   ```
   Replaces the existing 4-line TODO + log.

3. Row 226 (line ~609):
   ```rust
   if !self.bt_quit_sig.is_null() && ectx.IsSignaled(self.bt_quit_sig) {
       enqueue_main_window_action(ectx, |mw, app| mw.Quit(app));
   }
   ```
   Replaces the existing 5-line TODO + log.

Add the import `use crate::emMainWindow::enqueue_main_window_action;` at the top of `emMainControlPanel.rs`.

## Reentrance and borrow safety

The pattern is already in production via `emBookmarks.rs:748` and `Duplicate(&self, app)`'s own internal use. Recapping for spec completeness:

- `ectx.pending_actions()` returns `&RefCell<Vec<...>>`. The `borrow_mut().push()` borrow ends within the statement.
- The main loop drains `pending_actions` with a take-pattern (`mem::take` or equivalent), releasing the outer borrow before invoking each closure.
- Inside each closure, calling `app.pending_actions.borrow_mut().push(...)` (as `Duplicate` does) re-pushes for the next drain — fine, outer borrow is released.
- `with_main_window` borrows `MAIN_WINDOW` thread_local mutably; `Quit` / `ToggleFullscreen` / `Duplicate` bodies do not recursively touch `MAIN_WINDOW`.

## Phase ordering (commit boundaries)

1. **Phase 1 — Helper.** Add `enqueue_main_window_action` to `emMainWindow.rs`. Verifies signature compiles; no callers yet.
2. **Phase 2 — Wire reactions + remove stubs.** Replace 3 TODO + log blocks with `enqueue_main_window_action` calls. Add the import. Mechanical, single commit.
3. **Phase 3 — Reconciliation.** Update FU-002 bucket file's closure section noting the architectural-decision phase was obviated (the existing pending_actions pattern is the answer). Verify `cargo-nextest ntr` and `cargo clippy -D warnings` clean.

## Testing

- **Helper:** no dedicated test (thin wrapper, established pattern).
- **Reactions:** if existing integration tests cover the F4 / F11 / Shift+Alt+F4 keyboard paths (`emMainWindow.rs:269/283/302`), the click paths now share the same downstream call. Run the full `cargo-nextest ntr` to confirm no regressions.
- Manual smoke: clicking the Fullscreen / NewWindow / Quit buttons produces the same observable behavior as the keyboard shortcuts.

## Acceptance criteria

- All 3 `TODO(B-012-followup)` markers in `emMainControlPanel.rs` removed.
- All 3 `log::info!` "requires App access" placeholders removed.
- `enqueue_main_window_action` helper present in `emMainWindow.rs`.
- Click and keyboard paths for Duplicate / Fullscreen / Quit produce equivalent observable behavior.
- `cargo-nextest ntr` green; `cargo clippy -D warnings` green; `cargo xtask annotations` clean.

## Out of scope

- Any new `EngineCtx::app()` accessor or `Rc<RefCell<App>>` registry — the existing pending_actions pattern is sufficient.
- B-003 follow-ups at `emAutoplayControlPanel.rs:715, 727` — different axis (model-to-widget push, not App-bound).
- Generalizing the helper to non-MainWindow targets — wait until a second use case appears.

## Tree-wide sweep result

The bucket file flagged that a sweep "may surface additional sightings." Sweep performed; result: exactly 3 sites in `emMainControlPanel.rs`, all enumerated above. Other `TODO` / `stub` markers in `emmain` (emAutoplayControlPanel.rs:715/727, emAutoplay.rs:227, emVirtualCosmos.rs:1287-1292) belong to different axes. No expansion of FU-002 scope.

## References

- C++ source: `~/Projects/eaglemode-0.96.4/src/emMain/emMainWindow.cpp`:
  - `emMainWindow::Duplicate()` — lines 98-129.
  - `emMainWindow::ToggleFullscreen()` — `SetWindowFlags(GetWindowFlags()^WF_FULLSCREEN)`.
  - `emMainWindow::Quit()` — `GetScheduler().InitiateTermination(0)`.
- Rust precedent for the deferred-action pattern:
  - `crates/emmain/src/emBookmarks.rs:748` (call-site shape).
  - `crates/emmain/src/emMainWindow.rs:233` (the pattern used internally by `Duplicate` itself).
- Existing keyboard paths that share the same downstream calls: `emMainWindow.rs:269` (F4 → Duplicate), `:283` (Shift+Alt+F4 → Quit), `:302` (F11 → ToggleFullscreen).
- Bucket file: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-002-app-bound-reactions.md`.

## Note on bucket-file framing

The FU-002 bucket file's first phase was "architectural decision required" listing three options. Research showed all three were preempted by the established `pending_actions` pattern. The bucket file should be updated during Phase 3 reconciliation with a closure section noting:

> The architectural decision phase was obviated. The `App.pending_actions` queue is the established Rust pattern for deferred-App-action work, already pervasive at the time FU-002 was scoped. The actual implementation is one helper plus three 1-line reaction wirings.

This is a reminder for future bucket files: phrase architectural-decision phases as "verify whether an existing pattern applies" before assuming a decision is needed.
