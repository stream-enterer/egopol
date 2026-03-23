# Phase 4a: Diagnose Two Golden Test Divergences

Diagnosis only. Do not fix anything. Do not modify source code. Read, compare, classify, report.

## Divergences

1. **CAP-0076 Tunnel** — `widget_tunnel` golden test, 8.52% pixel failure (40,908/480,000), max_channel_diff=76
2. **CAP-0027 FileSelectionBox** — `widget_file_selection_box` golden test, 84.09% pixel failure (403,644/480,000), max_channel_diff=161

## Procedure for Each Divergence

### Step 1: Dump diff images

Run:
```
DUMP_GOLDEN=1 cargo test --test golden widget_tunnel 2>&1
DUMP_GOLDEN=1 cargo test --test golden widget_file_selection_box 2>&1
```

Check `tests/golden/debug/` for PPM diff images. Read the diff image to understand WHERE pixels diverge (edges only? entire regions? specific child panels?).

### Step 2: Compare test setup vs gen function

Read the Rust test in `tests/golden/widget.rs` — find `widget_tunnel` and `widget_file_selection_box`. Note the exact widget construction: constructor args, state mutations, viewport dimensions.

Read the C++ gen function in `tests/golden/gen/gen_golden.cpp` — find the corresponding `gen_widget_tunnel` and `gen_widget_file_selection_box` functions. Note the exact setup.

Compare them side by side. Report any differences in:
- Widget constructor parameters
- State mutations (set_depth, set_child_tallness, set_parent_directory, etc.)
- Viewport dimensions (width, height)
- Settle cycle count
- Child panel setup

If the setups don't match, the divergence is a **test scenario mismatch**, not a code bug. Report this and stop — the fix is to align the test setup, not to change implementation code.

### Step 3: Compare Rust implementation vs C++ implementation

Only if Step 2 shows matching setups.

**For Tunnel:**
- Read `src/widget/tunnel.rs` — find the `paint()` method
- Read `~/.local/git/eaglemode-0.96.4/src/emCore/emTunnel.cpp` — find the `Paint()` method
- Compare the painting logic line by line. Look for:
  - Rounded-rect radius calculation differences
  - Color interpolation formula differences
  - Coordinate calculation differences (x, y, w, h of each concentric ring)
  - Number of concentric rings / depth steps
  - Background fill color
  - Border rendering

**For FileSelectionBox:**
- Read `src/widget/file_selection_box.rs` — find `paint()` and `layout_children()` (or equivalent)
- Read `~/.local/git/eaglemode-0.96.4/src/emCore/emFileSelectionBox.cpp` — find `Paint()` and `LayoutChildren()`
- At 84% divergence, look for:
  - Entirely missing paint calls (whole regions not painted)
  - Wrong layout (children at wrong positions)
  - Wrong background color or missing background fill
  - Different number of child panels created
  - Different text content or font rendering

### Step 4: Classify the divergence

For each divergence, determine which domain from the classification system applies:

| Domain | Indicator |
|---|---|
| Domain 1: Pixel Arithmetic | Divergence is in color values, blending, or coverage at edges. max_diff is small (1-10). Pattern is scattered or edge-concentrated. |
| Domain 2: Geometry | Divergence is in positions, sizes, or shapes. Rects are offset or wrong size. Pattern shows shifted regions. |
| Domain 3: State Logic | Wrong UI state rendered (e.g., wrong panel visible, wrong selection state, wrong enabled/disabled). Large contiguous wrong regions. |
| Domain 5: API/Structure | Entire child panels missing or in wrong order. Layout fundamentally different. |

## Output

Write results to `state/run_003/divergence_diagnosis.json`:

```json
{
  "diagnosed_at": "<ISO8601>",
  "divergences": [
    {
      "capability_id": "CAP-NNNN",
      "test_function": "<string>",
      "pixels_failing": "<integer>",
      "max_diff": "<integer>",
      "setup_match": "<boolean: true if Rust test and C++ gen function use same setup>",
      "setup_differences": ["<string: list any differences if setup_match is false>"],
      "domain": "<Domain 1|Domain 2|Domain 3|Domain 5>",
      "root_cause": "<string: specific description of what differs>",
      "fix_strategy": "<port_arithmetic|port_algorithm|fix_geometry|fix_state_logic|fix_api_semantics>",
      "cpp_source_lines": "<string: file:line range to read for the fix>",
      "rust_source_lines": "<string: file:line range that needs changing>",
      "estimated_difficulty": "<low|medium|high>"
    }
  ]
}
```

## Rules

- Do not modify any source files.
- Do not modify any test files.
- Do not attempt fixes.
- Read C++ source, read Rust source, compare, classify, report.
- If a divergence is caused by test scenario mismatch (Step 2), say so and do not proceed to Step 3 for that divergence.
