use std::rc::Rc;

use crate::input::{Cursor, InputEvent, InputKey, InputVariant};
use crate::render::Painter;

use super::border::{Border, OuterBorderType};
use super::look::Look;

/// Toggle button widget — visually depressed when checked.
pub struct CheckButton {
    border: Border,
    look: Rc<Look>,
    checked: bool,
    last_w: f64,
    last_h: f64,
    pub on_check: Option<Box<dyn FnMut(bool)>>,
}

impl CheckButton {
    pub fn new(caption: &str, look: Rc<Look>) -> Self {
        Self {
            border: Border::new(OuterBorderType::RoundRect)
                .with_caption(caption)
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

    pub fn paint(&mut self, painter: &mut Painter, w: f64, h: f64) {
        self.last_w = w;
        self.last_h = h;
        let face_color = if self.checked {
            self.look.button_pressed()
        } else {
            self.look.button_bg_color
        };
        painter.paint_round_rect(1.0, 1.0, w - 2.0, h - 2.0, 3.0, face_color);
        self.border
            .paint_border(painter, w, h, &self.look, false, true);
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

    /// Whether this check button provides how-to help text.
    /// Matches C++ `emCheckButton::HasHowTo` (inherited from emButton, always true).
    pub fn has_how_to(&self) -> bool {
        true
    }

    /// Help text describing how to use this check button.
    ///
    /// Chains the border's base how-to with button + check-button specific
    /// sections. Matches C++ `emCheckButton::GetHowTo`.
    pub fn get_how_to(&self, enabled: bool, focusable: bool) -> String {
        let mut text = self.border.get_howto(enabled, focusable);
        text.push_str(HOWTO_CHECK_BUTTON);
        if self.checked {
            text.push_str(HOWTO_CHECKED);
        } else {
            text.push_str(HOWTO_NOT_CHECKED);
        }
        text
    }

    fn toggle(&mut self) {
        self.checked = !self.checked;
        if let Some(cb) = &mut self.on_check {
            cb(self.checked);
        }
    }
}

/// C++ `emCheckButton::HowToCheckButton`.
const HOWTO_CHECK_BUTTON: &str = "\n\n\
    CHECK BUTTON\n\n\
    This button can have checked or unchecked state. Usually this is a yes-or-no\n\
    answer to a question. Whenever the button is triggered, the check state toggles.\n";

/// C++ `emCheckButton::HowToChecked`.
const HOWTO_CHECKED: &str = "\n\n\
    CHECKED\n\n\
    Currently this check button is checked.\n";

/// C++ `emCheckButton::HowToNotChecked`.
const HOWTO_NOT_CHECKED: &str = "\n\n\
    UNCHECKED\n\n\
    Currently this check button is not checked.\n";

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn toggle_state() {
        let look = Look::new();
        let mut btn = CheckButton::new("Toggle", look);
        assert!(!btn.is_checked());
        btn.input(&InputEvent::release(InputKey::MouseLeft));
        // No paint call → last_w/last_h are 0 → check_mouse returns false
        // so mouse click is ignored. Use Space (keyboard, no hit test).
        assert!(!btn.is_checked());
        btn.input(&InputEvent::release(InputKey::Space));
        assert!(btn.is_checked());
        btn.input(&InputEvent::release(InputKey::Space));
        assert!(!btn.is_checked());
    }

    #[test]
    fn callback_receives_state() {
        let look = Look::new();
        let states = Rc::new(RefCell::new(Vec::new()));
        let states_clone = states.clone();

        let mut btn = CheckButton::new("CB", look);
        btn.on_check = Some(Box::new(move |checked| {
            states_clone.borrow_mut().push(checked);
        }));

        btn.input(&InputEvent::release(InputKey::Space));
        btn.input(&InputEvent::release(InputKey::Space));
        assert_eq!(*states.borrow(), vec![true, false]);
    }
}
