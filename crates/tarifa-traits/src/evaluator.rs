//! Signal evaluator trait for assessing signal quality.
//!
//! This module defines the `SignalEvaluator` trait, which provides methods
//! for evaluating the predictive power and characteristics of trading signals.
//! Common metrics include information coefficient (IC), information ratio (IR),
//! and turnover.

use crate::Signal;

/// Evaluates the quality and characteristics of trading signals.
///
/// The `SignalEvaluator` trait defines the interface for assessing signal
/// performance using standard quantitative finance metrics. These evaluations
/// help determine which signals to include in an alpha model and how to
/// weight them.
///
/// # Metrics
///
/// - **Information Coefficient (IC)**: Correlation between signal scores and
///   future returns, measuring predictive power
/// - **Information Ratio (IR)**: Mean IC divided by standard deviation of IC,
///   measuring consistency of predictions
/// - **Turnover**: Rate at which signal rankings change, affecting
///   transaction costs
///
/// # Example
///
/// ```no_run
/// use tarifa_traits::{SignalEvaluator, Signal};
///
/// struct SimpleEvaluator;
///
/// impl SignalEvaluator for SimpleEvaluator {
///     fn ic(&self, signal: &dyn Signal, horizon: usize) -> f64 {
///         // Compute information coefficient
///         0.05
///     }
///
///     fn ir(&self, signal: &dyn Signal, horizon: usize) -> f64 {
///         // Compute information ratio
///         0.5
///     }
///
///     fn turnover(&self, signal: &dyn Signal) -> f64 {
///         // Compute signal turnover
///         0.3
///     }
/// }
/// ```
pub trait SignalEvaluator {
    /// Computes the information coefficient for a signal.
    ///
    /// The IC measures the correlation between signal scores and future
    /// returns over a specified horizon. Higher absolute values indicate
    /// stronger predictive power.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to evaluate
    /// * `horizon` - Forward-looking period in days (e.g., 1, 5, 20)
    ///
    /// # Returns
    ///
    /// Returns the IC as a correlation coefficient, typically in [-1, 1].
    /// - Positive values: Signal predicts positive returns
    /// - Negative values: Signal predicts negative returns (can be inverted)
    /// - Zero: No predictive power
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{SignalEvaluator, Signal};
    /// # struct MyEvaluator;
    /// # impl SignalEvaluator for MyEvaluator {
    /// fn ic(&self, signal: &dyn Signal, horizon: usize) -> f64 {
    ///     // Correlate signal scores with forward returns
    ///     // at the specified horizon
    ///     0.08  // Example: 8% correlation
    /// }
    /// #     fn ir(&self, signal: &dyn Signal, horizon: usize) -> f64 { 0.0 }
    /// #     fn turnover(&self, signal: &dyn Signal) -> f64 { 0.0 }
    /// # }
    /// ```
    fn ic(&self, signal: &dyn Signal, horizon: usize) -> f64;

    /// Computes the information ratio for a signal.
    ///
    /// The IR measures the consistency of a signal's predictive power by
    /// dividing mean IC by the standard deviation of IC over time. Higher
    /// values indicate more reliable signals.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to evaluate
    /// * `horizon` - Forward-looking period in days
    ///
    /// # Returns
    ///
    /// Returns the IR, where:
    /// - IR > 0.5: Strong, consistent signal
    /// - IR > 0.3: Moderate signal quality
    /// - IR < 0.3: Weak or inconsistent signal
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{SignalEvaluator, Signal};
    /// # struct MyEvaluator;
    /// # impl SignalEvaluator for MyEvaluator {
    /// #     fn ic(&self, signal: &dyn Signal, horizon: usize) -> f64 { 0.0 }
    /// fn ir(&self, signal: &dyn Signal, horizon: usize) -> f64 {
    ///     // Compute IC over multiple periods
    ///     // Calculate mean(IC) / std(IC)
    ///     0.45  // Example: IR of 0.45
    /// }
    /// #     fn turnover(&self, signal: &dyn Signal) -> f64 { 0.0 }
    /// # }
    /// ```
    fn ir(&self, signal: &dyn Signal, horizon: usize) -> f64;

    /// Computes the turnover rate for a signal.
    ///
    /// Turnover measures how frequently signal rankings change period-to-period.
    /// Higher turnover implies higher transaction costs when trading the signal.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to evaluate
    ///
    /// # Returns
    ///
    /// Returns turnover as a fraction in [0, 1]:
    /// - 0.0: Signal rankings never change
    /// - 0.5: Moderate turnover
    /// - 1.0: Complete ranking reversal each period
    ///
    /// Typical interpretations:
    /// - Turnover < 0.3: Low turnover, suitable for high-frequency
    /// - Turnover > 0.7: High turnover, may be costly to trade
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::{SignalEvaluator, Signal};
    /// # struct MyEvaluator;
    /// # impl SignalEvaluator for MyEvaluator {
    /// #     fn ic(&self, signal: &dyn Signal, horizon: usize) -> f64 { 0.0 }
    /// #     fn ir(&self, signal: &dyn Signal, horizon: usize) -> f64 { 0.0 }
    /// fn turnover(&self, signal: &dyn Signal) -> f64 {
    ///     // Measure rank correlation period-over-period
    ///     // Return 1 - correlation as turnover
    ///     0.35  // Example: 35% turnover
    /// }
    /// # }
    /// ```
    fn turnover(&self, signal: &dyn Signal) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Date, MarketData, Result};
    use polars::prelude::*;

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

    struct TestEvaluator;

    impl SignalEvaluator for TestEvaluator {
        fn ic(&self, _signal: &dyn Signal, horizon: usize) -> f64 {
            // Return mock IC based on horizon
            match horizon {
                1 => 0.05,
                5 => 0.08,
                20 => 0.10,
                _ => 0.0,
            }
        }

        fn ir(&self, _signal: &dyn Signal, horizon: usize) -> f64 {
            // Return mock IR
            match horizon {
                1 => 0.3,
                5 => 0.5,
                20 => 0.6,
                _ => 0.0,
            }
        }

        fn turnover(&self, _signal: &dyn Signal) -> f64 {
            0.35
        }
    }

    #[test]
    fn test_evaluator_ic() {
        let evaluator = TestEvaluator;
        let signal = TestSignal {
            name: "test".to_string(),
        };

        assert_eq!(evaluator.ic(&signal, 1), 0.05);
        assert_eq!(evaluator.ic(&signal, 5), 0.08);
        assert_eq!(evaluator.ic(&signal, 20), 0.10);
    }

    #[test]
    fn test_evaluator_ir() {
        let evaluator = TestEvaluator;
        let signal = TestSignal {
            name: "test".to_string(),
        };

        assert_eq!(evaluator.ir(&signal, 1), 0.3);
        assert_eq!(evaluator.ir(&signal, 5), 0.5);
        assert_eq!(evaluator.ir(&signal, 20), 0.6);
    }

    #[test]
    fn test_evaluator_turnover() {
        let evaluator = TestEvaluator;
        let signal = TestSignal {
            name: "test".to_string(),
        };

        let turnover = evaluator.turnover(&signal);
        assert_eq!(turnover, 0.35);
        assert!(turnover >= 0.0 && turnover <= 1.0);
    }

    #[test]
    fn test_evaluator_with_different_signals() {
        let evaluator = TestEvaluator;

        let signal1 = TestSignal {
            name: "momentum".to_string(),
        };
        let signal2 = TestSignal {
            name: "value".to_string(),
        };

        // Both should work with the evaluator
        let ic1 = evaluator.ic(&signal1, 5);
        let ic2 = evaluator.ic(&signal2, 5);

        assert_eq!(ic1, ic2); // Same mock values
    }

    #[test]
    fn test_ic_range() {
        let evaluator = TestEvaluator;
        let signal = TestSignal {
            name: "test".to_string(),
        };

        for horizon in [1, 5, 20] {
            let ic = evaluator.ic(&signal, horizon);
            assert!(ic >= -1.0 && ic <= 1.0, "IC should be in [-1, 1]");
        }
    }

    #[test]
    fn test_turnover_range() {
        let evaluator = TestEvaluator;
        let signal = TestSignal {
            name: "test".to_string(),
        };

        let turnover = evaluator.turnover(&signal);
        assert!(
            turnover >= 0.0 && turnover <= 1.0,
            "Turnover should be in [0, 1]"
        );
    }
}
