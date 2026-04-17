# emView Viewing/Geometry Subsystem Rewrite — Design

**Date:** 2026-04-17
**Status:** Draft — pending user review
**Supersedes:** `docs/superpowers/plans/2026-04-17-rawvisitabs-followups.md` (all 5 tasks absorbed)
**Scope:** Single comprehensive spec. No follow-up specs.

## Problem Statement

The Rust port of `emView`'s viewing/geometry subsystem has accumulated structural drift from the C++ reference (`include/emCore/emView.h`, `src/emCore/emView.cpp`). A prior session landed a partial rewrite (`RawVisitAbs` body inlined into `Update`, change-block side effects added, `UpdateChildrenViewing` on `PanelTree`) but the underlying architecture still diverges from C++ at the field and method levels. A band-aid at `emView.rs:1181` forces `tree.set_pixel_tallness(1.0)` to compensate for a missing Home/Current viewport split. Knowledge of what is still missing is lossy across sessions and the follow-ups plan that captured it has itself become partially stale.

This spec closes the subsystem in one comprehensive pass — every C++ symbol in the viewing/geometry subsystem has a same-named Rust counterpart, every documented DIVERGED is explained at the point of divergence, and every Rust invention that substitutes for a missing C++ concept is either restored to the C++ name or removed in favor of the C++ concept.

## Goals

1. **File and Name Correspondence.** Every C++ field and method in the viewing/geometry subsystem has an identically named Rust counterpart (subject to existing camelCase-for-em-prefixed-types rules), or a `DIVERGED:` comment at the divergence point.
2. **Eliminate the pt band-aid.** Remove `tree.set_pixel_tallness(1.0)` at `emView.rs:1181` by restoring the Home/Current viewport split that owns `pixel_tallness` as a hardware property.
3. **Collapse `emView::Update` to C++ shape.** A single `Update()` method that performs the drain loop — notice draining, SVPChoiceByOpacityInvalid dispatch, SVPChoiceInvalid → RawVisitAbs, TitleInvalid, CursorInvalid — matching `emView.cpp:1292-1370`. The notice-ring drain itself legitimately lives on `PanelTree` (already ported) and `Update` calls through it; that split is a single DIVERGED.
4. **Audit and replace Sonnet 4.6 work.** Commits `bab81ec` and `3675687` were written by a lesser-quality model and are not trusted. Their diffs are re-verified against C++; correct lines are kept, incorrect lines are rewritten, the test in `3675687` is rewritten to assert C++ behavior rather than Rust's prior (possibly wrong) behavior.
5. **Close all five follow-ups Tasks.** Absorb Tasks 1–5 from `2026-04-17-rawvisitabs-followups.md`. No follow-up spec.

## Non-Goals

- Rewriting Rust-only animator, VIF, input-filter, or paint infrastructure except where a viewing/geometry method calls into it. (In particular: `Paint`, `Input` dispatch internals, and the `emViewAnimator` family are out of scope except for signatures and call sites.)
- Replacing the notice ring on `PanelTree` with a notice ring on `emView`. The ring was placed on `PanelTree` in commit `75c7c68` (Opus 4.6) for legitimate ownership reasons; that placement is preserved and documented as a DIVERGED.
- Extending the golden test suite. The existing suite is the primary regression oracle; no new goldens are added by this spec.

## Scope Verification

Every "missing from Rust" claim below was verified against HEAD by crate-wide grep before inclusion.

### Already landed — trusted (Opus 4.7, current session)

| Commit | Content |
|---|---|
| `430e7a7` | `UpdateChildrenViewing` on `PanelTree` |
| `96f78be` | Consolidate duplicate UpdateChildrenViewing |
| `28fad62` | Rename `svp` field → `supreme_viewed_panel` |
| `3770b00` | `RawVisitAbs` body inlined into `Update` (will be extracted to a named method in Phase 2) |

### Already landed — verify before building on (Opus 4.6)

| Commit | Content | Verification required |
|---|---|---|
| `75c7c68` | Notice ring + `HandleNotice` on `PanelTree` | Phase 1 Step 0: compare ring semantics against C++ `NoticeList` / `AddToNoticeList` / `emView::Update` inner loop. Confirm insertion order, unlink-before-dispatch, and wake-up signaling match C++. |

### Already landed — audit required (Sonnet 4.6)

| Commit | Content | Audit action |
|---|---|---|
| `bab81ec` | `cursor_invalid = true` and full-viewport dirty rect in RawVisitAbs change block | Phase 0: compare against `emView.cpp:1803-1806`. The Rust `InvalidatePainting()` analog (push `viewport_width × viewport_height` rect) is structurally suspect because C++ invalidates `CurrentX/Y/Width/Height` which are the *current*, possibly-popup-shifted rect — not the viewport dimensions. This line must be rewritten against the restored Home/Current split in Phase 1. |
| `3675687` | Test asserting the above | Phase 0: rewrite to assert C++ behavior. If C++ invalidates `CurrentX/Y/Width/Height`, the test asserts that rect — not `viewport_width × viewport_height`. |

### Still missing — grep-verified

Each row below was confirmed by crate-wide grep at HEAD. Zero occurrences found for each C++ symbol listed unless noted.

**Fields on `emView`:**

| C++ field | Rust status | Phase |
|---|---|---|
| `HomeX`, `HomeY`, `HomeWidth`, `HomeHeight`, `HomePixelTallness` | Missing. Rust has `viewport_width`, `viewport_height`, `home_pixel_tallness` only. | 1 |
| `CurrentX`, `CurrentY`, `CurrentWidth`, `CurrentHeight`, `CurrentPixelTallness` | Missing. | 1 |
| `SVPChoiceInvalid`, `SVPChoiceByOpacityInvalid` | Missing. Rust has `force_viewing_update` (partial analog) and `viewing_dirty` (Rust invention). | 1 |
| `MinSVP`, `MaxSVP` | Missing. Required for `SVPChoiceByOpacityInvalid` evaluation in Update. | 1 |
| `RestartInputRecursion`, `SettingGeometry`, `SVPUpdSlice` | Missing. | 1 |
| `LastMouseX`, `LastMouseY` | Missing. Required by CursorInvalid branch of Update (emView.cpp:1355). | 1 |
| `ViewFlagsSignal`, `FocusSignal`, `GeometrySignal` | Missing. Rust has `control_panel_signal`, `title_signal` only. | 1 |
| `ZoomScrollInAction` | Missing. | 1 |
| `PopupWindow: Option<WindowId>` | Missing. Rust `emWindow` has `WindowFlags::POPUP` defined but no emView-side popup window creation. | 4 |
| `HomeViewPort`, `CurrentViewPort`, `DummyViewPort` | Missing. Rust has no viewport indirection (no `emViewPort` class — separate port required, see D3a). | 4 |
| `ActiveAnimator`, `MagneticVA`, `VisitingVA` | Partial (animator pointers scattered). Consolidate to match C++ field names. | 1 |
| `FirstVIF`, `LastVIF` | Missing. | 5 |
| `UpdateEngine: UpdateEngineClass`, `EOIEngine: EOIEngineClass` | Missing. Rust has `eoi_countdown: Option<i32>` as ad-hoc replacement. | 5 |

**Methods on `emView`:**

| C++ method | Rust status | Phase |
|---|---|---|
| `RawVisitAbs(panel, vx, vy, vw, forceViewingUpdate)` | Body inlined into `Update`; not a named method. | 2 |
| `FindBestSVP`, `FindBestSVPInTree` | Missing. Required for SVP selection in RawVisitAbs (emView.cpp:1685). | 2 |
| `RawZoomOut(bool forceViewingUpdate)` (private overload) | Missing. Public `RawZoomOut()` exists. | 2 |
| `RawVisit(panel, relX, relY, relA, bool forceViewingUpdate)` (private overload) | Missing. Public `RawVisit` exists. | 2 |
| `SwapViewPorts(bool swapFocus)`, `GetMaxPopupViewRect(...)` | Missing. | 4 |
| `RecurseInput` (two overloads), `AddToNoticeList` on `emView`, `InvalidateHighlight` | Missing. | 5 |
| `SetViewPortTallness(tallness)` | Missing. | 6 |

**Architectural DIVERGEDs (to be documented in code, not closed):**

| Divergence | Rationale |
|---|---|
| Notice ring lives on `PanelTree`, not `emView` | C++ owns it on `emView` because C++ manages panels via raw pointers from `emView`. Rust stores panels in a `PanelTree` slotmap; the ring must live where the storage is. Commit `75c7c68` established this. |
| `emView::Update` calls `tree.HandleNotice(...)` instead of having the ring-drain inline | Direct consequence of the ring living on `PanelTree`. The Update drain loop's outer shape (notice → SVPChoiceByOpacityInvalid → SVPChoiceInvalid → Title → Cursor) is preserved exactly; only the "drain all pending notices" step delegates to the tree. |

### Cross-cutting gap — NoticeFlags reconciliation (absorbed Task 5)

C++ `emPanel.h:542-553` defines 10 `NoticeFlags` bits. Rust `emPanel.rs:173-201` defines 13 — the three extras are Rust inventions:

- `VISIBILITY` (Rust name for C++ `NF_VIEWING_CHANGED`) — rename to `VIEWING_CHANGED` to match C++.
- `VIEW_CHANGED` (Rust-only) — delete; consumers rewritten to use `VIEWING_CHANGED`.
- `CANVAS_CHANGED` (Rust-only; C++ folds canvas-color changes into `NF_VIEWING_CHANGED`) — delete; consumers rewritten to use `VIEWING_CHANGED`.
- `CHILDREN_CHANGED` (Rust name for C++ `NF_CHILD_LIST_CHANGED`) — rename to `CHILD_LIST_CHANGED`.

Bit values also renumbered to match C++ (`CHILD_LIST_CHANGED=1<<0`, `LAYOUT_CHANGED=1<<1`, `VIEWING_CHANGED=1<<2`, `ENABLE_CHANGED=1<<3`, `ACTIVE_CHANGED=1<<4`, `FOCUS_CHANGED=1<<5`, `VIEW_FOCUS_CHANGED=1<<6`, `UPDATE_PRIORITY_CHANGED=1<<7`, `MEMORY_LIMIT_CHANGED=1<<8`, `SOUGHT_NAME_CHANGED=1<<9`). Rust's current bit values diverge from C++ at every position.

Consumer sites (spec-write-time grep, non-load-bearing count) span `emmain`, `emfileman`, `emcore`, `emSubViewPanel`, `emVirtualCosmos`, `emMainPanel`, `emMainContentPanel`, `emDirEntryPanel`, `emDirEntryAltPanel`, `emDirPanel`, `emFileLinkPanel`, `emFileManSelInfoPanel`, `emPanelTree`, kani proofs, and golden test fixtures. Exact enumeration happens in Phase 7 commit.

### Active-path subsumed (absorbed Task 4)

The current Rust `Update` clears `in_active_path` and `is_active` on every panel on every frame (emView.rs:1306-1311). In C++, `ActivePath` is mutated only in `SetActivePanel` / `SetActivePanelBestPossible` — not in `Update`. The C++-shape `Update` rewrite in Phase 3 naturally eliminates the per-frame clear since C++ doesn't do it. No separate phase needed.

## Target Architecture

### Control flow (Update)

```text
emView::Update():
  loop:
    if (PopupWindow && close-signal-fired) { ZoomOut(); continue; }
    if (notice ring non-empty) {
      tree.HandleNotice(window_focused, current_pixel_tallness);  // DIVERGED: ring on PanelTree
      continue;
    }
    if (SVPChoiceByOpacityInvalid) {
      SVPChoiceByOpacityInvalid = false;
      if (!SVPChoiceInvalid && MinSVP != MaxSVP) {
        walk MinSVP..MaxSVP; if opacity change forces SVP re-choice, set SVPChoiceInvalid;
      }
      continue;
    }
    if (SVPChoiceInvalid) {
      SVPChoiceInvalid = false;
      let (panel, vx, vy, vw) = GetVisitedPanel().ViewedRect;
      RawVisitAbs(panel, vx, vy, vw, false);
      continue;
    }
    if (TitleInvalid) {
      TitleInvalid = false;
      recompute title from ActivePanel; if changed, invalidate_title();
      continue;
    }
    if (CursorInvalid) {
      CursorInvalid = false;
      recompute cursor from GetPanelAt(LastMouseX, LastMouseY); if changed, invalidate_cursor();
      continue;
    }
    break;
```

### Control flow (RawVisitAbs)

Port `emView.cpp:1543-1808` line-for-line:

1. Clear `SVPChoiceByOpacityInvalid`, `SVPChoiceInvalid`.
2. If `VF_NO_ZOOM`: substitute root panel; recompute `vx/vy/vw` against `CurrentWidth/Height/PixelTallness`.
3. Walk up ancestor chain clamping to `MaxSVPSize`.
4. Compute `vh` from `HomePixelTallness`.
5. If at RootPanel: apply Home-rect centering/clamping (emView.cpp:1588-1626).
6. If `VF_POPUP_ZOOM`: popup create/adjust/destroy branch — sets `forceViewingUpdate=true` on geometry change.
7. `FindBestSVP(&vp, &vx, &vy, &vw)` — walk chain picking best SVP candidate.
8. Compute `MinSVP` / `MaxSVP`.
9. Change-detect vs. previous SVP and rect. If unchanged and `!forceViewingUpdate`, return.
10. SVPUpdSlice throttling (fp-instability safeguard, emView.cpp:1734-1751).
11. Clear old SVP chain: `InViewedPath=0`, `Viewed=0`, notice (`VIEWING_CHANGED|UPDATE_PRIORITY_CHANGED|MEMORY_LIMIT_CHANGED`), `UpdateChildrenViewing`, walk parents clearing.
12. Set new SVP chain: fields, clip rect against Current*, notice, `UpdateChildrenViewing`, walk parents setting.
13. Side effects: `RestartInputRecursion = true`, `CursorInvalid = true`, `UpdateEngine->WakeUp()`, `InvalidatePainting()` on Current rect.

### Field layout (emView struct after rewrite)

Fields ordered to match C++ grouping (emView.h:680-715). Rust-only fields retained for Rust-specific needs are grouped separately at the end with a comment explaining why.

Rust-only field removals (replaced by C++ equivalents):
- `viewport_width`, `viewport_height` → `home_width`, `home_height` (+ `current_width`, `current_height`)
- `visited_vw`, `visited_vh` → derivable from `supreme_viewed_panel.viewed_width/height`; remove and update callers
- `viewing_dirty` → `svp_choice_invalid`
- `force_viewing_update` → remove as field; pass as method arg (matches C++ signature)
- `zoomed_out_before_sg` → `zoomed_out_before_sg` (already matches C++ `ZoomedOutBeforeSG`)
- `eoi_countdown` → replaced by `eoi_engine: EOIEngineClass`
- `needs_animator_abort` → DIVERGED (Rust-specific flow); retain with documented rationale
- `pending_animated_visit` → DIVERGED (Rust-specific flow); retain

### Method signatures

Every method that C++ has in the viewing/geometry subsystem is present on Rust `emView` by C++ name (camelCase preserved). Private-overload pairs (two methods with the same name, one public and one private with an extra `forceViewingUpdate` argument) are represented in Rust as two methods with the public one calling the private one. The private one keeps the name, prefixed or annotated only where Rust's no-overload rule forces a rename — in which case a `DIVERGED:` comment cites the collision.

Planned Rust signature naming for the two overload pairs:
- `RawVisit(panel, relX, relY, relA)` (pub) — wrapper
- `RawVisit(panel, relX, relY, relA, forceViewingUpdate)` — not expressible in Rust; **DIVERGED: Rust has no function overloading. Use a single `RawVisit(panel, relX, relY, relA, forceViewingUpdate: bool)` and have public callers pass `false`.** (Current public signature will be adjusted; callers updated in same phase.)
- `RawZoomOut()` (pub) — delegates to `RawZoomOut(false)`. Rust will mirror: `pub fn RawZoomOut(&mut self)` + `fn RawZoomOut_forced(&mut self, forceViewingUpdate: bool)` with a DIVERGED comment. Alternative: single method with default-arg wrapper — pick at implementation time, document choice.

## Design Decisions

### D1. Collapse `emView::Update` to C++ shape

C++ has one `Update()`. Rust currently has two behaviors split across `tree.HandleNotice` and `view.Update`. The public callers (`emGUIFramework` frame loop) will call `view.Update()` only; `view.Update` internally calls `tree.HandleNotice` for the notice-drain step. Single top-level entry, matching C++ callers. The sub-call to `tree.HandleNotice` is the only DIVERGED in the control flow.

### D2. Home/Current split restores pt as hardware property

`SetGeometry` takes explicit `pixelTallness` (matching C++ signature `emView.cpp:1238`). Constructor no longer derives pt from viewport aspect. `HomePixelTallness` and `CurrentPixelTallness` are separate fields; they differ only during popup. Every caller of `SetGeometry` that currently passes `(width, height)` is updated to pass `(x, y, width, height, pixelTallness)` explicitly. Caller enumeration happens in the phase 6 commit; count is not load-bearing in this spec.

### D3. Popup infrastructure is mandatory in scope

Per the "scope up on missing" rule, popup support is not deferred. Phase 4 ports:
- `emViewPort` class itself (C++ `emView.h:723`; currently no `emViewPort.rs`) — see D3a.
- `SwapViewPorts`, `GetMaxPopupViewRect` on `emView`
- `HomeViewPort` / `CurrentViewPort` / `DummyViewPort` fields
- `PopupWindow` field as `Option<Rc<RefCell<emWindow>>>` or equivalent
- emView→emWindow popup-creation call (`emWindow::new_popup(...)`) — if emWindow lacks the method, port it in same phase
- The popup branch in `RawVisitAbs` (emView.cpp:1628-1682)
- Popup close-signal wiring

### D3a. `emViewPort` port within Phase 4

C++ `emViewPort` is the view↔OS connection class. It owns `InvalidateCursor`/`InvalidatePainting`/`InvalidateTitle`/`RequestFocus`/`SetViewPosSize` etc. — methods that `emView` currently has open-coded bodies for. The Phase 4 port introduces the class, moves those method bodies onto it, and leaves thin delegating methods on `emView` (per C++). This is a substantial sub-port but the scope-up rule mandates it. If `emViewPort` proves large enough to deserve its own phase on drafting the plan, it may be split (by writing-plans) into Phase 4a (port `emViewPort`) and Phase 4b (popup wiring that uses it).

### D4. Audit Sonnet 4.6 work, replace as needed

Phase 0 is an audit-only phase — no new features. It reads `bab81ec` and `3675687` against C++ emView.cpp:1803-1806 and determines: keep, fix, or delete. Keep-as-is is the default only if the Sonnet lines exactly match C++ semantics; anything short of that is rewritten against the (yet-to-be-introduced) Home/Current split in later phases. The audit output is captured in the Phase 0 commit message.

### D5. NoticeFlags reconciliation is cross-cutting but bounded

Phase 8 renames ~4 bit names, renumbers 10 bits, rewrites ~15 consumer sites. Mechanical. Kani proof `proofs.rs:263` references the current bit layout and is rewritten. Golden tests don't depend on NoticeFlags bit *values* (only behaviors), so no golden regressions expected.

### D6. Active-path is subsumed by D1

The C++ `Update` doesn't touch `in_active_path` — `SetActivePanel` is the only mutator. Phase 3's rewrite of `Update` to C++ shape removes the per-frame clear at `emView.rs:1306-1311`. No separate phase.

## Phase Breakdown

Each phase is one commit. Each commit passes the pre-commit hook (`cargo fmt` + `cargo clippy -D warnings` + `cargo nextest run`).

**Phase 0 — Audit Sonnet 4.6 commits `bab81ec`, `3675687`.** Read-only phase (no code change). The commit is a docs note that records the audit outcome. If the audit finds bugs, the bugs are fixed inline in the phase that reaches them (the Phase 0 commit itself only documents findings).

**Phase 1 — Additive field-level port onto `emView`.** Strictly additive: adds Home/Current fields, SVPChoiceInvalid/ByOpacityInvalid, MinSVP/MaxSVP, RestartInputRecursion, SettingGeometry, SVPUpdSlice, LastMouseX/Y, three missing signals, ZoomScrollInAction — initialized in constructor, unused by method bodies (those are rewritten in later phases). Rust-invention fields (`viewport_width`, `viewport_height`, `visited_vw`, `visited_vh`, `viewing_dirty`, `force_viewing_update`) are **not** removed here; each is removed in the phase that rewrites its last reader (Phase 3 for `viewing_dirty`/`force_viewing_update`; Phase 6 for `viewport_width`/`height`/`visited_vw`/`vh`). Phase 8 is the final sweep. Verifies (Step 0) the notice ring from `75c7c68` against C++.

**Phase 2 — Extract `RawVisitAbs`, add `FindBestSVP`, `FindBestSVPInTree`.** Moves the RawVisitAbs body out of `Update` into a named method with C++ signature. Adds the two `FindBestSVP*` helpers. `RawVisit` / `RawZoomOut` private overloads introduced. At the end of Phase 2, `Update` still has SVP-recompute logic — Phase 3 collapses it.

**Phase 3 — Collapse `Update` to C++ drain loop.** Rewrite `Update` body to match `emView.cpp:1292-1370`: loop dispatching notice → SVPChoiceByOpacityInvalid → SVPChoiceInvalid (→ RawVisitAbs) → TitleInvalid → CursorInvalid. Per-frame active-path clear is removed (no C++ analog). Caller-trigger pathways also restored to match C++: `Scroll`/`Zoom` call `RawVisit(panel, rx, ry, ra, true)` directly (not via `SVPChoiceInvalid`); `SetGeometry` calls `RawZoomOut(true)` or `RawVisit(…, true)` directly; `emPanel::InvalidateViewing()` is the only path that flips `SVPChoiceInvalid` so a later `Update` call picks it up. `force_viewing_update` (the Rust field) is removed in favor of the `forceViewingUpdate` method argument it is intended to replicate.

**Phase 4 — Popup infrastructure.** Port `emViewPort` class; add `SwapViewPorts`, `GetMaxPopupViewRect`, `PopupWindow`, `HomeViewPort`/`CurrentViewPort`/`DummyViewPort` on `emView`; port popup branch in `RawVisitAbs` (emView.cpp:1628-1682). If `emWindow` lacks popup creation hooks, port them in same phase. (The plan may split this into Phase 4a/4b per D3a.)

**Phase 5 — `RecurseInput`, `InvalidateHighlight`, `AddToNoticeList`, `UpdateEngineClass`, `EOIEngineClass`.** Input-recursion + engine classes. `eoi_countdown` replaced by `EOIEngine`.

**Phase 6 — `SetGeometry` / `Scroll` / `Zoom` / `RawZoomOut` / `SetViewFlags` / constructor rewritten against new fields.** Removes the pt band-aid at `emView.rs:1181`. Every caller of `SetGeometry` updated to pass explicit `pixelTallness`. Adds `SetViewPortTallness` method.

**Phase 7 — NoticeFlags reconciliation.** Renames `VISIBILITY → VIEWING_CHANGED`, `CHILDREN_CHANGED → CHILD_LIST_CHANGED`; deletes `VIEW_CHANGED`, `CANVAS_CHANGED`; renumbers bits to C++ values. ~15 consumer sites updated. Kani proof updated.

**Phase 8 — Final reconciliation and cleanup.** Delete any Rust-invention field or method that by earlier phases has no remaining readers (i.e., was left alive during transition). Sweep for any `DIVERGED:` comments introduced in earlier phases that can now be closed. Audit the emView.rs line references quoted in this spec (e.g., "emView.rs:1181") for staleness — they were spec-write-time snapshots. No behavior changes; a zero-lines-of-behavior-diff commit is acceptable.

## Acceptance Criteria

### Per-phase (every phase-commit)

- Pre-commit hook passes without `--no-verify`.
- Golden: **≥235 passing, ≤8 failing, same failing set as baseline** (the 8 in `.config/nextest.toml`). A phase may unlock a previously-skipped test; if so, remove from skip list in that phase's commit.
- `emcore` lib: ≥819 passing, 0 failing.
- `pipeline` test crate: 0 failing.
- `behavioral` test crate: 0 failing.

### Runtime smoke (Phases 3 and 6, at minimum)

- `eaglemode` launches and stays ALIVE ≥ 15 seconds.
- No core dump.
- Manual visual check: cursor and paint refresh correctly when zooming into/out of a panel.

### Final (Phase 8 commit)

- All of the per-phase criteria, plus:
- `grep -rn 'viewport_width\|viewport_height\|visited_vw\|visited_vh\|viewing_dirty' crates/` returns zero hits outside `#[cfg(test)]` fixtures.
- `grep -rn 'NoticeFlags::VIEW_CHANGED\|NoticeFlags::VISIBILITY\|NoticeFlags::CANVAS_CHANGED\|NoticeFlags::CHILDREN_CHANGED' crates/` returns zero hits.
- `grep -n 'tree.set_pixel_tallness(1.0)' crates/emcore/src/emView.rs` returns zero hits.
- No `NOT PORTED` comments remain in the touched methods (per user's rule: no escape hatches).
- Every C++ `emView` method listed in the "still missing" scope table is present by name in `emView.rs`.

## In-Phase Discovery Rule

At implementation time, phases may discover additional missing C++ symbols not captured in the Scope Verification tables above. Per the "scope up on missing" feedback rule, such discoveries are **ported in the same phase that surfaces them, not deferred**. The phase commit message enumerates any in-phase additions so future sessions can audit them against the spec.

## Open Questions

Before writing-plans is invoked, the following must be resolved:

1. **Rust overload-collision naming.** For `RawVisit(…)` and `RawZoomOut(…)` where C++ has two overloads (public + private with extra `forceViewingUpdate` arg), pick one of: (a) single method with `bool` parameter and all callers pass explicit bool; (b) two methods with a naming scheme for the forced variant. Choice documented as DIVERGED either way. **Recommendation:** (a) — single `pub fn RawVisit(… , forceViewingUpdate: bool)` and `pub fn RawZoomOut(&mut self, forceViewingUpdate: bool)`. External callers that used the no-arg variant pass `false`.
2. **Popup window ownership.** C++ `PopupWindow` is a raw `emWindow *` with manual `new`/`delete`. Rust needs an owned handle. Options: `Option<Rc<RefCell<emWindow>>>` or `Option<Box<emWindow>>`. Constrained by existing `emWindow` construction patterns and lifetime requirements. Decide during Phase 4 prep.
3. **`UpdateEngineClass` / `EOIEngineClass` scheduler integration.** These are `emEngine` subclasses in C++. Rust `emEngine.rs` exists. Phase 5 needs confirmation that the Rust `emEngine` supports subclass-style `Cycle()` callbacks compatible with C++ expectations; if not, decide whether to extend `emEngine` or inline the engine's tick into `Update`.
4. **`emViewPort` scope boundary.** C++ `emViewPort` includes hardware-connection methods (`SetViewPosSize`, `RequestFocus`, input mapping, key-press raw). The Phase 4 port needs to decide what is in scope for the port of the class itself vs. what stays as `DIVERGED: not applicable to Rust's current windowing backend`. Recommendation: port the field definitions and signatures of all methods; implement bodies only for methods called by the emView viewing/geometry paths; document unimplemented methods with `DIVERGED:` citing the specific backend gap.

These are flagged explicitly so writing-plans can mark them as prerequisites rather than silently guess.

## Out of Scope

- `Paint`, `PaintHighlight`, paint-recursive machinery (except for `InvalidateHighlight` which is a state mutator).
- `emViewAnimator` family internals.
- `emViewInputFilter` internals (except `FirstVIF`/`LastVIF` pointers on `emView`).
- `emWindow` rewrites beyond popup creation hook (if even that is needed).
- Adding new golden tests.
