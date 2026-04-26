# F010 architectural grounding — layered contract enumeration

Top-down walk of the paint pipeline: `emView::Paint` → "pixels in window framebuffer." Each row names a layer, the contract it claims to uphold, and where (if anywhere) the contract is enforced. Every `(unenforced)` row is a candidate hypothesis source.

C++ reference paths (Eagle Mode 0.96.4, `~/Projects/eaglemode-0.96.4/`):
- `src/emCore/emView.cpp:1048-1146` (Paint)
- `src/emCore/emPainter.cpp:364-374` (Clear)
- `src/emCore/emFilePanel.cpp:187-200` (IsOpaque, Paint)

C++ paints **directly** to the framebuffer; there is no display-list, no compositor, no tile cache, no per-tile pre-fill. Every layer below labelled RUST_ONLY introduces a contract C++ never had to maintain.

## Pipeline layer table

| # | Layer | Inputs | Outputs | Stated contract | Where enforced (file:line) | RUST_ONLY? | Differs from C++? |
|---|-------|--------|---------|-----------------|----------------------------|------------|-------------------|
| 1 | `emWindow::render` (render driver / strategy chooser) | `view`, `tree`, dirty-tile counts, `gpu` | dispatches to one of three branches: viewport-buffer, parallel display-list, per-tile single-thread | "the chosen strategy produces pixels equivalent to a single direct paint pass for any panel tree" | `(unenforced)` — three branches are independent code paths; no equivalence assertion exists. Visual divergence between branches has surfaced before (memory id 272 4:26p Apr 25). | Yes — C++ has one path | Yes — C++ has no strategy split (`emView.cpp:1048`) |
| 2 | `emWindow::render` per-tile pre-fill | `view.background_color` (captured each frame, line 620) | tile/viewport buffer initialized to background_color (lines 640, 683, parallel branch in `emWindow.rs:777`, plus compositor pre-fills in `emViewRendererCompositor.rs:48,99,111`) | "every pixel a panel does not paint observes `view.background_color`, never BLACK or stale" | `emWindow.rs:620-621, 640, 683`; `emViewRendererCompositor.rs:48,99,111` (the call site is the enforcement; nothing checks correlation between background_color and the pre-fill source) | Yes — there is no separate buffer to pre-fill in C++; framebuffer state is whatever painter.Clear sets it to | Yes — F018 had to introduce this (`set_background_color` rule I.5); C++ does not have it (`emView.cpp:1063,1082`) |
| 3 | `view.Paint(tree, painter, TRANSPARENT)` entry | painter, root tree | painter receives every panel's draw calls in DFS order | "scale=1, conditional clear, push_state once around child loop, pop_state once after, DFS sibling/child traversal" | `emView.rs:4714-4717` (debug_assert scale=1); `emView.rs:4796/4880` (push/pop pairing); `emView.rs:4760-4772` (conditional clear via tree.IsOpaque) | No (mirrors C++) | No structural difference — Rust mirrors `emView.cpp:1048-1135` |
| 4 | Conditional canvas clear at SVP boundary | tree.IsOpaque(svp_id), svp_canvas, view.background_color, render-region rect | one `ClearWithCanvas(ncc, canvas_color)` if SVP not opaque or doesn't fully cover render region | "after this point, canvas_color = ncc; pixels not painted by SVP/children visibly observe ncc" | `emView.rs:4760-4772` matches `emView.cpp:1073-1084`; `ClearWithCanvas` records `PaintRect` in recording mode (`emPainter.rs:862-869`) so this **is** captured by the display-list path | No | No — C++ `emPainter::Clear(texture, canvasColor)` is itself a delegating PaintRect (`emPainter.cpp:364-374`); Rust matches |
| 5 | `paint_one_panel` → behavior dispatch | panel id, canvas_color, painter | invokes `Behavior::Paint(painter, panel_state)` | "the behavior gets a painter with the panel's clip+transform pre-installed; canvas_color reflects parent's resolved canvas" | `emView.rs:4796-4800,4840-4841` (set clip + transform per panel) | No | No |
| 6 | Behavior `Paint` impl (panels: emFilePanel, emDirPanel, emDirEntryPanel, emFileLinkPanel, emDirStatPanel, emStarFieldPanel, etc.) | painter, panel state | issues paint calls including `painter.Clear(color)` (single-arg, no canvas) and `painter.PaintRect`, `painter.PaintTextBoxed`, etc. | "all paint calls produce the same observable pixels regardless of painter mode (direct or recording)" | `(unenforced)` — `painter.Clear(color)` calls **at** `emfileman/src/emDirPanel.rs:459`, `emfileman/src/emFileLinkPanel.rs:236,295`, `emfileman/src/emDirStatPanel.rs:145`, `emstocks/src/emStocksFilePanel.rs:41`, `emmain/src/emStarFieldPanel.rs:197`, `emmain/src/emMainPanel.rs:277` are silently dropped in recording mode (`emPainter.rs:5775-5784` uses `require_direct`, no `DrawOp::Clear` exists). | No (panels exist in C++) | Yes — C++ panels call `painter.Clear(c, canvasColor)` (two-arg) at `emFilePanel.cpp:300,330,360`; Rust panels call single-arg `Clear(color)`. The signature divergence is the bug enabler. |
| 7 | `emPainter` dual-mode dispatch (direct vs recording) | DrawOp + state | rasterizes (direct) OR appends `RecordedOp` to ops vec (recording) | "every public paint method either records the op (recording) or produces a `DirectProof` and rasterizes (direct) — never silently no-ops" | `emPainter.rs:660-674` (`try_record`); `emPainter.rs:679-684` (`require_direct`) — but `require_direct` returns `None` (silent drop) for any method that uses it without a paired DrawOp | Yes — recording mode has no C++ analogue | Yes — C++ has only direct mode (`emPainter.cpp` throughout) |
| 7a | `require_direct` callers without paired DrawOp variant (sub-class of layer 7) | painter state | direct: rasterize; recording: silently return | "each `require_direct` site has a behavior-equivalent recording fallback" | `(unenforced)` — `Clear(color)` at `emPainter.rs:5775-5784` is a confirmed silent-drop site (memory id 741 9:04a Apr 26). Other `require_direct` users (the various paint helpers at `emPainter.rs:1052,3277,1314,1362,2545,2574,2837,2888,2948,2996,3056,3104,3430`) are reached only **after** a successful `try_record` at the public API — so they are guarded by a sibling DrawOp. `Clear(color)` is the only public API that uses `require_direct` directly. | Yes | Yes — C++ has no recording mode |
| 8 | DrawOp enum coverage | painter API surface | enumerates every recordable op | "every public draw method has a matching `DrawOp` variant" | `(unenforced)` — `emPainterDrawList.rs:14-` enumerates ~30 variants. There is no `DrawOp::Clear` variant. There is no compile-time check that every public `emPainter::Paint*` / `Clear*` method has a record path. | Yes | Yes |
| 9 | `DrawList::replay` state restoration | DrawList ops, painter, tile_offset | replays ops into the per-tile painter | "every recorded op replays exactly once with state equivalent to its recording-time snapshot" | `emPainterDrawList.rs:438-` — replay walks `&self.ops` and dispatches by `DrawOp` variant, reusing the painter's *current* state (push/pop/clip/transform are themselves DrawOps). `RecordedState` snapshot is captured (`emPainter.rs:646-658,669,695`) but **not consulted** during replay — the per-op `state` field is dead at replay time. | Partially — the dispatch loop is a positive contract (each variant has an arm); but the snapshot-equivalence claim is unenforced. | Yes (RUST_ONLY layer) | Yes — no analogue |
| 10 | Per-tile clip semantics during replay | tile_offset, recorded clip ops | tile painter clip should be the intersection of (recorded clip) ∩ (tile bounds) | "ops that fall outside this tile contribute no pixels; ops that span tile boundaries are correctly clipped" | `emPainterDrawList.rs:442` (`set_offset(-tile_offset)`); `emPainter::SetClipping` does intersection at record time, but at replay the recorded clip is re-applied to the tile painter — the tile-bounds intersection happens implicitly because the tile painter starts with `clip = (0,0,tile_w,tile_h)` and SetClipping intersects. | `emPainter.rs:755-783` (intersection logic) | Yes | Yes |
| 11 | Parallel replay determinism | DrawList (shared, immutable), per-tile painters | each tile's pixels are deterministic and independent of thread ordering | "tile compositing is associative — order of tile completion does not affect framebuffer" | `emWindow.rs:773-786` `pool.CallParallel` over `dirty_tiles`; results stored in `Vec<Mutex<Option<emImage>>>` indexed by tile, then composed back in order. Determinism rests on DrawList being read-only across threads (`unsafe impl Send/Sync` at `emPainterDrawList.rs:381,415`). | Send/Sync impls are `unsafe` and unchecked. | Yes | Yes |
| 12 | Tile upload to GPU | tile.image (RGBA8) | wgpu texture write | "queue.write_texture preserves pixel values byte-for-byte" | `emViewRendererCompositor.rs:226-244` (write_texture call); not asserted, relies on wgpu contract | Yes | Yes |
| 13 | wgpu render pass `LoadOp::Clear(background_color)` | compositor.background_color | framebuffer cleared to bg before tile composite | "background color from `view.background_color` reaches the surface clear" | `emViewRendererCompositor.rs:272-279`; `set_background_color` at `:144-146` set each frame at `emWindow.rs:621` | Yes | Yes |
| 14 | Tile composite (textured quad) | per-tile bind groups, draw 6 verts each | composited framebuffer | "each tile's pixels overlay the cleared framebuffer at its computed NDC offset; alpha blending matches `BlendState::ALPHA_BLENDING`" | `emViewRendererCompositor.rs:286-296` (draw loop); `:99` (`BlendState::ALPHA_BLENDING`) | `(unenforced)` — alpha blending is an *active* compositor decision (not pass-through). If a tile's RGBA contains semi-transparent pixels (e.g. text anti-alias against `TRANSPARENT` canvas), they get re-blended against the bg-clear at composite time. C++ has no such re-blending step. | Yes | Yes — no C++ composite pass |
| 15 | Surface present | wgpu Surface | OS framebuffer | "submit + present hands the texture to the OS unchanged" | `emViewRendererCompositor.rs:299-300`; relies on wgpu contract | Yes | Yes |

### Notes on classification

- Rows 1, 2, 7, 7a, 8, 9, 10, 11, 12, 13, 14, 15 are RUST_ONLY layers (the entire compositor + display-list pipeline).
- Row 4 (`ClearWithCanvas` at the SVP boundary) is **not** broken: it routes through PaintRect → DrawOp::PaintRect, mirroring C++ exactly. This is what makes Symptom Y (border-image gradients) work — non-Clear-based paint paths survive.
- Row 6 is the dense bug surface: any panel whose `Paint` implementation calls `painter.Clear(color)` is invisible in recording mode. emDirPanel at `emfileman/src/emDirPanel.rs:459` calls `painter.Clear(dc)` with `dc = DirContentColor` — exactly Symptom X. emDirStatPanel at `emfileman/src/emDirStatPanel.rs:145` clears `bg_color` then paints six small text+rect items — exactly Symptom Z.
- C++ panels (e.g. `emFilePanel.cpp:300,330,360`) use **two-arg** `painter.Clear(c, canvasColor)`, which delegates to PaintRect (`emPainter.cpp:364`). The Rust port that is at risk is the **single-arg** `Clear(color)` form, which has no C++ analogue at the panel API level — it's a Rust convenience that bypasses canvas_color and hence bypasses PaintRect → DrawOp::PaintRect.

## Derived hypothesis categories

Each rule below fires per the prompt's mapping:
- R1: every `(unenforced)` row.
- R2: every RUST_ONLY layer.
- R3: every "Differs from C++" row.
- R4: every enforced row (residual: enforcement could be wrong).

Rankings by rules-fired count.

### Category A (rules 1+2+3, 3 rules) — Layer 6: panel `Clear(color)` calls silently dropped in recording mode

- Rule(s) fired: R1 (unenforced — no compile-time check that every painter API has a recording path), R2 (recording mode is RUST_ONLY), R3 (C++ panels call two-arg Clear with canvas, Rust panels call one-arg Clear without canvas)
- Layer: 6 + 7a + 8
- Failure mode: any panel that fills its background with `painter.Clear(color)` produces no pixels in the parallel/display-list render branch (the only branch in use when `dirty_count > 1` and `render_pool.GetThreadCount() > 1`). The pre-fill at row 2 leaves the buffer at `background_color` (currently BLACK/TRANSPARENT for the desktop/cosmos), so the panel area renders the pre-fill color instead of the panel's chosen color.
- F010-fitting scenario: emDirPanel.rs:459 `painter.Clear(dc)` (dc = DirContentColor light grey) is dropped → panel interior appears as pre-fill (BLACK). Symptom X. emDirStatPanel.rs:145 `painter.Clear(bg_color)` is dropped → six-field info pane background never painted; subsequent `PaintTextBoxed` calls render text in their fg color but against the BLACK pre-fill, which produces unreadable / black-on-black for darkened text colors. Symptom Z. Border-image gradients in emDirEntryPanel use `PaintBorderImage`/`PaintLinearGradient`, both of which **do** record (PaintBorderImage at `emPainter.rs:4476-4496`, PaintLinearGradient at `:1585`), so they paint correctly. Symptom Y.

### Category B (rules 1+2+3, 3 rules) — Layer 1: render-strategy split

- Rule(s) fired: R1 (no equivalence check between branches), R2 (strategies are RUST_ONLY), R3 (C++ has one path)
- Layer: 1
- Failure mode: bugs may manifest only under one strategy. The display-list branch at `emWindow.rs:661-676` is taken when `render_pool.GetThreadCount() > 1 && dirty_count > 1`; the per-tile branch at `:677-696` and viewport-buffer branch at `:636-660` use `emPainter::new` (direct mode), where `Clear` works.
- F010-fitting scenario: Symptoms X+Z appear only when display-list path is selected; symptoms vanish when per-tile direct path runs (e.g. when only one tile is dirty, or when render thread count is 1). This is a direct check the synthesizer can run.

### Category C (rules 1+2+3, 3 rules) — Layer 2: per-frame background_color capture and propagation

- Rule(s) fired: R1 (only enforced at call sites — no check that all pre-fill sites are correlated), R2 (no pre-fill in C++), R3 (C++ relies on painter.Clear, not on buffer pre-fill)
- Layer: 2
- Failure mode: pre-fill site not updated to track `view.background_color`, or panels rely on pre-fill value differing from what arrives. This is the F018 contract surface; rule I.5 is what the recent `set_background_color` change was supposed to enforce.
- F010-fitting scenario: if `view.background_color` is BLACK or TRANSPARENT (the default before a panel sets a different bg), the pre-fill is BLACK; any panel that depends on `Clear(color)` to overwrite the pre-fill is invisible. Symptoms X+Z. The existing F018 work documents that the pre-fill itself is correct; the failure is upstream — panels do not paint, so the pre-fill is what shows.

### Category D (rules 1+2, 2 rules) — Layer 14: tile composite re-blending

- Rule(s) fired: R1 (alpha re-blend at composite is unasserted), R2 (no analogue in C++)
- Layer: 14
- Failure mode: tile RGBA contains semi-transparent pixels (e.g. anti-aliased text against `TRANSPARENT` canvas record); `BlendState::ALPHA_BLENDING` in the composite pass blends them against the wgpu-clear background, darkening or shifting their effective color from what direct rasterization would produce.
- F010-fitting scenario: explains color-shift but not solid-black panel interior. Most plausibly contributes to Symptom Z (text appearing dimmer/missing) but does not by itself produce Symptom X (an opaque rect is dropped, not blended). Lower prior than A/B/C.

### Category E (rules 2+3, 2 rules) — Layer 9: `DrawList::replay` state-snapshot equivalence

- Rule(s) fired: R2 (RUST_ONLY), R3 (no C++ analogue)
- Layer: 9
- Failure mode: replay reuses the painter's running state (push/pop/clip/transform via DrawOps) but the per-op `RecordedState` snapshot is **not** consulted. If recording inserts a state mutation that bypasses `record_state` (or omits a DrawOp for a state change), replay desyncs from recording.
- F010-fitting scenario: would produce mis-clipped or mis-translated paints, not a missing solid Clear. Lower prior than A/B/C for the specific X+Z+Y pattern.

### Category F (rules 2+3, 2 rules) — Layer 11: parallel-replay determinism / Send-Sync soundness

- Rule(s) fired: R2 (RUST_ONLY), R3 (no analogue)
- Layer: 11
- Failure mode: `unsafe impl Send/Sync for DrawOp` (the variants holding `*const emImage`) are unchecked. If a panel mutates a referenced emImage between record and replay, replay reads stale or invalid pixels. Won't cause solid black — the test golden harness uses one-shot trees — but explains intermittent corruption.
- F010-fitting scenario: low prior for X+Z; would not cleanly select against Y.

### Category G (rules 2+3, 2 rules) — Layer 12, 13, 15: GPU upload, surface clear, present

- Rule(s) fired: R2 (RUST_ONLY), R3 (no analogue)
- Layer: 12/13/15
- Failure mode: write_texture format mismatch (`Rgba8UnormSrgb` at `:174` — sRGB-encoded sampler reads the bytes through gamma, but rasterization wrote them in linear space → channel value drift). Could darken text but not erase it.
- F010-fitting scenario: explains a uniform color-cast across all three symptoms; but the report is "X black, Y correct, Z missing" which is mode-specific, not uniform — Y working argues against a global GPU pipeline bug.

### Category H (rules 4 only, 1 rule) — Layer 3: enforcement check is wrong

- Rule(s) fired: R4 (debug_assert scale=1, push/pop pairing — could be a false positive)
- Layer: 3
- Failure mode: `debug_assert!` at `emView.rs:4714` is compiled out in release; push/pop pairing assumes the child loop respects DFS order without intermediate save/restore.
- F010-fitting scenario: low prior — would manifest as transformed/clipped errors, not solid-black panel.

### Category I (rules 4 only, 1 rule) — Layer 4: conditional clear at SVP boundary

- Rule(s) fired: R4 (the `tree.IsOpaque(svp_id)` branch could be wrong)
- Layer: 4
- Failure mode: if `IsOpaque` returns true incorrectly, the SVP-boundary clear is skipped, leaving the pre-fill visible. C++ `emFilePanel::IsOpaque` at `emFilePanel.cpp:187-198` returns true for VFS_LOAD_ERROR / VFS_SAVE_ERROR / VFS_CUSTOM_ERROR. Rust must match.
- F010-fitting scenario: would only matter if a directory panel is the SVP, which is uncommon. Lower prior.

## Coverage notes

### Layers I considered and rejected, with reason

- **Input event dispatch** — out of scope; question is paint, not event handling.
- **Engine tick scheduling** — out of scope; question is "paint pipeline produces wrong pixels," not "engine fails to schedule paint."
- **Panel tree mutation during paint** — `paint_one_panel` takes `&mut tree` and uses `take_behavior` / `put_behavior`, but this affects behavior storage, not the paint output itself. Could produce "behavior moved during paint" but not "paint runs but rect is black."
- **Color packing format** — `emColor` is u32 RGBA. `Rgba8UnormSrgb` upload pulls bytes through gamma; this is in Layer 13 / Category G. If the byte order is wrong, *every* color is wrong, not selectively. Symptom Y arguing against this.

### Files I did not read fully and why

- `emView.rs` — read only `Paint` (lines 4700-4900). The rest of the 8089-line file is panel-tree management, input dispatch, viewport algebra, etc. — out of scope for the *paint pipeline* contract surface.
- `emPainter.rs` — read only the dual-mode dispatch (lines 280-700, plus `Clear`, `ClearWithCanvas`, `PaintRect` heads). The 10172-line file's many `Paint*` methods all follow the same `try_record` pattern (verified by grep for all `try_record` sites), so reading each one would not change the categorization.
- `emPainterDrawList.rs` — read replay header and DrawOp enum head; the per-variant arms (lines 446-996) are mechanical dispatches that call back into `emPainter`'s direct-mode methods.
- `emWindow.rs` — read only `render` and `render_parallel_inner`. Surface boot and resize are out of scope for "paint produces wrong pixels."
- `emGUIFramework.rs` — read only `wgpu::Instance` boot. Beyond surface materialization, it's app lifecycle.

### Open questions for the synthesizer

1. **Which strategy branch is active when X+Z manifest?** A targeted check: force `render_pool.GetThreadCount() = 1` (or kill the `dirty_count > 1` condition at `emWindow.rs:661`) and see if symptoms vanish. If they vanish, Category A or B is confirmed; if they persist, Category D / E / G are higher prior.
2. **Why does the viewport-buffer branch (`emWindow.rs:636-660`) work?** It uses `emPainter::new` (direct mode), so single-arg `Clear` rasterizes — Symptoms X+Z would not appear in this branch. Confirms Category A's mechanism but not which branch is selected at runtime when symptoms reproduce.
3. **Are there other `require_direct`-without-paired-DrawOp call sites?** Layer 7a's table claim ("`Clear(color)` is the only public API that uses `require_direct` directly") is from grep; should be re-verified. Search: `require_direct` appears at `emPainter.rs:1052,3277` (these are inside methods that already `try_record` at the top — guarded), and at `emPainter.rs:5776` (`Clear`). The other matches in the grep dump are `proof: DirectProof` parameters of private helpers — they receive a proof, not call `require_direct`.
4. **Does `DrawOp::Clear` need to be added, or should `painter.Clear(color)` callers be migrated to `ClearWithCanvas`?** C++ panels use two-arg `Clear(c, canvasColor)`. Migrating Rust panels to `ClearWithCanvas(color, canvas_color)` (which records as PaintRect) would make the bug structural rather than at the API surface — the simpler fix.
5. **What is `view.background_color` at the time X reproduces?** The observed pre-fill color (BLACK) implies `view.background_color = BLACK` (or `TRANSPARENT`, then composited against wgpu LoadOp::Clear which is itself `view.background_color`). This needs runtime confirmation — could affect Category C plausibility.
