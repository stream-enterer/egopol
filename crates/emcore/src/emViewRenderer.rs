use crate::emCore::emColor::emColor;
use crate::emCore::emImage::emImage;
use crate::emCore::emPanelTree::PanelTree;
use crate::emCore::emView::emView;

use crate::emCore::emPainterDrawList::DrawList;
use crate::emCore::emRenderThreadPool::emRenderThreadPool;
use super::emPainter::emPainter;

pub struct SoftwareCompositor {
    framebuffer: emImage,
}

impl SoftwareCompositor {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: emImage::new(width, height, 4),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.framebuffer = emImage::new(width, height, 4);
    }

    pub fn render(&mut self, tree: &mut PanelTree, view: &emView) {
        self.framebuffer.fill(emColor::BLACK);
        let mut painter = emPainter::new(&mut self.framebuffer);
        view.Paint(tree, &mut painter);
    }

    /// Render using the display-list + parallel-replay pipeline.
    ///
    /// 1. Record all draw operations via a recording `emPainter`.
    /// 2. Split the framebuffer into tiles of `tile_size × tile_size`.
    /// 3. Replay the draw list into each tile in parallel using `pool`.
    /// 4. Composite tile results back into the framebuffer.
    pub fn render_parallel(
        &mut self,
        tree: &mut PanelTree,
        view: &emView,
        pool: &emRenderThreadPool,
        tile_size: u32,
    ) {
        let w = self.framebuffer.GetWidth();
        let h = self.framebuffer.GetHeight();

        // Phase 1: record.
        let mut draw_list = DrawList::new();
        {
            let mut rec = emPainter::new_recording(w, h, draw_list.ops_mut());
            view.Paint(tree, &mut rec);
        }

        // Phase 2: split into tiles and replay in parallel.
        let cols = w.div_ceil(tile_size);
        let rows = h.div_ceil(tile_size);
        let tile_count = (cols * rows) as usize;
        let ts = tile_size as f64;

        let results: Vec<std::sync::Mutex<Option<emImage>>> = (0..tile_count)
            .map(|_| std::sync::Mutex::new(None::<emImage>))
            .collect();
        let results_ref = &results;
        let draw_list_ref = &draw_list;

        pool.CallParallel(
            |idx| {
                let col = (idx as u32) % cols;
                let row = (idx as u32) / cols;
                let tw = tile_size.min(w - col * tile_size);
                let th = tile_size.min(h - row * tile_size);
                let mut buf = emImage::new(tw, th, 4);
                buf.fill(emColor::BLACK);
                {
                    let mut p = emPainter::new(&mut buf);
                    draw_list_ref.replay(&mut p, (col as f64 * ts, row as f64 * ts));
                }
                *results_ref[idx].lock().expect("poisoned") = Some(buf);
            },
            tile_count,
        );

        // Phase 3: composite tiles into framebuffer.
        self.framebuffer.fill(emColor::BLACK);
        for (idx, result) in results.iter().enumerate() {
            let col = (idx as u32) % cols;
            let row = (idx as u32) / cols;
            if let Some(buf) = result.lock().expect("poisoned").take() {
                self.framebuffer.copy_from_rect(
                    col * tile_size,
                    row * tile_size,
                    &buf,
                    (0, 0, buf.GetWidth(), buf.GetHeight()),
                );
            }
        }
    }

    pub fn framebuffer(&self) -> &emImage {
        &self.framebuffer
    }
}
