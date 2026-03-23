# Phase 9a: Diagnose 8 Visual Bugs in TestPanel Standalone

Diagnosis only. Do not fix anything. Do not modify source code. Read, trace, classify, report.

## Context

Zuicchini is a Rust port of Eagle Mode's emCore UI toolkit. The port is complete (84/84 capabilities verified). A standalone binary launches a window displaying a `TestPanel` — a complex widget tree that exercises every emCore feature. Comparing the Rust window against the C++ original reveals 8 visual/behavioral differences.

**Before starting: View all screenshots** with the Read tool:
- C++ reference: `/home/ar/Pictures/Screenshots/cpp-test-panel1.png`, `cpp-test-panel2.png`, `cpp-test-panel3.png`, `cpp-test-panel4.png`
- Rust: `/home/ar/Pictures/Screenshots/rust-test-panel1.png`, `rust-test-panel2.png`, `rust-test-panel3.png`
- Side-by-side at smaller window: `/home/ar/Pictures/Screenshots/Screenshot_20260316_081608.png`

**Screenshot-to-bug mapping:**

| Bug | Visible in | Where to look |
|-----|-----------|---------------|
| 1 (collapsed widget) | rust-test-panel1 vs cpp-test-panel1 | Center-left: dark rect with vertical line vs full widget showcase |
| 2 (small font) | rust-test-panel1 vs cpp-test-panel1 | "Test Panel" title text size, "State:" text size |
| 3 (missing BgColor) | rust-test-panel1 vs cpp-test-panel1 | Lower-right: C++ has small ColorField, Rust has nothing |
| 4 (white rect border) | rust-test-panel2 vs cpp-test-panel2 | Selection indicator around Toolkit Test area |
| 5 (missing status logs) | rust-test-panel1 vs cpp-test-panel1 | Four small tiles, upper-right: text below "Pri=" line |
| 6 (wrong zoom/aspect) | Screenshot_20260316_081608 | Side-by-side: C++ shows overview, Rust shows zoomed-in tile |
| 7 (no wheel zoom) | Not visible in screenshots | Behavioral: mouse wheel does nothing in Rust |
| 8 (highlight top sliver) | rust-test-panel2 vs cpp-test-panel2 | Selection rect covers thin strip vs full panel |

**Key source files:**
- TestPanel: find with `grep -rn 'struct TestPanel' src/`
- View: `src/panel/view.rs`
- ViewAnimator: find with `grep -rn 'ViewAnimator\|view_animator' src/`
- ViewInputFilter: find with `grep -rn 'ViewInputFilter\|ZoomScrollFilter\|WheelZoom\|input_filter' src/`
- Panel behavior trait: `src/panel/behavior.rs`
- Panel tree: `src/panel/tree.rs`
- Window: `src/window/zui_window.rs`
- App: `src/window/app.rs`
- Painter: `src/render/painter.rs`
- Font cache: find with `grep -rn 'FontCache\|font_cache\|glyph' src/`
- Border: `src/widget/border.rs`
- Resource loading: find with `grep -rn 'emRes\|resource\|toolkit_images\|include_bytes' src/`

**C++ reference source:**
- TestPanel: `~/.local/git/eaglemode-0.96.4/src/emTest/emTestPanel.cpp`
- View: `~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp`
- ViewInputFilter: `~/.local/git/eaglemode-0.96.4/src/emCore/emViewInputFilter.cpp`
- Panel: `~/.local/git/eaglemode-0.96.4/src/emCore/emPanel.cpp`
- Resources: `~/.local/git/eaglemode-0.96.4/res/emCore/`

**Important:** When tracing C++ code, follow the inheritance chain. If a method isn't in the named file, check parent classes. Use `grep -rn 'MethodName' ~/.local/git/eaglemode-0.96.4/src/emCore/` to find the actual implementation.

## Search Rules

For every diagnostic step that involves searching:
1. Run the grep command specified. Read at least 30 lines of context around each match.
2. If grep returns zero results, try at least 2 broader search patterns before concluding "doesn't exist." The Rust port may use different names than C++.
3. After finding a match, Read the full function/method containing it — not just the matched line.
4. Do not conclude a feature is missing based on grep alone. A feature may exist under a different name, in a different module, or implemented via a different pattern (e.g., trait impl vs method, closure vs function pointer).

## Triage Order

Not all bugs are equally hard. Diagnose in this order to spend tokens where they matter:

**Tier 1 — Likely simple (diagnose first, move on quickly if resolved in <3 steps):**
- Bug 3 (missing BgColor widget) — probably "feature not ported"
- Bug 7 (no wheel zoom) — probably a missing event handler

**Tier 2 — Moderate (compare parameters/constants between Rust and C++):**
- Bug 2 (font size) — parameter or scaling comparison
- Bug 5 (missing status logs) — recursion depth or threshold comparison

**Tier 3 — Structural (spend the most effort here, may require multi-file tracing):**
- Bug 1 (collapsed widget) — layout/tallness investigation
- Bug 4 (selection border style) — resource system investigation
- Bug 6 (wrong zoom level) — view initialization investigation
- Bug 8 (highlight geometry) — panel rect computation

If a Tier 1 bug resolves conclusively in fewer than 3 steps, do not force yourself through remaining decision tree branches. Move on and spend the freed effort on Tier 3 bugs.

## Bugs to Diagnose

### Bug 1: Toolkit Test widget collapsed to vertical strip

**What is observed:** The center-left area of the TestPanel shows a label "Toolkit Test" above a dark rectangle split by a thin vertical line. In C++, this same area shows a full widget showcase (buttons, text fields, checkboxes, scalar fields, radio buttons, list boxes, splitter, etc.).

**What is known:** The widget exists (the label renders). The content is not visible — either children aren't created, the layout gives them zero width/height, or the children exist but their tallness/dimensions are wrong.

**Note:** This bug may share a root cause with Bug 8 (highlight covers top sliver). Both could stem from a panel sizing or tallness error. Diagnose independently but check for overlap.

**Diagnostic steps:**
1. Find the Toolkit Test panel in the Rust TestPanel source. It is a child panel created during `auto_expand`. Identify its type.
2. Read that type's `auto_expand` method. Does it create child widgets? List every child it creates.
3. Read that type's `layout_children` method. Does it compute non-zero widths AND heights for children? Read the full method, not just the first few lines.
4. Compare against C++ `emTestPanel.cpp` — find where the toolkit widget showcase is created. What panel type is it? What children does it create? Compare the child count.
5. Decision tree:
   - If Rust creates fewer children than C++ → the bug is in `auto_expand`. Record which children are missing.
   - If Rust creates the same children but `layout_children` computes zero widths or heights → the bug is in layout. Record the computation.
   - If Rust creates children with non-zero layout but the children's own `tallness` is wrong (e.g., 0.0 or very small) → the bug is in child panel initialization. Record the tallness values.
   - If layout and tallness look correct → the bug may be in the auto-expansion threshold preventing children from becoming visible. Check `SetAutoExpansionThreshold` or equivalent.
   - If none of the above branches match → record what you actually found, set confidence below 0.3, set `needs_manual_review: true`, and move on.

**Reframe trigger:** If after completing all branches you have not identified a specific file:line, re-examine the screenshots. Is the widget actually collapsed, or is it rendered at a different position/zoom than expected? If the latter, this may be the same issue as Bug 6 manifesting differently.

### Bug 2: Font renders at smaller size than C++

**What is observed:** The "Test Panel" title text is visibly smaller in Rust than C++. Status text ("State:", "Pri=") also appears smaller. Both versions render to similar window sizes.

**What is known:** The text renders (it's not missing). The size differs.

**Diagnostic steps:**
1. Find where the TestPanel paints its title text. Look for `paint_text` or `paint_text_layout` calls in the TestPanel's `paint()` method. Read the full paint method.
2. What height/size parameter is passed? Compare against C++ `emTestPanel::Paint()`. Record both values.
3. Decision tree:
   - If the height parameter differs between Rust and C++ → the bug is in the TestPanel paint code. Record both values.
   - If the height parameter is the same → the issue is downstream. Continue to step 4.
4. Check the panel's coordinate system. What are the panel's layout dimensions (x, y, w, h)? The same relative text height (e.g., 0.05) produces different pixel sizes if the panel's layout rect differs. Compare against C++.
5. If panel dimensions match → check the rendering pipeline for scaling discrepancies. Search broadly: `grep -rn 'scale_factor\|dpi\|hidpi\|pixel_ratio\|glyph.*size\|font.*height\|text.*scale' src/`. Check DPI handling, font cache size computation, and painter coordinate transforms. Record the first discrepancy found between Rust and C++.
   - If none of the above branches match → record what you found, set confidence below 0.3, set `needs_manual_review: true`, and move on.

### Bug 3: Background Color control panel widget missing

**What is observed:** C++ shows a small `ColorField` widget in the lower-right corner labeled "Background Color" that lets users change the TestPanel background. Rust does not show this widget.

**What is known:** C++ creates this via `TestPanel::CreateControlPanel()` which overrides `emPanel::CreateControlPanel()`. This is a "control panel" overlay mechanism — the view creates it when a panel is "visited."

**Diagnostic steps:**
1. Search: `grep -rn 'create_control_panel\|control_panel\|ControlPanel\|config_panel\|settings_panel\|bg_color' src/`.
2. If not found under any variant → this is a missing feature. Record it, set confidence 0.9, and move on.
3. If found: does the TestPanel override it? If no → fix is in TestPanel. If yes but returns None → stub never completed. If the View never calls it → mechanism not wired.

### Bug 4: Selection border renders as plain white rectangle instead of arrow pattern

**What is observed:** When a panel is selected/focused in Rust, a plain white rectangle border appears. In C++, the selection indicator is a patterned border with angled arrows/chevrons.

**What is known:** The selection mechanism works (something renders). The visual style differs.

**Note:** This bug shares a visual element with Bug 8. Both involve the selection highlight. Bug 4 is about appearance, Bug 8 is about geometry. They may or may not share a root cause.

**Diagnostic steps:**
1. Find where the view paints the selection/focus highlight. Search: `grep -rn 'highlight\|selection.*paint\|focus.*paint\|active.*highlight\|visit.*mark\|paint_rect.*selection\|draw.*focus' src/panel/view.rs src/render/painter.rs`. Read the full painting function.
2. In C++, find the selection highlight painting. Search: `grep -rn 'emGetInsResImage\|SelectionImage\|highlight\|PaintMark\|VisitMark' ~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp`. Follow inheritance if not found here.
3. List the C++ resource directory: `ls ~/.local/git/eaglemode-0.96.4/res/emCore/` — look for selection/highlight/arrow image files.
4. In Rust, check how resources are loaded: `grep -rn 'include_bytes\|load_image\|resource.*path\|toolkit_images' src/`. Is there a resource loading system?
5. Decision tree:
   - If Rust paints a `paint_rect` where C++ uses `PaintImage` with resource images → the image-based highlight was never ported. Record the C++ image file names and the Rust fallback code location.
   - If Rust loads images but paints them at wrong coordinates → the image loading works but the painting coordinates are wrong. Record both.
   - If no image loading system exists in Rust → the entire resource system needs porting. Record this as a missing feature.
   - If none of the above branches match → record what you found, set confidence below 0.3, set `needs_manual_review: true`, and move on.

### Bug 5: Scrolling input status log missing from 3 of 4 recursive TestPanel tiles

**What is observed:** The four small recursive TestPanel tiles in the upper-right each show "Test Panel", geometric shapes, and status text. In C++, all four have a small scrolling text area below the "Pri=" line showing input event history. In Rust, only 1 of 4 shows this area.

**What is known:** The feature works at some recursion depth (1 of 4 shows it). It fails at deeper recursion or smaller panel sizes.

**Diagnostic steps:**
1. Find how the recursive TestPanel tiles are created. In C++ `emTestPanel::AutoExpand()`, how many levels of recursion are created? What controls the depth?
2. In Rust, find the equivalent auto_expand. Is the recursion depth the same?
3. The scrolling text area is likely a child panel. Find where it's created — is it in `auto_expand` or somewhere else? What type is it?
4. Decision tree:
   - If the text panel is created in `auto_expand` with a recursion depth check → compare the depth limit in Rust vs C++.
   - If the text panel is created unconditionally but depends on auto-expansion threshold → compare `SetAutoExpansionThreshold()` / equivalent values. C++ tiles at deeper recursion have smaller screen area, and if the Rust threshold is higher, they won't auto-expand.
   - If the text panel is created at all depths but has a **minimum size check** for painting (won't render below a certain pixel height) → the bug is not in creation but in visibility. Check the text panel's `paint()` for size guards.
   - If the text panel IS created at all depths AND renders at all sizes → the bug may be in the **layout** of the recursive tiles giving the text panel zero height at smaller sizes.
   - If none of the above branches match → record what you found, set confidence below 0.3, set `needs_manual_review: true`, and move on.
5. For the one tile that works: what recursion level is it? Is it the largest tile? This tells you whether the issue is depth-based or size-based.

### Bug 6: Different aspect ratio / zoom level at smaller window size

**What is observed:** In the side-by-side screenshot at a smaller window size, C++ shows the full TestPanel content (toolkit widget, recursive tiles, status text) while Rust shows a zoomed-in view of one of the recursive tiles (geometric shapes, traffic lights). The content is different because the viewport/zoom level differs.

**What is known:** The content renders correctly at both zoom levels. The initial viewport or zoom-to-fit behavior differs.

**Note:** This may share a root cause with Bug 7 (mouse wheel zoom). Both involve the view/zoom subsystem.

**Diagnostic steps:**
1. Trace window creation. Read the standalone binary source (find with `grep -rn 'fn main' src/bin/ examples/`). What happens after the window and TestPanel are created? Is there a "visit root" or "zoom to fit" call?
2. In C++, read `emTestPanelStandalone.cpp` and trace what happens at startup. Does the framework automatically visit the root panel?
3. Search the view for home/fit behavior: `grep -rn 'home\|fit\|visit.*root\|initial.*view\|zoom.*all\|VisitFullsized' src/panel/view.rs`.
4. Decision tree:
   - If no initial viewport setup exists in Rust → the view starts at default coordinates (likely 0,0 with default zoom). The fix is adding a visit-root call after panel creation.
   - If a visit-root exists but computes wrong bounds → the root panel's reported size/tallness is wrong at startup (may not be computed yet). Record the timing.
   - If the viewport is set correctly at startup but changes immediately (e.g., an animation overrides it) → the bug is in the animator. Check for any animation that triggers on the first cycle.
   - If none of the above branches match → record what you found, set confidence below 0.3, set `needs_manual_review: true`, and move on.

**Reframe trigger:** If after completing all branches you have not identified a specific file:line, ask: is this a zoom/viewport issue, or could the root panel's layout dimensions be wrong? If the latter, check whether the root panel's tallness matches C++.

### Bug 7: Mouse wheel zoom does not work

**What is observed:** In C++, scrolling the mouse wheel zooms in/out on the TestPanel. In Rust, the mouse wheel does nothing.

**What is known:** This is a functional gap, not a visual one. The zoom infrastructure exists (the view can zoom — the recursive tiles prove this).

**Diagnostic steps:**
1. Trace the mouse wheel event path. Start at `src/window/app.rs` or `src/window/zui_window.rs` — find where winit `WindowEvent::MouseWheel` is handled. Read the handler.
2. Is the wheel event converted to an emCore input event? Search: `grep -rn 'MouseWheel\|wheel\|scroll.*event\|WheelDelta\|LineDelta\|PixelDelta' src/`.
3. In C++, `emViewInputFilter` (specifically `emDefaultTouchVIF` or `emMouseZoomScrollVIF`) handles wheel events and converts them to zoom. Find the equivalent in Rust: `grep -rn 'zoom.*wheel\|wheel.*zoom\|scroll.*zoom\|MouseZoom' src/`.
4. Decision tree:
   - If winit `MouseWheel` events are not handled at all in App/ZuiWindow → the event isn't being captured. The fix is adding a handler.
   - If wheel events are received but not forwarded to the view input chain → the bug is in event translation. Record the gap.
   - If wheel events reach the input chain but no filter handles them → the filter is missing or not registered. Check how the input filter chain is built at view creation.
   - If a filter exists and handles wheel but the zoom delta computes to zero → the bug is in the delta calculation. Check unit conversion (winit `LineDelta` vs `PixelDelta` vs C++'s wheel step units).
   - If wheel events are never received from winit → check winit documentation for required configuration.
   - If none of the above branches match → record what you found, set confidence below 0.3, set `needs_manual_review: true`, and move on.
5. Record where the chain breaks with file:line.

### Bug 8: Selection/focus box covers only top sliver of panels instead of entire panel

**What is observed:** When clicking on the Toolkit Test widget or the recursive tiles, the selection highlight (white rect, per Bug 4) covers only a thin horizontal strip at the top of the panel, not the full panel area. In C++ the selection highlight covers the entire panel bounds.

**What is known:** Selection occurs (the highlight renders). The highlight rect is wrong — too short.

**Note:** May share root cause with Bug 1 (collapsed widget). Both could involve wrong tallness/panel dimensions.

**Diagnostic steps:**
1. Find where the selection/focus highlight rect is computed. Search: `grep -rn 'highlight.*rect\|selection.*rect\|focus.*rect\|visit.*rect\|mark.*rect\|active.*rect' src/panel/view.rs src/panel/tree.rs`.
2. Read the highlight painting code fully. What rect does it use? Is it the panel's layout rect, the panel's essence rect, or something else?
3. In C++, find the equivalent: `grep -rn 'GetEssenceRect\|ClipX\|GetLayoutX\|GetLayoutY\|GetLayoutW\|GetLayoutH' ~/.local/git/eaglemode-0.96.4/src/emCore/emPanel.cpp`. What rect does the highlight use?
4. Decision tree:
   - If the Rust highlight uses the panel's width but not its full height → the height computation is wrong. Check if it uses `tallness * width` for height. Compare the tallness value against C++.
   - If the Rust highlight uses a hardcoded height or a fraction of the panel rect → the painting code is wrong. Record the formula.
   - If the panel's layout rect itself has a small height → the bug is upstream in panel sizing, not in highlight painting. This connects to Bug 1. Record the panel's actual layout dimensions.
   - If the rect is correct but a clip rect is constraining the highlight → the view's clip region is wrong. Check for `clip_rect` or `scissor` calls before the highlight paint.
   - If none of the above branches match → record what you found, set confidence below 0.3, set `needs_manual_review: true`, and move on.

## Cross-Bug Analysis (mandatory, do this after all 8 individual diagnoses)

Before writing the final output, check for shared root causes. The pairs below are hypotheses, not constraints — if your diagnosis reveals unexpected connections (e.g., Bugs 2 and 6 sharing a coordinate-system root cause), report those too.

1. **Bugs 1 + 8:** Both may involve panel tallness/sizing. If Bug 1's root cause involves wrong panel dimensions, check if the same dimensions explain Bug 8's short highlight rect.
2. **Bugs 4 + 8:** Both involve the selection highlight. If Bug 4's root cause involves the highlight painting code, check if that same code path is responsible for Bug 8's geometry.
3. **Bugs 6 + 7:** Both involve the view/zoom system. If Bug 6's root cause involves the view's initial state, check if the same code path handles wheel zoom setup.
4. **Consider the TestPanel itself:** If multiple bugs trace back to the TestPanel source (not the framework), the TestPanel port may have systematic issues. Note this in the output.

For each shared root cause found: record it in both bug entries' `related_bugs` field, but keep each diagnosis independent — a shared root cause must fully explain ALL symptoms of BOTH bugs.

## Output

Write results to `state/run_003/visual_bug_diagnosis.json`:

```json
{
  "diagnosed_at": "<ISO8601>",
  "bugs": [
    {
      "bug_id": 1,
      "title": "<short title>",
      "root_cause": "<specific description of what code is wrong>",
      "category": "<enum: layout|input|rendering|missing_feature|view_state|resource_loading>",
      "rust_source": "<file:line(s) where the problem is>",
      "cpp_reference": "<file:line(s) showing correct behavior>",
      "confidence": "<float 0.0-1.0: 0.8+ if you found the specific file:line and can explain WHY it produces the symptom; 0.5-0.8 if you found the relevant code area but the causal chain has a gap; below 0.5 if inferring from absence or analogy>",
      "ruled_out": ["<max 2: only causes you actually investigated and found evidence against. Omit if you never seriously considered an alternative>"],
      "verification_prediction": "<a specific, falsifiable claim the operator can check — e.g., 'adding dbg!() at file:line should print X'>",
      "related_bugs": ["<bug_ids that may share a root cause>"],
      "diagnostic_complete": true,
      "needs_manual_review": false,
      "manual_review_reason": "<string if needs_manual_review is true>"
    }
  ],
  "cross_bug_analysis": {
    "shared_root_causes": [
      {
        "bugs": [1, 8],
        "shared_cause": "<description or null if independent>",
        "evidence": "<what confirms the connection>"
      }
    ]
  }
}
```

## Rules

- Do not modify any source files.
- Do not attempt fixes.
- For every diagnostic step: grep first, then Read at least 30 lines of context around each match. Do not diagnose from grep output alone.
- If a grep returns zero results, try at least 2 broader search patterns before concluding "doesn't exist."
- Follow the decision trees. When a branch says "record X", write it in the output JSON.
- Diagnose each bug independently even if you suspect a shared cause. The cross-bug analysis step is where you connect them.
- Reference C++ source with exact file:line. If the logic is in a parent class, trace the inheritance and cite the actual implementation.
- All 8 bugs must appear in the output. None may be skipped.
- Confidence reflects whether you found the causal chain, not how many greps you ran. One grep that reveals the exact buggy line = high confidence. Ten greps of related-but-not-causal code = low confidence.
- If fix planning enters your reasoning, stop. Return to diagnosing the next bug. Fix planning is out of scope.
