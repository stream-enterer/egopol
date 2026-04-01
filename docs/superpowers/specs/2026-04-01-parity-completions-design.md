# Design: Eagle Mode Parity Completions

**Date:** 2026-04-01
**Goal:** Complete all deferred work from the main app launch that affects main app fidelity, infrastructure, and test coverage — reaching full parity with C++ emMain behavior.

## Scope

12 items from `docs/superpowers/gaps/2026-03-31-deferred-from-main-app.md`, organized into 5 dependency-ordered phases. Each phase has a concrete verification gate.

**In scope:** Items 3, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17a.
**Out of scope:** Viewer plugins (1), app plugins (2), audio/video (4), platform ports (5), cosmos items for unported apps (6), emTreeDump/emOsm (7), TicTacToe Easter egg, nested bookmark groups (17b — already working).

## Phase 1 — emSubViewPanel Integration + Slider Drag (Items 16, 13)

### Problem

C++ `emMainPanel` creates `emSubViewPanel* ControlViewPanel` and `emSubViewPanel* ContentViewPanel` (emMainPanel.h:106-107). Each sub-view has independent zoom/pan navigation. The Rust `emMainPanel` creates `emMainControlPanel` and `emMainContentPanel` as direct children (emMainPanel.rs:187-198), meaning they zoom with the parent.

The C++ `SliderPanel` handles mouse press/drag/release to resize the control/content split (emMainPanel.cpp:405-452). The Rust `SliderPanel` is a static color rectangle with no interaction (emMainPanel.rs:25-42).

### Changes

**emMainPanel (crates/emmain/src/emMainPanel.rs):**

- `LayoutChildren` creates two `emSubViewPanel` instances as children, then creates `emMainControlPanel` inside the control sub-view and `emMainContentPanel` inside the content sub-view.
- View flags configured per C++: `VF_POPUP_ZOOM | VF_ROOT_SAME_TALLNESS | VF_NO_ACTIVE_HIGHLIGHT` for control view, `VF_ROOT_SAME_TALLNESS` for content view.
- Content view panel activated on creation (C++ line 65).
- `GetControlView()` / `GetContentView()` accessors returning `&emView` exposed.
- EOI signal from control view wired: zoom-out control view + activate content view (C++ lines 116-119).
- `ControlEdgesImage` loaded from `ControlEdges.tga` via `emGetInsResImage`. Painted via `PaintBorderImage` (C++ lines 196-220).
- `UpdateCoordinates` ported exactly from C++ (lines 234-293) including all branches: `SliderY < t` special case, `ControlH < 1E-5` collapse case, `ControlX < 1E-5` width-limited case, and `Slider->Pressed` guard on slider position correction.
- `UpdateFullscreen` ported: auto-hide control view on fullscreen enter/exit (C++ lines 296-319).
- `UpdateSliderHiding` ported: 5-second `SliderTimer` auto-hide in fullscreen when slider at top (C++ lines 322-339).
- Mouse movement detection in `Input` for slider show/hide (C++ lines 143-158).
- `StartupOverlayPanel` becomes a real panel struct (not a bool flag): `IsOpaque() -> false` (critical for sub-view sizing), `Paint` showing "Loading..." text, `Input` eats all events, `GetCursor` returns wait cursor (C++ lines 505-565).

**SliderPanel (same file):**

- Fields: `MouseOver`, `Pressed`, `Hidden`, `PressMY`, `PressSliderY`, `SliderImage`.
- `SliderImage` loaded from `Slider.tga` via `emGetInsResImage`.
- `Input` handler: mouse-over detection (C++ line 412), left-button press/drag/release, double-click toggle via `DoubleClickSlider`, shift-key 4× sensitivity reduction (C++ line 442).
- `DragSlider` method: clamp slider position, update coordinates, save to config (C++ lines 342-357).
- `DoubleClickSlider`: toggle between 0 and saved position, default to 0.7 (C++ lines 360-374).
- `SetHidden`: controls visibility (C++ lines 396-402).
- `Paint`: `PaintRoundRect` background with state-dependent colors (pressed/hover/normal), polygon arrow indicators, `PaintImage` for slider texture (C++ lines 462-502).
- `SetFocusable(false)` (C++ line 386).

### Gate

Control and content panels have independent zoom/pan. Slider drag resizes the split. Double-click toggles control visibility. Auto-hide works in fullscreen. Shift-drag reduces sensitivity. Existing tests pass.

## Phase 2 — Independent Completions (Items 9, 10, 12, 15, 17a)

All items in this phase are independent of each other.

### Item 9 — Eagle Logo Polygons

**Problem:** Rust `emMainContentPanel::Paint` (line 104-119) draws "Eagle Mode" text placeholder. C++ `PaintEagle` (emMainContentPanel.cpp:132-349) renders 14 polygons with hundreds of coordinate pairs.

**Changes (crates/emmain/src/emMainContentPanel.rs):**

- Copy all 14 polygon arrays (`poly0` through `poly13`) and `polyColors` from C++ (lines 134-345).
- `PaintEagle` method: create a transformed painter with eagle coordinate origin/scale (C++ lines 88-100: shifted origin by `EagleShiftX/Y`, scaled by `EagleScaleX/Y`), then loop `PaintPolygon` for each polygon (C++ lines 346-348).
- Remove text placeholder and DIVERGED comment.

### Item 10 — Star.tga Textured Star Rendering

**Problem:** Rust `emStarFieldPanel::Paint` uses `PaintEllipse` for all star sizes (line 182-188). C++ `PaintOverlay` (emStarFieldPanel.cpp:102-147) has 3-tier rendering with texture.

**Changes (crates/emmain/src/emStarFieldPanel.rs):**

- Load `Star.tga` via `emGetInsResImage(ctx, "emMain", "Star.tga", 1)` in constructor. Store as `StarShape: emImage`.
- Move star rendering from `Paint` to `PaintOverlay` (stars render after children, matching C++).
- 3-tier rendering logic (C++ lines 111-146):
  - `vr > 4.0`: Extract star's hue and saturation. Compute glow alpha = `sat * 18.0` clamped to 255. First pass: `PaintImageColored` with `StarShape`, hue at full saturation, computed alpha. Second pass: `PaintImageColored` with `StarShape`, `sat - 10.0` saturation, full opacity.
  - `vr > 1.2` (after scaling `r *= 0.6`): `PaintEllipse` with star color.
  - `vr <= 1.2` (after further scaling `r *= 0.8862`): `PaintRect` with star color.
- Remove DIVERGED comment.

### Item 12 — Detached Control Window

**Problem:** Gap doc implies this is default C++ behavior. Verified: it's a cheat-code feature (`"ccw"`) triggered via `DoCustomCheat` (emMainWindow.cpp:271).

**Changes (crates/emmain/src/emMainWindow.rs):**

- Add `CreateControlWindow` method (C++ lines 309-327): creates a new `emWindow` with `VF_POPUP_ZOOM | VF_ROOT_SAME_TALLNESS` and `WF_AUTO_DELETE`, creates `emMainControlPanel` as its root.
- Wire `DoCustomCheat("ccw")` to call `CreateControlWindow`.
- Store `ControlWindow` as `Option<ZuiWindow>` on the window struct, raise existing window if already open.

### Item 15 — IPC Single-Instance

**Problem:** `emMain::try_ipc_client` always returns false (emMain.rs:33). `emMiniIpcClient::TrySend` and `emMiniIpcServer` are fully implemented in emcore (emMiniIpc.rs:223-475) but never called.

**Changes:**

**crates/emmain/src/emMain.rs:**

- `try_ipc_client`: call `emMiniIpcClient::TrySend(server_name, args)` instead of returning false. On success (server responded), return true. On error (no server), return false.
- `emMain` struct: port as an engine (C++ emMain is an `emEngine`) that:
  - Creates `emMiniIpcServer` with server name from `CalcServerName()`.
  - Polls server in `Cycle()` for incoming messages.
  - On message: parse args, call `NewWindow()` with geometry/fullscreen/visit overrides.
  - Owns `emSigModel` for reload signaling.

**crates/eaglemode/src/main.rs:**

- Wire: compute server name → try IPC client → if success, exit → else start server engine + GUI framework.

### Item 17a — emRec Color Round-Trip Fidelity

**Problem:** `emColorRec` serializes as `{R G B A}` int sub-struct. May not match C++ format exactly.

**Changes:**

- Load actual C++ `.emVcItem` and `.emBookmarks` files from `~/git/eaglemode-0.96.4/etc/emMain/`.
- Parse with Rust `emColorRec::FromRecStruct`, compare against expected values.
- If divergent: fix `FromRecStruct` / `ToRecStruct` to match C++ wire format.
- Add regression tests with known C++ color values.

### Gate

Eagle logo renders as 14 polygons. Stars use 3-tier rendering with TGA glow. Second `cargo run` sends IPC to first instance. Color round-trip matches C++. All existing tests pass.

## Phase 3 — Startup Animation + Autoplay (Items 11, 14)

### Item 11 — Startup Animation

**Problem:** Rust `create_main_window` (emMainWindow.rs:40-70) creates window and panels directly. C++ `StartupEngineClass::Cycle()` (emMainWindow.cpp:362-485) is a 12-state machine that stages panel creation across frames and runs a choreographed zoom.

**Changes (crates/emmain/src/emMainWindow.rs):**

- `StartupEngineClass` struct with state enum and `Cycle` method:
  - States 0-2: Idle wake-ups (yield to scheduler).
  - State 3: Create `emMainPanel` with `controlTallness = 0.0538`, set startup overlay, acquire `emAutoplayViewModel` from content view.
  - State 4: Acquire `emBookmarksModel`, search for start location bookmark. If `-visit` arg provided, use that instead.
  - State 5: Create `emMainControlPanel` in control sub-view.
  - State 6: Create `emMainContentPanel` in content sub-view.
  - State 7: Create `emVisitingViewAnimator`, `SetGoalFullsized(":", false)`, activate, start clock.
  - State 8: Wait up to 2 seconds for zoom animation.
  - State 9: If visit target exists, set new goal and activate.
  - State 10: Wait up to 2 seconds, then `RawZoomOut`, set active panel, remove startup overlay.
  - State 11+: 100ms pause, then final `Visit()` call, delete engine.

- Refactor: panel creation moves from `emMainPanel::LayoutChildren` into the startup engine (states 3-6). `emMainPanel` still owns the layout coordinates and child positioning, but child instantiation is driven by the engine.

- `emMainWindow` struct gains: `WindowStateSaver`, `BookmarksModel`, `AutoplayViewModel`, `StartupEngine`, `ControlPanel`, `ContentPanel`, `ToClose` flag.

- Window lifecycle methods ported: `Duplicate`, `ToggleFullscreen`, `ReloadFiles`, `ToggleControlView`, `Close`, `Quit` (C++ lines 98-171).

- Input handler: F4 (new/close/quit), F5 (reload), F11 (fullscreen), Escape (toggle control), bookmark hotkeys (C++ lines 193-263).

### Item 14 — Autoplay Panel Traversal

**Problem:** Five `emAutoplayViewAnimator` methods are stubbed (emAutoplay.rs:326-370). The panel tree API they need (`GetFirstChild`, `GetNext`, `GetIdentity`) exists in `emPanelTree.rs` (lines 537-573).

**Changes (crates/emmain/src/emAutoplay.rs):**

**emAutoplayViewAnimator:**

- Port traversal logic from C++ `emAutoplayViewAnimator` (emAutoplay.cpp). The animator extends `emViewAnimator` and owns an `emVisitingViewAnimator`.
- `AdvanceCurrentPanel`: state machine that walks the panel tree — `GoParent`, `GoChild`, `GoSame`, `InvertDirection`.
- `IsItem(panel)`: check if a panel is an autoplay item (has `APH_ITEM` handling).
- `IsCutoff(panel)`: check if recursion should stop (panel has `APH_CUTOFF` handling or depth limit).
- `CycleAnimation(dt)`: drives the `emVisitingViewAnimator` to the current goal, advances when goal reached.
- `LowPriEngineClass`: low-priority engine for background traversal work.
- Un-stub all five methods: `SetGoalToItemAt`, `SetGoalToPreviousItemOf`, `SetGoalToNextItemOf`, `SkipToPreviousItem`, `SkipToNextItem`.

**emAutoplayViewModel:**

- Port from plain data struct to full model with `Cycle()` method (C++ emAutoplay.cpp):
  - Drives `emAutoplayViewAnimator` based on UI state.
  - Item playing timer: tracks elapsed time per item, computes `ItemProgress`.
  - `UpdateFullsized`: ensures current panel is fullsized during playback.
  - `SaveLocation`: persists last location to config.
  - `SetScreensaverInhibited`: inhibits screensaver during playback.
  - `Input` handler: space (play/pause), left/right (skip), hotkeys.
- `Acquire(view)`: register as model in view's context.
- `SetConfigFilePath`: load/link config.
- `CanContinueLastAutoplay` / `ContinueLastAutoplay`: resume from saved location.

**emAutoplayControlPanel (new):**

- Port C++ `emAutoplayControlPanel` (emAutoplay.h:333-398). An `emPackGroup` containing:
  - `AutoplayButton`: check-button with progress arc overlay.
  - Prev/Next buttons.
  - "Continue Last" button.
  - Duration scalar field with logarithmic scale.
  - Recursive / Loop checkboxes.
- Wired to `emAutoplayViewModel`.

### Gate

App boots with choreographed 2-phase zoom animation (~2 seconds). Startup overlay shows "Loading..." then fades. Autoplay traverses cosmos items with play/pause/skip controls. `emAutoplayControlPanel` renders in sidebar.

## Phase 4 — Dynamic Plugin Loading (Item 3)

### Problem

`emFpPlugin::TryCreateFilePanel` checks a static resolver first (emFpPlugin.rs:245-247), so dynamic loading via `emTryResolveSymbol` is never exercised. The static registry in `static_plugins.rs` hardcodes four plugin functions.

The dynamic loading infrastructure is fully implemented: `emTryResolveSymbol` (emStd2.rs:358-373) does `dlopen`/`dlsym` via `libloading`. Plugin crates are already `cdylib + rlib`.

### Changes

**Remove static resolver:**

- Delete `crates/emmain/src/static_plugins.rs`.
- Remove `set_static_plugin_resolver` call from `crates/eaglemode/src/main.rs`.
- Remove `pub mod static_plugins` from `crates/emmain/src/lib.rs`.
- Remove `set_static_plugin_resolver` function from `crates/emcore/src/emFpPlugin.rs` and the `STATIC_RESOLVER` thread-local.

**Library search path:**

- `emTryOpenLib("emFileMan", false)` must find `libemfileman.so`. Options:
  - Set `RPATH` in the `eaglemode` binary via `build.rs` to point to the directory containing plugin `.so` files (e.g., `$ORIGIN/../lib/` or the cargo output directory).
  - Alternative: launcher script that sets `LD_LIBRARY_PATH`.
- Decision: use `RPATH` via `build.rs` for development (`$ORIGIN` relative) and document `LD_LIBRARY_PATH` for custom installs.

**Library name mapping:**

- `.emFpPlugin` config files say `Library = "emFileMan"`. Cargo produces `libemfileman.so` (lowercase, `lib` prefix).
- `emTryOpenLib` already handles the `lib` prefix and `.so` suffix (standard `libloading` behavior).
- Fix: update `.emFpPlugin` config files to use Cargo's actual output names (lowercase crate names: `emfileman`, `emstocks`). Config files should match actual artifact names. `emTryOpenLib` prepends `lib` and appends `.so` per platform convention.

**Verification:**

- The existing `test_plugin` crate (`crates/test_plugin/`) is a `cdylib` with `#[no_mangle]` entry points. Extend the existing plugin integration tests (`tests/integration/plugin_e2e.rs`, `tests/behavioral/fp_plugin.rs`) to verify end-to-end dynamic loading without the static resolver.

### Gate

All plugins load via `dlopen`/`dlsym` at runtime. No static resolver in the codebase. `cargo run` works with plugins as separate `.so` files. Plugin integration tests pass.

## Phase 5 — Golden Tests for emMain (Item 8)

### Problem

Golden test suite covers emCore rendering only. No tests for emMain panels.

### Changes

**C++ generator (tests/golden/gen/):**

- Extend `gen_golden.cpp` (or add `gen_golden_main.cpp`) to render emMain panels at known viewport sizes and dump pixel data. Requires linking against `emMain` headers.
- Generate reference data for: starfield at known seed/depth/viewport, eagle logo at known viewport, main panel split layout coordinates, cosmos item borders, bookmark buttons.

**Rust golden test modules (tests/golden/):**

- `starfield.rs`: Deterministic star positions/colors at known seeds and depths. Pixel output at known viewport sizes compared against C++ reference.
- `eagle_logo.rs`: Polygon rendering at known viewport size.
- `main_panel.rs`: Split layout geometry (control/content/slider coordinates) at known panel heights and slider positions. Rect comparison with f64 epsilon.
- `cosmos_items.rs`: Item panel border and title rendering.
- `control_panel.rs`: Sidebar button layout geometry.

**Comparison strategy:**

Uses existing comparison functions from `common.rs`: pixel (`ch_tol` + `max_fail_pct`), rect (f64 eps).

### Gate

Golden tests pass for all emMain panels. Divergence log clean at tolerance 0.

## Dependencies

```
Phase 1 (SubViewPanel + Slider)
  └── gates Phase 3 (Startup uses sub-views, autoplay needs stable panel tree)

Phase 2 (Eagle, Stars, IPC, Detached, Color)
  └── independent of Phase 1; gates Phase 5 (rendering must be final)

Phase 3 (Startup + Autoplay)
  └── depends on Phase 1; gates Phase 5

Phase 4 (Dynamic Loading)
  └── independent of Phases 1-3; gates Phase 5

Phase 5 (Golden Tests)
  └── depends on Phases 1-4 (all rendering/behavior finalized)
```

## Testing Strategy

### Per-phase unit tests

- Phase 1: `UpdateCoordinates` exact values at known heights/slider positions. Slider drag clamping. Double-click toggle logic. Auto-hide timer.
- Phase 2: Polygon coordinate data integrity (spot-check known vertices). Star tier thresholds. IPC round-trip. Color round-trip with C++ reference values.
- Phase 3: Startup engine state transitions. Autoplay state machine transitions. Panel traversal ordering.
- Phase 4: Dynamic symbol resolution for each plugin. Library name resolution. Error handling for missing libraries.

### Integration tests

- Phase 1: Panel tree structure with sub-views. Independent zoom verification.
- Phase 3: Full startup sequence in headless test harness.
- Phase 4: End-to-end plugin loading from `.emFpPlugin` config through `dlopen` to panel creation.

### Manual smoke tests

- After Phase 1: Drag slider, verify independent zoom.
- After Phase 3: `cargo run` shows choreographed startup. Autoplay cycles through cosmos items.
- After Phase 4: `cargo run` with no static resolver, verify all panels load.

## Files Modified

### Phase 1
- `crates/emmain/src/emMainPanel.rs` — major rewrite
- `crates/emcore/src/emSubViewPanel.rs` — no changes expected (already complete)

### Phase 2
- `crates/emmain/src/emMainContentPanel.rs` — add polygon data and PaintEagle
- `crates/emmain/src/emStarFieldPanel.rs` — 3-tier rendering, PaintOverlay, TGA loading
- `crates/emmain/src/emMainWindow.rs` — add CreateControlWindow, DoCustomCheat
- `crates/emmain/src/emMain.rs` — wire IPC client/server
- `crates/eaglemode/src/main.rs` — wire IPC flow
- `crates/emcore/src/emRecRecTypes.rs` — fix if color round-trip diverges

### Phase 3
- `crates/emmain/src/emMainWindow.rs` — StartupEngineClass, window lifecycle, input
- `crates/emmain/src/emAutoplay.rs` — traversal logic, view model cycle, control panel
- `crates/emmain/src/emMainPanel.rs` — panel creation refactored for engine-driven staging
- `crates/emmain/src/lib.rs` — re-export new types

### Phase 4
- `crates/emmain/src/static_plugins.rs` — deleted
- `crates/emmain/src/lib.rs` — remove static_plugins module
- `crates/eaglemode/src/main.rs` — remove set_static_plugin_resolver
- `crates/emcore/src/emFpPlugin.rs` — remove STATIC_RESOLVER
- `etc/emCore/FpPlugins/*.emFpPlugin` — update Library names to match Cargo output
- `crates/eaglemode/build.rs` — add RPATH configuration

### Phase 5
- `tests/golden/gen/` — extend C++ generator for emMain panels
- `tests/golden/starfield.rs` — new
- `tests/golden/eagle_logo.rs` — new
- `tests/golden/main_panel.rs` — new
- `tests/golden/cosmos_items.rs` — new
- `tests/golden/control_panel.rs` — new
- `tests/golden/main.rs` — register new test modules
