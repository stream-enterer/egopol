# Per-Widget Comparison Prompt Template

Use this template to drive the LLM comparison pass for each widget pair.

## Prompt

```
You are comparing a C++ widget implementation against its Rust port to find bugs.

## Widget: {{WIDGET_NAME}}
- C++ class: `em{{CPP_CLASS}}`
- C++ source: `{{CPP_SRC_PATH}}` ({{CPP_LOC}} lines)
- C++ header: `{{CPP_HDR_PATH}}`
- Rust module: `widget::{{RUST_MODULE}}`
- Rust source: `{{RUST_SRC_PATH}}` ({{RUST_LOC}} lines)

## Fidelity rules

The port has three fidelity layers. Apply the correct layer per function:

1. **Pixel arithmetic** (blend, coverage, interpolation, sampling):
   Must reproduce C++ integer formulas exactly. `(x*257+0x8073)>>16` not f64 division.
   No approximations in compositing pipeline.

2. **Geometry** (coordinates, rects, transforms, layout):
   Same algorithm and operation order. Boundary values must match C++.

3. **State logic** (data structures, ownership, API surface):
   Fully idiomatic Rust. Golden tests verify output, not structure.
   Preserve behavioral contracts (return values, side effects, ordering).

## Translation idioms (do NOT flag these as bugs)

[Insert content from rosetta-stone.md]

## Bug classes to check (ordered by priority)

1. Integer arithmetic divergence (signed/unsigned, overflow, rounding)
2. Off-by-one and boundary errors
3. Missing or incorrect state transitions
4. Float↔integer conversion errors
5. Missing functionality (unported methods, stubs, todo!())
6. Coordinate system and transform errors
7. String and text handling differences
8. Visual/rendering formula differences
9. Input handling differences (hit test, focus, event sequence)

## Golden test coverage for this widget

{{GOLDEN_TESTS_LIST}}

## Your task

1. Read both the C++ and Rust implementations completely.
2. For each public method / behavioral contract in the C++ class:
   a. Find the Rust equivalent
   b. Classify its fidelity layer
   c. Compare the implementation against the appropriate fidelity standard
   d. Note any discrepancy
3. Check for missing functionality — methods in C++ with no Rust equivalent.
4. Check for size asymmetry anomalies — if Rust is much larger or smaller than C++, explain why.
5. Check for incorrect constants, thresholds, or magic numbers.

## Output format

For each finding, report:

### [SEVERITY] Short description
- **C++ location**: `emFoo.cpp:123`
- **Rust location**: `foo.rs:456`
- **Fidelity layer**: pixel / geometry / state
- **Bug class**: (from taxonomy)
- **Description**: What differs and why it matters
- **Confidence**: high / medium / low
- **Golden test coverage**: covered / uncovered / partially covered

Severity levels:
- **BUG**: Definite behavioral divergence from C++
- **SUSPECT**: Likely bug but uncertain without runtime verification
- **GAP**: Missing functionality (stub, todo, unported method)
- **NOTE**: Interesting difference that may be intentional

At the end, provide:
- Total findings by severity
- Recommended golden tests to add (for uncovered findings)
- Overall assessment of port fidelity for this widget
```

## File paths for each widget

| Widget | C++ Source | C++ Header | Rust Source |
|--------|-----------|------------|-------------|
| Border | `/home/ar/.local/git/eaglemode-0.96.4/src/emCore/emBorder.cpp` | `.../include/emCore/emBorder.h` | `src/widget/border.rs` |
| Button | `.../src/emCore/emButton.cpp` | `.../include/emCore/emButton.h` | `src/widget/button.rs` |
| CheckBox | `.../src/emCore/emCheckBox.cpp` | `.../include/emCore/emCheckBox.h` | `src/widget/check_box.rs` |
| CheckButton | `.../src/emCore/emCheckButton.cpp` | `.../include/emCore/emCheckButton.h` | `src/widget/check_button.rs` |
| RadioButton | `.../src/emCore/emRadioButton.cpp` | `.../include/emCore/emRadioButton.h` | `src/widget/radio_button.rs` |
| RadioBox | `.../src/emCore/emRadioBox.cpp` | `.../include/emCore/emRadioBox.h` | `src/widget/radio_box.rs` |
| Label | `.../src/emCore/emLabel.cpp` | `.../include/emCore/emLabel.h` | `src/widget/label.rs` |
| TextField | `.../src/emCore/emTextField.cpp` | `.../include/emCore/emTextField.h` | `src/widget/text_field.rs` |
| ScalarField | `.../src/emCore/emScalarField.cpp` | `.../include/emCore/emScalarField.h` | `src/widget/scalar_field.rs` |
| ColorField | `.../src/emCore/emColorField.cpp` | `.../include/emCore/emColorField.h` | `src/widget/color_field.rs` |
| ListBox | `.../src/emCore/emListBox.cpp` | `.../include/emCore/emListBox.h` | `src/widget/list_box.rs` |
| FilePanel | `.../src/emCore/emFilePanel.cpp` | `.../include/emCore/emFilePanel.h` | `src/widget/file_panel.rs` |
| FileSelectionBox | `.../src/emCore/emFileSelectionBox.cpp` | `.../include/emCore/emFileSelectionBox.h` | `src/widget/file_selection_box.rs` |
| FileDialog | `.../src/emCore/emFileDialog.cpp` | `.../include/emCore/emFileDialog.h` | `src/widget/file_dialog.rs` |
| Dialog | `.../src/emCore/emDialog.cpp` | `.../include/emCore/emDialog.h` | `src/widget/dialog.rs` |
| Splitter | `.../src/emCore/emSplitter.cpp` | `.../include/emCore/emSplitter.h` | `src/widget/splitter.rs` |
| Tunnel | `.../src/emCore/emTunnel.cpp` | `.../include/emCore/emTunnel.h` | `src/widget/tunnel.rs` |
| ErrorPanel | `.../src/emCore/emErrorPanel.cpp` | `.../include/emCore/emErrorPanel.h` | `src/widget/error_panel.rs` |
| Look | `.../src/emCore/emLook.cpp` | `.../include/emCore/emLook.h` | `src/widget/look.rs` |
| CoreConfigPanel | `.../src/emCore/emCoreConfigPanel.cpp` | `.../include/emCore/emCoreConfigPanel.h` | `src/widget/core_config_panel.rs` |

All C++ paths start with `/home/ar/.local/git/eaglemode-0.96.4/`.
