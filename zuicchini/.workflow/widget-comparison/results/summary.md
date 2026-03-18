# Widget Comparison Summary — Final Session 1 Report

**Date**: 2026-03-18

## Finding Counts by Widget

| Widget | HIGH | MEDIUM | LOW | Total | Status |
|--------|------|--------|-----|-------|--------|
| Label | — | — | — | 6 (2 BUG, 3 GAP, 1 NOTE) | DONE |
| Button | — | — | — | 14 (3 BUG, 2 SUSPECT, 7 GAP, 2 NOTE) | DONE |
| CheckBox | — | — | — | 8 (2 BUG, 1 SUSPECT, 2 GAP, 2 INFO) | DONE |
| Splitter | — | 2 | 9 | 11 | DONE |
| ScalarField | 2 | 4 | 4 | 10 | DONE |
| ColorField | — | 1 | 3 | 8 (+ 4 CC) | DONE |
| RadioButton+RadioBox | — | 3 | 4 | 8 (+ 1 CC) | DONE |
| ListBox | — | 2 | 9 | 14 (+ 3 INFO) | DONE |
| TextField | 4 | 9 | 5 | 18 | DONE |
| TkTest (Layer 1) | — | — | — | 10+ GAPs | DONE (manual) |
| TestPanel (Layer 2) | — | — | — | — | Structural only |

**Session 1 grand total**: ~107 findings across 9 widgets + 2 composition layers

## Cross-Cutting Concerns (6 systemic issues affecting most/all widgets)

| ID | Issue | Impact |
|----|-------|--------|
| CC-01 | Code duplication across Button-family (5 widgets) | Fixes don't propagate |
| CC-02 | set_* methods don't fire signals/callbacks | Programmatic changes silent |
| CC-03 | No disabled state rendering | No visual dimming, no input gating |
| CC-04 | No VCT_MIN_EXT guard on input | Tiny widgets are clickable |
| CC-05 | DoLabel alignment defaults (Center vs Left) | All bordered widgets affected |
| CC-06 | hit_test() vs check_mouse() face-inset divergence | Clickable area slightly larger |

## Top 12 Highest-Severity Findings

| # | Widget | Finding | Severity |
|---|--------|---------|----------|
| 1 | TextField | Selection model divergence (anchor vs start/end, no clipboard publish) | HIGH |
| 2 | TextField | Undo/redo architecture (full-snapshot vs incremental, different visual behavior) | HIGH |
| 3 | TextField | Backspace modifier handling too permissive | HIGH |
| 4 | TextField | Ctrl+Left/Right calls wrong word-boundary function | HIGH |
| 5 | ScalarField | f64 vs i64 fundamental type mismatch | HIGH |
| 6 | ScalarField | Drag is relative (Rust) vs absolute (C++) | HIGH |
| 7 | TextField | Tab rendering not expanded during multi-line paint | MEDIUM |
| 8 | TextField | Double-click on delimiters selects empty range | MEDIUM |
| 9 | TextField | Drag-move has no live visual feedback | MEDIUM |
| 10 | RadioBox | Doesn't register in group on construction | MEDIUM |
| 11 | RadioButton | Drop doesn't re-index or adjust selection | MEDIUM |
| 12 | ListBox | Row height mismatch between hit test and paint | MEDIUM |

## Pixel Fidelity Assessment

A separate broad audit confirmed:
- All production blend paths use correct Blinn div255 formula
- Coverage, area sampling, bilinear interpolation, radial gradient sqrt table all faithful
- Fixed12 arithmetic correct, AffineMatrix composition correct
- AVX2 SIMD blend allows +/-1 LSB (within golden test tolerance)
- **Compositing pipeline: HIGH FIDELITY — no bugs found**

## TkTest Composition Gaps

Missing from Rust TkTest vs C++:
- Tunnels section (4 variants), Test Dialog section, File Selection section
- NoEOI button, custom scalar formatters (TextOfTimeValue, TextOfLevelValue)
- Single-column ListBox, Custom ListBox with recursive item panels
- Cycle() engine logic (signal wiring, dialog lifecycle)

## Remaining Work

### Priority (next session)
- **Border** (2676 LOC) — core render path, affects ALL widgets, CC-05/CC-06 root cause
- **CheckButton** (340 LOC) — CC-01 code duplication verification

### Lower Priority
- FileSelectionBox (inverse size asymmetry — likely missing logic)
- Dialog (size asymmetry 198 vs 590 LOC)
- Look, Tunnel, FilePanel, FileDialog, ErrorPanel, CoreConfigPanel

### Layer 2 (TestPanel full audit)
- 350+ LOC of paint primitive calls to verify parameter-identical
- Input log format string comparison
- PolyDrawPanel interactive widget comparison
