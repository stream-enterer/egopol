# emView Viewing Update: RawVisitAbs Rewrite

**Date:** 2026-04-17
**Status:** Approved for implementation (C++-parity auto-answers)

## Problem

Rust `emView::Update` recomputes viewing state every frame via a clear-then-rebuild
pattern (`clear_viewing_flags` + `compute_viewed_recursive`). This diverges from
C++ `emView::RawVisitAbs` (emView.cpp:1543–1808), which tracks
`SupremeViewedPanel` across frames and surgically updates only when the SVP or
viewed rect actually changes.

The divergence required a `prev_viewed` snapshot band-aid (commit b693d41) to
avoid firing `VIEW_CHANGED` notices every frame. Even with the band-aid:

- Runtime SIGSEGV within 3–6 s of startup (stack: `emSubViewPanel::Paint+356`,
  just after a `HandleNotice` call — suggests notice delivered at a
  semantically wrong moment corrupts sub-view lifecycle).
- Every-frame traversal of the entire panel tree is wasteful.
- Rust structure no longer overlays C++ 1:1 in the Update path.

## Goal

Replace the clear-then-rebuild recomputation with a transition-detecting port
of C++ `RawVisitAbs` + `emPanel::UpdateChildrenViewing` (emPanel.cpp:1454–1518).
Fire viewing-related notices only on actual transitions.

## Non-Goals

- Fixing the 8 pre-existing golden-test failures (`composition_*`,
  `notice_add_and_activate`, `notice_children_changed`, `notice_window_resize`,
  `testpanel_*`, `widget_file_selection_box`).
- Fixing the 6 pre-existing emcore lib test failures (emBorder/emPainter
  scaling).
- Separate investigation of any SIGSEGV that persists after this rewrite.

## Architecture

Two commits, each independently green.

### Commit 1 — `PanelTree::update_children_viewing`

Port C++ `emPanel::UpdateChildrenViewing` (emPanel.cpp:1454–1518) as
`pub(crate) fn update_children_viewing(&mut self, id: PanelId)` on `PanelTree`.

**Two branches, identical to C++:**

1. **Parent not Viewed**: iterate children; any child still `in_viewed_path`
   gets `viewed = false`, `in_viewed_path = false`, receives
   `VIEW_CHANGED | UPDATE_PRIORITY_CHANGED | MEMORY_LIMIT_CHANGED`, and recurses.
2. **Parent Viewed**: for each child compute absolute `viewed_x/y/width/height`
   from parent's viewed rect + child's `layout_rect`, intersect with parent's
   `clip_x1/y1/x2/y2` to produce the child's clip rect, set `viewed` based on
   non-empty clip, fire notice on transition, recurse.

No callers in this commit. Callable but unused; verified by cargo build.

### Commit 2 — Port `RawVisitAbs` into `emView::Update`

**State:** Ensure `supreme_viewed_panel: Option<PanelId>` (or existing `self.svp`)
persists across frames. If `self.svp` is already persistent, reuse it; mark
`DIVERGED:` if the name differs from C++ `SupremeViewedPanel`.

**Update body flow:**

1. Keep existing coord math (vx/vy/vw/vp via MaxSVP walk) — that produces the
   inputs C++ `RawVisitAbs` receives.
2. Change-detect: `forceViewingUpdate || svp_changed || viewed_rect_moved > 0.001`
   (matches C++ emView.cpp:1727–1752).
3. **No change** → early return; no traversal, no notices.
4. **Change**:
   - **Old SVP clear** (if prior SVP exists): `viewed = false`,
     `in_viewed_path = false`, notice, `update_children_viewing(old_svp)`.
     Walk parent chain clearing `in_viewed_path` with notice on each level
     until reaching a panel already off the path.
   - **New SVP set**: write `viewed = true`, `in_viewed_path = true`,
     `viewed_x/y/width/height`, `clip_x1/y1/x2/y2`, notice,
     `update_children_viewing(new_svp)`. Walk parent chain setting
     `in_viewed_path = true` with notices.
5. Update `self.svp = Some(new_svp)`.

**Preserve:** `zoomed_out_before_sg`, `in_active_path` propagation pass,
`svp_update_count`, `drain_navigation_requests`.

**Remove:**
- `clear_viewing_flags` (unless grep shows other callers — then shrink to the
  subset they need and comment the remaining coupling).
- `compute_viewed_recursive`.
- `PanelData.prev_viewed` field, its initialization, and its write in
  `clear_viewing_flags`.

## Divergences to Mark

- If `supreme_viewed_panel` field name differs from C++ `SupremeViewedPanel`:
  `DIVERGED:` comment at the field definition.
- If `update_children_viewing` signature differs from C++ (e.g., takes
  `&mut self` on the tree instead of running as a method on the panel):
  `DIVERGED:` comment at method definition noting C++ name is
  `emPanel::UpdateChildrenViewing` and reason is the Rust arena model.

## Verification

After each commit:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo build --release --bin eaglemode`
- `cargo test --release --test golden -- --test-threads=1`
  - Baseline: **235 passed, 8 failed**. Must not regress. Same 8 failures
    acceptable.
- `cargo test -p emcore --lib --release`
  - Baseline: **813 passed, 6 failed** (pre-existing).
- `cargo test --release --test pipeline --test behavioral`
  - Baseline: **378 + 312 passed, 0 failed**.
- Runtime smoke test:
  ```sh
  ulimit -c unlimited; rm -f /tmp/core.*
  env DISPLAY=:0 WAYLAND_DISPLAY=wayland-1 EM_DIR=/home/a0/git/eaglemode-rs \
      target/release/eaglemode 2>/tmp/rewrite.log &
  sleep 15; pgrep -af eaglemode; ls /tmp/core.* 2>&1
  WAYLAND_DISPLAY=wayland-1 grim -l 0 /tmp/shot.png
  ```
  Target: eaglemode alive after 15 s, no core dump, screenshot shows the
  `/home/a0` directory listing (bookmark `VisitAtProgramStart=yes`).

## Risks

1. **SVP `rel_a` convention mismatch.** Rust uses `rel_a = 1/ra`. Port formulas
   from C++ carefully at each site; unit-test individually if ambiguous.
2. **Notice ordering change.** Fewer/differently-timed notices may expose bugs
   in panel behaviors that incidentally depended on the every-frame firing.
   Mitigation: golden notice tests catch most; runtime smoke catches the rest.
3. **SIGSEGV may persist.** If so, it's a separate lifecycle bug in a sub-view
   panel's `notice()` handler, not in this rewrite. Flag as follow-up.
4. **`clear_viewing_flags` other callers.** Grep before removal.

## Pre-flight

- Revert or keep uncommitted instrumentation in `emPanelTree.rs` and
  `emViewAnimator.rs` (AE_NOTICE_TRACE / AE_ANIM_TRACE). Reverting keeps
  commits clean.
- Read `~/.claude/projects/-home-a0-git-eaglemode-rs/memory/MEMORY.md` and the
  four feedback files cited in the compaction prompt.
