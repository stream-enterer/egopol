// SPLIT: Split from emInput.h — input state types extracted
use std::collections::HashSet;

use crate::emCore::emInput::InputKey;

/// Tracks the current state of all input devices.
/// (C++ emInputState operator== / operator!= parity via PartialEq.)
#[derive(Clone, Debug, PartialEq)]
pub struct emInputState {
    /// Currently pressed keys.
    pressed: HashSet<InputKey>,
    /// Current mouse position in window coordinates.
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// Current touch points (id, x, y).
    touches: Vec<(u64, f64, f64)>,
}

impl emInputState {
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
    pub fn Get(&self, key: InputKey) -> bool {
        self.pressed.contains(&key)
    }

    /// Check if Shift is held.
    pub fn GetShift(&self) -> bool {
        self.Get(InputKey::Shift)
    }

    /// Check if Ctrl is held.
    pub fn GetCtrl(&self) -> bool {
        self.Get(InputKey::Ctrl)
    }

    /// Check if Alt is held.
    pub fn GetAlt(&self) -> bool {
        self.Get(InputKey::Alt)
    }

    /// Check if Meta (Cmd/Win) is held.
    pub fn GetMeta(&self) -> bool {
        self.Get(InputKey::Meta)
    }

    /// Whether the left mouse button is pressed.
    /// (C++ emInputState::GetLeftButton parity.)
    pub fn GetLeftButton(&self) -> bool {
        self.Get(InputKey::MouseLeft)
    }

    /// Whether the middle mouse button is pressed.
    /// (C++ emInputState::GetMiddleButton parity.)
    pub fn GetMiddleButton(&self) -> bool {
        self.Get(InputKey::MouseMiddle)
    }

    /// Whether the right mouse button is pressed.
    /// (C++ emInputState::GetRightButton parity.)
    pub fn GetRightButton(&self) -> bool {
        self.Get(InputKey::MouseRight)
    }

    // ── Modifier combo tests (C++ emInputState parity) ────────────────
    // Each tests that *exactly* the named modifiers are held, nothing more.

    /// No modifier keys held.
    pub fn IsNoMod(&self) -> bool {
        !self.GetShift() && !self.GetCtrl() && !self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Shift held.
    pub fn IsShiftMod(&self) -> bool {
        self.GetShift() && !self.GetCtrl() && !self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Ctrl held.
    pub fn IsCtrlMod(&self) -> bool {
        !self.GetShift() && self.GetCtrl() && !self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Alt held.
    pub fn IsAltMod(&self) -> bool {
        !self.GetShift() && !self.GetCtrl() && self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Meta held.
    pub fn IsMetaMod(&self) -> bool {
        !self.GetShift() && !self.GetCtrl() && !self.GetAlt() && self.GetMeta()
    }

    /// Exactly Shift+Ctrl held.
    pub fn IsShiftCtrlMod(&self) -> bool {
        self.GetShift() && self.GetCtrl() && !self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Shift+Alt held.
    pub fn IsShiftAltMod(&self) -> bool {
        self.GetShift() && !self.GetCtrl() && self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Shift+Meta held.
    pub fn IsShiftMetaMod(&self) -> bool {
        self.GetShift() && !self.GetCtrl() && !self.GetAlt() && self.GetMeta()
    }

    /// Exactly Ctrl+Alt held.
    pub fn IsCtrlAltMod(&self) -> bool {
        !self.GetShift() && self.GetCtrl() && self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Ctrl+Meta held.
    pub fn IsCtrlMetaMod(&self) -> bool {
        !self.GetShift() && self.GetCtrl() && !self.GetAlt() && self.GetMeta()
    }

    /// Exactly Alt+Meta held.
    pub fn IsAltMetaMod(&self) -> bool {
        !self.GetShift() && !self.GetCtrl() && self.GetAlt() && self.GetMeta()
    }

    /// Exactly Shift+Ctrl+Alt held.
    pub fn IsShiftCtrlAltMod(&self) -> bool {
        self.GetShift() && self.GetCtrl() && self.GetAlt() && !self.GetMeta()
    }

    /// Exactly Shift+Ctrl+Meta held.
    pub fn IsShiftCtrlMetaMod(&self) -> bool {
        self.GetShift() && self.GetCtrl() && !self.GetAlt() && self.GetMeta()
    }

    /// Exactly Shift+Alt+Meta held.
    pub fn IsShiftAltMetaMod(&self) -> bool {
        self.GetShift() && !self.GetCtrl() && self.GetAlt() && self.GetMeta()
    }

    /// Exactly Ctrl+Alt+Meta held.
    pub fn IsCtrlAltMetaMod(&self) -> bool {
        !self.GetShift() && self.GetCtrl() && self.GetAlt() && self.GetMeta()
    }

    /// All four modifiers held.
    pub fn IsShiftCtrlAltMetaMod(&self) -> bool {
        self.GetShift() && self.GetCtrl() && self.GetAlt() && self.GetMeta()
    }

    /// Clear all pressed keys. Returns true if any key was pressed.
    pub fn ClearKeyStates(&mut self) -> bool {
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
    pub fn SetTouch(&mut self, id: u64, x: f64, y: f64) {
        if let Some(touch) = self.touches.iter_mut().find(|t| t.0 == id) {
            touch.1 = x;
            touch.2 = y;
        } else {
            self.touches.push((id, x, y));
        }
    }

    /// Remove a touch point.
    pub fn RemoveTouch(&mut self, id: u64) {
        self.touches.retain(|t| t.0 != id);
    }

    /// Get all active touch points.
    pub fn GetTouchCount(&self) -> &[(u64, f64, f64)] {
        &self.touches
    }

    /// Get the set of currently pressed keys.
    pub fn GetKeyStates(&self) -> &HashSet<InputKey> {
        &self.pressed
    }

    /// Clear all active touch points.
    pub fn ClearTouches(&mut self) {
        self.touches.clear();
    }
}

impl Default for emInputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_combo_no_mod() {
        let s = emInputState::new();
        assert!(s.IsNoMod());
        assert!(!s.IsShiftMod());
        assert!(!s.IsCtrlMod());
    }

    #[test]
    fn modifier_combo_single() {
        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        assert!(s.IsShiftMod());
        assert!(!s.IsNoMod());
        assert!(!s.IsCtrlMod());
        assert!(!s.IsShiftCtrlMod());
    }

    #[test]
    fn modifier_combo_double() {
        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        assert!(s.IsShiftCtrlMod());
        assert!(!s.IsShiftMod());
        assert!(!s.IsCtrlMod());

        let mut s2 = emInputState::new();
        s2.press(InputKey::Alt);
        s2.press(InputKey::Meta);
        assert!(s2.IsAltMetaMod());
    }

    #[test]
    fn modifier_combo_triple() {
        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        assert!(s.IsShiftCtrlAltMod());
        assert!(!s.IsShiftCtrlMod());
        assert!(!s.IsShiftCtrlAltMetaMod());
    }

    #[test]
    fn modifier_combo_all_four() {
        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        s.press(InputKey::Meta);
        assert!(s.IsShiftCtrlAltMetaMod());
        assert!(!s.IsShiftCtrlAltMod());
    }

    #[test]
    fn modifier_remaining_combos() {
        let mut s = emInputState::new();
        s.press(InputKey::Alt);
        assert!(s.IsAltMod());

        let mut s = emInputState::new();
        s.press(InputKey::Meta);
        assert!(s.IsMetaMod());

        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Alt);
        assert!(s.IsShiftAltMod());

        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Meta);
        assert!(s.IsShiftMetaMod());

        let mut s = emInputState::new();
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        assert!(s.IsCtrlAltMod());

        let mut s = emInputState::new();
        s.press(InputKey::Ctrl);
        s.press(InputKey::Meta);
        assert!(s.IsCtrlMetaMod());

        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Ctrl);
        s.press(InputKey::Meta);
        assert!(s.IsShiftCtrlMetaMod());

        let mut s = emInputState::new();
        s.press(InputKey::Shift);
        s.press(InputKey::Alt);
        s.press(InputKey::Meta);
        assert!(s.IsShiftAltMetaMod());

        let mut s = emInputState::new();
        s.press(InputKey::Ctrl);
        s.press(InputKey::Alt);
        s.press(InputKey::Meta);
        assert!(s.IsCtrlAltMetaMod());
    }

    #[test]
    fn ClearKeyStates() {
        let mut s = emInputState::new();
        assert!(!s.ClearKeyStates());
        s.press(InputKey::Shift);
        s.press(InputKey::Key('A'));
        assert!(s.ClearKeyStates());
        assert!(s.IsNoMod());
        assert!(!s.Get(InputKey::Key('A')));
    }

    #[test]
    fn key_classifier_mouse() {
        assert!(InputKey::MouseLeft.emIsMouseInputKey());
        assert!(InputKey::WheelUp.emIsMouseInputKey());
        assert!(InputKey::MouseX2.emIsMouseInputKey());
        assert!(!InputKey::Shift.emIsMouseInputKey());
        assert!(!InputKey::Touch.emIsMouseInputKey());
    }

    #[test]
    fn key_classifier_touch() {
        assert!(InputKey::Touch.emIsTouchInputKey());
        assert!(!InputKey::MouseLeft.emIsTouchInputKey());
        assert!(!InputKey::Key('A').emIsTouchInputKey());
    }

    #[test]
    fn key_classifier_keyboard() {
        assert!(InputKey::Key('A').emIsKeyboardInputKey());
        assert!(InputKey::Shift.emIsKeyboardInputKey());
        assert!(InputKey::F1.emIsKeyboardInputKey());
        assert!(InputKey::Print.emIsKeyboardInputKey());
        assert!(InputKey::AltGr.emIsKeyboardInputKey());
        assert!(!InputKey::MouseLeft.emIsKeyboardInputKey());
        assert!(!InputKey::Touch.emIsKeyboardInputKey());
    }

    #[test]
    fn key_classifier_modifier() {
        assert!(InputKey::Shift.emIsModifierInputKey());
        assert!(InputKey::Ctrl.emIsModifierInputKey());
        assert!(InputKey::Alt.emIsModifierInputKey());
        assert!(InputKey::Meta.emIsModifierInputKey());
        assert!(!InputKey::AltGr.emIsModifierInputKey());
        assert!(!InputKey::Key('A').emIsModifierInputKey());
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
            let s = key.emInputKeyToString();
            let parsed = InputKey::emStringToInputKey(s);
            assert_eq!(parsed, Some(key), "roundtrip failed for {s}");
        }
    }
}
