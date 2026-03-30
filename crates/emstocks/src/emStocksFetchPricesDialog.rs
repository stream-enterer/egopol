use super::emStocksPricesFetcher::emStocksPricesFetcher;

/// Port of C++ emStocksFetchPricesDialog::ProgressBarPanel.
/// DIVERGED: Data model only — painting deferred.
pub struct ProgressBarPanel {
    pub progress_in_percent: f64,
}

impl Default for ProgressBarPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressBarPanel {
    pub fn new() -> Self {
        Self { progress_in_percent: 0.0 }
    }

    pub fn SetProgressInPercent(&mut self, progress: f64) {
        self.progress_in_percent = progress;
    }
}

/// Port of C++ emStocksFetchPricesDialog.
/// DIVERGED: Data model only — dialog infrastructure deferred.
pub struct emStocksFetchPricesDialog {
    pub fetcher: emStocksPricesFetcher,
    pub label_text: String,
    pub progress_bar: ProgressBarPanel,
}

impl emStocksFetchPricesDialog {
    pub fn new(api_script: &str, api_script_interpreter: &str, api_key: &str) -> Self {
        Self {
            fetcher: emStocksPricesFetcher::new(api_script, api_script_interpreter, api_key),
            label_text: String::new(),
            progress_bar: ProgressBarPanel::new(),
        }
    }

    /// Port of C++ AddStockIds.
    pub fn AddStockIds(&mut self, stock_ids: &[String]) {
        self.fetcher.AddStockIds(stock_ids);
    }

    /// Port of C++ UpdateControls.
    pub fn UpdateControls(&mut self) {
        self.progress_bar.SetProgressInPercent(self.fetcher.GetProgressInPercent());

        if self.fetcher.HasFinished() {
            let error = self.fetcher.GetError();
            if error.is_empty() {
                self.label_text = "Done.".to_string();
            } else {
                self.label_text = error.to_string();
            }
        } else if let Some(stock_id) = self.fetcher.GetCurrentStockId() {
            self.label_text = format!("Fetching prices for stock {}...", stock_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_new() {
        let dialog = emStocksFetchPricesDialog::new("script.pl", "perl", "key");
        assert!(dialog.fetcher.HasFinished());
        assert_eq!(dialog.progress_bar.progress_in_percent, 0.0);
    }

    #[test]
    fn dialog_add_stock_ids() {
        let mut dialog = emStocksFetchPricesDialog::new("script.pl", "perl", "key");
        dialog.AddStockIds(&["1".to_string(), "2".to_string()]);
        assert!(!dialog.fetcher.HasFinished());
    }

    #[test]
    fn dialog_update_controls_finished() {
        let mut dialog = emStocksFetchPricesDialog::new("", "", "");
        dialog.UpdateControls();
        assert_eq!(dialog.label_text, "Done.");
        assert_eq!(dialog.progress_bar.progress_in_percent, 100.0);
    }

    #[test]
    fn progress_bar_set() {
        let mut pb = ProgressBarPanel::new();
        pb.SetProgressInPercent(50.0);
        assert_eq!(pb.progress_in_percent, 50.0);
    }
}
