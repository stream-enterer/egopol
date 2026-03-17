mod border;
mod button;
mod check_box;
mod check_button;
mod color_field;
mod core_config_panel;
mod dialog;
mod error_panel;
pub(crate) mod field_panel;
mod file_dialog;
mod file_panel;
mod file_selection_box;
mod image_file_panel;
mod label;
mod list_box;
mod look;
mod radio_box;
mod radio_button;
mod scalar_field;
mod splitter;
mod text_field;
pub(crate) mod toolkit_images;
mod tunnel;

pub use border::{Border, InnerBorderType, OuterBorderType};
pub use button::Button;
pub use check_box::CheckBox;
pub use check_button::CheckButton;
pub use color_field::{ColorField, Expansion};
pub use core_config_panel::CoreConfigPanel;
pub use dialog::{Dialog, DialogResult};
pub use error_panel::ErrorPanel;
pub use file_dialog::{FileDialog, FileDialogCheckResult, FileDialogMode};
pub use file_panel::{FilePanel, VirtualFileState};
pub use file_selection_box::FileSelectionBox;
pub use image_file_panel::ImageFilePanel;
pub use label::Label;
pub use list_box::{DefaultItemPanel, ItemPanelInterface, ListBox, SelectionMode};
pub use look::Look;
pub use radio_box::RadioBox;
pub use radio_button::{RadioButton, RadioGroup, RadioLinearGroup, RadioRasterGroup};
pub use scalar_field::ScalarField;
pub use splitter::Splitter;
pub use text_field::TextField;
pub use tunnel::{Tunnel, TunnelChildRect};

use crate::foundation::Rect;

/// Rounded-rectangle hit test matching the C++ signed-distance formula used by
/// `emButton::CheckMouse`, `emTextField::CheckMouse`, and
/// `emScalarField::CheckMouse`.
///
/// Returns `true` when `(mx, my)` lies inside the rounded rectangle defined by
/// `rect` with corner radius `r`.
///
/// Formula: `dx = max(max(rx - mx, mx - rx - rw) + r, 0)`, same for dy,
/// then `hit = dx² + dy² ≤ r²`.
pub(crate) fn check_mouse_round_rect(mx: f64, my: f64, rect: &Rect, r: f64) -> bool {
    let dx = ((rect.x - mx).max(mx - rect.x - rect.w) + r).max(0.0);
    let dy = ((rect.y - my).max(my - rect.y - rect.h) + r).max(0.0);
    dx * dx + dy * dy <= r * r
}
