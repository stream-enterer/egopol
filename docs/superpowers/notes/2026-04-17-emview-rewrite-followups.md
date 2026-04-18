# emView Rewrite — Follow-up Items

Captured 2026-04-17 at the close of the 9-phase emView viewing/geometry
subsystem rewrite (commits `bfccade`..`68c6c59`). Each item below is
out-of-scope for that plan but should land in a follow-up plan or one-off.

---

## Backend gaps (require windowing/scheduler integration)

These are sanctioned by the plan's "backend-gap" carve-out. They cannot
be closed without a real OS-window backend and a scheduler signal bus.

### `emViewPort` — backend method stubs (`crates/emcore/src/emViewPort.rs`)
All marked `PHASE-5-TODO: backend …`. None of these run today; the
viewing/geometry path uses only the geometry accessors, focus state, and
`SetViewPosSize` (popup placement).
- `PaintView` (line 138) — backend compositing hook
- `GetViewCursor` (131) — backend cursor query
- `IsSoftKeyboardShown` (172) / `ShowSoftKeyboard` (180) — touch platform hooks
- `GetInputClockMS` (187) — currently returns 0; should return real monotonic ms
- `InputToView` (197) — backend → view input dispatch
- `InvalidateCursor` (205) — backend cursor dirty flag
- `InvalidatePainting` (213) — backend dirty-rect dispatch

### `emWindow` popup stubs (`crates/emcore/src/emWindow.rs`)
The Phase 4 popup `emWindow` struct (line 1422) is a placeholder; no
real OS window is created.
- `new_popup` (1441) — should create an actual OS popup window
- `SetViewPosSize` (1473) — should forward to OS window resize

### Scheduler-signal wiring (`crates/emcore/src/emView.rs`)
- Popup-close drain in `Update` (line 2122) — needs
  `IsSignaled(PopupWindow->GetCloseSignal())`; today the check is a
  `backend-gap:` comment
- `SwapViewPorts` Phase-4 popup branch (1616) — `// PHASE-5-TODO: wire close-signal wake-up`
- `SwapViewPorts` GeometrySignal fire (1675) — `// PHASE-5-TODO`
- `EOIEngineClass` / `UpdateEngineClass` (lines 206, 245) — `Cycle()` is
  not driven by a real scheduler; today they're constructed but never
  ticked outside test harness `tick_eoi`
- `GetMaxPopupViewRect` (2955) — falls back to the home rect because
  there is no real monitor backend

---

## Pre-existing structural drift (surfaced during the rewrite, not caused by it)

### `ZuiWindow` vs `emWindow` parallel types (`crates/emcore/src/emWindow.rs`)
- C++ `emWindow` is the heavyweight window class
- Rust historically named the heavyweight type `ZuiWindow` (line 37,
  ~1300 LOC); a new minimal `emWindow` struct (line 1422) was added in
  Phase 4 to satisfy popup wiring without restructuring `ZuiWindow`
- F&N rule violation: the file is `emWindow.rs` but the primary type is
  `ZuiWindow`. Two parallel structs now coexist
- Follow-up plan should either rename `ZuiWindow` → `emWindow` (deleting
  the stub) and route the stub's call sites through the renamed type,
  or carve out `emWindow.rs` into `emWindow.rs` + `emWindow_stub.rs`
  with a `SPLIT:` marker

### `svp_update_count` missing `DIVERGED:` annotation (`crates/emcore/src/emView.rs:192`)
- C++ name is `SVPUpdCount`. Rust uses snake_case without a
  `DIVERGED:` comment per F&N rule
- Sibling `SVPUpdSlice` (added Phase 1) keeps the C++ name, making the
  inconsistency conspicuous
- Either rename to `SVPUpdCount` (matching `SVPUpdSlice`) or add a
  `DIVERGED:` comment justifying snake_case

### `home_pixel_tallness` (Rust-invention, kept for compatibility)
- Phase 1 added the C++-named `HomePixelTallness` field; the prior
  Rust-invention `home_pixel_tallness` was retained with a cross-reference
  comment per the Phase 1 review
- Both fields are now read by different paths (mostly different files)
- Phase 6 was scheduled to remove `home_pixel_tallness` but the field
  still has internal readers; the removal was deferred
- Follow-up: audit each `home_pixel_tallness` reader, route them through
  `HomePixelTallness`, then remove the duplicate

### `current_pixel_tallness` on `PanelTree` (`crates/emcore/src/emPanelTree.rs:303`)
- Currently always 1.0 (initialized in `PanelTree::new`, no write path)
- Read by `RawVisitAbs` child-update logic (line 1168) and `emPanelCtx`
  (line 2474)
- Phase 6 removed the `tree.set_pixel_tallness(1.0)` band-aid in
  `Update` but did not add a write path for non-1.0 values
- Follow-up: at the start of `Update`, set
  `tree.current_pixel_tallness = view.CurrentPixelTallness` (or thread
  the value through the call chain so `PanelTree` doesn't need a mutable
  pt field at all)

---

## Test gaps (low risk; would harden against future regression)

### `invariant_equilibrium_at_target` skip at factor=1.0 (`crates/emcore/src/emViewAnimator.rs:3320`)
- Documented as a "KNOWN GAP (TODO phase 8)"
- At factor=1.0, root-centering clamps `viewed_x=0` regardless of the
  visit-stack `rel_x`. Rust's visit stack has no C++ analogue (C++
  derives rel coords from `ViewedX/Y` on every read)
- Two ways out:
  1. Have Rust derive rel coords from `ViewedX/Y` on every read
     (matching C++) and delete the visit stack
  2. Document the visit-stack semantics rigorously and accept the gap
- (1) is the F&N-correct choice but is a non-trivial refactor

### `InvalidateHighlight` weak guard (`crates/emcore/src/emView.rs`)
- Phase 5 implementation uses `self.active.is_some()` as a proxy for
  "active panel is viewed"
- C++ guard checks `ActivePanel->Viewed` and `VFlags`
- Tighten once the borrow flow allows reading the panel's `viewed`
  state inside `InvalidateHighlight` without a re-borrow conflict

---

## Closed deliberately (not follow-ups, just for the record)

The following surfaced during reviews but were judged correct as-shipped:
- Phase 1 review: `MinSVP`/`MaxSVP` constructor order swap → fixed
- Phase 1 review: `home_pixel_tallness`/`HomePixelTallness` cross-ref → added
- Phase 2 review: parity test "identity-only" weakness → strengthened with
  `test_phase2_raw_visit_abs_root_centering`
- Phase 2 review: `invariant_animator_convergence` retargeted to a
  ta>1.0 path so root-centering does not fire (exact convergence asserted)

---

## Acceptance state at plan close

- 2403/2403 tests pass on merged main (`68c6c59`)
- 235/243 golden (8 pre-existing baseline failures unchanged)
- Runtime smoke: `eaglemode --release` stays ALIVE ≥15s, no core dump
- Pre-commit hook clean throughout
- All 9 C++ methods from the spec's "Still missing" table present in
  `emView.rs` (RawVisitAbs, FindBestSVP, FindBestSVPInTree, SwapViewPorts,
  GetMaxPopupViewRect, RecurseInput, AddToNoticeList, InvalidateHighlight,
  SetViewPortTallness)

Refs: `docs/superpowers/specs/2026-04-17-emview-viewing-subsystem-design.md`
      `docs/superpowers/plans/2026-04-17-emview-viewing-subsystem-rewrite.md`

---

## CLOSED (partial) — 2026-04-18

10 of 11 follow-up items resolved across Phases 1–10 on `main`.
Phase 11 (visit-stack removal) deferred — see below.

- Spec: `docs/superpowers/specs/2026-04-17-emview-rewrite-followups-design.md` (commit `9f149ba`)
- Plan: `docs/superpowers/plans/2026-04-17-emview-rewrite-followups.md` (commit `407b620`)
- Delivered commits: `fc57a6a`..`4528b84` (13 commits including 3 follow-up DIVERGED cleanups)

Acceptance at close: 2409/2409 nextest; 237/243 golden (same 6 pre-existing
failures as the `68c6c59` baseline, no regression); runtime smoke alive
≥15s on each phase.

### Phases delivered

| Phase | Item | Commits |
|-------|------|---------|
| 1 | Rename `svp_update_count` → `SVPUpdCount` | `fc57a6a` |
| 2 | Remove `home_pixel_tallness` duplicate | `c272962`, `b5f9c42` |
| 3 | Remove `PanelTree::current_pixel_tallness`; thread from view | `5caa1ab` |
| 4 | Rename `ZuiWindow` → `emWindow`; drop popup stub | `386c90c`, `282cc57` |
| 5 | `emViewPort` 7-method backend wiring | `f1f6de0`, `866d51b` |
| 6 | Real popup `emWindow`, `Rc<RefCell<>>` windows, `GeometrySignal` | `c053689` |
| 7 | `impl emEngine` for EOI/Update; scheduler-driven | `33a44e7` |
| 8 | Popup-close drain + `SwapViewPorts` wake-up | `009a094` |
| 9 | `GetMaxPopupViewRect` from current monitor | `d10c636` |
| 10 | `InvalidateHighlight` guard tightened to C++ | `4528b84` |

### Phase 11 — DEFERRED

Task: delete `visit_stack`/`VisitState`; derive `rel_x/y/a` from
`ViewedX/Y/Width/Height` on every read (C++ `emPanel.cpp:608-617`).

Blocker (discovered during Phase 11 audit): the plan's helper snippet
references `self.ViewedX/Y/Width/Height` on `emView`, but those fields
live on `PanelTree::Panel` (as `viewed_x/y/width/height`), not on
`emView`. Deriving on `&self` alone is not possible. Resolving the gap
requires one of:

1. Thread `&PanelTree` through ~30 call sites (including
   `emViewInputFilter`, `emViewAnimator`, `emMainWindow`,
   golden/unit/support tests) — invasive, breaks many signatures.
2. Add view-level `ViewedX/Y/Width/Height` cache fields on `emView` kept
   in sync with the supreme viewed panel — architectural change not in
   the current plan.

Additional scope friction: `Visit`/`go_back`/`go_home` is a Rust-only
navigation API with no C++ equivalent, backed by `visit_stack`. The
unit test `view_visit_and_back` in `crates/eaglemode/tests/unit/panel.rs`
directly asserts on `visit_stack().len()` and `.panel`. Deleting the
stack either breaks those APIs or demands a replacement back-stack data
type — not specified in the plan.

Known consequence of the deferral: the `invariant_equilibrium_at_target`
animator test still skips `factor=1.0` with the existing `KNOWN GAP`
marker at `crates/emcore/src/emViewAnimator.rs:~3320`.

**Follow-up:** needs a separate plan that explicitly chooses between
(1) or (2) above, specifies the `Visit/go_back/go_home` replacement
API, and covers the unit-test migration.

### Residual markers in the tree

- `PHASE-6-FOLLOWUP:` on `PopupPlaceholder` (`emView.rs`) — popup
  creation deadlocks because `RawVisitAbs` has no `ActiveEventLoop`
  handle; Phase 6 kept the scaffold with these markers.
- `PHASE-6-FOLLOWUP:` on `emView::Input` animator-forward site and the
  VIF-chain migration note.
- `KNOWN GAP` in `emViewAnimator.rs` at the `factor=1.0` skip (see above).
- `UPSTREAM-GAP:` on `IsSoftKeyboardShown`/`ShowSoftKeyboard` in
  `emViewPort.rs` — matches absence of backend override in upstream
  Eagle Mode.
