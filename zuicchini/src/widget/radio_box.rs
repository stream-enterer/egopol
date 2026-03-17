use std::cell::RefCell;
use std::rc::Rc;

use crate::foundation::Color;
use crate::input::{Cursor, InputEvent, InputKey, InputVariant};
use crate::render::Painter;

use super::border::{Border, OuterBorderType};
use super::look::Look;
use super::radio_button::RadioGroup;

const CIRCLE_SIZE: f64 = 9.0;
const CIRCLE_LABEL_GAP: f64 = 4.0;

/// Small radio button variant — circle indicator with label text.
///
/// C++ `emRadioBox` inherits `emRadioButton : emCheckButton : emButton : emBorder`.
/// Constructor sets: `OBT_MARGIN`, `LabelAlignment=LEFT`, `ShownBoxed=true`.
/// The border is used for hit-test geometry (CheckMouse) even though the visual
/// is a custom circle + label paint.
pub struct RadioBox {
    border: Border,
    label: String,
    look: Rc<Look>,
    group: Rc<RefCell<RadioGroup>>,
    index: usize,
    last_w: f64,
    last_h: f64,
}

impl RadioBox {
    pub fn new(label: &str, look: Rc<Look>, group: Rc<RefCell<RadioGroup>>, index: usize) -> Self {
        Self {
            border: Border::new(OuterBorderType::Margin)
                .with_caption(label)
                .with_label_in_border(false)
                .with_label_alignment(crate::render::TextAlignment::Left)
                .with_how_to(true),
            label: label.to_string(),
            look,
            group,
            index,
            last_w: 0.0,
            last_h: 0.0,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub fn is_selected(&self) -> bool {
        self.group.borrow().selected() == Some(self.index)
    }

    pub fn set_checked(&mut self, checked: bool) {
        if checked {
            self.group.borrow_mut().select(self.index);
        } else if self.is_selected() {
            self.group.borrow_mut().set_check_index(None);
        }
    }

    pub fn paint(&mut self, painter: &mut Painter, _w: f64, _h: f64) {
        self.last_w = _w;
        self.last_h = _h;

        let cx = CIRCLE_SIZE / 2.0;
        let cy = CIRCLE_SIZE / 2.0;
        let r = CIRCLE_SIZE / 2.0;

        // Outer circle
        painter.paint_ellipse(cx, cy, r, r, self.look.input_bg_color, Color::TRANSPARENT);

        // Border ring — approximate with a slightly larger ellipse underneath
        painter.paint_ellipse(cx, cy, r, r, self.look.border_tint(), Color::TRANSPARENT);
        painter.paint_ellipse(
            cx,
            cy,
            r - 1.0,
            r - 1.0,
            self.look.input_bg_color,
            Color::TRANSPARENT,
        );

        // Filled dot when selected
        if self.is_selected() {
            painter.paint_ellipse(
                cx,
                cy,
                r - 2.5,
                r - 2.5,
                self.look.input_hl_color,
                Color::TRANSPARENT,
            );
        }

        // Label
        if !self.label.is_empty() {
            let label_x = CIRCLE_SIZE + CIRCLE_LABEL_GAP;
            let label_h = CIRCLE_SIZE;
            let label_y = (CIRCLE_SIZE - label_h) * 0.5;
            painter.paint_text(
                label_x,
                label_y,
                &self.label,
                label_h,
                1.0,
                self.look.fg_color,
                Color::TRANSPARENT,
            );
        }
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
                self.group.borrow_mut().select(self.index);
                true
            }
            InputKey::Space if event.variant == InputVariant::Release => {
                self.group.borrow_mut().select(self.index);
                true
            }
            _ => false,
        }
    }

    pub fn get_cursor(&self) -> Cursor {
        Cursor::Hand
    }

    pub fn preferred_size(&self) -> (f64, f64) {
        let w = if self.label.is_empty() {
            CIRCLE_SIZE
        } else {
            CIRCLE_SIZE + CIRCLE_LABEL_GAP + Painter::measure_text_width(&self.label, CIRCLE_SIZE)
        };
        (w, CIRCLE_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radio_box_selection() {
        let look = Look::new();
        let group = RadioGroup::new();

        let mut rb0 = RadioBox::new("X", look.clone(), group.clone(), 0);
        let mut rb1 = RadioBox::new("Y", look, group.clone(), 1);

        assert!(!rb0.is_selected());
        assert!(!rb1.is_selected());

        // Mouse clicks require paint; use Space for unit test.
        rb0.input(&InputEvent::release(InputKey::Space));
        assert!(rb0.is_selected());
        assert!(!rb1.is_selected());

        rb1.input(&InputEvent::release(InputKey::Space));
        assert!(!rb0.is_selected());
        assert!(rb1.is_selected());
    }
}
