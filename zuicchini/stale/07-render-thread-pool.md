# Implement emRenderThreadPool

This feature introduces multi-threaded rendering to zuicchini. It touches the rendering pipeline. Mistakes here cause data races, pixel corruption, or golden test regressions. You must investigate thoroughly before writing any code.

## Gate 1: Understand the C++ implementation

Read these files completely before proceeding:
- `~/.local/git/eaglemode-0.96.4/include/emCore/emRenderThreadPool.h`
- `~/.local/git/eaglemode-0.96.4/src/emCore/emRenderThreadPool.cpp`
- `~/.local/git/eaglemode-0.96.4/include/emCore/emViewRenderer.h`
- `~/.local/git/eaglemode-0.96.4/src/emCore/emViewRenderer.cpp`

After reading, answer these questions in a scratchpad (write to `state/run_003/render_pool_investigation.md`). Do not proceed to Gate 2 until every question has a concrete answer with file:line citations.

1. What is the exact signature of `CallParallel`? How does the calling thread participate — does it wait idle or does it also execute work items? Cite the specific lines.
2. How do child threads know when to wake up and when to terminate? What synchronization primitives are used?
3. In `emViewRenderer::ThreadRun`, what is `UserSpaceMutex`? When is it locked? When is it unlocked? Why does the painter need it? What happens if you remove it?
4. What is `SetUserSpaceMutex` on the painter? Find every call site in emCore where it's set. What does the painter do with it internally? Search `grep -rn SetUserSpaceMutex` across the entire emCore source.
5. How are tiles distributed to threads? Is it a shared counter, a pre-divided queue, or something else? Is the distribution deterministic across runs?
6. What is `GetBufferPainter`? Does each thread get its own buffer? What is the buffer lifetime?
7. What is `AsyncFlushBuffer`? What does it do with the painted pixels? Where do they go? How does it interact with the GPU/screen upload path?
8. How does `UpdateThreadCount` interact with `CoreConfig::MaxRenderThreads`? What happens when the config changes at runtime? What if `MaxRenderThreads` is 0 or negative?
9. What happens if a panel's `Paint()` method throws an exception or crashes during parallel rendering in C++? Is there any error containment?

## Gate 2: Understand the Rust rendering path

Search the zuicchini codebase to understand how rendering currently works single-threaded. Answer these questions in the same investigation file.

1. Does a `ViewRenderer` or equivalent exist in `src/`? If yes, read it. If no, how are panels currently painted — trace the call chain from the window's render event to the final pixel buffer.
2. Where are paint buffers allocated? Are they per-frame, per-tile, persistent?
3. How does the current painter get created? What state does it reference? Is any of that state behind `Rc<RefCell<>>` (which is not Send)?
4. Search for every use of `Rc`, `RefCell`, `Cell`, `*mut`, `*const`, `NonNull`, `UnsafeCell` in the painter and rendering path. List each one with file:line. All of these are thread-safety barriers.
5. Search for `unsafe` in the painter and renderer. List each one with file:line. Each needs `Send`/`Sync` analysis.
6. How does the panel tree traversal work during paint? Does `PaintView` or equivalent walk the tree and call each panel's `paint()`? Is any mutable state accessed during this walk?
7. Does zuicchini use wgpu for the final screen output? If yes, how do CPU-painted buffers get uploaded to GPU? This is the `AsyncFlushBuffer` equivalent — the multi-threaded path must not break it.
8. Are there any shared caches in the rendering path? Font glyph cache, image decode cache, texture atlas, color profile lookup? List each with file:line. These are hidden shared mutable state that concurrent painting would race on.

## Gate 3: Identify thread safety requirements

Based on Gates 1 and 2, answer:

1. Which Rust types in the rendering path are `!Send`? List each with its file:line.
2. Which shared state is accessed by multiple threads during parallel painting? For each, describe: what accesses it, whether access is read-only or read-write, and what the C++ does to synchronize it (UserSpaceMutex? nothing?).
3. What is the Rust equivalent of C++'s `UserSpaceMutex` pattern? Consider: `Mutex<()>` as a lock token, `RwLock` for read-heavy access, `parking_lot::Mutex` for performance, restructuring to avoid shared mutable state entirely, or `&mut` buffer slicing (each thread gets an exclusive mutable reference to its own tile region — zero-cost, enforced by the borrow checker, no runtime synchronization).
4. **Verify tile independence concretely.** Trace the data flow for a single tile paint operation from start to finish. List every memory location that is read and every memory location that is written. Confirm that no written location overlaps with any location read or written by a concurrent tile. If any overlap exists, describe what synchronization is needed.
5. What happens when a thread panics during `paint()`? In `std::thread::scope`, the panic propagates to the calling thread after join. In a manual pool, the thread dies silently. State which strategy the Rust implementation will use and why.

**Bail-out threshold:** If more than 5 types in the rendering hot path are `!Send` AND converting them requires changing their public API (not just adding `Arc` wrappers), this constitutes a major refactoring. Stop and report the list of types and the API changes required. If 5 or fewer types need adjustment, or the changes are internal (private fields only), proceed.

## Gate 4: Design decision

Before writing code, write a brief design in the investigation file covering:

1. `std::thread::scope` vs `rayon` vs manual thread pool — which and why. Consider: rayon changes work distribution nondeterministically. `std::thread::scope` matches the C++ model (fixed threads, barrier sync). A manual pool matches C++ most closely but is more code. Also consider: `&mut` buffer slicing with `std::thread::scope` may eliminate all synchronization overhead entirely — each thread gets an exclusive `&mut [u8]` slice of its tile buffer.
2. How will you handle `!Send` types? Options: make them `Send`, restructure to avoid sharing, use unsafe with justification, or redesign so threads don't need access.
3. Where will the thread pool live? As a model in Context (like C++)? As a field on the renderer?
4. How will you test this? At minimum: (a) byte-identical comparison of single-threaded vs multi-threaded output on the same scene, (b) tests with thread counts 0, 1, 2, and hardware_concurrency, (c) a scene complex enough to generate multiple tiles.
5. How will the buffer flush/GPU upload path (Gate 1 Q7, Gate 2 Q7) work with parallel rendering? Describe the synchronization or sequencing.
6. How will thread panics be handled? Describe the containment strategy.

## Gate 5: Implement

Only after Gates 1-4 are complete in the investigation file.

1. Implement `RenderThreadPool` — the generic parallel dispatch mechanism.
2. Handle edge cases: `max_render_threads <= 0` clamps to 1 (single-threaded). `max_render_threads > hardware_threads` clamps to hardware_threads.
3. Integrate it into the rendering path identified in Gate 2. The buffer flush/upload path must be updated per Gate 4 Q5.
4. Add a runtime toggle: `CoreConfig::max_render_threads == 1` means single-threaded (current behavior), `> 1` means parallel. The single-threaded path must remain as a fallback and be the default.
5. `cargo check --workspace`. Fix until clean.
6. Run `cargo test --workspace`. All tests must pass. Zero regressions.
7. Run golden tests **with multi-threading forced on**: `MAX_RENDER_THREADS=4 MEASURE_DIVERGENCE=1 cargo test --test golden 2>&1`. Compare divergence numbers against the previous baseline (from `state/run_003/golden_baseline.json`). If ANY golden test shows different divergence (higher OR lower), stop and investigate — multi-threaded rendering must produce bit-identical output to single-threaded.
8. Write a test that renders the same scene single-threaded and multi-threaded and asserts the output buffers are byte-identical. Run it with thread counts 1, 2, and 4.
9. If available on your toolchain, run `cargo test` under Miri (`cargo +nightly miri test --test golden -- <one_small_test>`) or ThreadSanitizer (`RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test --test golden -- <one_small_test>`) on at least one golden test. If neither is available, note this in the investigation file as a gap.

## Gate 6: Verify and benchmark

1. Run the full test suite with `max_render_threads = 1`. Record results.
2. Run the full test suite with `max_render_threads = 4`. Record results.
3. Results must be identical. If not, do not commit. Diagnose the difference.
4. Benchmark: render a complex scene (TestPanel expanded) 100 times single-threaded, then 100 times multi-threaded. Report wall-clock time for each. If multi-threaded is slower than single-threaded, the implementation has excessive synchronization overhead — diagnose before committing.
5. Commit: `feat(CAP-0057): implement RenderThreadPool for parallel tile rendering`

## Rules

- Do not skip gates. Each gate's output must exist in the investigation file before proceeding.
- Do not use `unsafe` without a written justification in the investigation file explaining why it's sound.
- Do not add `rayon` without first confirming it produces deterministic, bit-identical output for this use case.
- If Gate 3's bail-out threshold is exceeded (>5 `!Send` types requiring public API changes), stop and report.
- The single-threaded path must always work. Never break it.
