# emStocks Full Integration: Wiring to Existing emCore Primitives

**Date:** 2026-03-31
**Scope:** emStocks crate — all DIVERGED comments (61 occurrences, 56 unique comment blocks) and 10 TODOs across 8 files
**Approach:** Bottom-up by layer (A). Each phase has a strict "compiles + passes tests" gate before the next begins.
**Predecessor:** [gap-closure-design.md](2026-03-31-gap-closure-design.md) Phase 4

## Motivation

All emCore primitives that emStocks depends on already exist: emEngine, emTimer, PaintTextBoxed, emLinearGradientTexture, emStroke, emPanel/emBorder/emFilePanel, emListBox, all widget types (emTextField, emButton, emCheckBox, emScalarField, emRadioButton, emFileSelectionBox), and emDialog. The 45 DIVERGED comments and 10 TODOs in emStocks are not blocked on missing infrastructure — they are deferred integration points that need wiring.

This spec enumerates every DIVERGED and TODO comment, specifies whether it is resolved (code replaces comment) or updated (idiomatic Rust justification added), and assigns it to a phase.

## Layer 0 Idiom Divergences (Permanent)

These DIVERGED comments remain permanently. They mark intentional Rust idioms that improve on C++ patterns. The final pass updates their text to include explicit justifications.

| ID | File | Line | Current text | Updated text |
|----|------|------|-------------|--------------|
| I1 | emStocksRec.rs | 13-14 | `Rust enum replaces C++ int enum + emEnumRec subclass.` | `Rust enum replaces C++ int enum + emEnumRec subclass — Rust enums are the idiomatic equivalent of C++ int enums with associated string tables.` |
| I2 | emStocksRec.rs | 231 | `Returns (days, dates_valid) tuple instead of C++ bool* out-param.` | `Returns (i32, bool) tuple instead of C++ bool* out-param — Rust has no out-parameters; tuples are the idiomatic equivalent.` |
| I3 | emStocksRec.rs | 289 | `Rust struct fields use snake_case.` | `Rust struct fields use snake_case — required by Rust naming conventions (clippy::non_snake_case). Method names preserve C++ names per File and Name Correspondence.` |
| I4 | emStocksRec.rs | 630 | `Returns Option<f64> instead of bool + *pResult.` | `Returns Option<f64> instead of C++ bool + *pResult — Option is Rust's idiomatic replacement for success bool + out-pointer.` |
| I5 | emStocksRec.rs | 927 | `Returns Option<usize> instead of -1.` | `Returns Option<usize> instead of C++ -1 sentinel — Option<usize> is Rust's idiomatic replacement for signed-int sentinel values.` |
| I6 | emStocksRec.rs | 936-937 | `Returns Option<usize> instead of -1.` | Same as I5. |
| I7 | emStocksPricesFetcher.rs | 4-5 | `Uses BTreeMap<String, Option<usize>> instead of emAvlTreeMap/emCrossPtr.` | `Uses BTreeMap<String, Option<usize>> instead of C++ emAvlTreeMap<String, emCrossPtr<StockRec>> — BTreeMap is Rust's idiomatic ordered map; cross-pointers don't apply when StockRecs live in a Vec.` |
| I8 | emStocksItemChart.rs | 920-922 | Tuple return part of CalculateYScaleLevelRange DIVERGED | `Returns (min_level, min_dist, max_level) tuple instead of C++ output pointers — Rust has no out-parameters; tuples are the idiomatic equivalent.` |
| I9 | emStocksFileModel.rs | 20-23 | `Composition instead of multiple inheritance.` | Composition part stays: `Composition instead of C++ multiple inheritance — Rust has no MI; composition with delegation is the idiomatic equivalent.` emTimer part resolved in Phase 4. |

---

## Phase 0 — System Integration

**Dependencies:** None (leaf nodes).
**Files touched:** `emStocksFilePanel.rs`, `Cargo.toml` (emstocks crate), `emClipboard.rs` (emcore).
**Gate:** `cargo check` + `cargo-nextest ntr` pass.

### 0.1 Clipboard: add `arboard` crate

`emClipboard` trait exists with `emPrivateClipboard` (in-memory). Add `emSystemClipboard` implementation backed by `arboard` (cross-platform, Wayland+X11).

**TODOs resolved:**

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| T1 | emStocksFilePanel.rs | 156 | `TODO: put _clipboard text on system clipboard` | Call `clipboard.set_text(text)`. Remove TODO. |
| T2 | emStocksFilePanel.rs | 162 | `TODO: put _clipboard text on system clipboard` | Call `clipboard.set_text(text)`. Remove TODO. |
| T3 | emStocksFilePanel.rs | 167 | `TODO: read clipboard_text from system clipboard` | Call `clipboard.get_text()`. Remove TODO. |
| T4 | emStocksFilePanel.rs | 192 | `TODO: read selection text from system clipboard` | Call `clipboard.get_text()`. Remove TODO. |

### 0.2 Browser launching: add `open` crate

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| T5 | emStocksFilePanel.rs | 187 | `TODO: launch browser with _pages` | `for url in pages { open::that(url); }`. Remove TODO. |
| T6 | emStocksFilePanel.rs | 217 | `TODO: launch browser with _pages` | `for url in pages { open::that(url); }`. Remove TODO. |

### 0.3 Fetch dialog launch (deferred)

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| T7 | emStocksFilePanel.rs | 181 | `TODO: launch fetch dialog with _ids` | Deferred to Phase 4. Update TODO to: `TODO(Phase 4): create real emStocksFetchPricesDialog`. |

**Phase 0 totals:** 6 TODOs resolved, 1 TODO deferred to Phase 4.

---

## Phase 1 — emCore Primitive Wiring

**Dependencies:** None within emStocks (primitives exist in emcore).
**Files touched:** `emStocksFilePanel.rs`, `emStocksItemChart.rs`, `emStocksPricesFetcher.rs`.
**Gate:** `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr` pass.

### 1.1 PaintTextBoxed

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| T8 | emStocksFilePanel.rs | 40 | `TODO: Paint _msg using emPainter::PaintTextBoxed when available` | Call `painter.PaintTextBoxed(...)`. Remove TODO. |

### 1.2 emLinearGradientTexture

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D36 | emStocksItemChart.rs | 1001-1003 | `DIVERGED: C++ uses emLinearGradientTexture for gradient fill. We use a single blended color.` | Use `emTexture::LinearGradient { color_a, color_b, start, end }`. Remove DIVERGED. |

### 1.3 emRoundedStroke

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D37 | emStocksItemChart.rs | 1212-1213 | `DIVERGED: C++ uses PaintLine with emRoundedStroke and emStrokeEnd per segment.` | Use `PaintLine` per segment with `emStroke { join: LineJoin::Round, cap: LineCap::Round }` matching C++. Remove DIVERGED. |

### 1.4 emEngine trait for PricesFetcher

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D-hdr1 | emStocksPricesFetcher.rs | 2 | `DIVERGED: No emEngine trait impl (standalone Cycle method instead).` | Implement `emEngine` trait. Existing `Cycle` becomes the trait impl. Remove DIVERGED. |
| D-hdr2 | emStocksPricesFetcher.rs | 3 | `DIVERGED: No FileModel/FileStateSignal/ChangeSignal integration` | Deferred to Phase 4. Update to: `DIVERGED(Phase 4): FileModel/FileStateSignal/ChangeSignal integration pending.` |

**Phase 1 totals:** 3 DIVERGED resolved, 1 TODO resolved, 1 DIVERGED deferred to Phase 4.

---

## Phase 2 — Panel Framework Wiring

**Dependencies:** Phase 1 complete.
**Files touched:** `emStocksFilePanel.rs`, `emStocksItemChart.rs`.
**Gate:** `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr` pass.

### 2.1 emFilePanel integration

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D27 | emStocksFilePanel.rs | 19-21 | `DIVERGED: Rust uses emStocksRec directly since emStocksFileModel is not yet fully integrated.` | Deferred to Phase 4 (FileModel ownership). Update to: `DIVERGED(Phase 4): FileModel ownership pending.` |
| D28 | emStocksFilePanel.rs | 24-25 | `DIVERGED: C++ uses IsVFSGood() from emFilePanel. Rust uses simple bool.` | Wire to `emFilePanel::IsVFSGood()`. Remove DIVERGED. |

**TODOs resolved:**

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| T9 | emStocksFilePanel.rs | 267 | `TODO: lay out ListBox child once it is a real panel child` | Deferred to Phase 4. Update to: `TODO(Phase 4): ListBox as real panel child.` |
| T10 | emStocksFilePanel.rs | 272 | `TODO: implement when emFilePanel integration and signal infrastructure are in place.` | Wire `Cycle` to check `VirFileStateSignal` and call `UpdateControls`. Remove TODO. |

### 2.2 PaintParams elimination / view context

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D38 | emStocksItemChart.rs | 1297-1299 | `DIVERGED: PaintParams replaces view context queries` | Remove `PaintParams` struct. ItemChart queries its own panel context. Remove DIVERGED. |
| D32 | emStocksItemChart.rs | 485-487 | `DIVERGED: Rust accepts PaintParams bundle` | `PaintContent` takes `(painter, x, y, w, h, canvasColor)` matching C++. Remove DIVERGED. |
| D33 | emStocksItemChart.rs | 661-662 | `DIVERGED: C++ uses ViewToPanelY(GetClipY2())` | Use actual view context call. Remove DIVERGED. |
| D34 | emStocksItemChart.rs | 868 | `DIVERGED: C++ uses ViewToPanelX(GetClipX1())` | Use actual view context call. Remove DIVERGED. |
| D35 | emStocksItemChart.rs | 920-922 | PaintParams half of CalculateYScaleLevelRange DIVERGED | Replace PaintParams value with `ViewToPanelDeltaY(14.0)` call. Remove PaintParams part. Tuple-return part becomes idiom I8 (final pass). |

### 2.3 emBorder / panel lifecycle

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D29 | emStocksItemChart.rs | 34-35 | `DIVERGED: No emBorder/emPanel inheritance.` | ItemChart implements `PanelBehavior`, wraps in emBorder. Remove DIVERGED. |
| D30 | emStocksItemChart.rs | 216-218 | `DIVERGED: No IsViewed() check` | Use `PanelState::viewed`. Remove DIVERGED. |
| D31 | emStocksItemChart.rs | 423-424 | `DIVERGED: No GetContentRect() call` | Use `emBorder::GetContentRect()`. Remove DIVERGED. |

**Phase 2 totals:** 8 DIVERGED resolved, 2 TODOs resolved (1 DIVERGED deferred, 1 TODO deferred to Phase 4).

---

## Phase 3 — Widget Toolkit Wiring

**Dependencies:** Phase 2 complete.
**Files touched:** `emStocksListBox.rs`, `emStocksControlPanel.rs`, `emStocksItemPanel.rs`.
**Gate:** `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr` pass.

### 3.1 emListBox selection API (emStocksListBox.rs)

Replace local `selected_indices`/`active_index` tracking with real emListBox selection API.

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D1 | emStocksListBox.rs | 12-13 | `DIVERGED: Data model only — widget and panel infrastructure deferred.` | emStocksListBox wraps/extends emListBox as a real panel. Remove DIVERGED. |
| D2 | emStocksListBox.rs | 21-22 | `DIVERGED: C++ uses emListBox selection API. Rust tracks locally.` | Remove `selected_indices`. Delegate to `emListBox`. Remove DIVERGED. |
| D3 | emStocksListBox.rs | 26 | `DIVERGED: C++ uses panel active-path. Rust tracks locally.` | Remove `active_index`. Use panel active-path. Remove DIVERGED. |
| D4 | emStocksListBox.rs | 49 | `DIVERGED: C++ uses emListBox::GetSelectionCount()` | Delegate. Remove DIVERGED. |
| D5 | emStocksListBox.rs | 55 | `DIVERGED: C++ uses emListBox::IsSelected()` | Delegate. Remove DIVERGED. |
| D6 | emStocksListBox.rs | 61 | `DIVERGED: C++ uses emListBox::Select()` | Delegate. Remove DIVERGED. |
| D7 | emStocksListBox.rs | 69 | `DIVERGED: C++ uses emListBox::ClearSelection()` | Delegate. Remove DIVERGED. |
| D8 | emStocksListBox.rs | 75 | `DIVERGED: C++ uses emListBox::SetSelectedIndex()` | Delegate. Remove DIVERGED. |

### 3.2 PaintTextBoxed in ListBox

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D11 | emStocksListBox.rs | 325-326 | `DIVERGED: C++ calls emPainter::PaintTextBoxed. Rust returns the text.` | Call `painter.PaintTextBoxed(...)` directly. Change return from `Option<&str>` to `()`. Remove DIVERGED. |

### 3.3 Clipboard/browser in ListBox

Phase 0 added clipboard and browser crates. Wire them into ListBox methods. Dialog `ask` parameters deferred to Phase 4.

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D13 | emStocksListBox.rs | 373-374 | `DIVERGED: C++ copies to system clipboard. Rust returns string.` | Call system clipboard directly. Return `()`. Remove DIVERGED. |
| D15 | emStocksListBox.rs | 422-425 | `DIVERGED: ask param + returns clipboard string` | Wire clipboard. Remove clipboard DIVERGED. `ask` deferred: update to `DIVERGED(Phase 4): dialog ask parameter pending.` |
| D16 | emStocksListBox.rs | 439-441 | `DIVERGED: takes clipboard text as parameter` | Read from system clipboard. Remove clipboard-text param. `ask` deferred: update to `DIVERGED(Phase 4): dialog ask parameter pending.` |
| D19 | emStocksListBox.rs | 512-513 | `DIVERGED: C++ launches web browser. Rust returns URLs.` | Call `open::that()`. Remove DIVERGED. |

### 3.4 ControlPanel widget tree (emStocksControlPanel.rs)

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D20 | emStocksControlPanel.rs | 18-19 | `DIVERGED: Data model only — emFileSelectionBox deferred.` | Create real `emFileSelectionBox` child panel. Remove DIVERGED. |
| D21 | emStocksControlPanel.rs | 60-61 | `DIVERGED: Stub — actual widget creation deferred.` | Create real child widgets in category panels. Remove DIVERGED. |
| D22 | emStocksControlPanel.rs | 103-105 | `DIVERGED: Data model only — actual GUI widget types not yet implemented.` | Replace `Option<T>` data fields with real widget panel children. Remove DIVERGED. |
| D23 | emStocksControlPanel.rs | 245-246 | `DIVERGED: Data model only — widget layout and signal handling deferred.` | Wire AutoExpand/AutoShrink to create/destroy real widget children. Remove DIVERGED. |

### 3.5 ItemPanel widget tree (emStocksItemPanel.rs)

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D39 | emStocksItemPanel.rs | 6-8 | `DIVERGED: Data model with widget data fields — actual widget creation deferred.` | Create real widget children. Remove DIVERGED. |
| D40 | emStocksItemPanel.rs | 24 | `DIVERGED: Data model only — actual widget creation deferred.` | CategoryPanel creates real child widgets. Remove DIVERGED. |
| D41 | emStocksItemPanel.rs | 43-44 | `DIVERGED: Widget data fields as plain values instead of widget pointers.` | Replace with real widget panel references. Remove DIVERGED. |
| D42 | emStocksItemPanel.rs | 159-160 | `DIVERGED: Data model — widget creation deferred.` | emStocksItemPanel is a real panel with children. Remove DIVERGED. |
| D43 | emStocksItemPanel.rs | 169-170 | `DIVERGED: Widget data in struct instead of widget pointers.` | Replace `ItemWidgets` with real panel children. Remove DIVERGED. |
| D44 | emStocksItemPanel.rs | 219 | `DIVERGED: Creates ItemWidgets struct instead of widget tree.` | AutoExpand creates real widget children. Remove DIVERGED. |

**Phase 3 totals:** 20 DIVERGED resolved, 4 DIVERGED partially resolved (dialog `ask` half deferred to Phase 4).

---

## Phase 4 — Integrated Features

**Dependencies:** Phases 0-3 complete.
**Files touched:** `emStocksFileModel.rs`, `emStocksFilePanel.rs`, `emStocksListBox.rs`, `emStocksPricesFetcher.rs`, `emStocksControlPanel.rs`, `emStocksItemPanel.rs`.
**Gate:** `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr` pass.

### 4.1 emStocksFileModel full integration

FilePanel stops owning `emStocksRec` directly and instead owns `emStocksFileModel`, which owns the rec and provides signals.

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D25 | emStocksFileModel.rs | 16 | `DIVERGED: C++ forward declaration replaced by empty struct placeholder.` | Replace placeholder with real `emStocksFetchPricesDialog` reference. Remove DIVERGED. |
| D26 | emStocksFileModel.rs | 20-23 | `DIVERGED: Composition instead of MI. Save timer uses Instant instead of emTimer.` | Replace `Instant` with `emTimer`. Composition stays (becomes idiom I9). |
| D27 | emStocksFilePanel.rs | 19-21 | `DIVERGED(Phase 4): FileModel ownership pending.` (deferred from Phase 2) | FilePanel owns `emStocksFileModel`. Children access rec via model reference. Remove DIVERGED. |

### 4.2 "Takes rec parameter" collapse

Once FilePanel owns FileModel, all methods that take `rec`/`config` parameters read from the model instead.

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D9 | emStocksListBox.rs | 110-111 | `DIVERGED: takes rec parameter` | Read from owned FileModel reference. Remove DIVERGED. |
| D10 | emStocksListBox.rs | 120-121 | `DIVERGED: takes rec parameter` | Same. Remove DIVERGED. |
| D12 | emStocksListBox.rs | 340-341 | `DIVERGED: takes rec and config parameters` | Same. Remove DIVERGED. |
| D14 | emStocksListBox.rs | 395-398 | `DIVERGED: takes rec + ask param` | Read from model. Wire `ask` to `emDialog`. Remove DIVERGED. |
| D15 | emStocksListBox.rs | 422-425 | `DIVERGED(Phase 4): dialog ask parameter pending.` | Wire `ask` to `emDialog`. Remove DIVERGED. |
| D16 | emStocksListBox.rs | 439-441 | `DIVERGED(Phase 4): dialog ask parameter pending.` | Wire `ask` to `emDialog`. Remove DIVERGED. |
| D17 | emStocksListBox.rs | 480-481 | `DIVERGED: takes rec parameter` | Read from model. Remove DIVERGED. |
| D18 | emStocksListBox.rs | 493-495 | `DIVERGED: takes rec + ask param` | Read from model. Wire dialog. Remove DIVERGED. |
| D24 | emStocksControlPanel.rs | 288-289 | `DIVERGED: takes explicit parameters` | Read from owned references. Remove DIVERGED. |
| D45 | emStocksItemPanel.rs | 270-271 | `DIVERGED: takes stock and selected_date as parameters` | Read from model/parent. Remove DIVERGED. |

### 4.3 PricesFetcher FileModel integration

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| D-hdr2 | emStocksPricesFetcher.rs | 3 | `DIVERGED(Phase 4): FileModel/FileStateSignal/ChangeSignal integration pending.` | Wire Cycle to check file state via FileModel. Remove DIVERGED. |
| D46 | emStocksPricesFetcher.rs | 267-269 | `DIVERGED: ListBox date-selection update skipped` | Wire date-selection update through ListBox. Remove DIVERGED. |
| D47 | emStocksPricesFetcher.rs | 290-291 | `DIVERGED: No FileModel file-state check` | Add file-state guard in Cycle. Remove DIVERGED. |

### 4.4 Dialog confirmation flow

Wire `emDialog` for the 4 operations with `ask` parameters:

- `DeleteStocks(ask)` — confirmation dialog before delete
- `CutStocks(ask)` — confirmation dialog before cut
- `PasteStocks(ask)` — confirmation dialog before paste
- `SetInterest(ask)` — confirmation dialog before interest change

### 4.5 Fetch dialog + ListBox as panel child

| ID | File | Line | Current | Resolution |
|----|------|------|---------|------------|
| T7 | emStocksFilePanel.rs | 181 | `TODO(Phase 4): create real emStocksFetchPricesDialog` | Create real modal dialog child. Remove TODO. |
| T9 | emStocksFilePanel.rs | 267 | `TODO(Phase 4): ListBox as real panel child.` | ListBox is a real panel child. `LayoutChildren` positions it. Remove TODO. |

**Phase 4 totals:** 15 DIVERGED resolved, 1 DIVERGED updated (composition → idiom I9), 2 TODOs resolved.

---

## Final Pass — Idiom DIVERGED Updates

**Dependencies:** All phases complete.
**Files touched:** `emStocksRec.rs`, `emStocksItemChart.rs`, `emStocksPricesFetcher.rs`, `emStocksFileModel.rs`.
**No code changes — comment text only.**

Update all 9 permanent idiom DIVERGED comments (I1-I9) with explicit justifications as specified in the Layer 0 Idiom Divergences table above.

**Gate:** `cargo check`.

---

## Summary Scorecard

| Phase | DIVERGED resolved | DIVERGED updated | TODOs resolved | TODOs deferred |
|-------|-------------------|------------------|----------------|----------------|
| 0 | 0 | 0 | 6 | 1→Phase 4 |
| 1 | 3 | 0 | 1 | 0 |
| 2 | 8 | 0 | 2 | 1→Phase 4 |
| 3 | 20 | 0 | 0 | 0 |
| 4 | 15 | 1 | 2 | 0 |
| Final | 0 | 8 | 0 | 0 |
| **Total** | **47** | **9** | **11** | **0** |

**End state:** Zero TODOs. Zero deferred DIVERGED. 9 DIVERGED comments remain permanently with idiomatic-Rust justifications. All 56 unique DIVERGED comment blocks accounted for (47 resolved + 9 updated; D15, D16, D27, D-hdr2 appear in two phases due to partial resolution).

## Constraints

- Each phase gates on `cargo check` + `cargo clippy -- -D warnings` + `cargo-nextest ntr`.
- No `#[allow(...)]` / `#[expect(...)]` except per CLAUDE.md exemptions.
- No `unsafe`, `Arc`, `Mutex`, `Cow`, glob imports.
- File and Name Correspondence rules apply to all new code.
- Port Fidelity rules apply: pixel arithmetic exact, geometry same algorithm, state logic idiomatic Rust.
