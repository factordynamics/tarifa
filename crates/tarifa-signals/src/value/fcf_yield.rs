//! Free cash flow yield value signal.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use tarifa_traits::{Date, MarketData, Result, Signal, TarifaError};

/// Configuration for free cash flow yield signal.
///
/// This signal computes the ratio of free cash flow to market capitalization.
/// Higher values indicate stocks generating strong cash flows relative to price.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeCashFlowYieldConfig {
    /// Whether to apply winsorization to handle extreme outliers (default: true)
    pub winsorize: bool,

    /// Winsorization percentile threshold (default: 0.01 for 1%/99% winsorization)
    pub winsorize_pct: f64,

    /// Use trailing twelve months (TTM) FCF (default: true)
    pub use_ttm: bool,
}

impl Default for FreeCashFlowYieldConfig {
    fn default() -> Self {
        Self {
            winsorize: true,
            winsorize_pct: 0.01,
            use_ttm: true,
        }
    }
}

/// Free cash flow yield value signal.
///
/// Computes the ratio of free cash flow to market capitalization.
/// This signal captures the cash-generating ability of a company relative
/// to its market value, providing a measure of value that is less susceptible
/// to accounting manipulation than earnings.
///
/// The signal applies:
/// 1. FCF-to-price ratio calculation
/// 2. Optional winsorization to handle extreme values
/// 3. Cross-sectional standardization
///
/// # Example
///
/// ```ignore
/// use tarifa_signals::value::FreeCashFlowYield;
///
/// let signal = FreeCashFlowYield::default();
/// let scores = signal.score(&market_data, date)?;
/// ```
#[derive(Debug, Clone)]
pub struct FreeCashFlowYield {
    config: FreeCashFlowYieldConfig,
}

impl FreeCashFlowYield {
    /// Create a new free cash flow yield signal with the given configuration.
    #[must_use]
    pub const fn new(config: FreeCashFlowYieldConfig) -> Self {
        Self { config }
    }

    /// Check if winsorization is enabled.
    #[must_use]
    pub const fn winsorize_enabled(&self) -> bool {
        self.config.winsorize
    }

    /// Get the winsorization percentile.
    #[must_use]
    pub const fn winsorize_pct(&self) -> f64 {
        self.config.winsorize_pct
    }

    /// Check if using trailing twelve months FCF.
    #[must_use]
    pub const fn use_ttm(&self) -> bool {
        self.config.use_ttm
    }

    /// Apply winsorization to a vector of values.
    fn winsorize(&self, values: &mut [f64]) {
        if !self.config.winsorize || values.is_empty() {
            return;
        }

        let mut sorted: Vec<f64> = values.iter().filter(|x| x.is_finite()).copied().collect();
        if sorted.is_empty() {
            return;
        }
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = sorted.len();
        let lower_idx = ((n as f64) * self.config.winsorize_pct).floor() as usize;
        let upper_idx = ((n as f64) * (1.0 - self.config.winsorize_pct)).ceil() as usize;
        let upper_idx = upper_idx.min(n - 1);

        let lower_bound = sorted[lower_idx];
        let upper_bound = sorted[upper_idx];

        for v in values.iter_mut() {
            if v.is_finite() {
                if *v < lower_bound {
                    *v = lower_bound;
                } else if *v > upper_bound {
                    *v = upper_bound;
                }
            }
        }
    }
}

impl Default for FreeCashFlowYield {
    fn default() -> Self {
        Self::new(FreeCashFlowYieldConfig::default())
    }
}

impl Signal for FreeCashFlowYield {
    fn name(&self) -> &str {
        "fcf_yield"
    }

    fn score(&self, data: &MarketData, date: Date) -> Result<DataFrame> {
        let df = data.data();

        // Validate required columns
        for col in self.required_columns() {
            if !data.has_column(col) {
                return Err(TarifaError::MissingColumn(col.to_string()));
            }
        }

        // Filter data up to and including the specified date
        let date_col = df.column("date")?;
        let mask = date_col
            .as_materialized_series()
            .date()?
            .into_iter()
            .map(|d: Option<i32>| {
                d.map(|d| chrono::NaiveDate::from_num_days_from_ce_opt(d + 719163).unwrap() <= date)
                    .unwrap_or(false)
            })
            .collect::<BooleanChunked>();

        let filtered = df.filter(&mask)?;

        if filtered.is_empty() {
            return Err(TarifaError::InsufficientData(format!(
                "No data available up to date {}",
                date
            )));
        }

        // Get unique symbols and their most recent data
        let symbols = filtered.column("symbol")?.as_materialized_series().str()?;

        let unique_symbols: Vec<String> = symbols
            .unique()?
            .into_iter()
            .filter_map(|s: Option<&str>| s.map(|s| s.to_string()))
            .collect();

        let mut result_symbols = Vec::with_capacity(unique_symbols.len());
        let mut result_ratios = Vec::with_capacity(unique_symbols.len());

        for symbol in &unique_symbols {
            // Get most recent data point for this symbol
            let symbol_mask = filtered
                .column("symbol")?
                .as_materialized_series()
                .str()?
                .equal(symbol.as_str());

            let symbol_data = filtered.filter(&symbol_mask)?;
            let sorted = symbol_data.sort(
                ["date"],
                SortMultipleOptions::default().with_order_descending(true),
            )?;

            if sorted.is_empty() {
                continue;
            }

            // Get most recent FCF and market cap
            let fcf = sorted
                .column("free_cash_flow")?
                .as_materialized_series()
                .f64()?
                .get(0);

            let market_cap = sorted
                .column("market_cap")?
                .as_materialized_series()
                .f64()?
                .get(0);

            match (fcf, market_cap) {
                (Some(f), Some(mc)) if mc > 0.0 && f.is_finite() => {
                    let ratio = f / mc;
                    result_symbols.push(symbol.clone());
                    result_ratios.push(ratio);
                }
                _ => continue,
            }
        }

        if result_symbols.is_empty() {
            return Err(TarifaError::InsufficientData(
                "No symbols have valid FCF and market cap data".to_string(),
            ));
        }

        // Apply winsorization
        self.winsorize(&mut result_ratios);

        // Cross-sectional standardization (z-score)
        let mean = result_ratios.iter().sum::<f64>() / result_ratios.len() as f64;
        let variance = result_ratios
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (result_ratios.len() - 1).max(1) as f64;
        let std = variance.sqrt();

        let standardized: Vec<f64> = if std > 1e-10 {
            result_ratios.iter().map(|x| (x - mean) / std).collect()
        } else {
            vec![0.0; result_ratios.len()]
        };

        // Build result DataFrame
        let result = df! {
            "symbol" => result_symbols,
            "score" => standardized,
        }?;

        Ok(result)
    }

    fn lookback(&self) -> usize {
        // Value signals typically use point-in-time data
        0
    }

    fn required_columns(&self) -> &[&str] {
        &["symbol", "date", "free_cash_flow", "market_cap"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_config() {
        let config = FreeCashFlowYieldConfig::default();
        assert!(config.winsorize);
        assert_relative_eq!(config.winsorize_pct, 0.01);
        assert!(config.use_ttm);
    }

    #[test]
    fn test_custom_config() {
        let config = FreeCashFlowYieldConfig {
            winsorize: false,
            winsorize_pct: 0.05,
            use_ttm: false,
        };
        let signal = FreeCashFlowYield::new(config);
        assert!(!signal.winsorize_enabled());
        assert_relative_eq!(signal.winsorize_pct(), 0.05);
        assert!(!signal.use_ttm());
    }

    #[test]
    fn test_default_signal() {
        let signal = FreeCashFlowYield::default();
        assert!(signal.winsorize_enabled());
        assert_relative_eq!(signal.winsorize_pct(), 0.01);
        assert!(signal.use_ttm());
    }
}
