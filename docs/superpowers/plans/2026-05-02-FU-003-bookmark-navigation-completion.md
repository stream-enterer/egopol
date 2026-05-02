# FU-003 Bookmark Navigation Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the bookmark hotkey handler in `emMainWindow` to the existing `emSubViewPanel.visit_by_identity` dispatch (mirroring the click reaction in `emBookmarks::Cycle`) and rewrite the stale DIVERGED comment in `emBookmarks.rs` so it accurately describes the remaining (per-bookmark target view) divergence.

**Architecture:** Two near-trivial edits in `crates/emmain/src/`. Unit A converts a `log::info!` BLOCKED stub at `emMainWindow.rs:330-344` into a synchronous dispatch (the hotkey handler already owns `&mut App`, so it bypasses the `pending_actions` rail that the click reaction needs). Unit B is a comment-only rewrite at `emBookmarks.rs:723-733`. No new types, no behavioral changes outside the hotkey path.

**Tech Stack:** Rust, existing `emcore`/`emmain` crates, `emSubViewPanel::visit_by_identity`, `EngineCtx`, `with_home_tree_and_sched_ctx`, `with_behavior_as`. Test harness: `cargo-nextest ntr`. Annotation lint: `cargo xtask annotations`.

**Non-interactive defaults chosen by planner:**
- No new bookmark-hotkey integration test added (the spec explicitly says "if a bookmark-hotkey integration test exists, run it. Otherwise, manual smoke …"). A targeted unit test against `handle_input` would require constructing a full `App` plus tree; defer per spec's manual-smoke direction. Acceptance therefore falls on existing nextest suite + `cargo xtask annotations` + manual smoke described in the spec.
- Phase ordering: implement Unit A first (substantive), then Unit B (comment), then Phase 3 reconciliation, each as its own commit.
- Bucket file update is included as Phase 3.

---

## File Structure

| File | Modification |
| --- | --- |
| `crates/emmain/src/emMainWindow.rs` | Replace BLOCKED `log::info!` stub at lines 330-344 with synchronous `visit_by_identity` dispatch. |
| `crates/emmain/src/emBookmarks.rs` | Rewrite DIVERGED comment at lines 723-733 to describe the actual residual divergence (per-bookmark configurable target view). No code change. |
| `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-003-emview-multiview-port.md` | Append a closure section noting both sites resolved. |

No files created. No splits. Behavior preserved on all paths except the hotkey, which moves from no-op-with-log to active navigation.

---

## Task 1: Unit A — Bookmark Hotkey Wire-Up

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs:330-344`

- [ ] **Step 1: Read the current stub and the click-reaction precedent**

Open and read for context (no edit yet):
- `crates/emmain/src/emMainWindow.rs:300-348` — current handler structure, confirm `&mut App` is in scope and `Hotkey::from_event_and_state` is the right call.
- `crates/emmain/src/emBookmarks.rs:721-781` — the click-reaction dispatch that already uses `with_home_tree_and_sched_ctx` + `with_behavior_as::<emSubViewPanel>` + `visit_by_identity`. Unit A copies this pattern but runs synchronously.
- `crates/emcore/src/emSubViewPanel.rs` (search for `visit_by_identity`) — confirm signature `(identity: &str, rel_x: f64, rel_y: f64, rel_a: f64, adherent: bool, subject: &str, sc: &mut SchedCtx)`.

Expected: confirms the spec's API claims; no changes to types needed.

- [ ] **Step 2: Replace the BLOCKED stub with the synchronous dispatch**

In `crates/emmain/src/emMainWindow.rs`, replace lines 330-344 (the existing `// Bookmark hotkeys (C++ emMainWindow.cpp:247-260)` block through the closing `}` of that `if let` chain) with:

```rust
        // Bookmark hotkeys (C++ emMainWindow.cpp:247-260).
        if let Some(ref bm_model) = self.bookmarks_model
            && let Some(hotkey) = Hotkey::from_event_and_state(event.key, input_state)
        {
            let hotkey_str = hotkey.to_string();
            // Resolve the bookmark record under a short-lived borrow, then
            // drop the borrow before mutating `app` (which re-borrows the
            // bookmarks model transitively via the home tree). Mirrors the
            // click-reaction path in `emBookmarks.rs:721-781`, but runs
            // synchronously because `&mut App` is already in scope here
            // (the click path defers via `pending_actions` only because
            // `Cycle` lacks `&mut App`).
            let dispatch = {
                let bm = bm_model.borrow();
                bm.GetRec()
                    .SearchBookmarkByHotkey(&hotkey_str)
                    .map(|rec| {
                        (
                            rec.entry.LocationIdentity.clone(),
                            rec.entry.LocationRelX,
                            rec.entry.LocationRelY,
                            rec.entry.LocationRelA,
                            rec.entry.Name.clone(),
                        )
                    })
            };
            if let Some((identity, rel_x, rel_y, rel_a, subject)) = dispatch {
                if let Some(main_panel_id) = self.main_panel_id
                    && let Some(content_view_id) = app
                        .home_tree_mut()
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

Notes for the implementer:
- The borrow on `bm_model.borrow()` is dropped at the end of the `let dispatch = { ... };` block. Do not collapse this into a single `if let` chain — `app.home_tree_mut()` walks the home tree and may transitively touch the bookmarks model.
- Do not change the surrounding lines (the `autoplay_view_model` block above and the trailing `false` return below stay as-is).
- The `return true;` mirrors the BLOCKED stub's existing `return true;` — handled = consumed.

- [ ] **Step 3: Build and lint**

Run:
```bash
cargo check -p emmain
```
Expected: clean compile. If `with_behavior_as` requires a different turbofish path (e.g. `emcore::emSubViewPanel::emSubViewPanel` vs `crate::emSubViewPanel::emSubViewPanel`), match the path used in `emBookmarks.rs:770` exactly — that's the proven precedent.

Then:
```bash
cargo clippy -p emmain -- -D warnings
```
Expected: zero warnings. If clippy flags the nested `if let` ladder, accept its rewrite **only if** it preserves the borrow-then-drop ordering above.

- [ ] **Step 4: Run the targeted nextest filter**

Run:
```bash
cargo-nextest ntr -p emmain
```
Expected: all `emmain` tests pass. The hotkey path has no dedicated test, so this confirms no regression in surrounding handlers (`autoplay_view_model.Input`, the trailing `false` fall-through).

- [ ] **Step 5: Annotation lint**

Run:
```bash
cargo xtask annotations
```
Expected: clean. (The BLOCKED comment was removed; no new annotation added.)

- [ ] **Step 6: Commit Unit A**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "$(cat <<'EOF'
fix(emMainWindow): wire bookmark hotkey to emSubViewPanel.visit_by_identity

Replaces the BLOCKED log::info! stub at emMainWindow.rs:337 with a
synchronous dispatch mirroring the click-reaction pattern in
emBookmarks.rs:721-781. The hotkey handler already owns `&mut App`,
so the deferred `pending_actions` rail used by the Cycle-based click
path is unnecessary here.

Refs FU-003 plan, Unit A.
EOF
)"
```

Expected: commit lands. Pre-commit hook runs fmt + clippy + nextest; if it fails, fix the underlying issue and create a NEW commit (do not amend).

---

## Task 2: Unit B — DIVERGED Comment Correction

**Files:**
- Modify: `crates/emmain/src/emBookmarks.rs:723-733`

- [ ] **Step 1: Verify the DIVERGED block boundaries**

Read `crates/emmain/src/emBookmarks.rs:721-735` to confirm the exact text of the DIVERGED comment. The block as of this plan starts at line 724 with `// DIVERGED: (upstream-gap-forced) — Rust \`emView\` has not` and ends at line 733 with `// split lands.`. The implementer must match the exact lines at edit time; if they shifted, locate by content.

- [ ] **Step 2: Replace the comment text**

Replace the DIVERGED block (the lines beginning `// DIVERGED: (upstream-gap-forced) — Rust \`emView\` has not` through `// split lands.`, inclusive) with:

```rust
            // DIVERGED: (upstream-gap-forced) — In C++, each
            // `emBookmarkButton` holds a configurable `ContentView*`
            // pointer (`emBookmarks.cpp:1470`) that lets individual
            // bookmarks target a specific `emView` (e.g. a different
            // window's content view). Rust hardcodes "the home window's
            // content sub-view" for all bookmarks. Single-window installs
            // are observably equivalent; multi-window installs that
            // configure per-bookmark targeting diverge. Tracked as
            // future-work item B2 in the scratch dump
            // (`docs/scratch/2026-05-02-future-work-dump.md`); trigger to
            // schedule is "someone actually uses multi-window bookmark
            // targeting." The dispatch below correctly uses the ported
            // `emSubViewPanel::visit_by_identity` path for the home-window
            // case (matches C++ `emBookmarks.cpp:1523-1535`).
```

Preserve indentation (the surrounding `if !sig.is_null() && ectx.IsSignaled(sig) {` block uses 12-space indent for comments inside it). Do not modify any code below the comment — `let identity = self.bookmark.LocationIdentity.clone();` and the `pending_actions` push remain unchanged.

- [ ] **Step 3: Build and annotation lint**

```bash
cargo check -p emmain
cargo xtask annotations
```
Expected: both clean. The annotation lint must accept the rewritten DIVERGED block — it preserves the `(upstream-gap-forced)` category tag, which is the linter's required field.

- [ ] **Step 4: Run targeted nextest**

```bash
cargo-nextest ntr -p emmain
```
Expected: all `emmain` tests pass. (Comment-only edit, so this is a sanity check that nothing was accidentally touched.)

- [ ] **Step 5: Commit Unit B**

```bash
git add crates/emmain/src/emBookmarks.rs
git commit -m "$(cat <<'EOF'
docs(emBookmarks): correct stale DIVERGED note on multi-view dispatch

The previous DIVERGED comment claimed Rust hadn't ported the multi-view
content/control split. That's no longer accurate — emSubViewPanel and
emView::VisitByIdentity are ported and used by the dispatch immediately
below the comment. Rewrites the note to describe the actual residual
divergence: per-bookmark configurable ContentView* pointers
(emBookmarks.cpp:1470), which are a multi-window-only feature deferred
to future-work item B2.

Refs FU-003 plan, Unit B.
EOF
)"
```

Expected: commit lands.

---

## Task 3: Reconciliation

**Files:**
- Modify: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-003-emview-multiview-port.md`

- [ ] **Step 1: Open the bucket file and locate its closure section**

Read the FU-003 bucket file. If it has a "Status" or "Closure" section near the top or bottom, append to it; otherwise, add a new `## Closure (2026-05-02)` section at the end.

- [ ] **Step 2: Append the closure note**

Append (or create) at the file's tail:

```markdown
## Closure (2026-05-02)

Both call sites identified in the rescoped spec are resolved:

- `crates/emmain/src/emMainWindow.rs:330-344` — bookmark hotkey now
  dispatches synchronously through `emSubViewPanel::visit_by_identity`,
  matching the click-reaction path. BLOCKED comment removed.
- `crates/emmain/src/emBookmarks.rs:723-733` — DIVERGED comment
  rewritten to accurately describe the residual per-bookmark
  configurable-target-view divergence (multi-window only; tracked as
  future-work B2 in `docs/scratch/2026-05-02-future-work-dump.md`).

Out-of-scope items (B2, FileMan select_all ContentView introspection,
and other unrelated BLOCKED markers found during the FU-003 sweep)
remain in the scratch dump.

Acceptance gates run at closure: `cargo-nextest ntr` green, `cargo
clippy -- -D warnings` green, `cargo xtask annotations` clean.
```

- [ ] **Step 3: Run the full acceptance gate**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask annotations
cargo-nextest ntr
```
Expected: all three clean. If `cargo-nextest ntr` is slow on this machine, that's the project's standard final gate (per `CLAUDE.md`); do not skip.

- [ ] **Step 4: Commit reconciliation**

```bash
git add docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-003-emview-multiview-port.md
git commit -m "$(cat <<'EOF'
docs(FU-003): record closure of bookmark navigation bucket

Both target sites resolved (hotkey wire-up + DIVERGED comment
correction). Out-of-scope follow-ups remain in the scratch dump.

Refs FU-003 plan, Phase 3.
EOF
)"
```

Expected: commit lands.

---

## Acceptance Criteria

Cross-reference with spec §"Acceptance criteria":

- [x] `emMainWindow.rs:337` BLOCKED comment removed; bookmark hotkey navigates via `emSubViewPanel::visit_by_identity`. → Task 1.
- [x] `emBookmarks.rs:723-733` DIVERGED comment accurately describes current state. → Task 2.
- [x] `cargo-nextest ntr` green, `cargo clippy -D warnings` green, `cargo xtask annotations` clean. → Task 3 Step 3.

## Out of Scope (per spec)

- B2 (per-bookmark target view, multi-window-only) — scratch-dump entry, not implemented here.
- C (emFileMan `select_all` ContentView introspection) — different axis, scratch-dump entry.
- Other BLOCKED / upstream-gap-forced markers found during the FU-003 sweep (animator registry, mutation API, editing UI, rendering, attributes, orientation, viewport delegation, emContext introspection, CPU TSC).

## References

- C++ source: `~/Projects/eaglemode-0.96.4/src/emMain/emMainWindow.cpp:247-260` (bookmark hotkey dispatch); `src/emMain/emBookmarks.cpp:1470,1523-1535` (`emBookmarkButton::ContentView` field and click reaction).
- Rust precedent for the dispatch pattern: `crates/emmain/src/emBookmarks.rs:748-780` (deferred via `pending_actions`); Task 1's variant runs synchronously since `&mut App` is in scope at the hotkey site.
- emView Visit API: `crates/emcore/src/emView.rs` (`Visit`, `VisitByIdentity`).
- emSubViewPanel: `crates/emcore/src/emSubViewPanel.rs` (`visit_by_identity`).
- Spec: `docs/superpowers/specs/2026-05-02-FU-003-bookmark-navigation-completion-design.md`.
- Bucket file: `docs/debug/audits/2026-04-27-signal-drift-tier-b/remediation/followups/FU-003-emview-multiview-port.md`.
- Scratch dump for deferred items: `docs/scratch/2026-05-02-future-work-dump.md`.
