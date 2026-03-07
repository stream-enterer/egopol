use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaxRates {
    pub income: f64,
    pub corporate: f64,
    pub vat: f64,
    pub social_employer: f64,
    pub social_employee: f64,
    pub export: f64,
    pub capital_formation: f64,
}

impl TaxRates {
    /// Returns `true` if any rate is non-zero (i.e. overrides the model's calibration).
    pub fn has_overrides(&self) -> bool {
        self.income != 0.0
            || self.corporate != 0.0
            || self.vat != 0.0
            || self.social_employer != 0.0
            || self.social_employee != 0.0
            || self.export != 0.0
            || self.capital_formation != 0.0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EconSnapshot {
    pub quarter: u32,
    pub real_gdp: f64,
    pub nominal_gdp: f64,
    pub real_gdp_growth: f64,
    pub nominal_gdp_growth: f64,
    pub inflation: f64,
    pub unemployment: f64,
    pub euribor: f64,
    pub government_spending: f64,
    pub government_revenue: f64,
    pub government_debt: f64,
    pub consumption: f64,
    pub investment: f64,
    pub exports: f64,
    pub imports: f64,
    pub wage_growth: f64,
    pub price_level: f64,
    pub money_supply: f64,
    pub bank_deposits: f64,
    pub bank_loans: f64,
    pub equity_index: f64,
    pub housing_price: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EconState {
    pub tax_rates: TaxRates,
    pub history: Vec<EconSnapshot>,
}

impl EconState {
    pub fn latest(&self) -> Option<&EconSnapshot> {
        self.history.last()
    }
}
