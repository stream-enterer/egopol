# Widget Comparison Priority Order

Ranked by composite score: `(complexity * coverage_gap_weight * recency_weight * size_asymmetry_flag)`.

## Tier 1: High Priority (compare first)

These have the highest bug density risk due to complexity, recent churn, or suspicious size asymmetry.

| # | Widget | Why |
|---|--------|-----|
| 1 | **TextField** | Largest widget (3378 Rust LOC). Heavy recent churn (cursor blink, selection, multi-line, drag). Mixed fidelity layers. Multiple golden interaction tests but complex state machine. |
| 2 | **Border** | Second largest (2676 LOC). Core paint path — every widget renders through it. Pixel-exact fidelity required. 9-slice, label layout, outer/inner border types. |
| 3 | **ListBox** | Large (1992 LOC, C++ 1558). Selection modes, keywalk, custom item panels. Recent fixes (disabled state, scroll-down). Size growth suggests added logic. |
| 4 | **ScalarField** | Medium-large (982 LOC). Marks, range, increment/decrement. Mixed pixel + state. HowTo text constants ported from C++ line references. |
| 5 | **RadioButton** | Size asymmetry (819 Rust vs 520 C++). Dot rendering pixel path. Recent fix (dot offset, pressed state). |

## Tier 2: Medium Priority

| # | Widget | Why |
|---|--------|-----|
| 6 | **ColorField** | Paint-heavy (747 LOC). Expansion modes. Golden test coverage exists but color picker geometry is complex. |
| 7 | **FileSelectionBox** | Inverse size asymmetry (665 Rust vs 1620 C++) — likely missing significant logic. |
| 8 | **CheckButton** | Size asymmetry (340 Rust vs 190 C++). Recent fix (pressed state, DoButton non-boxed path). |
| 9 | **CheckBox** | Size asymmetry (346 Rust vs 90 C++). Suspicious — C++ is a thin wrapper, Rust is 4x larger. |
| 10 | **RadioBox** | Size asymmetry (350 Rust vs 89 C++). Same concern as CheckBox. |
| 11 | **Button** | Core widget (557 LOC). Rounded-rect hit test — formula must match C++ `CheckMouse()`. Recent fix (pressed state). |

## Tier 3: Lower Priority

| # | Widget | Why |
|---|--------|-----|
| 12 | **Splitter** | Moderate (293 LOC). Has golden interaction tests (drag, layout). |
| 13 | **FilePanel** | Moderate (479 LOC). State-only fidelity. |
| 14 | **Dialog** | Size asymmetry (198 Rust vs 590 C++) — may be missing features. |
| 15 | **Tunnel** | Moderate (332 LOC). Pixel + geometry. Has golden test. |
| 16 | **FileDialog** | Moderate (341 LOC). State-only. |
| 17 | **CoreConfigPanel** | Large but state-only (1569 LOC). Config UI, not performance-critical. |
| 18 | **Look** | Small (129 LOC). Theme values — easy to verify. |
| 19 | **Label** | Small (134 LOC). Text fitting algorithm — pixel fidelity but simple. |
| 20 | **ErrorPanel** | Tiny (92 LOC). Trivial. |

## Non-Widget Priority

| # | Component | Why |
|---|-----------|-----|
| A | **View** | Largest (3757 C++ LOC). Zoom, scroll, navigation — geometry fidelity. Recent fixes (zoom fix-point, scroll). |
| B | **ViewAnimator** | Large (2527 C++ LOC). Multiple animator types. Trajectory golden tests exist. |
| C | **Panel** | Large (2696 C++ LOC). Core tree management. Well-tested via golden interaction tests. |
| D | **Layouts** | Medium (878+854+683 C++). Geometry fidelity. Extensive golden tests. |
| E | **ViewInputFilter** | Medium (1614 C++). Input handling. Golden tests for zoom/pan/scroll. |
