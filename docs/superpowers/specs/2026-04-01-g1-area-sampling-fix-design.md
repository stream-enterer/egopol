# G1: Area Sampling Fix

## Objective

Make `interpolate_scanline_area_sampled` produce byte-identical output to C++ `InterpolateImageAreaSampled` for all inputs. Fix 23 G1 tests to pass at tol=0.

## Background

The current Rust implementation was written across multiple sessions (commits `d9a20f4`, `f522bb8`, and others). It reproduces the C++ area sampling algorithm but diverges on carry-over weight management and column-reuse (`pCy`) caching. At the tolerances that existed when it was written, it passed. At tol=0, 23 tests fail with max_diff up to 255.

Previous attempts to debug the carry simulation incrementally have not achieved exact parity. This spec calls for a literal port of the C++ source.

## Approach

Literal port of C++ `emPainter_ScTlIntImg.cpp` lines 735-826 (`InterpolateImageAreaSampled`) into Rust, replacing the current `interpolate_scanline_area_inner` and `simulate_carry_chain`. The implementer reads the C++ source, translates it to Rust preserving loop structure and arithmetic, and adapts for the batching constraint (256px `InterpolationBuffer`). The spec does not prescribe how to handle batching — the implementer determines the correct adaptation.

### C++ Reference

- **Primary:** `~/git/eaglemode-0.96.4/src/emCore/emPainter_ScTlIntImg.cpp` lines 735-826
- **Macros:** Same file, search for `DEFINE_AND_SET_COLOR`, `ADD_MUL_COLOR`, `READ_PREMUL_MUL_COLOR`, `WRITE_NO_ROUND_SHR_COLOR`, `FINPREMUL_SHR_COLOR` — these expand to the actual arithmetic
- **Template params:** `CHANNELS` (1/3/4), `EXTENSION` (EXTEND_EDGE/EXTEND_ZERO)

### Rust Files

- **Replace:** `crates/emcore/src/emPainterInterpolation.rs` — `interpolate_scanline_area_inner`, `simulate_carry_chain`, and related helpers
- **Possibly modify:** `crates/emcore/src/emPainter.rs` — callers of `interpolate_scanline_area_sampled` (if `carry_origin_x` or batching interface changes)
- **Preserve:** `AreaSampleTransform` struct, `SectionBounds`, `ImageExtension` enum, `rational_inv` — these are used by callers and other code paths

### Batching Constraint

The Rust callers process scanlines in batches of up to 256 pixels (limited by `InterpolationBuffer`). The C++ processes entire scanlines in one call. The implementer must determine how to handle carry state across batch boundaries while matching C++ output exactly. The existing `carry_origin_x` / `simulate_carry_chain` mechanism is one approach; there may be others.

## The 23 G1 Tests

| Test | max_diff | fail% | Entry point |
|------|----------|-------|-------------|
| testpanel_expanded | 255 | 4.56% | PaintBorderImage |
| composition_tktest_1x | 239 | 8.76% | PaintBorderImage |
| composition_tktest_2x | 239 | 2.09% | PaintBorderImage |
| widget_file_selection_box | 237 | 2.96% | PaintBorderImage |
| composed_border_nest | 153 | 2.07% | PaintBorderImage |
| widget_listbox | 136 | 0.04% | PaintBorderImage |
| starfield_small | 69 | 0.03% | PaintImageColored |
| colorfield_expanded | 54 | 0.75% | PaintBorderImage |
| starfield_large | 53 | 0.02% | PaintImageColored |
| listbox_expanded | 33 | 0.07% | PaintBorderImage |
| widget_button_normal | 31 | 0.03% | PaintBorderImage |
| widget_radiobutton | 31 | 0.04% | PaintBorderImage |
| widget_textfield_content | 26 | 0.04% | PaintBorderImage |
| widget_textfield_empty | 26 | 0.04% | PaintBorderImage |
| widget_textfield_single_char_square | 26 | 0.05% | PaintBorderImage |
| widget_listbox_single | 25 | 0.08% | PaintBorderImage |
| widget_listbox_empty | 25 | 0.03% | PaintBorderImage |
| widget_colorfield | 24 | 0.27% | PaintBorderImage |
| widget_colorfield_alpha_near | 24 | 0.71% | PaintBorderImage |
| widget_colorfield_alpha_opaque | 24 | 0.27% | PaintBorderImage |
| widget_colorfield_alpha_zero | 24 | 0.57% | PaintBorderImage |
| widget_checkbox_unchecked | 22 | 0.04% | paint_image_full |
| widget_splitter_v_extreme_tall | 19 | 0.02% | PaintBorderImage |

## Verification

- Run `cargo test --test golden -- --test-threads=1` after every change
- 23 G1 tests must pass (max_diff=0)
- 199 currently-passing tests must not regress
- 19 non-G1 failing tests must not get worse (max_diff unchanged or improved)
- `parallel_benchmark` must still pass (byte-identical parallel vs sequential)
- `cargo clippy -- -D warnings` and `cargo-nextest ntr` must pass

## Constraint

Full golden suite after every change. Pass count must never decrease. If fixing area sampling causes a previously-passing test to fail, the fix is wrong — back it out.

## Non-Goals

- Fixing G2-G9 (polygon rasterizer, adaptive table, roundrect, fill_span_blended, radial gradient, eagle_logo gradient, cosmos border, checkbox stroke)
- Performance optimization
- Refactoring callers beyond what's needed for the interface change
