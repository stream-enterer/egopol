// Port of C++ emStocksItemChart.h / emStocksItemChart.cpp

use super::emStocksConfig::emStocksConfig;
use super::emStocksRec::{
    AddDaysToDate, GetCurrentDate, GetDateDifference, ParseDate, StockRec,
};

/// Port of C++ emStocksItemChart::Price.
#[derive(Debug, Clone, Copy, Default)]
pub struct Price {
    pub valid: bool,
    pub value: f64,
}

impl Price {
    /// Port of C++ Price::Set. Parses a string to a price value.
    pub fn Set(&mut self, s: &str) {
        if s.is_empty() {
            self.valid = false;
            self.value = 0.0;
        } else {
            self.value = s.parse::<f64>().unwrap_or(0.0);
            self.valid = self.value > 0.0 || s.starts_with('0');
        }
    }
}

/// Port of C++ emStocksItemChart.
/// DIVERGED: No emBorder/emPanel inheritance. Data model and update logic are
/// ported; painting is stubbed (emStocks has no golden test infrastructure).
pub struct emStocksItemChart {
    // Data state
    data_up_to_date: bool,

    // Time range
    pub start_date: String,
    pub start_year: i32,
    pub start_month: i32,
    pub start_day: i32,
    pub end_date: String,
    pub total_days: i32,
    pub days_per_price: i32,

    // Price data
    pub owning_shares: bool,
    pub trade_price: Price,
    pub trade_price_text: String,
    pub trade_offset_days: i32,
    pub price_on_selected_date: Price,
    pub price_on_selected_date_text: String,
    pub desired_price: Price,
    pub desired_price_text: String,
    pub prices: Vec<Price>,
    pub min_price: Price,
    pub max_price: Price,

    // Transform
    pub x_offset: f64,
    pub x_factor: f64,
    pub y_offset: f64,
    pub y_factor: f64,
    pub lower_price: f64,
    pub upper_price: f64,

    // Associated stock record index (replaces C++ pointer/listener)
    stock_rec_index: Option<usize>,

    // Selected date from listbox
    pub selected_date: String,
}

impl Default for emStocksItemChart {
    fn default() -> Self {
        Self::new()
    }
}

impl emStocksItemChart {
    /// Port of C++ constructor defaults.
    pub fn new() -> Self {
        Self {
            data_up_to_date: false,
            start_date: String::new(),
            start_year: 0,
            start_month: 0,
            start_day: 0,
            end_date: String::new(),
            total_days: 1,
            days_per_price: 1,
            owning_shares: false,
            trade_price: Price {
                valid: false,
                value: 0.0,
            },
            trade_price_text: String::new(),
            trade_offset_days: i32::MIN,
            price_on_selected_date: Price {
                valid: false,
                value: 0.0,
            },
            price_on_selected_date_text: String::new(),
            desired_price: Price {
                valid: false,
                value: 0.0,
            },
            desired_price_text: String::new(),
            prices: Vec::new(),
            min_price: Price {
                valid: false,
                value: 0.0,
            },
            max_price: Price {
                valid: false,
                value: 0.0,
            },
            x_offset: 0.0,
            x_factor: 1.0,
            y_offset: 0.0,
            y_factor: -1.0,
            lower_price: 0.0,
            upper_price: 1.0,
            stock_rec_index: None,
            selected_date: String::new(),
        }
    }

    /// Get the stock rec index.
    pub fn GetStockRecIndex(&self) -> Option<usize> {
        self.stock_rec_index
    }

    /// Set which stock to display.
    pub fn SetStockRecIndex(&mut self, index: Option<usize>) {
        if self.stock_rec_index != index {
            self.stock_rec_index = index;
            self.InvalidateData();
        }
    }

    /// Set the selected date (from ListBox).
    pub fn SetSelectedDate(&mut self, date: &str) {
        if self.selected_date != date {
            self.selected_date = date.to_string();
            self.InvalidateData();
        }
    }

    /// Mark data as needing update.
    pub fn InvalidateData(&mut self) {
        self.data_up_to_date = false;
    }

    /// Port of C++ UpdateData. Recalculates all derived data from StockRec and Config.
    /// Takes stock_rec and config as parameters (avoids needing Rc<RefCell<>> references).
    pub fn UpdateData(&mut self, stock_rec: Option<&StockRec>, config: &emStocksConfig) {
        if self.data_up_to_date {
            return;
        }

        if let Some(rec) = stock_rec {
            self.UpdateTimeRange(rec, config);
            self.UpdatePrices1(rec);
            self.UpdatePrices2(rec);
            self.UpdateTransformation();
        } else {
            // No stock rec: clear everything
            self.owning_shares = false;
            self.trade_price.valid = false;
            self.trade_price_text.clear();
            self.trade_offset_days = i32::MIN;
            self.price_on_selected_date.valid = false;
            self.price_on_selected_date_text.clear();
            self.desired_price.valid = false;
            self.desired_price_text.clear();
            self.min_price.valid = false;
            self.max_price.valid = false;
            self.prices.clear();
        }

        self.data_up_to_date = true;
    }

    /// Port of C++ UpdateTimeRange.
    fn UpdateTimeRange(&mut self, _stock_rec: &StockRec, config: &emStocksConfig) {
        // C++: EndDate=ListBox.GetSelectedDate();
        self.end_date = self.selected_date.clone();
        if ParseDate(&self.end_date).is_none() {
            self.end_date = GetCurrentDate();
        }
        // C++: EndDate=emStocksFileModel::AddDaysToDate(1,EndDate);
        self.end_date = AddDaysToDate(1, &self.end_date);
        // C++: TotalDays=Config.CalculateChartPeriodDays(EndDate);
        self.total_days = config.CalculateChartPeriodDays(&self.end_date);
        // C++: StartDate=emStocksFileModel::AddDaysToDate(-TotalDays,EndDate);
        self.start_date = AddDaysToDate(-self.total_days, &self.end_date);
        // C++: emStocksRec::ParseDate(StartDate,&StartYear,&StartMonth,&StartDay);
        if let Some((y, m, d)) = ParseDate(&self.start_date) {
            self.start_year = y;
            self.start_month = m;
            self.start_day = d;
        } else {
            self.start_year = 0;
            self.start_month = 0;
            self.start_day = 0;
        }
        // C++: DaysPerPrice=CalculateDaysPerPrice();
        self.days_per_price = self.CalculateDaysPerPrice();
    }

    /// Port of C++ CalculateDaysPerPrice.
    /// DIVERGED: No IsViewed() check (no panel context). Uses simplified version
    /// that only depends on TotalDays, equivalent to C++ non-viewed path returning
    /// TotalDays but with the power-of-2/256 division for viewed panels.
    /// Since we have no view context, we use a fixed divisor approach.
    fn CalculateDaysPerPrice(&self) -> i32 {
        // C++: if (!IsViewed()) return TotalDays;
        // For the data-model-only port, we use the power-of-2 approach with
        // m = TotalDays/2 (the simpler bound when no view is available).
        let m = self.total_days / 2;
        let mut d = 1;
        while d < m {
            d <<= 1;
        }
        // C++ divides by 256 when using the view-based path, but since we
        // don't have a view, we just use 1 as minimum.
        // For tests: use the task-specified algorithm (d/256, min 1)
        d /= 256;
        if d <= 0 {
            d = 1;
        }
        d
    }

    /// Port of C++ UpdatePrices1. Sets trade price, price on selected date,
    /// desired price, and initializes min/max from those.
    fn UpdatePrices1(&mut self, stock_rec: &StockRec) {
        self.owning_shares = stock_rec.owning_shares;

        self.trade_price.Set(&stock_rec.trade_price);
        self.min_price = self.trade_price;
        self.max_price = self.trade_price;

        if self.trade_price.valid {
            let label = if self.owning_shares {
                "Purchase Price"
            } else {
                "Sale Price"
            };
            self.trade_price_text = format!("{}: {}", label, &stock_rec.trade_price);

            if !stock_rec.trade_date.is_empty() {
                let (diff, _valid) =
                    GetDateDifference(&self.start_date, &stock_rec.trade_date);
                self.trade_offset_days = diff;
            } else {
                self.trade_offset_days = i32::MIN;
            }
        } else {
            self.trade_price_text.clear();
            self.trade_offset_days = i32::MIN;
        }

        let price_str = stock_rec.GetPriceOfDate(&self.selected_date);
        self.price_on_selected_date.Set(&price_str);
        if self.price_on_selected_date.valid {
            if !self.min_price.valid
                || self.min_price.value > self.price_on_selected_date.value
            {
                self.min_price = self.price_on_selected_date;
            }
            if !self.max_price.valid
                || self.max_price.value < self.price_on_selected_date.value
            {
                self.max_price = self.price_on_selected_date;
            }
            self.price_on_selected_date_text = format!("Price: {}", price_str);
        } else {
            self.price_on_selected_date_text.clear();
        }

        self.desired_price.Set(&stock_rec.desired_price);
        if self.desired_price.valid {
            if !self.min_price.valid || self.min_price.value > self.desired_price.value {
                self.min_price = self.desired_price;
            }
            if !self.max_price.valid || self.max_price.value < self.desired_price.value {
                self.max_price = self.desired_price;
            }
            self.desired_price_text =
                format!("Desired Price: {}", &stock_rec.desired_price);
        } else {
            self.desired_price_text.clear();
        }
    }

    /// Port of C++ UpdatePrices2. Populates the prices array from StockRec price
    /// history, computing per-bucket averages and updating min/max.
    fn UpdatePrices2(&mut self, stock_rec: &StockRec) {
        if stock_rec.prices.is_empty() || stock_rec.last_price_date.is_empty() {
            self.prices.clear();
            return;
        }

        let s_bytes = stock_rec.prices.as_bytes();
        let s_len = s_bytes.len();

        let price_count = (self.total_days + self.days_per_price - 1) / self.days_per_price;
        self.prices = vec![
            Price {
                valid: false,
                value: 0.0,
            };
            price_count as usize
        ];

        let mut remaining_days = (self.total_days - 1) % self.days_per_price + 1;

        let (diff_days, _) =
            GetDateDifference(&stock_rec.last_price_date, &self.end_date);
        let mut diff_days = diff_days - 1;

        // s2 is the exclusive end pointer into the prices string
        let mut s2 = s_len;
        // t2 is the exclusive end index into the prices vec
        let mut t2 = self.prices.len();

        if diff_days < 0 {
            // LastPriceDate is after EndDate: skip prices from the end
            while s2 > 0 {
                s2 -= 1;
                if s_bytes[s2] == b'|' {
                    diff_days += 1;
                    if diff_days >= 0 {
                        break;
                    }
                }
            }
        } else if diff_days > 0 {
            // LastPriceDate is before EndDate: skip buckets from the end
            t2 = t2.saturating_sub((diff_days / self.days_per_price) as usize);
            remaining_days -= diff_days % self.days_per_price;
            if remaining_days <= 0 {
                t2 = t2.saturating_sub(1);
                remaining_days += self.days_per_price;
            }
        }

        if s2 == 0 || t2 == 0 {
            return;
        }

        let mut minv: f64 = 1e100;
        let mut maxv: f64 = -1e100;
        let mut tv: f64 = 0.0;
        let mut n: i32 = 0;

        loop {
            s2 -= 1;
            if s_bytes[s2] != b'|' {
                // Find start of this price value (scan back to previous '|' or start)
                while s2 > 0 && s_bytes[s2 - 1] != b'|' {
                    s2 -= 1;
                }
                // Parse the price value
                let price_str =
                    std::str::from_utf8(&s_bytes[s2..]).unwrap_or("0");
                // Find end of this value (up to next '|' or end)
                let val_end = price_str.find('|').unwrap_or(price_str.len());
                let sv: f64 = price_str[..val_end].parse().unwrap_or(0.0);
                tv += sv;
                n += 1;
                if minv > sv {
                    minv = sv;
                }
                if maxv < sv {
                    maxv = sv;
                }
            }
            remaining_days -= 1;
            if remaining_days <= 0 {
                t2 -= 1;
                if n > 0 {
                    self.prices[t2].valid = true;
                    self.prices[t2].value = tv / n as f64;
                }
                if t2 == 0 {
                    break;
                }
                remaining_days = self.days_per_price;
                tv = 0.0;
                n = 0;
            }
            if s2 == 0 {
                break;
            }
        }

        // Handle leftover partial bucket
        if t2 > 0 && n > 0 {
            t2 -= 1;
            self.prices[t2].valid = true;
            self.prices[t2].value = tv / n as f64;
        }

        if minv <= maxv {
            if !self.min_price.valid || self.min_price.value > minv {
                self.min_price.valid = true;
                self.min_price.value = minv;
            }
            if !self.max_price.valid || self.max_price.value < maxv {
                self.max_price.valid = true;
                self.max_price.value = maxv;
            }
        }
    }

    /// Port of C++ UpdateTransformation.
    /// DIVERGED: No GetContentRect() call (no panel context). Uses unit rect
    /// (x=0, y=0, w=1, h=1) with the same margin logic as C++.
    fn UpdateTransformation(&mut self) {
        // C++ calls GetContentRect(&x,&y,&w,&h) — we use unit rect
        let x: f64 = 0.0;
        let mut y: f64 = 0.0;
        let w: f64 = 1.0;
        let mut h: f64 = 1.0;
        let d = h * 0.008;
        y += d;
        h -= 2.0 * d;

        self.x_offset = x;
        if self.total_days > 0 {
            self.x_factor = w / self.total_days as f64;
        } else {
            self.x_factor = 1.0;
        }

        if self.min_price.valid && self.max_price.valid {
            let c: f64;
            if self.trade_price.valid {
                c = self.trade_price.value;
            } else if self.desired_price.valid {
                c = self.desired_price.value;
            } else {
                c = (self.min_price.value + self.max_price.value) * 0.5;
            }
            let d_price = f64::max(
                0.5 * c,
                f64::max(
                    self.max_price.value - c,
                    c - self.min_price.value,
                ),
            );
            let mut p1 = c - d_price;
            let mut p2 = c + d_price;
            if p1 < 0.0 {
                p1 = f64::min(0.0, self.min_price.value);
                p2 = self.max_price.value;
            }
            p2 = f64::max(p2, p1 + 1e-6);

            self.y_factor = h / (p1 - p2);
            self.y_offset = y - self.y_factor * p2;
            self.lower_price = p1;
            self.upper_price = p2;
        } else {
            let p1 = 0.0;
            let p2 = 100.0001;
            self.y_factor = h / (p1 - p2);
            self.y_offset = y - self.y_factor * p2;
            self.lower_price = p1;
            self.upper_price = p2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emStocks::emStocksConfig::ChartPeriod;

    #[test]
    fn price_set_valid() {
        let mut p = Price::default();
        p.Set("100.50");
        assert!(p.valid);
        assert!((p.value - 100.5).abs() < 1e-10);
    }

    #[test]
    fn price_set_empty() {
        let mut p = Price::default();
        p.Set("");
        assert!(!p.valid);
    }

    #[test]
    fn price_set_zero() {
        let mut p = Price::default();
        p.Set("0");
        assert!(p.valid);
        assert_eq!(p.value, 0.0);
    }

    #[test]
    fn chart_new_defaults() {
        let chart = emStocksItemChart::new();
        assert!(!chart.data_up_to_date);
        assert_eq!(chart.total_days, 1);
        assert_eq!(chart.days_per_price, 1);
    }

    #[test]
    fn chart_update_data_no_stock() {
        let mut chart = emStocksItemChart::new();
        let config = emStocksConfig::default();
        chart.UpdateData(None, &config);
        assert!(chart.prices.is_empty());
    }

    #[test]
    fn chart_update_data_with_stock() {
        let mut chart = emStocksItemChart::new();
        chart.selected_date = "2024-06-15".to_string();
        let config = emStocksConfig {
            chart_period: ChartPeriod::Week1,
            ..Default::default()
        };
        let mut stock = StockRec::default();
        stock.AddPrice("2024-06-10", "100");
        stock.AddPrice("2024-06-15", "105");

        chart.UpdateData(Some(&stock), &config);
        assert!(chart.total_days > 0);
        assert!(!chart.prices.is_empty());
    }

    #[test]
    fn calculate_days_per_price() {
        let mut chart = emStocksItemChart::new();
        chart.total_days = 365;
        assert_eq!(chart.CalculateDaysPerPrice(), 1); // 256/256=1, next power 512/256=2 but m=182, d=256 >= 182 so d=256, 256/256=1

        chart.total_days = 7;
        assert_eq!(chart.CalculateDaysPerPrice(), 1); // 4/256 = 0 -> 1
    }

    #[test]
    fn chart_trade_price_text() {
        let mut chart = emStocksItemChart::new();
        chart.selected_date = "2024-06-15".to_string();
        let config = emStocksConfig {
            chart_period: ChartPeriod::Week1,
            ..Default::default()
        };
        let mut stock = StockRec::default();
        stock.owning_shares = true;
        stock.trade_price = "50.00".to_string();
        stock.trade_date = "2024-06-12".to_string();
        stock.AddPrice("2024-06-15", "55");

        chart.UpdateData(Some(&stock), &config);
        assert!(chart.trade_price.valid);
        assert!(chart.trade_price_text.contains("Purchase Price"));
    }

    #[test]
    fn chart_desired_price() {
        let mut chart = emStocksItemChart::new();
        chart.selected_date = "2024-06-15".to_string();
        let config = emStocksConfig {
            chart_period: ChartPeriod::Week1,
            ..Default::default()
        };
        let mut stock = StockRec::default();
        stock.desired_price = "120.00".to_string();
        stock.AddPrice("2024-06-15", "100");

        chart.UpdateData(Some(&stock), &config);
        assert!(chart.desired_price.valid);
        assert!((chart.desired_price.value - 120.0).abs() < 1e-10);
        assert!(chart.desired_price_text.contains("Desired Price"));
    }

    #[test]
    fn chart_transformation_valid() {
        let mut chart = emStocksItemChart::new();
        chart.selected_date = "2024-06-15".to_string();
        let config = emStocksConfig {
            chart_period: ChartPeriod::Week1,
            ..Default::default()
        };
        let mut stock = StockRec::default();
        stock.AddPrice("2024-06-10", "100");
        stock.AddPrice("2024-06-15", "110");

        chart.UpdateData(Some(&stock), &config);
        // Y factor should be negative (price increases upward)
        assert!(chart.y_factor < 0.0);
        assert!(chart.upper_price > chart.lower_price);
    }

    #[test]
    fn chart_invalidate_resets_flag() {
        let mut chart = emStocksItemChart::new();
        let config = emStocksConfig::default();
        chart.UpdateData(None, &config);
        assert!(chart.data_up_to_date);
        chart.InvalidateData();
        assert!(!chart.data_up_to_date);
    }
}
