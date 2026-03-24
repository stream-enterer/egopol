use std::fmt;

/// Mouse cursor appearance.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum emCursor {
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

impl emCursor {
    // DIVERGED: Get — not applicable; Rust enum variant is the identity (C++ returns int id)
    // DIVERGED: ToString — renamed to `as_str`; `ToString` conflicts with Rust std::string::ToString trait
    /// Display name for this cursor type.
    pub fn emInputKeyToString(self) -> &'static str {
        match self {
            emCursor::Normal => "Normal",
            emCursor::Invisible => "Invisible",
            emCursor::Wait => "Wait",
            emCursor::Crosshair => "Crosshair",
            emCursor::Text => "Text",
            emCursor::Hand => "Hand",
            emCursor::ArrowN => "ArrowN",
            emCursor::ArrowS => "ArrowS",
            emCursor::ArrowE => "ArrowE",
            emCursor::ArrowW => "ArrowW",
            emCursor::ArrowNE => "ArrowNE",
            emCursor::ArrowNW => "ArrowNW",
            emCursor::ArrowSE => "ArrowSE",
            emCursor::ArrowSW => "ArrowSW",
            emCursor::ResizeNS => "ResizeNS",
            emCursor::ResizeEW => "ResizeEW",
            emCursor::ResizeNESW => "ResizeNESW",
            emCursor::ResizeNWSE => "ResizeNWSE",
            emCursor::Move => "Move",
        }
    }
}

impl fmt::Display for emCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.emInputKeyToString())
    }
}
