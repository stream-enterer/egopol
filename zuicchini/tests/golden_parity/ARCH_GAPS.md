# Architectural Gaps

Differences between C++ emPainter and Rust zuicchini that are caused by
fundamentally different algorithms, not bugs or tunable parameters.

All measurements taken at `ch_tol=1` (any pixel with channel diff > 1 counts).

## Status summary

All 53 golden parity tests pass. 29 painter tests:
- 20 tests at (1, 0.5) or tighter — near-exact parity
- 8 tests at (1..2, 1.0) or (2, 0.5) — minor rounding diffs
- 1 test at (19, 0.5) — image scaling boundary residual

Gaps C1, C2, C4, C5, C6 are **CLOSED**. C3 is **TIGHTENED** (19 from 70).
C7 remains at (50, 1.0) — design difference (integer sqrt vs float).

---

## ~~C1: polygon-coverage~~ — CLOSED

**Resolved** by porting the C++ polynomial coverage rasterizer. The
`rasterize_polynomial()` function in `scanline.rs` now matches the C++
`emPainter::PaintPolygon` algorithm exactly.

Tests now passing at (ch_tol=1, 0.5%):
- `ellipse_basic`, `ellipse_sector`, `ellipse_small`
- `polygon_tri`, `polygon_star`, `polygon_complex`
- `clip_basic`, `multi_compose`, `line_basic`

## ~~C2: stroke-expansion~~ — CLOSED

**Resolved** by porting C++ `PaintSolidPolyline` stroke expansion geometry
(miter limiting, normal computation, join handling, cap generation).

Tests now passing at (1, 0.1) to (1, 0.5):
- `outline_rect`, `outline_polygon`, `outline_ellipse`, `line_dashed`
- `polyline`, `line_thick`

`outline_round_rect` at (2, 0.5) — minor arc approximation segment count diff.

## C3: interpolation — Image scaling filter (TIGHTENED)

- **Root cause:** Different adaptive filter and fixed-point arithmetic between
  C++ and Rust at image boundaries with EXTEND_ZERO.
- **Fix applied:** Ported C++ premultiplied-alpha bicubic interpolation with
  full 2D premul accumulation (`r*a*weight` across all 16 taps), FINPREMUL
  (divide by 255), WRITE_SHR_CLIP (shift + clamp RGB to alpha). Added
  EXTEND_ZERO for even-channel images, pixel-center offset for upscaling,
  and high-precision 1024-scale factor table matching C++ BicubicFactorsTable.
- **Affected tests:**
  - `image_scaled` — max_diff=19, ~1% differ at ch_tol=10
    (was: max_diff=118, 30.68% differ at ch_tol=1)
- **Residual:** Edge pixels where the 4x4 bicubic kernel overlaps the image
  boundary with EXTEND_ZERO. C++ adaptive filter handles this differently.
- **Assessment:** Tolerance tightened from (70, 0.5) to (19, 0.5).

## ~~C4: stroke-ends~~ — CLOSED

**Resolved** by porting C++ `PaintArrow` decoration geometry for all 17
`StrokeEndType` variants.

- `line_ends_all` — now at (1, 1.0). Was (80, 17.0).

## ~~C5: compound~~ — CLOSED

Previously tracked as downstream effect of C1. Now that polygon coverage
is exact, `multi_compose` passes at (ch_tol=1, 0.5%).

## ~~C6: bezier-flattening~~ — CLOSED

**Resolved** by matching C++ bezier subdivision algorithm.

- `bezier_filled` — now at (1, 0.1). Was (80, 4.5).
- `bezier_stroked` — now at (1, 1.0). Was (80, 3.5).

## C7: gradient-rounding — Gradient texturing at boundaries

- **Root cause:** C++ uses integer sqrt lookup table for radial gradient
  distance computation; Rust uses f64 sqrt. This produces small per-pixel
  diffs across many pixels.
- **Affected tests:**
  - `gradient_radial` — max_diff=50, 25.08% differ at ch_tol=1.
    Tolerance: (50, 1.0).
  - `gradient_h`, `gradient_v` — max_diff=2, tolerance (2, 1.0).
- **Assessment:** Design difference. The integer sqrt approach trades accuracy
  for speed; f64 sqrt is more accurate. Not worth porting.

---

## Remaining non-trivial tolerances

| Test | Tolerance | Root cause |
|------|-----------|-----------|
| `gradient_radial` | (50, 1.0) | C7: integer sqrt vs float sqrt |
| `image_scaled` | (19, 0.5) | C3: boundary interpolation |
| `gradient_h` | (2, 1.0) | Gradient interpolation rounding |
| `gradient_v` | (2, 1.0) | Gradient interpolation rounding |
| `canvas_color` | (2, 0.5) | Canvas-color alpha blend rounding |
| `outline_round_rect` | (2, 0.5) | Arc approximation segment count |
| `line_ends_all` | (1, 1.0) | Minor sub-pixel diffs at decorations |
| `bezier_stroked` | (1, 1.0) | Minor sub-pixel diffs at stroke+arrow |
