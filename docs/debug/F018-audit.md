# F018 Audit — Compositor Integration Contract Compliance

Audit findings for the contract spec at
`docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md`.
Each section below maps to one open question or contract rule and records the
current Rust port's compliance status, evidence, and notes for the remediation
plan.

## Status legend

- **COMPLIANT** — current code provably satisfies the rule under all scenarios in scope.
- **VIOLATION** — current code provably fails the rule in at least one observable scenario.
- **PARTIAL** — code satisfies some scenarios but provably fails others; both noted.
- **INCONCLUSIVE** — verification deferred to remediation phase (requires test harness, visual check, or downstream open question).

---

## Open Questions

### O.1 — OS-driver canvas color initial value

**Question:** What value does the C++ OS driver pass as `canvasColor` to the top-level `emView::Paint` call? (Spec rule II.1.)

**Investigation:** The OS-driver entry point is `emViewRenderer::ThreadRun` and the single-threaded fallback in `emViewRenderer.cpp`. Both invoke the view through `emViewPort::PaintView`:
- `~/Projects/eaglemode-0.96.4/src/emCore/emViewRenderer.cpp:109` — `CurrentViewPort->PaintView(painter,0);`
- `~/Projects/eaglemode-0.96.4/src/emCore/emViewRenderer.cpp:140` — `CurrentViewPort->PaintView(painter,0);`

`PaintView` forwards to `emView::Paint(painter, canvasColor)`. The literal second argument is `0`, i.e. `emColor(0)` = RGBA(0,0,0,0) = transparent black.

**Finding:** C++ passes `emColor(0)` (transparent black) as the canvas-color argument to `emView::Paint`. This is the same value Rust uses (`emColor::TRANSPARENT`).

**Implication:** Rule II.1 audit (Task 14) is COMPLIANT. The Rust top-level canvas-color choice matches C++ exactly; no remediation needed at the OS-driver entry point.

### O.2 — Rust emPainter canvas-color carrier

**Question:** Does the Rust `emPainter` carry canvas color as a member, and if so where is it set/updated? (Spec rule II.5.)

**Investigation:** Read `emPainter` field declarations and grepped for SetCanvasColor/GetCanvasColor across `crates/emcore/src/`.
- Carrier field: `crates/emcore/src/emPainter.rs:200` — `canvas_color: emColor` inside `PainterState`. Initialized to `emColor::TRANSPARENT` in both `new` (line 547) and `new_recording` (line 581). Accessors `GetCanvasColor` (720) / `SetCanvasColor` (725).
- External SetCanvasColor call sites (panel paint code that updates the painter carrier mid-paint):
  - `crates/emcore/src/emView.rs:4770` — after the conditional clear, before SVP paint (mirrors C++ `emView.cpp:1083` `canvasColor=ncc`).
  - `crates/emcore/src/emView.rs:4812` — per-child loop, before each `paint_one_panel` (mirrors C++ `emView.cpp:1118` `p->Paint(pnt, p->CanvasColor)`).
  - `crates/emcore/src/emButton.rs:210`, `emCheckButton.rs:116`, `emRadioBox.rs:166`, `emRadioButton.rs:353`, `emBorder.rs:2038, 2246` — panels updating the carrier when they paint sub-content with a different canvas color (matches C++ practice of passing canvas color to nested paint calls).
- External GetCanvasColor readers (panel paint code reading the carrier instead of accepting a parameter): `emFilePanel.rs:166`, `emTunnel.rs:145`, `emButton.rs:191`, `emSplitter.rs:140`, `emCheckButton.rs:97`, `emFileSelectionBox.rs:129, 633`, `emScalarField.rs:344`. These are panels that read canvas color from the painter carrier rather than from a `Paint(canvasColor)` parameter.

**Finding:** The Rust painter DOES carry canvas color as a member field. The carrier is set/updated at the C++-equivalent points (after the conditional clear and per-child) — alignment is correct. The structural divergence is that C++ threads canvas color as an explicit parameter to every `emPanel::Paint` call, while Rust calls `SetCanvasColor` on the painter and lets panels read it via `GetCanvasColor`.

**Implication:** Rule II.5 audit (Task 18) records the carrier as a structural divergence (preserved-design-intent in C++ being expressed as carrier-state in Rust). Functionally the carrier is updated at every C++ update point that the spec investigation identified, so the canvas-color value seen by each panel matches C++. The carrier itself is not a F018 root cause; remediation may keep it but should document the divergence.

### O.3 — Per-tile painter clip rect

**Question:** In the per-tile single-threaded path (`emWindow.rs:668-687`), does the painter's clip rect cover the tile bounds or the viewport bounds? (Spec rule I.3.)

**Investigation:** Read `emPainter::new` constructor and the per-tile path in `emWindow.rs`.
- `emPainter::new(target: &mut emImage)` at `crates/emcore/src/emPainter.rs:524-557` sets `clip = ClipRect { x1: 0, y1: 0, x2: w as f64, y2: h as f64 }` where `w, h` are the target image's dimensions.
- Per-tile path at `crates/emcore/src/emWindow.rs:672-680` calls `emPainter::new(&mut tile.image)` with a 256×256 tile image, then `painter.translate(-(col*ts), -(row*ts))`. `translate` is an offset/scaling change that does not modify the clip rect (the clip rect lives in pixel space of the target image).

**Finding:** In the per-tile path, the painter's clip rect is `0..256 × 0..256` — TILE BOUNDS, not viewport bounds. `painter.ClearWithCanvas(...)` writes the full tile (the clip determines `PaintRect` extent — see `ClearWithCanvas` at `emPainter.rs:865-878`).

**Implication:** Rule I.3 audit (Task 10) treats the per-tile clear as writing the full tile when it fires. The conditional in `emView.rs:4727-4738` evaluates against viewport-rect bounds (rx1..rx2, ry1..ry2), independent of the painter's clip — so the conditional fires identically across tiles. When it fires in the per-tile path, the per-tile pre-fill BLACK at `emWindow.rs:674` is overwritten by the clear color (background or canvas). When it does NOT fire, the pre-fill remains visible in regions not subsequently overpainted by the panel — that's the I.1/III.3 cross-cut.

### O.4 — Recording painter records `Clear`

**Question:** Does the recording painter in `render_parallel_inner` record `Clear` ops, or bypass them? (Spec rule IV.5.)

**Investigation:** Searched `DrawOp` variants and read `ClearWithCanvas` impl.
- `DrawOp` enum at `crates/emcore/src/emPainterDrawList.rs:14` lists 37+ variants. `grep -n 'fn Clear\|DrawOp::Clear\b' emPainterDrawList.rs` returns no results — there is no dedicated `DrawOp::Clear` variant.
- `emPainter::ClearWithCanvas` at `crates/emcore/src/emPainter.rs:865-878` is implemented as a delegated call to `self.PaintRect(x, y, w, h, color, canvas_color)` over the current clip rect. `PaintRect` is recordable as `DrawOp::PaintRect` (`emPainterDrawList.rs:462`).
- `emView.rs:4737` calls `painter.ClearWithCanvas(ncc, canvas_color)` from the conditional-clear block. When invoked on a recording painter, this lowers to a `DrawOp::PaintRect` over the painter's full clip region with the clear's color and canvas-color arguments.

**Finding:** The recording painter DOES record the conditional clear — not as a `Clear` op (no such variant exists) but as a `PaintRect` covering the painter's clip rect. On replay into a per-tile painter, that `PaintRect` is replayed with the tile's current transform/clip, painting the clear color over the appropriate region of the tile.

**Implication:** Rule IV.5 audit (Task 28) is COMPLIANT — the parallel-replay path does see the clear. However, the replay's effective region for the recorded `PaintRect` depends on the painter clip at *record time*, which is the recording painter's viewport-sized clip. Replay into a per-tile painter then re-clips against the tile bounds. The pixel result should match the per-tile single-threaded path — but verifying this end-to-end is V.3 (strategy parity), which is INCONCLUSIVE without a test harness.

### O.5 — Compositor unallocated-tile behavior

**Question:** What does `WgpuCompositor::render_frame` do for tiles that are out of the active grid (resized smaller, or never allocated)? (Spec rules I.2, I.4.)

**Investigation:** Re-read `WgpuCompositor::render_frame`, `new`, `resize`, and the upload path.
- `WgpuCompositor::render_frame` at `crates/emcore/src/emViewRendererCompositor.rs:238-303` opens a render pass with `LoadOp::Clear(wgpu::Color::BLACK)` (line 261) covering the surface, then loops over `self.tiles: Vec<Option<TileGpuData>>` (line 17) drawing only `Some` slots (lines 272-280).
- Slots are `None` after `WgpuCompositor::new` (line 122-123) and after `resize` (line 295). They become `Some` on first call to `upload_tile` (line 152-211).
- `emWindow::render` uploads every dirty tile each frame (`emWindow.rs:683` inside the per-tile path; equivalent in the other strategies). Tiles start dirty (`Tile::new` at `emViewRendererTileCache.rs:22`), so on the first frame after construction or resize every visible tile gets uploaded — slots are `Some` from frame 1 onward in steady state.

**Finding:** For tile slots that are `None`, the wgpu render pass clears them to opaque black via `LoadOp::Clear(wgpu::Color::BLACK)` and does not draw over them. In practice this happens for at most one frame after construction or resize.

**Implication:** Rule I.4 audit (Task 11) treats the load-clear as visible during initial-frame transients AND, more importantly, as the alpha-blend background for any `Some` tile pixels with alpha < 255 (`BlendState::ALPHA_BLENDING` at line 97). Rule I.2 audit (Task 9) treats the tile init color (RGBA 0,0,0,0) as observable in the same way. The dominant load-bearing path for the F018 symptom is NOT the unallocated-tile case but the alpha-blend-through case where tiles ARE uploaded but contain non-opaque pixels.

---

## Cluster I — Pixel Equivalence

### I.1 — Framebuffer pre-state must not be observable

**Status:**
**Evidence:**
**Notes:**

### I.2 — Tile backing-store init color is not observable

**Status:**
**Evidence:**
**Notes:**

### I.3 — Conditional framebuffer clear must mirror C++

**Status:**
**Evidence:**
**Notes:**

### I.4 — Compositor load-clear color must not be observable

**Status:**
**Evidence:**
**Notes:**

### I.5 — Runtime `view.background_color` changes propagate

**Status:**
**Evidence:**
**Notes:**

---

## Cluster II — Canvas-color Propagation

### II.1 — `view.Paint` receives the OS-driver canvas color

**Status:**
**Evidence:**
**Notes:**

### II.2 — SVP receives the conditionally-updated canvas color

**Status:**
**Evidence:**
**Notes:**

### II.3 — Children receive their own `CanvasColor`

**Status:**
**Evidence:**
**Notes:**

### II.4 — Tile boundaries do not perturb canvas color

**Status:**
**Evidence:**
**Notes:**

### II.5 — `emPainter` is not a canvas-color carrier

**Status:**
**Evidence:**
**Notes:**

---

## Cluster III — Non-opaque Composition

### III.1 — Non-opaque SVP reveals view background

**Status:**
**Evidence:**
**Notes:**

### III.2 — Non-opaque child reveals parent

**Status:**
**Evidence:**
**Notes:**

### III.3 — Opaque-panel skip-clear remains valid under tiles

**Status:**
**Evidence:**
**Notes:**

---

## Cluster IV — Dirty-region Soundness

### IV.1 — `InvalidatePainting` propagates to tile cache and compositor

**Status:**
**Evidence:**
**Notes:**

### IV.2 — Painted region shrinking invalidates the difference

**Status:**
**Evidence:**
**Notes:**

### IV.3 — `IsOpaque` change invalidates SVP-choice path

**Status:**
**Evidence:**
**Notes:**

### IV.4 — All three render strategies obey the dirty contract identically

**Status:**
**Evidence:**
**Notes:**

### IV.5 — Recording-painter ops must record the conditional clear

**Status:**
**Evidence:**
**Notes:**

---

## Cluster V — Acceptance Criteria

### V.1 — F018 repro: `VFS_WAITING`/`VFS_LOADING` background is grey

**Status:**
**Evidence:**
**Notes:**

### V.2 — Background-color change visibly propagates

**Status:**
**Evidence:**
**Notes:**

### V.3 — Strategy parity

**Status:**
**Evidence:**
**Notes:**

### V.4 — Painted-region shrink shows no ghost

**Status:**
**Evidence:**
**Notes:**

### V.5 — Opacity transition rebuilds framebuffer

**Status:**
**Evidence:**
**Notes:**

---

## Summary

(Filled in by Task 36: total compliant / violation / partial / inconclusive counts, ordered list of violations to address, and any newly-discovered acceptance criteria.)
