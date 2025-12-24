//! Profit margins quality signal.

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use tarifa_traits::{Date, MarketData, Result, Signal, TarifaError};

/// Type of profit margin to compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarginType {
    /// Gross margin: (revenue - COGS) / revenue
    Gross,
    /// Operating margin: operating_income / revenue
    Operating,
    /// Net margin: net_income / revenue
    Net,
}

/// Configuration for profit margins signal.
///
/// Profit margins measure operational efficiency at different stages
/// of the income statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitMarginsConfig {
    /// Type of margin to compute (default: Operating)
    pub margin_type: MarginType,

    /// Whether to apply winsorization to handle extreme outliers (default: true)
    pub winsorize: bool,

    /// Winsorization percentile threshold (default: 0.01 for 1%/99% winsorization)
    pub winsorize_pct: f64,

    /// Use trailing twelve months (TTM) metrics (default: true)
    pub use_ttm: bool,
}

impl Default for ProfitMarginsConfig {
    fn default() -> Self {
        Self {
            margin_type: MarginType::Operating,
            winsorize: true,
            winsorize_pct: 0.01,
            use_ttm: true,
        }
    }
}

/// Profit margins quality signal.
///
/// Computes profit margins at various stages of the income statement:
/// - Gross margin: Measures efficiency in production/service delivery
/// - Operating margin: Measures core business profitability
/// - Net margin: Measures overall profitability after all expenses
///
/// The signal applies:
/// 1. Margin calculation (profit / revenue)
/// 2. Optional winsorization to handle extreme values
/// 3. Cross-sectional standardization
///
/// # Example
///
/// ```ignore
/// use tarifa_signals::quality::{ProfitMargins, ProfitMarginsConfig, MarginType};
///
/// // Operating margin (default)
/// let signal = ProfitMargins::default();
///
/// // Gross margin
/// let config = ProfitMarginsConfig {
///     margin_type: MarginType::Gross,
///     ..Default::default()
/// };
/// let gross_margin_signal = ProfitMargins::new(config);
/// ```
#[derive(Debug, Clone)]
pub struct ProfitMargins {
    config: ProfitMarginsConfig,
}

impl ProfitMargins {
    /// Create a new profit margins signal with the given configuration.
    #[must_use]
    pub const fn new(config: ProfitMarginsConfig) -> Self {
        Self { config }
    }

    /// Get the margin type.
    #[must_use]
    pub const fn margin_type(&self) -> MarginType {
        self.config.margin_type
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

    /// Check if using trailing twelve months metrics.
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

    /// Get the numerator column name based on margin type.
    const fn numerator_column(&self) -> &str {
        match self.config.margin_type {
            MarginType::Gross => "gross_profit",
            MarginType::Operating => "operating_income",
            MarginType::Net => "net_income",
        }
    }
}

impl Default for ProfitMargins {
    fn default() -> Self {
        Self::new(ProfitMarginsConfig::default())
    }
}

impl Signal for ProfitMargins {
    fn name(&self) -> &str {
        match self.config.margin_type {
            MarginType::Gross => "gross_margin",
            MarginType::Operating => "operating_margin",
            MarginType::Net => "net_margin",
        }
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

        let numerator_col = self.numerator_column();

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

            // Get numerator (gross_profit, operating_income, or net_income)
            let numerator = sorted
                .column(numerator_col)?
                .as_materialized_series()
                .f64()?
                .get(0);

            let revenue = sorted
                .column("revenue")?
                .as_materialized_series()
                .f64()?
                .get(0);

            match (numerator, revenue) {
                (Some(num), Some(rev)) if rev.abs() > 1e-10 && num.is_finite() => {
                    let margin = num / rev;
                    result_symbols.push(symbol.clone());
                    result_ratios.push(margin);
                }
                _ => continue,
            }
        }

        if result_symbols.is_empty() {
            return Err(TarifaError::InsufficientData(format!(
                "No symbols have valid {} and revenue data",
                numerator_col
            )));
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
        0
    }

    fn required_columns(&self) -> &[&str] {
        match self.config.margin_type {
            MarginType::Gross => &["symbol", "date", "gross_profit", "revenue"],
            MarginType::Operating => &["symbol", "date", "operating_income", "revenue"],
            MarginType::Net => &["symbol", "date", "net_income", "revenue"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_default_config() {
        let config = ProfitMarginsConfig::default();
        assert_eq!(config.margin_type, MarginType::Operating);
        assert!(config.winsorize);
        assert_relative_eq!(config.winsorize_pct, 0.01);
        assert!(config.use_ttm);
    }

    #[test]
    fn test_custom_config() {
        let config = ProfitMarginsConfig {
            margin_type: MarginType::Gross,
            winsorize: false,
            winsorize_pct: 0.05,
            use_ttm: false,
        };
        let signal = ProfitMargins::new(config);
        assert_eq!(signal.margin_type(), MarginType::Gross);
        assert!(!signal.winsorize_enabled());
        assert_relative_eq!(signal.winsorize_pct(), 0.05);
        assert!(!signal.use_ttm());
    }

    #[test]
    fn test_default_signal() {
        let signal = ProfitMargins::default();
        assert_eq!(signal.margin_type(), MarginType::Operating);
        assert!(signal.winsorize_enabled());
        assert_relative_eq!(signal.winsorize_pct(), 0.01);
        assert!(signal.use_ttm());
    }

    #[test]
    fn test_margin_types() {
        let gross = ProfitMarginsConfig {
            margin_type: MarginType::Gross,
            ..Default::default()
        };
        let operating = ProfitMarginsConfig {
            margin_type: MarginType::Operating,
            ..Default::default()
        };
        let net = ProfitMarginsConfig {
            margin_type: MarginType::Net,
            ..Default::default()
        };

        assert_eq!(ProfitMargins::new(gross).margin_type(), MarginType::Gross);
        assert_eq!(
            ProfitMargins::new(operating).margin_type(),
            MarginType::Operating
        );
        assert_eq!(ProfitMargins::new(net).margin_type(), MarginType::Net);
    }

    #[test]
    fn test_signal_names() {
        let gross = ProfitMargins::new(ProfitMarginsConfig {
            margin_type: MarginType::Gross,
            ..Default::default()
        });
        let operating = ProfitMargins::new(ProfitMarginsConfig {
            margin_type: MarginType::Operating,
            ..Default::default()
        });
        let net = ProfitMargins::new(ProfitMarginsConfig {
            margin_type: MarginType::Net,
            ..Default::default()
        });

        assert_eq!(gross.name(), "gross_margin");
        assert_eq!(operating.name(), "operating_margin");
        assert_eq!(net.name(), "net_margin");
    }
}
