# F010 hypothesis-category synthesis

Synthesizes outputs of:
- `architectural-grounding.md` — projection over paint-pipeline layers and their contracts.
- `differential-constraint.md` — projection over paint-primitive properties that discriminate Y from X+Z.

Mandate: reference every finding from every artifact (omission is failure); classify cross-projection relationships; identify joint blind spots; produce unified hypothesis-category list with explicit provenance. Categories from the prior provisional 7-list are deliberately excluded — that list was the unrigorous baseline this enumeration was designed to defeat.

---

## Step 1: Findings enumeration

### Findings from architectural-grounding.md

Layer table (15 rows). Categories derived under rules R1 (unenforced), R2 (RUST_ONLY), R3 (differs from C++), R4 (enforcement could be wrong):

- **A (R1+R2+R3)** — Layer 6/7a/8: panel `Clear(color)` calls silently dropped in recording mode. Six concrete call sites cited (`emDirPanel.rs:459`, `emDirStatPanel.rs:145`, `emFileLinkPanel.rs:236,295`, `emStarFieldPanel.rs:197`, `emMainPanel.rs:277`, `emStocksFilePanel.rs:41`). C++ panels use two-arg `Clear(c, canvasColor)` which delegates to `PaintRect`.
- **B (R1+R2+R3)** — Layer 1: render-strategy split. Three branches (viewport-buffer / parallel display-list / per-tile direct) at `emWindow.rs:636/661/677`; no equivalence assertion. Only display-list branch uses recording mode.
- **C (R1+R2+R3)** — Layer 2: per-frame `view.background_color` capture and tile pre-fill at `emWindow.rs:620,640,683`, `emViewRendererCompositor.rs:48,99,111`. F018 contract surface.
- **D (R1+R2)** — Layer 14: tile composite re-blending. `BlendState::ALPHA_BLENDING` at `emViewRendererCompositor.rs:99` re-blends per-tile RGBA against bg-clear at composite time. No C++ analogue.
- **E (R2+R3)** — Layer 9: `DrawList::replay` state-snapshot equivalence. `RecordedState` captured at `emPainter.rs:646-658,669,695` but not consulted at replay (`emPainterDrawList.rs:438+`). The snapshot field is dead at replay.
- **F (R2+R3)** — Layer 11: parallel-replay determinism. `unsafe impl Send/Sync` for DrawOp variants holding `*const emImage` at `emPainterDrawList.rs:381,415` is unchecked.
- **G (R2+R3)** — Layers 12/13/15: GPU pipeline. `write_texture` at `emViewRendererCompositor.rs:226-244`; sRGB sampler at `emViewRendererCompositor.rs:174` (`Rgba8UnormSrgb`). Could cause color cast.
- **H (R4)** — Layer 3: `debug_assert!(scale=1)` at `emView.rs:4714` and push/pop pairing at `:4796/4880`. Compiled out in release.
- **I (R4)** — Layer 4: SVP-boundary `tree.IsOpaque(svp_id)` branch correctness vs C++ `emFilePanel.cpp:187-198`.

Open questions raised:
1. Which strategy branch is active when X+Z manifest? Force `GetThreadCount=1`.
2. Why does viewport-buffer branch work? Direct mode.
3. Are there other `require_direct`-without-paired-DrawOp call sites? Verified: `Clear` is the only one.
4. Add `DrawOp::Clear` or migrate panel callers to `ClearWithCanvas`?
5. What is `view.background_color` at the time X reproduces?

Coverage notes record explicit rejections: input event dispatch, engine tick scheduling, panel-tree mutation during paint, color packing format. Files not read fully: `emView.rs` outside `Paint`, `emPainter.rs` outside dual-mode dispatch, `emPainterDrawList.rs` outside replay header, `emWindow.rs` outside `render`, `emGUIFramework.rs` outside wgpu boot.

### Findings from differential-constraint.md

Property matrix (22 rows P1–P22). Discrimination test: property partitions {Y} from {X, Z}.

Discriminating properties:
- **P1** (try_record) — partial: Clear vs others.
- **P2** (DrawOp variant) — partial: Clear vs others.
- **P3** (recording-mode behavior) — clean: Clear silently returns.
- **P4** (pixel emission kind) — partial; texture vs solid does not cleanly split Y from X+Z.
- **P8** (font cache dep) — partial; only PaintText.
- **P9** (image arg dep) — partial; Y + PaintText.
- **P10** (record_subops gate) — explicitly rejected (Y + PaintText share gating).
- **P11** (replay-side handler) — clean: no handler for Clear.
- **P12** (direct-only) — clean: Clear is the only `require_direct` primitive.
- **P16** (tile pre-fill seeds destination) — explains X mechanically; downstream of P3/P12.
- **P18** — KEY: Z's "missing" may be downstream of X (text on un-cleared black tile).
- **P19** (render strategy independence) — Clear works under direct render; recording broken.

Non-discriminating (rejected): P5, P6, P7, P13, P14, P15, P17, P20, P21, P22.

Categories derived:
- **1** (top) — Recording-mode dispatch hole. Provenance P1+P2+P3+P11+P12 + secondary P16/P18.
- **2** — Texture-sampling primitives misbehave at replay (font-atlas specific). Provenance P4+P8+P9.
- **3** — Sub-op recording (record_subops) gate. Rejected as non-discriminating per P10.
- **4** — Tile pre-fill / background composition. Provenance P16; downstream of Cat 1.

Open questions raised:
1. Is Z independent or downstream of X (P18)?
2. Does C++ have a recording analogue for `Clear`?
3. Font cache atlas threading model (process-wide vs per-thread)?
4. P15 canvas_color snapshot/op-arg micro-divergence — confirm not a class of bug affecting Z's small rects.

---

## Step 2–5: Cross-projection classification

### Convergences (same mechanism, both projections, different framings)

| # | Mechanism | Architectural finding | Differential finding | Confidence |
|---|---|---|---|---|
| C1 | `emPainter::Clear` silently dropped in recording mode | Cat A (Layer 6/7a/8) | Cat 1 (P1+P2+P3+P11+P12) | High — both projections converge from independent angles |
| C2 | Tile pre-fill is what shows through when Clear fails | Cat C (Layer 2) | Cat 4 (P16) | High — both flag as downstream/co-factor |
| C3 | Z's invisibility is plausibly downstream of X (text on un-cleared tile) | Cat A failure-scenario notes | P18 + Cat 1 secondary | Medium — both surface as "likely but not proven" |

### Asymmetric findings (visible in one projection, structurally invisible to the other)

| # | Finding | Provenance | Why other projection couldn't surface | Real or framing artifact? |
|---|---|---|---|---|
| A1 | Render-strategy split at `emWindow.rs:636/661/677` — bug fires only in display-list branch | (3) Cat B | (4)'s frame is per-primitive; strategy chooser is upstream of which painter type is created | Real. Diagnostic-critical: tells us under what conditions C1 fires. |
| A2 | Replay-side `RecordedState` snapshot is dead at replay | (3) Cat E | (4) considered P13/P14 but rejected as non-discriminating for Y vs X+Z | Real concern but not an F010 root-cause candidate |
| A3 | `unsafe impl Send/Sync` for DrawOp `*const emImage` variants is unchecked | (3) Cat F | (4)'s frame is per-primitive properties; threading is cross-cutting | Real (low prior for F010) |
| A4 | sRGB encoding (`Rgba8UnormSrgb`) at GPU sampler | (3) Cat G | (4)'s frame ends at PaintTarget; GPU upload is downstream of replay | Real (low prior — Y argues against global GPU bug) |
| A5 | Tile composite re-blending alpha against framebuffer | (3) Cat D | (4)'s frame is per-primitive recording; tile composition is post-replay | Real — could partial-explain Z dimming, not X solid black |
| A6 | Font-cache atlas threading at replay | (4) Cat 2 | (3)'s frame is layer contracts; font cache is internal state of one primitive | Real (low-mid prior) |
| A7 | `canvas_color` snapshot/op-arg disagreement micro-question | (4) P15 / open question | (3) didn't enter intra-primitive detail | Real (low prior, mechanistically narrow) |
| A8 | SVP-boundary `IsOpaque` branch correctness | (3) Cat I | (4) outside primitive-level frame | Real (low prior) |
| A9 | Push/pop pairing & `debug_assert!` compiled out in release | (3) Cat H | (4)'s frame can't see assertion correctness | Real (residual concern only) |

### Contradictions

None identified. Where the projections do not overlap, it is because their frames do not intersect, not because they disagree.

---

## Step 6: Joint blind spots — categories neither projection could structurally surface

Both projections look at the *paint pipeline*. (3) walks layers `emView::Paint` → wgpu present; (4) walks paint primitives in `emPainter`. Categories neither frame can produce:

- **B1: Theme / runtime-data correctness.** Is `DirContentColor` actually a light-grey value at runtime? If theme parse is broken, fixing C1 wouldn't fix the symptom — the panel would Clear to black even with the recording-mode hole closed.
- **B2: Panel state machine reaches gated arm.** `emDirPanel::Paint` Clears only in `VFS_LOADED` / `VFS_NO_FILE_MODEL`. If production state never reaches Loaded (vs. test harness), `Clear` is never even called. F010.md Phase 3 ruled this out at headless scale (RULED OUT for B-load and D-visibility), but the assumption that production reaches Loaded for the symptomatic panel was not re-verified after F018 closure.
- **B3: Paint-not-reached.** Is `emDirPanel::Paint` actually invoked for the symptomatic panel? A parent panel cull, an IsOpaque cascade, or a viewport-clip could prevent the Paint method from running at all.
- **B4: Stale tile cache from prior frame.** The visible black could be a tile painted during a *previous* frame (when state was VFS_WAITING) that was cached and is being re-presented. Neither projection investigates frame-to-frame staleness or cache invalidation.
- **B5: Font cache initialization order.** When does the global font cache become populated relative to first paint? If panel paint runs before fonts are loaded, glyphs may render as empty rectangles (visible as transparent → composited to whatever's below, often black).
- **B6: Build-config-only reproductions.** Does the symptom only appear in debug, or only release, or only with specific feature flags / GPU vendors / DPRs / window scaling factors? Neither projection accounts for environment.
- **B7: Recursive paint invocation.** A panel's Paint can trigger sub-panel Paint. If the recording-painter's internal state (PaintTarget, op vec, depth) is not preserved across recursive entries, sub-panel paints could land on the wrong target.
- **B8: Compositor invalidation timing.** When does a tile become "dirty" and require re-painting? If the dirty-tile criterion misses a state transition (e.g. VFS_WAITING → VFS_LOADED doesn't dirty the tile), the re-paint never runs.

These are the gaps the (5) pre-mortem must attack.

---

## Step 7: Unified hypothesis-category list

Each category cites provenance. Tiers reflect convergence count + mechanistic specificity + coverage of the X+Y+Z pattern.

### Tier 1 — high confidence (cross-projection convergence)

**H1. Recording-mode dispatch hole — `emPainter::Clear` silently dropped**
- Provenance: convergence C1 — (3) Cat A + (4) Cat 1.
- Mechanism: `emPainter::Clear` uses `require_direct` without a paired `DrawOp::Clear`; in `PaintTarget::DrawList` mode it returns silently. Tile pre-fill (`background_color`) shows through at the panel interior.
- Cleanest single-mechanism explanation for X. Z is plausibly downstream (B-tier joint check needed).
- Diagnostic discriminator: force per-tile direct branch (`GetThreadCount=1` or kill `dirty_count > 1` condition) — if X+Z vanish, H1 confirmed; if persist, H1 partial or wrong.

**H2. Tile pre-fill / `background_color` contract — what specifically shows through**
- Provenance: convergence C2 — (3) Cat C/Layer 2 + (4) Cat 4/P16.
- Mechanism: tile is pre-filled with `view.background_color` before replay; pre-fill is what the user sees when paint operations don't write the pixel.
- Co-factor with H1; explains *what color* the missing pixels become.
- Diagnostic: query `view.background_color` at runtime when X reproduces. If it isn't BLACK, but X is black, something else (wgpu LoadOp::Clear, compositor pre-fill site) is wrong.

### Tier 2 — single-projection, real and bounded

**H3. Render-strategy split — bug fires only in display-list branch**
- Provenance: asymmetric A1 — (3) Cat B; structurally invisible to (4).
- Mechanism: three render branches (viewport-buffer / parallel display-list / per-tile direct). Only display-list branch uses recording mode. Per-tile direct and viewport-buffer use `emPainter::new` (direct mode).
- Co-factor with H1: explains *under what conditions* H1 fires.
- Diagnostic: same as H1.

**H4. Texture-sampling primitives misbehave at replay (font-atlas specific)**
- Provenance: asymmetric A6 — (4) Cat 2; structurally invisible to (3).
- Mechanism: `PaintText` records `DrawOp::PaintText` but at replay the font cache atlas reference / initialization / glyph data is wrong. Could explain Z without explaining X.
- Plausible alternative if H1 doesn't fully account for Z.

**H5. Tile composite re-blending — alpha re-blend at compositor**
- Provenance: asymmetric A5 — (3) Cat D; structurally invisible to (4).
- Mechanism: `BlendState::ALPHA_BLENDING` at compositor pass blends per-tile RGBA against framebuffer; could darken anti-aliased text from the canvas color.
- Could partial-explain Z dimming. Not X.

**H6. `DrawList::replay` state-snapshot equivalence**
- Provenance: asymmetric A2 — (3) Cat E; (4) considered and rejected.
- Mechanism: `RecordedState` snapshot captured but not consulted at replay; replay relies on running painter state. A state mutation that bypasses `record_state` causes record/replay desync.
- Lower prior — would produce mis-clip / mis-transform, not solid-black.

### Tier 3 — residual concerns, low prior for F010

**H7. Send-Sync soundness for DrawOp variants holding `*const emImage`** — A3.
**H8. GPU pipeline (sRGB encoding, surface clear, present)** — A4. Y working argues against global GPU bug.
**H9. SVP-boundary `IsOpaque` branch correctness** — A8.
**H10. `canvas_color` snapshot/op-arg disagreement** — A7.
**H11. `debug_assert!` compiled out in release / push-pop pairing** — A9.

### Joint blind spots (input to (5) pre-mortem)

The pre-mortem must attack these. None can be derived from H1–H11 above:

- **B1**: theme/runtime-data correctness (DirContentColor parses to expected light-grey?).
- **B2**: panel state machine actually reaches `VFS_LOADED` in production at the symptomatic zoom level.
- **B3**: emDirPanel::Paint actually invoked (vs. parent-panel cull / viewport-clip).
- **B4**: stale tile from prior frame surviving cache invalidation.
- **B5**: font cache initialization order vs. first paint.
- **B6**: build-config / GPU-vendor / DPR-only reproductions.
- **B7**: recursive paint invocation safety.
- **B8**: compositor dirty-tile invalidation timing across state transitions.

---

## Final unified category list (numbered, for downstream methodology spec)

The methodology spec's pre-registration template uses these as the seed category list. Each pre-registered hypothesis must reference one of these category IDs (or document a new category not derivable from this synthesis).

| ID | Tier | Short name | Provenance |
|---|---|---|---|
| H1 | 1 | Recording-mode dispatch hole — Clear silently dropped | C1 ((3)Cat A + (4)Cat 1) |
| H2 | 1 | Tile pre-fill / background_color contract | C2 ((3)Cat C + (4)Cat 4) |
| H3 | 2 | Render-strategy split | (3)Cat B |
| H4 | 2 | Texture-sampling at replay (font-atlas) | (4)Cat 2 |
| H5 | 2 | Tile composite alpha re-blend | (3)Cat D |
| H6 | 2 | DrawList replay state-snapshot equivalence | (3)Cat E |
| H7 | 3 | Send-Sync soundness for DrawOp | (3)Cat F |
| H8 | 3 | GPU pipeline (sRGB / surface clear / present) | (3)Cat G |
| H9 | 3 | SVP-boundary IsOpaque correctness | (3)Cat I |
| H10 | 3 | canvas_color snapshot/op-arg disagreement | (4)P15 |
| H11 | 3 | debug_assert / push-pop pairing in release | (3)Cat H |
| B1 | blind spot | Theme/runtime-data correctness | joint |
| B2 | blind spot | Panel state machine reaches VFS_LOADED | joint |
| B3 | blind spot | Paint-not-reached for symptomatic panel | joint |
| B4 | blind spot | Stale tile from prior frame | joint |
| B5 | blind spot | Font cache initialization order | joint |
| B6 | blind spot | Build-config / environment-only repro | joint |
| B7 | blind spot | Recursive paint invocation safety | joint |
| B8 | blind spot | Compositor dirty-tile invalidation timing | joint |
