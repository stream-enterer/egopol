use std::rc::Rc;

use crate::foundation::{Color, Rect};
use crate::input::{Cursor, InputEvent, InputKey, InputVariant};
use crate::render::{LineCap, LineJoin, Painter, Stroke};

use super::border::{Border, OuterBorderType};
use super::look::Look;
use super::toolkit_images::with_toolkit_images;

/// CheckBox widget — Margin border with ShownBoxed paint path.
/// Matches C++ `emCheckBox` (which extends `emCheckButton` extends `emButton`).
///
/// C++ constructor chain:
///   emButton: OBT_INSTRUMENT_MORE_ROUND, LabelInBorder=false, ALIGN_CENTER
///   emCheckBox overrides: OBT_MARGIN, ALIGN_LEFT, ShownBoxed=true
pub struct CheckBox {
    border: Border,
    look: Rc<Look>,
    checked: bool,
    last_w: f64,
    last_h: f64,
    pub on_check: Option<Box<dyn FnMut(bool)>>,
}

impl CheckBox {
    pub fn new(label: &str, look: Rc<Look>) -> Self {
        Self {
            border: Border::new(OuterBorderType::Margin)
                .with_caption(label)
                .with_label_in_border(false)
                .with_label_alignment(crate::render::TextAlignment::Left)
                .with_how_to(true),
            look,
            checked: false,
            last_w: 0.0,
            last_h: 0.0,
            on_check: None,
        }
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    /// Paint using the C++ ShownBoxed path (emButton.cpp:233-341).
    ///
    /// Layout: small checkbox box on the left, label text on the right.
    /// The box contains: InputBgColor face → checkmark symbol → CheckBox image overlay.
    pub fn paint(&mut self, painter: &mut Painter, w: f64, h: f64) {
        self.last_w = w;
        self.last_h = h;
        // Paint outer border (Margin = transparent spacing only).
        self.border
            .paint_border(painter, w, h, &self.look, false, true);

        // C++ DoButton ShownBoxed: GetContentRect, then compute box + label geometry.
        let cr = self.border.content_rect(w, h, &self.look);

        let has_label = self.border.has_label();
        let (bx0, by0, bw0, lx, ly, lw, lh);
        if has_label {
            // C++ lines 239-249: label tallness drives proportions.
            let label_tallness = self.border.best_label_tallness().max(0.2);
            let mut box_w = label_tallness;
            let mut d = box_w * 0.1;
            let f = (cr.w / (box_w + d + 1.0)).min(cr.h / label_tallness);
            box_w *= f;
            d *= f;
            lw = cr.w - box_w - d;
            lh = box_w;
            lx = cr.x + cr.w - lw;
            ly = cr.y + (cr.h - lh) * 0.5;
            bw0 = box_w;
            bx0 = cr.x;
            by0 = cr.y + (cr.h - bw0) * 0.5;
        } else {
            bw0 = cr.w.min(cr.h);
            bx0 = cr.x;
            by0 = cr.y + (cr.h - bw0) * 0.5;
            lx = cr.x;
            ly = cr.y;
            lw = 0.0;
            lh = 0.0;
        }

        // Paint label to the right of the box.
        if has_label {
            self.border
                .paint_label(painter, Rect::new(lx, ly, lw, lh), &self.look, true);
        }

        // Inset for image area: d = bw * 0.13 (C++ line 262).
        let d = bw0 * 0.13;
        let bx = bx0 + d;
        let by = by0 + d;
        let bw = bw0 - 2.0 * d;
        let bh = bw;

        // Face inset: d = bw * 30/380 (C++ line 268).
        let d2 = bw * 30.0 / 380.0;
        let fx = bx + d2;
        let fy = by + d2;
        let fw = bw - 2.0 * d2;
        let fh = bh - 2.0 * d2;
        let fr = bw * 50.0 / 380.0;

        // Paint face (InputBgColor).
        let face_color = self.look.input_bg_color;
        painter.paint_round_rect(fx, fy, fw, fh, fr, face_color);
        painter.set_canvas_color(face_color);

        // Paint check symbol if checked (C++ PaintBoxSymbol, emButton.cpp:160-184).
        if self.checked {
            let check_color = self.look.input_fg_color;
            let verts = [
                (fx + fw * 0.2, fy + fh * 0.6),
                (fx + fw * 0.4, fy + fh * 0.8),
                (fx + fw * 0.8, fy + fh * 0.2),
            ];
            let mut stroke = Stroke::new(check_color, fw * 0.16);
            stroke.join = LineJoin::Round;
            stroke.cap = LineCap::Round;
            painter.paint_solid_polyline(&verts, &stroke, false, Color::TRANSPARENT);
        }

        // Paint checkbox image overlay (C++ line 318-331).
        with_toolkit_images(|img| {
            painter.paint_image_full(bx, by, bw, bh, &img.check_box, 255, Color::TRANSPARENT);
        });
    }

    /// Rounded-rect hit test matching C++ `emButton::CheckMouse`.
    fn hit_test(&self, mx: f64, my: f64) -> bool {
        if self.last_w <= 0.0 || self.last_h <= 0.0 {
            return false;
        }
        let (rect, r) = self
            .border
            .content_round_rect(self.last_w, self.last_h, &self.look);
        super::check_mouse_round_rect(mx, my, &rect, r)
    }

    pub fn input(&mut self, event: &InputEvent) -> bool {
        match event.key {
            InputKey::MouseLeft if event.variant == InputVariant::Release => {
                if !self.hit_test(event.mouse_x, event.mouse_y) {
                    return false;
                }
                self.toggle();
                true
            }
            InputKey::Space if event.variant == InputVariant::Release => {
                self.toggle();
                true
            }
            _ => false,
        }
    }

    pub fn get_cursor(&self) -> Cursor {
        Cursor::Hand
    }

    pub fn preferred_size(&self) -> (f64, f64) {
        let th = 13.0;
        let tw = Painter::measure_text_width(&self.border.caption, th);
        self.border.preferred_size_for_content(tw + 8.0, th + 4.0)
    }

    fn toggle(&mut self) {
        self.checked = !self.checked;
        if let Some(cb) = &mut self.on_check {
            cb(self.checked);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_toggle() {
        let look = Look::new();
        let mut cb = CheckBox::new("Enable", look);
        assert!(!cb.is_checked());
        // Mouse clicks require paint to set last_w/last_h; use Space for unit test.
        cb.input(&InputEvent::release(InputKey::Space));
        assert!(cb.is_checked());
        cb.input(&InputEvent::release(InputKey::Space));
        assert!(!cb.is_checked());
    }

    #[test]
    fn checkbox_preferred_size() {
        let look = Look::new();
        let cb = CheckBox::new("Hi", look);
        let (w, h) = cb.preferred_size();
        assert!(w > 0.0, "Should have positive width");
        assert!(h > 0.0, "Should have positive height");
    }
}
