use crate::foundation::Color;
use crate::panel::{PanelBehavior, PanelState};
use crate::render::{Painter, TextAlignment, VAlign};

/// A red panel that displays an error message with yellow text.
///
/// Port of C++ `emErrorPanel`. Used to show error conditions with a
/// distinctive visual style (dark red background, yellow text).
pub struct ErrorPanel {
    error_message: String,
}

const BG_COLOR: Color = Color::rgb(128, 0, 0);
const FG_COLOR: Color = Color::rgb(255, 255, 0);

impl ErrorPanel {
    pub fn new(error_message: &str) -> Self {
        Self {
            error_message: error_message.to_string(),
        }
    }

    pub fn error_message(&self) -> &str {
        &self.error_message
    }

    pub fn set_error_message(&mut self, message: &str) {
        self.error_message = message.to_string();
    }
}

impl PanelBehavior for ErrorPanel {
    fn is_opaque(&self) -> bool {
        true
    }

    fn canvas_color(&self) -> Color {
        BG_COLOR
    }

    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        let canvas_color = painter.canvas_color();

        painter.paint_rect(0.0, 0.0, w, h, BG_COLOR, canvas_color);

        if self.error_message.is_empty() {
            return;
        }

        painter.paint_text_boxed(
            0.05 * w,
            0.05 * h,
            0.9 * w,
            0.9 * h,
            &self.error_message,
            h / 2.0,
            FG_COLOR,
            BG_COLOR,
            TextAlignment::Center,
            VAlign::Center,
            TextAlignment::Left,
            1.0,
            false,
            0.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_panel_message() {
        let mut panel = ErrorPanel::new("Something went wrong");
        assert_eq!(panel.error_message(), "Something went wrong");
        panel.set_error_message("New error");
        assert_eq!(panel.error_message(), "New error");
    }

    #[test]
    fn error_panel_is_opaque() {
        let panel = ErrorPanel::new("error");
        assert!(panel.is_opaque());
    }

    #[test]
    fn error_panel_canvas_color() {
        let panel = ErrorPanel::new("error");
        assert_eq!(panel.canvas_color(), BG_COLOR);
    }
}
