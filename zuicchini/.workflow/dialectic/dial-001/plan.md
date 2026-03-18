# Dialectic Input: Zuicchini Zoom/Pan Pipeline — Highest Performance Architecture

## Scoring Axes

Use the 4 default axes (Defensibility, Specificity, Robustness, Compatibility) plus the following:

### Performance

**Definition**: Every frame completes in under 16.6ms (60fps) regardless of tree depth, zoom level, or pan velocity, while producing pixel-identical output to a full repaint.

**Metric**: p99 frame time in ms, measured across a sweep of zoom levels and pan velocities with varying tree depths (10, 100, 1000 panels).

**Benchmark scenario**: 1000-panel tree, 1080p viewport, single-threaded, continuous pan at 600px/s with interleaved 2x zoom pulses. Correctness gate: pixel-identical to full repaint.

**Scoring rubric**:
- 1.00: All 5 properties hold simultaneously — zero redundant pixel work, cost proportional to visible change not tree size, O(1) viewport transform, p99 under 16.6ms, bounded memory with graceful degradation on cache miss.
- 0.75: 4 of 5 properties hold; the missing one has a known, bounded cost (e.g., occasional redundant repaint of a small region, or memory grows slowly but has a cap).
- 0.50: Core dirty-region tracking works and most frames hit budget, but at least one property is structurally weak (e.g., layout recalc is O(tree) not O(visible), or cache eviction causes occasional stalls).
- 0.25: Some caching exists but frame time scales with tree size or zoom history; redundant pixel work is common; p99 blows budget on nontrivial trees.
- 0.00: Full repaint every frame, or architecture structurally prevents incremental update.

**Performance sub-properties** (used to justify scores):

1. **Zero redundant pixel work** — a pixel that didn't change between frames is never recomputed. If the user pans 5px right, only the newly-exposed column of pixels is rasterized. If zoom changes, only panels whose screen-space appearance actually changed are repainted.
2. **Work proportional to visible change, not tree size** — frame cost scales with the number of pixels that differ, not the number of panels in the tree. A 1000-panel tree where only 3 panels are on screen should cost the same as a 3-panel tree.
3. **Constant-time viewport transform** — translating or scaling the already-rendered content to its new position is O(1), not O(pixels). The expensive rasterization only runs for regions where the transform alone can't produce correct output (newly visible areas, panels that crossed a detail threshold, panels whose content changed).
4. **No perceptible latency spike** — p99 frame time stays under budget, not just median. No frames where layout recalculation, auto-expansion, or cache eviction causes a stall.
5. **Memory bounded** — the tile cache or retained surface doesn't grow unboundedly with zoom history. There's a fixed memory ceiling with graceful degradation (repaint on cache miss, not crash or stall).

## Approaches

### 1. Full-Dirty DrawList Replay (Current Architecture)

The existing system. On any viewport change (scroll, zoom, visit), `viewport_changed` is set, which causes `mark_all_dirty()` on the tile cache. Every tile is repainted every frame during active pan/zoom. The three-strategy render system mitigates cost: if >50% of tiles are dirty, the tree is walked once into a full viewport buffer and chunks are copied to tiles; if fewer are dirty and threads are available, a DrawList is recorded from one tree walk and replayed in parallel into each dirty tile; for single dirty tiles, the tree is painted directly into the tile buffer.

The core invariant is simplicity: the rendered output is always a fresh, complete rasterization of the current view state. There is no stale-cache correctness risk. Dirty rect tracking exists and works for content-only changes (e.g., cursor blink, button press), but viewport transforms bypass it entirely.

**Pan frame (5px right):** `raw_scroll_and_zoom` updates `rel_x`, sets `viewport_changed`. `about_to_wait` sees the flag, calls `mark_all_dirty()` on tile cache. `render()` counts all tiles as dirty, enters full-viewport strategy, walks entire tree, paints into viewport buffer, copies to all tiles, uploads all to GPU.

**Zoom frame (2x):** Identical flow. `zoom()` updates `rel_a`, sets `viewport_changed`. Full repaint of every tile.

**Failure mode:** Frame time is O(visible_pixels × primitives_per_panel), constant regardless of how small the viewport change was. A 1px pan costs the same as a 1000px pan. The DrawList parallel replay helps throughput but doesn't reduce total work — every pixel is still recomputed.

---

### 2. Tile Grid Shift on Pan

Keep the 256×256 tile grid, but on pan, remap tile positions instead of repainting everything. Each tile's content is valid at a specific viewport offset. When the viewport shifts by (dx, dy), tiles whose content is still on-screen get reassigned to their new grid position. Only tiles covering newly-exposed edges are marked dirty and rasterized.

The core invariant: a tile's rasterized content depends only on which viewport-coordinate rectangle it covers, and a pan changes which rectangle each grid slot maps to without changing the content of rectangles that remain on-screen.

Data structures: extend `Tile` with `origin_x, origin_y: i32` (the viewport pixel coordinate of the tile's top-left corner when it was last rendered). On pan, compute which existing tiles map to the new grid layout. Copy surviving tile references to their new slots; mark vacated edge slots dirty. The tile cache becomes a logical ring buffer in each axis.

**Pan frame (5px right):** `update_viewing` shifts all viewed coordinates. Before marking tiles, compute `shift = (5, 0)` in pixels. Since 5 < 256, all tile columns survive except the leftmost column is now partially off-screen and a new rightmost column is exposed. Reassign tile slots: `tile[col] = old_tile[col]` for surviving columns. Mark the new rightmost column dirty. Record the DrawList once, replay only into the ~4 dirty edge tiles (at 1080p, 5 tiles tall). Upload only those tiles. Total rasterization: ~5 tiles × 256×256 instead of ~40 tiles.

**Zoom frame (2x):** Tile content is scale-dependent, so no tiles can be reused at a different zoom level. Fall back to full repaint (same as Approach 1). The zoom case gets no benefit.

**Failure mode:** Zoom-heavy workloads see no improvement over the baseline. Diagonal pans dirty two edges (a column and a row) instead of one. Very fast pans (>256px/frame) shift by more than one tile width, reducing reuse. At extreme pan velocity, all tiles are shifted out and the approach degrades to full repaint. Also, this only works for pure translation — any rotation or non-uniform scale would invalidate all tiles.

---

### 3. Oversize Viewport Buffer

Render into a buffer larger than the viewport — for example, 1.5× in each dimension. The visible viewport is a movable window within this oversized buffer. Pan moves the window origin without re-rendering. Only when the viewport drifts beyond the buffer margin is new content rasterized, and only for the newly-exposed margin strip.

The core invariant: the oversized buffer always contains a superset of the pixels currently on screen, so pan is a pure copy-origin change until the margin is exhausted.

Data structures: `OversizeBuffer { image: Image, origin_x: f64, origin_y: f64, margin: f64, zoom_level: f64 }`. The buffer is allocated at `(vw * 1.5) × (vh * 1.5)` pixels. The viewport's top-left maps to `(origin_x, origin_y)` within the buffer. When the viewport moves, only the origin shifts. When the origin would move past the margin, the buffer content is shifted (memmove) and the exposed strip is rasterized.

**Pan frame (5px right):** Shift `origin_x += 5`. Check if `origin_x + vw > buffer_width - margin`. If not, the frame is just a changed copy-origin — no rasterization at all. The compositor reads from `(origin_x, origin_y)` in the oversized buffer. Cost: O(1) for the pan, O(viewport_pixels) for the GPU blit (unavoidable).

**Zoom frame (2x):** The zoom level changed, so the buffer's content is invalid at the new scale. The entire oversized buffer must be re-rendered. A variant could keep the old buffer and scale it as a placeholder while re-rendering asynchronously, but pixel-identical output requires a synchronous full repaint.

**Failure mode:** Memory cost is 1.5²× = 2.25× the viewport buffer size (for 1080p: ~13MB instead of ~8MB for RGBA). Zoom gets no benefit. The buffer shift (memmove of ~13MB) is not free — at ~5GB/s memcpy bandwidth, it's ~2.6ms. Very fast diagonal pan exhausts the margin quickly, causing frequent shifts. The margin size is a tuning parameter: larger margin = more memory + less frequent rasterization, smaller margin = more frequent rasterization.

---

### 4. Per-Panel Retained Surfaces

Each visible panel gets its own offscreen `Image` rendered at its current viewed resolution. Frame assembly composites all panel surfaces onto the viewport using their current `viewed_x/y/width/height` transforms. Pan and zoom only change the compositing transforms, not the panel content. A panel's surface is re-rendered only when: (a) its content changes (`invalidate_painting`), (b) it crosses a detail/visibility threshold (`viewed_rect.w` crosses 25px), or (c) its viewed size changes enough that the cached resolution would visibly alias.

The core invariant: rasterization cost is proportional to the number of panels whose pixel content actually changed, not the number of panels visible or the viewport delta.

Data structures: `PanelSurface { image: Image, rendered_at_width: f64, rendered_at_height: f64, content_generation: u64 }` stored alongside each panel (or in a parallel `SlotMap` keyed by `PanelId`). A generation counter on the panel increments on `invalidate_painting`. During compositing, each panel's surface is blit onto the viewport buffer with scaling if `viewed_width != rendered_at_width`.

**Pan frame (5px right):** `update_viewing` recomputes all `viewed_x/y/width/height`. No panel's content changed, so no surfaces are re-rendered. The compositor clears the viewport buffer and blits each visible panel's cached surface at its new `(viewed_x, viewed_y)` with appropriate scaling. Cost: O(visible_panels × blit_cost). For a 1000-panel tree with 20 visible panels, this is 20 scaled blits instead of 20 full rasterizations.

**Zoom frame (2x):** `update_viewing` changes `viewed_width/height` for all panels. Panels whose viewed size changed significantly (e.g., >10% different from their cached resolution) have their surfaces re-rendered at the new resolution. Panels that became too small (below 25px threshold) are skipped. Panels that became newly visible are rendered for the first time. Panels whose size didn't change much can reuse their surface with a minor scale during compositing.

**Failure mode:** Memory usage is proportional to the number of visible panels × their viewed pixel area. A deeply-zoomed-in panel could have a surface as large as the viewport. Compositing 20 surfaces with alpha blending and scaling is not free — bilinear scaling of a 1920×1080 surface is ~8M pixel reads. Overlapping panels require back-to-front compositing with alpha, which means the compositor must respect paint order. Non-opaque panels cannot be composited independently. Panel surfaces that change every frame (animations, clocks) negate the caching benefit for those panels.

---

### 5. Multi-Resolution Tile Pyramid

Maintain tile caches at multiple discrete zoom levels, similar to a mip-map. Each level stores 256×256 tiles rendered at that zoom's resolution. When the user zooms, select the nearest cached level and composite its tiles with minor scaling, while asynchronously rendering tiles at the exact new zoom level. Pan at a given zoom level uses tile shifting (Approach 2).

The core invariant: there is always a "close enough" pre-rendered tile available for any zoom level, so zoom transitions never require synchronous full rasterization.

Data structures: `TilePyramid { levels: Vec<TileLevel> }` where `TileLevel { zoom_factor: f64, tiles: TileCache, last_used: u64 }`. Zoom levels are spaced geometrically (e.g., each level is 2× the previous). The number of levels is bounded (e.g., 8 levels covering 256× zoom range). Each level has its own tile grid and LRU tracking. Total memory budget is shared across levels.

**Pan frame (5px right):** Identical to Approach 2 within the current zoom level's tile cache. Shift tile positions, rasterize edge tiles only.

**Zoom frame (2x):** Find the closest cached zoom level. If an exact match exists, use it directly. If not, find the two bracketing levels, scale the closer one's tiles to the new viewport coordinates (bilinear scale of 256×256 tiles), and composite. Meanwhile, queue the exact zoom level for rasterization. On the next frame, the exact tiles are ready and replace the scaled approximation. The scaled intermediate is not pixel-identical to a full repaint — the correctness gate requires that the final output (after the async render completes) is exact.

**Failure mode:** The pixel-identical requirement means the scaled intermediate cannot be the final output — it's a visual approximation during the transition. If the user zooms continuously, every frame requires new tile rasterization at the current zoom level, and the pyramid provides no benefit because no level is reused before it's evicted. Memory usage grows with the number of active zoom levels. The geometric spacing means a 1.5× zoom hits neither level exactly, requiring scale on every frame until the exact level is rendered. Eviction across levels is complex — which level's tiles to evict when memory is tight?

---

### 6. Damage-Clipped DrawList Replay

Extend the existing DrawList system with per-operation bounding boxes. On a viewport change, compute the damage region (the set of pixels that differ between the old and new view). Replay only the DrawOps whose bounding boxes intersect the damage region, clipped to the damage rect. For pan, the damage region is the newly-exposed edge strip. For zoom, the damage region is the entire viewport (since all pixels change scale), but panels that were off-screen and are now on-screen have a bounded damage rect.

The core invariant: the DrawList is a complete, ordered record of every draw operation, and replaying a subset clipped to a damage region produces the correct pixels within that region.

Data structures: extend `DrawOp` with `bbox: PixelRect` computed during recording. The `DrawList` gains `ops_by_bbox: Vec<(usize, PixelRect)>` — an index sorted by spatial extent for fast intersection queries. A `DamageRegion` is a list of non-overlapping pixel rectangles. On replay, the painter's clip rect is set to the damage region before iterating ops.

**Pan frame (5px right):** The damage region is `Rect(viewport_width - 5, 0, 5, viewport_height)` — a 5px-wide strip on the right edge. The previous frame's viewport buffer is shifted 5px left (memmove). Only DrawOps whose bbox intersects the 5px strip are replayed, clipped to that strip. The tree is not re-walked — the DrawList from the previous frame is reused with transformed bboxes. Cost: O(ops_intersecting_strip × strip_pixels).

**Zoom frame (2x):** Every pixel changes, so the damage region is the full viewport. All DrawOps are replayed. No benefit over Approach 1. A refinement: if the zoom is small (e.g., 1.01×), the old viewport buffer can be scaled and the damage region becomes only the pixels where the scaled approximation differs from the exact rendering — but computing this difference is itself expensive.

**Failure mode:** For zoom, the damage region is the entire viewport, so this approach degrades to full DrawList replay. The per-op bounding box computation adds overhead during recording. Maintaining a spatially-indexed DrawList adds memory and complexity. The memmove of the viewport buffer on pan is O(viewport_pixels) — non-trivial at 1080p (~8MB). The DrawList from the previous frame references panel state (image pointers) that must remain valid, which constrains when panels can be modified. If many DrawOps span the full viewport (e.g., a background rect), they intersect every damage region and are always replayed.

---

### 7. Pipelined Main-Thread / Render-Thread Split

Decouple tree traversal from rasterization using a frame-ahead pipeline. The main thread walks the tree, computes viewed coordinates, detects damage, and produces a DrawList each frame. A dedicated render thread consumes the DrawList and rasterizes tiles. The two threads overlap: while the render thread rasterizes frame N, the main thread is already walking the tree for frame N+1. Communication is via double-buffered DrawLists — no shared mutable state on the panel tree.

The core invariant: the main thread never blocks on rasterization, and the render thread never accesses the panel tree. Latency is hidden because tree walk and rasterization overlap in time.

Data structures: `FramePacket { draw_list: DrawList, damage_rects: Vec<PixelRect>, viewport_transform: ViewTransform, dirty_tiles: Vec<(u32, u32)> }`. Two FramePackets are allocated; the main thread fills one while the render thread drains the other. A channel (or atomic swap) passes ownership between threads. The render thread owns the tile cache and compositor. The main thread owns the panel tree and view.

**Pan frame (5px right):** Main thread: `update_viewing`, walk tree, record DrawList into the current FramePacket, compute dirty tiles, send packet to render thread. Render thread (concurrently): receives the previous frame's packet, replays DrawList into dirty tiles, uploads to GPU, presents. The pan's rasterization cost doesn't add to the main thread's frame time — it's hidden behind the next frame's tree walk.

**Zoom frame (2x):** Same pipeline. The main thread's cost is the tree walk + DrawList recording (~1-2ms from benchmarks). The render thread's cost is the full rasterization (~18ms). The user sees the zoom result one frame later (one frame of latency), but the main thread maintains 60fps responsiveness because it never blocks on rasterization.

**Failure mode:** Adds one frame of visual latency (the display shows frame N-1 while the main thread computes frame N). If the render thread can't keep up (rasterization >16.6ms), frames are dropped or the pipeline stalls. The DrawList's image pointers must remain valid until the render thread finishes — this constrains when panel behaviors can deallocate images, requiring either image reference counting or a fence. Double-buffering the DrawList doubles memory for draw operations. This approach doesn't reduce total rasterization work — it hides latency but doesn't eliminate redundant pixel work. It pairs well with any of Approaches 2-6 as the execution model, but alone it only helps throughput, not per-pixel efficiency.
