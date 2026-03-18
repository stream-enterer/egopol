# Golden Test Coverage Map

## Widget Golden Tests (pixel comparison)

| Widget | Render Tests | Interaction Tests | Coverage Assessment |
|--------|-------------|-------------------|-------------------|
| Border | `widget_border_rect`, `widget_border_round_rect`, `widget_border_group`, `widget_border_instrument`, `parallel_border_*` (4 variants) | — | Good render coverage; no interaction (Border is passive) |
| Button | `widget_button_normal` | `widget_button_click` | Minimal — only normal state rendered, single click interaction |
| CheckBox | `widget_checkbox_unchecked`, `widget_checkbox_checked` | `widget_checkbox_toggle` | Adequate — both states + toggle |
| CheckButton | — | `widget_checkbutton_toggle` | Gap — no render golden, only interaction |
| RadioButton | `widget_radiobutton` | `widget_radiobutton_switch` | Adequate — render + switch |
| RadioBox | — | — | Gap — no golden tests at all |
| Label | `widget_label`, `parallel_label` | — | Minimal — render only |
| TextField | `widget_textfield_empty`, `widget_textfield_content` | `widget_textfield_type`, `widget_textfield_backspace`, `widget_textfield_select`, `widget_textfield_cursor_nav` | Good — multiple states and interactions |
| ScalarField | `widget_scalarfield`, `parallel_scalarfield` | `widget_scalarfield_inc` | Adequate — render + increment |
| ColorField | `widget_colorfield`, `colorfield_expanded` | — | Moderate — two render states, no interaction |
| ListBox | `widget_listbox`, `listbox_expanded` | `widget_listbox_select`, `widget_listbox_multi`, `widget_listbox_toggle` | Good — render + three interaction modes |
| Splitter | `widget_splitter_h`, `widget_splitter_v` | `widget_splitter_setpos`, `widget_splitter_drag`, `splitter_layout_h`, `splitter_layout_v` | Good — both orientations, drag + layout |
| FilePanel | `widget_file_panel` | — | Minimal — render only |
| FileSelectionBox | `widget_file_selection_box` | — | Minimal — render only |
| Tunnel | `widget_tunnel` | — | Minimal — render only |
| ErrorPanel | `widget_error_panel` | — | Minimal — render only |
| Dialog | — | — | Gap — no golden tests |
| FileDialog | — | — | Gap — no golden tests |
| CoreConfigPanel | — | — | Gap — no golden tests |
| Look | — | — | Gap — tested indirectly via widget renders |

## Non-Widget Golden Tests

| Component | Tests | Coverage |
|-----------|-------|----------|
| Painter | 30+ tests (rect, ellipse, polygon, gradient, line, stroke, bezier, clip, image, text, transform) | Comprehensive |
| Compositor | 5 tests (single, overlap, nested, canvas_color, two_children) | Adequate |
| Layout | 22+ tests (linear h/v, raster, pack; spacing, alignment, adaptive, min/max) | Comprehensive |
| Input | 6 tests (mouse_hit, key_to_focused, scroll_delta, mouse_miss, nested_hit, drag_sequence) | Adequate |
| InputFilter | 9 tests (wheel zoom in/out, acceleration, middle pan/fling, keyboard scroll/zoom/release) | Good |
| Notice | 13 tests (active, focus, layout, children, window focus, resize, enable, remove) | Comprehensive |
| Interaction | 22+ tests (activate, focus, tab, visit, disabled, remove scenarios) | Comprehensive |
| Scheduler | 11 tests (signal, timer, engine) | Adequate |
| Animator | 11 tests (kinetic, speeding, swiping, visiting trajectories) | Good |
| TestPanel | 2 tests (root, expanded) | Minimal |

## Coverage Gaps Summary

**No golden tests at all:** RadioBox, Dialog, FileDialog, CoreConfigPanel, Look
**Render only (no interaction):** Label, ColorField, FilePanel, FileSelectionBox, Tunnel, ErrorPanel
**Interaction only (no render golden):** CheckButton
**Minimal single-state coverage:** Button (only normal state)
