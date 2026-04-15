# Cosmos Application Launch — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make eaglemode-rs launch a fully functional cosmos — faithful C++ port — with emfileman and emstocks plugins.

**Architecture:** Audit-driven, bottom-up. The code is already substantially ported. Fix divergences found in detailed C++/Rust comparison, wire control panel actions, verify golden tests stay at 239 pass / 4 fail throughout.

**Tech Stack:** Rust, emcore panel framework, emfileman/emstocks plugin crates, golden test harness.

**Spec:** `docs/superpowers/specs/2026-04-15-cosmos-app-launch-design.md`

**Baseline:** 239 golden tests pass, 4 fail (composition_tktest_1x, composition_tktest_2x, testpanel_expanded, testpanel_root). No new failures allowed at any point.

---

## Phase 1: Baseline & Config Fixes

### Task 1: Verify golden test baseline

**Files:** None modified

- [ ] **Step 1: Run golden tests**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

Expected: `test result: FAILED. 239 passed; 4 failed; 0 ignored`

- [ ] **Step 2: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1 | tail -10
```

Expected: No warnings.

- [ ] **Step 3: Run nextest**

```bash
cargo-nextest ntr 2>&1 | tail -10
```

Expected: All unit tests pass.

---

### Task 2: Fix Stocks1.emVcItem CopyToUser flag

The Stocks1.emVcItem has `CopyToUser = true` but the CopyToUser file-copy logic is not implemented in Rust. Since we ship the .emStocks file in the repo, set CopyToUser to false. Also create the missing Stocks1.emStocks content file.

**Files:**
- Modify: `etc/emMain/VcItems/Stocks1.emVcItem`
- Create: `etc/emMain/VcItemFiles/Stocks1.emStocks` (if missing)

- [ ] **Step 1: Check if Stocks1.emStocks exists in VcItemFiles**

```bash
ls -la etc/emMain/VcItemFiles/
```

- [ ] **Step 2: Fix Stocks1.emVcItem — set CopyToUser to false**

In `etc/emMain/VcItems/Stocks1.emVcItem`, change:
```
CopyToUser = true
```
to:
```
CopyToUser = false
```

- [ ] **Step 3: Create Stocks1.emStocks content file if missing**

Check the C++ original at `~/git/eaglemode-0.96.4/etc/emMain/VcItemFiles/` for the Stocks1.emStocks format. Create a minimal valid .emStocks file in `etc/emMain/VcItemFiles/Stocks1.emStocks`.

- [ ] **Step 4: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

Expected: 239 passed, 4 failed (unchanged).

- [ ] **Step 5: Commit**

```bash
git add etc/emMain/VcItems/Stocks1.emVcItem etc/emMain/VcItemFiles/
git commit -m "fix: set Stocks1 CopyToUser=false, add content file"
```

---

## Phase 2: Virtual Cosmos Fidelity Fixes

### Task 3: Fix emVirtualCosmosItemPanel::CalcBorders default

C++ returns (0.03, 0.05, 0.03, 0.03) when no item_rec (using t=1.0, bs=1.0 defaults). Rust returns (0,0,0,0).

**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

- [ ] **Step 1: Fix CalcBorders to match C++ default**

In `emVirtualCosmos.rs`, find `CalcBorders`:
```rust
pub fn CalcBorders(&self) -> (f64, f64, f64, f64) {
    let Some(rec) = &self.item_rec else {
        return (0.0, 0.0, 0.0, 0.0);
    };
```

Replace with:
```rust
pub fn CalcBorders(&self) -> (f64, f64, f64, f64) {
    let (t, bs) = match &self.item_rec {
        Some(rec) => (rec.ContentTallness, rec.BorderScaling),
        None => (1.0, 1.0),
    };
    let b = t.min(1.0) * bs;
    (b * 0.03, b * 0.05, b * 0.03, b * 0.03)
}
```

Remove the remaining lines that duplicate the calculation when item_rec is Some.

- [ ] **Step 2: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "fix: CalcBorders default matches C++ (t=1.0, bs=1.0)"
```

---

### Task 4: Fix emVirtualCosmosPanel panel naming

C++ uses `"_StarField"` for background and item names directly (no prefix). Rust uses `"background"` and `"item:"` prefix. Panel names affect bookmark identity resolution.

**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

- [ ] **Step 1: Fix background panel name**

Find the line creating the background panel (approximately):
```rust
let child_id = ctx.create_child_with("background", Box::new(bg));
```

Change to:
```rust
let child_id = ctx.create_child_with("_StarField", Box::new(bg));
```

- [ ] **Step 2: Remove "item:" prefix from item panel names**

Find `ITEM_PANEL_PREFIX`:
```rust
const ITEM_PANEL_PREFIX: &str = "item:";
```

And where it's used:
```rust
let child_name = format!("{}{}", ITEM_PANEL_PREFIX, rec.Name);
```

Change to use the name directly (matching C++):
```rust
let child_name = rec.Name.clone();
```

Remove the `ITEM_PANEL_PREFIX` constant.

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "fix: cosmos panel names match C++ (_StarField, no item: prefix)"
```

---

### Task 5: Fix emVirtualCosmosItemPanel::Paint to match C++

The Rust Paint uses simple PaintRect strips for borders. C++ uses a hollow polygon frame + PaintBorderImage decorations + PaintTextBoxed for title. This is the biggest visual divergence.

**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

- [ ] **Step 1: Read the C++ Paint method in detail**

Read `~/git/eaglemode-0.96.4/src/emMain/emVirtualCosmos.cpp` lines 361-443 to understand the exact rendering:

1. Background fill (PaintRect or Clear depending on border color opacity)
2. Hollow polygon border frame (10-point polygon for the border outline)
3. PaintBorderImage for outer border decoration
4. PaintBorderImage for inner border decoration
5. PaintTextBoxed for title text in top border

- [ ] **Step 2: Rewrite Paint to match C++ exactly**

Replace the current simplified Paint method with a faithful port that:
- Computes border dimensions identically to C++ CalcBorders
- Paints the background rect inside the border
- Paints the hollow border polygon (10 vertices: outer corners CW then inner corners CCW)
- Paints outer and inner border images using PaintBorderImage with the same source rects and mirror flags
- Paints the title using PaintTextBoxed with the same alignment and positioning

The C++ code computes `d` and `e` values for the border image insets. Port those formulas exactly.

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "fix: cosmos item Paint matches C++ (polygon border, border images, text)"
```

---

### Task 6: Fix emVirtualCosmosItemPanel::IsOpaque

C++ checks whether border color and background color are both opaque. Rust hardcodes false.

**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

- [ ] **Step 1: Read C++ IsOpaque**

Read `~/git/eaglemode-0.96.4/src/emMain/emVirtualCosmos.cpp` around line 358 for the IsOpaque implementation.

- [ ] **Step 2: Port IsOpaque to match C++**

Replace the hardcoded `false` with the C++ logic that checks border and background color alpha channels.

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "fix: cosmos item IsOpaque checks border/background alpha per C++"
```

---

### Task 7: Set background panel properties

C++ sets `SetFocusable(false)` and `SetAutoplayHandling(APH_CUTOFF)` on the starfield background panel. Rust doesn't.

**Files:**
- Modify: `crates/emmain/src/emVirtualCosmos.rs`

- [ ] **Step 1: Add property settings after background panel creation**

After the background panel is created in `LayoutChildren`, set:
```rust
ctx.set_focusable(child_id, false);
ctx.set_autoplay_handling(child_id, AutoplayHandling::Cutoff);
```

Check what API the panel tree provides for setting these properties. If `ctx` doesn't have these methods, check how other panels set focusable/autoplay handling and follow the same pattern.

- [ ] **Step 2: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
git add crates/emmain/src/emVirtualCosmos.rs
git commit -m "fix: starfield background non-focusable with APH_CUTOFF per C++"
```

---

## Phase 3: Main Panel Fixes

### Task 8: Fix StartupOverlayPanel::Paint text color

C++ uses `0x404040FF` (dark gray) for text. Rust uses `0xFFFFFFFF` (white).

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

- [ ] **Step 1: Fix text color in StartupOverlayPanel::Paint**

Find the PaintTextBoxed call in StartupOverlayPanel::Paint (around line 265):
```rust
emColor::from_packed(0xFFFFFFFF),
```

Change to:
```rust
emColor::from_packed(0x404040FF),
```

- [ ] **Step 2: Fix text height parameter**

C++ uses `ViewToPanelDeltaY(30.0)` for the text height. Check what this resolves to in the Rust framework and use the equivalent. If `ViewToPanelDeltaY` is not available, read the C++ implementation to understand what it computes (likely `30.0 / view_height * panel_height`) and replicate.

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "fix: startup overlay text color/size matches C++ (0x404040FF)"
```

---

### Task 9: Add config Save() calls in emMainPanel

C++ calls `MainConfig->Save()` in DragSlider and DoubleClickSlider. Rust omits these.

**Files:**
- Modify: `crates/emmain/src/emMainPanel.rs`

- [ ] **Step 1: Check how config saving works in Rust**

Read the emMainConfig implementation to find if there's a Save() method or equivalent. Check how other config models (emBookmarksModel, emAutoplayConfig) handle persistence.

- [ ] **Step 2: Add Save() calls**

In `DragSlider()`, after `SetControlViewSize()`:
```rust
self.config.borrow_mut().SetControlViewSize(self.unified_slider_pos);
// Add: save config to disk
self.config.borrow().Save();  // or whatever the Rust API is
```

In `DoubleClickSlider()`, after `SetControlViewSize(0.7)`:
```rust
self.config.borrow_mut().SetControlViewSize(0.7);
// Add: save config to disk
self.config.borrow().Save();
```

If Save() doesn't exist yet, check C++ `emConfigModel::Save()` and implement.

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainPanel.rs
git commit -m "fix: persist control view size on slider drag/double-click per C++"
```

---

## Phase 4: Control Panel — Make Functional

### Task 10: Audit and fix emMainControlPanel against C++

The Rust control panel is a "simplified flat panel with manual vertical layout" using stub `ControlButton` types. C++ creates a rich widget tree with `emLinearGroup`, signal connections, and functional buttons. This needs to become a faithful port.

**Files:**
- Modify: `crates/emmain/src/emMainControlPanel.rs`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emMainControlPanel.cpp`
- Reference: `~/git/eaglemode-0.96.4/include/emMain/emMainControlPanel.h`

- [ ] **Step 1: Read C++ emMainControlPanel constructor (lines 28-229)**

Understand the full widget tree:
1. "About Eagle Mode" group with icon + description text
2. Core config panel (emCoreConfigPanel)
3. "Main Commands" pack group with:
   - BtNewWindow (emButton)
   - BtFullscreen (emCheckButton)
   - BtAutoHideControlView (emCheckBox)
   - BtAutoHideSlider (emCheckBox)
   - BtReload (emButton)
   - BtAutoplay (emAutoplayControlPanel)
   - BtClose (emButton)
   - BtQuit (emButton)
4. emBookmarksPanel
5. Signal connections for all buttons

- [ ] **Step 2: Read C++ Cycle method (lines 247-293)**

Understand signal handling:
- Content view control panel signal → RecreateContentControlPanel
- Window flags signal → BtFullscreen check state
- Config change signal → AutoHide checkbox states
- Button click signals → MainWin.Duplicate/ToggleFullscreen/ReloadFiles/Close/Quit

- [ ] **Step 3: Rewrite emMainControlPanel to match C++ structure**

Replace the current stub implementation with a faithful port:
- Store references to all button panel IDs
- In LayoutChildren, create the proper widget hierarchy matching C++
- Create bookmarks panel via emBookmarksPanel::new
- Create autoplay control panel
- Use real emButton/emCheckButton types from emcore (or the closest Rust equivalents)

- [ ] **Step 4: Implement Cycle for button action handling**

Wire button clicks to actions:
- New Window: call window duplication (or log for now if not available)
- Fullscreen: call ToggleFullscreen on main window
- Reload Files: call ReloadFiles
- Close: call Close
- Quit: call Quit

Access the main window via `with_main_window()` from emMainWindow.

- [ ] **Step 5: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 6: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1 | tail -10
```

- [ ] **Step 7: Commit**

```bash
git add crates/emmain/src/emMainControlPanel.rs
git commit -m "feat: faithful port of emMainControlPanel with functional buttons"
```

---

## Phase 5: Audit Remaining emmain Files

### Task 11: Audit emMainWindow against C++

**Files:**
- Modify: `crates/emmain/src/emMainWindow.rs`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emMainWindow.cpp`

- [ ] **Step 1: Read both files and compare method-by-method**

Focus on:
1. `handle_input` / `Input` — keyboard shortcuts (F4, F5, F11, ESC/MENU)
2. `StartupEngine::Cycle` — state machine (states 0-11)
3. `Duplicate()` — does Rust have window duplication?
4. `GetTitle()` — title format
5. `ToggleControlView()` — does it work?
6. `RecreateContentPanels()` — does Rust have reload support?
7. `DoCustomCheat()` — cheat code handling

- [ ] **Step 2: Fix any divergences found**

Apply fixes to match C++ behavior. Common issues to look for:
- Missing keyboard shortcuts
- Incorrect startup state transitions
- Missing bookmark hotkey activation in Input handler

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMainWindow.rs
git commit -m "fix: emMainWindow audit fixes — match C++ input/startup/title"
```

---

### Task 12: Audit emBookmarks against C++

**Files:**
- Modify: `crates/emmain/src/emBookmarks.rs`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emBookmarks.cpp`
- Reference: `~/git/eaglemode-0.96.4/include/emMain/emBookmarks.h`

- [ ] **Step 1: Read both files and compare**

Focus on:
1. `emBookmarksRec` — field structure, from_rec/to_rec parsing
2. `emBookmarksModel::Acquire` — config file path, loading
3. `emBookmarksPanel` — does it render bookmark buttons?
4. `emBookmarkButton` — does clicking navigate?
5. `SearchBookmarkByHotkey` — hotkey resolution
6. `SearchStartLocation` — startup location
7. Default bookmark colors

- [ ] **Step 2: Fix any divergences found**

Common issues:
- Missing bookmark navigation on click (need to trigger view animation)
- Missing icon rendering
- Missing editing UI (InsertNewBookmark, etc.) — acceptable to skip for initial launch if editing is not required

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emBookmarks.rs
git commit -m "fix: emBookmarks audit fixes"
```

---

### Task 13: Audit emAutoplay against C++

**Files:**
- Modify: `crates/emmain/src/emAutoplay.rs`
- Modify: `crates/emmain/src/emAutoplayControlPanel.rs`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emAutoplay.cpp`

- [ ] **Step 1: Read both files and compare**

Focus on:
1. `emAutoplayViewAnimator` — CycleAnimation, LowPriCycle, AdvanceCurrentPanel
2. `emAutoplayViewModel` — Cycle, SetAutoplaying, Input
3. `emAutoplayControlPanel` — button creation, Cycle, signal handling

The autoplay system is complex. Verify the core state machine (AdvanceCurrentPanel) matches C++.

- [ ] **Step 2: Fix any divergences found**

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emAutoplay.rs crates/emmain/src/emAutoplayControlPanel.rs
git commit -m "fix: emAutoplay audit fixes"
```

---

### Task 14: Audit emStarFieldPanel against C++

**Files:**
- Modify: `crates/emmain/src/emStarFieldPanel.rs`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emStarFieldPanel.cpp`

- [ ] **Step 1: Read both files and compare**

Focus on:
1. Star generation — PRNG formula, star count, color generation
2. Paint — star rendering tiers (textured, ellipse, rect)
3. LayoutChildren — 4-quadrant subdivision
4. OverlayPanel — C++ has one, Rust may not
5. TicTacToePanel — C++ easter egg at depth > 50

- [ ] **Step 2: Fix any divergences found**

Key items:
- OverlayPanel (C++ creates it for input handling; verify if needed in Rust)
- Star shape image generation
- Star rendering order and method selection thresholds

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emStarFieldPanel.rs
git commit -m "fix: emStarFieldPanel audit fixes"
```

---

### Task 15: Audit emMain and emMainConfig against C++

**Files:**
- Modify: `crates/emmain/src/emMain.rs`
- Modify: `crates/emmain/src/emMainConfig.rs`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emMain.cpp`
- Reference: `~/git/eaglemode-0.96.4/src/emMain/emMainConfig.cpp`

- [ ] **Step 1: Read both files and compare**

Focus on:
1. `CalcServerName` — server name format
2. `try_ipc_client` — IPC protocol
3. `emMain::on_reception` — NewWindow/ReloadFiles handling
4. `emMainConfig` — fields match (AutoHideControlView, AutoHideSlider, ControlViewSize)
5. Config file path resolution

- [ ] **Step 2: Fix any divergences found**

- [ ] **Step 3: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add crates/emmain/src/emMain.rs crates/emmain/src/emMainConfig.rs
git commit -m "fix: emMain/emMainConfig audit fixes"
```

---

## Phase 6: Plugin Wiring Verification

### Task 16: Verify FpPlugin loading and panel creation

**Files:**
- Possibly modify: `crates/emcore/src/emFpPlugin.rs`
- Possibly modify: `crates/emcore/src/emInstallInfo.rs`
- Reference: `~/git/eaglemode-0.96.4/include/emCore/emFpPlugin.h`

- [ ] **Step 1: Trace the plugin loading path**

Verify that `emFpPluginList::Acquire()` can find and load plugins:
1. `emGetConfigDirOverloadable("emCore", Some("FpPlugins"))` resolves to `$EM_DIR/etc/emCore/FpPlugins/`
2. The 4 `.emFpPlugin` files are parsed correctly
3. Symbol resolution finds the `#[no_mangle]` functions in the correct libraries

- [ ] **Step 2: Verify emFileLinkFpPlugin creates correct panels**

The Home and Root cosmos items use `.emFileLink` files. The chain is:
1. emVirtualCosmosItemPanel creates a panel for `Home.emFileLink`
2. emFpPluginList matches `.emFileLink` → emFileLinkFpPluginFunc
3. emFileLinkPanel loads the `.emFileLink` file (Path="~", HaveDirEntry=yes)
4. emFileLinkPanel creates an emDirEntryPanel for the directory

Verify each step. Check that emFileLinkPanel can find the `.emFileLink` files in `$EM_DIR/etc/emMain/VcItemFiles/`.

- [ ] **Step 3: Verify emStocksFpPlugin creates correct panels**

The Stocks1 item uses `.emStocks` extension:
1. emFpPluginList matches `.emStocks` → emStocksFpPluginFunc
2. emStocksFilePanel is created

- [ ] **Step 4: Fix any issues found**

Common issues:
- Library path resolution (cdylib output may not be on LD_LIBRARY_PATH)
- Since emfileman and emstocks are compiled as cdylib+rlib, the plugin functions should be available via static linking if the binary depends on both crates. Verify this.
- Config path version mismatch could cause emGetConfigDirOverloadable to fail

- [ ] **Step 5: Run golden tests — verify no regression**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

```bash
git add -A  # only changed files
git commit -m "fix: plugin wiring verification and fixes"
```

---

## Phase 7: Final Verification

### Task 17: Full test suite verification

**Files:** None (verification only)

- [ ] **Step 1: Run full golden test suite**

```bash
cargo test --test golden -- --test-threads=1 2>&1 | tail -10
```

Expected: 239 passed, 4 failed (unchanged from baseline).

- [ ] **Step 2: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1 | tail -10
```

Expected: No warnings.

- [ ] **Step 3: Run nextest**

```bash
cargo-nextest ntr 2>&1 | tail -10
```

Expected: All unit tests pass.

- [ ] **Step 4: Verify binary builds**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: Clean build.

- [ ] **Step 5: Smoke test launch (if display available)**

```bash
EM_DIR=$(pwd) cargo run --release 2>&1 | head -20
```

If no display: verify the binary gets past argument parsing and into the event loop (will panic at XOpenDisplayFailed, which is expected without a display server).

---

### Task 18: Final audit sweep

**Files:** All emmain files

- [ ] **Step 1: Check for remaining TODO/FIXME/unimplemented! in emmain**

```bash
grep -rn 'TODO\|FIXME\|unimplemented!\|todo!' crates/emmain/src/
```

Review each hit. Fix or document any that affect cosmos functionality.

- [ ] **Step 2: Check for remaining DIVERGED comments that should have been fixed**

```bash
grep -rn 'DIVERGED' crates/emmain/src/
```

Each DIVERGED comment should be either:
- A documented, intentional Rust-specific adaptation, OR
- Fixed to match C++ by now

- [ ] **Step 3: Verify all 10 emmain files have C++ counterparts accounted for**

Cross-reference against the spec audit matrix. Every C++ method in emMain sources should have a Rust equivalent or a DIVERGED annotation.

- [ ] **Step 4: Final commit if any cleanup needed**

```bash
git add crates/emmain/src/
git commit -m "chore: final audit cleanup for cosmos launch"
```
