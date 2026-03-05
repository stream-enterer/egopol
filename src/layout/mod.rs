pub mod linear;
pub mod pack;
pub mod raster;

use crate::panel::PanelId;

/// Axis orientation for layout algorithms.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
    /// Switches between horizontal and vertical based on the container's
    /// tallness (height / width). If tallness <= threshold, use horizontal
    /// (wide panel → children side by side); if tallness > threshold, use
    /// vertical (tall panel → children stacked). Matches C++
    /// `horizontal = (h/w <= OrientationThreshold)`.
    Adaptive {
        tallness_threshold: f64,
    },
}

impl Orientation {
    /// Resolve to a concrete horizontal or vertical based on container rect.
    pub fn resolve(self, w: f64, h: f64) -> ResolvedOrientation {
        match self {
            Self::Horizontal => ResolvedOrientation::Horizontal,
            Self::Vertical => ResolvedOrientation::Vertical,
            Self::Adaptive { tallness_threshold } => {
                let tallness = if w > 0.0 { h / w } else { 1.0 };
                if tallness <= tallness_threshold {
                    ResolvedOrientation::Horizontal
                } else {
                    ResolvedOrientation::Vertical
                }
            }
        }
    }
}

/// A resolved (non-adaptive) orientation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResolvedOrientation {
    Horizontal,
    Vertical,
}

/// Cross-axis alignment for children within a layout.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

/// Spacing configuration for layouts.
///
/// Matches C++ spacing model with separate horizontal/vertical inner spacing.
/// In horizontal layout, `inner_h` goes between children and `margin_top`/
/// `margin_bottom` above/below. In vertical layout, `inner_v` goes between
/// children and `margin_left`/`margin_right` beside.
#[derive(Clone, Debug)]
pub struct Spacing {
    /// Space between children when laid out horizontally.
    pub inner_h: f64,
    /// Space between children when laid out vertically.
    pub inner_v: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            inner_h: 0.0,
            inner_v: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
        }
    }
}

impl Spacing {
    pub fn uniform(margin: f64, inner: f64) -> Self {
        Self {
            inner_h: inner,
            inner_v: inner,
            margin_left: margin,
            margin_right: margin,
            margin_top: margin,
            margin_bottom: margin,
        }
    }

    /// Get inner spacing for a resolved orientation.
    pub fn inner_for(&self, orientation: ResolvedOrientation) -> f64 {
        match orientation {
            ResolvedOrientation::Horizontal => self.inner_h,
            ResolvedOrientation::Vertical => self.inner_v,
        }
    }
}

/// Per-child constraint used by LinearLayout and PackLayout.
#[derive(Clone, Debug)]
pub struct ChildConstraint {
    /// Relative weight for distributing space on the main axis.
    pub weight: f64,
    /// Minimum size on the main axis.
    pub min_main: f64,
    /// Maximum size on the main axis (f64::INFINITY = unconstrained).
    pub max_main: f64,
    /// Preferred tallness (height / width) for layout scoring.
    pub preferred_tallness: f64,
    /// Minimum tallness (height / width) constraint.
    pub min_tallness: f64,
    /// Maximum tallness (height / width) constraint (f64::INFINITY = unconstrained).
    pub max_tallness: f64,
}

impl Default for ChildConstraint {
    fn default() -> Self {
        Self {
            weight: 1.0,
            min_main: 0.0,
            max_main: f64::INFINITY,
            preferred_tallness: 1.0,
            min_tallness: 1e-4,
            max_tallness: 1e4,
        }
    }
}

/// Helper: get constraint for a child, falling back to default.
pub(crate) fn get_constraint<'a>(
    constraints: &'a std::collections::HashMap<PanelId, ChildConstraint>,
    child: PanelId,
    default: &'a ChildConstraint,
) -> &'a ChildConstraint {
    constraints.get(&child).unwrap_or(default)
}
