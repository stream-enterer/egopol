# Label Audit Report

**Date**: 2026-03-18
**Agent**: Calibration batch, label auditor
**C++ files**: emLabel.cpp (50 LOC), emLabel.h (61 LOC) = 111 LOC — inherits from emBorder (1970 LOC)
**Rust file**: label.rs (134 LOC)

## Findings: 6 total

### [BUG] Label block horizontal alignment: C++ left-aligns, Rust centers
- **C++**: emBorder.cpp:1293-1301 — `LabelAlignment` defaults to `EM_ALIGN_LEFT`, skips centering
- **Rust**: label.rs:68 — `cx += (cw - w2) * 0.5` always centers
- When text fits horizontally (w2 <= content width), C++ places text at left edge, Rust centers it
- **Masked by golden test**: "Hello World" at 1.0x0.75 is likely width-constrained (w2 > w), so centering branch never runs
- A short caption on a wide panel would expose this
- **Confidence**: high | **Coverage**: effectively uncovered (golden passes by coincidence)

### [BUG] Text line alignment hardcoded to Center instead of Left
- **C++**: emBorder.cpp:1394 — passes `CaptionAlignment` (default `EM_ALIGN_LEFT`) to `PaintTextBoxed`
- **Rust**: label.rs:96 — `TextAlignment::Center` hardcoded
- For single-line text: invisible. For multi-line captions with different-length lines: left vs center alignment of each line
- **Confidence**: high | **Coverage**: uncovered (golden uses single-line text)

### [GAP] No description or icon support
- **C++**: emLabel.h:40-45 — constructor accepts description and icon, DoLabel lays them out
- **Rust**: label.rs:16-23 — only caption
- Likely intentional scope reduction since emLabel is typically caption-only
- **Confidence**: high | **Coverage**: uncovered

### [GAP] No disabled state handling
- **C++**: emLabel.cpp:44-47 — `IsEnabled()` check, `GetTransparented(75.0)` dims fg_color
- **Rust**: label.rs:33 — hardcoded `enabled: true`, no transparency
- Cross-cutting: CC-03
- **Confidence**: high | **Coverage**: uncovered

### [GAP] No alignment configurability
- **C++**: emBorder.h:87-99 — `SetLabelAlignment`, `SetCaptionAlignment`, `SetDescriptionAlignment`
- **Rust**: no setters, hardcoded to wrong defaults (Center instead of Left)
- **Confidence**: high | **Coverage**: uncovered

### [NOTE] canvas_color passed as TRANSPARENT
- **C++**: passes canvasColor from border system through to PaintTextBoxed
- **Rust**: label.rs:94 — hardcodes `Color::TRANSPARENT`
- For OBT_MARGIN (Label's default): functionally equivalent since Margin doesn't fill background
- **Confidence**: low | **Coverage**: covered (golden passes)

## Summary

| Severity | Count |
|----------|-------|
| BUG | 2 |
| GAP | 3 |
| NOTE | 1 |

## Recommended Tests
1. Short caption ("Hi") on wide panel (1.0 x 0.3) — exposes horizontal alignment bug
2. Multi-line caption ("Line One\nLine Two") — exposes text line alignment bug
3. Disabled label rendering (requires adding enabled state first)

## Overall Assessment
Correct for the most common case (single-line caption that fills width). The alignment bugs are REAL but masked by the single golden test. No pixel arithmetic errors. The main risk is that these defaults affect ALL border-based widgets that use DoLabel internally, not just Label itself.
