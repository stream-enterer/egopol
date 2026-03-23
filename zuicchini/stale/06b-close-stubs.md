# Phase 6b: Close Category A Stubs

Implement all 25 Category A stubs across 3 files. Read the diagnosis at `state/run_003/stub_diagnosis.json` for exact line numbers and C++ source references.

**Port fidelity rules:** Image methods involving pixel arithmetic (fill, copy, interpolation, transforms) are Domain 1 — port C++ integer arithmetic exactly. ListBox and TextField methods are Domain 3/5 — idiomatic Rust, preserve behavioral contract.

## File 1: Image (`src/foundation/image.rs`) — 10 stubs

Work in this order (dependencies flow downward):

### Low difficulty (6 items)

1. **fill** (line 117) — Remove `assert_eq!(self.channel_count, 4)`. Add match on channel_count (1-4) to decompose color into the appropriate bytes. C++ lines 494-591.

2. **fill_rect** (line 156) — Same fix as fill. Remove 4-channel assertion, add channel_count match. C++ lines 494-591.

3. **calc_min_max_rect** (line 404) — Remove early return for non-4-channel. Decompose background color per channel_count (1: grey, 2: grey+alpha, 3: rgb, 4: rgba). C++ lines 1186-1228.

4. **determine_all_colors_sorted** (line 774) — Add `limit: usize` parameter. Return empty vec if unique colors exceed limit. Handle all channel counts. C++ lines 1398-1432.

5. **get_cropped** (line 265) — Add optional `channel_count: Option<u8>` parameter. If Some, convert channels during crop. If None, preserve. C++ lines 1311-1332.

6. **get_cropped_by_alpha** (line 365) — Pass channel_count through to get_cropped. C++ lines 1335-1341.

### Medium difficulty (3 items)

7. **copy_from_rect** (line 208) — Remove `assert_eq!(self.channel_count, src.channel_count)`. Implement all 16 src/dst channel_count combinations (1→1, 1→2, ..., 4→4) with appropriate color conversion. C++ lines 628-823, uses EM_IMG_COPY_LOOP macro. Domain 1: port the conversion formulas exactly.

8. **copy_transformed** (line 500) — Remove 4-channel assertions. Support all src/dst channel_count combinations in the affine transform loop. C++ lines 886-1145. Domain 1: port the interpolation and channel conversion exactly.

9. **try_parse_xpm** (line 749) — Full XPM parser. Read C++ lines 111-282 carefully. Parse header (width, height, num_colors, sym_size), build color table from color entries (key types c/g/g4/m/s), map pixel symbols to colors. Uses Color::try_parse for X11 color names — check if this exists in Rust. If not, implement basic hex color parsing and named color lookup.

### High difficulty (1 item)

10. **get_pixel_interpolated** (line 471) — Implement area-sampling path when w >= 1.0 or h >= 1.0. C++ lines 389-491. Currently only does bilinear (ignoring w/h params). The area-sampling iterates over all covered pixels with fractional edge weights. **Domain 1: port the C++ integer arithmetic exactly.** This feeds the rendering pipeline.

## File 2: ListBox (`src/widget/list_box.rs`) — 7 stubs

### Low difficulty (5 items)

1. **scroll_to_index** (line 1326) — Implement scroll-down case. Currently only scrolls up. C++ lines 915-916. Use the existing `item_bottom` variable that's currently suppressed with `let _ =`.

2. **ListBox::paint inline path** (line 891) — Add ReadOnly/disabled color logic. When selection_mode is ReadOnly, use `look.output_bg_color`/`output_fg_color`/`output_hl_color`. When disabled, blend colors at 80%. C++ lines 562-577.

3. **input / select_by_input** (line 983) — Add enabled check. Return early without processing input if widget is disabled. C++ line 790.

4. **keywalk_search** (line 1260) — Replace `str::contains()` with subsequence matching for `*`-prefixed search. C++ lines 874-908: advance through haystack character by character, reset needle index on mismatch.

5. **Notice / focus change handler** — Add focus-loss handling. When panel leaves focused path, clear `keywalk_chars`. C++ lines 647-656. Implement via the existing notice or cycle mechanism.

### Medium difficulty (2 items)

6. **DefaultItemPanelBehavior::paint** (line 144) — Add ReadOnly/disabled color logic matching the inline path fix above. Needs access to selection mode and enabled state from the parent ListBox. C++ lines 554-608.

7. **auto_expand_items / auto_shrink_items** (line 817) — Add base expand/shrink calls. Validate that the item panel interface exists (log error if missing, don't fatal). C++ lines 686-718.

## File 3: TextField (`src/widget/text_field.rs`) — 4 stubs

### Low difficulty (2 items)

1. **paint cursor transparency** (line 1092) — Add transparency levels: 75% alpha for read-only cursor, 88% alpha for blink-off state. C++ lines 1057-1059, uses `GetTransparented()`.

2. **paint output field colors** (line 1073) — When field is not editable, use `look.output_bg_color`/`output_fg_color`/`output_hl_color` instead of input colors. C++ lines 956-965, branches on `IsEditable()`.

### Medium difficulty (2 items)

3. **paint cursor shape** (line 1096) — Replace rectangle cursor with I-beam polygon (8 vertices with serifs) for insert mode and frame polygon (10 vertices with inner cutout) for overwrite mode. C++ lines 1056-1091. Use the existing `painter.paint_polygon()` API.

4. **paint multi-line selection** (line 1169) — Replace per-row rectangles with a single 8-vertex polygon spanning row boundaries. C++ lines 976-991. Use `painter.paint_polygon()`.

## Per-File Procedure

For each file (Image, ListBox, TextField):

1. Read `state/run_003/stub_diagnosis.json` for exact line numbers and C++ references.
2. Read the C++ source for each stub method.
3. Implement all stubs in the file, low difficulty first.
4. Run `cargo check --workspace` after each method. Fix until clean.
5. Run `cargo test --workspace` after completing all stubs in the file. Record any failures.
6. Commit per file:
   - `fix(CAP-0033): close 10 Image stubs — multi-channel support, XPM parser, area sampling`
   - `fix(CAP-0041): close 7 ListBox stubs — disabled state, scroll-down, keywalk, focus cleanup`
   - `fix(CAP-0069): close 4 TextField stubs — cursor shape, selection polygon, output colors`

## Rules

- Domain 1 methods (Image pixel arithmetic): port C++ integer formulas exactly. Do not use f64 approximations.
- Domain 3/5 methods (ListBox, TextField behavior): idiomatic Rust, preserve behavioral contract.
- Do not modify existing tests. If an existing test breaks, the stub fix introduced a regression — revert that specific method and log the issue.
- If a stub depends on an API that doesn't exist in the Rust codebase (e.g., Color::try_parse for XPM), check if it exists first. If not, implement the minimum needed or skip that one stub and log why.
- If `cargo check` fails 5 times on one stub, skip it and move on. Log the error.

## After All Three Files

1. Run `cargo test --workspace`. Report total count, pass count, failures.
2. Run `MEASURE_DIVERGENCE=1 cargo test --test golden 2>&1`. Report whether any golden test divergence changed (better or worse). The ListBox and TextField paint changes may affect widget golden tests.
