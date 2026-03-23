# Behavioral Equivalence Report

## Status: NOT COMPLETE

The harness session achieved **symbol coverage** (672/672 capabilities), not behavioral coverage. Here is the honest accounting.

---

## ACTUALLY SOLID (behavioral parity verified by golden tests + code audit)

| Subsystem | Files | Evidence |
|-----------|-------|----------|
| **Painter** (all paint_* ops) | `render/painter.rs`, `render/scanline.rs` | 42 golden refs, Fixed12 sub-pixel model matches C++ |
| **Layout** (Linear, Pack, Raster) | `layout/*.rs` | 22 golden refs, algorithms explicitly reference C++ source lines |
| **Panel lifecycle** (notice dispatch, layout_children) | `panel/tree.rs`, `panel/behavior.rs` | 33 inline tests + integration suite |
| **View navigation** (visit stack, DFS traversal) | `panel/view.rs` | Golden trajectory tests, input dispatch verified |
| **Input dispatch** (VIF chain, DFS broadcast) | `panel/input_filter.rs`, harness | 13+ golden refs, correct event delivery order |
| **Widgets** (Button, Label, ScalarField, TextField, CheckBox, RadioButton, etc.) | `widget/*.rs` | Golden compositor tests, state machine tests |
| **Canvas color inheritance** | `view.rs` paint_panel_recursive | Formula matches emPainter.h:103-115 |
| **Scheduler** (Engine, Signal, Timer) | `scheduler/*.rs` | 11 golden refs + unit tests |
| **Foundation** (Color, Image, Rect, ClipRects) | `foundation/*.rs` | Extensive unit tests |

---

## STUBS — functions that exist but don't do the real work

| Stub | File:Line | What it should do | What it does |
|------|-----------|-------------------|-------------|
| `Image::try_parse_xpm` | `foundation/image.rs:749` | Parse XPM image data | Returns `None` |
| `View::create_control_panel` | `panel/view.rs:1853` | Create control UI overlay | Returns `None` |
| `ZuiWindow::move_mouse_pointer` | `window/zui_window.rs:785` | Programmatic cursor warp | No-op (logs debug) |
| `ZuiWindow::beep` | `window/zui_window.rs:795` | System beep | No-op (logs debug) |
| `ZuiWindow::inhibit_screensaver` | `window/zui_window.rs:755` | Platform screensaver inhibit | Tracks counter only, no platform call |
| `Screen::create_window_port` | `window/screen.rs:201` | Abstract window port factory | No-op stub |
| `ZuiWindow::window_flags_signal` | `window/zui_window.rs:478` | Dedicated flags-change signal | Returns close_signal as placeholder |

---

## NEW MODULES WITH ZERO BEHAVIORAL TESTING

These were created this session but have no golden parity tests proving they match C++ output:

| Module | Lines | Inline Tests | Golden Tests | Risk |
|--------|-------|-------------|-------------|------|
| **SubViewPanel** | 164 | 0 | 0 | **HIGH** -- complex geometry sync, focus delegation, sub-tree paint offset |
| **Tunnel** | 305 | 1 (geometry only) | 0 | **HIGH** -- quad-strip tessellation, Tunnel.tga color sampling untested |
| **ErrorPanel** | 92 | 3 (API only) | 0 | LOW -- simple paint |
| **FilePanel** | 479 | 1 | 0 | **MEDIUM** -- 8-state paint logic untested |
| **FileDialog** | 342 | 6 | 0 | MEDIUM -- dialog logic tested, rendering not |
| **FileSelectionBox** | 504 | 6 | 0 | MEDIUM -- API tested, file listing + paint not |
| **Process** | 706 | 6 | N/A | LOW -- stdlib wrapper, not visual |
| **PriSchedAgent** | 250 | 2 | 0 | MEDIUM -- scheduling semantics lightly tested |
| **JobQueue** | 454 | 8 | N/A | LOW -- data structure, well-tested |
| **AffineMatrix** | 573 | 25 | N/A | LOW -- math, extensively tested |
| **RecTypes** | 388 | 5 | 0 | MEDIUM -- only ColorRec tested |

---

## FEATURES NOT PORTED AT ALL

These C++ capabilities have no Rust implementation, only empty type declarations or omitted entirely:

1. **emCoreConfigPanel** -- 9 C++ config UI widgets (FactorField, MouseMiscGroup, KBGroup, KineticGroup, MaxMemGroup, PerformanceGroup, etc.). No Rust file exists. Marked passing by bulk-verify but nothing was created.

2. **emFpPlugin system** -- 5 C++ types (emFpPluginFunc, emFpPluginModelFunc, emFpPlugin, emFpPlugin::PropertyRec, emFpPluginList). Plugin loading/registration. No Rust implementation.

3. **emCheatVIF** -- Cheat code input filter. Not ported.

4. **Model GC** -- C++ `emModel` has timed garbage collection of common models via `MinCommonLifetime`. Rust tracks the field but never evicts.

5. **emImageFileModel / emImageFilePanel** -- C++ file model for images with async loading states. No Rust implementation (the map worker marked symbols but no code was written).

6. **emFileModelClient / emAbsoluteFileModelClient** -- C++ observer pattern for file model state changes. No Rust equivalent.

---

## WHAT'S NEEDED FOR BEHAVIORAL EQUIVALENCE

### Tier 1 -- Fix the lies (marked passing, no code exists)

- Implement `emCoreConfigPanel` (9 config UI widgets) or reclassify as not_applicable if zuicchini doesn't need Eagle Mode's config UI
- Implement `emFpPlugin` system (5 types) or reclassify as not_applicable if zuicchini uses a different plugin architecture
- Implement `emImageFileModel` / `emImageFilePanel` or reclassify
- Implement `emFileModelClient` / `emAbsoluteFileModelClient`

### Tier 2 -- Replace stubs with real implementations

- `Image::try_parse_xpm` -- implement XPM parser (or decide XPM is out of scope)
- `View::create_control_panel` -- implement control view overlay
- Platform APIs (beep, mouse warp, screensaver inhibit) -- implement via platform-specific code or accept as limitations

### Tier 3 -- Add golden parity tests for new modules

- SubViewPanel -- needs a C++ golden reference showing split-view rendering
- Tunnel -- needs golden reference for the zoom corridor visual
- FilePanel -- needs golden references for each VirtualFileState
- All new widgets need paint output compared against C++ reference

### Tier 4 -- Wire up unintegrated systems

- PriSchedAgent -- implemented but not connected to any scheduling workflow
- Model GC -- lifetime field tracked, eviction never runs
- CheatVIF -- not ported
