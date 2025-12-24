//! Equal-weighted signal combination strategy.

use ndarray::Array1;
use serde::{Deserialize, Serialize};
use tarifa_traits::Result;

use crate::combiner::{Combiner, SignalScore};

/// Configuration for equal-weighted signal combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualWeightConfig {
    /// Whether to re-standardize the output to z-scores (mean=0, std=1)
    pub normalize: bool,
}

impl Default for EqualWeightConfig {
    fn default() -> Self {
        Self { normalize: true }
    }
}

/// Equal-weighted combiner that averages all signal scores.
///
/// This is the simplest combination strategy - it computes the arithmetic mean
/// of all input signals. Optionally re-standardizes the output to z-scores.
///
/// # Examples
///
/// ```rust,no_run
/// use tarifa_combine::{EqualWeightCombiner, EqualWeightConfig, Combiner, SignalScore};
/// use ndarray::Array1;
///
/// let config = EqualWeightConfig { normalize: true };
/// let combiner = EqualWeightCombiner::new(config);
///
/// let signals = vec![
///     SignalScore {
///         name: "momentum".to_string(),
///         scores: Array1::from_vec(vec![0.5, -0.2, 1.0]),
///     },
///     SignalScore {
///         name: "value".to_string(),
///         scores: Array1::from_vec(vec![-0.3, 0.8, 0.1]),
///     },
/// ];
///
/// let composite = combiner.combine(&signals).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct EqualWeightCombiner {
    config: EqualWeightConfig,
}

impl EqualWeightCombiner {
    /// Create a new equal-weight combiner with the given configuration.
    pub const fn new(config: EqualWeightConfig) -> Self {
        Self { config }
    }

    /// Standardize a vector to z-scores (mean=0, std=1).
    fn standardize(&self, scores: &Array1<f64>) -> Array1<f64> {
        let mean = scores.mean().unwrap_or(0.0);
        let std = scores.std(1.0); // ddof=1 for sample std

        if std < 1e-10 {
            // Avoid division by zero for constant signals
            return Array1::zeros(scores.len());
        }

        (scores - mean) / std
    }
}

impl Default for EqualWeightCombiner {
    fn default() -> Self {
        Self::new(EqualWeightConfig::default())
    }
}

impl Combiner for EqualWeightCombiner {
    fn combine(&self, signals: &[SignalScore]) -> Result<Array1<f64>> {
        if signals.is_empty() {
            return Err("Cannot combine zero signals".into());
        }

        let n_assets = signals[0].scores.len();

        // Validate all signals have the same length
        for signal in signals {
            if signal.scores.len() != n_assets {
                return Err(format!(
                    "Signal '{}' has {} assets, expected {}",
                    signal.name,
                    signal.scores.len(),
                    n_assets
                )
                .into());
            }
        }

        // Compute simple average
        let mut composite = Array1::zeros(n_assets);
        for signal in signals {
            composite += &signal.scores;
        }
        composite /= signals.len() as f64;

        // Optionally re-standardize
        if self.config.normalize {
            composite = self.standardize(&composite);
        }

        // Validate output
        if composite.iter().any(|&x| !x.is_finite()) {
            return Err("Combination produced non-finite values".into());
        }

        Ok(composite)
    }

    fn name(&self) -> &str {
        "equal_weight"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal_weight_basic() {
        let combiner = EqualWeightCombiner::default();

        let signals = vec![
            SignalScore {
                name: "sig1".to_string(),
                scores: Array1::from_vec(vec![1.0, 0.0, -1.0]),
            },
            SignalScore {
                name: "sig2".to_string(),
                scores: Array1::from_vec(vec![-1.0, 0.0, 1.0]),
            },
        ];

        let result = combiner.combine(&signals).unwrap();

        // Should average to [0, 0, 0], which standardizes to [0, 0, 0]
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|&x| x.abs() < 1e-10));
    }

    #[test]
    fn test_equal_weight_without_normalization() {
        let config = EqualWeightConfig { normalize: false };
        let combiner = EqualWeightCombiner::new(config);

        let signals = vec![
            SignalScore {
                name: "sig1".to_string(),
                scores: Array1::from_vec(vec![2.0, 4.0, 6.0]),
            },
            SignalScore {
                name: "sig2".to_string(),
                scores: Array1::from_vec(vec![0.0, 2.0, 4.0]),
            },
        ];

        let result = combiner.combine(&signals).unwrap();

        // Should be [1, 3, 5]
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 3.0).abs() < 1e-10);
        assert!((result[2] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_equal_weight_mismatched_lengths() {
        let combiner = EqualWeightCombiner::default();

        let signals = vec![
            SignalScore {
                name: "sig1".to_string(),
                scores: Array1::from_vec(vec![1.0, 2.0]),
            },
            SignalScore {
                name: "sig2".to_string(),
                scores: Array1::from_vec(vec![1.0, 2.0, 3.0]),
            },
        ];

        let result = combiner.combine(&signals);
        assert!(result.is_err());
    }

    #[test]
    fn test_equal_weight_empty_signals() {
        let combiner = EqualWeightCombiner::default();
        let result = combiner.combine(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_equal_weight_single_signal() {
        let combiner = EqualWeightCombiner::default();

        let signals = vec![SignalScore {
            name: "sig1".to_string(),
            scores: Array1::from_vec(vec![1.0, -1.0, 0.0]),
        }];

        let result = combiner.combine(&signals).unwrap();
        assert_eq!(result.len(), 3);
    }
}
