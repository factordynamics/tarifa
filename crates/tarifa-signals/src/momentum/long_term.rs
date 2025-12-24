//! Long-term momentum signal based on 12-month returns, skipping the most recent month.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use tarifa_traits::{Date, MarketData, Result, Signal, TarifaError};

/// Configuration for long-term momentum signal.
///
/// This signal computes cumulative returns over a long lookback period,
/// typically 12 months (252 trading days), while skipping the most recent
/// month to avoid short-term reversal effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMomentumConfig {
    /// Number of trading days to look back (default: 252 days ≈ 12 months)
    pub lookback_days: usize,

    /// Number of recent days to skip to avoid reversal (default: 21 days ≈ 1 month)
    pub skip_days: usize,
}

impl Default for LongTermMomentumConfig {
    fn default() -> Self {
        Self {
            lookback_days: 252,
            skip_days: 21,
        }
    }
}

/// Long-term momentum signal.
///
/// Computes 12-month cumulative returns while skipping the most recent month.
/// This approach follows the classical momentum strategy that avoids short-term
/// reversal effects documented in the literature.
///
/// The signal computes returns from (t - lookback_days) to (t - skip_days).
///
/// # Example
///
/// ```ignore
/// use tarifa_signals::momentum::LongTermMomentum;
///
/// // Default: 12-month momentum skipping last month
/// let signal = LongTermMomentum::default();
/// let scores = signal.score(&market_data, date)?;
/// ```
#[derive(Debug, Clone)]
pub struct LongTermMomentum {
    config: LongTermMomentumConfig,
}

impl LongTermMomentum {
    /// Create a new long-term momentum signal with the given configuration.
    #[must_use]
    pub const fn new(config: LongTermMomentumConfig) -> Self {
        Self { config }
    }

    /// Get the lookback period in days.
    #[must_use]
    pub const fn lookback_days(&self) -> usize {
        self.config.lookback_days
    }

    /// Get the number of recent days to skip.
    #[must_use]
    pub const fn skip_days(&self) -> usize {
        self.config.skip_days
    }

    /// Get the effective lookback period (total lookback minus skipped days).
    #[must_use]
    pub const fn effective_lookback(&self) -> usize {
        self.config.lookback_days - self.config.skip_days
    }
}

impl Default for LongTermMomentum {
    fn default() -> Self {
        Self::new(LongTermMomentumConfig::default())
    }
}

impl Signal for LongTermMomentum {
    fn name(&self) -> &str {
        "long_term_momentum"
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

            // Long-term momentum skips the most recent skip_days
            // Need: lookback_days + 1 prices to compute return from lookback_days ago to skip_days ago
            if prices.len() < self.config.lookback_days + 1 {
                // Skip symbols without enough data
                continue;
            }

            let n = prices.len();
            // Price at (t - skip_days) - this is our "current" price for LT momentum
            let skip_idx = if self.config.skip_days > 0 && n > self.config.skip_days {
                n - 1 - self.config.skip_days
            } else {
                n - 1
            };

            // Price at (t - lookback_days)
            let lookback_idx = n - 1 - self.config.lookback_days;

            if skip_idx <= lookback_idx {
                // Not enough effective lookback
                continue;
            }

            let end_price = prices[skip_idx];
            let start_price = prices[lookback_idx];

            // Compute cumulative return
            let momentum = (end_price / start_price) - 1.0;

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
        let config = LongTermMomentumConfig::default();
        assert_eq!(config.lookback_days, 252);
        assert_eq!(config.skip_days, 21);
    }

    #[test]
    fn test_custom_config() {
        let config = LongTermMomentumConfig {
            lookback_days: 300,
            skip_days: 30,
        };
        let signal = LongTermMomentum::new(config);
        assert_eq!(signal.lookback_days(), 300);
        assert_eq!(signal.skip_days(), 30);
        assert_eq!(signal.effective_lookback(), 270);
    }

    #[test]
    fn test_default_signal() {
        let signal = LongTermMomentum::default();
        assert_eq!(signal.lookback_days(), 252);
        assert_eq!(signal.skip_days(), 21);
        assert_eq!(signal.effective_lookback(), 231);
    }
}
