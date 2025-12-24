//! Book-to-price value signal.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use tarifa_traits::{Date, MarketData, Result, Signal, TarifaError};

/// Configuration for book-to-price signal.
///
/// This signal computes the ratio of book value to market capitalization.
/// Higher values indicate potentially undervalued stocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookToPriceConfig {
    /// Whether to apply winsorization to handle extreme outliers (default: true)
    pub winsorize: bool,

    /// Winsorization percentile threshold (default: 0.01 for 1%/99% winsorization)
    pub winsorize_pct: f64,
}

impl Default for BookToPriceConfig {
    fn default() -> Self {
        Self {
            winsorize: true,
            winsorize_pct: 0.01,
        }
    }
}

/// Book-to-price value signal.
///
/// Computes the ratio of book value of equity to market capitalization.
/// This is a traditional value signal where higher values indicate stocks
/// trading below their book value.
///
/// The signal applies:
/// 1. Book-to-price ratio calculation
/// 2. Optional winsorization to handle extreme values
/// 3. Cross-sectional standardization
///
/// # Example
///
/// ```ignore
/// use tarifa_signals::value::BookToPrice;
///
/// let signal = BookToPrice::default();
/// let scores = signal.score(&market_data, date)?;
/// ```
#[derive(Debug, Clone)]
pub struct BookToPrice {
    config: BookToPriceConfig,
}

impl BookToPrice {
    /// Create a new book-to-price signal with the given configuration.
    #[must_use]
    pub const fn new(config: BookToPriceConfig) -> Self {
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

impl Default for BookToPrice {
    fn default() -> Self {
        Self::new(BookToPriceConfig::default())
    }
}

impl Signal for BookToPrice {
    fn name(&self) -> &str {
        "book_to_price"
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

            // Get most recent book value and market cap
            let book_value = sorted
                .column("book_value")?
                .as_materialized_series()
                .f64()?
                .get(0);

            let market_cap = sorted
                .column("market_cap")?
                .as_materialized_series()
                .f64()?
                .get(0);

            match (book_value, market_cap) {
                (Some(bv), Some(mc)) if mc > 0.0 && bv.is_finite() => {
                    let ratio = bv / mc;
                    result_symbols.push(symbol.clone());
                    result_ratios.push(ratio);
                }
                _ => continue,
            }
        }

        if result_symbols.is_empty() {
            return Err(TarifaError::InsufficientData(
                "No symbols have valid book value and market cap data".to_string(),
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
        &["symbol", "date", "book_value", "market_cap"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_config() {
        let config = BookToPriceConfig::default();
        assert!(config.winsorize);
        assert_relative_eq!(config.winsorize_pct, 0.01);
    }

    #[test]
    fn test_custom_config() {
        let config = BookToPriceConfig {
            winsorize: false,
            winsorize_pct: 0.05,
        };
        let signal = BookToPrice::new(config);
        assert!(!signal.winsorize_enabled());
        assert_relative_eq!(signal.winsorize_pct(), 0.05);
    }

    #[test]
    fn test_default_signal() {
        let signal = BookToPrice::default();
        assert!(signal.winsorize_enabled());
        assert_relative_eq!(signal.winsorize_pct(), 0.01);
    }

    #[test]
    fn test_winsorization() {
        let signal = BookToPrice::new(BookToPriceConfig {
            winsorize: true,
            winsorize_pct: 0.2, // 20% for testing
        });

        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        signal.winsorize(&mut values);

        // 20% winsorization on 10 values: lower bound at index 2, upper at index 8
        for v in &values {
            assert!(*v >= 3.0 && *v <= 9.0);
        }
    }
}
