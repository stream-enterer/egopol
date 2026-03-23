# Implement Control Panel Mechanism

## What this is

The control panel is an overlay widget that appears when a panel is "visited" (actively focused). In C++, it shows panel-specific controls — the TestPanel uses it to display a Background Color picker. The mechanism delegates upward through the panel tree: each panel's `create_control_panel` override can return a custom widget, or delegate to its parent.

## What already exists in Rust

Most of the infrastructure is ported. Read each before implementing:

| Component | File | Status |
|-----------|------|--------|
| `SubViewPanel` | `src/panel/sub_view_panel.rs` | Fully implemented |
| `PanelBehavior::create_control_panel()` | `src/panel/behavior.rs:197` | Trait method exists, default returns None |
| `PanelTree::create_control_panel()` delegation | `src/panel/tree.rs:1333-1354` | Walks parent chain to find override |
| `View::invalidate_control_panel()` | `src/panel/view.rs:1582-1585` | Sets `control_panel_invalid = true` |
| `View::is_control_panel_invalid()` | `src/panel/view.rs:1610-1612` | Reads flag |
| `View::clear_control_panel_invalid()` | `src/panel/view.rs:1615-1616` | Clears flag |
| `control_panel_invalid` field | `src/panel/view.rs:76` | Exists |
| `View::create_control_panel()` | `src/panel/view.rs:1882-1884` | **STUB — returns None** |

## What's missing

Three pieces, in implementation order:

### 1. View::create_control_panel() implementation (~10 lines)

The stub at view.rs:1882 must forward to the active panel's create_control_panel.

Read C++ at `emView.cpp:193-198`:
```cpp
emPanel * emView::CreateControlPanel(emPanel & parent, const emString & name) {
    if (!ActivePanel) return NULL;
    return ActivePanel->CreateControlPanel(parent, name);
}
```

In Rust: get the active panel from the view, call `tree.create_control_panel(active_panel_id, parent, name)` which already does the parent-chain walk-up.

### 2. Lifecycle management in the window/app layer (~40-60 lines)

Something must:
- Check `view.is_control_panel_invalid()` each frame (in the app's about_to_wait or event loop)
- When invalid: destroy the old control panel (if any), call `view.create_control_panel(parent, name)` to get a new one, store the result
- Call `view.clear_control_panel_invalid()` after recreation
- Position the control panel in the window (below the main viewport, or as an overlay)

Read C++ lifecycle at `emMainControlPanel.cpp:317-324` (RecreateContentControlPanel):
```cpp
void emMainControlPanel::RecreateContentControlPanel() {
    if (ContentControlPanel) delete ContentControlPanel;
    ContentControlPanel = ContentView.CreateControlPanel(*this, "context");
}
```

In Rust: the simplest approach is a field on ZuiWindow (or App) holding `Option<PanelId>` for the current control panel. On each frame, if `control_panel_invalid`, destroy the old panel and create a new one.

**Design decision:** C++ uses a two-view split (control SubViewPanel + content SubViewPanel) managed by emMainPanel. This is application-level architecture from emMain, not emCore. For the standalone test binary, a simpler approach works: create the control panel as a child of the root view's panel tree, positioned at the bottom of the viewport. Read the existing SubViewPanel to understand whether embedding a separate view is necessary or whether a simple child panel suffices.

### 3. TestPanel::create_control_panel override (~15-20 lines)

The TestPanel must override the trait method to create a ColorField for background color.

Read C++ at `emTestPanel.cpp:511-535`:
```cpp
emPanel * emTestPanel::CreateControlPanel(ParentArg parent, const emString & name) {
    if (BgColorField) {
        BgColorField = new emColorField(parent, name, "Background Color");
        BgColorField->SetColor(BgColor);
        // ... signal wiring
    }
    return BgColorField;
}
```

In Rust: override `create_control_panel` in the TestPanel's PanelBehavior impl. Create a ColorField, set its color to the current background, wire the color-change signal.

## Implementation steps

1. **Read all existing infrastructure** listed in the table above. Understand the delegation chain and invalidation mechanism.

2. **Implement View::create_control_panel** at view.rs:1882. Forward to active panel via the tree's delegation method. This is the minimal fix that makes the infrastructure functional.

3. **Implement lifecycle management.** Choose one approach:
   - **Simple:** In ZuiWindow or the standalone binary, check `control_panel_invalid` each frame, recreate as a positioned child panel.
   - **Full:** Create a control SubViewPanel in the window layout, embed a separate view for control panels. This matches C++ architecture but is more work.

   Start with the simple approach. Read how the existing standalone binary creates and positions its root panel — the control panel needs similar positioning logic.

4. **Implement TestPanel override.** In `examples/test_panel.rs`, override `create_control_panel` to create a ColorField with "Background Color" caption. Wire the color signal to update `BgColor` and trigger repaint.

5. **Verify:**
   - `cargo check --workspace`
   - `cargo test --workspace` — zero regressions
   - Run the standalone binary. Visit the TestPanel (click on it). The Background Color widget should appear.
   - Commit: `feat: implement control panel mechanism with TestPanel BgColor widget`

## C++ reference files

- `~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp:193-198` — CreateControlPanel
- `~/.local/git/eaglemode-0.96.4/src/emCore/emView.cpp:302-308` — SetActivePanel signals ControlPanelSignal
- `~/.local/git/eaglemode-0.96.4/src/emCore/emPanel.cpp:1244-1246` — Default delegation up parent chain
- `~/.local/git/eaglemode-0.96.4/src/emCore/emPanel.cpp:1323-1325` — InvalidateControlPanel
- `~/.local/git/eaglemode-0.96.4/src/emTest/emTestPanel.cpp:511-535` — TestPanel override creating BgColorField
- `~/.local/git/eaglemode-0.96.4/src/emMain/emMainControlPanel.cpp:317-324` — Lifecycle management
- `~/.local/git/eaglemode-0.96.4/src/emCore/emSubViewPanel.cpp` — SubViewPanel (already ported)

## Rules

- Do not rewrite SubViewPanel. It's already ported and working.
- Do not restructure the view or panel tree. Use the existing delegation chain.
- Start with the simple lifecycle approach (child panel, not SubViewPanel embed) unless you determine it's insufficient after reading the code.
- The control panel must appear when any panel is visited, not just TestPanel — the mechanism is generic, TestPanel is just one consumer.
- `cargo test --workspace` must pass before committing.
