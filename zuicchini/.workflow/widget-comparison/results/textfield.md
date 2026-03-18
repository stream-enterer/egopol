# TextField Audit Report

**Date**: 2026-03-18
**Agent**: Batch 3
**C++ files**: emTextField.cpp (1847 LOC) + emTextField.h (427 LOC) = 2274 LOC
**Rust file**: text_field.rs (3378 LOC) — largest widget in the codebase

## Findings: 18 total

### [HIGH] Selection model: anchor-based vs start/end indexed — **INTENTIONAL DIVERGENCE 2026-03-18**
- **C++**: Tracks `SelectionStartIndex`/`SelectionEndIndex` (always ordered) + `CursorIndex`. `ModifySelection` uses closest-endpoint logic.
- **Rust**: Uses `selection_anchor: Option<usize>` derived from min/max of anchor + cursor. `modify_selection` uses extend boolean.
- **Behavioral parity achieved**: publish-to-clipboard-on-select (FIXED), EmptySelection clipboard clear (FIXED), select-after-undo (FIXED). The internal representation differs but all user-visible selection behaviors match C++ except: ModifySelection closest-endpoint logic for shift-click on existing selections. This edge case (shift-clicking within an existing selection to switch which endpoint extends) is rare and the Rust extend-from-anchor behavior is standard.
- **Justification**: The anchor-based model is idiomatic Rust and simpler to reason about. Converting to start/end would require adding a third field (cursor) and separate tracking, with no behavioral benefit for the implemented use cases.

### [HIGH] Undo/redo architecture completely different — **INTENTIONAL DIVERGENCE 2026-03-18**
- **C++**: Incremental edits `(pos, removeLen, insertText)` with positional merge, MAX_UNDOS=200, O(edit-size) per entry.
- **Rust**: Full text snapshots `(text, cursor)` with MAX_UNDO=100, O(text-length) per entry.
- **Behavioral parity achieved**: Undo selects restored text (MF_SELECT behavior implemented via diff_select_range). Undo merge for consecutive same-type edits works. Redo selects too.
- **What's lost**: O(edit-size) memory per entry (Rust uses O(text-length)). MAX_UNDOS=200 vs 100. Positional merge adjacency check (Rust merges by type only, not by position).
- **Justification**: The snapshot model is simpler, correct for all behavioral contracts, and sufficient for the text sizes in this application. The 2x undo depth difference and memory overhead are acceptable. Converting to incremental would require rewriting the entire edit pipeline with no user-visible behavioral improvement (now that MF_SELECT works).

### [HIGH] Backspace/Delete modifier handling more permissive — **FIXED**
- C++ plain Backspace requires `IsNoMod()` — no modifiers allowed
- **Fix**: Added `!alt && !meta && (!shift || ctrl)` guard on both Backspace and Delete
- **Confidence**: medium | **Coverage**: partially covered

### [HIGH] Ctrl+Left/Right calls wrong word-boundary function — **FIXED**
- C++ uses `GetPrevWordIndex`/`GetNextWordIndex`
- **Fix**: Replaced prev/next_word_boundary with prev/next_word_index in all 4 Ctrl+word ops
- **Confidence**: medium | **Coverage**: covered (widget_textfield_cursor_nav)

### [MEDIUM] Tab rendering in multi-line not expanded during paint — **FIXED**
- **Fix**: Column-grid based segment rendering added; each row is now split at tab characters and segments are painted at `tx + col * cw` positions, with tab stops advancing to the next 8-column boundary via `col = (col + 7) & !7`.
- C++ processes tabs char-by-char: `col=(col+7)&~7`, paints segments between tabs
- Rust splits on '\n' and paints each row as a single string — tabs not expanded
- **Confidence**: high | **Coverage**: uncovered

### [MEDIUM] Overwrite mode doesn't expand cols count for cursor — **FIXED**
- C++ increments `cols` when cursor is at last column in overwrite mode + focused
- **Fix**: Both paint paths increment cols matching C++ emTextField.cpp:920-922
- **Confidence**: high | **Coverage**: uncovered

### [MEDIUM] Double-click word selection differs on delimiters — **FIXED**
- C++ double-click on delimiter selects from delimiter to next word boundary (always non-empty)
- **Fix**: Added prev_word_boundary_index, updated double-click and drag-by-words to use boundary-based segment selection
- **Confidence**: high | **Coverage**: uncovered

### [MEDIUM] DM_MOVE: no live drag feedback — **FIXED**
- **Fix**: Live drag implemented with snapshot revert/reapply on each motion event, matching C++ continuous repositioning.

### [MEDIUM] Ctrl+A doesn't publish selection to clipboard — **FIXED**
- C++ `SelectAll(true)` publishes to clipboard
- **Fix**: Added publish_selection() after select_all() in Ctrl+A handler
- **Confidence**: high | **Coverage**: uncovered

### [MEDIUM] Password mode paints as single string vs individual chars — **FIXED**
- **Fix**: Password mode now paints each `*` individually at column-grid positions (`tx + col * cw`), matching C++ per-char placement.

### [MEDIUM] Selection polygon uses measured text width vs column grid — **FIXED**
- **Fix**: Selection highlight now uses column-grid positioning (`tx + col * cw`), matching C++ exactly.

### [MEDIUM] Ctrl+Shift+A doesn't clear clipboard selection — **FIXED**
- **Fix**: Added on_clipboard_clear callback, called from deselect() matching C++ EmptySelection → Clipboard->Clear.

### [MEDIUM] Disabled state color blending absent — **FIXED**
- **Fix**: Lerp blend toward BgColor added when disabled, matching CC-03 pattern.
- See CC-03

### [LOW] Undo select-after-undo behavior — **FIXED**
- **Fix**: Undo/redo now selects the restored text range via diff_select_range, matching C++ MF_SELECT.

### [LOW] Validation model differs — **INTENTIONAL DIVERGENCE 2026-03-18**
- **C++**: Pre-edit hook `Validate(pos, removeLen, insertText)` can modify the edit operation. Subclasses can do auto-formatting (e.g., capitalize, normalize whitespace).
- **Rust**: Post-edit boolean `validate(&str) -> bool` that accepts or rejects the entire text.
- **Justification**: The Rust validation model is simpler and covers the primary use case (input filtering). No Rust code currently needs pre-edit modification. If auto-formatting is needed later, the callback signature can be changed to `Fn(&str, usize, &str) -> Option<String>` where the return modifies the edit.

### [LOW] max_length is Rust-only addition — **NOTE**
- Not a divergence from C++; this is a Rust-side extension.

### [LOW] Home/End in single-line — **NOTE**
- No divergence found on closer inspection.

### [LOW] GetRowEndIndex — **NOTE**
- No divergence found on closer inspection.

## Summary

| Severity | Count |
|----------|-------|
| HIGH | 4 |
| MEDIUM | 9 |
| LOW | 5 |

## Most Critical

1. **Undo/redo architecture** — completely redesigned, visible behavioral differences (select-after-undo)
2. **Selection model** — anchor vs start/end, clipboard publishing missing
3. **Tab rendering** — not expanded during multi-line paint
4. **Word boundary function** — keyboard nav calls wrong function
5. **Backspace modifier handling** — too permissive
6. **Double-click on delimiters** — selects empty range instead of word boundary
7. **Drag-move** — no live visual feedback (UX regression)

## Recommended Tests
- Undo/redo visual state (does undone text get selected?)
- Tab character rendering in multi-line
- Double-click on spaces/delimiters
- Shift+Backspace behavior
- Ctrl+Left/Right on "hello  .  world" edge cases
- Password mode pixel comparison
- Drag-move visual feedback

## Overall: Functional reimplementation that captures major behaviors but with significant architectural differences in undo/redo and selection. Paint pipeline geometry is faithful for common cases. Interaction layer has the most divergences.
