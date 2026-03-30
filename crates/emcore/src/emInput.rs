/// Input key identifiers for keyboard and mouse.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InputKey {
    // Mouse buttons
    MouseLeft,
    MouseRight,
    MouseMiddle,
    MouseX1,
    MouseX2,

    // Mouse wheel
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,

    // emScreen touch
    Touch,

    // Modifier keys
    Shift,
    Ctrl,
    Alt,
    Meta,
    AltGr,

    // Navigation
    Escape,
    Tab,
    Enter,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // System keys
    Print,
    Pause,
    Menu,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Printable keys
    Space,
    Key(char),
}

impl InputKey {
    /// True for mouse buttons and mouse wheel keys.
    pub(crate) fn emIsMouseInputKey(self) -> bool {
        matches!(
            self,
            InputKey::MouseLeft
                | InputKey::MouseRight
                | InputKey::MouseMiddle
                | InputKey::MouseX1
                | InputKey::MouseX2
                | InputKey::WheelUp
                | InputKey::WheelDown
                | InputKey::WheelLeft
                | InputKey::WheelRight
        )
    }

    /// True for the screen touch key.
    pub(crate) fn emIsTouchInputKey(self) -> bool {
        self == InputKey::Touch
    }

    /// True for all keyboard keys, including modifiers.
    pub(crate) fn emIsKeyboardInputKey(self) -> bool {
        !self.emIsMouseInputKey() && !self.emIsTouchInputKey()
    }

    /// True for modifier keys (Shift, Ctrl, Alt, Meta).
    pub fn emIsModifierInputKey(self) -> bool {
        matches!(
            self,
            InputKey::Shift | InputKey::Ctrl | InputKey::Alt | InputKey::Meta
        )
    }

    /// Convert this key to its string name (C++ emInputKeyToString parity).
    pub fn emInputKeyToString(self) -> &'static str {
        match self {
            InputKey::MouseLeft => "LeftButton",
            InputKey::MouseMiddle => "MiddleButton",
            InputKey::MouseRight => "RightButton",
            InputKey::WheelUp => "WheelUp",
            InputKey::WheelDown => "WheelDown",
            InputKey::WheelLeft => "WheelLeft",
            InputKey::WheelRight => "WheelRight",
            InputKey::MouseX1 => "BackButton",
            InputKey::MouseX2 => "ForwardButton",
            InputKey::Touch => "Touch",
            InputKey::Shift => "Shift",
            InputKey::Ctrl => "Ctrl",
            InputKey::Alt => "Alt",
            InputKey::Meta => "Meta",
            InputKey::AltGr => "AltGr",
            InputKey::ArrowUp => "CursorUp",
            InputKey::ArrowDown => "CursorDown",
            InputKey::ArrowLeft => "CursorLeft",
            InputKey::ArrowRight => "CursorRight",
            InputKey::PageUp => "PageUp",
            InputKey::PageDown => "PageDown",
            InputKey::Home => "Home",
            InputKey::End => "End",
            InputKey::Print => "Print",
            InputKey::Pause => "Pause",
            InputKey::Menu => "Menu",
            InputKey::Insert => "Insert",
            InputKey::Delete => "Delete",
            InputKey::Backspace => "Backspace",
            InputKey::Tab => "Tab",
            InputKey::Enter => "Enter",
            InputKey::Escape => "Escape",
            InputKey::Space => "Space",
            InputKey::Key('0') => "0",
            InputKey::Key('1') => "1",
            InputKey::Key('2') => "2",
            InputKey::Key('3') => "3",
            InputKey::Key('4') => "4",
            InputKey::Key('5') => "5",
            InputKey::Key('6') => "6",
            InputKey::Key('7') => "7",
            InputKey::Key('8') => "8",
            InputKey::Key('9') => "9",
            InputKey::Key(c) if c.is_ascii_uppercase() => match c {
                'A' => "A",
                'B' => "B",
                'C' => "C",
                'D' => "D",
                'E' => "E",
                'F' => "F",
                'G' => "G",
                'H' => "H",
                'I' => "I",
                'J' => "J",
                'K' => "K",
                'L' => "L",
                'M' => "M",
                'N' => "N",
                'O' => "O",
                'P' => "P",
                'Q' => "Q",
                'R' => "R",
                'S' => "S",
                'T' => "T",
                'U' => "U",
                'V' => "V",
                'W' => "W",
                'X' => "X",
                'Y' => "Y",
                'Z' => "Z",
                _ => "?",
            },
            InputKey::F1 => "F1",
            InputKey::F2 => "F2",
            InputKey::F3 => "F3",
            InputKey::F4 => "F4",
            InputKey::F5 => "F5",
            InputKey::F6 => "F6",
            InputKey::F7 => "F7",
            InputKey::F8 => "F8",
            InputKey::F9 => "F9",
            InputKey::F10 => "F10",
            InputKey::F11 => "F11",
            InputKey::F12 => "F12",
            InputKey::Key(_) => "?",
        }
    }

    /// Parse a key from its string name (C++ emStringToInputKey parity).
    pub fn emStringToInputKey(s: &str) -> Option<InputKey> {
        match s {
            "LeftButton" => Some(InputKey::MouseLeft),
            "MiddleButton" => Some(InputKey::MouseMiddle),
            "RightButton" => Some(InputKey::MouseRight),
            "WheelUp" => Some(InputKey::WheelUp),
            "WheelDown" => Some(InputKey::WheelDown),
            "WheelLeft" => Some(InputKey::WheelLeft),
            "WheelRight" => Some(InputKey::WheelRight),
            "BackButton" => Some(InputKey::MouseX1),
            "ForwardButton" => Some(InputKey::MouseX2),
            "Touch" => Some(InputKey::Touch),
            "Shift" => Some(InputKey::Shift),
            "Ctrl" => Some(InputKey::Ctrl),
            "Alt" => Some(InputKey::Alt),
            "Meta" => Some(InputKey::Meta),
            "AltGr" => Some(InputKey::AltGr),
            "CursorUp" => Some(InputKey::ArrowUp),
            "CursorDown" => Some(InputKey::ArrowDown),
            "CursorLeft" => Some(InputKey::ArrowLeft),
            "CursorRight" => Some(InputKey::ArrowRight),
            "PageUp" => Some(InputKey::PageUp),
            "PageDown" => Some(InputKey::PageDown),
            "Home" => Some(InputKey::Home),
            "End" => Some(InputKey::End),
            "Print" => Some(InputKey::Print),
            "Pause" => Some(InputKey::Pause),
            "Menu" => Some(InputKey::Menu),
            "Insert" => Some(InputKey::Insert),
            "Delete" => Some(InputKey::Delete),
            "Backspace" => Some(InputKey::Backspace),
            "Tab" => Some(InputKey::Tab),
            "Enter" => Some(InputKey::Enter),
            "Escape" => Some(InputKey::Escape),
            "Space" => Some(InputKey::Space),
            "F1" => Some(InputKey::F1),
            "F2" => Some(InputKey::F2),
            "F3" => Some(InputKey::F3),
            "F4" => Some(InputKey::F4),
            "F5" => Some(InputKey::F5),
            "F6" => Some(InputKey::F6),
            "F7" => Some(InputKey::F7),
            "F8" => Some(InputKey::F8),
            "F9" => Some(InputKey::F9),
            "F10" => Some(InputKey::F10),
            "F11" => Some(InputKey::F11),
            "F12" => Some(InputKey::F12),
            s if s.len() == 1 => {
                let c = s.chars().next()?;
                if c.is_ascii_digit() || c.is_ascii_uppercase() {
                    Some(InputKey::Key(c))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Variant of an input event.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum InputVariant {
    /// Key or button pressed down.
    Press,
    /// Key or button released.
    Release,
    /// Repeated key press (held down).
    Repeat,
    /// Mouse moved (no button state change).
    Move,
}

/// An input event.
#[derive(Clone, Debug)]
pub struct emInputEvent {
    /// Which key or button.
    pub key: InputKey,
    /// Event variant (press, release, repeat).
    pub variant: InputVariant,
    /// Characters generated by this event (for text input).
    pub chars: String,
    /// Number of event repetitions. 0 = first press, 1 = double-click, etc.
    /// (C++ emInputEvent::GetRepeat parity.)
    pub repeat: i32,
    /// Source variant info: 1 for numpad / right-side modifier keys, 0 otherwise.
    /// (C++ emInputEvent::GetVariant parity.)
    pub source_variant: i32,
    /// Mouse X position in panel coordinates.
    pub mouse_x: f64,
    /// Mouse Y position in panel coordinates.
    pub mouse_y: f64,
    /// Modifier key states at the time of the event.
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
    /// Whether this event has been eaten (consumed by a handler).
    /// (C++ emInputEvent::Eat parity.)
    pub(crate) eaten: bool,
}

impl emInputEvent {
    pub fn press(key: InputKey) -> Self {
        Self {
            key,
            variant: InputVariant::Press,
            chars: String::new(),
            repeat: 0,
            source_variant: 0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            eaten: false,
        }
    }

    pub fn release(key: InputKey) -> Self {
        Self {
            key,
            variant: InputVariant::Release,
            chars: String::new(),
            repeat: 0,
            source_variant: 0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            eaten: false,
        }
    }

    /// Whether this is a repeated event (repeat count > 0).
    pub fn is_repeat(&self) -> bool {
        self.repeat > 0
    }

    /// Whether this event is empty (eaten or has no meaningful content).
    /// (C++ emInputEvent::IsEmpty parity.)
    pub fn IsEmpty(&self) -> bool {
        self.eaten
    }

    /// Eat this event, marking it as consumed so other handlers skip it.
    /// (C++ emInputEvent::Eat parity.)
    pub fn eat(&mut self) {
        self.eaten = true;
        self.chars.clear();
    }

    /// Whether this event matches the given key.
    /// (C++ emInputEvent::IsKey parity.)
    pub fn is_key(&self, key: InputKey) -> bool {
        !self.eaten && self.key == key
    }

    /// Whether this is a left mouse button event.
    /// (C++ emInputEvent::IsLeftButton parity.)
    pub fn is_left_button(&self) -> bool {
        self.is_key(InputKey::MouseLeft)
    }

    /// Whether this is a middle mouse button event.
    /// (C++ emInputEvent::IsMiddleButton parity.)
    pub fn is_middle_button(&self) -> bool {
        self.is_key(InputKey::MouseMiddle)
    }

    /// Whether this is a right mouse button event.
    /// (C++ emInputEvent::IsRightButton parity.)
    pub fn is_right_button(&self) -> bool {
        self.is_key(InputKey::MouseRight)
    }

    /// Whether this is a mouse event.
    pub fn is_mouse_event(&self) -> bool {
        !self.eaten && self.key.emIsMouseInputKey()
    }

    /// Whether this is a touch event.
    pub fn is_touch_event(&self) -> bool {
        !self.eaten && self.key.emIsTouchInputKey()
    }

    /// Whether this is a keyboard event (key is keyboard, or chars non-empty).
    pub fn is_keyboard_event(&self) -> bool {
        !self.eaten && (self.key.emIsKeyboardInputKey() || !self.chars.is_empty())
    }

    pub fn with_chars(mut self, chars: &str) -> Self {
        self.chars = chars.to_string();
        self
    }

    pub fn with_mouse(mut self, x: f64, y: f64) -> Self {
        self.mouse_x = x;
        self.mouse_y = y;
        self
    }

    pub fn with_modifiers(mut self, state: &super::emInputState::emInputState) -> Self {
        self.shift = state.GetShift();
        self.ctrl = state.GetCtrl();
        self.alt = state.GetAlt();
        self.meta = state.GetMeta();
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift_ctrl(mut self) -> Self {
        self.shift = true;
        self.ctrl = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn with_repeat(mut self, repeat: i32) -> Self {
        self.repeat = repeat;
        self
    }

    pub fn with_variant(mut self, variant: InputVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Create a move event (no button press/release, just position update).
    pub fn mouse_move(key: InputKey, x: f64, y: f64) -> Self {
        Self {
            key,
            variant: InputVariant::Move,
            chars: String::new(),
            repeat: 0,
            source_variant: 0,
            mouse_x: x,
            mouse_y: y,
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            eaten: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eat_marks_empty() {
        let mut ev = emInputEvent::press(InputKey::Key('A')).with_chars("a");
        assert!(!ev.IsEmpty());
        assert!(ev.is_key(InputKey::Key('A')));
        ev.eat();
        assert!(ev.IsEmpty());
        assert!(!ev.is_key(InputKey::Key('A')));
        assert!(ev.chars.is_empty());
    }

    #[test]
    fn source_variant_default() {
        let ev = emInputEvent::press(InputKey::Enter);
        assert_eq!(ev.source_variant, 0);
    }

    #[test]
    fn is_button_helpers() {
        let left = emInputEvent::press(InputKey::MouseLeft);
        assert!(left.is_left_button());
        assert!(!left.is_middle_button());
        assert!(!left.is_right_button());

        let mid = emInputEvent::press(InputKey::MouseMiddle);
        assert!(mid.is_middle_button());

        let right = emInputEvent::press(InputKey::MouseRight);
        assert!(right.is_right_button());
    }

    #[test]
    fn event_type_queries() {
        let mouse = emInputEvent::press(InputKey::MouseLeft);
        assert!(mouse.is_mouse_event());
        assert!(!mouse.is_keyboard_event());
        assert!(!mouse.is_touch_event());

        let key = emInputEvent::press(InputKey::Key('A'));
        assert!(key.is_keyboard_event());
        assert!(!key.is_mouse_event());

        let touch = emInputEvent::press(InputKey::Touch);
        assert!(touch.is_touch_event());
    }

    #[test]
    fn eaten_event_not_typed() {
        let mut ev = emInputEvent::press(InputKey::MouseLeft);
        assert!(ev.is_mouse_event());
        ev.eat();
        assert!(!ev.is_mouse_event());
        assert!(!ev.is_keyboard_event());
        assert!(!ev.is_touch_event());
    }

    #[test]
    fn mouse_move_constructor() {
        let ev = emInputEvent::mouse_move(InputKey::MouseLeft, 10.0, 20.0);
        assert_eq!(ev.variant, InputVariant::Move);
        assert_eq!(ev.mouse_x, 10.0);
        assert_eq!(ev.mouse_y, 20.0);
    }
}
