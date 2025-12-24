//! Volatility-scaled signal combination strategy.

use ndarray::Array1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tarifa_traits::Result;

use crate::combiner::{Combiner, SignalScore};

/// Configuration for volatility-scaled signal combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolScaledConfig {
    /// Target volatility for the composite signal (in z-score units)
    pub target_vol: f64,

    /// Whether to use IC weighting before volatility scaling
    pub ic_weight: bool,

    /// IC lookback period (only used if ic_weight = true)
    pub ic_lookback: usize,

    /// IC decay factor (only used if ic_weight = true)
    pub decay_factor: f64,
}

impl Default for VolScaledConfig {
    fn default() -> Self {
        Self {
            target_vol: 1.0,   // Standard z-score volatility
            ic_weight: true,   // Use IC weighting by default
            ic_lookback: 60,   // ~3 months
            decay_factor: 0.0, // Equal weight
        }
    }
}

/// Volatility-scaled combiner that produces signals with target volatility.
///
/// This combiner first combines signals (optionally using IC weights), then scales
/// the output to achieve a target volatility. This is useful for controlling the
/// "aggressiveness" of the alpha signal.
///
/// # Examples
///
/// ```rust,no_run
/// use tarifa_combine::{VolScaledCombiner, VolScaledConfig, Combiner, SignalScore};
/// use ndarray::Array1;
///
/// let config = VolScaledConfig {
///     target_vol: 1.5,  // 50% higher volatility than standard
///     ic_weight: true,
///     ic_lookback: 60,
///     decay_factor: 0.3,
/// };
/// let mut combiner = VolScaledCombiner::new(config);
///
/// // Update IC history if using IC weighting
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
pub struct VolScaledCombiner {
    config: VolScaledConfig,
    ic_history: HashMap<String, Vec<f64>>,
}

impl VolScaledCombiner {
    /// Create a new volatility-scaled combiner with the given configuration.
    pub fn new(config: VolScaledConfig) -> Self {
        Self {
            config,
            ic_history: HashMap::new(),
        }
    }

    /// Update IC history for a signal.
    ///
    /// Call this after each period to maintain the rolling IC window.
    /// Only relevant if `ic_weight` is enabled.
    ///
    /// # Arguments
    ///
    /// * `signal_name` - Name of the signal
    /// * `ic` - Information coefficient for this period
    pub fn update_ic(&mut self, signal_name: &str, ic: f64) {
        if !self.config.ic_weight {
            return; // IC history not used
        }

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
            _ => return 0.0,
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
        if !self.config.ic_weight {
            // Equal weight
            return vec![1.0 / signals.len() as f64; signals.len()];
        }

        let ics: Vec<f64> = signals
            .iter()
            .map(|s| self.compute_weighted_ic(&s.name))
            .collect();

        // Handle edge cases
        if ics.iter().all(|&ic| ic.abs() < 1e-10) {
            return vec![1.0 / signals.len() as f64; signals.len()];
        }

        // Use absolute IC for weighting
        let abs_ics: Vec<f64> = ics.iter().map(|&ic| ic.abs()).collect();
        let total: f64 = abs_ics.iter().sum();

        if total < 1e-10 {
            return vec![1.0 / signals.len() as f64; signals.len()];
        }

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

    /// Scale a vector to target volatility.
    fn scale_to_vol(&self, scores: &Array1<f64>) -> Array1<f64> {
        let std = scores.std(1.0);

        if std < 1e-10 {
            return scores.clone();
        }

        scores * (self.config.target_vol / std)
    }
}

impl Default for VolScaledCombiner {
    fn default() -> Self {
        Self::new(VolScaledConfig::default())
    }
}

impl Combiner for VolScaledCombiner {
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

        // Compute weights (IC-based or equal)
        let weights = self.compute_weights(signals);

        // Weighted combination
        let mut composite = Array1::zeros(n_assets);
        for (signal, &weight) in signals.iter().zip(weights.iter()) {
            composite += &(&signal.scores * weight);
        }

        // Standardize to z-scores
        composite = self.standardize(&composite);

        // Scale to target volatility
        composite = self.scale_to_vol(&composite);

        // Validate output
        if composite.iter().any(|&x| !x.is_finite()) {
            return Err("Combination produced non-finite values".into());
        }

        Ok(composite)
    }

    fn name(&self) -> &str {
        "vol_scale"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_scale_default() {
        let combiner = VolScaledCombiner::default();

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
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_vol_scale_target() {
        let config = VolScaledConfig {
            target_vol: 2.0,
            ic_weight: false,
            ic_lookback: 60,
            decay_factor: 0.0,
        };
        let combiner = VolScaledCombiner::new(config);

        let signals = vec![SignalScore {
            name: "sig1".to_string(),
            scores: Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        }];

        let result = combiner.combine(&signals).unwrap();

        // Check that std is approximately target_vol
        let std = result.std(1.0);
        assert!((std - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_vol_scale_with_ic_weight() {
        let config = VolScaledConfig {
            target_vol: 1.0,
            ic_weight: true,
            ic_lookback: 60,
            decay_factor: 0.0,
        };
        let mut combiner = VolScaledCombiner::new(config);

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
        assert!(result.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_vol_scale_without_ic_weight() {
        let config = VolScaledConfig {
            target_vol: 1.5,
            ic_weight: false,
            ic_lookback: 60,
            decay_factor: 0.0,
        };
        let combiner = VolScaledCombiner::new(config);

        let signals = vec![
            SignalScore {
                name: "sig1".to_string(),
                scores: Array1::from_vec(vec![2.0, 4.0, 6.0, 8.0]),
            },
            SignalScore {
                name: "sig2".to_string(),
                scores: Array1::from_vec(vec![0.0, 2.0, 4.0, 6.0]),
            },
        ];

        let result = combiner.combine(&signals).unwrap();

        // Should be scaled to 1.5 volatility
        let std = result.std(1.0);
        assert!((std - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_vol_scale_constant_signal() {
        let config = VolScaledConfig {
            target_vol: 2.0,
            ic_weight: false,
            ic_lookback: 60,
            decay_factor: 0.0,
        };
        let combiner = VolScaledCombiner::new(config);

        let signals = vec![SignalScore {
            name: "sig1".to_string(),
            scores: Array1::from_vec(vec![5.0, 5.0, 5.0, 5.0]),
        }];

        let result = combiner.combine(&signals).unwrap();

        // Constant signal should produce zeros after standardization
        assert!(result.iter().all(|&x| x.abs() < 1e-10));
    }

    #[test]
    fn test_vol_scale_mismatched_lengths() {
        let combiner = VolScaledCombiner::default();

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
}
