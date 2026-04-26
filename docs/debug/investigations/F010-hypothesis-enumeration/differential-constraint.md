# F010 differential constraint enumeration — property matrix

Goal: enumerate paint-primitive properties whose values differ between the working
primitive (Y = `PaintBorderImage`) and the failing primitives
(X = `Clear`, Z = `PaintText` + ancillary `PaintRect`). Each discriminating
property becomes a hypothesis category for the synthesizer.

All citations use absolute paths to the file
`/home/alex/Projects/eaglemode-rs/crates/emcore/src/emPainter.rs` unless
otherwise noted; line numbers come from the Read calls in this session.

## Property matrix

| Property | Clear | PaintRect | PaintText | PaintBorderImage |
| --- | --- | --- | --- | --- |
| P1: Recording-mode dispatch (calls `try_record`) | NO — only `require_direct` (`emPainter.rs:5775-5783`) | YES (`emPainter.rs:980-989`) | YES (`emPainter.rs:3736-3746`) | YES (delegates to `PaintBorderImageSrcRect` which records, `emPainter.rs:4420-4441`, recorder at `4476-4495`) |
| P2: Has a corresponding `DrawOp::*` variant (so it survives the record→replay round trip) | NO — no variant exists in `emPainterDrawList.rs` enum (lines 14, 35, 157, 177, 446-964 list every variant; no `Clear`) | YES — `DrawOp::PaintRect` (`emPainterDrawList.rs:35`) | YES — `DrawOp::PaintText` (`emPainterDrawList.rs:177`) | YES — `DrawOp::PaintBorderImage` (`emPainterDrawList.rs:157`) |
| P3: Behavior in compositor recording mode (when `target == PaintTarget::DrawList`) | Silent early-return at `emPainter.rs:5776-5778` (no op recorded, no pixels) | Op recorded (`emPainter.rs:980`); rest of body returns at `require_direct` check `emPainter.rs:1052-1058` | Op recorded; sub-ops gated by `record_subops` flag (`emPainter.rs:3747-3749`) | Op recorded; sub-ops gated by same flag (`emPainter.rs:4496-4498`) |
| P4: Pixel emission kind | Solid fill (`fill_rect_pixels`, `emPainter.rs:5783`) | Solid scanline fill with edge antialiasing (`paint_rect_scanline`, `emPainter.rs:1059-1069`) | Glyph-atlas texture sampling (`PaintImageColored` per char, `emPainter.rs:3793-3802`) | 9-slice texture sampling (`PaintImageSrcRect` per slice, `emPainter.rs:4527-4574`) |
| P5: Reads `state.canvas_color` (carrier) | Argument-less; uses whatever canvas_color is on the state at call time | Saves/sets/restores `state.canvas_color = canvas_color` arg (`emPainter.rs:991-992, 1089`) | `canvas_color` arg is forwarded to `PaintImageColored` glyph calls (`emPainter.rs:3793, 3802` block) | `canvas_color` arg drives RoundX pixel-snap branch (`emPainter.rs:4504-4521`) and is forwarded to each slice |
| P6: Source of the color it writes | `color` arg (light grey `DirContentColor`) | `color` arg, alpha-blended with existing pixel | Glyph atlas texture × `color` arg (modulated) | Source image RGBA, possibly modulated by `alpha` arg only |
| P7: Geometry source for output region | `state.clip` rectangle (full clip area, `emPainter.rs:5779-5782`) | Logical (x,y,w,h) transformed via `state.scale_x/y` + `offset_x/y` (`emPainter.rs:995-1018`) | Per-character cursor stepping in user coords (`emPainter.rs:3775-3791`) | 9-slice rectangles in user coords; 4 corner + 4 edge + 1 centre (`emPainter.rs:4527-4574+`) |
| P8: Depends on font cache | NO | NO | YES — `emFontCache::atlas()` (`emPainter.rs:3773`) and `GetChar` (`emPainter.rs:3787`) | NO |
| P9: Depends on caller-supplied image | NO | NO | YES (font atlas image, internally) | YES (`image: &emImage` arg) |
| P10: `record_subops` interaction | None (never reaches recording path) | None — body still runs to `require_direct` and returns there | YES — sub-ops suppressed unless `record_subops` (`emPainter.rs:3747-3749`); only the parent `DrawOp::PaintText` is recorded | YES — same gating (`emPainter.rs:4496-4498`) |
| P11: Replay-side handler in `DrawList::replay` | NONE (no variant) | `painter.PaintRect(...)` (`emPainterDrawList.rs:460-467`) | `painter.PaintText(...)` (`emPainterDrawList.rs:678-695`) | `painter.PaintBorderImage(...)` (`emPainterDrawList.rs:639-677`) |
| P12: Direct-only? (works only when `PaintTarget::emImage`) | YES — `require_direct` at `emPainter.rs:679-684`, `5776` | NO — has both record path (no-op pixel) and direct path | NO | NO |

## Discrimination analysis

A property discriminates iff `PaintBorderImage` value differs from at least one of
{`Clear`, `PaintRect`, `PaintText`}.

| Property | Discriminates Y from X+Z? | Notes |
| --- | --- | --- |
| P1 (try_record) | PARTIAL — partitions {Y, PaintRect, PaintText} from {Clear} only. PaintRect (Z's small rects) DOES record. | Cleanly explains X but not all of Z. |
| P2 (has DrawOp variant) | PARTIAL — same partition as P1: only `Clear` lacks a variant. | Same as P1; mechanically inseparable. |
| P3 (behavior in recording mode) | YES — `Clear` returns silently with no recorded op; the others record. | Combined with P11, this is the only property that gives `Clear` a degenerate state. |
| P4 (pixel emission kind) | YES — Y samples a 9-slice texture; Clear/PaintRect emit solids; PaintText samples glyph atlas. But the partition is messy — solid vs textured does NOT cleanly separate Y from X+Z (PaintText is textured like Y). | Doesn't cleanly explain why Z fails when its texture path is the same shape as Y's. |
| P5 (canvas_color carrier read) | NO clean partition — all four read/use canvas_color in some way. | Not discriminating in itself, but the `canvas_color` value at recording time may differ between primitives (snapshot timing — see P14, P15). |
| P6 (color source) | NO clean partition. | Y uses image RGBA + alpha; X uses `color`; PaintRect uses `color`; PaintText uses atlas × color. Not a 1-step partition. |
| P7 (geometry source) | NO — every primitive ultimately resolves to pixel rects after transform. | |
| P8 (font cache dep) | PARTIAL — only PaintText. Doesn't explain X. | If font atlas were uninitialized in compositor, only Z would fail; X would still fail by another cause. Plausible co-explanation. |
| P9 (image arg dep) | PARTIAL — Y and PaintText depend on textures; X does not. | Same partial split as P4. |
| P10 (record_subops) | YES — `Clear` doesn't touch it; `PaintRect` doesn't touch it; `PaintText` & `PaintBorderImage` both gate on it. | But Y works while PaintText fails despite IDENTICAL gating logic. So `record_subops` is not the discriminator unless something downstream of the gate differs. |
| P11 (replay handler) | YES — there is no replay handler for `Clear`. | Pairs with P3: Clear is dropped on the floor at record time, so replay has nothing to do. |
| P12 (direct-only) | YES — `Clear` is the only primitive that early-returns under `PaintTarget::DrawList`. | Cleanest single-property partition for the X side. |

## Iteration 2 — added properties

Properties I missed on the first pass:

| Property | Clear | PaintRect | PaintText | PaintBorderImage |
| --- | --- | --- | --- | --- |
| P13: State snapshot captured at record time (the `RecordedOp.state` at `emPainter.rs:669`) used by replay | N/A (not recorded) | YES — `state_snapshot()` taken before push to ops (`emPainter.rs:662-673`) | YES — same | YES — same |
| P14: Replay-time state restoration: replay reads `RecordedOp.state` and re-applies it before invoking the painter method | N/A | YES — see `DrawList::replay` (`emPainterDrawList.rs:438+`) | YES | YES |
| P15: Op carries `canvas_color` parameter explicitly (vs. relying on snapshot state) | N/A | YES — `DrawOp::PaintRect.canvas_color` (`emPainterDrawList.rs:35`) | YES — `DrawOp::PaintText.canvas_color` (`emPainterDrawList.rs:177`) | YES — `DrawOp::PaintBorderImage.canvas_color` (`emPainterDrawList.rs:157`) |
| P16: Tile pre-fill seeds the destination pixels before replay | dest is `bg` (`emViewRenderer.rs:99` `buf.fill(bg)`) — Clear writes are absent so pixel = bg = solid black (the renderer pre-fills with `background_color`) | dest is bg; PaintRect blends/overwrites on top | dest is bg; if a PaintRect-Z fill happened first, glyphs paint on that; if not, glyphs paint on bg | dest is bg; image samples overwrite |
| P17: Output is opaque or transparent in source | Opaque (light grey) | Opaque (varies) | Opaque glyph (varies); transparent between glyphs | Mixed — slices have transparent edges (gradients) |
| P18: Z's missing-text could be "rendered correctly on top of black background", appearing identical to "missing entirely" | The fact that DirContentColor (light grey) is also missing means the BACKGROUND on which Z paints is wrong, not Z itself | — | If Z's PaintText paints onto a black tile (because Clear was a no-op), foreground-color text might still be visible — UNLESS the text color is also dark, OR text colors derived from a panel background that isn't there | — |
| P19: Render strategy independence | renders correctly under direct rendering (no DrawList) — see `PaintTarget::emImage` arm of `require_direct` returning `Some` (`emPainter.rs:680-682`) | works in both modes | works in both modes | works in both modes |

Updated discrimination check:

| Property | Discriminates Y from X+Z? | Notes |
| --- | --- | --- |
| P13 | NO partition for Y vs X+Z (snapshot uniformly applies); discriminates Clear from all (no snapshot at all) | Same axis as P3/P11. |
| P14 | Same. | |
| P15 | NO clean partition. | But could explain a stale-snapshot variant of canvas_color (see Cat 2 below). |
| P16 | YES — explains X. The framebuffer/tile pre-fill is `bg` (likely black per `emViewRenderer.rs:85` `bg = self.background_color`); since Clear is a no-op, X's pixels stay at bg. | This is a *consequence* of P3/P12 rather than an independent root cause. |
| P17 | NO clean partition. | |
| P18 | KEY OBSERVATION — Z may not be "failing to render"; Z may be rendering invisibly because the background under it is wrong. This means Z is not necessarily an independent failure — it may be downstream of X. | If true, the only true discriminating property is P3/P12 (Clear is a no-op), and Z's apparent failure is a secondary effect of X. |
| P19 | YES — Clear has correct direct-render path; only the recording path is broken. | Reinforces P3/P12. |

## Iteration 3 — additional properties

| Property | Clear | PaintRect | PaintText | PaintBorderImage |
| --- | --- | --- | --- | --- |
| P20: Op log function (`op_log_fn`) sees this op? | NO (never recorded → no log call) | YES (`emPainter.rs:665-667`) | YES | YES |
| P21: Threading model interaction (replayed in parallel tile workers, `emViewRenderer.rs:92-108`) | N/A | parallel-safe | parallel-safe | parallel-safe |
| P22: Image lifetime through record→replay (raw pointer semantics for `&emImage`) | N/A | N/A | font atlas is a static singleton — always live | Caller's `emImage` ref stored as `*const emImage` — must outlive replay (memory observation 79) |

P20-P22 don't introduce new clean Y vs X+Z partitions beyond what P3/P12 already gave.

## Properties considered and rejected as non-discriminating

- P5 (canvas_color carrier read), P6 (color source), P7 (geometry source),
  P13–P15 (snapshot/replay state plumbing), P17 (opaque vs transparent
  output), P20–P22 (logging/threading/image lifetime). None of these
  partition `{PaintBorderImage}` from `{Clear, PaintRect, PaintText}` in a
  way that uniquely picks the failing set.

## Derived hypothesis categories (ranked)

### Category 1 (top-ranked): Recording-mode dispatch is broken for primitives without a `DrawOp` variant
- **Distinguishing property**: P1 + P2 + P3 + P11 + P12 — `Clear` is the *only* paint primitive among the four that uses `require_direct` rather than `try_record`, has no `DrawOp::*` variant, and therefore early-returns to a no-op when the painter target is `PaintTarget::DrawList` (recording mode used by the parallel tile compositor at `emViewRenderer.rs:74`).
- **Failure scenario**: When `render_parallel` records into a `DrawList`, every `Clear` call in panel paint code silently does nothing. Tiles are pre-filled with `background_color` (`emViewRenderer.rs:99`); since the panel-interior fill is missing, the user sees the pre-fill (black) where the panel interior should have been DirContentColor light-grey. Z then paints text onto this black background; if text color is itself dark or relies on contrast against DirContentColor, Z appears invisible (P18).
- **Why this ranks first**: it is the *only* property in the matrix that produces a clean degenerate value for X (Clear) — every other primitive records or replays normally. If we accept that Z's invisibility is a secondary consequence of X's missing background (P18), this single property explains the whole pattern.
- **Code citations**: `emPainter.rs:679-684` (`require_direct`), `emPainter.rs:5775-5783` (Clear body), `emPainterDrawList.rs:14-440` (DrawOp enum + replay match has no Clear arm), `emViewRenderer.rs:71-108` (record/replay flow), `emViewRenderer.rs:99` (tile pre-fill).

### Category 2: Texture-sampling primitives misbehave under recording (font-atlas / image-sampling specific)
- **Distinguishing property**: P4 + P8 + P9 — Y and PaintText both sample textures, yet only PaintText fails. If recording-mode reproduction of texture-sampling primitives has a bug, it must be one that affects glyph-atlas sampling but not 9-slice border-image sampling.
- **Failure scenario**: PaintText records `DrawOp::PaintText` but at replay time the font atlas reference, font cache initialization, or glyph atlas pixel data is wrong (e.g. atlas is per-thread and unpopulated in worker threads; or atlas pointer captured at record time is invalid in the worker context). PaintBorderImage's image is supplied by the caller and stored as `*const emImage` (P22) which the parent panel keeps alive across replay; the font atlas may have a different lifetime/initialization story.
- **Why this ranks below Cat 1**: it requires a bug specific to glyph atlas access at replay time that does not affect border-image access. It also does not explain X (Clear) failing — Cat 2 would need to be paired with Cat 1, in which case Cat 1 is the simpler explanation for both.
- **Code citations**: `emPainter.rs:3773` (`emFontCache::atlas()`), `emPainter.rs:3787` (`GetChar`), `emPainter.rs:3793-3802` (PaintImageColored on atlas), contrast with `emPainter.rs:4527-4574` (PaintImageSrcRect on caller image).

### Category 3: Sub-op recording (`record_subops`) interaction
- **Distinguishing property**: P10 — `Clear` and `PaintRect` are leaf primitives with no sub-ops; `PaintText` and `PaintBorderImage` both have a `record_subops` gate that returns early in recording mode unless explicitly enabled.
- **Failure scenario**: If sub-op recording were the bug, both PaintText and PaintBorderImage would behave identically; but they don't (Y works, Z fails). So the gate itself is not the bug. Possible refinement: the *replay* path differs in how it re-enters PaintText vs PaintBorderImage, and one path's re-entry produces different sub-op behavior. Inspection of the replay handlers (`emPainterDrawList.rs:639-695`) shows both call back into the corresponding `painter.PaintXxx(...)` method with the same parameters.
- **Why this ranks below Cat 2**: the property explicitly fails the discrimination test for {Y vs PaintText} on its own; it can only contribute as a co-factor.
- **Code citations**: `emPainter.rs:3747-3749`, `emPainter.rs:4496-4498`, `emPainter.rs:486, 606`, `emPainterDrawList.rs:639-695`.

### Category 4: Tile pre-fill / background composition
- **Distinguishing property**: P16 — only X relies on the destination starting as DirContentColor (i.e. relies on the panel interior being filled). Y is opaque enough that pre-fill doesn't matter; Z is foreground-on-(intended-)background.
- **Failure scenario**: Even if every `Clear` call were faithfully recorded and replayed, if the tile pre-fill uses a wrong background color (e.g. the renderer's `background_color` field is wrong) and `Clear`'s pixel writes happen before/after some other operation that overwrites them, the panel interior would still come out wrong. `emViewRenderer.rs:99` pre-fills with `bg = self.background_color`; if `background_color` is opaque black and `Clear` is missing, pre-fill wins.
- **Why this ranks below Cat 1**: it's downstream of Cat 1. Pre-fill being black would only matter because Clear is missing; if Clear were correctly applied, pre-fill colour would be invisible.
- **Code citations**: `emViewRenderer.rs:85, 99, 111`.

## Iteration log
- **First-pass property list**: 12 (P1–P12).
- **Properties added in iteration 2**: P13 (state snapshot), P14 (replay state restoration), P15 (canvas_color carried explicitly), P16 (tile pre-fill seeds destination), P17 (opaque vs transparent), P18 (Z appearing invisible may be downstream of X), P19 (per-render-strategy independence). 7 added.
- **Properties added in iteration 3**: P20 (op log fn), P21 (threading), P22 (image lifetime / raw pointer). 3 added.
- **Properties considered and rejected as non-discriminating**: P5, P6, P7, P13, P14, P15, P17, P20, P21, P22.

## Open questions for the synthesizer
- Is Z's apparent failure independently rooted, or is it a downstream visual consequence of X (per P18)? If the panel-interior background were correctly painted DirContentColor light-grey, would Z's text become visible without any code change? This determines whether Cat 1 alone suffices or whether we need a separate Cat 2-style hypothesis.
- Does the C++ reference treat `Clear` as a paintable op (i.e. does it have an analogous record/replay design)? The Port Ideology demands C++ ground truth here; if upstream C++ has a recording analogue for Clear that Rust dropped, this is fidelity-bug rather than upstream-gap-forced.
- For Cat 2 testing: is the font cache atlas a process-wide singleton or per-thread? `emFontCache::atlas()` (`emPainter.rs:3773`) — its threading story decides whether glyph-atlas sampling can fail in the parallel replay worker context independent of Cat 1.
- Property P15: although every recorded DrawOp carries `canvas_color` explicitly, is there a code path where the recording-time `state.canvas_color` (snapshot, P13) and the op's `canvas_color` argument disagree? PaintRect mutates `state.canvas_color` *after* `try_record` (`emPainter.rs:980-992`) — the snapshot at record time would have the OLD canvas_color, while the op carries the NEW. Replay applies snapshot then calls `painter.PaintRect(..., canvas_color)` which re-mutates correctly — but worth confirming this isn't a class of bug that selectively affects Z's small rects.
