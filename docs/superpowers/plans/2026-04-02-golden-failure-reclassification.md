# Golden Failure Reclassification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reclassify all 37 remaining golden test failures into groups by verified root cause, producing an updated catalog.

**Architecture:** Generate diff images for all 37 failures. Re-validate the G2-G9 hypotheses from the original catalog (quick check — run the same tests, confirm same divergence patterns). Then investigate the 18 former-G1 tests using the 9-slice investigation findings as a starting point, splitting them into sub-groups by actual root cause (HowTo text, text rendering, unknown large-divergence). Assemble into a prioritized catalog.

**Tech Stack:** Rust golden tests, C++ reference at `~/git/eaglemode-0.96.4/`, PPM diff images

**Key files:**
- Test runner: `cargo test --test golden -- --test-threads=1`
- Diff image generator: `DUMP_GOLDEN=1 cargo test --test golden <name> -- --test-threads=1`
- Diff images output: `target/golden-debug/{actual,expected,diff}_<name>.ppm`
- Test sources: `crates/eaglemode/tests/golden/*.rs`
- Original catalog: `docs/superpowers/specs/2026-04-01-golden-failure-catalog.md`
- Output: `docs/superpowers/specs/2026-04-02-golden-failure-catalog.md`

**Critical constraint:** This is a read-only investigation. Do NOT modify any production code.

---

### Task 1: Generate diff images and structured error data for all 37 failures

- [ ] **Step 1: Run all golden tests with DUMP_GOLDEN=1**

```bash
DUMP_GOLDEN=1 cargo test --test golden -- --test-threads=1 2>&1 | tee /tmp/golden-reclass.txt
```

This creates PPM files at `target/golden-debug/{actual,expected,diff}_<name>.ppm` for each failing test.

- [ ] **Step 2: Verify 37 diff images were generated**

```bash
ls target/golden-debug/diff_*.ppm | wc -l
```

Expected: 37.

- [ ] **Step 3: Extract max_diff and first divergent pixels for each test**

```bash
grep -E 'max_diff=|mismatch:' /tmp/golden-reclass.txt | head -80
```

Save the structured error data — each test's max_diff, fail count, and first 10 divergent pixel coordinates with actual/expected RGB values.

---

### Task 2: Re-validate G2-G9 hypotheses (19 tests)

The original catalog classified these 19 tests into groups G2-G9. Confirm the hypotheses still hold by checking the same divergence patterns.

**G2 tests (6):** testpanel_root, bezier_stroked, widget_scalarfield, widget_scalarfield_zero_range, widget_scalarfield_min_value, widget_scalarfield_max_value
**G3 tests (2):** image_scaled, composed_splitter_content
**G4 tests (1):** golden_widget_border_roundrect_thin
**G5 tests (1):** multi_compose
**G6 tests (1):** gradient_radial
**G7 tests (1):** eagle_logo
**G8 tests (1):** cosmos_item_border
**G9 tests (1):** widget_checkbox_checked

Note: The original catalog had `widget_border_round_rect` in G4 (2 tests) — it now passes, leaving G4 with 1 test. The original G3 had 6 tests — 4 now pass (`widget_splitter_h`, `widget_splitter_h_pos0`, `widget_splitter_h_pos1`, `widget_error_panel`), leaving 2.

- [ ] **Step 1: Check G2 (polygon rasterizer) — 6 tests**

From `/tmp/golden-reclass.txt`, extract the error data for the 6 G2 tests. Original hypothesis: `fill_polygon_aa` / `rasterize_polynomial` FP edge-crossing accumulation differs from C++.

Verify:
- testpanel_root: max_diff still ~255, divergent at (22,26) area (polygon outlines)?
- bezier_stroked: max_diff still ~53, divergent at bezier curve edges?
- widget_scalarfield (×4): max_diff still ~12, divergent at diagonal polygon edges (value arrow)?

If the divergence patterns match the original catalog, mark G2 as **confirmed**. If patterns changed, investigate.

- [ ] **Step 2: Check G3 (adaptive Hermite table) — 2 tests**

From `/tmp/golden-reclass.txt`, extract error data for image_scaled and composed_splitter_content.

Original hypothesis: Runtime f64 Hermite factor table rounds differently from C++ compile-time table. Both should have max_diff=1.

Verify max_diff=1 and scattered single-pixel divergences (not contiguous regions). If confirmed, mark G3 as **confirmed**.

- [ ] **Step 3: Check G4-G9 (1 test each)**

For each single-test group, verify the divergence pattern matches the original catalog:

- G4 (`golden_widget_border_roundrect_thin`): max_diff~24, corner pixels only
- G5 (`multi_compose`): max_diff=1, 7% of pixels, bulk interior spans
- G6 (`gradient_radial`): max_diff=1, 32 pixels at ellipse AA boundary
- G7 (`eagle_logo`): max_diff~175, 55% of pixels, gradient ±1 + structural outliers
- G8 (`cosmos_item_border`): max_diff~130, row 11 BLACK where C++ has blended border
- G9 (`widget_checkbox_checked`): max_diff~236, checkmark stroke interior pixels

If all match, mark each as **confirmed**.

- [ ] **Step 4: Record re-validation results**

For each group, note: "Confirmed — same pattern as original catalog" or "Changed — [describe new pattern]". This feeds into the final catalog.

---

### Task 3: Classify the 18 former-G1 tests

These tests were originally attributed to area sampling carry-over (disproven). The 9-slice investigation identified three root causes. Classify each test into one of these or a new group.

**The 18 tests:**
testpanel_expanded (255), composition_tktest_1x (239), composition_tktest_2x (239), widget_file_selection_box (237), composed_border_nest (153), widget_listbox (136), starfield_small (69), colorfield_expanded (54), starfield_large (53), listbox_expanded (33), widget_button_normal (31), widget_radiobutton (31), widget_textfield_content (26), widget_textfield_empty (26), widget_textfield_single_char_square (26), widget_listbox_single (25), widget_listbox_empty (25), widget_colorfield (24), widget_colorfield_alpha_near (24), widget_colorfield_alpha_opaque (24), widget_colorfield_alpha_zero (24), widget_checkbox_unchecked (22), widget_splitter_v_extreme_tall (19)

Wait — that's 23 items but should be 18. The difference: starfield_small, starfield_large, colorfield_expanded, testpanel_expanded, and composed_border_nest are in the "large-divergence unknown" sub-group. Let me just list all 18 and classify as we go.

- [ ] **Step 1: Classify by max_diff into suspected sub-groups**

Initial triage based on the 9-slice investigation findings:

**Sub-group A — suspected HowTo text (max_diff 19-31, ~12 tests):**
widget_button_normal (31), widget_radiobutton (31), widget_textfield_content (26), widget_textfield_empty (26), widget_textfield_single_char_square (26), widget_listbox_single (25), widget_listbox_empty (25), widget_colorfield (24), widget_colorfield_alpha_near (24), widget_colorfield_alpha_opaque (24), widget_colorfield_alpha_zero (24), widget_checkbox_unchecked (22), widget_splitter_v_extreme_tall (19)

**Sub-group B — large divergence, unknown (max_diff 33-255, ~5-8 tests):**
testpanel_expanded (255), composition_tktest_1x (239), composition_tktest_2x (239), widget_file_selection_box (237), composed_border_nest (153), widget_listbox (136), starfield_small (69), colorfield_expanded (54), starfield_large (53), listbox_expanded (33)

Note: starfield tests may have a different root cause (PaintEllipse/PaintImageColored, not border rendering).

- [ ] **Step 2: Verify Sub-group A (HowTo text) by examining diff images**

For 3 representative tests from Sub-group A — `widget_button_normal`, `widget_checkbox_unchecked`, `widget_colorfield` — view the diff images.

The 9-slice investigation found HowTo text is rendered as a very faint pill (~6.5% opacity) overlaid on the border. Key evidence:
- Divergent pixels should be at/near the border region, not the widget content
- The divergence should be small (max_diff ~20-31) and affect a small percentage of pixels
- The spatial pattern should show faint text-shaped regions in the diff image (may be too faint to see in the diff visualization since max_ch_diff ≤ 31 will be dim red)

Read the test error output for the exact divergent pixel coordinates. Check if they cluster at the bottom of the border area (where HowTo text typically appears in emBorder).

Read `~/git/eaglemode-0.96.4/src/emCore/emBorder.cpp` to find where `GetHowTo()` is called and where the text is rendered. Then check the Rust `emBorder.rs` for the absence of this code.

- [ ] **Step 3: Verify Sub-group B (large divergence) by examining diff images**

For 3 representative tests — `composed_border_nest` (153), `widget_listbox` (136), `composition_tktest_1x` (239) — view the diff images.

Key questions:
- Are the divergent pixels in contiguous regions (structural) or scattered (arithmetic)?
- Do the divergent regions correspond to border images, widget content, or compositing overlaps?
- The 9-slice investigation said "actual ~80-100 channels lighter than expected" — verify this by checking the error output RGB values

For starfield tests (69, 53): these render via PaintEllipse and PaintImageColored, NOT PaintBorderImage. They may belong to a separate group. Check if the divergent pixels are at star edges (likely PaintEllipse polygon AA, related to original G2) or in star glow textures (PaintImageColored, possibly area sampling related).

- [ ] **Step 4: Trace the large-divergence root cause**

For `composed_border_nest` or `widget_listbox` (whichever has the clearest divergence pattern):

1. Read the test source to find which widget is rendered
2. Read the widget's Paint method to find which emPainter calls it makes
3. Using the divergent pixel coordinates, determine which paint call produces those pixels
4. Read the corresponding C++ code path
5. Compare Rust vs C++ for that specific call

The goal is to determine whether the large divergence is from:
- Missing rendering code (a paint call C++ makes that Rust doesn't)
- Wrong rendering parameters (right call, wrong arguments)
- Compositing/blending difference (individual primitives correct but composited differently)

- [ ] **Step 5: Refine sub-groups based on investigation**

Sub-groups A and B may split further or merge based on the findings. For example:
- Sub-group A might split into "HowTo text missing" and "text rendering rounding" if some tests have correct HowTo text but wrong text pixels
- Sub-group B might split into "starfield-specific" and "border compositing" groups
- Some Sub-group B tests (composite panels) might just be aggregates of Sub-group A divergences amplified through compositing

Record the refined sub-groups.

---

### Task 4: Assemble the updated catalog and commit

- [ ] **Step 1: Write the catalog document**

Create `docs/superpowers/specs/2026-04-02-golden-failure-catalog.md` with:

1. Summary table (all groups with test counts, max_diff ranges, root causes)
2. Per-group detail sections (spatial pattern, code path, C++ reference, hypothesis)
3. For each hypothesis, note whether it is **verified** (observed directly), **carried forward** (from original catalog, re-validated), or **speculative** (inferred but not traced to code)

Format:
```markdown
# Golden Failure Catalog (2026-04-02)

Supersedes the 2026-04-01 catalog. 37 tests across N groups.

## Summary

| Group | Code Path | Tests | max_diff range | Status | Likely cause |
|-------|-----------|-------|----------------|--------|--------------|
| ... | ... | ... | ... | verified/carried/speculative | ... |

## [Group Name]

**Tests (N):** ...
**max_diff range:** ...
**Status:** verified / carried forward / speculative
**Divergent code path:** ...
**C++ reference:** ...
**Spatial pattern:** ...
**Root cause hypothesis:** ...
```

- [ ] **Step 2: Verify coverage**

Count all tests in the catalog. Must be exactly 37, each in exactly one group.

- [ ] **Step 3: Priority-order the groups**

Order by:
1. Number of tests affected (descending)
2. Status: verified before speculative
3. max_diff (ascending for likely-easier fixes)

- [ ] **Step 4: Update memory**

Update `/home/a0/.claude/projects/-home-a0-git-eaglemode-rs/memory/divergence_inventory.md` to point to the new catalog and summarize the groups.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-02-golden-failure-catalog.md
git commit -m "docs: add reclassified golden failure catalog (37 tests, supersedes 2026-04-01)

Incorporates findings from area sampling fix (5 tests fixed) and 9-slice
investigation (G1 hypothesis disproven). Former G1 tests reclassified
into HowTo text, text rendering, and large-divergence unknown groups.
G2-G9 hypotheses re-validated."
```

---

## Critical Rules

1. **No production code changes.** This is classification only.
2. **C++ source is truth.** Read `~/git/eaglemode-0.96.4/` — don't trust Rust comments or prior documentation.
3. **Don't trust the old catalog.** G1 was wrong. Re-validate everything.
4. **Mark hypothesis confidence.** Every group must say whether its hypothesis is verified, carried forward, or speculative.
5. **Every test in exactly one group.** 37 tests total. Count before committing.
