use crate::foundation::{Color, Image};
use crate::panel::{PanelBehavior, PanelState};
use crate::render::Painter;
use crate::widget::file_panel::FilePanel;

/// A panel that displays an image file with aspect-ratio preservation.
///
/// Port of C++ `emImageFilePanel`. Wraps a `FilePanel` for status display
/// and holds a cached copy of the current image for painting.
pub struct ImageFilePanel {
    file_panel: FilePanel,
    current_image: Option<Image>,
}

impl Default for ImageFilePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageFilePanel {
    pub fn new() -> Self {
        Self {
            file_panel: FilePanel::new(),
            current_image: None,
        }
    }

    pub fn with_model() -> Self {
        Self {
            file_panel: FilePanel::with_model(),
            current_image: None,
        }
    }

    pub fn file_panel(&self) -> &FilePanel {
        &self.file_panel
    }

    pub fn file_panel_mut(&mut self) -> &mut FilePanel {
        &mut self.file_panel
    }

    /// Update the cached image for painting.
    pub fn set_current_image(&mut self, image: Option<Image>) {
        self.current_image = image;
    }

    /// Calculate the aspect-ratio-preserving rectangle for the image within
    /// the panel bounds. Returns `(x, y, w, h)` or `None` if no image.
    ///
    /// Port of C++ `emImageFilePanel::GetEssenceRect`. The image is centered
    /// within panel width 1.0 and proportional height.
    pub fn get_essence_rect(&self, panel_w: f64, panel_h: f64) -> Option<(f64, f64, f64, f64)> {
        let image = self.current_image.as_ref()?;
        let iw = image.width() as f64;
        let ih = image.height() as f64;
        if iw <= 0.0 || ih <= 0.0 || panel_w <= 0.0 || panel_h <= 0.0 {
            return None;
        }

        let image_aspect = iw / ih;
        let panel_aspect = panel_w / panel_h;

        if image_aspect > panel_aspect {
            // Image is wider than panel — fit to width, center vertically
            let w = panel_w;
            let h = panel_w / image_aspect;
            let x = 0.0;
            let y = (panel_h - h) * 0.5;
            Some((x, y, w, h))
        } else {
            // Image is taller than panel — fit to height, center horizontally
            let h = panel_h;
            let w = panel_h * image_aspect;
            let x = (panel_w - w) * 0.5;
            let y = 0.0;
            Some((x, y, w, h))
        }
    }
}

impl PanelBehavior for ImageFilePanel {
    fn is_opaque(&self) -> bool {
        if self.file_panel.vir_file_state().is_good() {
            false
        } else {
            self.file_panel.is_opaque()
        }
    }

    fn canvas_color(&self) -> Color {
        self.file_panel.canvas_color()
    }

    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        if !self.file_panel.vir_file_state().is_good() {
            self.file_panel.paint(painter, w, h, state);
            return;
        }

        if let Some(ref image) = self.current_image {
            if let Some((ix, iy, iw, ih)) = self.get_essence_rect(w, h) {
                let canvas_color = painter.canvas_color();
                painter.paint_image_full(ix, iy, iw, ih, image, 255, canvas_color);
            }
        }
    }
}
