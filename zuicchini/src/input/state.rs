use std::collections::HashSet;

use super::event::InputKey;

/// Tracks the current state of all input devices.
/// (C++ emInputState operator== / operator!= parity via PartialEq.)
#[derive(Clone, Debug, PartialEq)]
pub struct InputState {
    /// Currently pressed keys.
    pressed: HashSet<InputKey>,
    /// Current mouse position in window coordinates.
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// Current touch points (id, x, y).
    touches: Vec<(u64, f64, f64)>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            touches: Vec::new(),
        }
    }

    /// Record a key press.
    pub fn press(&mut self, key: InputKey) {
        self.pressed.insert(key);
    }

    /// Record a key release.
    pub fn release(&mut self, key: InputKey) {
        self.pressed.remove(&key);
    }

    /// Check if a key is currently pressed.
    pub fn is_pressed(&self, key: InputKey) -> bool {
        self.pressed.contains(&key)
    }

    /// Check if Shift is held.
    pub fn shift(&self) -> bool {
        self.is_pressed(InputKey::Shift)
    }

    /// Check if Ctrl is held.
    pub fn ctrl(&self) -> bool {
        self.is_pressed(InputKey::Ctrl)
    }

    /// Check if Alt is held.
    pub fn alt(&self) -> bool {
        self.is_pressed(InputKey::Alt)
    }

    /// Check if Meta (Cmd/Win) is held.
    pub fn meta(&self) -> bool {
        self.is_pressed(InputKey::Meta)
    }

    /// Whether the left mouse button is pressed.
    /// (C++ emInputState::GetLeftButton parity.)
    pub fn left_button(&self) -> bool {
        self.is_pressed(InputKey::MouseLeft)
    }

    /// Whether the middle mouse button is pressed.
    /// (C++ emInputState::GetMiddleButton parity.)
    pub fn middle_button(&self) -> bool {
        self.is_pressed(InputKey::MouseMiddle)
    }

    /// Whether the right mouse button is pressed.
    /// (C++ emInputState::GetRightButton parity.)
    pub fn right_button(&self) -> bool {
        self.is_pressed(InputKey::MouseRight)
    }

    // ── Modifier combo tests (C++ emInputState parity) ────────────────
    // Each tests that *exactly* the named modifiers are held, nothing more.

    /// No modifier keys held.
    pub fn is_no_mod(&self) -> bool {
        !self.shift() && !self.ctrl() && !self.alt() && !self.meta()
    }

    /// Exactly Shift held.
    pub fn is_shift_mod(&self) -> bool {
        self.shift() && !self.ctrl() && !self.alt() && !self.meta()
    }

    /// Exactly Ctrl held.
    pub fn is_ctrl_mod(&self) -> bool {
        !self.shift() && self.ctrl() && !self.alt() && !self.meta()
    }

    /// Exactly Alt held.
    pub fn is_alt_mod(&self) -> bool {
        !self.shift() && !self.ctrl() && self.alt() && !self.meta()
    }

    /// Exactly Meta held.
    pub fn is_meta_mod(&self) -> bool {
        !self.shift() && !self.ctrl() && !self.alt() && self.meta()
    }

    /// Exactly Shift+Ctrl held.
    pub fn is_shift_ctrl_mod(&self) -> bool {
        self.shift() && self.ctrl() && !self.alt() && !self.meta()
    }

    /// Exactly Shift+Alt held.
    pub fn is_shift_alt_mod(&self) -> bool {
        self.shift() && !self.ctrl() && self.alt() && !self.meta()
    }

    /// Exactly Shift+Meta held.
    pub fn is_shift_meta_mod(&self) -> bool {
        self.shift() && !self.ctrl() && !self.alt() && self.meta()
    }

    /// Exactly Ctrl+Alt held.
    pub fn is_ctrl_alt_mod(&self) -> bool {
        !self.shift() && self.ctrl() && self.alt() && !self.meta()
    }

    /// Exactly Ctrl+Meta held.
    pub fn is_ctrl_meta_mod(&self) -> bool {
        !self.shift() && self.ctrl() && !self.alt() && self.meta()
    }

    /// Exactly Alt+Meta held.
    pub fn is_alt_meta_mod(&self) -> bool {
        !self.shift() && !self.ctrl() && self.alt() && self.meta()
    }

    /// Exactly Shift+Ctrl+Alt held.
    pub fn is_shift_ctrl_alt_mod(&self) -> bool {
        self.shift() && self.ctrl() && self.alt() && !self.meta()
    }

    /// Exactly Shift+Ctrl+Meta held.
    pub fn is_shift_ctrl_meta_mod(&self) -> bool {
        self.shift() && self.ctrl() && !self.alt() && self.meta()
    }

    /// Exactly Shift+Alt+Meta held.
    pub fn is_shift_alt_meta_mod(&self) -> bool {
        self.shift() && !self.ctrl() && self.alt() && self.meta()
    }

    /// Exactly Ctrl+Alt+Meta held.
    pub fn is_ctrl_alt_meta_mod(&self) -> bool {
        !self.shift() && self.ctrl() && self.alt() && self.meta()
    }

    /// All four modifiers held.
    pub fn is_shift_ctrl_alt_meta_mod(&self) -> bool {
        self.shift() && self.ctrl() && self.alt() && self.meta()
    }

    /// Clear all pressed keys. Returns true if any key was pressed.
    pub fn clear_key_states(&mut self) -> bool {
        let had_keys = !self.pressed.is_empty();
        self.pressed.clear();
        had_keys
    }

    /// Update mouse position.
    pub fn set_mouse(&mut self, x: f64, y: f64) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    /// Update a touch point. Inserts if new, updates if existing.
    pub fn set_touch(&mut self, id: u64, x: f64, y: f64) {
        if let Some(touch) = self.touches.iter_mut().find(|t| t.0 == id) {
            touch.1 = x;
            touch.2 = y;
        } else {
            self.touches.push((id, x, y));
        }
    }

    /// Remove a touch point.
    pub fn remove_touch(&mut self, id: u64) {
        self.touches.retain(|t| t.0 != id);
    }

    /// Get all active touch points.
    pub fn touches(&self) -> &[(u64, f64, f64)] {
        &self.touches
    }

    /// Get the set of currently pressed keys.
    pub fn pressed_keys(&self) -> &HashSet<InputKey> {
        &self.pressed
    }

    /// Clear all active touch points.
    pub fn clear_touches(&mut self) {
        self.touches.clear();
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_combo_no_mod() {
        let s = InputState::new();
        assert!(s.is_no_mod());
        assert!(!s.is_shift_mod());
        assert!(!s.is_ctrl_mod());
    }

    #[test]
    fn modifier_combo_single() {
        let mut s = InputState::new();
        s.press(InputKey::Shift);
        assert!(s.is_shift_mod());
        assert!(!s.is_no_mod());
        assert!(!s.is_ctrl_mod());
        assert!(!s.is_shift_ctrl_mod());
    }

    #[test]
    fn modifier_combo_double() {
        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        assert!(s.is_shift_ctrl_mod());
        assert!(!s.is_shift_mod());
        assert!(!s.is_ctrl_mod());

        let mut s2 = InputState::new();
        s2.press(InputKey::Alt);
        s2.press(InputKey::Meta);
        assert!(s2.is_alt_meta_mod());
    }

    #[test]
    fn modifier_combo_triple() {
        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        assert!(s.is_shift_ctrl_alt_mod());
        assert!(!s.is_shift_ctrl_mod());
        assert!(!s.is_shift_ctrl_alt_meta_mod());
    }

    #[test]
    fn modifier_combo_all_four() {
        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        s.press(InputKey::Meta);
        assert!(s.is_shift_ctrl_alt_meta_mod());
        assert!(!s.is_shift_ctrl_alt_mod());
    }

    #[test]
    fn modifier_remaining_combos() {
        let mut s = InputState::new();
        s.press(InputKey::Alt);
        assert!(s.is_alt_mod());

        let mut s = InputState::new();
        s.press(InputKey::Meta);
        assert!(s.is_meta_mod());

        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Alt);
        assert!(s.is_shift_alt_mod());

        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Meta);
        assert!(s.is_shift_meta_mod());

        let mut s = InputState::new();
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        assert!(s.is_ctrl_alt_mod());

        let mut s = InputState::new();
        s.press(InputKey::Ctrl);
        s.press(InputKey::Meta);
        assert!(s.is_ctrl_meta_mod());

        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        s.press(InputKey::Meta);
        assert!(s.is_shift_ctrl_meta_mod());

        let mut s = InputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Alt);
        s.press(InputKey::Meta);
        assert!(s.is_shift_alt_meta_mod());

        let mut s = InputState::new();
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        s.press(InputKey::Meta);
        assert!(s.is_ctrl_alt_meta_mod());
    }

    #[test]
    fn clear_key_states() {
        let mut s = InputState::new();
        assert!(!s.clear_key_states());
        s.press(InputKey::Shift);
        s.press(InputKey::Key('A'));
        assert!(s.clear_key_states());
        assert!(s.is_no_mod());
        assert!(!s.is_pressed(InputKey::Key('A')));
    }

    #[test]
    fn key_classifier_mouse() {
        assert!(InputKey::MouseLeft.is_mouse());
        assert!(InputKey::WheelUp.is_mouse());
        assert!(InputKey::MouseX2.is_mouse());
        assert!(!InputKey::Shift.is_mouse());
        assert!(!InputKey::Touch.is_mouse());
    }

    #[test]
    fn key_classifier_touch() {
        assert!(InputKey::Touch.is_touch());
        assert!(!InputKey::MouseLeft.is_touch());
        assert!(!InputKey::Key('A').is_touch());
    }

    #[test]
    fn key_classifier_keyboard() {
        assert!(InputKey::Key('A').is_keyboard());
        assert!(InputKey::Shift.is_keyboard());
        assert!(InputKey::F1.is_keyboard());
        assert!(InputKey::Print.is_keyboard());
        assert!(InputKey::AltGr.is_keyboard());
        assert!(!InputKey::MouseLeft.is_keyboard());
        assert!(!InputKey::Touch.is_keyboard());
    }

    #[test]
    fn key_classifier_modifier() {
        assert!(InputKey::Shift.is_modifier());
        assert!(InputKey::Ctrl.is_modifier());
        assert!(InputKey::Alt.is_modifier());
        assert!(InputKey::Meta.is_modifier());
        assert!(!InputKey::AltGr.is_modifier());
        assert!(!InputKey::Key('A').is_modifier());
    }

    #[test]
    fn key_string_roundtrip() {
        let keys = [
            InputKey::MouseLeft,
            InputKey::Shift,
            InputKey::ArrowUp,
            InputKey::F12,
            InputKey::Print,
            InputKey::Pause,
            InputKey::Menu,
            InputKey::AltGr,
            InputKey::Touch,
            InputKey::Key('A'),
            InputKey::Key('0'),
            InputKey::Space,
        ];
        for key in keys {
            let s = key.as_str();
            let parsed = InputKey::from_str_name(s);
            assert_eq!(parsed, Some(key), "roundtrip failed for {s}");
        }
    }
}
