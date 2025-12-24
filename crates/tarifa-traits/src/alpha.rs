//! Alpha model trait for generating expected returns.
//!
//! This module defines the `AlphaModel` trait, which combines multiple signals
//! to produce expected return forecasts for securities. Alpha models are the
//! core predictive component of quantitative trading strategies.

use crate::{Date, Result, Signal, Symbol};
use ndarray::Array1;
use polars::prelude::*;

/// An alpha model that generates expected return forecasts.
///
/// The `AlphaModel` trait defines the interface for combining signals into
/// actionable predictions. Implementations aggregate multiple signals,
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
/// use tarifa_traits::{AlphaModel, Signal, Symbol, Date, Result};
/// use ndarray::Array1;
/// use polars::prelude::*;
///
/// struct SimpleAlpha {
///     signals: Vec<Box<dyn Signal>>,
/// }
///
/// impl AlphaModel for SimpleAlpha {
///     fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>> {
///         // Combine signals to produce expected returns
///         Ok(Array1::zeros(universe.len()))
///     }
///
///     fn signal_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> {
///         // Return raw signal scores
///         Ok(DataFrame::default())
///     }
///
///     fn signals(&self) -> Vec<&dyn Signal> {
///         self.signals.iter().map(|s| s.as_ref()).collect()
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
    /// - Signal computation fails
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
    ///     // Compute signals for each symbol
    ///     // Combine signals using model weights
    ///     // Return expected returns vector
    ///     Ok(Array1::zeros(universe.len()))
    /// }
    /// #     fn signal_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> { Ok(DataFrame::default()) }
    /// #     fn signals(&self) -> Vec<&dyn tarifa_traits::Signal> { vec![] }
    /// # }
    /// ```
    fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>>;

    /// Returns the raw signal scores for a universe of securities.
    ///
    /// This method provides access to the underlying signal values before
    /// they are combined into expected returns. This is useful for:
    /// - Signal analysis and debugging
    /// - Custom combination strategies
    /// - Performance attribution
    ///
    /// # Arguments
    ///
    /// * `universe` - Slice of symbols to compute signals for
    /// * `date` - The date as of which to compute signals
    ///
    /// # Returns
    ///
    /// Returns a DataFrame with columns:
    /// - `symbol`: Security identifier
    /// - One column per signal containing its scores
    ///
    /// # Errors
    ///
    /// Returns an error if signal computation fails.
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
    /// fn signal_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> {
    ///     // Compute each signal
    ///     // Join results into a single DataFrame
    ///     // Return with one column per signal
    ///     Ok(DataFrame::default())
    /// }
    /// #     fn signals(&self) -> Vec<&dyn tarifa_traits::Signal> { vec![] }
    /// # }
    /// ```
    fn signal_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame>;

    /// Returns references to the signals used by this alpha model.
    ///
    /// This provides introspection into the model's components, allowing
    /// for analysis, documentation, and validation of the signal set.
    ///
    /// # Returns
    ///
    /// Returns a vector of trait object references to the component signals.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{AlphaModel, Signal, Symbol, Date, Result};
    /// # use ndarray::Array1;
    /// # use polars::prelude::*;
    /// # struct MyAlpha { signals: Vec<Box<dyn Signal>> }
    /// # impl AlphaModel for MyAlpha {
    /// #     fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>> { Ok(Array1::zeros(0)) }
    /// #     fn signal_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame> { Ok(DataFrame::default()) }
    /// fn signals(&self) -> Vec<&dyn Signal> {
    ///     self.signals.iter().map(|s| s.as_ref()).collect()
    /// }
    /// # }
    /// ```
    fn signals(&self) -> Vec<&dyn Signal>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MarketData, Signal};
    use chrono::NaiveDate;

    struct TestSignal {
        name: String,
    }

    impl Signal for TestSignal {
        fn name(&self) -> &str {
            &self.name
        }

        fn score(&self, _data: &MarketData, _date: Date) -> Result<DataFrame> {
            Ok(df! {
                "symbol" => &["AAPL", "MSFT"],
                "score" => &[0.5, -0.3],
            }
            .unwrap())
        }

        fn lookback(&self) -> usize {
            20
        }

        fn required_columns(&self) -> &[&str] {
            &["close"]
        }
    }

    struct TestAlpha {
        signals: Vec<Box<dyn Signal>>,
    }

    impl AlphaModel for TestAlpha {
        fn expected_returns(&self, universe: &[Symbol], _date: Date) -> Result<Array1<f64>> {
            Ok(Array1::zeros(universe.len()))
        }

        fn signal_scores(&self, universe: &[Symbol], _date: Date) -> Result<DataFrame> {
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

        fn signals(&self) -> Vec<&dyn Signal> {
            self.signals.iter().map(|s| s.as_ref()).collect()
        }
    }

    #[test]
    fn test_alpha_expected_returns() {
        let alpha = TestAlpha { signals: vec![] };
        let universe = vec!["AAPL".to_string(), "MSFT".to_string()];
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = alpha.expected_returns(&universe, date);
        assert!(result.is_ok());

        let returns = result.unwrap();
        assert_eq!(returns.len(), 2);
    }

    #[test]
    fn test_alpha_signal_scores() {
        let alpha = TestAlpha { signals: vec![] };
        let universe = vec!["AAPL".to_string()];
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = alpha.signal_scores(&universe, date);
        assert!(result.is_ok());

        let scores = result.unwrap();
        assert_eq!(scores.height(), 1);
        assert!(scores.column("symbol").is_ok());
    }

    #[test]
    fn test_alpha_signals() {
        let signal1 = Box::new(TestSignal {
            name: "signal1".to_string(),
        });
        let signal2 = Box::new(TestSignal {
            name: "signal2".to_string(),
        });

        let alpha = TestAlpha {
            signals: vec![signal1, signal2],
        };

        let signals = alpha.signals();
        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].name(), "signal1");
        assert_eq!(signals[1].name(), "signal2");
    }

    #[test]
    fn test_alpha_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn AlphaModel>>();
    }

    #[test]
    fn test_empty_universe() {
        let alpha = TestAlpha { signals: vec![] };
        let universe: Vec<Symbol> = vec![];
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = alpha.expected_returns(&universe, date);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
