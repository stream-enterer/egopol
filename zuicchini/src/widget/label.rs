use std::rc::Rc;

use crate::foundation::Color;
use crate::render::{Painter, TextAlignment, VAlign};

use super::border::{Border, OuterBorderType};
use super::look::Look;

/// Non-focusable text display widget.
pub struct Label {
    border: Border,
    look: Rc<Look>,
}

impl Label {
    pub fn new(caption: &str, look: Rc<Look>) -> Self {
        Self {
            border: Border::new(OuterBorderType::Margin)
                .with_caption(caption)
                .with_label_in_border(false),
            look,
        }
    }

    pub fn set_caption(&mut self, text: &str) {
        self.border.caption = text.to_string();
    }

    pub fn caption(&self) -> &str {
        &self.border.caption
    }

    /// Set horizontal alignment of the label block within content area.
    /// Matches C++ `emBorder::SetLabelAlignment`.
    pub fn set_label_alignment(&mut self, a: TextAlignment) {
        self.border.label_alignment = a;
    }

    /// Set text line alignment for the caption.
    /// Matches C++ `emBorder::SetCaptionAlignment`.
    pub fn set_caption_alignment(&mut self, a: TextAlignment) {
        self.border.set_caption_alignment(Some(a));
    }

    pub fn paint(&self, painter: &mut Painter, w: f64, h: f64) {
        self.border
            .paint_border(painter, w, h, &self.look, false, true);

        if self.border.caption.is_empty() {
            return;
        }

        // C++ emLabel::PaintContent → PaintLabel → DoLabel.
        // DoLabel measures text at unit height, then scales proportionally
        // to fit the content area.
        let cr = self.border.content_rect(w, h, &self.look);
        let mut cx = cr.x;
        let mut cy = cr.y;
        let mut cw = cr.w;
        let mut ch = cr.h;

        if cw <= 0.0 || ch <= 0.0 {
            return;
        }

        let min_ws = 0.5_f64;

        // Measure text at unit height.
        let (cap_w, cap_h) = Painter::get_text_size(&self.border.caption, 1.0, true, 0.0);
        if cap_w <= 0.0 || cap_h <= 0.0 {
            return;
        }

        // Scale to fill height.
        let mut f = ch / cap_h;
        let w2 = f * cap_w;

        if w2 <= cw {
            // Fits horizontally — apply LabelAlignment (C++ emBorder.cpp:1292-1301).
            let slack = cw - w2;
            match self.border.label_alignment {
                TextAlignment::Left => {}
                TextAlignment::Center => cx += slack * 0.5,
                TextAlignment::Right => cx += slack,
            }
            cw = w2;
        } else {
            // Width constrained — check if min squeeze fits.
            let min_total_w = cap_w * min_ws;
            let w2_min = f * min_total_w;
            if w2_min > cw {
                // Must reduce height to fit.
                f = cw / min_total_w;
                let h2 = f * cap_h;
                // Center vertically.
                cy += (ch - h2) * 0.5;
                ch = h2;
            }
        }

        let char_h = cap_h * f;
        // C++ DoLabel: boxAlignment=EM_ALIGN_CENTER, textAlignment=CaptionAlignment.
        let cap_align = self
            .border
            .caption_alignment
            .unwrap_or(self.border.label_alignment);

        painter.paint_text_boxed(
            cx,
            cy,
            cw,
            ch,
            &self.border.caption,
            char_h,
            self.look.fg_color,
            Color::TRANSPARENT,
            TextAlignment::Center,
            VAlign::Center,
            cap_align,
            min_ws,
            true,
            0.0,
        );
    }

    pub fn preferred_size(&self) -> (f64, f64) {
        let ch = 13.0;
        let tw = Painter::measure_text_width(&self.border.caption, ch);
        let lh = ch + 2.0;
        (tw + 4.0, lh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_caption() {
        let look = Look::new();
        let mut label = Label::new("Hello", look);
        assert_eq!(label.caption(), "Hello");
        label.set_caption("World");
        assert_eq!(label.caption(), "World");
    }

    #[test]
    fn label_preferred_size() {
        let look = Look::new();
        let label = Label::new("Test", look);
        let (w, h) = label.preferred_size();
        // Width = measured text width + 4px padding
        assert!(w > 4.0, "Label should have positive width");
        assert!(h > 0.0, "Label should have positive height");
    }
}
