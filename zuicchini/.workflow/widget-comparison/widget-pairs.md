# Widget Pair Mapping: C++ emCore → Rust zuicchini

## Mapping Table

| C++ Class | C++ Files (LOC) | Rust Module | Rust File (LOC) | Fidelity Layer | Notes |
|-----------|----------------|-------------|-----------------|----------------|-------|
| `emBorder` | `emBorder.cpp` (1460) + `.h` (510) = 1970 | `widget::Border` | `border.rs` (2676) | Pixel + Geometry | Paint-heavy; outer/inner border types, label layout, 9-slice |
| `emButton` | `emButton.cpp` (452) + `.h` (171) = 623 | `widget::Button` | `button.rs` (557) | Pixel + Geometry | Rounded-rect hit test, pressed/hover states |
| `emCheckBox` | `emCheckBox.cpp` (38) + `.h` (52) = 90 | `widget::CheckBox` | `check_box.rs` (346) | State | Thin wrapper, delegates to Border; Rust much larger (suspicious) |
| `emCheckButton` | `emCheckButton.cpp` (99) + `.h` (91) = 190 | `widget::CheckButton` | `check_button.rs` (340) | Pixel + State | DoButton non-boxed path |
| `emRadioButton` | `emRadioButton.cpp` (269) + `.h` (251) = 520 | `widget::RadioButton` | `radio_button.rs` (819) | Pixel + State | Dot rendering, exclusive selection |
| `emRadioBox` | `emRadioBox.cpp` (37) + `.h` (52) = 89 | `widget::RadioBox` | `radio_box.rs` (350) | State | Thin wrapper; Rust much larger (suspicious) |
| `emLabel` | `emLabel.cpp` (50) + `.h` (61) = 111 | `widget::Label` | `label.rs` (134) | Pixel | Text fitting algorithm in DoLabel |
| `emTextField` | `emTextField.cpp` (1847) + `.h` (427) = 2274 | `widget::TextField` | `text_field.rs` (3378) | Pixel + Geometry + State | Undo/redo, selection, cursor blink, multi-line |
| `emScalarField` | `emScalarField.cpp` (527) + `.h` (236) = 763 | `widget::ScalarField` | `scalar_field.rs` (982) | Pixel + Geometry + State | Marks, range, text formatter |
| `emColorField` | `emColorField.cpp` (540) + `.h` (167) = 707 | `widget::ColorField` | `color_field.rs` (747) | Pixel + Geometry | Color picker, expansion modes |
| `emListBox` | `emListBox.cpp` (1075) + `.h` (483) = 1558 | `widget::ListBox` | `list_box.rs` (1992) | State + Geometry | Selection modes, keywalk, custom item panels |
| `emFilePanel` | `emFilePanel.cpp` (431) + `.h` (175) = 606 | `widget::FilePanel` | `file_panel.rs` (479) | State | File browser with filtering |
| `emFileSelectionBox` | `emFileSelectionBox.cpp` (1217) + `.h` (403) = 1620 | `widget::FileSelectionBox` | `file_selection_box.rs` (665) | State | Rust much smaller — likely missing logic |
| `emFileDialog` | `emFileDialog.cpp` (204) + `.h` (310) = 514 | `widget::FileDialog` | `file_dialog.rs` (341) | State | Modal file picker |
| `emDialog` | `emDialog.cpp` (335) + `.h` (255) = 590 | `widget::Dialog` | `dialog.rs` (198) | State | Generic modal dialog; Rust smaller |
| `emSplitter` | `emSplitter.cpp` (271) + `.h` (139) = 410 | `widget::Splitter` | `splitter.rs` (293) | Geometry + State | Resizable divider, drag handling |
| `emTunnel` | `emTunnel.cpp` (192) + `.h` (114) = 306 | `widget::Tunnel` | `tunnel.rs` (332) | Pixel + Geometry | Magnifier/zoom window |
| `emErrorPanel` | `emErrorPanel.cpp` (63) + `.h` (56) = 119 | `widget::ErrorPanel` | `error_panel.rs` (92) | State | Simple error display |
| `emLook` | `emLook.cpp` (218) + `.h` (218) = 436 | `widget::Look` | `look.rs` (129) | State | Theme/colors; Rust smaller — may inline differently |
| `emCoreConfigPanel` | `emCoreConfigPanel.cpp` (876) + `.h` (203) = 1079 | `widget::CoreConfigPanel` | `core_config_panel.rs` (1569) | State | Config UI |

## Non-Widget Comparison Units (Panel/Layout/Render)

| C++ Class | C++ LOC | Rust Module | Rust LOC | Fidelity Layer |
|-----------|---------|-------------|----------|----------------|
| `emPanel` | 2696 | `panel::PanelTree` | `tree.rs` + `behavior.rs` + `ctx.rs` | State + Geometry |
| `emView` | 3757 | `panel::View` | `view.rs` | Geometry + State |
| `emViewAnimator` | 2527 | `panel::ViewAnimator` | `animator.rs` | Geometry |
| `emViewInputFilter` | 1614 | `panel::InputFilter` | `input_filter.rs` | State |
| `emSubViewPanel` | 358 | `panel::SubViewPanel` | `sub_view_panel.rs` | State + Geometry |
| `emLinearLayout` | 878 | `layout::LinearLayout` | `linear.rs` | Geometry |
| `emPackLayout` | 854 | `layout::PackLayout` | `pack.rs` | Geometry |
| `emRasterLayout` | 683 | `layout::RasterLayout` | `raster.rs` | Geometry |
| `emPainter` | ~8000+ (multi-file) | `render::Painter` | `painter.rs` + `scanline*.rs` | Pixel (exact) |
| `emGroup` | 121 | (absorbed into Border/layout) | — | State |

## Rust-Only Modules (no C++ counterpart to compare)

- `widget::FieldPanel` — Rust-specific container
- `widget::ImageFilePanel` — Rust-specific
- `widget::ToolkitImages` — Rust-specific icon loading
