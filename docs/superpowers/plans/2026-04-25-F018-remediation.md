# F018 Compositor Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the four root-cause violations identified by the F018 audit (I.1 BLACK pre-fills, I.4 BLACK compositor load-clear, IV.3 missing `SVPChoiceByOpacityInvalid` set-to-true, II.5 painter as canvas-color carrier) so the Rust port matches the C++ paint contract documented in `docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md`.

**Architecture:** Three phases. Phase 1 plumbs `view.background_color` through the render pipeline (one new piece of compositor state) and replaces four hardcoded BLACK fills, gated by a new behavioral test that asserts the configured background color is visible during `VFS_LOADING`. Phase 2 mirrors C++'s `SVPChoiceByOpacityInvalid = true` write inside Rust's two `emView::InvalidatePainting` overloads. Phase 3 removes the canvas-color carrier from `emPainter`, threading canvas color as an explicit parameter through `PanelBehavior::Paint`.

**Tech Stack:** Rust workspace (`cargo`, `cargo-nextest`, `cargo clippy`), wgpu compositor, single-threaded UI tree under `Rc<RefCell<…>>`. C++ reference at `~/Projects/eaglemode-0.96.4/`. Pre-commit hook runs `cargo fmt`, then `cargo clippy -- -D warnings`, then `cargo nextest run`.

**Inputs (read these first):**
- `docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md` — the contract.
- `docs/debug/F018-audit.md` — per-rule compliance status with evidence.
- `docs/superpowers/specs/2026-04-25-F018-compositor-remediation-design.md` — the design this plan implements.

---

## Phase 1 — Plumb `view.background_color` through the render path

Closes I.1, I.4, plus transitively I.2, I.5 (compositor layer), III.1, III.2, III.3, V.1.

The phase replaces four hardcoded `emColor::BLACK` / `wgpu::Color::BLACK` sites with `view.background_color`, threaded from the view through the window into the compositor.

### Task 1: Add a V.1 behavioral test (failing)

**Files:**
- Create: `crates/emcore/tests/f018_v1_loading_background.rs`

**Note for executor.** This test asserts that the SoftwareCompositor's framebuffer shows `view.background_color` (not `BLACK`) in regions where an `emFilePanel` does not paint during `VFS_LOADING`. It exercises the I.1 pre-fill site at `crates/emcore/src/emViewRenderer.rs:37`. It does **not** exercise the wgpu `LoadOp::Clear` site (`emViewRendererCompositor.rs:261`) — that path requires a wgpu device and is verified by the manual visual gate at the end of the phase (Task 8). If this test passes before the fix lands, do not lower the bar — switch the test to use `SoftwareCompositor::render_parallel` (a different code path that has its own pre-fill at `:87` and `:99`); document which path actually surfaces the bug. The behavioral assertion (background color visible where the panel does not paint) stays the same.

- [ ] **Step 1: Write the failing test**

```rust
//! V.1 acceptance harness for F018: VFS_LOADING background must be
//! `view.background_color`, not BLACK.
//!
//! Spec: docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md §V.1
//! Audit: docs/debug/F018-audit.md §III.1, §I.1, §I.4

use emcore::emColor::emColor;
use emcore::emFilePanel::{emFilePanel, VfsState};
use emcore::emPanelTree::PanelTree;
use emcore::emView::emView;
use emcore::emViewRenderer::SoftwareCompositor;

/// Spec V.1: a non-opaque SVP (emFilePanel during VFS_LOADING) must reveal
/// `view.background_color` in regions it does not paint.
#[test]
fn vfs_loading_reveals_background_color() {
    // Use a distinct, non-default color so the assertion proves we observe
    // the configured value, not a default that coincidentally matches.
    let bg = emColor::rgba(0xFF, 0x00, 0x00, 0xFF); // opaque red

    let (mut tree, panel_id) = build_loading_directory_panel();
    let mut view = build_view_with_svp(&mut tree, panel_id);
    view.SetBackgroundColor(bg);

    let mut compositor = SoftwareCompositor::new(256, 256);
    compositor.render(&mut tree, &view);

    // Sample a pixel known to be inside the panel clip rect but outside
    // the centered "Loading…" text region. Top-left of the panel is safe.
    let fb = compositor.framebuffer();
    let px = fb.GetPixel(10, 10);
    let tol = 4; // matches tests/golden/common.rs channel tolerance
    assert!(
        channel_diff(px, bg) <= tol,
        "expected background red ~{:?}, got {:?}",
        bg,
        px,
    );
}

fn channel_diff(a: emColor, b: emColor) -> i32 {
    let (ar, ag, ab, _aa) = (a.GetRed() as i32, a.GetGreen() as i32, a.GetBlue() as i32, a.GetAlpha() as i32);
    let (br, bg_, bb, _ba) = (b.GetRed() as i32, b.GetGreen() as i32, b.GetBlue() as i32, b.GetAlpha() as i32);
    (ar - br).abs().max((ag - bg_).abs()).max((ab - bb).abs())
}

// Construct a minimal PanelTree containing one emFilePanel forced into
// VFS_LOADING. The panel is registered as the SVP via build_view_with_svp.
fn build_loading_directory_panel() -> (PanelTree, emcore::emPanelTree::PanelId) {
    // EXECUTOR: confirm the exact constructors before writing this body —
    // emFilePanel may need a model + path; if VFS_LOADING is gated on a
    // real model load, use a synthetic model that stays in the loading
    // state (search emFilePanel.rs for VfsState::VFS_LOADING setters).
    todo!("see EXECUTOR note above");
}

fn build_view_with_svp(
    tree: &mut PanelTree,
    panel_id: emcore::emPanelTree::PanelId,
) -> emView {
    // EXECUTOR: confirm the SVP-installation API on emView. Look for
    // tests in crates/emcore/tests/ or crates/emcore/src/emView.rs that
    // wire up an SVP without going through the full engine; mirror the
    // pattern here.
    todo!("see EXECUTOR note above");
}
```

The two `todo!()` bodies are flagged for the executor because the exact panel-tree construction API is the kind of fact you should not guess — it is faster for the executor (with the codebase open) to confirm by reading `crates/emcore/tests/` for an existing pattern than for this plan to spell it out. Replace `todo!()` with the concrete constructor calls, do not commit `todo!()` into the test.

- [ ] **Step 2: Run the test and confirm it fails**

```bash
cargo nextest run --test f018_v1_loading_background
```

Expected: FAIL. Either compilation fails on the `todo!()` (until the executor fills in the constructors), or, once the constructors are in, the assertion fails because the sampled pixel is BLACK or near-black, not red.

If the test passes before the fix lands, see the executor note above — switch to `render_parallel` and confirm that path surfaces the bug.

- [ ] **Step 3: Commit the failing test**

```bash
git add crates/emcore/tests/f018_v1_loading_background.rs
git commit -m "test(F018): add V.1 failing harness — VFS_LOADING background color

Asserts SoftwareCompositor framebuffer shows view.background_color
(red, configured per-test) in non-painted regions during VFS_LOADING.
Currently fails — fixed in the next commit by Phase 1 plumbing.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Add `background_color` to `WgpuCompositor`

**Files:**
- Modify: `crates/emcore/src/emViewRendererCompositor.rs:13-22, 31-135, 237-303`

The compositor gains one piece of view state — the background color used by the wgpu `LoadOp::Clear`. This is the "plumb view→compositor" data path the spec calls for.

- [ ] **Step 1: Add the field**

Edit `crates/emcore/src/emViewRendererCompositor.rs`. In the struct definition (currently at `:13-22`), add a `background_color` field of type `crate::emColor::emColor`:

```rust
pub struct WgpuCompositor {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    tiles: Vec<Option<TileGpuData>>,
    cols: u32,
    rows: u32,
    viewport_width: u32,
    viewport_height: u32,
    background_color: crate::emColor::emColor,
}
```

- [ ] **Step 2: Initialize the field in `new`**

Inside `WgpuCompositor::new`, just before the `Self { … }` literal at the end (`:125-134`), the `Self` literal grows one field. Default to opaque black so behavior on a freshly-constructed compositor with no `set_background_color` call is identical to today (the test at Task 1 sets `view.background_color` explicitly):

```rust
        Self {
            pipeline,
            bind_group_layout,
            sampler,
            tiles,
            cols,
            rows,
            viewport_width,
            viewport_height,
            background_color: crate::emColor::emColor::BLACK,
        }
```

- [ ] **Step 3: Add a setter**

Inside `impl WgpuCompositor`, after `viewport_size` at `:299-302` (or wherever the existing accessors live), add:

```rust
    /// Set the background color used by the wgpu render pass `LoadOp::Clear`.
    /// Must be called every frame from the render driver before
    /// [`Self::render_frame`], so the load-clear reflects any runtime change
    /// to `view.background_color` (per F018 contract rule I.5).
    pub fn set_background_color(&mut self, color: crate::emColor::emColor) {
        self.background_color = color;
    }
```

- [ ] **Step 4: Replace `LoadOp::Clear(wgpu::Color::BLACK)` at `:261`**

Currently `:261` reads:
```rust
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
```

Replace with a conversion from `self.background_color` to `wgpu::Color`. `wgpu::Color` is f64 0.0–1.0 per channel. `emColor` exposes `GetRed/GetGreen/GetBlue/GetAlpha` returning u8 0–255. The pipeline format is `Rgba8UnormSrgb` (line 163), so the load-clear color goes through sRGB encoding by wgpu — the f64 channel values must be the *linear* representation. The simplest faithful conversion is to keep the BLACK behavior bit-exact when `background_color == BLACK` and otherwise convert each u8 channel to f64 via `u8 as f64 / 255.0`. wgpu handles sRGB encoding on store.

```rust
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.background_color.GetRed() as f64 / 255.0,
                        g: self.background_color.GetGreen() as f64 / 255.0,
                        b: self.background_color.GetBlue() as f64 / 255.0,
                        a: self.background_color.GetAlpha() as f64 / 255.0,
                    }),
```

If the executor finds (via Task 8 visual gate) that the load-clear color appears too light or too dark relative to the in-tile grey, the conversion is wrong direction on sRGB — switch to a linear-space conversion using `(c / 255.0).powf(2.2)` per channel and re-verify. Do not skip the visual gate.

- [ ] **Step 5: Apply the same plumbing to `SoftwareCompositor`**

Edit `crates/emcore/src/emViewRenderer.rs`. The struct at `:10-13` has no `background_color` field today. Add it, default-init to BLACK, expose a setter, and replace the two `framebuffer.fill(emColor::BLACK)` sites and the per-thread tile-buffer fill in `render_parallel`:

```rust
pub struct SoftwareCompositor {
    framebuffer: emImage,
    model: PainterModel,
    background_color: emColor,
}

impl SoftwareCompositor {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: emImage::new(width, height, 4),
            model: PainterModel::load_from_config(),
            background_color: emColor::BLACK,
        }
    }

    pub fn set_background_color(&mut self, color: emColor) {
        self.background_color = color;
    }

    // … existing methods …
}
```

Inside `render_with_setup` at `:33-42`, replace `:37`:
```rust
        self.framebuffer.fill(self.background_color);
```

Inside `render_parallel` at `:50-112`, replace `:87` (per-thread tile buffer fill):
```rust
                buf.fill(self.background_color);
```
And `:99` (post-replay framebuffer fill):
```rust
        self.framebuffer.fill(self.background_color);
```

The closure at `:81-94` captures `self.background_color` by value if it's `Copy`. `emColor` is a `#[derive(Copy, Clone)]` `u32` newtype — confirm by reading `crates/emcore/src/emColor.rs`. If for any reason a borrow-of-self pattern conflicts with the existing closure capture of `model`, follow the same pattern used for `model` at `:73`:

```rust
        let bg = self.background_color;
        // … then use `bg` inside the closure …
```

- [ ] **Step 6: Run `cargo check`**

```bash
cargo check -p emcore
```

Expected: SUCCESS. The new field, setter, and fill-color replacement should compile cleanly. If a borrow-checker complaint about `self` inside the parallel closure surfaces, apply the `let bg = self.background_color;` pattern from Step 5.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emViewRendererCompositor.rs crates/emcore/src/emViewRenderer.rs
git commit -m "feat(F018): plumb background_color to WgpuCompositor and SoftwareCompositor

Adds a background_color field + setter to both compositors. Replaces
hardcoded BLACK in WgpuCompositor::render_frame load-clear and in the
three SoftwareCompositor fill sites (render framebuffer pre-fill, tile
buffer pre-fill, post-replay framebuffer fill).

Closes F018 contract rule I.4 mechanism (compositor side); the wiring
from view.background_color through emWindow lands in the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Wire `view.background_color` from `emWindow` into the compositors

**Files:**
- Modify: `crates/emcore/src/emWindow.rs:567-701` (render method) and `:713-789` (render_parallel_inner)

This task replaces the three `emColor::BLACK` fills inside `emWindow::render` and `render_parallel_inner` with `view.background_color`, and pushes the value into `WgpuCompositor` once per frame.

- [ ] **Step 1: Capture `background_color` once at frame start**

Inside `emWindow::render` at `:567-701`, after the `Self` destructure and the `(winit_window, surface, …)` match (around `:579-600`), add:

```rust
        let background_color = view.GetBackgroundColor();
        compositor.set_background_color(background_color);
```

Place this before the dirty-tile counting loop at `:618-626`. The value is captured into `background_color` once for use by all three render strategies; the call to `compositor.set_background_color` updates the wgpu load-clear that runs at frame composite time.

- [ ] **Step 2: Replace the three `tile.image.fill` / `viewport_buffer.fill` BLACK sites**

`crates/emcore/src/emWindow.rs:632`:
```rust
            viewport_buffer.fill(background_color);
```

`crates/emcore/src/emWindow.rs:674`:
```rust
                        tile.image.fill(background_color);
```

`crates/emcore/src/emWindow.rs:767` (inside `render_parallel_inner`'s closure body): the closure cannot borrow `view` because the surrounding code already moved into the closure context. Follow the same pattern used for `draw_list_ref` and `dirty_ref`:

In the body of `render_parallel_inner` at `:713-789`, add a parameter `background_color: emColor` to the function signature so it is passed in from the caller. Update the call at `:656-667` to pass `background_color`:

```rust
            Self::render_parallel_inner(
                view,
                tile_cache,
                compositor,
                surface_config,
                render_pool,
                tree,
                gpu,
                cols,
                rows,
                tile_size,
                background_color,
            );
```

And update the function signature (`:713-723`):

```rust
    #[allow(clippy::too_many_arguments)]
    fn render_parallel_inner(
        view: &mut emView,
        tile_cache: &mut TileCache,
        compositor: &mut WgpuCompositor,
        surface_config: &wgpu::SurfaceConfiguration,
        render_pool: &mut emRenderThreadPool,
        tree: &mut crate::emPanelTree::PanelTree,
        gpu: &GpuContext,
        cols: u32,
        rows: u32,
        tile_size: u32,
        background_color: emColor,
    ) {
```

Inside the closure body at `:763-774`, replace `:767`:
```rust
                buffer.fill(background_color);
```

`background_color` is `emColor` which is `Copy`, so the closure capture is by value — no `Arc`/`Mutex` plumbing required.

- [ ] **Step 3: Run `cargo check` and `cargo clippy`**

```bash
cargo check -p emcore && cargo clippy -p emcore -- -D warnings
```

Expected: SUCCESS. If clippy complains about `too_many_arguments`, it is suppressed by the existing `#[allow(clippy::too_many_arguments)]` attribute on `render_parallel_inner` (`:712`); the new parameter does not change the suppression.

- [ ] **Step 4: Re-run the V.1 test and confirm it now passes**

```bash
cargo nextest run --test f018_v1_loading_background
```

Expected: PASS.

- [ ] **Step 5: Run the full test suite**

```bash
cargo nextest run
```

Expected: SUCCESS, no regressions. If any pre-existing golden test that depended on the BLACK pre-fill regresses, see Task 4 below — it is a test-side change, not a fix-side regression.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emWindow.rs
git commit -m "feat(F018): wire view.background_color through emWindow render path

Replaces three BLACK pre-fills (single-buffer fallback, per-tile,
parallel per-thread tile buffer) with view.background_color. Pushes
the value into WgpuCompositor::set_background_color once per frame so
the load-clear reflects runtime SetBackgroundColor changes (rule I.5
compositor layer).

Combined with the previous commit, closes F018 contract rules I.1
(framebuffer pre-state observable) and I.4 (compositor load-clear
observable). V.1 behavioral test now passes.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Update any golden tests that asserted BLACK pre-fill

**Files:**
- Modify (if needed): `crates/eaglemode/tests/golden/*.rs`, `tests/golden/common.rs`

Run the suite — if every golden test passes, this task is a no-op and you skip to Task 5.

If a golden test fails because its expected fixture was generated under the BLACK-pre-fill regime and now sees the new background color in the same uncovered region, the fixture needs regeneration:

- [ ] **Step 1: Identify regressed tests**

```bash
cargo nextest run 2>&1 | grep FAIL
```

- [ ] **Step 2: Verify the regression is the F018 fix, not a different bug**

For each failing golden test, run `scripts/verify_golden.sh <name>` (per `CLAUDE.md` §Verification tooling) and confirm the divergence is localized to background regions — not text, geometry, or coverage shifts. If it is, the fixture is stale; if not, the F018 fix has unintended consequences and must be investigated before continuing.

- [ ] **Step 3: Regenerate fixtures (only if step 2 confirmed background-only divergence)**

```bash
scripts/verify_golden.sh --regen
```

Per `CLAUDE.md`: "Use only when intentionally updating the baseline; overwrites golden data files." The C++ baseline does not have a BLACK pre-fill (C++ has no pre-fill at all — see contract spec §I.1 C++ reference), so post-fix Rust output should converge toward C++ output, not diverge from it. If the regenerated fixture diverges from C++ ops via `python3 scripts/diff_draw_ops.py <name> --no-table`, the F018 fix is wrong, not the fixture.

- [ ] **Step 4: Commit (only if regeneration was needed)**

```bash
git add crates/eaglemode/tests/golden/<paths>
git commit -m "test(F018): regenerate golden fixtures after I.1/I.4 fix

Pre-F018 fixtures captured the Rust-only BLACK pre-fill in regions
where C++ never had any pre-fill. With Phase 1 of F018 remediation
landed, those regions now show view.background_color, matching C++
behavior. Re-validated against C++ ops via diff_draw_ops.py.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Manual visual gate (V.1 in the binary)

**Files:** none (manual run).

The V.1 behavioral test exercises only the SoftwareCompositor pre-fill site. The wgpu `LoadOp::Clear` path (and the alpha-blend-through I.4 case) is verified by direct observation against C++ Eagle Mode 0.96.4.

- [ ] **Step 1: Build and run the eaglemode binary**

```bash
cargo run --release --bin eaglemode
```

(Per the in-memory feedback file `feedback_timeout_for_app_launch.md`: launching the GUI binary may block; use a short timeout if running this from automation.)

- [ ] **Step 2: Reproduce the F018 scenario**

Per `docs/debug/ISSUES.json` F018 repro: navigate to a directory and zoom in. While the directory is in `VFS_WAITING` / `VFS_LOADING` (panel briefly shows the loading text), observe the panel background color.

- [ ] **Step 3: Verify it is grey, not BLACK**

Expected: `view.background_color` (default `0x808080` grey, set at `crates/emcore/src/emView.rs:664`) is visible during loading, identical to C++ Eagle Mode 0.96.4 in the same scenario.

If grey: Phase 1 is complete.

If still BLACK: the wgpu load-clear path likely has a sRGB conversion issue (see Task 2 Step 4 note) or the alpha-blend-through case is not yet covered. Investigate by:
1. Patching the load-clear color to a distinct red (`emColor::rgba(0xFF, 0, 0, 0xFF)`) temporarily, re-run, observe — if the user still sees BLACK, the load-clear is being painted over by alpha=255 tile content (the bug is elsewhere). If the user sees red, the load-clear is the leak source.
2. If load-clear is the leak source: the sRGB conversion is wrong (see Task 2 Step 4) — switch to the linear-space conversion.

Do not move to Phase 2 until the visual gate passes.

---

## Phase 2 — Set `SVPChoiceByOpacityInvalid` on invalidation

Closes IV.3. Independent of Phase 1.

### Task 6: Add a TDD test for `SVPChoiceByOpacityInvalid` propagation

**Files:**
- Create: `crates/emcore/tests/f018_iv3_svpchoice_invalidation.rs`

- [ ] **Step 1: Write the failing test**

```rust
//! IV.3: emView::InvalidatePainting must set SVPChoiceByOpacityInvalid = true
//! to mirror C++ emPanel::InvalidatePainting (emPanel.cpp:1284-1290, 1296-1302).
//!
//! Spec: docs/superpowers/specs/2026-04-25-F018-compositor-integration-contract-design.md §IV.3

use emcore::emPanelTree::PanelTree;
use emcore::emView::emView;

#[test]
fn invalidate_painting_sets_svp_choice_by_opacity_invalid() {
    let (mut tree, panel_id) = build_minimal_tree();
    let mut view = emView::new();
    // Wire panel into view as a viewed panel — exact API per existing tests
    // in crates/emcore/src/emView.rs (search for fn paint_basic_smoke or
    // similar). EXECUTOR: confirm and replace the todo!() below.
    install_panel_as_viewed(&mut view, &mut tree, panel_id);

    assert!(!view.SVPChoiceByOpacityInvalid, "precondition");

    view.InvalidatePainting(&tree, panel_id);

    assert!(view.SVPChoiceByOpacityInvalid,
            "InvalidatePainting must set SVPChoiceByOpacityInvalid = true");
}

#[test]
fn invalidate_painting_rect_sets_svp_choice_by_opacity_invalid() {
    let (mut tree, panel_id) = build_minimal_tree();
    let mut view = emView::new();
    install_panel_as_viewed(&mut view, &mut tree, panel_id);

    assert!(!view.SVPChoiceByOpacityInvalid, "precondition");

    view.invalidate_painting_rect(&tree, panel_id, 0.0, 0.0, 1.0, 1.0);

    assert!(view.SVPChoiceByOpacityInvalid,
            "invalidate_painting_rect must set SVPChoiceByOpacityInvalid = true");
}

fn build_minimal_tree() -> (PanelTree, emcore::emPanelTree::PanelId) {
    todo!("EXECUTOR: see Task 1 panel-tree note");
}

fn install_panel_as_viewed(
    _view: &mut emView,
    _tree: &mut PanelTree,
    _panel_id: emcore::emPanelTree::PanelId,
) {
    todo!("EXECUTOR: confirm SVP/viewed-panel install API");
}
```

The two `todo!()`s have the same caveat as Task 1: the panel-tree wiring API is not worth spelling out in this plan when the executor can read it directly. Replace with concrete calls.

- [ ] **Step 2: Run and confirm both tests fail**

```bash
cargo nextest run --test f018_iv3_svpchoice_invalidation
```

Expected: both tests FAIL — `SVPChoiceByOpacityInvalid` stays `false` because no path sets it to `true` (audit IV.3 evidence).

- [ ] **Step 3: Commit the failing tests**

```bash
git add crates/emcore/tests/f018_iv3_svpchoice_invalidation.rs
git commit -m "test(F018): add IV.3 failing tests — SVPChoiceByOpacityInvalid

Asserts that emView::InvalidatePainting and invalidate_painting_rect
both set SVPChoiceByOpacityInvalid = true, mirroring C++
emPanel::InvalidatePainting (emPanel.cpp:1284-1290, 1296-1302).
Currently fails — fixed in the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Set `SVPChoiceByOpacityInvalid = true` in both invalidation overloads

**Files:**
- Modify: `crates/emcore/src/emView.rs:3159-3166, 3173-3215`

- [ ] **Step 1: Edit `InvalidatePainting` (no-arg overload) at `:3159-3166`**

Currently:
```rust
    pub fn InvalidatePainting(&mut self, tree: &PanelTree, panel: PanelId) {
        let p = match tree.GetRec(panel) {
            Some(p) if p.viewed => p,
            _ => return,
        };
        self.dirty_rects
            .push(Rect::new(p.clip_x, p.clip_y, p.clip_w, p.clip_h));
    }
```

Add the flag write after the dirty-rect push. The flag write happens unconditionally — C++ `emPanel.cpp:1284-1290` sets it from inside `emPanel::InvalidatePainting`, which only fires for viewed panels (the early return covers the equivalent guard):

```rust
    pub fn InvalidatePainting(&mut self, tree: &PanelTree, panel: PanelId) {
        let p = match tree.GetRec(panel) {
            Some(p) if p.viewed => p,
            _ => return,
        };
        self.dirty_rects
            .push(Rect::new(p.clip_x, p.clip_y, p.clip_w, p.clip_h));
        self.SVPChoiceByOpacityInvalid = true;
    }
```

- [ ] **Step 2: Edit `invalidate_painting_rect` at `:3173-3215`**

Currently the bottom of the function ends:
```rust
        if vw > 0.0 && vh > 0.0 {
            self.dirty_rects.push(Rect::new(vx, vy, vw, vh));
        }
    }
```

Per C++ `emPanel.cpp:1296-1302`, the rect-arg `InvalidatePainting` sets the flag whenever the call applies (the early-return panel-not-viewed guard already short-circuits the no-op case at the top of the Rust function). C++ sets it inside the `if (vw > 0.0 && vh > 0.0)` block — i.e., only when the invalidation actually contributed a rect. Mirror that:

```rust
        if vw > 0.0 && vh > 0.0 {
            self.dirty_rects.push(Rect::new(vx, vy, vw, vh));
            self.SVPChoiceByOpacityInvalid = true;
        }
    }
```

- [ ] **Step 3: Run the IV.3 tests**

```bash
cargo nextest run --test f018_iv3_svpchoice_invalidation
```

Expected: PASS for both tests.

- [ ] **Step 4: Run the full test suite**

```bash
cargo nextest run
```

Expected: SUCCESS. Setting the flag triggers SVP re-evaluation on the next paint cycle (`emView.rs:2623-2624`); no test should regress on this. If a test does regress, the most likely cause is that `SVPChoiceByOpacityInvalid` was never being set in steady-state Rust runs — and now it is, exposing a latent SVP-re-evaluation bug. Before assuming the F018 fix is wrong, read `emView.rs:2624` (the consumer) and `:1883` (the other clearer site) to understand what the re-evaluation does.

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emView.rs
git commit -m "fix(F018): set SVPChoiceByOpacityInvalid in InvalidatePainting overloads

Mirrors C++ emPanel::InvalidatePainting (emPanel.cpp:1284-1290,
1296-1302). Without this, opacity transitions on a panel did not
trigger SVP re-evaluation — the view continued using the old SVP
even when the panel's IsOpaque() return value changed.

Closes F018 contract rule IV.3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 3 — Remove painter canvas-color carrier

Closes II.5. Widest blast radius — every panel paint method is touched. Independent of Phases 1 and 2.

### Task 8: Inventory and feasibility study

**Files:** read-only — produces a working note saved as part of the commit on Task 9 (or filed as an open question if infeasible).

The painter has internal `saved_canvas; state.canvas_color = canvas_color; … sub-paint … state.canvas_color = saved_canvas` patterns at `crates/emcore/src/emPainter.rs:999-1097, 1137-1315` inside high-level paint methods (`PaintTextBoxed`, `PaintImage`, etc.). These bookkeeping writes live inside the painter and serve to keep `state.canvas_color` correct for any sub-paint method that reads it. The remediation spec calls for *removing* the field; if these internal callers also read it, the field cannot simply be deleted.

- [ ] **Step 1: Enumerate every read of `state.canvas_color` inside `emPainter.rs`**

```bash
grep -n 'self.state.canvas_color\|state\.canvas_color' crates/emcore/src/emPainter.rs
```

For each read site, classify it as:
- **(R)** read of `state.canvas_color` to thread through to a deeper paint helper that takes canvas_color as a parameter — replaceable: pass the explicit param instead.
- **(S)** save/restore bookkeeping — disappears once the field is gone.
- **(U)** read by some method that does not take canvas_color as a parameter — these are the obstacle. They must either gain a parameter or be confirmed unreachable.

- [ ] **Step 2: Write the inventory to the commit message of Task 9 (or to a new open question)**

If every site is (R) or (S), the field can be removed cleanly — proceed to Task 9.

If any site is (U), STOP. File the inventory as a new open question — record the U-class sites with line numbers in `docs/debug/F018-audit.md` under a new "Phase 3 inventory" section, commit the audit edit, and decide before continuing whether to:
- Add a `canvas_color` parameter to the (U) methods (and any callers).
- Or keep the field as a private internal-only implementation detail (matching the spec's option (b) note in §3.3, but downscoped: still remove the public Get/Set accessors used by panel code).

The latter narrows II.5's spec interpretation: the contract violation is panels reading the carrier; the painter's own internal book-keeping is invisible to the contract. If you take this path, document the deviation as a `RUST_ONLY:` (with category `language-forced` if Rust's borrow rules make threading the parameter through a particular helper infeasible without restructuring, or unannotated prose if it is plain idiom adaptation). Do not invent a forced category that does not apply.

---

### Task 9: Add `canvas_color` parameter to `PanelBehavior::Paint`

**Files:**
- Modify: `crates/emcore/src/emPanel.rs:214-216`
- Modify: `crates/emcore/src/emView.rs:4769-4771, 4811-4813, 4871-4877`

The C++ panel paint signature is `emPanel::Paint(emPainter& painter, emColor canvasColor)`. The Rust analogue is `PanelBehavior::Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState)` — width/height plus a state struct, no `canvasColor`. Add a `canvas_color: emColor` parameter.

- [ ] **Step 1: Edit the trait at `crates/emcore/src/emPanel.rs:214-216`**

```rust
pub trait PanelBehavior: AsAny {
    /// Paint the panel's content.
    ///
    /// `canvas_color` is the canvas color the panel may rely on to be
    /// already present at every pixel of its target region (per F018
    /// contract rule II — value of `emPanel::Paint`'s `canvasColor`
    /// parameter in C++).
    fn Paint(
        &mut self,
        _painter: &mut emPainter,
        _canvas_color: emColor,
        _w: f64,
        _h: f64,
        _state: &PanelState,
    ) {}
```

The existing `emColor` import in `emPanel.rs` covers the new type reference; if `emColor` is not in scope, add `use crate::emColor::emColor;` at the top of the file.

- [ ] **Step 2: Edit `paint_one_panel` at `crates/emcore/src/emView.rs:4871-4907`**

Add a `canvas_color: emColor` parameter and pass it through to `behavior.Paint`:

```rust
    fn paint_one_panel(
        &self,
        tree: &mut PanelTree,
        painter: &mut emPainter,
        id: PanelId,
        canvas_color: emColor,
        layout: Rect,
    ) {
        if let Some(mut behavior) = tree.take_behavior(id) {
            // … existing body unchanged up to behavior.Paint …
            behavior.Paint(painter, canvas_color, 1.0, tallness, &state);
            tree.put_behavior(id, behavior);
        }
    }
```

- [ ] **Step 3: Update the two call sites in the dispatcher**

`emView.rs:4769-4771`:
```rust
            // C++ line 1098: p->Paint(pnt, canvasColor)
            self.paint_one_panel(tree, painter, svp_id, canvas_color, svp_layout);
```

`emView.rs:4811-4813`:
```rust
                                // C++ line 1118: p->Paint(pnt, p->CanvasColor)
                                self.paint_one_panel(tree, painter, p, p_canvas, p_layout);
```

(Note: the existing `painter.SetCanvasColor(...)` calls at `:4770` and `:4812` are NOT removed yet — they stay alongside the new parameter so any reader still using `painter.GetCanvasColor()` continues to work during the migration. Removal happens in Task 13 after every reader is migrated.)

- [ ] **Step 4: Run `cargo check`**

```bash
cargo check -p emcore
```

Expected: failure. Every panel that overrides `Paint` will have a signature mismatch. The next task migrates them.

---

### Task 10: Migrate every `PanelBehavior::Paint` implementor to the new signature

**Files (the 11 currently identified — the executor must enumerate any others surfaced by `cargo check`):**
- `crates/emcore/src/emFilePanel.rs` (impl at `:431`)
- `crates/emcore/src/emButton.rs` (impl at `:177`)
- `crates/emcore/src/emCheckButton.rs`
- `crates/emcore/src/emCheckBox.rs`
- `crates/emcore/src/emRadioBox.rs`
- `crates/emcore/src/emRadioButton.rs`
- `crates/emcore/src/emBorder.rs`
- `crates/emcore/src/emTunnel.rs`
- `crates/emcore/src/emSplitter.rs`
- `crates/emcore/src/emFileSelectionBox.rs`
- `crates/emcore/src/emScalarField.rs`
- Plus any others found by `cargo check`'s "trait not implemented" / "method has incompatible signature" errors after Task 9.

For each implementor:

- [ ] **Step 1: Add the `canvas_color: emColor` parameter to the `Paint` impl signature**

Before:
```rust
fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
```

After:
```rust
fn Paint(&mut self, painter: &mut emPainter, canvas_color: emColor, w: f64, h: f64, _state: &PanelState) {
```

- [ ] **Step 2: Replace `painter.GetCanvasColor()` with `canvas_color`**

For each `let canvas_color = painter.GetCanvasColor();` (or similar), delete the line — `canvas_color` is now an in-scope parameter.

For implementations that read the canvas color for their own logic (e.g., to compute a tinted face color), the same parameter is used. Be careful with shadowing: if a panel computes a *new* canvas color for sub-content (e.g., `emButton.rs:210` `painter.SetCanvasColor(face_color)` — see Task 11), use a distinct local name for the new color.

- [ ] **Step 3: Replace intra-panel `painter.SetCanvasColor(new); … sub-paint … painter.SetCanvasColor(old)` patterns**

The 6 intra-panel writers (`emButton.rs:210`, `emCheckButton.rs:116`, `emRadioBox.rs:166`, `emRadioButton.rs:353`, `emBorder.rs:2038, 2246`) update the painter carrier mid-paint to communicate canvas color to sub-paint helpers. Under the parameter shape, the new color is passed as an argument to the sub-paint helper directly.

For each writer site, find the sub-paint method invocation that follows it. If that method already takes a canvas-color parameter (most panel-internal helpers do — they call `painter.PaintTextBoxed(...,canvas_color,...)` etc.), pass the new color as that parameter and drop the `painter.SetCanvasColor` call. If the sub-paint method does not take a canvas-color parameter, add one.

Example for `emButton.rs:210`:

Before:
```rust
        painter.SetCanvasColor(face_color);
        // … sub-paint helpers that read painter.GetCanvasColor() …
```

After:
```rust
        // … same sub-paint helpers, but pass face_color as their
        //     canvas_color parameter ...
```

- [ ] **Step 4: After each file is migrated, `cargo check`**

```bash
cargo check -p emcore
```

Iterate per file until `cargo check` passes. The exact set of intermediate-error states depends on call-graph order; do not assume a particular order.

- [ ] **Step 5: Run `cargo clippy` and `cargo nextest run`**

```bash
cargo clippy -p emcore -- -D warnings && cargo nextest run
```

Expected: SUCCESS. If any panel paint test now fails, it is because the new parameter is being passed wrong from the dispatcher — recheck the dispatcher edits at Task 9 Step 3.

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emPanel.rs crates/emcore/src/emView.rs crates/emcore/src/<all migrated panel files>
git commit -m "refactor(F018): thread canvas_color as PanelBehavior::Paint parameter

Adds canvas_color: emColor to the PanelBehavior::Paint trait method,
mirroring C++ emPanel::Paint(emPainter&, emColor canvasColor). Migrates
every implementor to read the parameter instead of painter.GetCanvasColor().

Painter carrier (emPainter::canvas_color, GetCanvasColor, SetCanvasColor)
is still present and still updated by the dispatcher — removed in the
next commit once no reader remains.

Partial F018 contract rule II.5 fix.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: Verify no production reader of `painter.GetCanvasColor()` remains

**Files:** verification only.

- [ ] **Step 1: Grep for remaining readers**

```bash
grep -rn 'painter\.GetCanvasColor\|painter\.SetCanvasColor' crates/ | grep -v '/tests/' | grep -v 'emPainterDrawList.rs.*p\.SetCanvasColor'
```

Expected: empty. The remaining `p.SetCanvasColor` matches in `emPainterDrawList.rs:1004-1238` are inside test fixtures for the recording painter; they are out-of-scope for this migration (tests that exercise the recording op directly stay).

The `DrawOp::SetCanvasColor` variant at `emPainterDrawList.rs:32` and its replay at `:459` stay for now — see Task 12.

If grep returns any non-test match, return to Task 10 Step 1 and migrate that file.

- [ ] **Step 2: Confirm dispatcher writes are still in place**

```bash
grep -n 'painter\.SetCanvasColor' crates/emcore/src/emView.rs
```

Expected: matches at the two dispatcher sites (`:4770, :4812`). They are removed in Task 13.

---

### Task 12: Decide the fate of `DrawOp::SetCanvasColor`

**Files:**
- Modify (depending on decision): `crates/emcore/src/emPainterDrawList.rs:32, 459`
- Modify (depending on decision): `crates/emcore/src/emPainter.rs:725-727`

`DrawOp::SetCanvasColor` is the recording-painter op that captures `painter.SetCanvasColor` calls. After Task 11, no production code calls `painter.SetCanvasColor` outside the dispatcher — and Task 13 removes those.

Two paths forward:

**Path A — keep `DrawOp::SetCanvasColor`.** It is referenced by tests inside `emPainterDrawList.rs:1004-1238`. Tests have legitimate reason to manipulate painter state directly. The variant stays, the public `SetCanvasColor` method on `emPainter` stays (audit-tagged as test-only or made `#[cfg(test)]`), and the contract is satisfied because no production reader uses the carrier.

**Path B — remove `DrawOp::SetCanvasColor`.** Migrate the test fixtures that still use it to a different mechanism (likely: pass the color as a parameter to whichever paint op is being recorded). Removes the variant, the replay arm, and the painter accessors entirely.

- [ ] **Step 1: Read the test sites at `emPainterDrawList.rs:1004-1238`**

```bash
grep -n 'p\.SetCanvasColor' crates/emcore/src/emPainterDrawList.rs
```

For each match, read 3-5 lines of context to understand what the test is asserting. If every test's purpose is testable by passing canvas_color as a parameter to the next paint op, Path B is feasible. If any test specifically asserts the recording captures `SetCanvasColor` as a separate op, Path A is mandatory.

- [ ] **Step 2: Choose and document**

Add a one-line comment at `emPainterDrawList.rs:32` explaining the chosen path. If Path A:

```rust
    // RUST_ONLY: language-forced — canvas-color is threaded as a per-op parameter
    // in production paint code (see F018 contract II.5), but the recording-painter
    // test fixtures use this op to drive painter state directly without going
    // through a full paint method. Test-only support.
    SetCanvasColor(emColor),
```

If Path B: delete the variant, the replay arm at `:459`, and migrate test sites.

The plan does not pre-decide between A and B because the test-site reading must come first. **Default to Path A** if pressed for time — it is the smaller change and the contract is satisfied either way.

- [ ] **Step 3: `cargo nextest run`**

```bash
cargo nextest run
```

Expected: SUCCESS.

- [ ] **Step 4: Commit (only if Path B was chosen and changes were made)**

```bash
git add crates/emcore/src/emPainterDrawList.rs
git commit -m "refactor(F018): remove DrawOp::SetCanvasColor recording variant

[Path B description: explain test migration]

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

If Path A: the comment annotation can be folded into Task 13's commit.

---

### Task 13: Remove painter canvas-color carrier

**Files:**
- Modify: `crates/emcore/src/emPainter.rs:200, 547, 581, 720-727`
- Modify: `crates/emcore/src/emView.rs:4770, 4812`
- Modify (Path A): `crates/emcore/src/emPainter.rs:725-727` (gate `SetCanvasColor` behind `#[cfg(test)]` or annotate as RUST_ONLY)
- Modify (only if Task 8 step 2 found U-class internal readers): the internal painter saved_canvas/restore patterns

- [ ] **Step 1: Remove the dispatcher `SetCanvasColor` calls**

`emView.rs:4770`: delete the `painter.SetCanvasColor(canvas_color);` line. The next line (`paint_one_panel` call) already passes `canvas_color` as a parameter (Task 9 Step 3).

`emView.rs:4812`: delete the `painter.SetCanvasColor(p_canvas);` line. The next line (`paint_one_panel` call) already passes `p_canvas`.

- [ ] **Step 2: Decide field removal vs. internal-only retention based on Task 8 inventory**

If Task 8 Step 1 classified every internal read as (R) or (S):

- Remove the field from `PainterState` at `emPainter.rs:200`.
- Remove the initializers at `:547, :581`.
- Remove `GetCanvasColor` at `:720-722`.
- Remove `SetCanvasColor` at `:725-727` (or, if Path A in Task 12, keep it gated behind `#[cfg(test)]`).
- Replace every internal (R) read with the explicit canvas_color parameter the enclosing method already takes.
- Delete every internal (S) saved_canvas/restore — the field no longer exists.

If Task 8 found U-class readers and the decision was "keep the field":

- Make the field private and remove only the public accessors.
- Add a doc comment at the field explaining it is internal-only and not the canvas-color carrier the contract forbids.

- [ ] **Step 3: `cargo check`, `cargo clippy`, `cargo nextest run`**

```bash
cargo check -p emcore && cargo clippy -p emcore -- -D warnings && cargo nextest run
```

Expected: SUCCESS.

- [ ] **Step 4: Verify the carrier is gone (or contained)**

```bash
grep -n 'GetCanvasColor\|SetCanvasColor' crates/emcore/src/emPainter.rs
```

Expected (clean field removal): only `DrawOp::SetCanvasColor` (Path A from Task 12) or zero matches (Path B).

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emPainter.rs crates/emcore/src/emView.rs
git commit -m "refactor(F018): remove painter canvas-color carrier

Removes (or makes test-only) emPainter::canvas_color, GetCanvasColor,
SetCanvasColor. Canvas color is now threaded as an explicit parameter
to PanelBehavior::Paint, mirroring C++ emPanel::Paint(emPainter&,
emColor canvasColor).

Closes F018 contract rule II.5.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 4 — Final verification and ISSUES.json

### Task 14: Update `docs/debug/ISSUES.json` F018 status

**Files:**
- Modify: `docs/debug/ISSUES.json`

- [ ] **Step 1: Read the F018 entry**

Open `docs/debug/ISSUES.json` and locate the entry with `id: F018`.

- [ ] **Step 2: Update fields**

- `status` → `fixed` (assuming the visual gate at Task 5 passed and all phases landed).
- `kind` stays `design` (it was reclassified there during the audit; the fix is the design's implementation).
- Add a `fixed_in` or `closed_by` field if the schema supports one — see other recently-closed issues in the file for the convention.
- Append to the issue's notes: a one-line summary citing the spec, audit, and remediation plan paths.

- [ ] **Step 3: Run any ISSUES.json validator that exists in the repo**

```bash
find . -name '*.sh' -path '*issues*' -executable
ls scripts/ | grep -i issue
```

If a validator script exists, run it. If not, commit and move on.

- [ ] **Step 4: Commit**

```bash
git add docs/debug/ISSUES.json
git commit -m "docs(F018): mark issue fixed after remediation plan execution

All four root-cause violations closed:
- I.1, I.4 (Phase 1) — view.background_color plumbed; V.1 visual gate passed
- IV.3 (Phase 2) — SVPChoiceByOpacityInvalid set in InvalidatePainting
- II.5 (Phase 3) — painter canvas-color carrier removed

V.2/V.3/V.4/V.5 acceptance harness deferred to sibling spec.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Self-review summary (for the executor)

Spec coverage this plan claims:
- **I.1** — Tasks 2 Step 5, 3 Step 2 (four pre-fill sites).
- **I.4** — Tasks 2 Step 4 (wgpu LoadOp::Clear), 3 Step 1 (frame-start setter call).
- **I.5** (compositor layer) — Task 3 Step 1 (set_background_color called every frame).
- **IV.3** — Tasks 6, 7 (test + flag set in both overloads).
- **II.5** — Tasks 8–13.
- **V.1** behavioral — Tasks 1, 3 Step 4.
- **V.1** manual visual gate — Task 5.
- **Issue close-out** — Task 14.

Open questions deferred from the spec (OP.1–OP.7):
- OP.1 (compositor plumbing mechanism) — resolved in Task 2 as setter.
- OP.2 (parallel-replay capture) — resolved in Task 3 as parameter to `render_parallel_inner`.
- OP.3 (existing test coverage of opacity transitions) — Task 7 Step 4 acknowledges this; if a regression surfaces, investigate before declaring the plan done.
- OP.4 (paint_one_panel signature) — confirmed by inspection (Task 9 Step 2).
- OP.5 (helper paint methods) — addressed per file in Task 10 Step 3.
- OP.6 (V.1 harness path) — chosen `SoftwareCompositor::render` in Task 1; falls back to `render_parallel` if needed.
- OP.7 (visual gate documented as a step) — Task 5.

Risk hotspots:
- Task 2 Step 4 sRGB conversion direction — if Task 5 visual gate observes wrong-shade grey, switch to linear-space conversion.
- Task 8 inventory — if U-class internal readers exist, narrow II.5 scope and document.
- Task 4 (golden test regenerations) — must verify against C++ ops via `diff_draw_ops.py` before regenerating, not after.
