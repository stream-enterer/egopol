# emStocks Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the C++ emStocks app module to Rust, stress-testing newly ported emCore types (emArray, emAvlTreeMap, emList, emCrossPtr) and proving emCore completeness.

**Architecture:** Bottom-up data layer first (emStocksRec, emStocksConfig, emStocksFileModel), then engine layer (emStocksPricesFetcher), then UI panels, then plugin registration. Each phase is gated by tests passing.

**Tech Stack:** Rust, emCore (Rc/RefCell ownership, emRec serialization, emEngine cooperative scheduling, emProcess IPC)

**Spec:** `docs/superpowers/specs/2026-03-29-emStocks-port-design.md`

**C++ source:** `~/git/eaglemode-0.96.4/include/emStocks/` and `~/git/eaglemode-0.96.4/src/emStocks/`

---

## File Structure

### New files to create

```
src/emStocks/mod.rs                        -- module declarations + re-exports
src/emStocks/emStocksRec.rs                -- emStocksRec, StockRec, Interest
src/emStocks/emStocksConfig.rs             -- emStocksConfig, ChartPeriod, Sorting
src/emStocks/emStocksFileModel.rs          -- emStocksFileModel
src/emStocks/emStocksPricesFetcher.rs      -- emStocksPricesFetcher
src/emStocks/emStocksItemChart.rs          -- emStocksItemChart
src/emStocks/emStocksItemPanel.rs          -- emStocksItemPanel, CategoryPanel
src/emStocks/emStocksListBox.rs            -- emStocksListBox
src/emStocks/emStocksControlPanel.rs       -- emStocksControlPanel, FileFieldPanel, CategoryPanel
src/emStocks/emStocksFetchPricesDialog.rs  -- emStocksFetchPricesDialog, ProgressBarPanel
src/emStocks/emStocksFilePanel.rs          -- emStocksFilePanel
src/emStocks/emStocksFpPlugin.rs           -- plugin registration
tests/behavioral/emstocks_rec.rs           -- Record round-trip, date math tests
tests/behavioral/emstocks_prices_fetcher.rs -- COW + emCrossPtr stress tests
tests/unit/emstocks.rs                     -- unit tests for enums, parsing, search
```

### Files to modify

```
src/lib.rs                                 -- add `pub mod emStocks;`
src/emCore/emFileModel.rs                  -- add emAbsoluteFileModelClient
tests/behavioral/main.rs                   -- add mod declarations (if test harness uses one)
tests/unit/main.rs                         -- add mod declarations (if test harness uses one)
```

---

## Phase 1: emCore Gap + Data Layer

### Task 1: Add emAbsoluteFileModelClient to emFileModel.rs

**Files:**
- Modify: `src/emCore/emFileModel.rs`
- Test: `tests/behavioral/file_model.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emCore/emFileModel.h` (search for `emAbsoluteFileModelClient`). It holds a pointer to a file model + a signal link for change/state notifications. In Rust, this translates to an `Rc<RefCell<T>>` + signal tracking.

- [ ] **Step 1: Read the C++ emAbsoluteFileModelClient definition**

Read `~/git/eaglemode-0.96.4/include/emCore/emFileModel.h` and search for `emAbsoluteFileModelClient`. Note the fields (pointer to model, signal connections) and methods (`SetModel`, `GetModel`, implicit validity via pointer). Also read `~/git/eaglemode-0.96.4/src/emCore/emFileModel.cpp` for the implementation.

- [ ] **Step 2: Write failing test for emAbsoluteFileModelClient**

Add a test to `tests/behavioral/file_model.rs` (create the file if it doesn't exist). The test should verify:
1. Create a client pointing to a file model — `get()` returns Some
2. Drop the file model — `get()` returns None
3. Signal connectivity — client tracks model's change signal

Check how existing behavioral tests in `tests/behavioral/` are structured (e.g., `tests/behavioral/cross_ptr.rs`) and follow the same pattern.

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo-nextest ntr -E 'test(absolute_file_model_client)'
```

Expected: compilation error (type doesn't exist yet)

- [ ] **Step 4: Implement emAbsoluteFileModelClient**

Add to `src/emCore/emFileModel.rs`:

```rust
/// Port of C++ emAbsoluteFileModelClient.
/// Holds a tracked reference to a file model with change signal connectivity.
///
/// DIVERGED: Uses Weak<RefCell<T>> instead of raw pointer. Validity is
/// determined by Weak::upgrade() rather than explicit null check.
pub(crate) struct emAbsoluteFileModelClient<T> {
    model: Weak<RefCell<T>>,
    change_signal: Option<SignalId>,
    file_state_signal: Option<SignalId>,
}
```

Implement methods matching C++ API names:
- `new()` — empty client
- `SetModel(model: &Rc<RefCell<T>>, change_signal: SignalId, file_state_signal: SignalId)` — set tracked model
- `GetModel() -> Option<Rc<RefCell<T>>>` — returns upgraded Rc if alive
- `GetChangeSignal() -> Option<SignalId>`
- `GetFileStateSignal() -> Option<SignalId>`

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo-nextest ntr -E 'test(absolute_file_model_client)'
```

- [ ] **Step 6: Run full test suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 7: Commit**

```bash
git add src/emCore/emFileModel.rs tests/behavioral/file_model.rs
git commit -m "feat(emCore): add emAbsoluteFileModelClient to emFileModel"
```

---

### Task 2: Create emStocks module skeleton

**Files:**
- Create: `src/emStocks/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create src/emStocks/ directory and mod.rs**

```rust
// src/emStocks/mod.rs
#![allow(non_camel_case_types)]

pub mod emStocksRec;
```

- [ ] **Step 2: Create a minimal emStocksRec.rs placeholder**

```rust
// src/emStocks/emStocksRec.rs
// Port of C++ emStocksRec.h / emStocksRec.cpp
```

- [ ] **Step 3: Add emStocks module to lib.rs**

Modify `src/lib.rs`:

```rust
#[allow(non_snake_case)]
pub mod emCore;

#[allow(non_snake_case)]
pub mod emStocks;
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/emStocks/mod.rs src/emStocks/emStocksRec.rs
git commit -m "feat(emStocks): add module skeleton"
```

---

### Task 3: Port Interest enum and StockRec

**Files:**
- Create content in: `src/emStocks/emStocksRec.rs`
- Test: inline `#[cfg(test)]` module in same file + `tests/behavioral/emstocks_rec.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksRec.h` (InterestType enum, InterestRec class, StockRec class) and `~/git/eaglemode-0.96.4/src/emStocks/emStocksRec.cpp` (InterestRec::TryStartReading for deprecated identifier handling, all StockRec methods).

- [ ] **Step 1: Write failing tests for Interest enum**

Add inline tests to `src/emStocks/emStocksRec.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interest_from_str_canonical() {
        assert_eq!(Interest::from_str("HIGH"), Ok(Interest::High));
        assert_eq!(Interest::from_str("MEDIUM"), Ok(Interest::Medium));
        assert_eq!(Interest::from_str("LOW"), Ok(Interest::Low));
    }

    #[test]
    fn interest_from_str_deprecated_with_bug() {
        // C++ InterestRec with bugInDeprecatedIdentifiers=true:
        // "LOW_INTEREST" -> HIGH, "HIGH_INTEREST" -> LOW (bug preserved)
        assert_eq!(Interest::from_deprecated_bugged("LOW_INTEREST"), Interest::High);
        assert_eq!(Interest::from_deprecated_bugged("HIGH_INTEREST"), Interest::Low);
        assert_eq!(Interest::from_deprecated_bugged("MEDIUM_INTEREST"), Interest::Medium);
    }

    #[test]
    fn interest_from_str_deprecated_no_bug() {
        // C++ InterestRec with bugInDeprecatedIdentifiers=false:
        assert_eq!(Interest::from_deprecated_normal("LOW_INTEREST"), Interest::Low);
        assert_eq!(Interest::from_deprecated_normal("HIGH_INTEREST"), Interest::High);
    }

    #[test]
    fn interest_display() {
        assert_eq!(Interest::High.to_string(), "HIGH");
        assert_eq!(Interest::Medium.to_string(), "MEDIUM");
        assert_eq!(Interest::Low.to_string(), "LOW");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo-nextest ntr -E 'test(interest)'
```

Expected: compilation error

- [ ] **Step 3: Implement Interest enum**

In `src/emStocks/emStocksRec.rs`:

```rust
use std::fmt;
use std::str::FromStr;

/// Port of C++ emStocksRec::InterestType + InterestRec.
/// DIVERGED: Rust enum replaces C++ int enum + emEnumRec subclass.
/// Deprecated identifier handling is via explicit methods rather than
/// virtual TryStartReading override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Interest {
    High = 0,
    Medium = 1,
    Low = 2,
}

impl fmt::Display for Interest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Interest::High => write!(f, "HIGH"),
            Interest::Medium => write!(f, "MEDIUM"),
            Interest::Low => write!(f, "LOW"),
        }
    }
}

impl FromStr for Interest {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HIGH" => Ok(Interest::High),
            "MEDIUM" => Ok(Interest::Medium),
            "LOW" => Ok(Interest::Low),
            _ => Err(format!("Unknown interest identifier: {}", s)),
        }
    }
}

impl Interest {
    /// Parse deprecated identifier with bug-in-deprecated-identifiers=true.
    /// C++ emStocksRec.cpp lines 62-72: LOW_INTEREST maps to HIGH (bug preserved).
    pub(crate) fn from_deprecated_bugged(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "LOW_INTEREST" => Interest::High,    // Bug: swapped
            "MEDIUM_INTEREST" => Interest::Medium,
            "HIGH_INTEREST" => Interest::Low,    // Bug: swapped
            _ => Interest::from_str(s).unwrap_or(Interest::Medium),
        }
    }

    /// Parse deprecated identifier with bug-in-deprecated-identifiers=false.
    pub(crate) fn from_deprecated_normal(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "LOW_INTEREST" => Interest::Low,
            "MEDIUM_INTEREST" => Interest::Medium,
            "HIGH_INTEREST" => Interest::High,
            _ => Interest::from_str(s).unwrap_or(Interest::Medium),
        }
    }
}
```

- [ ] **Step 4: Run Interest tests**

```bash
cargo-nextest ntr -E 'test(interest)'
```

- [ ] **Step 5: Write failing tests for StockRec**

Add to the test module:

```rust
#[test]
fn stock_rec_default() {
    let rec = StockRec::default();
    assert_eq!(rec.id, "");
    assert_eq!(rec.interest, Interest::Medium);
    assert!(rec.web_pages.is_empty());
}

#[test]
fn stock_rec_record_round_trip() {
    let mut rec = StockRec::default();
    rec.id = "42".to_string();
    rec.name = "Test Stock".to_string();
    rec.symbol = "TST".to_string();
    rec.interest = Interest::High;
    rec.web_pages = vec!["https://example.com".to_string()];

    let serialized = rec.to_rec();
    let deserialized = StockRec::from_rec(&serialized).unwrap();

    assert_eq!(deserialized.id, "42");
    assert_eq!(deserialized.name, "Test Stock");
    assert_eq!(deserialized.symbol, "TST");
    assert_eq!(deserialized.interest, Interest::High);
    assert_eq!(deserialized.web_pages, vec!["https://example.com"]);
}
```

- [ ] **Step 6: Implement StockRec**

Port the struct with all 18+ fields from C++ `StockRec`. Implement `Default` (matching C++ constructor defaults — empty strings, `false` bools, `Interest::Medium`). Implement `Record` trait with field names matching C++ exactly: `"Id"`, `"Name"`, `"Symbol"`, `"WKN"`, `"ISIN"`, etc.

Read the full C++ field list from `~/git/eaglemode-0.96.4/src/emStocks/emStocksRec.cpp` constructor (lines 84-109) to get exact field names and default values.

Include `cross_ptr_list: emCrossPtrList` field and `LinkCrossPtr` method matching C++.

```rust
use crate::emCore::emCrossPtr::{emCrossPtrList, emCrossPtrPrivate};

/// Port of C++ emStocksRec::StockRec.
/// DIVERGED: Rust struct fields use snake_case. Method names preserve C++ names.
#[derive(Debug, Clone)]
pub(crate) struct StockRec {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub wkn: String,
    pub isin: String,
    pub country: String,
    pub sector: String,
    pub collection: String,
    pub comment: String,
    pub owning_shares: bool,
    pub own_shares: String,
    pub trade_price: String,
    pub trade_date: String,
    pub prices: String,
    pub last_price_date: String,
    pub desired_price: String,
    pub expected_dividend: String,
    pub inquiry_date: String,
    pub interest: Interest,
    pub web_pages: Vec<String>,
    cross_ptr_list: emCrossPtrList,
}
```

- [ ] **Step 7: Run tests**

```bash
cargo-nextest ntr -E 'test(stock_rec)'
```

- [ ] **Step 8: Commit**

```bash
git add src/emStocks/emStocksRec.rs
git commit -m "feat(emStocks): port Interest enum and StockRec with Record impl"
```

---

### Task 4: Port StockRec date arithmetic and price methods

**Files:**
- Modify: `src/emStocks/emStocksRec.rs`

C++ reference: `~/git/eaglemode-0.96.4/src/emStocks/emStocksRec.cpp` — all static methods on emStocksRec (ParseDate, CompareDates, GetDaysOfMonth, AddDaysToDate, GetDateDifference, GetCurrentDate) and StockRec methods (GetPricePtrOfDate, GetPriceOfDate, GetPricesDateBefore, GetPricesDateAfter, AddPrice, IsMatchingSearchText, GetTradeValue, GetValueOfDate, GetDifferenceValueOfDate, GetAchievementOfDate, GetRiseUntilDate).

- [ ] **Step 1: Write failing tests for date arithmetic**

```rust
#[test]
fn parse_date_valid() {
    let (y, m, d) = emStocksRec::ParseDate("2024-03-15").unwrap();
    assert_eq!((y, m, d), (2024, 3, 15));
}

#[test]
fn parse_date_negative_year() {
    let (y, m, d) = emStocksRec::ParseDate("-500-01-01").unwrap();
    assert_eq!(y, -500);
}

#[test]
fn compare_dates() {
    assert!(emStocksRec::CompareDates("2024-03-15", "2024-03-16") < 0);
    assert!(emStocksRec::CompareDates("2024-03-16", "2024-03-15") > 0);
    assert_eq!(emStocksRec::CompareDates("2024-03-15", "2024-03-15"), 0);
}

#[test]
fn days_of_month() {
    assert_eq!(emStocksRec::GetDaysOfMonth(2024, 2), 29); // leap year
    assert_eq!(emStocksRec::GetDaysOfMonth(2023, 2), 28);
    assert_eq!(emStocksRec::GetDaysOfMonth(2024, 1), 31);
    assert_eq!(emStocksRec::GetDaysOfMonth(2024, 4), 30);
}

#[test]
fn add_days_to_date() {
    assert_eq!(emStocksRec::AddDaysToDate(1, "2024-03-31"), "2024-04-01");
    assert_eq!(emStocksRec::AddDaysToDate(-1, "2024-01-01"), "2023-12-31");
    assert_eq!(emStocksRec::AddDaysToDate(366, "2024-01-01"), "2025-01-01");
}

#[test]
fn get_date_difference() {
    assert_eq!(emStocksRec::GetDateDifference("2024-01-01", "2024-01-02"), 1);
    assert_eq!(emStocksRec::GetDateDifference("2024-01-02", "2024-01-01"), -1);
    assert_eq!(emStocksRec::GetDateDifference("2024-01-01", "2025-01-01"), 366); // leap year
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo-nextest ntr -E 'test(date)'
```

- [ ] **Step 3: Implement date arithmetic**

Port the C++ static methods exactly. These are the functions from `emStocksRec.cpp`:
- `ParseDate` — parses "YYYY-MM-DD" format, handles negative years
- `CompareDates` — uses `(y1-y2)*16+m1-m2)*32+d1-d2` formula from C++
- `GetDaysOfMonth` — leap year logic
- `AddDaysToDate` — both the `(&mut y, &mut m, &mut d)` and `(days, date) -> String` variants
- `GetDateDifference` — both the 6-arg and 2-arg variants
- `GetCurrentDate` — uses `chrono::Local` or `time` crate (check what's in Cargo.toml; if neither, use `std::time`)

Match the C++ algorithm exactly — the date arithmetic has specific boundary handling (the `while d<-213` / `while d>243` loops in AddDaysToDate).

- [ ] **Step 4: Run date tests**

```bash
cargo-nextest ntr -E 'test(date)'
```

- [ ] **Step 5: Write failing tests for price methods**

```rust
#[test]
fn stock_add_price_and_retrieve() {
    let mut stock = StockRec::default();
    stock.AddPrice("2024-03-15", "100.50");
    assert_eq!(stock.last_price_date, "2024-03-15");
    assert_eq!(stock.GetPriceOfDate("2024-03-15"), "100.50");
}

#[test]
fn stock_add_multiple_prices() {
    let mut stock = StockRec::default();
    stock.AddPrice("2024-03-14", "99.00");
    stock.AddPrice("2024-03-15", "100.50");
    assert_eq!(stock.GetPriceOfDate("2024-03-14"), "99.00");
    assert_eq!(stock.GetPriceOfDate("2024-03-15"), "100.50");
    assert_eq!(stock.last_price_date, "2024-03-15");
}

#[test]
fn stock_is_matching_search_text() {
    let mut stock = StockRec::default();
    stock.name = "Apple Inc.".to_string();
    stock.symbol = "AAPL".to_string();
    assert!(stock.IsMatchingSearchText("apple"));
    assert!(stock.IsMatchingSearchText("AAPL"));
    assert!(!stock.IsMatchingSearchText("GOOG"));
}

#[test]
fn stock_get_trade_value() {
    let mut stock = StockRec::default();
    stock.owning_shares = true;
    stock.trade_price = "150.00".to_string();
    stock.own_shares = "10".to_string();
    assert_eq!(stock.GetTradeValue(), Some(1500.0));
}

#[test]
fn stock_get_trade_value_not_owning() {
    let stock = StockRec::default();
    assert_eq!(stock.GetTradeValue(), None);
}
```

- [ ] **Step 6: Implement price methods on StockRec**

Port from C++ `emStocksRec.cpp`. Key methods:
- `GetPricePtrOfDate` — internal helper, returns slice into prices string
- `GetPriceOfDate` — extracts single price by date
- `GetPricesDateBefore` / `GetPricesDateAfter` — navigate price history
- `AddPrice` — complex method with MAX_NUM_PRICES trimming, pipe-separated storage
- `IsMatchingSearchText` — case-insensitive substring search across fields
- `GetTradeValue`, `GetValueOfDate`, `GetDifferenceValueOfDate`, `GetAchievementOfDate`, `GetRiseUntilDate` — financial calculations

DIVERGED: Rust methods return `Option<f64>` instead of C++ `bool + *pResult` pattern.

Match the C++ AddPrice algorithm exactly — it handles inserting prices at arbitrary dates, trimming old prices beyond MAX_NUM_PRICES, and managing the pipe-separated Prices string.

- [ ] **Step 7: Run price tests**

```bash
cargo-nextest ntr -E 'test(stock)'
```

- [ ] **Step 8: Implement SharePriceToString and PaymentPriceToString**

Port the C++ formatting functions. `SharePriceToString` uses adaptive decimal places based on magnitude. `PaymentPriceToString` uses fixed 2 decimal places.

- [ ] **Step 9: Run full test suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 10: Commit**

```bash
git add src/emStocks/emStocksRec.rs
git commit -m "feat(emStocks): port StockRec date arithmetic and price methods"
```

---

### Task 5: Port emStocksRec (top-level record)

**Files:**
- Modify: `src/emStocks/emStocksRec.rs`

C++ reference: `~/git/eaglemode-0.96.4/src/emStocks/emStocksRec.cpp` — emStocksRec constructor, GetFormatName, InventStockId, GetStockIndexById, GetStockIndexByStock, GetLatestPricesDate, GetPricesDateBefore, GetPricesDateAfter.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn emstocks_rec_default() {
    let rec = emStocksRec::default();
    assert!(rec.stocks.is_empty());
}

#[test]
fn emstocks_rec_record_round_trip() {
    let mut rec = emStocksRec::default();
    let mut stock = StockRec::default();
    stock.id = "1".to_string();
    stock.name = "Test".to_string();
    rec.stocks.push(stock);

    let serialized = rec.to_rec();
    let deserialized = emStocksRec::from_rec(&serialized).unwrap();
    assert_eq!(deserialized.stocks.len(), 1);
    assert_eq!(deserialized.stocks[0].name, "Test");
}

#[test]
fn emstocks_rec_format_name() {
    let rec = emStocksRec::default();
    assert_eq!(rec.GetFormatName(), "emStocks");
}

#[test]
fn emstocks_rec_invent_stock_id() {
    let mut rec = emStocksRec::default();
    assert_eq!(rec.InventStockId(), "1");

    let mut stock = StockRec::default();
    stock.id = "5".to_string();
    rec.stocks.push(stock);
    assert_eq!(rec.InventStockId(), "6");
}

#[test]
fn emstocks_rec_get_stock_index_by_id() {
    let mut rec = emStocksRec::default();
    let mut stock = StockRec::default();
    stock.id = "42".to_string();
    rec.stocks.push(stock);
    assert_eq!(rec.GetStockIndexById("42"), Some(0));
    assert_eq!(rec.GetStockIndexById("99"), None);
}
```

- [ ] **Step 2: Run to verify failure**

```bash
cargo-nextest ntr -E 'test(emstocks_rec)'
```

- [ ] **Step 3: Implement emStocksRec**

```rust
/// Port of C++ emStocksRec.
pub(crate) struct emStocksRec {
    pub stocks: Vec<StockRec>,
}
```

Implement methods matching C++ names:
- `GetFormatName() -> &str` — returns `"emStocks"`
- `InventStockId() -> String` — finds max ID + 1 (uses emAvlTreeSet for overflow case, matching C++)
- `GetStockIndexById(id: &str) -> Option<usize>` — DIVERGED: returns Option instead of -1
- `GetStockIndexByStock(stock: &StockRec) -> Option<usize>` — pointer comparison via std::ptr::eq
- `GetLatestPricesDate() -> String` — scans all stocks
- `GetPricesDateBefore(date: &str) -> String`
- `GetPricesDateAfter(date: &str) -> String`

Implement `Record` trait: `to_rec()` serializes stocks as `RecValue::Array` of `RecValue::Struct`. `from_rec()` parses with format name `"emStocks"`.

- [ ] **Step 4: Run tests**

```bash
cargo-nextest ntr -E 'test(emstocks_rec)'
```

- [ ] **Step 5: Run full suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 6: Commit**

```bash
git add src/emStocks/emStocksRec.rs
git commit -m "feat(emStocks): port emStocksRec with Record impl and stock index methods"
```

---

### Task 6: Port emStocksConfig

**Files:**
- Create: `src/emStocks/emStocksConfig.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksConfig.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksConfig.cpp`.

- [ ] **Step 1: Add module declaration**

Add `pub mod emStocksConfig;` to `src/emStocks/mod.rs`.

- [ ] **Step 2: Write failing tests for ChartPeriod and Sorting enums**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_period_from_str() {
        assert_eq!(ChartPeriod::from_str("PT_1_WEEK"), Ok(ChartPeriod::Week1));
        assert_eq!(ChartPeriod::from_str("PT_1_YEAR"), Ok(ChartPeriod::Year1));
        assert_eq!(ChartPeriod::from_str("PT_20_YEARS"), Ok(ChartPeriod::Years20));
    }

    #[test]
    fn sorting_from_str() {
        assert_eq!(Sorting::from_str("SORT_BY_NAME"), Ok(Sorting::ByName));
        assert_eq!(Sorting::from_str("SORT_BY_DIFFERENCE"), Ok(Sorting::ByDifference));
    }

    #[test]
    fn config_record_round_trip() {
        let config = emStocksConfig::default();
        let serialized = config.to_rec();
        let deserialized = emStocksConfig::from_rec(&serialized).unwrap();
        assert_eq!(deserialized.chart_period, config.chart_period);
        assert_eq!(deserialized.sorting, config.sorting);
        assert_eq!(deserialized.web_browser, config.web_browser);
    }

    #[test]
    fn calculate_chart_period_days_fixed() {
        let config = emStocksConfig {
            chart_period: ChartPeriod::Week1,
            ..Default::default()
        };
        assert_eq!(config.CalculateChartPeriodDays("2024-06-15"), 7);
    }

    #[test]
    fn calculate_chart_period_days_month() {
        let config = emStocksConfig {
            chart_period: ChartPeriod::Month1,
            ..Default::default()
        };
        // 2024-05-15 to 2024-06-15
        let days = config.CalculateChartPeriodDays("2024-06-15");
        assert_eq!(days, 31); // May has 31 days
    }

    #[test]
    fn is_in_visible_categories_empty_means_all_visible() {
        let categories: Vec<String> = vec![];
        assert!(emStocksConfig::IsInVisibleCategories(&categories, "anything"));
    }

    #[test]
    fn is_in_visible_categories_binary_search() {
        let categories = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        assert!(emStocksConfig::IsInVisibleCategories(&categories, "B"));
        assert!(!emStocksConfig::IsInVisibleCategories(&categories, "D"));
    }
}
```

- [ ] **Step 3: Run to verify failure**

```bash
cargo-nextest ntr -E 'test(chart_period) | test(sorting) | test(config_record) | test(calculate_chart) | test(visible_categories)'
```

- [ ] **Step 4: Implement ChartPeriod and Sorting enums**

Port from C++ `PeriodType` and `SortingType`. Include `FromStr`, `Display`, enum values matching C++ identifiers.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChartPeriod {
    Week1, Weeks2, Month1, Months3, Months6,
    Year1, Years3, Years5, Years10, Years20,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sorting {
    ByName, ByTradeDate, ByInquiryDate, ByAchievement,
    ByOneWeekRise, ByThreeWeekRise, ByNineWeekRise,
    ByDividend, ByPurchaseValue, ByValue, ByDifference,
}
```

- [ ] **Step 5: Implement emStocksConfig struct and Record trait**

Port all fields from C++ constructor (emStocksConfig.cpp lines 107-153). Default values:
- `api_script_interpreter`: `"perl"`
- `web_browser`: `"firefox"`
- `chart_period`: `Year1`
- `min_visible_interest`: `Interest::Low`
- `sorting`: `ByName`
- Other strings: empty, bools: false

Implement `CalculateChartPeriodDays` and `IsInVisibleCategories` matching C++ exactly.

- [ ] **Step 6: Run tests**

```bash
cargo-nextest ntr -E 'test(chart_period) | test(sorting) | test(config_record) | test(calculate_chart) | test(visible_categories)'
```

- [ ] **Step 7: Run full suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 8: Commit**

```bash
git add src/emStocks/emStocksConfig.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksConfig with ChartPeriod, Sorting, and Record impl"
```

---

### Task 7: Port emStocksFileModel

**Files:**
- Create: `src/emStocks/emStocksFileModel.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksFileModel.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksFileModel.cpp`.

- [ ] **Step 1: Add module declaration**

Add `pub mod emStocksFileModel;` to `src/emStocks/mod.rs`.

- [ ] **Step 2: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_model_create() {
        // Verify emStocksFileModel can be constructed
        // Details depend on how emRecFileModel works — check existing
        // tests in tests/behavioral/file_model.rs for the pattern
    }

    #[test]
    fn file_model_has_cross_ptr_dialog_field() {
        // Verify PricesFetchingDialog field is emCrossPtr and starts invalid
        let model = create_test_file_model();
        assert!(!model.PricesFetchingDialog.is_valid());
    }
}
```

- [ ] **Step 3: Implement emStocksFileModel**

The C++ class uses multiple inheritance (emRecFileModel + emStocksRec + emRecListener). In Rust, compose these:

```rust
/// Port of C++ emStocksFileModel.
/// DIVERGED: Composition instead of multiple inheritance.
/// emRecFileModel<emStocksRec> handles file I/O.
/// emRecListener behavior via RecListenerList callback.
pub(crate) struct emStocksFileModel {
    pub rec: emStocksRec,
    // file_model handles load/save state machine
    // save_timer: managed via timer infrastructure
    pub PricesFetchingDialog: emCrossPtr<emStocksFetchPricesDialog>,
    // ... signal IDs, timer ID
}
```

Key behaviors to port:
- Constructor: `PostConstruct(*this)` → initialize file model with record reference
- `SetListenedRec(this)` → register listener callback that calls `OnRecChanged`
- `AddWakeUpSignal(SaveTimer.GetSignal())` → connect timer signal to engine
- `OnRecChanged()` → `SaveTimer.Start(15000)`
- `Cycle()` → check timer signal, call `Save(true)` if fired, delegate to base
- Destructor: if timer running, `Save(true)`

Study how existing `emRecFileModel` and `emConfigModel` are used in Rust (check `emCoreConfig.rs` for a working example).

- [ ] **Step 4: Run tests**

```bash
cargo-nextest ntr -E 'test(file_model)'
```

- [ ] **Step 5: Run full suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 6: Commit**

```bash
git add src/emStocks/emStocksFileModel.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksFileModel with save timer and emCrossPtr dialog"
```

---

### Task 8: Phase 1 behavioral tests

**Files:**
- Create: `tests/behavioral/emstocks_rec.rs`
- Modify: `tests/behavioral/main.rs` (add mod declaration)

- [ ] **Step 1: Write behavioral test for Record round-trip via file**

Test that an emStocksRec can be serialized to the emRec text format, written to a file, read back, and deserialized with identical data. This exercises the full emRec parser/writer pipeline.

```rust
#[test]
fn emstocks_rec_file_round_trip() {
    let mut rec = emStocksRec::default();
    let mut stock = StockRec::default();
    stock.id = "1".to_string();
    stock.name = "Test Corp".to_string();
    stock.prices = "100|101|102".to_string();
    stock.last_price_date = "2024-03-15".to_string();
    stock.interest = Interest::High;
    stock.web_pages = vec!["https://example.com".to_string()];
    rec.stocks.push(stock);

    // Serialize to emRec format
    let rec_struct = rec.to_rec();
    let text = write_rec_with_format(&rec_struct, "emStocks");

    // Parse back
    let parsed = parse_rec_with_format(&text, "emStocks").unwrap();
    let deserialized = emStocksRec::from_rec(&parsed).unwrap();

    assert_eq!(deserialized.stocks.len(), 1);
    assert_eq!(deserialized.stocks[0].prices, "100|101|102");
    assert_eq!(deserialized.stocks[0].interest, Interest::High);
}
```

- [ ] **Step 2: Write behavioral test for emCrossPtr in FileModel**

```rust
#[test]
fn file_model_cross_ptr_dialog_lifecycle() {
    // Simulate: create FileModel, create dialog, link via emCrossPtr,
    // drop dialog, verify FileModel sees invalid cross-ptr
}
```

- [ ] **Step 3: Run behavioral tests**

```bash
cargo-nextest ntr -E 'test(emstocks)'
```

- [ ] **Step 4: Commit**

```bash
git add tests/behavioral/emstocks_rec.rs tests/behavioral/main.rs
git commit -m "test(emStocks): add Phase 1 behavioral tests for Record round-trip"
```

---

## Phase 2: Engine Layer

### Task 9: Port emStocksPricesFetcher

**Files:**
- Create: `src/emStocks/emStocksPricesFetcher.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksPricesFetcher.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksPricesFetcher.cpp`.

This is the most complex type and the primary emCore stress-test. It uses emList<emCrossPtr<T>>, emAvlTreeMap<String, emCrossPtr<T>>, emProcess, and emEngine.

- [ ] **Step 1: Add module declaration**

Add `pub mod emStocksPricesFetcher;` to `src/emStocks/mod.rs`.

- [ ] **Step 2: Write failing tests for output line parsing**

The PricesFetcher reads stdout lines like `2024-03-15 100.50` from the API script. Port the `ProcessOutBufferLine` parsing logic and test it independently.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_output_line_valid() {
        let result = parse_price_line("2024-03-15 100.50");
        assert_eq!(result, Some(("2024-03-15".to_string(), "100.5".to_string())));
    }

    #[test]
    fn parse_output_line_with_whitespace() {
        let result = parse_price_line("  2024-03-15  100.50  ");
        assert_eq!(result, Some(("2024-03-15".to_string(), "100.5".to_string())));
    }

    #[test]
    fn parse_output_line_invalid() {
        assert_eq!(parse_price_line("not a date"), None);
        assert_eq!(parse_price_line("2024-03-15"), None); // no price
    }

    #[test]
    fn process_out_buffer_lines() {
        // Test that buffer with multiple lines + partial last line
        // correctly processes complete lines and retains partial
        let mut buf = "2024-03-14 99.0\n2024-03-15 100.5\n2024-03-".as_bytes().to_vec();
        let lines = extract_complete_lines(&mut buf);
        assert_eq!(lines.len(), 2);
        assert_eq!(buf, b"2024-03-");
    }
}
```

- [ ] **Step 3: Run to verify failure**

```bash
cargo-nextest ntr -E 'test(parse_output_line) | test(process_out_buffer)'
```

- [ ] **Step 4: Implement output parsing**

Port `ProcessOutBufferLines` and `ProcessOutBufferLine` from C++. The line parser:
1. Skips leading whitespace
2. Parses date as YYYY-MM-DD (three groups of digits separated by `-`)
3. Skips non-numeric chars after date
4. Parses price as float
5. Formats price via `SharePriceToString`

- [ ] **Step 5: Write failing tests for core fetcher struct**

```rust
#[test]
fn fetcher_add_stock_ids() {
    let mut fetcher = create_test_fetcher();
    fetcher.AddStockIds(&["1".to_string(), "2".to_string()]);
    assert!(!fetcher.HasFinished());
    assert_eq!(fetcher.GetProgressInPercent(), 25.0); // (0 + 0.5) * 100 / 2
}

#[test]
fn fetcher_has_finished_initially() {
    let fetcher = create_test_fetcher();
    assert!(fetcher.HasFinished());
}
```

- [ ] **Step 6: Implement emStocksPricesFetcher struct**

Port the full struct from C++ header. Key fields:

```rust
pub(crate) struct emStocksPricesFetcher {
    file_model: Rc<RefCell<emStocksFileModel>>,
    file_model_client: emAbsoluteFileModelClient<emStocksFileModel>,
    list_boxes: emList<emCrossPtr<emStocksListBox>>,
    api_script: String,
    api_script_interpreter: String,
    api_key: String,
    stock_ids: Vec<String>,  // emArray<emString> in C++
    stock_recs_map: emAvlTreeMap<String, emCrossPtr<StockRec>>,
    current_index: i32,
    current_symbol: String,
    current_start_date: String,
    current_process: Option<emProcess>,
    current_process_active: bool,
    current_stock_updated: bool,
    out_buffer: Vec<u8>,
    err_buffer: Vec<u8>,
    no_data_stocks: String,
    error: String,
    change_signal: SignalId,
}
```

Port all methods matching C++ names: `AddListBox`, `AddStockIds`, `GetCurrentStockId`, `GetCurrentStockRec`, `GetProgressInPercent`, `HasFinished`, `GetError`, `GetChangeSignal`, `Cycle`, `StartProcess`, `PollProcess`, `SetFailed`, `Clear`, `GetStockRec`, `UpdateStockRecsMapValues`, `CalculateDate`, `ProcessOutBufferLines`, `ProcessOutBufferLine`, `AddPrice`.

The `Cycle` implementation follows C++ exactly:
1. Check file model state (must be Loaded or Unsaved)
2. If process active, poll it
3. If process not active, start next one
4. Return whether process is active

- [ ] **Step 7: Run tests**

```bash
cargo-nextest ntr -E 'test(fetcher)'
```

- [ ] **Step 8: Run full suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 9: Commit**

```bash
git add src/emStocks/emStocksPricesFetcher.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksPricesFetcher with process management and parsing"
```

---

### Task 10: Phase 2 behavioral tests — COW + emCrossPtr stress

**Files:**
- Create: `tests/behavioral/emstocks_prices_fetcher.rs`
- Modify: `tests/behavioral/main.rs`

These tests are the primary emCore stress-tests.

- [ ] **Step 1: Write emList<emCrossPtr> invalidation test**

```rust
#[test]
fn emlist_crossptr_invalidation() {
    // Create a list of cross-pointers
    let target1 = Rc::new(RefCell::new(TestTarget::new()));
    let target2 = Rc::new(RefCell::new(TestTarget::new()));
    let mut list1 = target1.borrow().cross_ptr_list();
    let mut list2 = target2.borrow().cross_ptr_list();

    let mut list: emList<emCrossPtr<TestTarget>> = emList::new();
    list.Add(emCrossPtr::from_target(&target1, &mut list1));
    list.Add(emCrossPtr::from_target(&target2, &mut list2));

    // Both valid
    assert!(list.GetFirst().unwrap().is_valid());

    // Drop target1 — its cross-ptr should become invalid
    drop(target1);
    // First element invalid, second still valid
    let first = list.GetFirst().unwrap();
    assert!(!first.is_valid());
}
```

- [ ] **Step 2: Write emAvlTreeMap<String, emCrossPtr> test**

```rust
#[test]
fn avltreemap_crossptr_ordered_iteration_with_invalid() {
    let target_a = Rc::new(RefCell::new(TestTarget::new()));
    let target_b = Rc::new(RefCell::new(TestTarget::new()));
    // ... create cross-ptrs, insert into map with keys "a", "b"
    // Drop target_a
    // Iterate map in order — "a" entry has invalid cross-ptr, "b" is valid
    // Verify no crash, correct ordering preserved
}
```

- [ ] **Step 3: Write COW + emCrossPtr interaction test**

```rust
#[test]
fn cow_list_crossptr_clone_independence() {
    // Create emList with cross-pointer
    // Clone the list (COW — should share backing store)
    // Invalidate the cross-pointer
    // Both original and clone should see invalidation (shared flag)
    // Mutate clone (triggers COW deep copy)
    // Verify independence after COW clone
}
```

- [ ] **Step 4: Run behavioral tests**

```bash
cargo-nextest ntr -E 'test(emlist_crossptr) | test(avltreemap_crossptr) | test(cow_list_crossptr)'
```

- [ ] **Step 5: Commit**

```bash
git add tests/behavioral/emstocks_prices_fetcher.rs tests/behavioral/main.rs
git commit -m "test(emStocks): add Phase 2 behavioral tests for COW + emCrossPtr stress"
```

---

## Phase 3: UI Panels

### Task 11: Port emStocksItemChart

**Files:**
- Create: `src/emStocks/emStocksItemChart.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksItemChart.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksItemChart.cpp`.

- [ ] **Step 1: Add module declaration**

- [ ] **Step 2: Read the full C++ implementation**

Read `~/git/eaglemode-0.96.4/src/emStocks/emStocksItemChart.cpp` completely. Note the `PaintContent` method structure — it draws X/Y axes, price bars, desired price line, trade price line, and the price history graph. Note the `Price` nested struct and the data update flow.

- [ ] **Step 3: Implement emStocksItemChart**

Port the struct with all fields from C++ header. The `Price` nested struct:

```rust
#[derive(Debug, Clone, Copy, Default)]
struct Price {
    valid: bool,
    value: f64,
}
```

Port all methods:
- Constructor matching C++ field initialization
- `GetStockRec` / `SetStockRec`
- `Cycle` — manages `DataUpToDate` flag and `UpdateTimeout`
- `Notice` — reacts to panel state changes
- `PaintContent` — the main rendering method using emPainter
- Internal update methods: `UpdateData`, `UpdateLayout`
- Internal paint methods: `PaintXScale`, `PaintYScale`, `PaintPriceBars`, `PaintDesiredPriceLine`, `PaintTradePriceLine`, `PaintPriceGraph`

The painter calls use standard emPainter methods already available in Rust: `PaintLine`, `PaintRect`, `PaintRectOutline`, `PaintTextBoxed`.

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

- [ ] **Step 5: Add golden test for chart rendering**

If the C++ emStocks can be built and chart output captured, create a golden test reference. Otherwise, create a Rust-generated baseline:
1. Create a test stock with known price data
2. Render via emStocksItemChart into an emImage
3. Save as golden reference with PROVENANCE comment noting it's Rust-generated
4. Add to `tests/golden/` following existing golden test patterns

```bash
cargo test --test golden -- emstocks_item_chart
```

- [ ] **Step 6: Commit**

```bash
git add src/emStocks/emStocksItemChart.rs src/emStocks/mod.rs tests/golden/
git commit -m "feat(emStocks): port emStocksItemChart with price chart rendering"
```

---

### Task 12: Port emStocksItemPanel

**Files:**
- Create: `src/emStocks/emStocksItemPanel.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksItemPanel.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksItemPanel.cpp`.

- [ ] **Step 1: Add module declaration**

- [ ] **Step 2: Read the full C++ implementation**

Note the complex widget layout: text fields, checkboxes, radio buttons, nested CategoryPanel. Note the `emListBox::ItemPanelInterface` implementation.

- [ ] **Step 3: Implement emStocksItemPanel and CategoryPanel**

Port the struct, all widget fields (matching C++ field names per Name Correspondence), nested `CategoryPanel`. Key methods:
- Constructor with widget creation
- `GetStockRec` / `SetStockRec`
- `Cycle` — `UpdateControls` logic
- `Input` — button click handling
- `OnRecChanged` — listener callback
- `AutoExpand` / `AutoShrink` — panel lifecycle

CategoryPanel handles the country/sector/collection dropdown with autocomplete from existing values.

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add src/emStocks/emStocksItemPanel.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksItemPanel with CategoryPanel"
```

---

### Task 13: Port emStocksListBox

**Files:**
- Create: `src/emStocks/emStocksListBox.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksListBox.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksListBox.cpp`.

- [ ] **Step 1: Add module declaration**

- [ ] **Step 2: Read the full C++ implementation**

Note the sorting logic (comparison function with multiple sort keys), item management, clipboard operations, and date tracking via `SelectedDate` + `SelectedDateSignal`.

- [ ] **Step 3: Write failing test for sorting**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_stocks_by_name() {
        // Create test stocks with different names
        // Apply ByName sorting
        // Verify order
    }
}
```

- [ ] **Step 4: Implement emStocksListBox**

Port the full class. Key methods matching C++ names:
- Constructor
- `GetSelectedDate` / `SetSelectedDate` / `GetSelectedDateSignal`
- `GetStockCount` / `GetStock`
- `NewStock` / `CutStocks` / `CopyStocks` / `PasteStocks` / `DeleteStocks`
- `StartToFetchSharePrices`
- `SetInterest`
- `ShowWebPages`
- `FindSelected` / `FindNext` / `FindPrevious`
- `Cycle` / `Paint` / `CreateItemPanel`
- Internal `UpdateItems` and comparison function

- [ ] **Step 5: Run tests and full suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 6: Commit**

```bash
git add src/emStocks/emStocksListBox.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksListBox with sorting and stock management"
```

---

### Task 14: Port emStocksControlPanel

**Files:**
- Create: `src/emStocks/emStocksControlPanel.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksControlPanel.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksControlPanel.cpp`.

- [ ] **Step 1: Add module declaration**

- [ ] **Step 2: Implement emStocksControlPanel, FileFieldPanel, CategoryPanel**

This is the settings panel with many widgets. Port all fields and methods matching C++ names. Nested `FileFieldPanel` handles file path input with browser. Nested `CategoryPanel` handles visibility filtering.

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src/emStocks/emStocksControlPanel.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksControlPanel with FileFieldPanel and CategoryPanel"
```

---

### Task 15: Port emStocksFetchPricesDialog

**Files:**
- Create: `src/emStocks/emStocksFetchPricesDialog.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksFetchPricesDialog.h` and `~/git/eaglemode-0.96.4/src/emStocks/emStocksFetchPricesDialog.cpp`.

- [ ] **Step 1: Add module declaration**

- [ ] **Step 2: Implement emStocksFetchPricesDialog and ProgressBarPanel**

Port the dialog that wraps emStocksPricesFetcher. Nested `ProgressBarPanel` extends emBorder with custom `PaintContent` that draws a progress bar.

Key methods:
- Constructor creates Fetcher + Label + ProgressBar
- `AddListBox` / `AddStockIds` — delegate to fetcher
- `Cycle` — update label text and progress percentage
- `UpdateControls`

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src/emStocks/emStocksFetchPricesDialog.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksFetchPricesDialog with ProgressBarPanel"
```

---

### Task 16: Port emStocksFilePanel and emStocksFpPlugin

**Files:**
- Create: `src/emStocks/emStocksFilePanel.rs`
- Create: `src/emStocks/emStocksFpPlugin.rs`
- Modify: `src/emStocks/mod.rs`

C++ reference: `~/git/eaglemode-0.96.4/include/emStocks/emStocksFilePanel.h`, `~/git/eaglemode-0.96.4/src/emStocks/emStocksFilePanel.cpp`, `~/git/eaglemode-0.96.4/src/emStocks/emStocksFpPlugin.cpp`.

- [ ] **Step 1: Add module declarations**

- [ ] **Step 2: Implement emStocksFilePanel**

Port the top-level file panel. Key methods:
- Constructor
- `SetFileModel` — sets the file model and creates child panels
- `Cycle` — update controls based on file model state
- `Input` — keyboard shortcuts
- `IsOpaque` / `Paint` — background rendering
- `LayoutChildren` — positions ListBox and ControlPanel
- `CreateControlPanel` — creates the emStocksControlPanel

- [ ] **Step 3: Implement emStocksFpPlugin**

Port the plugin registration. Since dynamic loading isn't wired up, create a static registration function:

```rust
/// Port of C++ emStocksFpPluginFunc.
/// DIVERGED: Static registration instead of dynamic loading via extern "C".
pub(crate) fn register_emstocks_plugin(/* plugin list or context */) {
    // Register .emStocks file type handler
    // Creates emStocksFilePanel for matching files
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

- [ ] **Step 5: Run full suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 6: Commit**

```bash
git add src/emStocks/emStocksFilePanel.rs src/emStocks/emStocksFpPlugin.rs src/emStocks/mod.rs
git commit -m "feat(emStocks): port emStocksFilePanel and plugin registration"
```

---

## Phase 4: Integration & Polish

### Task 17: Integration tests

**Files:**
- Create or modify: `tests/integration/emstocks.rs`

- [ ] **Step 1: Create a test .emStocks file**

Create a minimal test fixture at `tests/fixtures/test.emStocks` containing valid emRec-format stock data (2-3 stocks with prices, different interests, etc.). Generate this by examining the C++ emStocks format — look at `emStocksRec::GetFormatName()` which returns `"emStocks"`.

- [ ] **Step 2: Write integration test for file load → record parse**

```rust
#[test]
fn load_emstocks_file() {
    let data = include_str!("../fixtures/test.emStocks");
    let rec_struct = parse_rec_with_format(data, "emStocks").unwrap();
    let stocks_rec = emStocksRec::from_rec(&rec_struct).unwrap();
    assert!(stocks_rec.stocks.len() >= 2);
    // Verify specific stock data
}
```

- [ ] **Step 3: Write integration test for full round-trip**

```rust
#[test]
fn emstocks_full_round_trip() {
    let data = include_str!("../fixtures/test.emStocks");
    let rec_struct = parse_rec_with_format(data, "emStocks").unwrap();
    let stocks_rec = emStocksRec::from_rec(&rec_struct).unwrap();

    // Modify
    let original_name = stocks_rec.stocks[0].name.clone();
    // ... modify and save back

    let output = write_rec_with_format(&stocks_rec.to_rec(), "emStocks");
    let re_parsed = parse_rec_with_format(&output, "emStocks").unwrap();
    let re_loaded = emStocksRec::from_rec(&re_parsed).unwrap();
    assert_eq!(re_loaded.stocks[0].name, original_name);
}
```

- [ ] **Step 4: Run integration tests**

```bash
cargo-nextest ntr -E 'test(emstocks)'
```

- [ ] **Step 5: Commit**

```bash
git add tests/integration/emstocks.rs tests/fixtures/test.emStocks
git commit -m "test(emStocks): add integration tests for file load and round-trip"
```

---

### Task 18: Update CORRESPONDENCE.md and final cleanup

**Files:**
- Modify: `docs/CORRESPONDENCE.md`
- Modify: `src/emStocks/mod.rs` (final module declarations)

- [ ] **Step 1: Update CORRESPONDENCE.md**

Add a new section documenting the emStocks port:

```markdown
## emStocks port (2026-03-29)

First outside-emCore app module ported. 10 C++ headers → 10 .rs files
in src/emStocks/. All types follow File and Name Correspondence.

emCore gaps closed:
- emAbsoluteFileModelClient added to emFileModel.rs

emCore types stress-tested:
- emList<emCrossPtr<T>> — COW list with cross-pointer values
- emAvlTreeMap<String, emCrossPtr<T>> — ordered map with cross-pointer values
- emArray<String> — COW array for stock ID queue
- emCrossPtr<T> — explicit invalidation for dialog and record tracking
- emProcess — external script spawning with pipe I/O
- emEngine — cooperative scheduling for price fetching
- emRecFileModel — file I/O state machine for stock data
- emConfigModel — configuration persistence
- emTimer — delayed save with signal integration
```

- [ ] **Step 2: Verify all module declarations are complete in mod.rs**

Ensure `src/emStocks/mod.rs` declares all 11 modules.

- [ ] **Step 3: Run full test suite one final time**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 4: Commit**

```bash
git add docs/CORRESPONDENCE.md src/emStocks/mod.rs
git commit -m "docs: update CORRESPONDENCE.md for emStocks port"
```

---

## Execution Notes

### Forward references

emStocksPricesFetcher (Task 9) references emStocksListBox which isn't ported until Task 13. Use a forward declaration pattern:
- In Task 9, declare emStocksListBox as a type alias or empty struct placeholder
- In Task 13, replace with the real implementation
- Alternatively, use `mod emStocksListBox;` with a minimal stub that compiles

### emCore gaps discovered during porting

If any emCore type is missing methods that emStocks needs, add them to the emCore type in its existing .rs file (per File and Name Correspondence), write tests, and commit separately before continuing the emStocks task.

### Test harness structure

Check how `tests/behavioral/main.rs` and `tests/unit/main.rs` include test modules. Follow the existing pattern. If they use `mod` declarations, add new modules there. If they use the `#[test]` attribute directly, create standalone test files.
