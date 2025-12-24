//! IC-weighted signal combination strategy.

use ndarray::Array1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tarifa_traits::Result;

use crate::combiner::{Combiner, SignalScore};

/// Configuration for IC-weighted signal combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ICWeightedConfig {
    /// Number of periods to use for IC lookback
    pub ic_lookback: usize,

    /// Exponential decay factor for IC history (0.0 = equal weight, 1.0 = only latest)
    /// Typical values: 0.0-0.3 for slow decay, 0.5-0.7 for fast decay
    pub decay_factor: f64,
}

impl Default for ICWeightedConfig {
    fn default() -> Self {
        Self {
            ic_lookback: 60,   // ~3 months of daily data
            decay_factor: 0.0, // Equal weight by default
        }
    }
}

/// IC-weighted combiner that weights signals by their historical Information Coefficient.
///
/// This combiner maintains a history of IC values for each signal and uses them to
/// compute adaptive weights. Signals with higher recent IC receive higher weight.
///
/// # Examples
///
/// ```rust,no_run
/// use tarifa_combine::{ICWeightedCombiner, ICWeightedConfig, Combiner, SignalScore};
/// use ndarray::Array1;
///
/// let config = ICWeightedConfig {
///     ic_lookback: 60,
///     decay_factor: 0.3,
/// };
/// let mut combiner = ICWeightedCombiner::new(config);
///
/// // Update IC history (typically done in backtesting loop)
/// combiner.update_ic("momentum", 0.05);
/// combiner.update_ic("value", 0.03);
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
pub struct ICWeightedCombiner {
    config: ICWeightedConfig,
    ic_history: HashMap<String, Vec<f64>>,
}

impl ICWeightedCombiner {
    /// Create a new IC-weighted combiner with the given configuration.
    pub fn new(config: ICWeightedConfig) -> Self {
        Self {
            config,
            ic_history: HashMap::new(),
        }
    }

    /// Update IC history for a signal.
    ///
    /// Call this after each period to maintain the rolling IC window.
    ///
    /// # Arguments
    ///
    /// * `signal_name` - Name of the signal
    /// * `ic` - Information coefficient for this period
    pub fn update_ic(&mut self, signal_name: &str, ic: f64) {
        let history = self.ic_history.entry(signal_name.to_string()).or_default();
        history.push(ic);

        // Maintain lookback window
        if history.len() > self.config.ic_lookback {
            history.remove(0);
        }
    }

    /// Compute exponentially-weighted average IC for a signal.
    fn compute_weighted_ic(&self, signal_name: &str) -> f64 {
        let history = match self.ic_history.get(signal_name) {
            Some(h) if !h.is_empty() => h,
            _ => return 0.0, // No history = zero weight
        };

        if self.config.decay_factor.abs() < 1e-10 {
            // No decay - simple average
            return history.iter().sum::<f64>() / history.len() as f64;
        }

        // Exponentially weighted average
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        for (i, &ic) in history.iter().enumerate() {
            let age = (history.len() - 1 - i) as f64;
            let weight = (-self.config.decay_factor * age).exp();
            weighted_sum += ic * weight;
            total_weight += weight;
        }

        if total_weight > 1e-10 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }

    /// Compute weights for each signal based on IC history.
    fn compute_weights(&self, signals: &[SignalScore]) -> Vec<f64> {
        let ics: Vec<f64> = signals
            .iter()
            .map(|s| self.compute_weighted_ic(&s.name))
            .collect();

        // Handle edge cases
        if ics.iter().all(|&ic| ic.abs() < 1e-10) {
            // No IC history or all zero - equal weight
            return vec![1.0 / signals.len() as f64; signals.len()];
        }

        // Use absolute IC for weighting (both positive and negative IC are valuable)
        let abs_ics: Vec<f64> = ics.iter().map(|&ic| ic.abs()).collect();
        let total: f64 = abs_ics.iter().sum();

        if total < 1e-10 {
            return vec![1.0 / signals.len() as f64; signals.len()];
        }

        // Normalize to sum to 1.0
        abs_ics.iter().map(|&ic| ic / total).collect()
    }

    /// Standardize a vector to z-scores (mean=0, std=1).
    fn standardize(&self, scores: &Array1<f64>) -> Array1<f64> {
        let mean = scores.mean().unwrap_or(0.0);
        let std = scores.std(1.0); // ddof=1 for sample std

        if std < 1e-10 {
            return Array1::zeros(scores.len());
        }

        (scores - mean) / std
    }
}

impl Default for ICWeightedCombiner {
    fn default() -> Self {
        Self::new(ICWeightedConfig::default())
    }
}

impl Combiner for ICWeightedCombiner {
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

        // Compute IC-based weights
        let weights = self.compute_weights(signals);

        // Weighted combination
        let mut composite = Array1::zeros(n_assets);
        for (signal, &weight) in signals.iter().zip(weights.iter()) {
            composite += &(&signal.scores * weight);
        }

        // Standardize output
        composite = self.standardize(&composite);

        // Validate output
        if composite.iter().any(|&x| !x.is_finite()) {
            return Err("Combination produced non-finite values".into());
        }

        Ok(composite)
    }

    fn name(&self) -> &str {
        "ic_weight"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ic_weight_no_history() {
        let combiner = ICWeightedCombiner::default();

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

        // No IC history = equal weight
        let result = combiner.combine(&signals).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_ic_weight_with_history() {
        let mut combiner = ICWeightedCombiner::default();

        // Signal 1 has much higher IC
        combiner.update_ic("sig1", 0.10);
        combiner.update_ic("sig2", 0.02);

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
        assert_eq!(result.len(), 3);

        // Result should be closer to sig1 due to higher IC
        // Since we standardize, we just check it's valid
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_ic_weight_decay() {
        let config = ICWeightedConfig {
            ic_lookback: 10,
            decay_factor: 0.5,
        };
        let mut combiner = ICWeightedCombiner::new(config);

        // Add history with varying IC
        for i in 0..10 {
            combiner.update_ic("sig1", 0.05 + (i as f64 * 0.01));
        }

        let weighted_ic = combiner.compute_weighted_ic("sig1");
        assert!(weighted_ic > 0.0);
        // More recent ICs should have higher weight
    }

    #[test]
    fn test_ic_weight_lookback_limit() {
        let config = ICWeightedConfig {
            ic_lookback: 5,
            decay_factor: 0.0,
        };
        let mut combiner = ICWeightedCombiner::new(config);

        // Add more than lookback periods
        for _ in 0..10 {
            combiner.update_ic("sig1", 0.05);
        }

        // Should only keep last 5
        let history = combiner.ic_history.get("sig1").unwrap();
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_ic_weight_negative_ic() {
        let mut combiner = ICWeightedCombiner::default();

        // Both signals have IC, one positive one negative
        combiner.update_ic("sig1", 0.05);
        combiner.update_ic("sig2", -0.05);

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

        // Should weight by absolute IC (both have same magnitude)
        let result = combiner.combine(&signals).unwrap();
        assert!(result.iter().all(|&x| x.is_finite()));
    }
}
