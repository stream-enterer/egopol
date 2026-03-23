use crate::emCore::emColor::emColor;


/// emStroke end type matching Eagle Mode's 17 `emStrokeEnd` variants.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StrokeEndType {
    Butt,
    Cap,
    Arrow,
    ContourArrow,
    LineArrow,
    Triangle,
    ContourTriangle,
    Square,
    ContourSquare,
    HalfSquare,
    Circle,
    ContourCircle,
    HalfCircle,
    Diamond,
    ContourDiamond,
    HalfDiamond,
    emStroke,
}

/// emStroke end decoration with configurable color and size factors.
#[derive(Copy, Clone, Debug)]
pub struct emStrokeEnd {
    /// The type of end decoration.
    pub end_type: StrokeEndType,
    /// Fill color for Contour* variants.
    pub inner_color: emColor,
    /// Multiplier on decoration width (default 1.0).
    pub width_factor: f64,
    /// Multiplier on decoration length (default 1.0).
    pub length_factor: f64,
}

impl emStrokeEnd {
    /// Create a butt (no decoration) stroke end.
    pub fn butt() -> Self {
        Self {
            end_type: StrokeEndType::Butt,
            inner_color: emColor::TRANSPARENT,
            width_factor: 1.0,
            length_factor: 1.0,
        }
    }

    /// Create a stroke end with the given type and default factors.
    pub fn new(end_type: StrokeEndType) -> Self {
        Self {
            end_type,
            inner_color: emColor::TRANSPARENT,
            width_factor: 1.0,
            length_factor: 1.0,
        }
    }

    /// Set the inner color (for Contour* variants).
    pub fn with_inner_color(mut self, color: emColor) -> Self {
        self.inner_color = color;
        self
    }

    /// Set the width factor.
    pub fn with_width_factor(mut self, factor: f64) -> Self {
        self.width_factor = factor;
        self
    }

    /// Set the length factor.
    pub fn with_length_factor(mut self, factor: f64) -> Self {
        self.length_factor = factor;
        self
    }

    /// Whether this end type draws a decoration (everything except Butt and Cap).
    /// Matches C++ `emStrokeEnd::IsDecorated()` which returns `Type >= ARROW`.
    pub fn IsDecorated(&self) -> bool {
        !matches!(self.end_type, StrokeEndType::Butt | StrokeEndType::Cap)
    }
}

