use std::rc::Rc;

use crate::emCore::emColor::emColor;

/// Theme configuration matching emLook's 10-color system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct emLook {
    pub bg_color: emColor,
    pub fg_color: emColor,
    pub button_bg_color: emColor,
    pub button_fg_color: emColor,
    pub input_bg_color: emColor,
    pub input_fg_color: emColor,
    pub input_hl_color: emColor,
    pub output_bg_color: emColor,
    pub output_fg_color: emColor,
    pub output_hl_color: emColor,
}

impl emLook {
    /// Create a new look wrapped in `Rc` with the default theme.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// emBorder tint: bg_color darkened ~20%.
    pub fn border_tint(&self) -> emColor {
        self.bg_color.darken(0.20)
    }

    /// Focus tint: same as input highlight color.
    pub fn focus_tint(&self) -> emColor {
        self.input_hl_color
    }

    /// Disabled foreground: fg blended 50% toward bg.
    pub fn disabled_fg(&self) -> emColor {
        self.fg_color.GetBlended(self.bg_color, 0.5)
    }

    /// emButton hover: button_bg lightened ~15%.
    pub fn button_hover(&self) -> emColor {
        self.button_bg_color.lighten(0.15)
    }

    /// emButton pressed: button_bg darkened ~15%.
    pub fn button_pressed(&self) -> emColor {
        self.button_bg_color.darken(0.15)
    }

    /// Apply this look to a target look reference, optionally for recursive use.
    ///
    /// Port of C++ `emLook::Apply(emPanel*, bool recursively)`.
    /// In C++, Apply walks a panel tree and calls `emBorder::SetLook()` on each
    /// emBorder descendant. In Rust, widgets store `Rc<emLook>` directly, so
    /// `apply` replaces the target reference with a clone of this look.
    /// When `recursively` is true, the caller should propagate to child panels.
    pub fn apply(&self, target: &mut Rc<emLook>, _recursively: bool) {
        *target = Rc::new(self.clone());
    }

    /// Apply this look to multiple targets at once.
    ///
    /// Convenience for recursive application across a widget subtree.
    pub fn apply_all(&self, targets: &mut [&mut Rc<emLook>]) {
        let shared = Rc::new(self.clone());
        for t in targets {
            **t = shared.clone();
        }
    }
}

impl Default for emLook {
    fn default() -> Self {
        Self {
            bg_color: emColor::rgba(0x51, 0x5E, 0x84, 0xFF),
            fg_color: emColor::rgba(0xEF, 0xF0, 0xF4, 0xFF),
            button_bg_color: emColor::rgba(0x59, 0x67, 0x90, 0xFF),
            button_fg_color: emColor::rgba(0xF2, 0xF2, 0xF7, 0xFF),
            input_bg_color: emColor::rgba(0xEF, 0xF0, 0xF4, 0xFF),
            input_fg_color: emColor::rgba(0x02, 0x0E, 0x1D, 0xFF),
            input_hl_color: emColor::rgba(0x00, 0x38, 0xC0, 0xFF),
            output_bg_color: emColor::rgba(0xA7, 0xA9, 0xB0, 0xFF),
            output_fg_color: emColor::rgba(0x07, 0x0B, 0x18, 0xFF),
            output_hl_color: emColor::rgba(0x00, 0x2B, 0x9A, 0xFF),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_look_has_emlook_colors() {
        let look = emLook::default();
        assert_eq!(look.bg_color, emColor::rgba(0x51, 0x5E, 0x84, 0xFF));
        assert_eq!(look.fg_color, emColor::rgba(0xEF, 0xF0, 0xF4, 0xFF));
        assert_eq!(look.input_hl_color, emColor::rgba(0x00, 0x38, 0xC0, 0xFF));
    }

    #[test]
    fn partial_eq_same_defaults() {
        let a = emLook::default();
        let b = emLook::default();
        assert_eq!(a, b);
    }

    #[test]
    fn partial_eq_different_color() {
        let a = emLook::default();
        let b = emLook {
            bg_color: emColor::rgba(0xFF, 0x00, 0x00, 0xFF),
            ..emLook::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn derived_colors_are_reasonable() {
        let look = emLook::default();
        // border_tint should be darker than bg
        let bt = look.border_tint();
        assert!(bt.GetRed() < look.bg_color.GetRed());
        // button_hover should be lighter than button_bg
        let bh = look.button_hover();
        assert!(bh.GetRed() > look.button_bg_color.GetRed());
    }
}
