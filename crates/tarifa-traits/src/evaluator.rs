//! Factor evaluator trait for assessing factor quality.
//!
//! This module defines the `FactorEvaluator` trait, which provides methods
//! for evaluating the predictive power and characteristics of trading factors.
//! Common metrics include information coefficient (IC), information ratio (IR),
//! and turnover.

use factors::Factor;

/// Evaluates the quality and characteristics of trading factors.
///
/// The `FactorEvaluator` trait defines the interface for assessing factor
/// performance using standard quantitative finance metrics. These evaluations
/// help determine which factors to include in an alpha model and how to
/// weight them.
///
/// # Metrics
///
/// - **Information Coefficient (IC)**: Correlation between factor scores and
///   future returns, measuring predictive power
/// - **Information Ratio (IR)**: Mean IC divided by standard deviation of IC,
///   measuring consistency of predictions
/// - **Turnover**: Rate at which factor rankings change, affecting
///   transaction costs
///
/// # Example
///
/// ```no_run
/// use tarifa_traits::FactorEvaluator;
/// use factors::Factor;
///
/// struct SimpleEvaluator;
///
/// impl FactorEvaluator for SimpleEvaluator {
///     fn ic(&self, factor: &dyn Factor, horizon: usize) -> f64 {
///         // Compute information coefficient
///         0.05
///     }
///
///     fn ir(&self, factor: &dyn Factor, horizon: usize) -> f64 {
///         // Compute information ratio
///         0.5
///     }
///
///     fn turnover(&self, factor: &dyn Factor) -> f64 {
///         // Compute factor turnover
///         0.3
///     }
/// }
/// ```
pub trait FactorEvaluator {
    /// Computes the information coefficient for a factor.
    ///
    /// The IC measures the correlation between factor scores and future
    /// returns over a specified horizon. Higher absolute values indicate
    /// stronger predictive power.
    ///
    /// # Arguments
    ///
    /// * `factor` - The factor to evaluate
    /// * `horizon` - Forward-looking period in days (e.g., 1, 5, 20)
    ///
    /// # Returns
    ///
    /// Returns the IC as a correlation coefficient, typically in [-1, 1].
    /// - Positive values: Factor predicts positive returns
    /// - Negative values: Factor predicts negative returns (can be inverted)
    /// - Zero: No predictive power
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::FactorEvaluator;
    /// # use factors::Factor;
    /// # struct MyEvaluator;
    /// # impl FactorEvaluator for MyEvaluator {
    /// fn ic(&self, factor: &dyn Factor, horizon: usize) -> f64 {
    ///     // Correlate factor scores with forward returns
    ///     // at the specified horizon
    ///     0.08  // Example: 8% correlation
    /// }
    /// #     fn ir(&self, factor: &dyn Factor, horizon: usize) -> f64 { 0.0 }
    /// #     fn turnover(&self, factor: &dyn Factor) -> f64 { 0.0 }
    /// # }
    /// ```
    fn ic(&self, factor: &dyn Factor, horizon: usize) -> f64;

    /// Computes the information ratio for a factor.
    ///
    /// The IR measures the consistency of a factor's predictive power by
    /// dividing mean IC by the standard deviation of IC over time. Higher
    /// values indicate more reliable factors.
    ///
    /// # Arguments
    ///
    /// * `factor` - The factor to evaluate
    /// * `horizon` - Forward-looking period in days
    ///
    /// # Returns
    ///
    /// Returns the IR, where:
    /// - IR > 0.5: Strong, consistent factor
    /// - IR > 0.3: Moderate factor quality
    /// - IR < 0.3: Weak or inconsistent factor
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use tarifa_traits::FactorEvaluator;
    /// # use factors::Factor;
    /// # struct MyEvaluator;
    /// # impl FactorEvaluator for MyEvaluator {
    /// #     fn ic(&self, factor: &dyn Factor, horizon: usize) -> f64 { 0.0 }
    /// fn ir(&self, factor: &dyn Factor, horizon: usize) -> f64 {
    ///     // Compute IC over multiple periods
    ///     // Calculate mean(IC) / std(IC)
    ///     0.45  // Example: IR of 0.45
    /// }
    /// #     fn turnover(&self, factor: &dyn Factor) -> f64 { 0.0 }
    /// # }
    /// ```
    fn ir(&self, factor: &dyn Factor, horizon: usize) -> f64;

    /// Computes the turnover rate for a factor.
    ///
    /// Turnover measures how frequently factor rankings change period-to-period.
    /// Higher turnover implies higher transaction costs when trading the factor.
    ///
    /// # Arguments
    ///
    /// * `factor` - The factor to evaluate
    ///
    /// # Returns
    ///
    /// Returns turnover as a fraction in [0, 1]:
    /// - 0.0: Factor rankings never change
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
    /// # use tarifa_traits::FactorEvaluator;
    /// # use factors::Factor;
    /// # struct MyEvaluator;
    /// # impl FactorEvaluator for MyEvaluator {
    /// #     fn ic(&self, factor: &dyn Factor, horizon: usize) -> f64 { 0.0 }
    /// #     fn ir(&self, factor: &dyn Factor, horizon: usize) -> f64 { 0.0 }
    /// fn turnover(&self, factor: &dyn Factor) -> f64 {
    ///     // Measure rank correlation period-over-period
    ///     // Return 1 - correlation as turnover
    ///     0.35  // Example: 35% turnover
    /// }
    /// # }
    /// ```
    fn turnover(&self, factor: &dyn Factor) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use factors::{DataFrequency, FactorCategory};
    use polars::prelude::*;

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
            &["close", "volume"]
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

    struct TestEvaluator;

    impl FactorEvaluator for TestEvaluator {
        fn ic(&self, _factor: &dyn Factor, horizon: usize) -> f64 {
            // Return mock IC based on horizon
            match horizon {
                1 => 0.05,
                5 => 0.08,
                20 => 0.10,
                _ => 0.0,
            }
        }

        fn ir(&self, _factor: &dyn Factor, horizon: usize) -> f64 {
            // Return mock IR
            match horizon {
                1 => 0.3,
                5 => 0.5,
                20 => 0.6,
                _ => 0.0,
            }
        }

        fn turnover(&self, _factor: &dyn Factor) -> f64 {
            0.35
        }
    }

    #[test]
    fn test_evaluator_ic() {
        let evaluator = TestEvaluator;
        let factor = TestFactor {
            name: "test".to_string(),
        };

        assert_eq!(evaluator.ic(&factor, 1), 0.05);
        assert_eq!(evaluator.ic(&factor, 5), 0.08);
        assert_eq!(evaluator.ic(&factor, 20), 0.10);
    }

    #[test]
    fn test_evaluator_ir() {
        let evaluator = TestEvaluator;
        let factor = TestFactor {
            name: "test".to_string(),
        };

        assert_eq!(evaluator.ir(&factor, 1), 0.3);
        assert_eq!(evaluator.ir(&factor, 5), 0.5);
        assert_eq!(evaluator.ir(&factor, 20), 0.6);
    }

    #[test]
    fn test_evaluator_turnover() {
        let evaluator = TestEvaluator;
        let factor = TestFactor {
            name: "test".to_string(),
        };

        let turnover = evaluator.turnover(&factor);
        assert_eq!(turnover, 0.35);
        assert!(turnover >= 0.0 && turnover <= 1.0);
    }

    #[test]
    fn test_evaluator_with_different_factors() {
        let evaluator = TestEvaluator;

        let factor1 = TestFactor {
            name: "momentum".to_string(),
        };
        let factor2 = TestFactor {
            name: "value".to_string(),
        };

        // Both should work with the evaluator
        let ic1 = evaluator.ic(&factor1, 5);
        let ic2 = evaluator.ic(&factor2, 5);

        assert_eq!(ic1, ic2); // Same mock values
    }

    #[test]
    fn test_ic_range() {
        let evaluator = TestEvaluator;
        let factor = TestFactor {
            name: "test".to_string(),
        };

        for horizon in [1, 5, 20] {
            let ic = evaluator.ic(&factor, horizon);
            assert!(ic >= -1.0 && ic <= 1.0, "IC should be in [-1, 1]");
        }
    }

    #[test]
    fn test_turnover_range() {
        let evaluator = TestEvaluator;
        let factor = TestFactor {
            name: "test".to_string(),
        };

        let turnover = evaluator.turnover(&factor);
        assert!(
            turnover >= 0.0 && turnover <= 1.0,
            "Turnover should be in [0, 1]"
        );
    }
}
