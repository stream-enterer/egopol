# emtest Plugin Design

**Date:** 2026-04-30
**Status:** Approved

## Goal

Port C++ `emTest/emTestPanel` into the eaglemode-rs virtualcosmos as a proper plugin crate (`crates/emtest`), replacing the partial standalone binary at `examples/test_panel.rs`. The result must be behaviorally indistinguishable from the C++ `emTestPanel` plugin as seen from the virtualcosmos — same panel hierarchy, same widget inventory, same BgColor persistence, same image.

## Section 1: Crate structure

New workspace member `crates/emtest/`:

```
crates/emtest/
├── Cargo.toml
└── src/
    ├── lib.rs          # plugin entry point + mod declarations
    └── emTestPanel.rs  # TestPanel, TkTestGrp, TkTest, PolyDrawPanel, CustomListBox
```

`Cargo.toml`:
```toml
[package]
name = "emtest"
version = "0.1.0"
edition = "2021"

[lib]
name = "emTestPanel"
crate-type = ["cdylib", "rlib"]

[lints]
workspace = true

[dependencies]
emcore = { path = "../emcore" }
```

`lib.rs` exports one symbol: `#[no_mangle] pub fn emTestPanelFpPluginFunc(...)`. It checks that the plugin has no properties (returns error string if any present, matching C++ behavior), then constructs and returns `TestPanel::new(0, bg_color)`.

`Cargo.toml` workspace `members` gains `"crates/emtest"`. `examples/test_panel.rs` is deleted.

## Section 2: Infrastructure additions to emcore

### `emRes.rs` — `emGetInsResImage`

Free function: `pub fn emGetInsResImage(res_subdir: &str, name: &str) -> emImage`

Resolves path: walks up from `std::env::current_exe()` until it finds a `res/` directory, then loads `res/<res_subdir>/<name>` as a TGA via the existing `emResTga` loader. Returns a blank 1×1 RGBA image on any error (matches C++ graceful degradation).

`teddy.tga` is copied from `~/Projects/eaglemode-0.96.4/res/icons/teddy.tga` into `res/emTest/icons/teddy.tga` in the Rust workspace.

### `emVarModel.rs` — `GetAndRemove` / `Set`

Two free functions operating on a per-view `HashMap<String, emColor>` stored in `AppState`:

- `pub fn GetAndRemove(ctx: &mut dyn ConstructCtx, key: &str, default: emColor) -> emColor`
- `pub fn Set(ctx: &mut dyn ConstructCtx, key: &str, value: emColor)`

Scoped to `emColor` (the only use case in emTestPanel). The store key convention matches C++: `"emTestPanel - BgColor of " + identity`.

`TestPanel` saves BgColor on `Drop` (only if changed from default, matching C++).

## Section 3: emTestPanel port

### TestPanel

Top-level panel (depth 0 at root). Differences from `examples/test_panel.rs`:

- BgColor initialized via `GetAndRemove(ctx, key, 0x001C38FF)` in constructor
- BgColor saved via `Set` in `Drop` impl, only when changed from default
- `test_image` loaded via `emGetInsResImage("emTest/icons", "teddy.tga")`
- `CreateControlPanel` unchanged (emLabel with identity + BgColor)
- `SetAutoExpansionThreshold(900.0)` on self and on each recursive `tp1`–`tp4` child

### TkTestGrp

Unchanged from example — 2×2 grid of `TkTest` instances, fourth one disabled.

### TkTest

Full C++ widget inventory:

**Buttons group** (3 items):
- b1: `emButton("Button")`
- b2: `emButton("Long Desc")` + `SetDescription(100-line string)` 
- b3: `emButton("NoEOI")` + `SetNoEOI(true)`

**Check widgets group** (6 items): 3× `emCheckButton` + 3× `emCheckBox`

**Radio widgets group** (6 items): plain `emRasterGroup` parent holding `RadioGroup`; 3× `emRadioButton` + 3× `emRadioBox`

**Text fields group** (4 items): same as example, add `SetDescription` strings on each

**Scalar fields group** (6 items):
- sf1: read-only, default range
- sf2: editable, default range
- sf3: editable, range −1000..1000, scale marks
- sf4: editable, range 1..5, custom `SetTextOfValueFunc` (Level: "Low"/"Medium"/"High" etc.)
- sf5 (PlayLength): editable, 0..24h in ms, custom time formatter, scale marks
- sf6 (PlayPos): editable, 0..sf5.value, same time formatter — max updated when sf5 changes via signal

**Color fields group** (3 items): same as example

**Tunnels group** (4 items):
- t1: `emTunnel("Tunnel")` → `emButton` end
- t2: `emTunnel("Deeper Tunnel")`, depth=30 → `emRasterGroup` end
- t3: `emTunnel("Square End")`, child_tallness=1.0 → `emRasterGroup` end
- t4: `emTunnel("Square End, Zero Depth")`, child_tallness=1.0, depth=0 → `emRasterGroup` end

**List boxes group** (7 items):
- l1–l5: same as example (Empty, Single, ReadOnly, Multi, Toggle)
- l6: Single, `SetFixedColumnCount(1)`
- l7: `CustomListBox` (multi-selection, custom item rendering via `item_panel_factory`)

**Dialogs group**: checkboxes for `TopLevel`, `VF_POPUP_ZOOM` (checked), `WF_MODAL` (checked), `WF_UNDECORATED`, `WF_POPUP`, `WF_MAXIMIZED`, `WF_FULLSCREEN`; "Create Test Dialog" button spawns `emDialog` containing a recursive `TkTest`; signal wired in `Cycle`

**File choosers group**: `emFileSelectionBox` + 3 buttons (Open, Open Multi+AllowDir, Save As) that each create an `emFileDialog`; result shown via `emDialog::ShowMessage`; signal wired in `Cycle`. Note: `emDialog::ShowMessage` is currently `unimplemented!()` in emcore — implementing it is a required step of this work.

### PolyDrawPanel

Restructured from flat panel to `emLinearGroup` + `CanvasPanel` child (matching C++ `emLinearGroup` + inner `CanvasPanel`). Outer `emLinearGroup` provides the panel frame. Inner `CanvasPanel` holds vertex state, drag logic, and all painting.

### CustomListBox / CustomItemPanel

Implemented via `emListBox`'s `item_panel_factory`. The factory returns a `CustomItemPanel` that draws a colored rectangle + item text, and updates on `ItemSelectionChanged`.

## Section 4: Plugin wiring & cleanup

**`etc/emCore/FpPlugins/emTestPanel.emFpPlugin`**:
```
#%rec:emFpPlugin%#

FileTypes = { ".emTestPanel" }
FileFormatName = "emTestPanel"
Priority = 1.0
Library = "emTestPanel"
Function = "emTestPanelFpPluginFunc"
```

**`etc/emMain/VcItemFiles/TestPanel.emTestPanel`**: empty file (extension triggers the plugin; path passed to the panel constructor).

**Cleanup**: `examples/test_panel.rs` deleted; workspace `members` updated.
