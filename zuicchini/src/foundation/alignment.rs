/// Nine-position content alignment.
///
/// Matches C++ emAlignment (EM_ALIGN_TOP_LEFT, etc.) for positioning
/// content within a rectangular area.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContentAlignment {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl ContentAlignment {
    /// Convert to a static string representation.
    ///
    /// Matches C++ emAlignmentToString.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "TopLeft",
            Self::Top => "Top",
            Self::TopRight => "TopRight",
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
            Self::BottomLeft => "BottomLeft",
            Self::Bottom => "Bottom",
            Self::BottomRight => "BottomRight",
        }
    }

    /// Parse from a string. Case-insensitive.
    ///
    /// Matches C++ emStringToAlignment.
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "topleft" | "top_left" | "top-left" => Some(Self::TopLeft),
            "top" => Some(Self::Top),
            "topright" | "top_right" | "top-right" => Some(Self::TopRight),
            "left" => Some(Self::Left),
            "center" => Some(Self::Center),
            "right" => Some(Self::Right),
            "bottomleft" | "bottom_left" | "bottom-left" => Some(Self::BottomLeft),
            "bottom" => Some(Self::Bottom),
            "bottomright" | "bottom_right" | "bottom-right" => Some(Self::BottomRight),
            _ => None,
        }
    }

    /// Horizontal factor: 0.0 for left, 0.5 for center, 1.0 for right.
    pub fn h_factor(self) -> f64 {
        match self {
            Self::TopLeft | Self::Left | Self::BottomLeft => 0.0,
            Self::Top | Self::Center | Self::Bottom => 0.5,
            Self::TopRight | Self::Right | Self::BottomRight => 1.0,
        }
    }

    /// Vertical factor: 0.0 for top, 0.5 for center, 1.0 for bottom.
    pub fn v_factor(self) -> f64 {
        match self {
            Self::TopLeft | Self::Top | Self::TopRight => 0.0,
            Self::Left | Self::Center | Self::Right => 0.5,
            Self::BottomLeft | Self::Bottom | Self::BottomRight => 1.0,
        }
    }
}

impl std::fmt::Display for ContentAlignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_variants() {
        let variants = [
            ContentAlignment::TopLeft,
            ContentAlignment::Top,
            ContentAlignment::TopRight,
            ContentAlignment::Left,
            ContentAlignment::Center,
            ContentAlignment::Right,
            ContentAlignment::BottomLeft,
            ContentAlignment::Bottom,
            ContentAlignment::BottomRight,
        ];
        for v in &variants {
            let s = v.as_str();
            let parsed = ContentAlignment::from_str_opt(s).expect(s);
            assert_eq!(*v, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn case_insensitive_parse() {
        assert_eq!(
            ContentAlignment::from_str_opt("TOPLEFT"),
            Some(ContentAlignment::TopLeft)
        );
        assert_eq!(
            ContentAlignment::from_str_opt("center"),
            Some(ContentAlignment::Center)
        );
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(ContentAlignment::from_str_opt("diagonal"), None);
    }

    #[test]
    fn factors() {
        assert!((ContentAlignment::TopLeft.h_factor() - 0.0).abs() < f64::EPSILON);
        assert!((ContentAlignment::Center.h_factor() - 0.5).abs() < f64::EPSILON);
        assert!((ContentAlignment::BottomRight.v_factor() - 1.0).abs() < f64::EPSILON);
    }
}
