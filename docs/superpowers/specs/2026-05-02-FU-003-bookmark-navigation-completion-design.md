# FU-003 — bookmark navigation completion

**Bucket:** [FU-003](../../debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-003-emview-multiview-port.md) (rescoped 2026-05-02 from "emView multi-view port" — see bucket file's scope correction preamble).
**Date:** 2026-05-02
**Scope:** `emmain` (emBookmarks, emMainWindow).
**Prereqs:** none.

## Summary

The bucket originally framed this as a multi-thousand-line emView port. Research showed the multi-view infrastructure (`emSubViewPanel`, `emView::VisitByIdentity`, sub-view dispatch via `home_tree.with_behavior_as::<emSubViewPanel>`) is already ported and is already used correctly in `emBookmarks.rs`'s click reaction. Two sites still don't use it: the bookmark *hotkey* handler (`emMainWindow.rs:337`) is a `log::info!` BLOCKED stub, and the DIVERGED comment at `emBookmarks.rs:723-733` is partly stale. Total: ~30 LoC.

## Design intent (per Port Ideology)

The Rust port preserves C++'s sub-view design intent — `emMainPanel` owns two `emSubViewPanel` children, each holding a real `emView` instance, dispatched into via `visit_by_identity`. This bucket simply propagates that working pattern to two more call sites; no new structural decisions.

## Work units

Two units, one commit per unit:

### Unit A — bookmark hotkey wire-up (emmain)

**File:** `crates/emmain/src/emMainWindow.rs:330-344`.

**Change:** replace the `log::info!` BLOCKED stub at line 337 with a dispatch that mirrors the click-reaction closure pattern at `emBookmarks.rs:748-780`. The hotkey handler is already inside `handle_input(&mut self, event, input_state, app: &mut App)` — `&mut App` is in scope. So the dispatch can run **synchronously** rather than going through `pending_actions`:

```rust
// Bookmark hotkey (C++ emMainWindow.cpp:247-260).
if let Some(ref bm_model) = self.bookmarks_model
    && let Some(hotkey) = Hotkey::from_event_and_state(event.key, input_state)
{
    let hotkey_str = hotkey.to_string();
    let bm = bm_model.borrow();
    if let Some(rec) = bm.GetRec().SearchBookmarkByHotkey(&hotkey_str) {
        let identity = rec.entry.LocationIdentity.clone();
        let rel_x = rec.entry.LocationRelX;
        let rel_y = rec.entry.LocationRelY;
        let rel_a = rec.entry.LocationRelA;
        let subject = rec.entry.Name.clone();
        drop(bm); // release borrow before app mutation

        if let Some(main_panel_id) = self.main_panel_id
            && let Some(content_view_id) = app.home_tree_mut()
                .with_behavior_as::<crate::emMainPanel::emMainPanel, _>(
                    main_panel_id,
                    |mp| mp.GetContentViewPanelId(),
                )
                .flatten()
        {
            app.with_home_tree_and_sched_ctx(|tree, sc| {
                tree.with_behavior_as::<emcore::emSubViewPanel::emSubViewPanel, _>(
                    content_view_id,
                    |svp| {
                        svp.visit_by_identity(
                            &identity, rel_x, rel_y, rel_a, true, &subject, sc,
                        );
                    },
                );
            });
        }
        return true;
    }
}
```

The synchronous-vs-deferred difference from the click path is justified: hotkey handler already has `&mut App`, so the deferral that the click path needs (Cycle has only `&mut EngineCtx`) is unnecessary here.

**Verify during implementation:**
- `Hotkey::from_event_and_state` already returns the right type (it's used in the existing stub at line 332).
- `with_home_tree_and_sched_ctx` and `home_tree_mut` are the same accessors used by the click path; reuse.
- The borrow on `bm = bm_model.borrow()` must be released before `app.home_tree_mut()` — confirm by stepping through borrows.

**Tests:** existing handle_input tests cover keyboard paths; if a bookmark-hotkey integration test exists, run it. Otherwise, manual smoke: configure a bookmark with a hotkey, press the hotkey, observe the same navigation as a click on the bookmark button.

### Unit B1 — DIVERGED comment correction (emmain)

**File:** `crates/emmain/src/emBookmarks.rs:723-733`.

**Change:** the existing comment claims "Rust `emView` has not ported the multi-view content/control split, so the C++ `emBookmarkButton::ContentView` field [...] has no Rust counterpart." The first half is wrong (multi-view infrastructure is ported and the dispatch below uses it). Rewrite to describe the *actual* remaining divergence — per-bookmark configurable target view (multi-window installs only):

```rust
// DIVERGED: (upstream-gap-forced) — In C++, each `emBookmarkButton`
// holds a configurable `ContentView*` pointer that lets individual
// bookmarks target a specific emView (e.g., a different window's
// content view). Rust hardcodes "the home window's content sub-view"
// for all bookmarks. Single-window installs are observably
// equivalent; multi-window installs that configure per-bookmark
// targeting diverge. Tracked as future-work item B2 in the scratch
// dump (`docs/scratch/2026-05-02-future-work-dump.md`); trigger to
// schedule is "someone actually uses multi-window bookmark targeting."
// The dispatch below correctly uses the ported `emSubViewPanel.visit_by_identity`
// path for the home-window case (matches C++ `emBookmarks.cpp:1523-1535`).
```

No code change beyond the comment; behavior is identical.

**Tests:** none — comment-only edit. Verify `cargo xtask annotations` accepts the rewritten DIVERGED block (category cite preserved).

## Phase ordering (commit boundaries)

1. **Phase 1 — Unit A** (hotkey wire-up). Removes one BLOCKED comment.
2. **Phase 2 — Unit B1** (DIVERGED accuracy). Comment-only.
3. **Phase 3 — Reconciliation.** Update FU-003 bucket file's closure section confirming both sites resolved; verify `cargo xtask annotations` and `cargo-nextest ntr` clean.

Phases independent; can land in any order, but A is the substantive one.

## Acceptance criteria

- `emMainWindow.rs:337` BLOCKED comment removed; bookmark hotkey navigates via `emSubViewPanel.visit_by_identity` (same downstream path as the click reaction).
- `emBookmarks.rs:723-733` DIVERGED comment accurately describes current state.
- `cargo-nextest ntr` green; `cargo clippy -D warnings` green; `cargo xtask annotations` clean.

## Out of scope

- B2 (per-bookmark target view) — speculative, multi-window-only. Scratch-dump entry.
- C (emFileMan select_all ContentView introspection) — different axis. Scratch-dump entry.
- Other BLOCKED / upstream-gap-forced markers found during the FU-003 sweep — unrelated axes (animator registry, mutation API, editing UI, rendering, attributes, orientation, viewport delegation, emContext introspection, CPU TSC). Recorded in scratch.

## References

- C++ source: `~/Projects/eaglemode-0.96.4/src/emMain/emMainWindow.cpp:247-260` (bookmark hotkey dispatch); `src/emMain/emBookmarks.cpp:1470,1523-1535` (emBookmarkButton ContentView field and click reaction).
- Rust precedent: `crates/emmain/src/emBookmarks.rs:748-780` — working multi-view dispatch pattern. Unit A copies/adapts.
- emView Visit API: `crates/emcore/src/emView.rs:1094-1199` (`Visit`, `VisitByIdentity`, etc.).
- emSubViewPanel: `crates/emcore/src/emSubViewPanel.rs:157-168` (`visit_by_identity`).
- Bucket file: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-003-emview-multiview-port.md`.
