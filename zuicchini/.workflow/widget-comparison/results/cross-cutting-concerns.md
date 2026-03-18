# Cross-Cutting Concerns

Issues that span multiple widgets. Track here to ensure all affected widgets are checked.

## CC-01: Code Duplication Across Button-Family Widgets

**Found in**: CheckBox audit (2026-03-18)
**Affected widgets**: Button, CheckButton, CheckBox, RadioButton, RadioBox
**Issue**: Rust has no shared base widget. Each independently implements:
- Input handling (mouse down/up/move)
- Hit testing (rounded-rect signed-distance)
- Toggle logic (for check/radio variants)
- Paint pipeline

When one is fixed, the others may not be. Check that fixes applied to one are reflected in all.

**Specific divergence found**:
- Button handles Enter+Space for keyboard activation
- CheckBox/CheckButton only handle Space
- RadioButton/RadioBox — TBD (not yet audited)

## CC-02: Missing `set_*` Signal Firing — **PARTIALLY FIXED**

**Found in**: CheckBox audit (2026-03-18)
**Affected widgets**: All widgets with setter methods
**Issue**: C++ `SetChecked()` fires `CheckSignal` and calls `CheckChanged()`. Rust `set_checked()` just sets the bool silently. This means programmatic state changes don't fire callbacks.
**Fixed for**: CheckBox, CheckButton — set_checked now fires on_check callback when state changes.
**Remaining**: `set_text()`, `set_value()`, `set_color()`, `set_selected()` — not yet checked.

## CC-03: Missing Disabled State Rendering

**Found in**: CheckBox audit (2026-03-18)
**Affected widgets**: Potentially all
**Issue**: C++ paints disabled overlays (gray translucent rect) and makes label colors transparent. Rust may not have this across the board.

## CC-05: DoLabel Alignment Defaults Wrong (Center vs Left) — **PARTIALLY FIXED**

**Found in**: Label audit (2026-03-18)
**Affected widgets**: ALL border-based widgets (every widget that has a caption/label)
**Issue**: C++ `LabelAlignment` defaults to `EM_ALIGN_LEFT`. Rust always centers the label block. C++ `CaptionAlignment` defaults to `EM_ALIGN_LEFT` for text line alignment. Rust hardcodes `TextAlignment::Center`.
**Fixed for**: Label widget (removed centering, changed text_alignment to Left). Border's paint_label was already correct (uses label_alignment which defaults to Left).
**Remaining**: Verify no other widgets bypass border's paint_label with hardcoded Center.

**Masked by**: golden tests use width-constrained or single-line text, so the bug is invisible in tests but visible with short captions on wide panels or multi-line text.

## CC-06: hit_test() vs check_mouse() Divergence — **FIXED**

**Found in**: Button audit (2026-03-18)
**Affected widgets**: Button, CheckButton, CheckBox, RadioButton, RadioBox (any that use hit_test for input)
**Issue**: `input()` dispatches via `hit_test()` which tests the raw content_round_rect. The public `check_mouse()` has the correct C++ face-inset formula. The clickable area is slightly larger than C++ intended.
**Fix**: Non-boxed (Button, RadioButton, CheckButton): face inset d=(14/264)*r applied. Boxed (CheckBox, RadioBox): content_rect with r=h*0.2.

## CC-04: Missing VCT_MIN_EXT Checks

**Found in**: Button audit (2026-03-18, in progress)
**Affected widgets**: All interactive widgets
**Issue**: C++ gates input handling on `GetViewCondition(VCT_MIN_EXT) >= threshold`. This prevents interaction with widgets that are too small on screen. Rust may not have this guard.
