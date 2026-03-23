# Phase 9b: Diagnose 5 Persistent Visual Bugs (Round 2)

Diagnosis only. Do not fix anything. Do not modify source code. Read, trace, classify, report.

## Context

Round 1 diagnostics and fixes resolved: highlight rect geometry (Bug 8), font size (Bug 2), recursion depth (Bug 5), and a scheduler timing issue. Round 2 screenshots show progress but 5 bugs persist.

**View all screenshots first:**
- Rust round 2: `/home/ar/Pictures/Screenshots/rust_screenshots_round_21.png` (zoomed in, toolkit visible), `round_22.png` (zoomed out, toolkit + tiles), `round_23.png` (zoomed out, no selection)
- C++ reference: `/home/ar/Pictures/Screenshots/cpp-test-panel1.png`, `cpp-test-panel2.png`, `cpp-test-panel3.png`

**Screenshot-to-bug mapping:**

| Bug | Visible in | Compare against | Where to look |
|-----|-----------|----------------|---------------|
| R2-1 (content overflow) | round_21 vs cpp-test-panel1 | "Test Panel" title is cropped at top — the content extends above the viewport. C++ shows title fully visible with space above. |
| R2-2 (toolkit layout wrong) | round_22 vs cpp-test-panel2 | Toolkit Test area shows a 4x4 grid of buttons/checkboxes/radio buttons in Rust. C++ shows a structured layout: text fields, scalar fields, color field, list box, splitter dividing two halves, not a flat grid. |
| R2-3 (arrows wrong) | round_21 | Selection arrows around recursive tiles — arrows point outward (away from panel) instead of inward (toward panel). Color is white/grey, should be yellow. Spacing between arrows and panel edge, arrow count, and drop shadow all need comparison against C++. |
| R2-4 (BgColor widget) | round_22 vs cpp-test-panel2 | Lower-right: C++ has small Background Color swatch. Rust still missing. |
| R2-5 (wheel zoom) | Not visible | Mouse wheel still does nothing. Round 1 diagnosed: animate_wheel() exists but is never called from main loop. |

**Key source files (from round 1 diagnosis):**
- TestPanel: `examples/test_panel.rs`
- View highlight painting: `src/panel/view.rs:1928+`
- VIF wheel handling: `src/panel/input_filter.rs:442+` (animate_wheel), `input_filter.rs:588+` (filter)
- App main loop: `src/window/app.rs:253+` (about_to_wait)
- View zoom: `src/panel/view.rs:515+` (raw_zoom_out)

**C++ reference source:**
- TestPanel: `~/.local/git/eaglemode-0.96.4/src/emTest/emTestPanel.cpp`
- View highlight: `~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp:2149-2479`
- VIF: `~/.local/git/eaglemode-0.96.4/src/emCore/emViewInputFilter.cpp`

## Triage Order

**Tier 1 — Previously diagnosed, fix apparently not applied:**
- R2-5 (wheel zoom) — Round 1 found the root cause (animate_wheel never called). Verify: is the fix present? If not, this is a "fix not applied" issue, not a new diagnosis.
- R2-4 (BgColor widget) — Round 1 found two issues (tiny widget + stub create_control_panel). Verify: were either fixed?

**Tier 2 — Partially fixed, need deeper investigation:**
- R2-1 (content overflow) — The aspect ratio fix was applied but is incorrect. Need to compare the actual `raw_zoom_out` / `ROOT_SAME_TALLNESS` implementation against C++.
- R2-3 (arrows wrong) — Arrows now appear (progress from round 1 where they were absent), but direction, color, spacing, count, and shadow are all wrong. Detailed comparison needed.

**Tier 3 — Structural:**
- R2-2 (toolkit layout) — The toolkit widget is now visible (progress from round 1 where it was collapsed), but the layout is a flat grid instead of a structured widget showcase. Deep comparison of child panel hierarchy needed.

## Bugs to Diagnose

### R2-1: Content overflow — viewport doesn't contain root panel

**What is observed:** The "Test Panel" title is cropped at the top of the viewport. Content extends above and below the visible area. In C++, the root panel fits entirely within the viewport with the title fully visible.

**What is known:** Round 1 diagnosed this as `raw_zoom_out` hardcoding `rel_a=1.0` and missing `ROOT_SAME_TALLNESS` handling. A fix was attempted but the result is still wrong.

**Diagnostic steps:**
1. Read `src/panel/view.rs` — find `raw_zoom_out` and `is_zoomed_out`. Read the current implementation. Has it been changed from the hardcoded `rel_a=1.0`?
2. Read the C++ `RawZoomOut()` at `emView.cpp:1811-1825`. Copy the exact formula. Compare line by line against the Rust implementation.
3. Check `set_view_flags` for `ROOT_SAME_TALLNESS` handling. Read C++ `SetViewFlags` at `emView.cpp:145-150`. Compare.
4. Specifically check: what values does the Rust code use for `HomeWidth`, `HomeHeight`, `HomeTallness`, and `PixelTallness`? Are these computed correctly? `grep -rn 'home_width\|home_height\|home_tallness\|pixel_tallness\|HomeWidth\|HomeHeight' src/panel/view.rs`.
5. Decision tree:
   - If `raw_zoom_out` still uses `rel_a=1.0` → the fix was never applied.
   - If `raw_zoom_out` has a formula but it doesn't match C++ → the formula is wrong. Record both formulas.
   - If the formula matches but the input values (`HomeTallness`, `PixelTallness`, viewport dimensions) differ → the inputs are wrong. Record them.
   - If none match → set confidence below 0.3, `needs_manual_review: true`, move on.

### R2-2: Toolkit Test layout is a flat grid instead of structured widget showcase

**What is observed:** Rust shows a 4x4 grid of identical-looking buttons, checkboxes, and radio buttons. C++ shows a structured layout with two halves divided by a splitter — left half has text fields, scalar fields, color field; right half has list box and other widgets. The structure, widget types, and arrangement are all different.

**What is known:** Round 1 diagnosed this as flat sibling hierarchy instead of nested sp → sp1/sp2 → t1a/t1b/t2a/t2b. A fix was attempted but the result is still wrong — the widget is visible now but shows wrong content.

**Diagnostic steps:**
1. Read the current TkTestGrpPanel (or equivalent) in `examples/test_panel.rs`. Find its `auto_expand` method. List every child it creates and what type each child is.
2. Read C++ `emTestPanel.cpp` — find `TkTestGrp::AutoExpand()`. List every child and its type. The C++ creates:
   - `sp`: an `emSplitter` dividing two halves
   - `sp1`/`sp2`: children of `sp`, each a group panel
   - `t1a`: `emTextField` (in sp1)
   - `t1b`: `emScalarField` + `emColorField` (in sp1)
   - `t2a`: `emListBox` (in sp2)
   - `t2b`: various buttons/checkboxes (in sp2)
3. Compare: does Rust create the same widget types? Or does it create generic buttons/checkboxes for everything?
4. Check the parent-child hierarchy. In C++, `sp1` and `sp2` are children of `sp` (the splitter). Are they in Rust?
5. Decision tree:
   - If Rust creates different widget types (all buttons instead of TextField, ScalarField, ColorField, ListBox) → the auto_expand was rewritten incorrectly. Record what was created vs what should be.
   - If Rust creates correct types but in wrong hierarchy (flat instead of nested under splitter) → hierarchy fix from round 1 was incomplete.
   - If Rust creates correct types in correct hierarchy but the layout widget (RasterLayout vs Splitter) is wrong → the layout container is wrong.
   - If none match → set confidence below 0.3, `needs_manual_review: true`, move on.

### R2-3: Selection arrows face outward, wrong color, wrong spacing/count/shadow

**What is observed:** Selection arrows now appear (round 1 had plain white rect). But:
- Arrows point outward (away from the selected panel) instead of inward (toward it)
- Arrows are white/grey instead of yellow
- Arrow count, spacing from panel edge, and drop shadow all appear different from C++

**What is known:** The `PaintHighlight` with arrow polygons was ported in round 1 fixes. The implementation has errors in direction, color, and geometry.

**Diagnostic steps:**
1. Read the Rust `paint_highlight` implementation (start at `src/panel/view.rs:1928`). Find the arrow polygon generation code.
2. Read the C++ `PaintHighlight` at `emView.cpp:2149-2479`. Specifically find:
   - `PaintHighlightArrow()` — what angle/direction are arrows generated at? Which way do they point relative to the border edge?
   - What color is used? Search for color constants — C++ likely uses a yellow/gold highlight color. `grep -n 'Highlight.*Color\|ArrowColor\|0x.*FF' emView.cpp`.
   - What spacing between arrows and the panel edge? What gap between consecutive arrows?
   - What drop shadow parameters (offset, blur, color, opacity)?
3. Compare arrow direction: C++ arrows should point inward toward the panel center. If Rust arrows point outward, the angle is inverted (off by 180 degrees or the polygon vertices are in reversed order).
4. Compare color: find the exact C++ color value. Then `grep -rn 'highlight.*color\|arrow.*color\|0xFFFF\|Color::' src/panel/view.rs` in Rust.
5. Compare count/spacing: how does C++ compute the number of arrows per edge? Is it based on edge length? What's the minimum spacing?
6. Compare drop shadow: does C++ call `PaintRect` or `PaintRoundedRect` with a shadow offset before drawing the arrow? What are the shadow parameters?
7. Decision tree:
   - Arrow direction wrong → the angle parameter in the polygon generation is inverted. Record the C++ angle vs Rust angle.
   - Color wrong → wrong color constant. Record both values.
   - Spacing/count wrong → wrong computation for arrow placement along edges. Record both formulas.
   - Shadow wrong → missing or wrong shadow paint call. Record C++ shadow parameters.
   - If none match → set confidence below 0.3, `needs_manual_review: true`, move on.

### R2-4: Background Color widget still missing

**What is observed:** No change from round 1.

**Diagnostic steps:**
1. Check if any fix was applied since round 1. `grep -rn 'create_control_panel\|control_panel\|BgColor\|bg_color.*field' examples/test_panel.rs src/panel/view.rs`.
2. If `create_control_panel` in `view.rs` still returns None → stub never completed. Record this as "round 1 fix not applied."
3. If it was changed but the control panel still doesn't appear → trace the call path: when does the view call `create_control_panel`? Is it on panel visit? Is the panel being visited?

### R2-5: Mouse wheel zoom still doesn't work

**What is observed:** No change from round 1.

**Diagnostic steps:**
1. Check if the round 1 fix was applied. `grep -rn 'animate_wheel\|animate_grip\|vif.*tick\|vif.*animate' src/window/app.rs src/window/zui_window.rs`.
2. If `animate_wheel` is still never called from the main loop → round 1 fix not applied. Record this.
3. If it IS called: add `grep -rn 'wheel_active\|wheel_spring' src/panel/input_filter.rs` — is the spring state being updated correctly? Check if `wheel_active` ever becomes true during runtime.
4. Decision tree:
   - If animate calls were never added → fix not applied.
   - If animate calls exist but wheel_active is never set → the event path from winit to VIF filter is broken. Re-trace from `MouseWheel` handler.
   - If wheel_active is set and animate runs but no zoom occurs → the zoom delta computation produces zero. Check `raw_scroll_and_zoom` parameters.
   - If none match → set confidence below 0.3, `needs_manual_review: true`, move on.

## Cross-Bug Analysis

After diagnosing all 5:
1. **R2-1 + R2-2:** Both involve the root panel's layout/viewport. If the aspect ratio fix is wrong (R2-1), it could affect how child panels are laid out, which could contribute to R2-2's wrong arrangement. Check if fixing R2-1 alone would change R2-2's appearance.
2. **R2-4 + R2-5:** Both are "round 1 fix not applied" candidates. If both are simply not applied, the root cause is process-level (fixes were diagnosed but not committed), not code-level.
3. Check for any unexpected connections between the 5 bugs.

## Output

Write to `state/run_003/visual_bug_diagnosis_r2.json` using the same schema as `visual_bug_diagnosis.json`:

```json
{
  "diagnosed_at": "<ISO8601>",
  "bugs": [
    {
      "bug_id": "R2-N",
      "title": "<short title>",
      "root_cause": "<specific description>",
      "category": "<layout|input|rendering|missing_feature|view_state|resource_loading>",
      "rust_source": "<file:line(s)>",
      "cpp_reference": "<file:line(s)>",
      "confidence": "<0.0-1.0: 0.8+ if causal chain found; 0.5-0.8 if area found but gap in chain; <0.5 if inferring>",
      "ruled_out": ["<max 2, only actually investigated>"],
      "verification_prediction": "<falsifiable claim>",
      "related_bugs": [],
      "round1_status": "<not_applied|partially_fixed|new_issue>",
      "diagnostic_complete": true,
      "needs_manual_review": false,
      "manual_review_reason": null
    }
  ],
  "cross_bug_analysis": { "shared_root_causes": [] }
}
```

## Rules

- Do not modify any source files.
- Do not attempt fixes.
- Grep first, then read 30+ lines of context. Read full functions.
- Try 2+ broader search patterns before concluding "doesn't exist."
- Confidence reflects causal chain quality, not grep count.
- If fix planning enters your reasoning, stop and move on.
- All 5 bugs must appear in output.
- For each bug, explicitly state whether the round 1 fix was applied, partially applied, or not applied.
