# Zero-Tolerance Golden Parity

## Objective

Every golden test passes at `compare_images(name, actual, expected, w, h, 0, 0.0)` — zero channel tolerance, zero failure percentage. No exceptions, no "known divergences."

## Current State

241 golden tests exist. At tol=0, **42 fail** (measured 2026-04-01). The tests currently pass only because non-zero tolerances hide divergences — some as high as `ch_tol=130, max_fail=75%`.

### Confirmed Root Cause: Gradient Hash Formula Regression

Commit `6ae3e74` (2026-03-12) fixed `Color::lerp` to use the C++ gradient hash formula:
```
((a*(255-g) + b*g) * 257 + 0x8073) >> 16
```
Commit `66f299b` added `+0.5` pixel center sampling. Together they achieved exact parity (`gradient_h`, `gradient_v` at ch_tol=0).

Commit `b645fb3` (2026-03-14) replaced the hash formula with `emColor::GetBlended`'s formula:
```
w2 = (weight * 655.36 + 0.5) as i32; (a*w1 + b*w2 + 32768) >> 16
```
This matches `emColor::GetBlended` but NOT the C++ gradient rendering pipeline, which uses the hash formula internally (`emPainter_ScTlPSInt.cpp:297`). The regression was never caught because tolerances hid it.

### All 42 Failures at tol=0

Root causes are **unknown** except for eagle_logo (gradient regression) and gradient_radial/multi_compose (likely related). The implementation must audit each against C++ source — do not assume root causes from prior agent documentation.

| Test | max_diff | fail% | Notes |
|------|----------|-------|-------|
| testpanel_expanded | 255 | 4.56% | Composite of many primitives |
| testpanel_root | 255 | 2.79% | Composite of many primitives |
| tktest_1x | 239 | 8.84% | Widget grid composite |
| tktest_2x | 239 | 2.09% | Widget grid composite 2x |
| widget_file_selection_box | 237 | 2.96% | Composite widget |
| widget_checkbox_checked | 236 | 0.07% | |
| composed_border_nest | 153 | 2.07% | |
| widget_listbox | 136 | 0.04% | |
| cosmos_item_border | 130 | 0.67% | |
| stress_test_overlay | 111 | 0.76% | |
| widget_border_round_rect | 79 | 0.003% | |
| starfield_small | 69 | 0.03% | |
| colorfield_expanded | 54 | 0.75% | |
| bezier_stroked | 53 | 0.18% | |
| starfield_large | 53 | 0.02% | |
| listbox_expanded | 33 | 0.07% | |
| widget_button_normal | 31 | 0.03% | |
| widget_radiobutton | 31 | 0.04% | |
| widget_textfield_content | 26 | 0.04% | |
| widget_textfield_empty | 26 | 0.04% | |
| widget_textfield_single_char_square | 26 | 0.05% | |
| widget_listbox_single | 25 | 0.08% | |
| widget_listbox_empty | 25 | 0.03% | |
| widget_colorfield | 24 | 0.27% | |
| widget_colorfield_alpha_near | 24 | 0.71% | |
| widget_colorfield_alpha_opaque | 24 | 0.27% | |
| widget_colorfield_alpha_zero | 24 | 0.57% | |
| widget_border_roundrect_thin | 24 | 0.0008% | |
| widget_checkbox_unchecked | 22 | 0.04% | |
| widget_splitter_v_extreme_tall | 19 | 0.02% | |
| widget_scalarfield | 12 | 0.25% | |
| widget_scalarfield_zero_range | 12 | 0.20% | |
| widget_scalarfield_min_value | 12 | 0.07% | |
| widget_scalarfield_max_value | 12 | 0.06% | |
| eagle_logo | 2 | 54.72% | **Confirmed**: gradient hash regression |
| composed_splitter_content | 1 | 0.002% | |
| widget_splitter_h | 1 | 0.0002% | |
| widget_splitter_h_pos0 | 1 | 0.0002% | |
| widget_splitter_h_pos1 | 1 | 0.0002% | |
| widget_error_panel | 1 | 0.0006% | |
| multi_compose | 1 | 7.18% | |
| image_scaled | 1 | 0.75% | |
| gradient_radial | 1 | 0.05% | |

## Approach

### Phase 1: Restore gradient hash formula

Restore the `((a*(255-g)+b*g)*257+0x8073)>>16` formula to the gradient sampling path. This was proven to achieve exact parity in the zuicchini era. `GetBlended` itself stays unchanged — it correctly matches `emColor::GetBlended`. The gradient path needs its own blending that matches the C++ scanline tool's hash formula.

**Scope**: `sample_linear_gradient` in `emPainterInterpolation.rs`. Possibly also the radial gradient path if it uses `GetBlended`.

**Gate**: `eagle_logo`, `gradient_h`, `gradient_v`, `gradient_radial` pass at tol=0.

### Phase 2: Drop all tolerances to 0, fix what breaks

1. All `compare_images` calls already use `(0, 0.0)`.
2. Run the full golden suite.
3. For each failure, audit the Rust code against the corresponding C++ source in `~/git/eaglemode-0.96.4/`. The C++ source is the single source of truth — not comments, not documentation, not prior agent analysis.
4. Fix the Rust code to reproduce the C++ formula exactly.
5. **After every fix, run the full golden suite.** Never accept a regression — if fixing test A causes test B to fail, both tests share a code path and the fix is wrong. Back it out and understand why before proceeding.
6. Repeat until 0 failures.

**Gate**: `cargo test --test golden -- --test-threads=1` passes with 0 failures, all `compare_images` calls at `(0, 0.0)`.

### Phase 3: Clean up

1. Remove all `ch_tol`/tolerance comments that document "known divergences" — they no longer exist.
2. Update the divergence inventory memory to reflect zero-tolerance state.
3. Remove `channel_tolerance` and `max_failure_pct` parameters from `compare_images` if they are now always 0 — or keep them as dead code defense against regression (implementer's judgment).

**Gate**: `cargo clippy -- -D warnings` and `cargo-nextest ntr` pass.

## Methodology for Phase 2 Audits

For each failing test:

1. **Generate diff image**: `DUMP_GOLDEN=1 cargo test --test golden <name>` — visually identify where pixels differ.
2. **Trace the rendering path**: What Rust paint method produces the divergent pixels? Follow from the test's `Paint` call to the pixel output.
3. **Find the C++ equivalent**: Locate the same rendering path in `~/git/eaglemode-0.96.4/src/emCore/`. The C++ file structure mirrors the Rust structure. Do not trust comments in the Rust code or prior agent documentation about what the C++ does — read the actual C++ source.
4. **Classify the divergence**:
   - **Arithmetic**: Scattered ±1-2 LSB noise across many pixels. Cause: wrong rounding, wrong formula, wrong precision. Fix: port the exact C++ formula.
   - **Structural**: Large contiguous regions of wrong color, missing shapes, shifted elements. Cause: wrong paint call, wrong branch, wrong compositing order, missing code path. Fix: fix the rendering logic, not the arithmetic.
   - If the diff image is ambiguous, `DUMP_GOLDEN=1` produces actual/expected/diff PNGs — compare visually before assuming arithmetic.
5. **Compare formulas** (for arithmetic divergences): Diff the actual arithmetic. Look for:
   - Different rounding (`>>16` vs `/256`, `+0.5` vs truncation)
   - Different precision (8-bit vs 16-bit weights)
   - Different formula entirely (hash formula vs GetBlended)
   - Missing pixel-center offset (`+0.5`)
   - Different operation order
6. **Fix**: Port the exact C++ formula or logic. Do not approximate.
7. **Verify**: Run the **full** golden suite, not just the test you're fixing. If any previously-passing test regresses, the fix is wrong — back it out.

## Antipattern: Fix-Regress Loops

History: DIV-008 (`6ae3e74`) fixed the gradient formula. Two days later, `b645fb3` replaced it with a different formula, undoing the fix. Neither session knew about the other. This pattern — fix A, break B, fix B, break A — is the primary risk when 42 tests share rendering code paths.

**Defenses:**

1. **Full suite after every change.** No exceptions. A fix that passes its target test but regresses another test is not a fix.
2. **Never modify a formula without reading the C++ source for that specific code path.** "Matching GetBlended" is not the same as "matching the gradient pipeline." The C++ uses different formulas in different contexts — verify which one applies.
3. **If a code path is shared by multiple tests, fix it once correctly.** Do not apply per-test patches. If the same blend function serves gradients, images, and compositing, the fix must be correct for all three.
4. **Track the pass/fail count monotonically.** The number of passing tests must never decrease during Phase 2. If it does, stop and investigate before proceeding.

## Non-Goals

- Refactoring Rust architecture to match C++ template structure
- Performance optimization
- Adding new golden tests
- Fixing non-pixel tests (trajectory, behavioral, input — these already pass)

## C++ Reference Files

| C++ File | What it contains |
|----------|-----------------|
| `emPainter_ScTlPSInt.cpp` | Hash formula blending for gradients and images: `((x*257+0x8073)>>16` |
| `emPainter_ScTlIntGra.cpp` | Linear/radial gradient interpolation (integer stepping) |
| `emPainter_ScTlPSCol.cpp` | Color blending with canvas color hash |
| `emPainter.cpp` | PaintEllipse, PaintRoundRect, PaintBezier, PaintBorderImage |
| `emColor.cpp` | GetBlended (correct as-is in Rust) |
| `emTexture.h` | emLinearGradientTexture setup (TX, TDX, TDY computation) |

## Success Criteria

```bash
# All compare_images calls use (0, 0.0)
grep -r 'compare_images' crates/eaglemode/tests/golden/*.rs | grep -v '0, 0.0'
# Should return nothing (except the function definition itself)

# All golden tests pass
cargo test --test golden -- --test-threads=1
# 241 passed; 0 failed

# Divergence log clean
cat target/golden-divergence/divergence.jsonl | grep -v '"max_diff":0' | grep -v '"pass":true' | grep -v '"mismatches":0'
# Should return nothing
```
