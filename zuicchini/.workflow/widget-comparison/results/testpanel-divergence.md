# TestPanel (Layer 2) Structural Comparison

**Date**: 2026-03-18
**C++ file**: `emTestPanel.cpp` (1491 LOC) + `emTestPanel.h` (201 LOC)
**Rust file**: `examples/test_panel.rs` (~1400 LOC)

## Structure Match Summary

| Feature | C++ | Rust | Match? |
|---------|-----|------|--------|
| Background color | 0x001C38FF | ✓ | Verify |
| Paint primitives (polygons, ellipses, rects, etc.) | ~350 LOC of paint calls | ✓ | Needs audit |
| Input logging | 20-entry InputLog, PaintText display | ✓ | Verify format |
| State display (Focused, InFocusedPath, etc.) | ✓ | ✓ | Verify |
| Recursive children (TP1-TP4) | 4 self-embedded emTestPanel instances | ✓ | ✓ |
| BgColorField | Editable emColorField with signal wiring | ✓ | Verify signal |
| PolyDrawPanel | Interactive polygon drawing | ✓ | Needs audit |
| TkTestGrp | Wrapper around TkTest | ✓ | Verify |
| Auto-expansion threshold | 900.0 | ✓ | ✓ |
| Child layout positions | 7 children with explicit Layout() coords | ✓ (CHILD_LAYOUT const) | ✓ |
| VCT_WIDTH < 25 early return | Skips detail paint when too small | ? | Verify |
| ControlPanel (CreateControlPanel) | Label in control panel with identity/color | ? | Likely missing |
| BgColor persistence (VarModel) | Saves BgColor across view changes | ? | Likely missing |
| TestImage (teddy.tga) | Loaded via emGetInsResImage | ? | Verify |
| Focused color selection | 3-way: Focused/InFocusedPath/Other | ✓ | Verify colors |
| GetTitle() | Returns "Test Panel" | ? | May not exist |

## Paint Primitives to Audit (C++ emTestPanel::Paint)

These are critical for pixel-exact comparison since they exercise the Painter API:

1. **Background fill**: `PaintRect(0,0,1,h,BgColor,canvasColor)`
2. **Border outline**: `PaintRectOutline(0.01,0.01,1.0-0.02,h-0.02,0.02,...)`
3. **Title text**: `PaintTextBoxed(0.02,0.02,0.49,0.07,"Test Panel",...)`
4. **State text**: `PaintTextBoxed(0.05,0.4,0.9,0.05,...,EM_ALIGN_LEFT)`
5. **Priority/MemLim text**: `PaintTextBoxed(0.05,0.45,...,EM_ALIGN_LEFT)`
6. **Input log lines**: 20 `PaintText` calls
7. **Tab test text**: `PaintTextBoxed` with tab chars and `relLineSpace=0.1`
8. **Triangle polygon**: 3 vertices at (0.7,0.6), (0.6,0.7), (0.8,0.8)
9. **Hollow rectangle** (10-vertex polygon with hole): CW outer + CCW inner
10. **Reverse-wound hollow rect**: Same but opposite winding
11. **Circle polygon**: 64-vertex circle at (0.65, 0.85)
12. **Clipped circle**: 64-vertex circle rendered through a clipped emPainter
13. **Ellipse polygon**: 64-vertex ellipse
14. **Overlapping triangles**: 4 pairs with/without canvasColor
15. **Thin polygons**: Degenerate near-line triangles
16. **Small ellipses**: Various sizes, some with sectors
17. **Rect/round-rect outlines**: Various thicknesses and styles
18. **Ellipse outlines**: Various, including arcs and sectors
19. **Bezier curves**: Filled and stroked variants
20. **Line with stroke ends**: All StrokeEnd types in a radial pattern
21. **Polyline**: With contour arrow start
22. **Polygon outline**: Triangle
23. **Gradient fills**: Linear, radial on polygons and rects
24. **Image textures**: Various extensions (tiled, edge, zero)
25. **Colored image texture**: `PaintImageColored` equivalent

## Known Divergences from TkTest Audit (see tktest-divergence.md)

The TkTest inside TestPanel has the same gaps as the standalone TkTest:
- Missing: Tunnels, Test Dialog, File Selection
- Missing: NoEOI button, custom scalar formatters, single-column listbox, custom listbox

## Items Requiring Detailed Audit

1. **Every paint primitive call** must be parameter-identical (coordinates, colors, stroke types)
2. **Input log format** string must match C++ exactly for golden test comparison
3. **PolyDrawPanel** — complex interactive widget with configurable stroke/fill — needs its own comparison
4. **Signal wiring** — BgColorField → BgColor → repaint/relayout cycle
5. **Tab character rendering** in text
