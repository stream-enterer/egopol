use std::fmt;

/// Mouse cursor appearance.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cursor {
    Normal,
    Invisible,
    Wait,
    Crosshair,
    Text,
    Hand,
    ArrowN,
    ArrowS,
    ArrowE,
    ArrowW,
    ArrowNE,
    ArrowNW,
    ArrowSE,
    ArrowSW,
    ResizeNS,
    ResizeEW,
    ResizeNESW,
    ResizeNWSE,
    Move,
}

impl Cursor {
    /// Display name for this cursor type.
    pub fn as_str(self) -> &'static str {
        match self {
            Cursor::Normal => "Normal",
            Cursor::Invisible => "Invisible",
            Cursor::Wait => "Wait",
            Cursor::Crosshair => "Crosshair",
            Cursor::Text => "Text",
            Cursor::Hand => "Hand",
            Cursor::ArrowN => "ArrowN",
            Cursor::ArrowS => "ArrowS",
            Cursor::ArrowE => "ArrowE",
            Cursor::ArrowW => "ArrowW",
            Cursor::ArrowNE => "ArrowNE",
            Cursor::ArrowNW => "ArrowNW",
            Cursor::ArrowSE => "ArrowSE",
            Cursor::ArrowSW => "ArrowSW",
            Cursor::ResizeNS => "ResizeNS",
            Cursor::ResizeEW => "ResizeEW",
            Cursor::ResizeNESW => "ResizeNESW",
            Cursor::ResizeNWSE => "ResizeNWSE",
            Cursor::Move => "Move",
        }
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
