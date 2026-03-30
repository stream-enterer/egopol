// SPLIT: Split from emColorField.h — inner panel type extracted
use std::rc::Rc;

use crate::emCore::emCursor::emCursor;
use crate::emCore::emInput::emInputEvent;
use crate::emCore::emInputState::emInputState;
use crate::emCore::emPanel::{NoticeFlags, PanelBehavior, PanelState};
use crate::emCore::emPainter::emPainter;

use super::emBorder::{InnerBorderType, OuterBorderType};
use crate::emCore::emButton::emButton;
use crate::emCore::emCheckBox::emCheckBox;
use crate::emCore::emLabel::emLabel;
use crate::emCore::emListBox::emListBox;
use crate::emCore::emLook::emLook;
use crate::emCore::emScalarField::emScalarField;
use crate::emCore::emTextField::emTextField;

/// PanelBehavior wrapper for emScalarField — used by emColorField expansion.
pub(crate) struct ScalarFieldPanel {
    pub scalar_field: emScalarField,
}

impl ScalarFieldPanel {
    pub fn new(
        caption: &str,
        min: f64,
        max: f64,
        value: f64,
        look: Rc<emLook>,
        editable: bool,
    ) -> Self {
        let mut sf = emScalarField::new(min, max, look);
        sf.SetCaption(caption);
        sf.SetValue(value);
        sf.SetEditable(editable);
        sf.border_mut().outer = OuterBorderType::Rect;
        sf.border_mut().inner = InnerBorderType::CustomRect;
        sf.border_mut().SetBorderScaling(2.0);
        Self { scalar_field: sf }
    }
}

impl PanelBehavior for ScalarFieldPanel {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        self.scalar_field.Paint(painter, w, h, state.enabled);
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
    ) -> bool {
        self.scalar_field.Input(event, _state, _input_state)
    }

    fn GetCursor(&self) -> emCursor {
        self.scalar_field.GetCursor()
    }
}

/// PanelBehavior wrapper for emTextField — used by emColorField expansion.
pub(crate) struct TextFieldPanel {
    pub text_field: emTextField,
}

impl TextFieldPanel {
    pub fn new(caption: &str, text: &str, look: Rc<emLook>, editable: bool) -> Self {
        let mut tf = emTextField::new(look);
        tf.SetCaption(caption);
        tf.SetText(text);
        tf.SetEditable(editable);
        tf.border_mut().outer = OuterBorderType::Rect;
        tf.border_mut().inner = InnerBorderType::CustomRect;
        tf.border_mut().SetBorderScaling(2.0);
        Self { text_field: tf }
    }
}

impl PanelBehavior for TextFieldPanel {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        self.text_field.cycle_blink(state.in_focused_path());
        self.text_field.Paint(painter, w, h, state.enabled);
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.text_field.on_focus_changed(state.in_focused_path());
        }
    }
}

/// PanelBehavior wrapper for emCheckBox.
pub(crate) struct CheckBoxPanel {
    pub check_box: emCheckBox,
}

impl PanelBehavior for CheckBoxPanel {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        self.check_box.Paint(painter, w, h, state.enabled);
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
    ) -> bool {
        self.check_box.Input(event, _state, _input_state)
    }

    fn GetCursor(&self) -> emCursor {
        self.check_box.GetCursor()
    }
}

/// PanelBehavior wrapper for emListBox.
pub(crate) struct ListBoxPanel {
    pub list_box: emListBox,
}

impl PanelBehavior for ListBoxPanel {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        self.list_box.Paint(painter, w, h);
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
    ) -> bool {
        self.list_box.Input(event, _state, _input_state)
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.list_box.on_focus_changed(state.in_active_path);
        }
        if flags.intersects(NoticeFlags::ENABLE_CHANGED) {
            self.list_box.on_enable_changed(state.enabled);
        }
    }
}

/// PanelBehavior wrapper for emButton.
pub(crate) struct ButtonPanel {
    pub button: emButton,
}

impl PanelBehavior for ButtonPanel {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, state: &PanelState) {
        self.button.Paint(painter, w, h, state.enabled);
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        _input_state: &emInputState,
    ) -> bool {
        self.button.Input(event, _state, _input_state)
    }

    fn GetCursor(&self) -> emCursor {
        self.button.GetCursor()
    }
}

/// PanelBehavior wrapper for emLabel (non-focusable text display).
pub(crate) struct LabelPanel {
    pub label: emLabel,
}

impl PanelBehavior for LabelPanel {
    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        self.label.PaintContent(painter, w, h, _state.enabled);
    }
}
