//! Short-term momentum signal based on 1-month returns.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use tarifa_traits::{Date, MarketData, Result, Signal, TarifaError};

/// Configuration for short-term momentum signal.
///
/// This signal computes cumulative returns over a short lookback period,
/// typically 1 month (21 trading days).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMomentumConfig {
    /// Number of trading days to look back (default: 21 days ≈ 1 month)
    pub lookback_days: usize,
}

impl Default for ShortTermMomentumConfig {
    fn default() -> Self {
        Self { lookback_days: 21 }
    }
}

/// Short-term momentum signal.
///
/// Computes 1-month cumulative returns and applies cross-sectional standardization.
/// Positive scores indicate stocks with strong recent performance.
///
/// # Example
///
/// ```ignore
/// use tarifa_signals::momentum::ShortTermMomentum;
///
/// let signal = ShortTermMomentum::default();
/// let scores = signal.score(&market_data, date)?;
/// ```
#[derive(Debug, Clone)]
pub struct ShortTermMomentum {
    config: ShortTermMomentumConfig,
}

impl ShortTermMomentum {
    /// Create a new short-term momentum signal with the given configuration.
    #[must_use]
    pub const fn new(config: ShortTermMomentumConfig) -> Self {
        Self { config }
    }

    /// Get the lookback period in days.
    #[must_use]
    pub const fn lookback_days(&self) -> usize {
        self.config.lookback_days
    }
}

impl Default for ShortTermMomentum {
    fn default() -> Self {
        Self::new(ShortTermMomentumConfig::default())
    }
}

impl Signal for ShortTermMomentum {
    fn name(&self) -> &str {
        "short_term_momentum"
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

        // Get unique symbols
        let symbols = filtered.column("symbol")?.as_materialized_series().str()?;

        let unique_symbols: Vec<String> = symbols
            .unique()?
            .into_iter()
            .filter_map(|s: Option<&str>| s.map(|s| s.to_string()))
            .collect();

        // Compute momentum for each symbol
        let mut result_symbols = Vec::with_capacity(unique_symbols.len());
        let mut result_scores = Vec::with_capacity(unique_symbols.len());

        for symbol in &unique_symbols {
            // Filter for this symbol and sort by date
            let symbol_mask = filtered
                .column("symbol")?
                .as_materialized_series()
                .str()?
                .equal(symbol.as_str());

            let symbol_data = filtered.filter(&symbol_mask)?;
            let sorted = symbol_data.sort(["date"], Default::default())?;

            let close_col = sorted.column("close")?.as_materialized_series().f64()?;

            let prices: Vec<f64> = close_col.into_iter().flatten().collect();

            if prices.len() < self.config.lookback_days + 1 {
                // Skip symbols without enough data
                continue;
            }

            // Get the lookback period prices
            let n = prices.len();
            let current_price = prices[n - 1];
            let past_price = prices[n - 1 - self.config.lookback_days];

            // Compute cumulative return
            let momentum = (current_price / past_price) - 1.0;

            result_symbols.push(symbol.clone());
            result_scores.push(momentum);
        }

        if result_symbols.is_empty() {
            return Err(TarifaError::InsufficientData(format!(
                "No symbols have {} days of data",
                self.lookback()
            )));
        }

        // Cross-sectional standardization (z-score)
        let mean = result_scores.iter().sum::<f64>() / result_scores.len() as f64;
        let variance = result_scores
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (result_scores.len() - 1).max(1) as f64;
        let std = variance.sqrt();

        let standardized: Vec<f64> = if std > 1e-10 {
            result_scores.iter().map(|x| (x - mean) / std).collect()
        } else {
            vec![0.0; result_scores.len()]
        };

        // Build result DataFrame
        let result = df! {
            "symbol" => result_symbols,
            "score" => standardized,
        }?;

        Ok(result)
    }

    fn lookback(&self) -> usize {
        self.config.lookback_days
    }

    fn required_columns(&self) -> &[&str] {
        &["symbol", "date", "close"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ShortTermMomentumConfig::default();
        assert_eq!(config.lookback_days, 21);
    }

    #[test]
    fn test_custom_config() {
        let config = ShortTermMomentumConfig { lookback_days: 30 };
        let signal = ShortTermMomentum::new(config);
        assert_eq!(signal.lookback_days(), 30);
    }

    #[test]
    fn test_default_signal() {
        let signal = ShortTermMomentum::default();
        assert_eq!(signal.lookback_days(), 21);
    }
}
