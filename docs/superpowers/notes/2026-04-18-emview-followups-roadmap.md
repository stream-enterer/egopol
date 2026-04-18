# emView Followups — Sequencing Roadmap

**Date:** 2026-04-18
**Source:** `docs/superpowers/notes/2026-04-18-emview-followups-execution-debt.md`
**Purpose:** Order the residual workstreams from the execution-debt report into waves. This is a sequencing document, not a design — each wave that needs design enters its own brainstorm → spec → plan cycle.
**Strategy:** Debt-down-first. Knock out bounded C++-mirror ports and audits before touching architectural workstreams, so the harder waves land against a calmer tree.

---

## Wave 1 — C++-mirror residual ports

**Shape:** one small spec → one plan, bundling four bounded items that all have the form "we didn't port what C++ does."

| Item | Source | Description |
|---|---|---|
| W1a | §3.3 | Port `emView::Input` animator-forward — mirror `emView.cpp:1004` `ActiveAnimator->Input(event, state)` |
| W1b | §4.8 | Audit C++ `InvalidateHighlight()` call sites; mirror them in Rust |
| W1c | §3.1 | Doc comments on `PaintView`/`InvalidatePainting` warning callers against holding `rc.borrow_mut()` when calling. Full re-entrancy audit deferred until real callers wire. |
| W1d | §3.2 | One-line comment in `emView.rs:1733` tying the GeometrySignal double-fire to `emView.cpp:1678 + 1995` |

**Why first:** zero architecture; builds C++-mirror discipline; clears 4 review findings and 1 `PHASE-6-FOLLOWUP:` marker cheaply.

---

## Wave 2 — Minor cleanups housekeeping pass

**Shape:** no spec; single PR with direct edits.

| Item | Source | Description |
|---|---|---|
| §4.3 | `emSubViewPanel.rs:48` | Add comment tying literal `1.0` to `CurrentPixelTallness` initial value |
| §4.4 | `emGUIFramework.rs:~393` | Simplify redundant `RefMut` rebind |
| §4.5 | `emGUIFramework.rs` `dispatch_forward_events` | Fix or remove misleading doc comment |
| §4.6 | `tests/unit/popup_window.rs` | Remove dead DISPLAY/WAYLAND_DISPLAY gate |
| §4.7 | `emGUIFramework.rs` | Hold the `let mut win = rc.borrow_mut();` style consistently |
| §7.4 | `emViewAnimator.rs:~3320` | Refresh `KNOWN GAP (TODO phase 8)` comment text to reference the current phase number |

**Parallelizable with W1.**

---

## Wave 3 — Popup creation architecture

**Shape:** brainstorm → spec → plan.

**Source:** §2.2.

**Architectural decision required.** Choose between:
1. Rewrite `test_phase4_popup_zoom_creates_popup_window` to tolerate asynchronous popup creation (poll across a scheduler tick).
2. Add a synchronous popup-slot API that returns a handle immediately while actual window creation is deferred.
3. Pre-materialize a pool of OS windows at framework startup and assign one to the popup on `RawVisit`.

**Outcomes when complete:** `PopupPlaceholder` scaffold removed; the remaining 5 `PHASE-6-FOLLOWUP:` markers in `emView.rs` cleared (the W1c marker on `Input` does not belong to this wave).

**Why before Wave 4:** Phase 11 touches `RawVisitAbs` indirectly. Settling popup architecture first means the visit-stack rewrite doesn't have to re-reason about popup interaction mid-flight.

---

## Wave 4 — Phase 11 visit-stack rewrite

**Shape:** brainstorm → spec → plan. Most invasive workstream in the queue.

**Source:** §1.

**Brainstorm must resolve, in order:**
- §1.1 — Data-model choice: thread `&PanelTree` through ~30 call sites *or* add view-level `Viewed{X,Y,Width,Height}` cache fields on `emView` synced from the supreme viewed panel each `Update`/`SwapViewPorts`.
- §1.2 — Fate of `Visit`/`go_back`/`go_home`: delete outright (loses Rust-only nav), replace backing store with absolute-Viewed snapshots, or keep stack for nav while deriving rel coords.
- §1.4 — Fate of `pending_animated_visit` (separate field; not covered by original plan scope).
- §1.5 — Pre-existing writer inconsistency at `emView.rs:861` (Visit writes rel coords without updating Viewed first). Decide: pre-existing bug to fix, or behavior to preserve.
- §8.3 — Migrate or delete `view_visit_and_back` in `tests/unit/panel.rs:140-165`.
- §1.6 — Re-add `factor=1.0` to `invariant_equilibrium_at_target` in `emViewAnimator.rs:~3320`.

**Resolved input:** §1.3 — `VisitState.panel` is semantically "active panel" (equals `self.active`). No further investigation needed.

---

## Wave 5 — Finishers

**Shape:** two small specs. Parallelizable with each other after Wave 4 lands.

### W5a — Phase-8 test promotion
**Source:** §2.3.
Drive `close_signal` end-to-end through a single real engine. Sub-decision: widen `PanelTree::get_mut` visibility (with downstream consequences) vs. build a test-support shim.

### W5b — Multi-window pixel-tallness semantics
**Source:** §3.4.
Either document the current `windows.values().next().unwrap_or(1.0)` behavior with a TODO marker, or design a real multi-window resolution. Choice depends on whether multi-window is on the near-term roadmap.

---

## Sequencing summary

```
W1 ─┬─ W2  →  W3  →  W4  →  W5a
    │                       └ W5b
    └ (W2 may run alongside W1)
```

- W1: small spec, small plan
- W2: direct edits, no spec
- W3: full brainstorm cycle (architecture)
- W4: full brainstorm cycle (architecture, largest)
- W5: two small specs, parallelizable

Only W3 and W4 require new brainstorming sessions. W1 enters writing-plans directly with a one-page spec. W2 is opportunistic. W5 spec depth is small.
