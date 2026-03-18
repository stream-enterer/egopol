# TkTest Composition Divergence Report

**Date**: 2026-03-18
**Comparison**: C++ `emTkTestPanel.cpp` (480 LOC) vs Rust `test_toolkit.rs` (436 LOC)
**Severity**: Multiple GAPs

## Summary

The Rust test_toolkit.rs is missing entire widget categories and many individual widget configurations present in the C++ TkTest. These are not bugs per se, but they represent missing test coverage — widgets and configurations that exist in C++ but have no Rust test exerciser.

## Missing Widget Categories

### [GAP] Tunnels section — entirely absent
- **C++ location**: emTkTestPanel.cpp:258-276
- **C++ content**: 4 tunnel variants (default, depth=30, square-end tallness=1.0, square-end+zero-depth)
- **Rust**: No tunnel group in test_toolkit.rs at all
- **Impact**: Tunnel widget has only golden render test, no TkTest composition exercise

### [GAP] Test Dialog section — entirely absent
- **C++ location**: emTkTestPanel.cpp:329-344
- **C++ content**: 7 checkboxes (Top-Level, VF_POPUP_ZOOM, WF_MODAL, WF_UNDECORATED, WF_POPUP, WF_MAXIMIZED, WF_FULLSCREEN) + "Create Test Dialog" button + Cycle() logic
- **Rust**: No dialog test section
- **Impact**: Dialog widget, window flags, and popup zoom entirely unexercised

### [GAP] File Selection section — entirely absent
- **C++ location**: emTkTestPanel.cpp:346-361
- **C++ content**: FileSelectionBox with 3 filter types + Open/Open Multi/Save buttons + FileDialog with Cycle() finish handling
- **Rust**: No file selection section
- **Impact**: FileSelectionBox and FileDialog composition untested

## Missing Individual Widgets

### [GAP] Button: missing NoEOI variant
- **C++ location**: emTkTestPanel.cpp:161-162 — `bt->SetNoEOI()`
- **Rust**: Only 2 buttons (Button, Long Desc); C++ has 3 (Button, Long Desc, NoEOI)
- **Impact**: NoEOI behavior unexercised

### [GAP] Button: missing long description
- **C++ location**: emTkTestPanel.cpp:156-160 — 100 lines of repeated description text
- **Rust**: Button "Long Desc" exists but has no SetDescription equivalent

### [GAP] CheckButton/CheckBox: only 2+2 vs C++ 3+3
- **C++ location**: emTkTestPanel.cpp:164-171
- **Rust**: 2 CheckButtons + 2 CheckBoxes vs C++ 3+3
- **Impact**: Minor — same widget, just fewer instances

### [GAP] ScalarField: 3 variants vs C++ 6
- **C++ location**: emTkTestPanel.cpp:207-240
- Missing Rust:
  - `sf4` "Level" with custom TextOfValueFunc (TextOfLevelValue)
  - `sf5` "Play Length" with TextOfTimeValue, complex mark intervals, signal wiring
  - `sf6` "Play Position" linked to sf5 max value
- **Impact**: Custom text formatters, signal wiring between scalar fields, and the Cycle() update logic are all unexercised

### [GAP] ListBox: 5 variants vs C++ 7
- **C++ location**: emTkTestPanel.cpp:278-327
- Missing Rust:
  - `l6` "Single Column" — `SetFixedColumnCount(1)`
  - `l7` "Custom List Box" — `CustomListBox` with custom `CreateItemPanel`, recursive item panels
- **Impact**: Fixed column count and custom item panel interface untested

### [GAP] C++ TkTest has Cycle() engine logic — Rust has no equivalent
- **C++ location**: emTkTestPanel.cpp:371-437
- **C++ content**: Signal-driven update cycle:
  - SFLen → SFPos max value update
  - Dialog creation with configurable view/window flags
  - FileDialog open/save/multi-select lifecycle
- **Rust**: TkTestPanel has no equivalent Cycle/engine/signal machinery
- **Impact**: Dynamic widget interaction (signal wiring, dialog lifecycle) entirely untested

## Assessment

The Rust TkTest is a static snapshot exerciser — it creates widgets and paints them. The C++ TkTest is an interactive demo with signal-driven dynamic behavior. The gap is substantial for testing:

1. **Static rendering** — mostly covered (modulo missing categories)
2. **Dynamic interaction** — not covered at all (no Cycle, no signal wiring, no dialog lifecycle)
3. **Advanced configurations** — partially missing (NoEOI, custom formatters, fixed columns, custom list panels)
