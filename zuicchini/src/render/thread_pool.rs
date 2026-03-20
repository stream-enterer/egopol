use std::sync::atomic::{AtomicUsize, Ordering};

use crate::dlog;

/// Thread pool for parallel tile rendering.
///
/// Port of C++ `emRenderThreadPool`. Manages a configurable number of
/// worker threads for rendering tiles concurrently. Uses `std::thread::scope`
/// for zero-cost thread lifetime management.
pub struct RenderThreadPool {
    thread_count: usize,
}

impl RenderThreadPool {
    /// Create a new pool with the given maximum thread count.
    ///
    /// The actual thread count is `min(max_threads, hardware_concurrency)`,
    /// clamped to `[1, hardware_concurrency]`.
    ///
    /// If the `MAX_RENDER_THREADS` environment variable is set, it overrides
    /// the `max_render_threads` parameter (for testing).
    pub fn new(max_render_threads: i32) -> Self {
        let config_max = match std::env::var("MAX_RENDER_THREADS") {
            Ok(val) => val.parse::<i32>().unwrap_or(max_render_threads),
            Err(_) => max_render_threads,
        };
        let count = Self::compute_count(config_max);
        dlog!("RenderThreadPool: thread_count={}", count);
        Self {
            thread_count: count,
        }
    }

    fn compute_count(max_render_threads: i32) -> usize {
        let hw = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let max = if max_render_threads <= 0 {
            1
        } else {
            max_render_threads as usize
        };
        max.min(hw).max(1)
    }

    /// Number of threads (including the calling thread).
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    /// Update the thread count from a new config value.
    pub fn update_thread_count(&mut self, max_render_threads: i32) {
        self.thread_count = Self::compute_count(max_render_threads);
    }

    /// Call `f(index)` for `index` in `0..count`, distributed across threads.
    ///
    /// The calling thread participates in the work (matching C++ behavior).
    /// If `thread_count == 1`, runs everything on the calling thread.
    pub fn call_parallel<F>(&self, f: F, count: usize)
    where
        F: Fn(usize) + Send + Sync,
    {
        if count == 0 {
            return;
        }
        if self.thread_count <= 1 || count == 1 {
            for i in 0..count {
                f(i);
            }
            return;
        }

        let counter = AtomicUsize::new(0);
        let f_ref = &f;

        std::thread::scope(|s| {
            // Spawn N-1 worker threads; the calling thread also participates.
            let workers = self.thread_count.min(count) - 1;
            for _ in 0..workers {
                s.spawn(|| loop {
                    let idx = counter.fetch_add(1, Ordering::Relaxed);
                    if idx >= count {
                        break;
                    }
                    f_ref(idx);
                });
            }
            // Calling thread participates.
            loop {
                let idx = counter.fetch_add(1, Ordering::Relaxed);
                if idx >= count {
                    break;
                }
                f_ref(idx);
            }
        });
    }
}
