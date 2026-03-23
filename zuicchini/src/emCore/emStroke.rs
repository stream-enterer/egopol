use crate::emCore::emColor::emColor;
pub use super::emStrokeEnd::{emStrokeEnd, StrokeEndType};

/// Line join style.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

/// Line cap style.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// Dash pattern type matching C++ `emStroke::DashTypeEnum`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DashType {
    /// Solid line (no dashes).
    Solid,
    /// Dashes only.
    Dashed,
    /// Dots only.
    Dotted,
    /// Alternating dashes and dots.
    DashDotted,
}



/// emStroke properties for outlined shapes.
#[derive(Clone, Debug)]
pub struct emStroke {
    /// emStroke color.
    pub color: emColor,
    /// emStroke width in pixels.
    pub width: f64,
    /// Line join style.
    pub join: LineJoin,
    /// Line cap style.
    pub cap: LineCap,
    /// Start end style.
    pub start_end: emStrokeEnd,
    /// Finish end style.
    pub finish_end: emStrokeEnd,
    /// Dash pattern: alternating on/off lengths. Empty = solid line.
    /// This is the legacy API; prefer `dash_type` + factors for C++ parity.
    pub dash_pattern: Vec<f64>,
    /// Dash offset (legacy pattern API).
    pub dash_offset: f64,
    /// Dash type (C++ parity API). Overrides `dash_pattern` when not `Solid`.
    pub dash_type: DashType,
    /// Dash length factor (C++ `DashLengthFactor`). Default 1.0.
    pub dash_length_factor: f64,
    /// Gap length factor (C++ `GapLengthFactor`). Default 1.0.
    pub gap_length_factor: f64,
}

impl Default for emStroke {
    fn default() -> Self {
        Self {
            color: emColor::BLACK,
            width: 1.0,
            join: LineJoin::Miter,
            cap: LineCap::Butt,
            start_end: emStrokeEnd::butt(),
            finish_end: emStrokeEnd::butt(),
            dash_pattern: Vec::new(),
            dash_offset: 0.0,
            dash_type: DashType::Solid,
            dash_length_factor: 1.0,
            gap_length_factor: 1.0,
        }
    }
}

impl emStroke {
    /// Create a simple solid stroke with the given color and width.
    pub fn new(color: emColor, width: f64) -> Self {
        Self {
            color,
            width,
            ..Default::default()
        }
    }

    /// Whether this stroke uses any dash pattern (via either API).
    pub fn is_dashed(&self) -> bool {
        self.dash_type != DashType::Solid || !self.dash_pattern.is_empty()
    }
}
