# SP6 — Surface-Creation De-duplication — Implementation Plan

**Spec:** `docs/superpowers/specs/2026-04-19-emview-sp6-surface-creation-dedup-design.md`
**Date:** 2026-04-19
**Estimated scope:** one commit, ~60 lines net delta (add helper, delete two duplicated blocks).

## Anti-patterns this plan hardens against

- **Drift from C++:** N/A — no C++ analogue on this path (wgpu/winit forced divergence). No `DIVERGED:` marker needed.
- **Silent behaviour change:** verified by keeping field initialization order, clamping rules, and `create_surface → configure → compositor → tile_cache → viewport_buffer` sequence identical.
- **Missing call site:** Phase 0 Task 0.1 grep-audit confirms exactly two call sites before touching anything.
- **Test-count regression:** no new tests; 2443/2443 nextest baseline must hold.
- **Scope creep:** no call-site rewrites beyond the two duplicated blocks; no API surface changes; no `pub` promotion.

## Phase gates

Each phase closes with a gate. If a gate fails, **fix the cause**; do not skip. No `--no-verify`.

---

## Phase 0 — Audit (read-only)

**Task 0.1.** Grep-confirm there are exactly two call sites constructing a `MaterializedSurface` literal:

```bash
rg -n 'MaterializedSurface\s*\{' crates/
```

Expected hits: `crates/emcore/src/emWindow.rs` (inside `create`) and `crates/emcore/src/emGUIFramework.rs` (inside `materialize_popup_surface`). Zero others.

**Task 0.2.** Grep-confirm the surface-init sequence is nowhere else:

```bash
rg -n 'instance\.create_surface|WgpuCompositor::new\(' crates/
```

Expected: same two files only.

**Gate 0:** Outputs match expectations. Proceed.

---

## Phase 1 — Add helper

**Task 1.1.** In `crates/emcore/src/emWindow.rs`, add an `impl MaterializedSurface` block immediately below the `pub(crate) struct MaterializedSurface { … }` definition, containing the `build` associated function from spec §4 verbatim.

- Ownership: takes `winit_window: Arc<winit::window::Window>` by value; stores it in the returned struct. One internal `winit_window.clone()` for `create_surface`.
- Visibility: `pub(crate)`.
- Reads `inner_size()` once, clamps to `max(1)`, writes clamped values into `surface_config`.

**Gate 1:** `cargo check` passes; `cargo clippy -- -D warnings` passes. (The helper is not yet called; dead-code warning is not expected because `MaterializedSurface` itself is already `pub(crate)`-used, but if clippy flags `build` as unused, that's a signal to do Phase 2 immediately — do not `#[allow(dead_code)]`.) In practice, Phase 2 follows in the same commit, so run Gate 1 as a quick compile-only check and move on.

---

## Phase 2 — Migrate `emWindow::create`

**Task 2.1.** In `emWindow::create` (`crates/emcore/src/emWindow.rs:~155–216`), replace the block from:

```rust
let size = winit_window.inner_size();
let w = size.width.max(1);
// …
let viewport_buffer = crate::emImage::emImage::new(w, h, 4);
```

…through the explicit `MaterializedSurface { winit_window, surface, surface_config, compositor, tile_cache, viewport_buffer }` literal inside the `OsSurface::Materialized(Box::new(...))` expression, with:

```rust
let materialized = MaterializedSurface::build(gpu, winit_window);
let w = materialized.surface_config.width;
let h = materialized.surface_config.height;
```

Then the `Self { os_surface: OsSurface::Materialized(Box::new(materialized)), … }` literal.

Note: `emView::new(root_panel, w as f64, h as f64, core_config)` must still see post-clamp `w`, `h` — reading back from `materialized.surface_config.width/height` preserves identical values.

**Task 2.2.** Re-verify no stray references to the pre-move `winit_window` binding remain after the helper consumes it. (The `Arc` was locally created and passed in; the previous code kept `winit_window` alive via the struct literal's field capture. The helper likewise moves it into the returned struct. No later code in `create` references `winit_window` after this point.) Confirm by reading lines following the replaced block.

**Gate 2:** `cargo check` passes; `cargo clippy -- -D warnings` passes.

---

## Phase 3 — Migrate `emGUIFramework::materialize_popup_surface`

**Task 3.1.** In `materialize_popup_surface` (`crates/emcore/src/emGUIFramework.rs:~258–309`), replace the block from:

```rust
let size = winit_window.inner_size();
let w = size.width.max(1);
// …
let viewport_buffer = crate::emImage::emImage::new(w, h, 4);
```

…through the literal `MaterializedSurface { … }` passed into `OsSurface::Materialized(Box::new(...))`, with:

```rust
let gpu = self.gpu.as_ref().expect("GPU not initialized");
let materialized = crate::emWindow::MaterializedSurface::build(gpu, winit_window.clone());
let w = materialized.surface_config.width;
let h = materialized.surface_config.height;
```

Keep the surrounding borrow-block assigning `w_mut.os_surface = OsSurface::Materialized(Box::new(materialized))` and the `SetGeometry` call unchanged (just the contents of `Box::new(...)` now being the moved-in `materialized`).

**Task 3.2.** Confirm the three uses of `winit_window` *after* the replaced block still compile:
- `let window_id = winit_window.id();`
- `self.windows.insert(window_id, win_rc.clone());`
- `winit_window.request_redraw();`

These all reference `winit_window` (the outer `Arc`), unaffected by the helper consuming `winit_window.clone()`.

**Gate 3:** `cargo check` passes; `cargo clippy -- -D warnings` passes.

---

## Phase 4 — Full verification

**Task 4.1.** Run `cargo-nextest ntr`. Must remain **2443 / 2443** passed, 9 skipped.

**Task 4.2.** Run `cargo test --test golden -- --test-threads=1` (or `scripts/verify_golden.sh --report`). Must remain **237 passed / 6 failed** (same pre-existing failures: `composition_tktest_{1x,2x}`, `notice_window_resize`, `testpanel_{expanded,root}`, `widget_file_selection_box`).

**Task 4.3.** Smoke: `timeout 20 cargo run --release --bin eaglemode`. Expected exit 143 or 124.

**Gate 4:** All three pass.

---

## Phase 5 — Commit

**Task 5.1.** `git add` the two modified source files + the spec + the plan.

**Task 5.2.** Commit with body:

```
sp6: dedup MaterializedSurface construction

Extract the wgpu/winit surface-init sequence shared between
emWindow::create and emGUIFramework::materialize_popup_surface into
MaterializedSurface::build. No observable behaviour change; no new
tests; tests 2443/2443, golden 237/6 unchanged.

Closes §3.5 item 1 / §8.1 item 13 of the emView subsystem closeout.
```

**Gate 5:** pre-commit hook (fmt + clippy + nextest) passes. If it fails, fix the cause and redo via a new commit (no `--amend`, no `--no-verify`).

---

## Phase 6 — Update closeout doc

**Task 6.1.** Edit `docs/superpowers/notes/2026-04-18-emview-subsystem-closeout.md`:
- §3.5 item 1 — strike through, append `**Closed by SP6 on 2026-04-19** (<commit-sha>)`.
- §8.0 SP6 row — change State column from `Optional; may skip entirely` to `**Complete 2026-04-19**` with commit SHA.
- §8.0 suggested execution order — strike through `SP6`.
- §8.1 item 13 — strike through, append closed-by note.
- §6 — no change expected (no new `DIVERGED:` markers).
- §1 — `Known Rust-port incompletenesses remaining` line: remove `W3 surface de-dup (SP6, optional),`.

**Task 6.2.** Commit the doc update separately:

```
sp6: closeout — SP6 complete
```

---

## Rollback plan

If any gate fails and the cause is non-obvious, revert the WIP with `git checkout -- crates/` and restart from Phase 0. The helper is additive and isolated; there's no partial migration state worth preserving.
