//! Signal trait for generating trading signals.
//!
//! This module defines the `Signal` trait, which is the core abstraction for
//! computing scores for securities based on market data. Signals can represent
//! momentum indicators, value factors, technical patterns, or any other
//! quantitative measure used in trading strategies.

use crate::{Date, MarketData, Result};
use polars::prelude::*;

/// A trading signal that scores securities.
///
/// The `Signal` trait defines the interface for computing signal scores from
/// market data. Implementations should be thread-safe (`Send + Sync`) to enable
/// parallel computation.
///
/// # Signal Scores
///
/// Signal scores typically represent:
/// - Relative rankings of securities (e.g., momentum percentiles)
/// - Standardized values (e.g., z-scores)
/// - Binary indicators (e.g., 0/1 for pattern presence)
/// - Raw measurements (e.g., price-to-book ratios)
///
/// The interpretation depends on the specific signal implementation.
///
/// # Example
///
/// ```no_run
/// use tarifa_traits::{Signal, MarketData, Result, Date};
/// use polars::prelude::*;
///
/// struct SimpleSignal;
///
/// impl Signal for SimpleSignal {
///     fn name(&self) -> &str {
///         "simple_signal"
///     }
///
///     fn score(&self, data: &MarketData, date: Date) -> Result<DataFrame> {
///         // Compute signal scores
///         Ok(data.data().clone())
///     }
///
///     fn lookback(&self) -> usize {
///         20
///     }
///
///     fn required_columns(&self) -> &[&str] {
///         &["close"]
///     }
/// }
/// ```
pub trait Signal: Send + Sync {
    /// Returns the name of this signal.
    ///
    /// The name should be unique and descriptive, as it's used for
    /// identification in logging, analytics, and result storage.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::Signal;
    /// # struct MySignal;
    /// # impl Signal for MySignal {
    /// fn name(&self) -> &str {
    ///     "momentum_20d"
    /// }
    /// #     fn score(&self, data: &tarifa_traits::MarketData, date: tarifa_traits::Date) -> tarifa_traits::Result<polars::prelude::DataFrame> { todo!() }
    /// #     fn lookback(&self) -> usize { 20 }
    /// #     fn required_columns(&self) -> &[&str] { &[] }
    /// # }
    /// ```
    fn name(&self) -> &str;

    /// Computes signal scores for securities at a given date.
    ///
    /// # Arguments
    ///
    /// * `data` - Market data containing prices, volumes, and fundamentals
    /// * `date` - The date for which to compute scores
    ///
    /// # Returns
    ///
    /// Returns a DataFrame with at minimum:
    /// - A `symbol` column identifying each security
    /// - A `score` column containing the signal values
    ///
    /// Additional metadata columns may be included as needed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required columns are missing from the data
    /// - Insufficient historical data is available
    /// - Computation fails for any other reason
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{Signal, MarketData, Result, Date};
    /// # use polars::prelude::*;
    /// # struct MySignal;
    /// # impl Signal for MySignal {
    /// #     fn name(&self) -> &str { "test" }
    /// fn score(&self, data: &MarketData, date: Date) -> Result<DataFrame> {
    ///     // Filter data up to the specified date
    ///     // Compute signal logic
    ///     // Return DataFrame with symbol and score columns
    ///     todo!()
    /// }
    /// #     fn lookback(&self) -> usize { 20 }
    /// #     fn required_columns(&self) -> &[&str] { &[] }
    /// # }
    /// ```
    fn score(&self, data: &MarketData, date: Date) -> Result<DataFrame>;

    /// Returns the lookback period in days.
    ///
    /// The lookback period specifies how many days of historical data
    /// are required to compute this signal. For example, a 20-day momentum
    /// signal would return 20.
    ///
    /// This is used for:
    /// - Data validation and preparation
    /// - Determining when signals can first be computed
    /// - Memory management for rolling computations
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::Signal;
    /// # struct MySignal;
    /// # impl Signal for MySignal {
    /// #     fn name(&self) -> &str { "test" }
    /// #     fn score(&self, data: &tarifa_traits::MarketData, date: tarifa_traits::Date) -> tarifa_traits::Result<polars::prelude::DataFrame> { todo!() }
    /// fn lookback(&self) -> usize {
    ///     20  // Requires 20 days of history
    /// }
    /// #     fn required_columns(&self) -> &[&str] { &[] }
    /// # }
    /// ```
    fn lookback(&self) -> usize;

    /// Returns the required data columns for this signal.
    ///
    /// Lists the column names that must be present in the market data
    /// for this signal to compute successfully. Common columns include
    /// "close", "volume", "open", "high", "low", and fundamental fields.
    ///
    /// This is used for:
    /// - Data validation before computation
    /// - Optimizing data loading and filtering
    /// - Documentation and introspection
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::Signal;
    /// # struct MySignal;
    /// # impl Signal for MySignal {
    /// #     fn name(&self) -> &str { "test" }
    /// #     fn score(&self, data: &tarifa_traits::MarketData, date: tarifa_traits::Date) -> tarifa_traits::Result<polars::prelude::DataFrame> { todo!() }
    /// #     fn lookback(&self) -> usize { 20 }
    /// fn required_columns(&self) -> &[&str] {
    ///     &["close", "volume", "market_cap"]
    /// }
    /// # }
    /// ```
    fn required_columns(&self) -> &[&str];
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    struct TestSignal {
        name: String,
        lookback: usize,
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
            self.lookback
        }

        fn required_columns(&self) -> &[&str] {
            &["close", "volume"]
        }
    }

    #[test]
    fn test_signal_name() {
        let signal = TestSignal {
            name: "test_signal".to_string(),
            lookback: 20,
        };
        assert_eq!(signal.name(), "test_signal");
    }

    #[test]
    fn test_signal_lookback() {
        let signal = TestSignal {
            name: "test".to_string(),
            lookback: 30,
        };
        assert_eq!(signal.lookback(), 30);
    }

    #[test]
    fn test_signal_required_columns() {
        let signal = TestSignal {
            name: "test".to_string(),
            lookback: 20,
        };
        let cols = signal.required_columns();
        assert_eq!(cols.len(), 2);
        assert!(cols.contains(&"close"));
        assert!(cols.contains(&"volume"));
    }

    #[test]
    fn test_signal_score() {
        let signal = TestSignal {
            name: "test".to_string(),
            lookback: 20,
        };

        let df = df! {
            "symbol" => &["AAPL"],
            "close" => &[150.0],
            "volume" => &[1000000],
        }
        .unwrap();

        let market_data = MarketData::new(df);
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let result = signal.score(&market_data, date);
        assert!(result.is_ok());

        let scores = result.unwrap();
        assert_eq!(scores.height(), 2);
        assert!(scores.column("symbol").is_ok());
        assert!(scores.column("score").is_ok());
    }

    #[test]
    fn test_signal_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn Signal>>();
    }
}
