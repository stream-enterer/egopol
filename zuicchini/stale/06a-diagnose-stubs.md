# Phase 6a: Diagnose Partial Stubs

Diagnosis only. Do not fix anything. Do not modify source code. Read, classify, report.

## Input

5 capabilities marked `stub` in the capability map. Each has a Rust implementation where some methods are incomplete.

## Procedure

For each capability below:

1. Read the Rust source file. Find every method that is a stub (returns None, returns default, no-op, logs debug, uses a placeholder).
2. Read the corresponding C++ header and source to understand what the method should do.
3. Classify each stub method into one of these categories:

### Category A: Implementable

The C++ behavior can be ported to Rust with existing dependencies. No platform APIs, no external crates, no missing infrastructure.

Record: the C++ source lines, what the method should do, estimated difficulty (low/medium/high).

### Category B: Platform Limitation

The method requires platform APIs (window system calls, OS-specific behavior) that zuicchini handles differently or doesn't expose. Examples: programmatic cursor warping, system beep, screensaver inhibit.

Record: what platform API is needed, whether winit/wgpu exposes it, whether a real implementation is possible or this is a genuine limitation.

### Category C: Unnecessary

The method exists in C++ for reasons that don't apply in Rust (manual memory management, C++ compatibility shims, features superseded by Rust idioms).

Record: why it's unnecessary.

### Category D: Blocked

The method depends on other unported features or infrastructure that doesn't exist yet.

Record: what it depends on.

## Capabilities to Diagnose

### CAP-0033: Image (`src/foundation/image.rs`)
- Known stub: `try_parse_xpm` returns None
- Search for other stubs: `grep -n 'None\|todo\|stub\|placeholder\|unimplemented\|default()' src/foundation/image.rs` and check each hit against the C++ API
- C++ source: `~/.local/git/eaglemode-0.96.4/include/emCore/emImage.h` and `src/emCore/emImage.cpp`

### CAP-0041: ListBox (`src/widget/list_box.rs`)
- Search for stubs: `grep -n 'None\|todo\|stub\|placeholder\|unimplemented\|default()' src/widget/list_box.rs`
- C++ source: `~/.local/git/eaglemode-0.96.4/include/emCore/emListBox.h` and `src/emCore/emListBox.cpp`

### CAP-0069: TextField (`src/widget/text_field.rs`)
- Search for stubs: `grep -n 'None\|todo\|stub\|placeholder\|unimplemented\|default()' src/widget/text_field.rs`
- C++ source: `~/.local/git/eaglemode-0.96.4/include/emCore/emTextField.h` and `src/emCore/emTextField.cpp`

### CAP-0082: Window (`src/window/zui_window.rs`)
- Known stubs: `move_mouse_pointer` (no-op), `beep` (no-op), `inhibit_screensaver` (counter only), `window_flags_signal` (returns close_signal as placeholder)
- Search for others: `grep -n 'no.op\|stub\|placeholder\|todo\|debug\!.*not' src/window/zui_window.rs`
- C++ source: `~/.local/git/eaglemode-0.96.4/include/emCore/emWindow.h` and `src/emCore/emWindow.cpp`

### CAP-0064: Std1/Fixed12 (`src/foundation/fixed.rs`)
- Already confirmed covered by 42 painter golden tests. Verify: are there any stub methods remaining?
- Search: `grep -n 'None\|todo\|stub\|placeholder\|unimplemented\|default()' src/foundation/fixed.rs`
- If no stubs found: classify as "no action needed — reclassify from stub to verified"

## Output

Write results to `state/run_003/stub_diagnosis.json`:

```json
{
  "diagnosed_at": "<ISO8601>",
  "capabilities": [
    {
      "capability_id": "CAP-NNNN",
      "rust_file": "<string>",
      "stubs": [
        {
          "method": "<string: function/method name>",
          "rust_line": "<integer>",
          "category": "A|B|C|D",
          "cpp_source": "<string: file:lines>",
          "description": "<string: what it should do>",
          "evidence": "<string: why this category>",
          "estimated_difficulty": "<low|medium|high|n/a>"
        }
      ],
      "summary": "<string: overall assessment for this capability>"
    }
  ]
}
```

## Rules

- Run grep for every capability. Do not assume you know all the stubs — find them.
- Do not modify any files except `stub_diagnosis.json`.
- For each stub, read both the Rust implementation AND the C++ source before classifying.
- If a capability has zero stubs, say so explicitly.
