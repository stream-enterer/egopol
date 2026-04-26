# F010 pre-mortem — adversarial categories outside H1-H11 + B1-B8

Mandate: surface root-cause categories that the prior architectural-grounding (Frame A) and differential-constraint (Frame B) projections are STRUCTURALLY incapable of seeing, or had to assume away. Pre-mortem premise: a fix lands on one of H1-H11, mechanical+GUI verification passes, three weeks later the same symptom recurs. What did the existing hypothesis space miss?

The existing space (H1-H11 + B1-B8) is a *paint-pipeline projection*. Both frames look at the pipeline. Categories outside the pipeline (input-determining state, environment, build, scheduler, theme data, asset lifecycle, recursion, frame history, observer effects) are systematically under-represented.

---

## Method 1: Frame-violation analysis

### Frame A (architectural grounding) — blind properties

Frame A is a static layer-by-layer walk of contracts in the paint pipeline. It cannot detect:

1. **Temporal / cross-frame state divergence.** Frame A inspects each layer at a single moment. It cannot see "this layer was correct at frame N but produced wrong output at frame N+1 because layer M's state from frame N-1 leaked through." Example surfaced category: prior-frame compositor residue.
2. **Runtime data correctness.** Frame A audits structure (which function calls which, which contract is enforced). It cannot detect "the value `DirContentColor` resolves to at runtime is wrong" — that requires reading what the theme parser produced from disk. (Already B1, but the *class* — runtime-data, not just theme — extends to atlas glyph metrics, font metric tables, per-locale text shaping data.)
3. **Environment-conditional behavior.** Frame A is implementation-of-the-pipeline; it does not parameterize over GPU vendor, driver version, DPR, color profile, or window scaling. (B6 names this once but does not enumerate sub-classes.)
4. **Init / activation order.** Frame A reads the steady-state contract. It cannot see "during the first N frames after launch, layer X is not yet initialized and silently no-ops." (B5 names font-cache only; this generalizes.)
5. **Observer effect / instrumentation perturbation.** Frame A reads code; it cannot detect that the symptom only appears under one harness configuration (e.g. release with `RUST_LOG=off`, or windowed vs offscreen). The mechanical test may sit in a regime that hides the bug.
6. **Side-channel state changes outside the pipeline.** Frame A walks Paint → present. It cannot see paths where input-event handlers, IPC, IO callbacks, engine ticks, or background workers MUTATE state used downstream by paint, between record and replay, or between paint and present.
7. **Build-time conditional compilation.** `cfg(debug_assertions)`, `cfg(feature = ...)`, target-arch differences. Frame A reads source as if all branches are live.

### Frame B (differential constraint) — blind properties

Frame B walks paint-primitive properties and asks which discriminate Y from X+Z. It cannot detect:

1. **Which primitive is dispatched.** Frame B inspects each primitive's properties; it presumes the correct primitive arm is reached. A panel-state-machine bug that routes paint to the wrong arm (or skips the call) is invisible.
2. **Cross-primitive interactions.** Frame B compares primitives in isolation. It cannot see "PaintRect at depth N corrupts state used by PaintText at depth N+1." (Related to A2/H6 but at primitive-pair granularity.)
3. **Off-axis differential properties.** Y-vs-X+Z is the chosen axis. A property that splits X+Y vs Z (or splits along zoom level, or panel-tree depth) is structurally invisible.
4. **Frame-history and stale-cache.** Per-primitive properties are within-frame. The bug could be "Y's tile was drawn this frame, X+Z's tiles are stale from a prior frame."
5. **Asset/resource lifetime independent of dispatch.** A primitive can dispatch correctly and reference a resource ID that was valid when recorded but invalid when replayed (atlas eviction, GPU buffer reuse, texture handle recycled).
6. **Whether the panel is even painted.** Frame B starts from "X is a Clear, Z is a PaintText" — it presumes both calls are reached. Panel-cull, parent IsOpaque cascade, or viewport-clip eliding the entire emDirPanel::Paint method is invisible. (B3, but only as a blind spot label.)
7. **Multi-pass / multi-target painting.** A primitive may paint to a side target (offscreen scratch / shadow buffer) that is then composited; per-primitive comparison cannot see the composite step.

### Categories surfaced from frame violations

- (M1-1) Cross-frame compositor residue — distinct from B4 (B4 is "stale tile painted in prior frame surviving"; this is "compositor texture from prior frame is being sampled because the present-time blend reuses it as src").
- (M1-2) Input-driven state mutation between record and replay (engine tick, file-watcher callback, IPC) — not in B1-B8.
- (M1-3) Build-config-conditional code path (cfg-gated arm only compiled in release, or only with a feature flag).
- (M1-4) Observer-effect / instrumentation-only reproduction (the symptom only appears when DUMP_DRAW_OPS is OFF, etc.).
- (M1-5) Panel-state-machine misroute (different from B2 — B2 is "doesn't reach VFS_LOADED"; M1-5 is "reaches a state that calls a different paint path than the test fixture exercises").
- (M1-6) Off-axis differential — bug splits X+Y vs Z, or splits by zoom level, masquerading as "X+Z fail Y works" because the user only zoomed once.
- (M1-7) GPU resource lifetime — atlas eviction, buffer reuse, texture handle recycling, between record and replay.
- (M1-8) Multi-target paint composition — paint goes to a scratch target that is then composited under wrong blend mode.

---

## Method 2: Bug-taxonomy walk

### First-pass taxonomy coverage

| Slot | Covered by H1-H11 + B1-B8? | If not, candidate category |
|---|---|---|
| State-snapshot desync (record vs replay state divergence) | Yes — H6 | — |
| Resource lifetime (atlas eviction, GPU buffer reuse, texture handle recycling) | Partially — H7 covers raw-pointer soundness, not lifecycle | T1: GPU/atlas resource lifetime distinct from soundness |
| Cache coherence (CPU↔GPU sync, dirty flagging, invalidation semantics) | Partial — B4/B8 name tile-cache; not CPU↔GPU sync | T2: CPU-side state written but not visible to GPU at present time (e.g. mapped-buffer flush missing) |
| Color/alpha format (sRGB↔linear, premultiplied↔straight, byte-order) | Partial — H8 names sRGB; not premultiplied vs straight, not byte-order | T3: alpha-mode mismatch (premultiplied vs straight) — subtle: could yield black where canvas-color was expected, since text drawn with straight alpha onto a premultiplied buffer may produce zero-RGB |
| Threading (data race, init order, thread-local resource not in worker) | Partial — H7 names Send/Sync; not init-order or thread-local | T4: thread-local font/atlas/state not present in parallel-replay worker |
| Numeric drift (FP accumulation, integer overflow, rounding) | No | T5: integer overflow or rounding in tile coordinate → off-by-one tile-grid alignment placing X+Z paint outside any tile's visible region while Y's border-image lands within |
| Environment (GPU vendor, driver, OS, DPR, color profile, window scaling) | Named B6 only as label | T6: DPR / Wayland fractional-scale interaction (sub-class of B6 — not enumerated) |
| Build config (debug vs release, feature flags, conditional compilation) | No | T7: cfg(debug_assertions) gating (overlaps M1-3) |
| Upstream library (wgpu version, winit timing, font lib) | No | T8: wgpu/winit upgrade-induced semantic change (e.g. SurfaceConfiguration alpha-mode default flipped between versions) |
| Sequencing (paint vs swap, vsync interaction, frame ordering) | Partial — A1/H3 names render-strategy split; not vsync/swap-chain ordering | T9: present-time ordering — Y's commands flushed in frame N, X+Z's flushed in frame N+1 due to multi-encoder submit |
| Geometry (clip vs viewport intersection, NDC mapping, half-pixel offsets) | No | T10: clip rect computed in one space, viewport in another; X+Z primitives clipped to zero-area |
| Initialization (cold-start, lazy not yet ready, default vs configured) | Partial — B5 names font cache only | T11: panel-painter init order — `RecordingPainter` constructor returns before some field is ready |

### Iteration 2: missing slots

After first pass, slots not in the seed taxonomy:

- **Slot S1: Asset/data correctness on disk.** Theme file truncated, font file missing a glyph subtable, locale data wrong. (B1 names theme; not the broader asset class.)
- **Slot S2: User-action sequence dependence.** The bug requires a specific sequence of user actions to manifest (zoom-in then zoom-out then zoom-in). Synthesis frames are stateless w.r.t. user history.
- **Slot S3: Memory pressure / allocator.** Allocator returns NULL or recycled memory under pressure; raw-pointer DrawOps observe wrong content.
- **Slot S4: Logging / panic / error-recovery paths.** A swallowed Result drops the X+Z paint silently; Y has no error path so works.
- **Slot S5: Coordinate-system ambiguity.** Logical f64 coords vs pixel i32 — a panel that is sub-pixel-positioned at a zoom level may round to a degenerate rect on X+Z's primitives but not Y's border-image (which is pre-quantized to integer pixels by tile boundaries).

Coverage update:
- Slot S1 partially in B1; broader asset-correctness candidate added (T12).
- Slot S2 not covered — candidate T13 (user-history-dependent state).
- Slot S3 not covered — candidate T14 (allocator/memory-pressure).
- Slot S4 not covered — candidate T15 (silent error-path swallowing X+Z primitives).
- Slot S5 not covered — candidate T16 (coordinate-system rounding asymmetry between primitive classes).

### Categories surfaced from taxonomy walk

- (M2-T1) GPU/atlas resource *lifecycle* — eviction/recycle, distinct from H7 soundness.
- (M2-T2) CPU↔GPU sync — mapped-buffer flush, write_texture vs queue.submit ordering.
- (M2-T3) Premultiplied vs straight alpha mismatch.
- (M2-T4) Thread-local resource missing in parallel-replay worker.
- (M2-T5) Integer/FP rounding in tile coord placing X+Z outside any tile.
- (M2-T6) DPR / Wayland fractional-scale interaction.
- (M2-T7) cfg(debug_assertions)-gated path.
- (M2-T8) wgpu/winit upgrade semantic change (alpha-mode default, etc.).
- (M2-T9) Multi-encoder submit ordering — Y in frame N, X+Z in frame N+1.
- (M2-T10) Clip-vs-viewport coordinate-space mismatch.
- (M2-T11) RecordingPainter init incomplete at first paint.
- (M2-T12) Asset on disk corrupt/missing (broader than theme).
- (M2-T13) User-history-dependent state (zoom-out-then-in sequence).
- (M2-T14) Allocator / memory-pressure / recycled allocation observed by raw-pointer DrawOp.
- (M2-T15) Silent Result-drop in X+Z code path; Y has no error path.
- (M2-T16) Coordinate-system rounding asymmetry between primitive classes (i32 quantization).

---

## Method 3: Symptom-variant counterfactuals

### V1 (X+Y both fail — no panel paints)
Root-cause category: panel-not-reached / parent-cull (B3) OR engine-tick scheduler not invoking Paint at all OR window-surface configuration broken. This points at *paint dispatch upstream of primitives*. None of H1-H11 are scoped to "no paint at all"; B3 is the existing label. **No new category.**

### V2 (Only Z fails — panel bg correct, borders correct, text broken)
Root-cause category: **font-cache initialization order, glyph atlas eviction, or text-shaping pipeline failure** specific to text primitives. Already H4 + B5. But V2 also matches: **per-locale text-shaping data missing on this system**, **freetype/harfbuzz lib version mismatch**, **font fallback chain different in production vs test**. These are sub-categories of M2-T12 and B6. **Reinforces M2-T12.**

### V3 (All three work at first paint, then fail after VFS_WAITING → VFS_LOADED transition)
Root-cause category: **state-transition handling — invalidation does not fire, or fires but invalidates wrong region, or the transition recompiles a paint plan from stale snapshot**. Partially B8. But also: **the state-transition itself runs an init-once path that succeeds first time then is silently skipped second time** — an init-order/idempotency bug. Surfaces **M3-V3a: idempotency bug in state-transition handler**. Not in H1-H11+B1-B8.

### V4 (Failure is intermittent — works sometimes, fails sometimes, same launch)
Root-cause category: **non-determinism**. Sub-classes:
- Threading race (H7 covers soundness, not data race correctness).
- Allocator/heap layout dependent (M2-T14).
- Time-dependent — frame-rate-sensitive (e.g. tile invalidation only fires when frame-time exceeds a threshold).
- Random init order between async tasks.
Surfaces **M3-V4a: non-deterministic scheduling / ordering between async paint-prep tasks**. Not in H1-H11+B1-B8.

### V5 (X fails, Z works, Y works — only background broken)
Root-cause category: H1 (recording-mode dispatch hole for Clear) — already top hypothesis. But also: **panel background is painted by a different code path than text/borders** (e.g. emPanel::PaintBackground vs emDirPanel::Paint), and the PaintBackground virtual is overridden in C++ but not in Rust port. Surfaces **M3-V5a: virtual-method override missing in Rust port** (vtable correspondence broken). Distinct from H1: H1 is "Clear primitive dropped at painter layer"; M3-V5a is "the panel never even calls Clear in Rust because the virtual didn't dispatch." Adjacent to M1-5 / B2 but at the vtable layer.

### Categories surfaced from variants
- (M3-V3a) State-transition handler idempotency bug — runs once, silently skips on re-entry.
- (M3-V4a) Non-deterministic async-task ordering between paint-prep stages.
- (M3-V5a) Virtual-method override missing in Rust port (Paint, PaintBackground, PaintContent).

---

## Method 4: Fix-survival scenarios

### Scenario: H1 fix lands, symptom resurfaces

- Fix: add `DrawOp::Clear { color, canvas_color, rect }` variant; emit it from `emPainter::Clear` in recording mode; replay handler dispatches to direct `PaintRect`. Mechanical: a unit test verifying recorded ops include a `Clear` op for a `Clear`-emitting panel.
- Mechanical test passing: `cargo test --test golden card_blue_dirpanel` green; `divergence_report.py` shows DirContent tile no longer black.
- GUI verification passing: user launches eaglemode-rs, zooms into Card-Blue dir, sees light-grey background and six-field info pane. Visual confirmation.
- Resurfacing condition: **three weeks later the user changes their `~/.eaglemode/Themes/CardBlue.emTmp` file or upgrades to a theme version where `DirContentColor` is changed to a value that renders as black** — but the *real* theme file still says light-grey. The recording-mode hole was closed, but the value being passed to Clear was sourced from a stale theme cache that was never reloaded after the theme file changed at runtime.
- Actual root cause: **theme cache invalidation on file change** — distinct from B1 (which is "parses to wrong value"), this is "parses correctly initially but cache is not invalidated when source changes."
- Surfaces: **M4-S1: theme/asset hot-reload cache invalidation absent.**

### Scenario: H2 fix lands

- Fix: explicitly set `view.background_color` to canvas_color of root panel each frame; harden `LoadOp::Clear` at compositor pass to use the canvas color rather than a global default.
- Mechanical test passing: golden test for tile pre-fill matches expected canvas color in PPM dump.
- GUI verification passing: panel interior is no longer black; matches C++ reference visually.
- Resurfacing condition: **user opens a directory containing a file whose preview panel uses a different canvas_color (e.g. a code file with dark theme content panel inside a light Card-Blue parent)**. The frame's root canvas color is light-grey, but the deeper panel's tile is also pre-filled light-grey, masking the fact that the deeper panel's own Clear is still being silently dropped.
- Actual root cause: **per-panel-subtree canvas color** is the contract; H2's "frame-global background_color" is a coarsening that worked for one panel and fails for nested panels. Real root cause is still H1 + a per-subtree canvas accounting bug.
- Surfaces: **M4-S2: canvas-color is per-panel-subtree, not per-frame** — adjacent to H10 but distinct (H10 is snapshot/op-arg disagreement; this is contract-coarseness).

### Scenario: H3 fix lands (render-strategy split)

- Fix: force per-tile direct branch always (bypass display-list path). Mechanical: clippy clean, unit tests pass.
- Mechanical test passing: golden tests pass because direct mode is correct.
- GUI verification passing: user sees correct rendering on their dev machine.
- Resurfacing condition: **performance-degradation rollback merges later** that re-enables display-list for `dirty_count > N` because direct mode is too slow for full re-paint scenarios. The re-enabled display-list path still has the H1 hole, OR has acquired a *new* hole because the now-rarely-used path bit-rotted while the direct path was authoritative.
- Actual root cause: **bit-rot of disabled-but-not-removed code path** under "fix by avoidance" — when the fix is "don't go down that path" rather than "fix the path," the underlying bug remains and re-emerges on path re-enablement.
- Surfaces: **M4-S3: avoidance-fix bit-rot** — a methodology-level category; the fix architecture itself contains the failure mode.

### Scenario: H4 fix lands (font atlas at replay)

- Fix: ensure font cache is fully populated for all glyphs in the replay-ahead pass; cache is shared via `Rc` across record/replay.
- Mechanical test passing: PaintText golden tests stable across 100 runs.
- GUI verification passing: text visible.
- Resurfacing condition: **user installs a new font via fontconfig that has a different glyph set; OR the glyph atlas is evicted under memory pressure when the user opens a very large directory**. The font cache wasn't *not populated* — it was populated correctly initially, then evicted, and the eviction signal was not propagated to any in-flight DrawList.
- Actual root cause: **cache eviction during replay** (M2-T1 — GPU/atlas resource lifecycle).
- Surfaces: **M4-S4: cache-eviction-during-replay** — atlas evicted between record and replay, DrawOp's stale handle samples garbage / black.

### Categories surfaced from fix-survival
- (M4-S1) Theme / asset hot-reload cache invalidation absent.
- (M4-S2) Canvas-color contract is per-panel-subtree, but implementation treats it as per-frame.
- (M4-S3) Avoidance-fix bit-rot — disabling the broken path leaves the bug latent.
- (M4-S4) Cache eviction during replay (refines M2-T1).

---

## New candidate categories (P1, P2, ...)

The following candidates survived: (a) being genuinely outside H1-H11+B1-B8, (b) fitting the X+Z-fail-but-Y-works pattern, and (c) at least one method as provenance. Sorted by triangulation strength (multi-method first), then mechanistic specificity.

### P1: Cache eviction / GPU-resource lifecycle during recording-to-replay window

- Provenance: Method 1 (Frame B blind property #5 — asset/resource lifetime), Method 2 (T1 — GPU/atlas lifecycle distinct from H7 soundness), Method 4 (S4 — cache-eviction-during-replay survives H4 fix). Triangulated by 3 methods.
- F010 fit: Y is a border-image — its texture is uploaded once and pinned (border-images are typically long-lived, low-eviction-priority). X is a Clear (no texture, but if H1 is fixed, the replacement rect uses canvas_color directly). Z is text — glyphs are atlased, atlas is the most evicted resource. Under the joint H1-fix scenario, X would still fail if the per-tile pre-fill texture binding is recycled before composite; Z fails because atlas pages are evicted between record and replay. **Pattern matches: Y's resource is pinned, X+Z's resources are eviction-eligible.**
- Falsifiability: instrument atlas/texture-handle lifecycles and verify no eviction occurs between `DrawOp` recording and replay. If lifetimes are pinned for the entire frame, P1 is killed.
- Distinct from H1-H11+B1-B8: not H1 (H1 is dispatch hole — Clear simply not recorded); not H7 (Send/Sync soundness, not lifetime correctness); not B4 (B4 is *tile* staleness — whole tile reused; P1 is sub-tile resource eviction within a re-painted tile). B4 names the wrong granularity.
- Code citation: no citation — this is a runtime/lifecycle property. Investigation hook: `emViewRendererCompositor.rs` resource handles, font-cache atlas eviction policy.

### P2: State-transition handler idempotency / hot-reload cache invalidation

- Provenance: Method 3 (V3a — state-transition idempotency), Method 4 (S1 — theme hot-reload invalidation absent). Triangulated by 2 methods.
- F010 fit: Y (outer/inner borders) is rendered from a border-image asset that is loaded once at theme-init and held for the panel's lifetime. X (panel interior bg) and Z (info pane fields) sample state that depends on per-frame computed values (`DirContentColor` resolved via theme lookup, info-pane field values from a VFS query). If a state transition (theme reload, VFS_WAITING→VFS_LOADED) sets a "needs recompute" flag and the handler clears the flag without doing the recompute (idempotency-violation), X+Z see stale-zero/black values while Y's pre-loaded image is unaffected.
- Falsifiability: log every theme/VFS state transition and verify the post-transition recompute fires. If recompute always runs on every transition, P2 is killed.
- Distinct from H1-H11+B1-B8: not B1 (B1 is "parses wrong"; P2 is "parses right initially, then drifts"); not B2 (B2 is "doesn't reach VFS_LOADED"; P2 is "reaches VFS_LOADED but the transition didn't run its recompute"); not B8 (B8 is *compositor* dirty-tile invalidation; P2 is *data-layer* invalidation upstream of paint).
- Code citation: no citation — this is a temporal / init-order property. Investigation hook: `emDirPanel::Notice`, theme-reload watchers, VFS state-machine transition handlers.

### P3: Virtual-method / vtable-correspondence break in Rust port

- Provenance: Method 3 (V5a). Single-method but mechanistically high-fit and CLAUDE.md File-and-Name correspondence makes this an idiosyncratic risk in this codebase.
- F010 fit: emPanelBase has virtuals (Paint, PaintBackground, PaintContent, IsContentOpaque). C++ overrides in emDirPanel set the panel-interior paint behavior. In Rust port, if the override is missing OR not wired through a trait dispatch table, the parent class's default (which may be a no-op or a different color) runs instead. This produces "Clear never called" — Frame B presumes Clear *is* called and just gets dropped at the painter layer.
- Falsifiability: instrument every panel virtual entry and confirm `emDirPanel::Paint` runs for the symptomatic panel. If it does, P3 is killed (and the paint primitive really is being dispatched, just dropped — H1).
- Distinct from H1-H11+B1-B8: not H1 (H1 fires after dispatch reaches Clear; P3 is "dispatch never reaches the override"); not B3 (B3 is "Paint method not reached *at all*"; P3 is "wrong Paint method reached" — base instead of override). B2 is closest but B2 is state-machine-arm; P3 is vtable-layer.
- Code citation: investigation hook is `emDirPanel.rs` Paint/PaintContent overrides and trait-dispatch wiring; check whether trait method is named differently or shadowed.

### P4: Non-deterministic async-task ordering between paint-prep stages

- Provenance: Method 1 (Frame A blind property #6 — side-channel state changes), Method 3 (V4a — non-determinism). Triangulated by 2 methods.
- F010 fit: paint-prep involves multiple async stages (VFS query for info pane, theme resolution, font-cache prepopulation). If their completion order is non-deterministic, the symptomatic launch sees X+Z's prep stages complete *after* the paint frame consumes them; Y's border-image prep happens at theme-load and is always ready. Bug is sometimes-reproducing (could be why F010 was historically flaky to repro).
- Falsifiability: pin all async paint-prep to a deterministic order; if symptom is gone, P4 confirmed; if persists, killed.
- Distinct from H1-H11+B1-B8: not H7 (H7 is Send/Sync; P4 is ordering correctness); not B5 (B5 is font-cache only; P4 is general async-prep ordering).
- Code citation: no citation — runtime / ordering property. Investigation hook: engine-tick scheduler, file-model loaders, theme-resolution path.

### P5: Build-config-conditional code path (cfg-gated arm)

- Provenance: Method 1 (Frame A blind property #7), Method 2 (T7). Triangulated by 2 methods.
- F010 fit: a code path that is only compiled in release (or only with `--features production`) routes panel-interior paint differently. The symptom is build-config-specific: GUI verification on dev's debug build passes; user runs release build; X+Z fail.
- Falsifiability: reproduce the bug under both cargo profiles and with a feature-flag matrix. If symptom is profile-invariant, P5 killed.
- Distinct from H1-H11+B1-B8: not B6 (B6 is environment — GPU/DPR; P5 is build-time, same hardware). H11 names debug_assert specifically; P5 is broader.
- Code citation: no citation — build-config property. Investigation hook: `rg 'cfg\\(' src/emCore/emPainter` and surrounding modules.

### P6: Avoidance-fix bit-rot of disabled paint path

- Provenance: Method 4 (S3). Single-method, methodology-level rather than mechanistic, but listed because the F010 investigation has already considered "force direct mode" as a diagnostic — that diagnostic could become the fix.
- F010 fit: not a direct F010 root cause but a *resurfacing risk*. If the H3 / H1-via-avoidance fix lands, the display-list path bit-rots; symptom returns when path is re-enabled.
- Falsifiability: methodology-level; mitigated by NOT using avoidance fixes for H1.
- Distinct from H1-H11+B1-B8: methodology category, not in scope of either frame.
- Code citation: process-level.

### P7: Multi-pass / multi-target paint composition under wrong blend mode

- Provenance: Method 1 (Frame B blind property #7), Method 2 (T9 — multi-encoder submit ordering, weakly).
- F010 fit: if the panel-interior and info-pane primitives go to a scratch target (offscreen surface for clipping) that is then composited with a different blend mode than borders use, X+Z would composite wrong while Y (which goes direct) is unaffected.
- Falsifiability: enumerate all wgpu encoder/render-pass instantiations during a frame; if all paint goes to the same target with the same blend, P7 killed.
- Distinct from H1-H11+B1-B8: H5 (tile composite re-blend) is at the tile-composite stage; P7 is *intra-frame* multi-pass to a side target.
- Code citation: investigation hook: `emViewRendererCompositor.rs` encoder/render-pass count per frame.

### P8: Coordinate-system rounding asymmetry — Clear/PaintRect/text round to degenerate rects, border-image preserves area

- Provenance: Method 2 (T16).
- F010 fit: at the symptomatic zoom level, the panel's logical f64 coords for interior rect round to a zero-area i32 pixel rect; Clear silently does nothing (not because of H1 but because zero-area). Borders are tile-aligned and always have non-zero area.
- Falsifiability: log the i32 pixel rect for emDirPanel::Paint Clear at the symptomatic zoom; if non-degenerate, P8 killed.
- Distinct from H1-H11+B1-B8: H1 is "primitive recorded as no-op for being a Clear in DrawList mode"; P8 is "primitive a no-op for being zero-area." Different mechanism, same observable.
- Code citation: investigation hook: f64→i32 rect conversion in panel paint sites and `emPainter::Clear` rect computation.

---

## Summary

Top-3 candidates by triangulation + F010-fit:
1. **P1 — GPU/atlas resource lifecycle (eviction during record→replay window)** — 3 methods, distinct from H7 and B4, explains Y-pinned vs X+Z-evictable asymmetry.
2. **P2 — State-transition handler idempotency / hot-reload invalidation** — 2 methods, explains both VFS-state and theme-reload resurfacing.
3. **P3 — Virtual-method override missing in Rust port** — single method but high specificity; CLAUDE.md File-and-Name correspondence makes this a structural risk for this codebase.
