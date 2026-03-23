# RenderThreadPool Investigation

## Gate 1: C++ Implementation

### Q1: CallParallel signature
```cpp
void CallParallel(Func func, void * data, int count);
```
- emRenderThreadPool.cpp:32-57
- Calling thread participates: it locks the Mutex, increments `CurrentStarted`, unlocks, runs `func(data, i)`, re-locks, repeats. It does NOT wait idle.
- Child threads do the same (emRenderThreadPool.cpp:174-179).

### Q2: Thread wake/terminate
- Child threads wait on `ActivateEvent.Receive()` (line 172).
- `TerminateChildThreads` flag checked under Mutex (line 170).
- Primitives: `emThreadMiniMutex` (spin mutex), `emThreadEvent` (semaphore-like).

### Q3: UserSpaceMutex
- emViewRenderer.h:105 — field `emThreadMiniMutex UserSpaceMutex`.
- emViewRenderer.cpp:132 — locked before tree walk in `ThreadRun`.
- emViewRenderer.cpp:142 — unlocked after `PaintView` returns, before `AsyncFlushBuffer`.
- The Painter's `SetUserSpaceMutex` (emPainter.cpp:311-318) stores the mutex pointer.
- Inside every paint method, `UserSpaceLeaveGuard` (emPainter.h:964-980) RAII-unlocks the mutex before pixel work and re-locks on scope exit.
- Without it: threads would serialize entirely, no parallelism in pixel rendering.

### Q4: SetUserSpaceMutex call sites
- emViewRenderer.cpp:108 — single-threaded path: `SetUserSpaceMutex(NULL, NULL)` (disabled).
- emViewRenderer.cpp:139 — multi-threaded path: `SetUserSpaceMutex(&UserSpaceMutex, &usmLockedByThisThread)`.
- emPainter.h:828-829 — stored as raw pointers `UserSpaceMutex*` + `bool* USMLockedByThisThread`.
- Used in `LeaveUserSpace()` (line 942), `EnterUserSpace()` (line 952), `UserSpaceLeaveGuard` (line 964).

### Q5: Tile distribution
- Shared counter `CurrentStarted` under Mutex (emRenderThreadPool.cpp:44-46).
- Each thread/caller increments counter, takes that index.
- NOT deterministic per-thread, but output is deterministic (same pixels regardless of which thread paints which tile).

### Q6: GetBufferPainter
- Virtual method (emViewRenderer.h:72-74). Each thread gets its own buffer indexed by `bufIndex`.
- Buffer lifetime: allocated by `PrepareBuffers`, reused across frames.

### Q7: AsyncFlushBuffer
- Virtual method, must be thread-safe (emViewRenderer.h:82-85).
- Called after PaintView, with UserSpaceMutex unlocked.
- Uploads painted pixels to screen (GPU/window system).

### Q8: UpdateThreadCount
- emRenderThreadPool.cpp:100-116.
- `n = min(HardwareThreadCount, MaxRenderThreads) - 1` (subtract 1 because caller is also a thread).
- If n < 0, clamps to 0 (single-threaded).
- Destroys and recreates threads when count changes.

### Q9: Error containment
- None. If Paint() throws or crashes, the thread dies silently.

## Gate 2: Rust Rendering Path

### Q1: ViewRenderer equivalent
- No `ViewRenderer` exists. `ZuiWindow::render()` (zui_window.rs:166-248) directly iterates tiles, creates `Painter`, calls `View::paint()`.

### Q2: Paint buffers
- `viewport_buffer: Image` — pre-allocated at window size, reused per frame (zui_window.rs:40).
- Per-tile: `TileCache` stores `Tile { image: Image (256×256), dirty, last_used }` (tile_cache.rs:8).

### Q3: Painter creation
- `Painter::new(target: &'a mut Image)` — borrows image mutably, creates value-type `PainterState`.
- State: `offset_x/y, scale_x/y, ClipRect, canvas_color, alpha` — all Copy types.
- No Rc/RefCell/Cell in Painter or PainterState.

### Q4: Thread-safety barriers (Rc, RefCell, etc.)
- **Painter/rendering path**: ZERO. No Rc, RefCell, Cell, raw pointers, NonNull, UnsafeCell.
- **Widget behaviors**: `Rc<Look>` in ~20 widgets. `Rc<RefCell<ConfigModel<CoreConfig>>>`, `Rc<Cell<u64>>`, `Rc<RefCell<RadioGroup>>` in config/radio widgets.

### Q5: Unsafe in painter/renderer
- ZERO. Entire rendering path is 100% safe Rust.

### Q6: Panel tree traversal during paint
- `View::paint_panel_recursive()` (view.rs:2007) walks tree with `&mut PanelTree`.
- For each panel: `tree.take_behavior(id)` → `behavior.paint(painter, ...)` → `tree.put_behavior(id, behavior)`.
- The `&mut PanelTree` is required for take_behavior/put_behavior.

### Q7: GPU output
- `WgpuCompositor` (compositor.rs). Per-tile GPU textures, `queue.write_texture()` for upload, composites as textured quads via WGSL shader.

### Q8: Shared caches
- All `OnceLock` (immutable after init, Sync):
  - Font atlas: `static ATLAS: OnceLock<Image>` (em_font.rs:28-32)
  - Gradient sqrt table: `static TABLE: OnceLock<Box<[u8; N]>>` (painter.rs:30-38)
  - Bicubic/adaptive/lanczos tables: `static` OnceLock (interpolation.rs:644,670,983)

## Gate 3: Thread Safety Requirements

### Q1: !Send types in rendering path
- `PanelTree` — contains `Box<dyn PanelBehavior>` which is !Send because behaviors contain `Rc<Look>`.
- `Rc<Look>` is in nearly every widget behavior.
- `Rc<RefCell<ConfigModel<CoreConfig>>>` in CoreConfigPanel family.
- `Rc<RefCell<RadioGroup>>` in RadioBox/RadioButton.
- All other rendering types (Image, Painter, View, TileCache) are Send+Sync.

### Q2: Shared state during parallel painting
- **PanelTree structure** (positions, visibility, clip rects): read-only during paint. Accessed by all threads.
- **Panel behaviors**: mutable via `paint(&mut self)`. Same behavior is painted by multiple threads (once per intersecting tile). 5-6 behaviors cache `last_w/last_h` (benign same-value writes). `SubViewPanel` does real mutation.
- **OnceLock caches**: read-only, Sync. No synchronization needed.
- **Image buffers**: each thread writes to its own tile buffer. No sharing.

### Q3: Rust equivalent of UserSpaceMutex
- **SpinMutex** (matching C++ `emThreadMiniMutex`) with RAII leave/enter guards.
- The Painter gets an optional `UserSpaceContext` containing a pointer to the shared SpinMutex and a per-thread locked flag.
- Each heavy paint method creates a `UserSpaceLeaveGuard` — unlocks before pixel work, re-locks on drop.
- When `UserSpaceContext` is None (single-threaded path), guard is a no-op.

### Q4: Tile independence verification
- **Read during tile paint**: PanelTree structure (viewed_x/y, width/height, clip, canvas_color, children list), View state (viewport dimensions, active panel, highlight), OnceLock caches.
- **Write during tile paint**: tile's Image buffer (exclusive per-thread), behavior's `last_w/last_h/visible_height` (same value from all threads).
- **Overlap**: Behavior `last_w/last_h` writes overlap across tiles but are benign (same value). Behavior `Rc` derefs overlap but don't modify refcount.
- **Synchronization needed**: SpinMutex serializes ALL tree + behavior access. Pixel work (scanline fills, text rendering) runs with mutex released.

### Q5: Thread panic strategy
- `std::thread::scope` propagates panics to the calling thread after all threads complete.
- A panicking thread's tile is simply not rendered. Other threads continue.
- After scope returns, the panic is re-raised on the main thread.

### Bail-out assessment
- 4 !Send types in the rendering path: `Rc<Look>`, `Rc<RefCell<ConfigModel>>`, `Rc<Cell<u64>>`, `Rc<RefCell<RadioGroup>>`.
- Converting to Arc would change public API (constructors, factory methods).
- However, **we don't need Send/Sync** — the C++ approach uses a single mutex to serialize all behavior access, not concurrent behavior access. We can use `unsafe impl Sync` on a wrapper with the SpinMutex guarantee, avoiding any Rc→Arc conversion.
- Proceed with unsafe + SpinMutex approach.

## Gate 4: Design Decision

### Q1: Threading model
**`std::thread::scope` + SpinMutex** (hybrid of C++ patterns).
- `std::thread::scope` for thread lifetime management (threads don't outlive the render call).
- `SpinMutex` for UserSpaceMutex pattern (serialize tree/behavior access, release for pixel work).
- Not rayon: determinism required, rayon's work-stealing changes tile assignment.
- Not a persistent thread pool initially — `std::thread::scope` has ~50-100µs thread creation overhead per frame, acceptable for 60fps. Can optimize to persistent pool later if benchmarks show need.
- Atomic counter for tile dispatch (same as C++).

### Q2: Handling !Send types
- **Unsafe Send/Sync wrapper** around `&mut PanelTree`:
  ```
  SAFETY: During parallel rendering, PanelTree is accessed exclusively
  through a SpinMutex. Only one thread at a time touches the tree or any
  Rc values in behaviors. Rc values are only dereferenced (not cloned or
  dropped) during paint — no reference count changes occur. This matches
  the C++ UserSpaceMutex pattern exactly.
  ```
- The wrapper is only used within `std::thread::scope` — the scope guarantees the tree outlives all threads.

### Q3: Thread pool location
- `RenderThreadPool` struct stored in `ZuiWindow`.
- Configured from `CoreConfig::max_render_threads`.
- Contains: thread_count (computed from config + hardware_concurrency).
- Provides: `call_parallel(f, count)` method using `std::thread::scope`.

### Q4: Testing
- (a) Byte-identical comparison: render with max_render_threads=1 and max_render_threads=4, assert identical pixel output.
- (b) Thread count edge cases: 0 (clamped to 1), 1 (single-threaded), 2, hardware_concurrency.
- (c) Golden tests: run with MAX_RENDER_THREADS=4, compare divergence against baseline.

### Q5: Buffer flush / GPU upload
- GPU upload (`compositor.upload_tile`) stays on the main thread, after all tiles are painted.
- Parallel phase: paint tiles into thread-local buffers, copy results to shared results array.
- Sequential phase: upload results to GPU, composite, present.
- This differs from C++ (which flushes per-tile inside threads) but is simpler and avoids GPU thread-safety issues.

### Q6: Thread panic handling
- `std::thread::scope` automatically propagates panics.
- If a thread panics, its tile isn't rendered (visual artifact for that frame).
- After scope, panic propagates to main thread — logs error, frame is partially rendered.
- No explicit containment needed beyond what scope provides.

## Gate 5: Implementation Summary

### Architecture: Display List
The implementation uses a display list approach instead of the C++ UserSpaceMutex pattern:
1. **Phase 1 (single-threaded)**: Walk panel tree, call `behavior.paint()` with a recording `Painter`. Draw operations are captured as `DrawOp` values into a `Vec<DrawOp>`.
2. **Phase 2 (parallel via `std::thread::scope`)**: Each thread picks dirty tiles from an atomic counter. For each tile, replay the `DrawList` into a tile-sized `Image` buffer with tile-specific coordinate offset.
3. **Phase 3 (single-threaded)**: Upload rendered tiles to GPU and composite.

### Why not the C++ UserSpaceMutex pattern?
- Rust's aliasing rules prevent holding `&mut PanelTree` while releasing a mutex inside painter methods. Two `&mut` references would exist simultaneously (one per thread), which is UB under Rust's memory model.
- `PanelBehavior` types contain `Rc<Look>` making them `!Send/!Sync`. Converting to `Arc` would violate CLAUDE.md's "no Arc for UI tree" rule.
- The display list approach avoids both issues: recording is single-threaded (no Send/Sync needed), replay is parallel on independent buffers (no shared mutable state).

### Unsafe justification
`DrawOp` contains `*const Image` raw pointers for image data owned by panel behaviors. `unsafe impl Send/Sync for DrawOp` is sound because:
1. Images are owned by behaviors in `PanelTree`, which is not modified between recording and replay
2. `std::thread::scope` ensures all replay threads complete before the function returns
3. After the function returns, the caller can freely modify the tree

### Files changed
- `src/render/draw_list.rs` — NEW: `DrawOp` enum, `DrawList` struct with `replay()`
- `src/render/thread_pool.rs` — NEW: `RenderThreadPool` with `call_parallel()`
- `src/render/painter.rs` — `PaintTarget` enum, recording mode support, `new_recording()`, `read_pixel()` helper
- `src/render/mod.rs` — module declarations for `draw_list` and `thread_pool`
- `src/window/zui_window.rs` — `render_parallel()`, `set_max_render_threads()`, env var support

### Test results
- `draw_list_replay_matches_direct`: byte-identical output for thread counts 1, 2, 4
- All 1097 unit + golden tests pass with `MAX_RENDER_THREADS=1`
- All 191 golden tests pass with `MAX_RENDER_THREADS=4`
- Zero clippy warnings

### Miri / ThreadSanitizer
Not run — nightly toolchain not installed. Noted as a gap.

## Gate 6: Verification and Benchmark

### Test results with max_render_threads=1
All 1105 tests pass (199 golden + 8 parallel + 612 unit + ... others).

### Test results with max_render_threads=4
All 1105 tests pass. The 8 parallel tests verify byte-identical output
between 1-thread and 4-thread tiled rendering using the display list pipeline.

### Results are identical
Single-threaded tiled rendering and multi-threaded tiled rendering produce
byte-identical output for all tested scenes (border, checkbox, label,
scalar field) with tile sizes 32, 64, and 128.

Note: Tiled rendering has inherent sub-pixel AA artifacts at tile boundaries
compared to viewport (non-tiled) rendering. These artifacts are identical
between single- and multi-threaded tiled paths. They are preexisting in the
per-tile rendering path used by ZuiWindow.

### Divergence comparison against baseline
All golden tests show unchanged divergence, except `widget_colorfield` which
IMPROVED (2664→2428 failing pixels, max 185→71). This improvement is from
earlier commits (ListBox/TextField stub closures), not from thread pool changes.

### Benchmark (100 iterations, 800x600, Border with caption)
- Single-threaded: 4391.6ms total (43.92ms/frame)
- Multi-threaded (4 threads, 128px tiles): 2164.1ms total (21.64ms/frame)
- Speedup: 2.03x

The 2x speedup with 4 threads reflects the overhead of:
1. Display list recording (single-threaded tree walk + DrawOp capture)
2. Thread pool dispatch (std::thread::scope spawning overhead)
3. Tile compositing (copying tile buffers back to framebuffer)

The recording phase is overhead not present in direct rendering, limiting
the theoretical speedup. More complex scenes with heavier pixel work
(text, gradients, images) should see better speedup ratios.
