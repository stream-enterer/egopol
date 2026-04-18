# W3 — Popup Creation Architecture (Design)

**Date:** 2026-04-18
**Source:** `docs/superpowers/notes/2026-04-18-emview-followups-roadmap.md` (Wave 3)
**Origin:** `docs/superpowers/notes/2026-04-18-emview-followups-execution-debt.md` §2.2
**Scope:** Remove `PopupPlaceholder`; restore `emView::PopupWindow` to hold real `emWindow` instances; clear the four PopupPlaceholder-related `PHASE-6-FOLLOWUP:` markers.

**Marker accounting correction:** execution-debt §2.2 listed six `PHASE-6-FOLLOWUP:` markers; the animator-forward marker at `emView.rs:~3524` was cleared by Wave 1 commit `40d70ce`. Five markers currently remain: four are PopupPlaceholder-related and cleared by this wave. The fifth (`emView.rs:3611`, "VIF-chain + panel-broadcast dispatch migration from `emWindow::dispatch_input`") is about input-dispatch refactoring architecturally independent of popup; it was co-located in `emView.rs` with a `PHASE-6-FOLLOWUP:` prefix but is not popup work. **It survives W3 unchanged** and is flagged for a future input-dispatch wave. The roadmap's "5 cleared" phrasing incorrectly bundled it.

---

## Ideological frame

eaglemode-rs is an **observational port**. From any external vantage — user-visible behavior, event ordering, signals, focus transitions, emCore observers — the port must be indistinguishable from the C++ original. Below that surface:

1. **Forced divergence** — places where winit/wgpu genuinely cannot express the C++ original. Accepted grudgingly, minimized.
2. **Author's design intent** — deliberate architectural choices by an exceptional engineer. Preserved.

The test for "forced": not "awkward," not "would require refactoring" — impossible under winit's API. When a forced divergence exists, we reproduce the C++ author's intent as tightly as possible within the smallest-necessary concession.

## Problem

C++ `emView::RawVisitAbs` at `emView.cpp:1636` constructs the popup window synchronously inside the `VF_POPUP_ZOOM` branch:

```cpp
PopupWindow=new emWindow(*this, 0, emWindow::WF_POPUP, "emViewPopup");
UpdateEngine->AddWakeUpSignal(PopupWindow->GetCloseSignal());
PopupWindow->SetBackgroundColor(GetBackgroundColor());
SwapViewPorts(true);
if (wasFocused && !Focused) CurrentViewPort->RequestFocus();
```

After the call returns, `PopupWindow != NULL` and all side-effects are in place. Dozens of downstream emCore observers (`emView::IsPoppedUp()`, `IsViewed()`, close-signal drain at `emView.cpp:1299`, `emPanel.cpp:166,186`, `emDialog.cpp:248`, `emViewAnimator.cpp:389,821,914,1618`, `emViewInputFilter.cpp:720`, `emWindow.cpp:94`) depend on this atomic transition within the same `Update` pass.

The Phase-6 Rust port replaced this with a scaffolding type `PopupPlaceholder` because winit's `Window::new` requires `&ActiveEventLoop` + `&GpuContext`, only reachable inside `ApplicationHandler` callbacks. `emView::Update` runs under `rc.borrow_mut()` on the current window with no handle to the event loop or the framework's window map.

### Design-intent analysis

The synchronous `PopupWindow != NULL` after `RawVisitAbs` is **load-bearing design intent**, not a syntactic accident of X11:

- `IsPoppedUp()` (`emView.h:437, 945`) is a public emCore contract, consulted by multiple classes to drive navigation, animation, and input decisions.
- `IsViewed()` at `emView.cpp:917` uses `PopupWindow==NULL` as a tri-state discriminator for home-view panels within the same frame.
- The five side-effects at `emView.cpp:1636-1645` form an atomic popup-entry transition the author chose to express indivisibly.

The forced winit divergence is narrower than "popup creation as a whole" — it is only **OS surface creation** (winit `Window` + wgpu `Surface`), which must happen inside a callback carrying `&ActiveEventLoop`. The `emWindow` struct itself — close-signal, viewport, bg color, flags, geometry, signals — is a pure emCore-level entity with no winit dependency at construction time.

## Approach

Decouple what C++/X11 let the author conflate:
- **(A) The `emWindow` object** — emCore-level entity. Created synchronously from anywhere.
- **(B) The OS surface** — winit `Window` + wgpu `Surface`. Materialized in a winit callback.

`emView::RawVisitAbs` creates (A) synchronously, preserving every C++ observable. (B) materializes in the next `about_to_wait` drain via the existing `pending_actions` channel (already used for `Duplicate`/ccw). First paint of the popup lands one tick (~16.7 ms on 60 Hz) later — the same one-frame delay C++ users experience on X11 (`MapNotify` round-trip) and Wayland (compositor round-trip). Not perceptible to the human eye at the pixel level, and not observable from inside emCore at all.

## Architecture

### `emWindow` struct

`emWindow` currently holds `window: Arc<winit::Window>`, `surface`, `config`, etc. directly. Change: these move into a new enum:

```rust
enum OsSurface {
    Pending {
        flags: WindowFlags,
        caption: String,
        requested_pos_size: Option<(i32, i32, i32, i32)>,
    },
    Materialized {
        window: Arc<winit::window::Window>,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        // ...all other fields that depend on the materialized surface
    },
}
```

All non-surface `emWindow` fields (close-signal, viewport, bg color, flags, signals, `emView`) stay at struct level. They are populated at construction for both variants.

Construction paths that currently build a materialized `emWindow` keep doing so (non-popup windows materialize at framework startup via `resumed`). Only the popup path enters `Pending` initially.

### `emWindow` method behavior while `Pending`

- `SetViewPosSize(x, y, w, h)` — updates `requested_pos_size`. No winit call. Mirrors C++ `SetViewPosSize` which records intent for the X server.
- `SetBackgroundColor` — writes struct-level field (unchanged).
- `request_redraw()` — no-op. Materialization issues a `request_redraw()` once at the end of its drain.
- `render()` — unreachable (not in `fw.windows` until materialized); guarded by `debug_assert!(matches!(self.surface, OsSurface::Materialized { .. }))`.
- All four signals (`flags_signal`, `focus_signal`, `geometry_signal`, `close_signal`) — struct-level, fully functional.
- `view()` / `view_mut()` — unchanged.

### `emView::PopupWindow`

Type reverts from `Option<Rc<RefCell<PopupPlaceholder>>>` to **`Option<Rc<RefCell<emWindow>>>`** — exactly the C++ type (`emWindow * PopupWindow` at `emView.h:670`). The `PopupPlaceholder` type and file section are deleted.

### `emGUIFramework`

One new method:

```rust
fn materialize_popup_surface(
    &mut self,
    win_rc: Rc<RefCell<emWindow>>,
    event_loop: &ActiveEventLoop,
) {
    if Rc::strong_count(&win_rc) == 1 {
        // Popup was dropped before materialization ran. Cancel cleanly.
        return;
    }
    // 1. Extract Pending params from win_rc.borrow().surface.
    // 2. Construct winit::Window + wgpu::Surface via the same path
    //    emWindow::create uses for non-popup windows.
    // 3. Replace OsSurface::Pending with OsSurface::Materialized.
    // 4. Apply requested_pos_size if Some.
    // 5. Insert win_rc into self.windows under the new WindowId.
    // 6. win_rc.borrow().request_redraw().
}
```

Uses the existing `pending_actions: Vec<DeferredAction>` at `emGUIFramework.rs:322` — drained at the top of `about_to_wait`. No new deferral mechanism.

### Ownership during `Pending`

The `DeferredAction` closure holds `Rc<RefCell<emWindow>>` strongly. During `Pending`, the framework's pending queue owns the struct on behalf of the system — the popup doesn't yet have a home in `self.windows`. Once materialized, `self.windows` takes over ownership, matching today's pattern.

## Lifecycle

### Creation (enter popup mode) — in `emView::RawVisitAbs`, mirroring `emView.cpp:1628-1645`

1. Allocate `close_signal` on the scheduler.
2. Construct `emWindow` struct via new `emWindow::new_popup_pending(flags, caption, close_signal, bg_color, view_handle)` — populates every struct field, sets `surface: OsSurface::Pending { flags, caption, requested_pos_size: None }`. Synchronous, no winit dependency.
3. `self.PopupWindow = Some(rc.clone())`.
4. Enqueue `DeferredAction` capturing `rc.clone()` that invokes `fw.materialize_popup_surface(rc, event_loop)` on the next `about_to_wait`.
5. Continue C++ sequence: `UpdateEngine->AddWakeUpSignal(close_signal)`, `SetBackgroundColor`, `SwapViewPorts(true)`, conditional `CurrentViewPort->RequestFocus()`. All operate on struct fields.

Post-return invariants (observable from emCore): `IsPoppedUp() == true`, `IsViewed(home_panel) == false`, close-signal wired to update engine, bg color set, viewports swapped, focus transferred (or requested). Identical to C++.

### Per-frame geometry updates — `emView.cpp:1666-1674`

`PopupWindow->SetViewPosSize(x1, y1, x2-x1, y2-y1)` at `emView.rs` (currently lines ~1740+) writes to the struct. If `Pending`, updates `requested_pos_size` — applied at materialization. If `Materialized`, issues the winit resize immediately. Intent always captured.

### Destruction (exit popup mode) — in `emView::RawVisitAbs`, mirroring `emView.cpp:1676-1681`

1. `SwapViewPorts(true)`.
2. `let popup = self.PopupWindow.take()`.
3. If `Materialized`: enqueue `DeferredAction` to remove the entry from `fw.windows` on next `about_to_wait` (preserves symmetry, avoids dropping during iteration).
4. If `Pending`: do nothing extra — the pending-materialization closure's `Rc::strong_count(&win_rc) == 1` guard cancels cleanly when it runs.
5. `Signal(GeometrySignal)` — fires immediately.

From emCore observers' perspective, the popup ceases to exist synchronously.

### Close-signal path — `emView.cpp:1299`

`emView::Update`'s popup-close drain reads `self.PopupWindow` and checks the scheduler for the close-signal. Both the reference and the signal live on the struct, not the OS surface; the path works identically whether `Pending` or `Materialized`. A close triggered during the entry frame (before materialization) tears down cleanly.

## Data flow (popup entry, single frame)

```
Tick N about_to_wait:
  drain pending_actions (no popup action yet)
  scheduler.DoTimeSlice
  for window in self.windows:
    view.Update(tree)
      -> RawVisitAbs hits VF_POPUP_ZOOM + outside-home
         -> construct emWindow struct (Pending surface)
         -> self.PopupWindow = Some(rc)
         -> enqueue materialize_popup_surface(rc)
         -> wire close_signal, SetBackgroundColor, SwapViewPorts, RequestFocus
  (popup struct exists; emCore observers see IsPoppedUp() == true)

Tick N+1 about_to_wait:
  drain pending_actions
    -> materialize_popup_surface runs
    -> winit Window + wgpu Surface created
    -> self.windows.insert(popup)
    -> request_redraw on popup
  scheduler.DoTimeSlice
  for window in self.windows:  // now includes popup
    ...

Tick N+1 RedrawRequested for popup:
  first paint.
```

## Scaffold removal

- `PopupPlaceholder` struct, its `new_popup`, its `SetViewPosSize`, and associated impls in `emView.rs:10-70` — **deleted**.
- Four `PHASE-6-FOLLOWUP:` markers cleared:
  - `emView.rs:10` (PopupPlaceholder struct doc)
  - `emView.rs:33` (PopupPlaceholder `new_popup`)
  - `emView.rs:54` (PopupPlaceholder `SetViewPosSize`)
  - `emView.rs:1679` (RawVisitAbs call site — rewritten to construct real `emWindow`)
- `emView.rs:3611` (VIF-chain migration note) — **survives W3**. Input-dispatch refactor independent of popup architecture. Flagged for a separate future wave.
- `emView.rs:~3524` (`emView::Input` animator-forward) — already cleared by Wave 1 commit `40d70ce`; not present at W3 start.

## Testing

### `test_phase4_popup_zoom_creates_popup_window` — unchanged, now asserts the real invariant

The test assertion `v.PopupWindow.is_some()` after `RawVisit` was correct all along; it was blocked only by the scaffolding type. After this wave, `PopupWindow: Option<Rc<RefCell<emWindow>>>` and the test asserts the observational-port contract directly.

### `popup_window_creation_path_is_gated_on_display` — cleanup

Update to reference `emGUIFramework::materialize_popup_surface` function pointer (replacing the old `emWindow::new_popup` assertion). Remove the dead DISPLAY/WAYLAND_DISPLAY gate noted in execution-debt §4.6 — both branches ran unconditionally; simplify to the reachability assertion.

### New: `test_w3_popup_surface_materializes_on_about_to_wait`

Unit test. DISPLAY-gated (real gate — creating a `winit::Window` requires a display server). Body:

1. Construct framework with a home window.
2. Trigger `VF_POPUP_ZOOM` + a Visit that lands outside the home rect.
3. After the triggering `Update`: assert `PopupWindow.is_some()`, surface is `Pending`, `fw.windows` does not yet contain popup.
4. Run one `about_to_wait` pass with real `&ActiveEventLoop`.
5. Assert surface is `Materialized`, popup is in `fw.windows`, first `request_redraw` was issued.

### New: `test_w3_popup_dropped_before_materialization_cancels_cleanly`

Unit test. Does NOT require display (the cancellation path short-circuits before any winit call). Body:

1. Construct framework with a home window.
2. Trigger popup entry (popup Pending, materialization enqueued).
3. Before next `about_to_wait`, trigger popup exit (`PopupWindow.take()` path).
4. Run one `about_to_wait` pass.
5. Assert `fw.windows` unchanged, `pending_actions` drained, no winit window created, no panic.

### Golden tests — zero impact

No golden test currently exercises popup rendering. The 237/6 baseline is preserved.

### Phase-6-followup acceptance check

Plan's final acceptance phase greps for `PHASE-6-FOLLOWUP` in `crates/` and asserts exactly one remaining line: `emView.rs:3611` (VIF-chain migration, input-dispatch refactor unrelated to popup).

## Out of scope

- Wave 1 residuals (`emView::Input` animator-forward, `InvalidateHighlight` call sites, `PaintView`/`InvalidatePainting` re-entrancy doc comments, geometry-signal double-fire comment).
- Wave 5a — Phase-8 test promotion to a single real engine.
- Wave 5b — multi-window pixel tallness design.
- Any changes to `emWindow::new_popup` semantics beyond wiring it through the Pending/Materialized split (the helper at `emWindow.rs` stays; it's now invoked by `materialize_popup_surface` rather than by `RawVisitAbs`).
- Phase 11 visit-stack rewrite (Wave 4).

## Acceptance

- `PopupPlaceholder` type and file section deleted.
- `emView::PopupWindow: Option<Rc<RefCell<emWindow>>>`.
- Four `PHASE-6-FOLLOWUP:` markers cleared; one (`emView.rs:3611` VIF-chain, unrelated to popup) remains and is explicitly out of scope.
- `test_phase4_popup_zoom_creates_popup_window` passes unchanged.
- Two new tests pass (DISPLAY-gated materialization, non-gated cancellation).
- `cargo clippy -- -D warnings` clean.
- `cargo-nextest ntr` passes at or above the current 2409/2409 baseline (+2 new tests).
- Golden baseline 237/6 preserved.
- Smoke test (`timeout 20 cargo run --release --bin eaglemode`) exits 124 or 143.
