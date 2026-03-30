//! Port of C++ emFileLinkPanel content coordinate calculation and border constants.

pub const BORDER_BG_COLOR: u32 = 0xBBBBBBFF;
pub const BORDER_FG_COLOR: u32 = 0x444444FF;
pub const MIN_VIEW_PERCENT: f64 = 60.0;

/// Calculate content coordinates within the link panel.
/// panel_height: GetHeight()
/// have_border: whether to show border (depends on parent panel type)
/// have_dir_entry: whether the link target has a dir entry (from FileLinkModel)
/// _theme_height: theme.Height for inner scaling
/// pad_l/t/r/b: theme LnkPaddingL/T/R/B
#[allow(clippy::too_many_arguments)]
pub fn CalcContentCoords(
    panel_height: f64,
    have_border: bool,
    _have_dir_entry: bool,
    _theme_height: f64,
    pad_l: f64,
    pad_t: f64,
    pad_r: f64,
    pad_b: f64,
) -> (f64, f64, f64, f64) {
    if !have_border {
        return (0.0, 0.0, 1.0, panel_height);
    }
    // With border: apply padding
    let x = pad_l;
    let y = pad_t * panel_height;
    let w = 1.0 - pad_l - pad_r;
    let h = panel_height - (pad_t + pad_b) * panel_height;
    (x.max(0.0), y.max(0.0), w.max(0.001), h.max(0.001))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_coords_no_border() {
        let (x, y, w, _h) = CalcContentCoords(1.0, false, false, 1.5, 0.0, 0.0, 0.0, 0.0);
        assert!((x - 0.0).abs() < 1e-9);
        assert!((y - 0.0).abs() < 1e-9);
        assert!((w - 1.0).abs() < 1e-9);
    }

    #[test]
    fn content_coords_with_border() {
        let (x, y, w, _h) = CalcContentCoords(1.0, true, false, 1.5, 0.05, 0.05, 0.05, 0.05);
        assert!(x > 0.0);
        assert!(y > 0.0);
        assert!(w < 1.0);
    }

    #[test]
    fn border_colors() {
        assert_eq!(BORDER_BG_COLOR, 0xBBBBBBFF_u32);
        assert_eq!(BORDER_FG_COLOR, 0x444444FF_u32);
    }
}
