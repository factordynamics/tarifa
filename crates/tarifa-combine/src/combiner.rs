//! Core trait definition for signal combiners.

use ndarray::Array1;
use tarifa_traits::Result;

/// Score output from a single signal for combination.
///
/// Each signal produces a vector of z-scores (mean=0, std=1) for a universe of assets.
/// The combiner takes multiple of these and produces a composite alpha score.
#[derive(Debug, Clone)]
pub struct SignalScore {
    /// Signal name (for debugging and IC tracking)
    pub name: String,

    /// Z-scores for each asset in the universe
    pub scores: Array1<f64>,
}

/// Combines multiple signal scores into a composite alpha.
///
/// Implementors define different strategies for weighting and combining signals.
/// All implementations must be thread-safe (Send + Sync) to support parallel processing.
///
/// # Examples
///
/// ```rust,no_run
/// use tarifa_combine::{Combiner, SignalScore};
/// use ndarray::Array1;
///
/// struct MyCombiner;
///
/// impl Combiner for MyCombiner {
///     fn combine(&self, signals: &[SignalScore]) -> tarifa_traits::Result<Array1<f64>> {
///         // Custom combination logic
///         Ok(Array1::zeros(signals[0].scores.len()))
///     }
///
///     fn name(&self) -> &str {
///         "my_combiner"
///     }
/// }
/// ```
pub trait Combiner: Send + Sync {
    /// Combine multiple signals into a composite alpha vector.
    ///
    /// # Arguments
    ///
    /// * `signals` - Slice of signal scores to combine. All signals must have the same length.
    ///
    /// # Returns
    ///
    /// A composite alpha score vector with the same length as the input signals.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Signal vectors have mismatched lengths
    /// - No signals provided
    /// - Combination produces invalid values (NaN, Inf)
    fn combine(&self, signals: &[SignalScore]) -> Result<Array1<f64>>;

    /// Name of this combination strategy.
    ///
    /// Used for logging, debugging, and identification in backtests.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_score_creation() {
        let score = SignalScore {
            name: "test".to_string(),
            scores: Array1::from_vec(vec![0.5, -0.2, 1.0]),
        };

        assert_eq!(score.name, "test");
        assert_eq!(score.scores.len(), 3);
    }
}
