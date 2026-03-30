// Port of C++ emStocksFilePanel.h / emStocksFilePanel.cpp

/// Port of C++ emStocksFilePanel.
/// DIVERGED: Data model only — panel framework integration deferred.
pub struct emStocksFilePanel {
    pub bg_color: u32, // emColor packed RGBA
}

impl emStocksFilePanel {
    pub fn new() -> Self {
        Self {
            bg_color: 0x131520FF, // matches C++ BgColor(0x131520ff)
        }
    }

    /// Port of C++ GetIconFileName.
    pub fn GetIconFileName(&self) -> &str {
        "documents.tga"
    }
}

impl Default for emStocksFilePanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_panel_new() {
        let panel = emStocksFilePanel::new();
        assert_eq!(panel.bg_color, 0x131520FF);
    }

    #[test]
    fn file_panel_icon() {
        let panel = emStocksFilePanel::new();
        assert_eq!(panel.GetIconFileName(), "documents.tga");
    }
}
