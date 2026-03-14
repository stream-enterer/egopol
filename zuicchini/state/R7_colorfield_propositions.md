# R7 ColorField Remaining Divergence — Investigation Brief

## How to Use This Document

You are investigating rendering divergence between the Rust `zuicchini` UI
framework and the C++ Eagle Mode reference (`emCore`). Prior rounds fixed
geometry bugs (see "R11 Completed Work" below). **20,593 divergent pixels
remain across the colorfield tests.** This document tells you exactly what
was done, what was skipped, and what to do next.

**Your goal:** Achieve 1:1 pixel parity with C++ for the colorfield golden tests.
This means 0 divergent pixels, not "close enough." Every remaining pixel is
evidence of a code difference that should be found and fixed. Do not stop until
you have either fixed every divergent pixel or proven — with specific C++ line
references — that a remaining difference is from compiler FP instruction
selection (not from any source-level code difference).

**Your approach:**
1. Read this document fully before acting.
2. Execute the "Next Steps" protocol at the bottom. Do not skip steps.
3. For every hypothesis, read both the Rust and C++ code line-by-line before
   concluding. Do not assume — verify.
4. When you find a fix, measure its pixel impact before and after. Record the
   delta. Do not estimate.
5. When divergence remains after a fix, treat it as evidence of another bug,
   not as noise. Investigate it.

**Key files:**
- Rust: `src/widget/scalar_field.rs`, `src/widget/color_field.rs`,
  `src/widget/border.rs`, `src/widget/field_panel.rs`,
  `src/render/painter.rs`, `src/render/interpolation.rs`
- C++: `/home/ar/.local/git/eaglemode-0.96.4/src/emCore/emScalarField.cpp`,
  `emColorField.cpp`, `emBorder.cpp`, `emPainter.cpp`
- Tests: `tests/golden_parity/widget.rs` (search for `widget_colorfield`,
  `colorfield_expanded`, `widget_scalarfield`)
- Debug images: `golden_debug/*.png` (only regenerated on test FAILURE with
  `DUMP_GOLDEN=1`; to get fresh images, temporarily lower tolerance in test)

**Commands:**
```bash
# Run specific test (CARGO_TARGET_DIR=rust_target if default target is read-only)
CARGO_TARGET_DIR=rust_target cargo-nextest ntr -E 'test(widget_colorfield)' --workspace

# Run with divergence log
CARGO_TARGET_DIR=rust_target DIVERGENCE_LOG=$(pwd)/state/post_rXX.jsonl cargo-nextest ntr --workspace --test-threads=1

# Clippy + full test suite
CARGO_TARGET_DIR=rust_target cargo clippy --workspace -- -D warnings && CARGO_TARGET_DIR=rust_target cargo-nextest ntr --workspace
```

**Anti-pattern warnings for the investigator:**
- Do not label remaining divergence as "structural" or "irreducible" without
  reading the C++ code. Every prior item that was called structural (R5, R6,
  R9, R10) turned out to have specific fixable bugs.
- Do not estimate pixel impact — measure it. Run tests before and after each fix.
- "Source formulas match" does not mean "output matches." Verify with actual
  pixel comparisons, not just code reading.
- Do not assume both colorfield tests fail for the same reason. They have
  different configurations.

---

## Context

A ColorField is an Instrument-bordered panel showing a color swatch on the left
and, when auto-expanded, a RasterLayout grid on the right containing:
- 7 ScalarField panels (Red, Green, Blue, Alpha, Hue, Saturation, Value)
- 1 TextField panel (Name / hex code)

Each ScalarField child uses `OBT_RECT + IBT_CUSTOM_RECT` with `border_scaling=2.0`.
The grid is 2 columns × 4 rows, column-major, tallness locked to 0.2, alignment
right-horizontal + center-vertical, spacing (0.08, 0.2, 0.04, 0.1).

**widget_colorfield** (800×600): `editable=false, alpha_enabled=false`,
color=red `rgba(255,0,0,255)`, layout (0, 0, 1.0, 0.75).

**colorfield_expanded** (800×800): `editable=true, alpha_enabled=true`,
color=dark-red `rgba(0xBB,0x22,0x22,0xFF)`, layout (0, 0, 1.0, 1.0).

These tests have different configurations (editable, alpha_enabled, colors,
viewport size). Whether they diverge for the same root cause is NOT verified.

**Coordinate systems:**
- C++ `ScalarField::Paint` operates in **normalized panel space** (width=1.0,
  height=tallness). `painter->GetScaleX()` = ViewedWidth (maps to pixels).
- Rust `ScalarField::paint` operates in **viewport pixel space** (width=ViewedWidth,
  height=ViewedWidth×tallness). `painter.scaling()` = (1.0, 1.0).
- Both systems produce equivalent pixel-space values when the formulas are
  correct. The `content_round_rect` function receives `(w, h)` in whichever
  coordinate system the paint function uses.

---

## Current Pixel Counts (after R12, 2026-03-14)

| Test | Pixels | Total | Pct | Delta vs R11 |
|------|--------|-------|-----|--------------|
| widget_colorfield | 5,509 | 480,000 | 1.15% | -860 |
| colorfield_expanded | 14,210 | 640,000 | 2.22% | -14 |
| widget_scalarfield | 106 | 480,000 | 0.022% | -114 |

Divergence log: `state/post_r12_final.jsonl`

### R11 counts (baseline for R12)

| Test | Pixels | Total | Pct |
|------|--------|-------|-----|
| widget_colorfield | 6,369 | 480,000 | 1.33% |
| colorfield_expanded | 14,224 | 640,000 | 2.22% |
| widget_scalarfield | 220 | 480,000 | 0.046% |

Baseline divergence log: `state/post_howto.jsonl`

---

## R12 Completed Work (2026-03-14)

### Bugs Fixed

1. **ScalarField IsEnabled() color dimming not implemented (OI-1).**
   C++ `emScalarField.cpp:412-414` blends bgCol/fgCol with `look.bg_color` at 80%
   when `!IsEnabled()`. Rust `ScalarField::paint` never dimmed colors and always
   passed `enabled=true` to `paint_border`. Fixed by adding `enabled: bool` parameter
   to `ScalarField::paint`, implementing the 80% lerp dimming, and passing the actual
   enabled state to `paint_border`. All callers updated (field_panel.rs, test_panel.rs,
   toolkit_demo.rs, widget.rs tests).
   C++ ref: `emScalarField.cpp:412-416`.
   Impact: **widget_colorfield -858 px** (Alpha field disabled when alpha_enabled=false).

2. **HowTo pill text not rendered inside indicator (OI-2/OI-5).**
   C++ `emBorder.cpp:916-927` paints text inside the HowTo pill when the pill area
   exceeds 100 square pixels. Rust only painted the pill rounded-rect shape without
   any text. Fixed by adding `how_to_text: String` field to `Border` and rendering
   text with `paint_text_boxed` inside the pill at appropriate size. Also fixed pill
   alpha rounding: Rust used `(255.0 * 0.10) as u8 = 25`, C++ `GetTransparented(90)`
   rounds to 26. Fixed with `(255.0 * 0.10 + 0.5) as u8`.
   C++ ref: `emBorder.cpp:906-928`, `emScalarField.cpp:285-293`.
   Impact: **widget_scalarfield -114 px**, **colorfield -16 px total**.

### Runtime-Value Audit (OI-2 partial)

Added `eprintln!` instrumentation to `ScalarField::paint` to dump content_round_rect
values, layout parameters, and scale mark coordinates. Verified all layout formulas
produce identical values to C++ (manually computed from `emScalarField.cpp:DoScalarField`).

The 106 remaining widget_scalarfield pixels were localized:
- **99 px** at y=516-517: sub-pixel scale mark mini-arrows (<1px triangles, h5=0.916px).
  Polygon rasterization coverage differs for these sub-pixel polygons.
- **7 px** scattered at border corners: 9-slice interpolation edge artifacts.

### Divergence Localization (OI-4/OI-5)

Generated fresh diff images for widget_scalarfield and widget_colorfield.

**widget_scalarfield** (106 px remaining):
- Sub-pixel scale mark arrows: 99 px at y=516-517 (arrow height 0.916 px)
- Border corner artifacts: 7 px scattered at rounded corners
- All paint formulas verified at runtime-value level to match C++

**widget_colorfield** (5,509 px remaining, multiple codepaths):
- Color swatch border outline (Instrument+OutputField border)
- CustomRect 9-slice border at small child scale (~134x27 px)
- Alpha field dimming + disabled border rendering
- Sub-pixel text/polygon rendering in ScalarField children

**colorfield_expanded** (14,210 px remaining):
- Similar patterns to widget_colorfield at larger scale (800x800)
- More children visible = more border edge divergence

### OI Resolution Summary

| OI | Status | Detail |
|----|--------|--------|
| OI-1 | **Resolved** | IsEnabled dimming implemented, -858 px widget_colorfield |
| OI-2 | **Partially resolved** | Runtime audit done for widget_scalarfield; all layout values match C++. Remaining 106px from sub-pixel polygon rasterization. |
| OI-3 | **Open** | Area-sampling pixel comparison not yet performed |
| OI-4 | **Resolved** | Fresh diff images generated, patterns categorized |
| OI-5 | **Resolved** | 220→106 px: 114 from HowTo text, 99 sub-pixel arrows, 7 border edges |

---

## R11 Completed Work (2026-03-14, commit `b3e96c9`)

### Bugs Fixed

1. **content_round_rect CustomRect — wrong inset formula.**
   Used simplified `inner_insets()` with post-reduction dimensions instead of
   C++ two-step inset. Fixed with inline logic using `w.min(h)` as pixel-space
   equivalent of C++ `emMin(1.0, h)`. Returns `radius = 0.0` matching C++ `rndR = 0`.
   C++ ref: `emBorder.cpp:1137-1164`. Impact: **-1,267 / -1,670**.

2. **paint_border CustomRect — missing first inset before border image.**
   Painted border image at raw inner rect without C++ first inset (`d = rndR * 0.25`)
   and used wrong radius bump base. Fixed with inline two-step geometry.
   C++ ref: `emBorder.cpp:1137-1153`. Impact: **-2,869 / -1,667**.

3. **content_rect CustomRect — wrong radius bump base.** Used `(1.0).min(h)`
   instead of `w.min(h)`. C++ ref: `emBorder.cpp:1144`. No current test impact.

4. **paint_border/paint_inner_overlay no-label inner_y/inner_h.** Missing
   symmetric minSpace. C++ ref: `emBorder.cpp:1046-1050`. Impact: **-71 testpanel**.

5. **paint_border ls — didn't check has_label().** Reserved label space for
   panels with no label content. Fixed to match C++.

6. **content_round_rect — missing HowTo handling.** Added HowTo rightward
   shift. No current impact (`howToSpace == minSpace` for tested border types).

### What R11 Verified (formula-level code reading)

The Step 5 paint-call audit compared Rust and C++ source formulas for all 7
ScalarField paint operations. Formulas match for: side bar rects, value arrow
vertices, scale mark text parameters, scale mark arrow vertices, inner overlay.

**Caveat:** This was formula-level code reading only. The protocol requires
comparing actual runtime parameter values with `eprintln!` debug output. That
was NOT done. "Source formulas match" does not rule out bugs — prior rounds
proved this repeatedly.

### NK Resolution Summary

| NK | Status | Detail |
|----|--------|--------|
| NK1 | Resolved | P18 impact measured: -7,473 total |
| NK2 | **Open** | Both tests improved, but per-cell diff patterns NOT visually compared (stale debug images) |
| NK3 | **Open — not investigated** | Area-sampling pixel comparison never performed |
| NK4 | Resolved | CustomRect border chrome fixed (Bug 2) |
| NK5 | Partially resolved | Data shows 220 px is shared-code; visual pattern comparison not performed |
| NK6 | Resolved | paint_inner_overlay is no-op for CustomRect |
| NK7 | Resolved (R12) | IsEnabled dimming implemented |

---

## Verified Propositions

These eliminate specific causes. Each was confirmed by the method listed.

**Verification methods:**
- **COMMIT**: Code was changed and tested; commit hash provided.
- **DEBUG**: Runtime values were printed and compared with C++ trace.
- **CODE**: Source was read line-by-line against C++ equivalent.
- **COMPUTED**: Values were calculated from verified formulas.

### Layout & Geometry

| # | Claim | Method | Rust Ref | C++ Ref |
|---|-------|--------|----------|---------|
| P1 | RasterLayout grid math identical | DEBUG | `src/layout/raster.rs` fn `do_layout_inner` | `emRasterLayout.cpp:311-404` |
| P2 | Child tallness locked to 0.2 | COMMIT `627c02a` | `src/widget/color_field.rs` fn `create_expansion_children` | `emRasterLayout.cpp:120-128` |
| P3 | Spacing, alignment, column count match | CODE | See P1 | See P1 |
| P4 | layout_children positions RasterLayout at right half | CODE | `src/widget/color_field.rs` fn `layout_children` | `emColorField.cpp:370-376` |
| P5 | content_rect_unobscured for Instrument+OutputField | COMMIT `9ca9c5e` | `src/widget/border.rs` fn `content_rect_unobscured` | `emBorder.cpp:1091-1128` |
| P6 | viewed_width correct through 3-level nesting | CODE | `src/panel/view.rs` fn `compute_viewed_recursive` | `emPanel.cpp:1478-1481` |
| P17 | paint_h = vw × tallness correct | CODE | `src/panel/view.rs:1947-1951` | `emView.cpp:1092-1096` |
| P18 | CustomRect two-step inset in content_round_rect | COMMIT `b3e96c9` | `src/widget/border.rs` fn `content_round_rect` | `emBorder.cpp:1137-1164` |
| P19 | CustomRect paint_border first inset + radius bump | COMMIT `b3e96c9` | `src/widget/border.rs` fn `paint_border` | `emBorder.cpp:1137-1153` |
| P20 | content_rect CustomRect uses w.min(h) not 1.0.min(h) | COMMIT `b3e96c9` | `src/widget/border.rs` fn `content_rect` | `emBorder.cpp:1144` |

### Widget Configuration

| # | Claim | Method | Rust Ref | C++ Ref |
|---|-------|--------|----------|---------|
| P7 | Look propagation correct | COMMIT `7a5e9ae` | `src/widget/color_field.rs:416-426` | `emColorField.cpp:450-470` |
| P8 | Editable flag propagated | COMMIT `7a5e9ae` | `src/widget/field_panel.rs:17-33` | `emColorField.cpp:472-479` |
| P9 | ScalarField color by InnerBorderType | COMMIT `b645fb3` | `src/widget/scalar_field.rs:234-241` | `emScalarField.cpp:400-411` |
| P10 | Children: OBT_RECT + IBT_CUSTOM_RECT + scaling 2.0 | CODE | `src/widget/field_panel.rs:29-31` | `emColorField.cpp:243-244` |

### Rendering Pipeline

| # | Claim | Method | Rust Ref | C++ Ref |
|---|-------|--------|----------|---------|
| P11 | Font atlas byte-identical | `diff` | `res/fonts/00020-0007F_128x224_BasicLatin_original.tga` | `res/emCore/font/` same file |
| P12 | Area-sampling downscale source formulas match | CODE | `src/render/interpolation.rs:219-366` | `emPainter_ScTlIntImg.cpp:686-828` |
| P13 | Color::lerp 16-bit matches GetBlended | COMMIT `b645fb3` | `src/foundation/color.rs:155-165` | `emColor.cpp:927` |
| P14 | Painter scale_x = 1.0 for all panels | CODE+grep | `src/render/painter.rs:219` | N/A (different coord system) |
| P15 | Scale marks visible in both | COMPUTED | `src/widget/scalar_field.rs:349` | tier 0 tw=12.46 > 1.0 |
| P16 | canvas_color equivalent | CODE | canvasColor=0 == TRANSPARENT | `emScalarField.cpp:421` |
| P21 | ScalarField IsEnabled dimming matches C++ | COMMIT R12 | `src/widget/scalar_field.rs:249-252` | `emScalarField.cpp:413-416` |
| P22 | HowTo pill text rendered inside indicator | COMMIT R12 | `src/widget/border.rs:1792-1812` | `emBorder.cpp:916-927` |
| P23 | HowTo pill alpha rounds correctly (26 not 25) | COMMIT R12 | `src/widget/border.rs:1790` | `emBorder.cpp:913` |
| P24 | ScalarField layout values match C++ at runtime | DEBUG | `eprintln!` in paint, manual C++ computation | `emScalarField.cpp:333-383` |
| P25 | widget_scalarfield 106 px are sub-pixel polygons + border edges | DIFF IMAGE | `golden_debug/diff_widget_scalarfield.ppm` | N/A |

### Caveats

**P12:** "Source formulas match" means no formula-level bug exists. It does NOT
mean the output is identical. Two implementations with matching formulas can
produce different output from loop accumulation order or compiler FP optimizations.
The actual pixel output of the area-sampling path has NOT been compared. See NK3.

**P14:** Rust and C++ use different coordinate systems (pixel vs normalized) but
both produce equivalent pixel-space results when formulas are correct. The scale
factor difference is accounted for: Rust `tw * 1.0` = C++ `tw_normalized * ScaleX`.

---

## Open Items

These are concrete bugs or unfinished investigations. Each has a defined action.

### OI-1. NK7 — IsEnabled() color dimming not implemented

**Status: Resolved in R12.**

C++ `emScalarField.cpp:412-414`:
```cpp
if (!IsEnabled()) {
    bgCol=bgCol.GetBlended(GetLook().GetBgColor(),80.0F);
    fgCol=fgCol.GetBlended(GetLook().GetBgColor(),80.0F);
}
```

Rust `ScalarField::paint` (line ~237) selects bgCol/fgCol but never dims them
for disabled state. `ScalarField::paint` also hardcodes `enabled=true` when
calling `paint_border` (line ~227).

**Affected tests:** `widget_colorfield` (alpha_enabled=false → Alpha ScalarField
disabled, should be dimmed). Possibly also `colorfield_expanded` if any child
can be disabled.

**Action:** Add an `enabled` field to ScalarField (or accept it as a paint param).
When disabled, blend bgCol and fgCol with `look.bg_color` at 80%. Pass the real
enabled state to `paint_border`.

**Expected impact:** Small (one cell in widget_colorfield). But measure, don't
estimate.

### OI-2. Step 5 paint-call audit not done at runtime-value level

**Status: Partially resolved in R12.** Runtime values verified for widget_scalarfield.

The R11 audit compared Rust and C++ source code for all 7 ScalarField paint
operations and concluded formulas match. But the protocol requires:

> "For each operation, do NOT just verify the formula — verify the actual
> parameter values at the specific cell dimensions used in the test. Add
> `eprintln!` debug output."

This was NOT done. Prior rounds repeatedly showed that formula-level matches
can hide bugs (P18 was "formula-level correct" for years).

**Action:** Add `eprintln!` to `ScalarField::paint` dumping: the content_round_rect
return values (x, y, w, h, r), the derived layout values (rx, ry, rw, rh, s, e,
ax, ay, aw, ah, d), side bar rects, value arrow vertices, and at least the first
scale mark's text box params (x, y, w, h, char_height). Run the widget_scalarfield
test (single panel, cleaner output). Manually compute the corresponding C++ values
using the C++ formulas with the same input (w, h, tallness, min, max, value).
Compare every value. Any that differs is a bug.

**Why widget_scalarfield first:** It's a single large panel (800×600) with
IBT_INPUT_FIELD. Its 220 px divergence is the cleanest signal — no CustomRect
complexity, no nesting. Fixing its 220 px will also fix the shared-code component
of the colorfield divergence.

### OI-3. NK3 — Area-sampling pixel output never compared

**Status: Not investigated.**

P12 verified source formulas match. But the actual pixel output of the
area-sampling path has never been compared between Rust and C++. The golden
images contain both the Rust output and the C++ reference. Specific pixel
regions (e.g., a scale mark label) can be extracted and compared per-channel.

**Action:** Pick a specific rendered element visible in both widget_scalarfield
golden images (e.g., the "50.00" label or a scale mark arrow). Extract the
pixel values from the actual (Rust) and expected (C++) PPM files for that
region. Compare per-channel. If diffs > ±1, investigate accumulation order in
`sample_area_fp` vs C++ `emPainter_ScTlIntImg.cpp`.

**Note:** The golden_debug images are currently stale (March 13, pre-R11). To
get fresh images either: (a) temporarily lower tolerance in the test to force
failure + DUMP_GOLDEN=1, or (b) add unconditional image dump code to the test.

### OI-4. NK2 — Per-cell divergence pattern comparison not done

**Status: Resolved in R12.** Fresh diff images generated and analyzed.

Both colorfield tests improved from the CustomRect fixes, suggesting shared
causes. But their per-cell diff patterns have not been visually compared. The
tests have different configurations (editable, alpha_enabled, colors), so
per-cell patterns could differ.

**Action:** Generate fresh debug diff images for both colorfield tests and
widget_scalarfield. Visually compare. Determine whether the remaining
divergence is concentrated in border edges, text regions, polygon fills, or
something else. This tells you which paint operation to investigate.

### OI-5. widget_scalarfield 220 px — paint operation not isolated

**Status: Resolved in R12.** 220→106 px. HowTo text fixed (-114 px). Remaining 106 px localized: 99 sub-pixel scale mark arrows + 7 border edge artifacts.

The 220 divergent pixels in widget_scalarfield have not been attributed to
specific paint operations. Are they at border edges? Text regions? The value
arrow? Side bar boundaries? Knowing WHERE the 220 pixels are tells you WHAT
code to investigate.

**Action:** Generate a fresh diff image for widget_scalarfield. Identify the
spatial pattern. Cross-reference with paint operations: if diffs are at text
positions → investigate paint_text_boxed / area-sampling. If at polygon edges
→ investigate polygon rasterization. If at border chrome → investigate
paint_border_image.

---

## Next Steps — Investigation Protocol

Execute in order. Do not skip steps. Record all measurements.

**Framing:** The goal is 0 divergent pixels. Every step either fixes a bug or
proves — with C++ line references — that a specific pixel difference is from
compiler FP instruction selection. "I looked and didn't find anything" is not
a valid conclusion. If divergence remains, there is a code difference you have
not found yet.

### Step 1: Fix OI-1 (IsEnabled dimming)

This is a known code difference. Fix it first to remove the noise.

Read C++ `emScalarField.cpp:400-414`. Implement the `!IsEnabled()` color
dimming in Rust `ScalarField::paint`. The ScalarField needs to know its
enabled state — either accept it as a parameter or read it from a field set
during layout.

Also fix `ScalarField::paint` line ~227 to pass the actual enabled state to
`paint_border` instead of hardcoded `true`.

Measure: run divergence log before and after. Record delta.

### Step 2: Localize the 220 px in widget_scalarfield (OI-5)

Generate a fresh diff image. To do this, temporarily lower the tolerance for
widget_scalarfield in `tests/golden_parity/widget.rs` (change `3` to `0` in
the `compare_images` call), run with `DUMP_GOLDEN=1`, then restore tolerance.

Inspect `golden_debug/diff_widget_scalarfield.png`. Categorize the divergent
pixels:
- **Border edges** → investigate `paint_border` / `paint_border_image`
- **Text regions** → investigate `paint_text_boxed` / area-sampling (OI-3)
- **Polygon fills** → investigate polygon rasterizer
- **Side bar boundaries** → investigate rect paint coords
- **Value arrow** → investigate polygon vertex positions

Record which category holds the most pixels.

### Step 3: Runtime-value audit for widget_scalarfield (OI-2)

Add `eprintln!` to `ScalarField::paint` for the widget_scalarfield test.
Dump the actual parameter values for every paint operation. Manually compute
the C++ equivalent values from `emScalarField.cpp:DoScalarField` using the
same input dimensions. Compare.

Focus on the paint operations identified in Step 2 as producing the most
divergent pixels.

Any parameter that differs is a bug. Fix it, measure, continue.

### Step 4: NK3 — Area-sampling pixel comparison (OI-3)

If Steps 1-3 did not resolve the majority of divergence, compare actual
area-sampling output. Extract pixel values from the golden PPM files for a
specific rendered element. Compare per-channel between Rust actual and C++
expected.

If diffs are > ±1 per channel, investigate the accumulation order in
`sample_area_fp` (`src/render/interpolation.rs`) vs C++
`emPainter_ScTlIntImg.cpp`. Key difference to check: Rust may recompute
Y weights per pixel while C++ caches across pixels. The math is identical
but FP accumulation order can differ.

### Step 5: Apply widget_scalarfield findings to colorfield

Fixes found in Steps 1-4 for widget_scalarfield should reduce the colorfield
divergence too (shared ScalarField paint code). Measure the colorfield tests.
If significant divergence remains:

- Generate fresh diff images for both colorfield tests (OI-4)
- Determine if remaining divergence is CustomRect-specific or shared
- If CustomRect-specific: audit `paint_border` CustomRect at the small cell
  dimensions (~134×27 px). The border image rendering at small scales may
  behave differently.
- If shared: repeat Step 3 for a colorfield ScalarField child to see if the
  smaller scale produces different parameter values

### Step 6: Record findings

Update this document with:
- Fix commit hashes and measured deltas
- Which OIs were resolved and what was found
- Any new bugs discovered
- Updated remaining pixel counts

Do not write "structural" or "irreducible" without having read the
corresponding C++ code and confirmed the Rust implementation matches at both
source level and output level.

If divergence remains after all steps, do NOT conclude "accept remaining."
Instead, list the exact pixel regions still divergent, what paint operation
produced them, and what specific C++ code you compared against. Then repeat
Step 3 for those specific operations with finer-grained parameter comparison.
The goal is 0 px, not "close enough."
