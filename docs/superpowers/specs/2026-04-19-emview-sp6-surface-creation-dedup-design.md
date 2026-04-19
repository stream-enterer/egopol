# SP6 — W3 Surface-Creation De-duplication — Design

**Date:** 2026-04-19
**Scope:** Extract the ~50-line wgpu/winit surface-init sequence duplicated between `emWindow::create` and `emGUIFramework::materialize_popup_surface` into a single helper.
**Residual origin:** §3.5 item 1 and §8.1 item 13 of `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md`.

## 1. Background

W3 (popup architecture) introduced `OsSurface { Pending, Materialized }` and a `MaterializedSurface` struct holding the winit window, wgpu surface + config, compositor, tile cache, and viewport buffer. Two call sites build one:

- `emWindow::create` (`crates/emcore/src/emWindow.rs:~165–216`) — constructs the `MaterializedSurface` inline while creating a regular window.
- `emGUIFramework::materialize_popup_surface` (`crates/emcore/src/emGUIFramework.rs:~258–300`) — constructs the same struct when a pending popup materializes.

The two blocks are character-for-character identical from `let surface = gpu.instance.create_surface(...)` through `let viewport_buffer = ...`. Closeout §3.5 item 1 flagged this as optional dedup.

## 2. Goal

Replace the two duplicated blocks with a single `pub(crate) fn build(gpu: &GpuContext, winit_window: Arc<winit::window::Window>) -> MaterializedSurface` associated function on `MaterializedSurface`. No observable behavior change. No new tests.

## 3. Classification (CLAUDE.md)

- **Port Ideology layer:** `MaterializedSurface` has no C++ analogue (wgpu/winit is a forced divergence from emCore's X11/GL path). Construction is idiom-adaptation territory.
- **Authority order:** no C++ to defer to for this specific sequence. Rust idiom governs.
- **No new `DIVERGED:` marker.** The existing `MaterializedSurface` divergence annotation (implicit in the forced-divergence popup architecture) covers this.
- **Do NOT list:** no `#[allow]`, no `Arc<Mutex<…>>`, no `Cow`, no glob imports introduced.

## 4. Signature

```rust
impl MaterializedSurface {
    pub(crate) fn build(
        gpu: &GpuContext,
        winit_window: Arc<winit::window::Window>,
    ) -> Self {
        let size = winit_window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);

        let surface = gpu
            .instance
            .create_surface(winit_window.clone())
            .expect("failed to create surface");

        let caps = surface.get_capabilities(&gpu.adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &surface_config);

        let compositor = WgpuCompositor::new(&gpu.device, format, w, h);
        let tile_cache = TileCache::new(w, h, 256);
        let viewport_buffer = crate::emImage::emImage::new(w, h, 4);

        Self {
            winit_window,
            surface,
            surface_config,
            compositor,
            tile_cache,
            viewport_buffer,
        }
    }
}
```

## 5. Call-site changes

### 5.1 `emWindow::create`

Replace the block starting at the `let size = winit_window.inner_size();` line through the explicit `MaterializedSurface { ... }` literal with:

```rust
let materialized = MaterializedSurface::build(gpu, winit_window);
let (w, h) = (
    materialized.surface_config.width,
    materialized.surface_config.height,
);
let core_config = Rc::new(RefCell::new(emCoreConfig::default()));
let view = emView::new(root_panel, w as f64, h as f64, core_config);
// … vif_chain unchanged …
let window = Rc::new(RefCell::new(Self {
    os_surface: OsSurface::Materialized(Box::new(materialized)),
    view: Rc::new(RefCell::new(view)),
    // … rest unchanged …
}));
```

`w` / `h` read back from `surface_config` so `emView::new` still sees post-clamp dimensions without re-reading `inner_size()`.

### 5.2 `emGUIFramework::materialize_popup_surface`

Replace the block from `let size = winit_window.inner_size();` through the literal `MaterializedSurface { … }` inside the `w_mut.os_surface = OsSurface::Materialized(...)` assignment with:

```rust
let gpu = self.gpu.as_ref().expect("GPU not initialized");
let materialized = MaterializedSurface::build(gpu, winit_window.clone());
let w = materialized.surface_config.width;
let h = materialized.surface_config.height;

{
    let mut w_mut = win_rc.borrow_mut();
    w_mut.os_surface = OsSurface::Materialized(Box::new(materialized));
    w_mut
        .view_mut()
        .SetGeometry(&mut self.tree, 0.0, 0.0, w as f64, h as f64, 1.0);
}
```

`winit_window.clone()` is preserved because the `window_id = winit_window.id()` line + `self.windows.insert(window_id, win_rc.clone())` + `winit_window.request_redraw()` all still need `winit_window` after the move into `build`.

## 6. Non-goals

- No behavioural change anywhere.
- No C++-alignment work (out of scope — both paths are already forced divergences).
- No public API surface change (`pub(crate)` only).
- No new tests; the existing 2443/2443 nextest suite + 237/6 golden baseline verify equivalence.

## 7. Verification

- `cargo check` — must pass.
- `cargo clippy -- -D warnings` — must pass.
- `cargo-nextest ntr` — must remain 2443/2443 (no changes to test count expected).
- Smoke: `timeout 20 cargo run --release --bin eaglemode` — exits 143 or 124.
- Golden: 237/243 (same 6 pre-existing failures).

## 8. Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Subtle ordering change between `surface.configure` and dependent objects | Very low | Helper preserves exact sequence; ops stay identical |
| `w`/`h` clamping semantics drift | Low | Read back from `surface_config.width`/`height` (already clamped inside helper) — identical to prior local `w.max(1)` / `h.max(1)` values |
| Missed call site | Low | Grep for `create_surface(`+`WgpuCompositor::new(`+`TileCache::new(` confirms only two call sites exist |

## 9. Closeout impact

On merge, mark §8.1 item 13 and §3.5 item 1 closed in
`docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md`. Update §8.0
SP6 row (currently "Optional; may skip entirely") and §6 marker table if
counts shift (no new `DIVERGED:` expected).
