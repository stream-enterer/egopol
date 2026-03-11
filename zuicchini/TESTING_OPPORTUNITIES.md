# Headless Testing Opportunities

Tiered by suitability for headless golden testing against C++ emCore.

## Tier 1 — High Value, Low Difficulty

### Layout rect goldens
- **C++ files:** `emLinearLayout.cpp`, `emRasterLayout.cpp`, `emPackLayout.cpp`
- **Type:** Numeric comparison (layout_rect values)
- **Why:** Pure geometry, deterministic, no pixel tolerance issues. Three complex layout engines with no golden verification beyond harness API tests.
- **Approach:** Create panel trees with known constraints, trigger layout, compare child rect positions/sizes between C++ and Rust.

### TestPanel full render — DONE
- **C++ files:** `emTestPanel.cpp` (in `src/emTest/`)
- **Type:** Pixel comparison
- **Tests:** `testpanel_root` (tol=3, max=15%, 13.78% actual diff at tol=0), `testpanel_expanded` (ignored, ~50% structural diff)
- **Root-only test** covers paint primitives, text, background — meaningful parity check.
- **Expanded test** has large structural diffs because the Rust `TkTestPanel` uses a flat 3-column grid instead of nested `RasterGroup` containers (which are implemented but not used in the test), and `PolyDrawPanel` is simplified (no control widgets). C++ also has types excluded from port scope (`emTunnel`, `emFileSelectionBox`) that contribute child panels.
- **Fix applied:** `view.set_window_focused(false)` to match C++ unfocused render state (was causing 24.5% → 13.78% diff).

### emRec serialization
- **C++ files:** `emRec.cpp`, `emRec.h`
- **Type:** Data parity (byte-level)
- **Why:** Pure data transform, no rendering. Deterministic read/write of emCore's record format.
- **Approach:** Serialize/deserialize identical structures in both, compare output bytes.

## Tier 2 — High Value, Medium Difficulty

### ColorField expanded layout — DONE
- **C++ files:** `emColorField.cpp`
- **Type:** Pixel comparison (800x800)
- **Test:** `colorfield_expanded` (ignored, 39.44% diff at tol=0, passes at tol=3/45%)
- **Gap:** Rust `ColorField::auto_expand()` creates in-memory `Expansion` data but does not create child panels (RasterLayout + 8 ScalarFields + TextField). C++ renders full right-half editing UI; Rust renders swatch-only.
- **Unblocks when:** Rust ColorField creates actual child panels in `auto_expand()` matching C++ `emColorField::AutoExpand()` structure.

### ListBox with item panels — DONE
- **C++ files:** `emListBox.cpp`
- **Type:** Pixel comparison (800x800)
- **Test:** `listbox_expanded` (ignored, 44.00% diff at tol=0, passes at tol=3/50%)
- **Gap:** C++ creates child `DefaultItemPanel` panels laid out by `emRasterGroup` grid (multi-column). Rust paints items inline as single-column rows in `paint()` — no child panels, different layout geometry.
- **Unblocks when:** Rust ListBox creates child panels matching C++ `emRasterGroup` item panel architecture, or rendering converges via inline painting improvements.

### Splitter drag + layout sequences — DONE
- **C++ files:** `emSplitter.cpp`
- **Type:** Numeric (interaction + layout)
- **Tests:** `splitter_layout_h`, `splitter_layout_v` (both pass, eps=1e-9)
- **Approach:** 4-step sequences (initial, two repositions, clamp test) comparing child panel rects against C++ golden. Uses `OBT_NONE/IBT_NONE` border to test pure layout math. Horizontal (1.0x0.75) and vertical (1.0x1.0) orientations tested.

### FileModel lifecycle
- **C++ files:** `emFileModel.cpp`
- **Type:** State sequence comparison
- **Why:** Harness covers API surface but no behavioral test drives the full load/ready/save/flush cycle.
- **Approach:** Drive state machine through transitions, compare state sequence and timing.

## Tier 3 — High Value, High Difficulty (Blocked)

### Border 9-slice composition
- **C++ files:** `emBorder.cpp` (800+ lines)
- **Type:** Pixel comparison
- **Why:** 4 Phase 6 golden tests fail here (button ~60%, radiobutton ~57%, colorfield ~33%, listbox ~31%). Root cause: C++ fixed-point vs Rust float interpolation precision in 9-slice image scaling.
- **Blocker:** Requires resolving fixed-point/float rounding strategy before tests can pass. High value once unblocked since borders appear in every widget.

## Tier 4 — Moderate Value

### Text rendering edge cases
- **C++ files:** `emFontCache.cpp`
- **Type:** Pixel comparison with tolerance
- **Why:** Phase 2 golden covers basic text. Edge cases (long strings, empty strings, special chars, alignment combos) untested.
- **Caveat:** Font rendering has inherent platform variance, needs wide tolerance.

## Not Applicable

### emHmiDemo components
- **C++ files:** `src/emHmiDemo/` (17+ files: Pump, Tank, Conveyor, Mixer, Station)
- **Why not:** Application-level demo panels, not framework. Outside zuicchini's scope.

### emTunnel, emFileSelectionBox
- **C++ files:** `emTunnel.h`, `emFileSelectionBox.h` (both emCore)
- **Why not:** Classified `not_applicable` by harness. `emTunnel` is an Eagle Mode-specific depth-zoom container; `emFileSelectionBox` is a file browser widget with no zuicchini use case. Both are emCore types but outside port scope.

## Existing Coverage Reference

| Area | Golden Phase | Tests | Status |
|------|-------------|-------|--------|
| Painter primitives | Phase 1-3 | 18 | All pass |
| TestPanel integration | — | 2 written | 1 pass, 1 ignored (structural diff) |
| Widget rendering | Phase 6 | 18 written | 12 pass, 6 ignored (Tier 3 blocker + expansion gaps) |
| Widget interaction | Phase 7 | 15 | All pass |
| Animator trajectories | Phase 8 | 10 | All pass |
| Input filters | Phase 9 | 8 | All pass |
| Focus navigation | Phase 4 | 41 | All pass |
| Window | Phase 5 | 3 | All pass |
| Harness API parity | — | 416 | All pass |
