// SPLIT: Split from emInput.h — hotkey types extracted
use std::fmt;

use crate::emCore::emInput::InputKey;
use crate::emCore::emInputState::emInputState;

/// A hotkey is a modifier+key combination.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: InputKey,
}

impl Hotkey {
    /// Create a hotkey with just a key, no modifiers.
    pub fn new(key: InputKey) -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            key,
        }
    }

    /// Parse a hotkey from a string like "Ctrl+Shift+C" or "Alt+F4".
    pub fn TryParse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
        if parts.is_empty() {
            return None;
        }

        let mut hotkey = Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            key: InputKey::Space, // placeholder
        };

        let mut found_key = false;
        for (i, part) in parts.iter().enumerate() {
            let lower = part.to_lowercase();
            if i < parts.len() - 1 {
                // Modifier
                match lower.as_str() {
                    "ctrl" | "control" => hotkey.ctrl = true,
                    "alt" => hotkey.alt = true,
                    "shift" => hotkey.shift = true,
                    "meta" | "cmd" | "win" | "super" => hotkey.meta = true,
                    _ => return None,
                }
            } else {
                // Key (last part)
                hotkey.key = parse_key_name(&lower)?;
                found_key = true;
            }
        }

        if found_key {
            Some(hotkey)
        } else {
            None
        }
    }

    /// Check if this hotkey matches the current input state plus a just-pressed key.
    pub fn Match(&self, key: InputKey, state: &emInputState) -> bool {
        self.key == key
            && self.ctrl == state.GetCtrl()
            && self.alt == state.GetAlt()
            && self.shift == state.GetShift()
            && self.meta == state.GetMeta()
    }

    /// Clear all modifier keys.
    pub fn ClearModifiers(&mut self) {
        self.ctrl = false;
        self.alt = false;
        self.shift = false;
        self.meta = false;
    }

    /// Add a modifier key. Non-modifier keys are ignored.
    pub fn AddModifier(&mut self, key: InputKey) {
        match key {
            InputKey::Ctrl => self.ctrl = true,
            InputKey::Alt => self.alt = true,
            InputKey::Shift => self.shift = true,
            InputKey::Meta => self.meta = true,
            _ => {}
        }
    }

    /// Set the main (non-modifier) key.
    pub fn SetKey(&mut self, key: InputKey) {
        self.key = key;
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.meta {
            parts.push("Meta");
        }
        parts.push(self.key.emInputKeyToString());
        write!(f, "{}", parts.join("+"))
    }
}

fn parse_key_name(name: &str) -> Option<InputKey> {
    match name {
        "escape" | "esc" => Some(InputKey::Escape),
        "tab" => Some(InputKey::Tab),
        "enter" | "return" => Some(InputKey::Enter),
        "backspace" => Some(InputKey::Backspace),
        "delete" | "del" => Some(InputKey::Delete),
        "insert" | "ins" => Some(InputKey::Insert),
        "home" => Some(InputKey::Home),
        "end" => Some(InputKey::End),
        "pageup" | "pgup" => Some(InputKey::PageUp),
        "pagedown" | "pgdn" => Some(InputKey::PageDown),
        "up" | "arrowup" => Some(InputKey::ArrowUp),
        "down" | "arrowdown" => Some(InputKey::ArrowDown),
        "left" | "arrowleft" => Some(InputKey::ArrowLeft),
        "right" | "arrowright" => Some(InputKey::ArrowRight),
        "space" => Some(InputKey::Space),
        "f1" => Some(InputKey::F1),
        "f2" => Some(InputKey::F2),
        "f3" => Some(InputKey::F3),
        "f4" => Some(InputKey::F4),
        "f5" => Some(InputKey::F5),
        "f6" => Some(InputKey::F6),
        "f7" => Some(InputKey::F7),
        "f8" => Some(InputKey::F8),
        "f9" => Some(InputKey::F9),
        "f10" => Some(InputKey::F10),
        "f11" => Some(InputKey::F11),
        "f12" => Some(InputKey::F12),
        "print" | "printscreen" => Some(InputKey::Print),
        "pause" | "break" => Some(InputKey::Pause),
        "menu" | "contextmenu" => Some(InputKey::Menu),
        s if s.len() == 1 => Some(InputKey::Key(s.chars().next().unwrap())),
        _ => None,
    }
}
