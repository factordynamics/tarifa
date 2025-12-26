//! Alpha model trait for generating expected returns.
//!
//! This module defines the `AlphaModel` trait, which combines multiple factors
//! to produce expected return forecasts for securities. Alpha models are the
//! core predictive component of quantitative trading strategies.

use crate::{Date, Result, Symbol};
use factors::Factor;
use ndarray::Array1;
use polars::prelude::*;

/// An alpha model that generates expected return forecasts.
///
/// The `AlphaModel` trait defines the interface for combining factors into
/// actionable predictions. Implementations aggregate multiple factors,
/// potentially with different weights or combination methods, to produce
/// expected returns for a universe of securities.
///
/// # Expected Returns
///
/// Expected returns represent the model's forecast of future returns,
/// typically over a specific horizon (e.g., 1 day, 5 days, 20 days).
/// These forecasts drive portfolio construction and position sizing.
///
/// # Example
///
/// ```no_run
/// use tarifa_traits::{AlphaModel, Symbol, Date, Result};
/// use factors::Factor;
/// use ndarray::Array1;
/// use polars::prelude::*;
///
/// struct SimpleAlpha {
///     factors: Vec<Box<dyn Factor>>,
/// }
///
/// impl AlphaModel for SimpleAlpha {
///     fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>> {
///         // Combine factors to produce expected returns
///         Ok(Array1::zeros(universe.len()))
///     }
///
///     fn factor_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> {
///         // Return raw factor scores
///         Ok(DataFrame::default())
///     }
///
///     fn factors(&self) -> Vec<&dyn Factor> {
///         self.factors.iter().map(|s| s.as_ref()).collect()
///     }
/// }
/// ```
pub trait AlphaModel: Send + Sync {
    /// Generates expected returns for a universe of securities.
    ///
    /// This is the primary output of an alpha model, producing a vector of
    /// expected returns aligned with the input universe.
    ///
    /// # Arguments
    ///
    /// * `universe` - Slice of symbols to generate forecasts for
    /// * `date` - The date as of which to compute expected returns
    ///
    /// # Returns
    ///
    /// Returns an array of expected returns with length equal to `universe.len()`.
    /// The i-th element corresponds to the expected return for `universe[i]`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Factor computation fails
    /// - Insufficient data is available
    /// - Any symbol in the universe is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{AlphaModel, Symbol, Date, Result};
    /// # use ndarray::Array1;
    /// # use polars::prelude::*;
    /// # struct MyAlpha;
    /// # impl AlphaModel for MyAlpha {
    /// fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>> {
    ///     // Compute factors for each symbol
    ///     // Combine factors using model weights
    ///     // Return expected returns vector
    ///     Ok(Array1::zeros(universe.len()))
    /// }
    /// #     fn factor_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> { Ok(DataFrame::default()) }
    /// #     fn factors(&self) -> Vec<&dyn factors::Factor> { vec![] }
    /// # }
    /// ```
    fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>>;

    /// Returns the raw factor scores for a universe of securities.
    ///
    /// This method provides access to the underlying factor values before
    /// they are combined into expected returns. This is useful for:
    /// - Factor analysis and debugging
    /// - Custom combination strategies
    /// - Performance attribution
    ///
    /// # Arguments
    ///
    /// * `universe` - Slice of symbols to compute factors for
    /// * `date` - The date as of which to compute factors
    ///
    /// # Returns
    ///
    /// Returns a DataFrame with columns:
    /// - `symbol`: Security identifier
    /// - One column per factor containing its scores
    ///
    /// # Errors
    ///
    /// Returns an error if factor computation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{AlphaModel, Symbol, Date, Result};
    /// # use ndarray::Array1;
    /// # use polars::prelude::*;
    /// # struct MyAlpha;
    /// # impl AlphaModel for MyAlpha {
    /// #     fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>> { Ok(Array1::zeros(0)) }
    /// fn factor_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> {
    ///     // Compute each factor
    ///     // Join results into a single DataFrame
    ///     // Return with one column per factor
    ///     Ok(DataFrame::default())
    /// }
    /// #     fn factors(&self) -> Vec<&dyn factors::Factor> { vec![] }
    /// # }
    /// ```
    fn factor_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame>;

    /// Returns references to the factors used by this alpha model.
    ///
    /// This provides introspection into the model's components, allowing
    /// for analysis, documentation, and validation of the factor set.
    ///
    /// # Returns
    ///
    /// Returns a vector of trait object references to the component factors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{AlphaModel, Symbol, Date, Result};
    /// # use factors::Factor;
    /// # use ndarray::Array1;
    /// # use polars::prelude::*;
    /// # struct MyAlpha { factors: Vec<Box<dyn Factor>> }
    /// # impl AlphaModel for MyAlpha {
    /// #     fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>> { Ok(Array1::zeros(0)) }
    /// #     fn factor_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> { Ok(DataFrame::default()) }
    /// fn factors(&self) -> Vec<&dyn Factor> {
    ///     self.factors.iter().map(|s| s.as_ref()).collect()
    /// }
    /// # }
    /// ```
    fn factors(&self) -> Vec<&dyn Factor>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use factors::{DataFrequency, FactorCategory};

    #[derive(Debug)]
    struct TestFactor {
        name: String,
    }

    impl Factor for TestFactor {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "Test factor for unit tests"
        }

        fn category(&self) -> FactorCategory {
            FactorCategory::Momentum
        }

        fn required_columns(&self) -> &[&str] {
            &["close"]
        }

        fn lookback(&self) -> usize {
            20
        }

        fn frequency(&self) -> DataFrequency {
            DataFrequency::Daily
        }

        fn compute_raw(&self, _data: &LazyFrame, _date: NaiveDate) -> factors::Result<DataFrame> {
            Ok(df! {
                "symbol" => &["AAPL", "MSFT"],
                "date" => &[NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); 2],
                "test_factor" => &[0.5, -0.3],
            }
            .unwrap())
        }
    }

    struct TestAlpha {
        factors: Vec<Box<dyn Factor>>,
    }

    impl AlphaModel for TestAlpha {
        fn expected_returns(&self, universe: &[Symbol], _date: Date) -> Result<Array1<f64>> {
            Ok(Array1::zeros(universe.len()))
        }

        fn factor_scores(&self, universe: &[Symbol], _date: Date) -> Result<DataFrame> {
            let mut symbols = Vec::new();
            let mut scores = Vec::new();

            for symbol in universe {
                symbols.push(symbol.clone());
                scores.push(0.0);
            }

            Ok(df! {
                "symbol" => symbols,
                "score" => scores,
            }
            .unwrap())
        }

        fn factors(&self) -> Vec<&dyn Factor> {
            self.factors.iter().map(|s| s.as_ref()).collect()
        }
    }

    #[test]
    fn test_alpha_expected_returns() {
        let alpha = TestAlpha { factors: vec![] };
        let universe = vec!["AAPL".to_string(), "MSFT".to_string()];
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = alpha.expected_returns(&universe, date);
        assert!(result.is_ok());

        let returns = result.unwrap();
        assert_eq!(returns.len(), 2);
    }

    #[test]
    fn test_alpha_factor_scores() {
        let alpha = TestAlpha { factors: vec![] };
        let universe = vec!["AAPL".to_string()];
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = alpha.factor_scores(&universe, date);
        assert!(result.is_ok());

        let scores = result.unwrap();
        assert_eq!(scores.height(), 1);
        assert!(scores.column("symbol").is_ok());
    }

    #[test]
    fn test_alpha_factors() {
        let factor1 = Box::new(TestFactor {
            name: "factor1".to_string(),
        });
        let factor2 = Box::new(TestFactor {
            name: "factor2".to_string(),
        });

        let alpha = TestAlpha {
            factors: vec![factor1, factor2],
        };

        let factors = alpha.factors();
        assert_eq!(factors.len(), 2);
        assert_eq!(factors[0].name(), "factor1");
        assert_eq!(factors[1].name(), "factor2");
    }

    #[test]
    fn test_alpha_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn AlphaModel>>();
    }

    #[test]
    fn test_empty_universe() {
        let alpha = TestAlpha { factors: vec![] };
        let universe: Vec<Symbol> = vec![];
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = alpha.expected_returns(&universe, date);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
