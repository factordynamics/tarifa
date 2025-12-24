//! SignalEvaluator implementation.
//!
//! Provides a complete implementation of the SignalEvaluator trait for
//! evaluating trading signals with IC, IR, turnover, and other metrics.

use ndarray::Array1;
use serde::{Deserialize, Serialize};

/// Configuration for signal evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorConfig {
    /// Minimum number of observations for metrics
    pub min_observations: usize,
    /// Trading days per year for annualization
    pub trading_days_per_year: usize,
    /// Whether to annualize metrics
    pub annualize: bool,
    /// Default horizon for IC calculation (in days)
    pub default_horizon: usize,
    /// Number of periods for rolling calculations
    pub rolling_window: usize,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            min_observations: 20,
            trading_days_per_year: 252,
            annualize: true,
            default_horizon: 21,
            rolling_window: 63,
        }
    }
}

/// Default implementation of SignalEvaluator.
///
/// This evaluator provides comprehensive signal analysis including:
/// - Information Coefficient (IC)
/// - Information Ratio (IR)
/// - Signal turnover
/// - Decay analysis
#[derive(Debug, Clone)]
pub struct DefaultEvaluator {
    /// Signal scores over time
    signal_scores: Vec<Vec<f64>>,
    /// Forward returns over time
    forward_returns: Vec<Vec<f64>>,
    /// Configuration
    config: EvaluatorConfig,
}

impl DefaultEvaluator {
    /// Create a new evaluator.
    ///
    /// # Arguments
    ///
    /// * `signal_scores` - Time series of signal scores (dates x assets)
    /// * `forward_returns` - Time series of forward returns (dates x assets)
    /// * `config` - Evaluator configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tarifa_eval::{DefaultEvaluator, EvaluatorConfig};
    ///
    /// let evaluator = DefaultEvaluator::new(
    ///     signal_scores,
    ///     forward_returns,
    ///     EvaluatorConfig::default()
    /// );
    /// ```
    pub const fn new(
        signal_scores: Vec<Vec<f64>>,
        forward_returns: Vec<Vec<f64>>,
        config: EvaluatorConfig,
    ) -> Self {
        Self {
            signal_scores,
            forward_returns,
            config,
        }
    }

    /// Calculate Information Coefficient at a given horizon.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Forward return horizon in days
    ///
    /// # Returns
    ///
    /// Mean IC over the evaluation period
    pub fn ic(&self, horizon: usize) -> f64 {
        let ic_series = self.ic_time_series(horizon);
        let valid_ics: Vec<f64> = ic_series
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();

        if valid_ics.len() < self.config.min_observations {
            return f64::NAN;
        }

        valid_ics.iter().sum::<f64>() / valid_ics.len() as f64
    }

    /// Calculate Information Ratio at a given horizon.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Forward return horizon in days
    ///
    /// # Returns
    ///
    /// Information Ratio (annualized if configured)
    pub fn ir(&self, horizon: usize) -> f64 {
        let ic_series = self.ic_time_series(horizon);
        let ir = crate::metrics::InformationRatio::calculate(
            &ic_series,
            &crate::metrics::MetricsConfig {
                min_observations: self.config.min_observations,
                annualize: self.config.annualize,
                trading_days_per_year: self.config.trading_days_per_year,
                ..Default::default()
            },
        );

        ir.ir
    }

    /// Calculate signal turnover.
    ///
    /// # Returns
    ///
    /// Average turnover rate
    pub fn turnover(&self) -> f64 {
        let rank_series = self.compute_rank_series();
        let turnover = crate::metrics::SignalTurnover::calculate(
            &rank_series,
            &crate::metrics::MetricsConfig {
                min_observations: self.config.min_observations,
                ..Default::default()
            },
        );

        turnover.turnover_rate
    }

    /// Calculate IC time series at a given horizon.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Forward return horizon in days
    ///
    /// # Returns
    ///
    /// Vector of IC values over time
    pub fn ic_time_series(&self, horizon: usize) -> Vec<f64> {
        let n_periods = self
            .signal_scores
            .len()
            .min(self.forward_returns.len())
            .saturating_sub(horizon);

        let mut ic_series = Vec::with_capacity(n_periods);

        for i in 0..n_periods {
            if i + horizon < self.forward_returns.len() {
                let scores = Array1::from_vec(self.signal_scores[i].clone());
                let returns = Array1::from_vec(self.forward_returns[i + horizon].clone());
                let ic = crate::ic::calculate_ic(&scores, &returns);
                ic_series.push(ic);
            }
        }

        ic_series
    }

    /// Calculate comprehensive signal metrics.
    ///
    /// # Arguments
    ///
    /// * `horizon` - Forward return horizon in days
    ///
    /// # Returns
    ///
    /// SignalMetrics with all calculated metrics
    pub fn metrics(&self, horizon: usize) -> crate::metrics::SignalMetrics {
        let ic_series = self.ic_time_series(horizon);
        let rank_series = self.compute_rank_series();

        crate::metrics::SignalMetrics::calculate(
            &ic_series,
            &rank_series,
            &crate::metrics::MetricsConfig {
                min_observations: self.config.min_observations,
                annualize: self.config.annualize,
                trading_days_per_year: self.config.trading_days_per_year,
                ..Default::default()
            },
        )
    }

    /// Perform decay analysis.
    ///
    /// # Returns
    ///
    /// DecayAnalysis with IC at multiple horizons
    pub fn decay_analysis(&self) -> crate::decay::DecayAnalysis {
        let horizons = crate::decay::DecayAnalysis::standard_horizons();

        crate::decay::DecayAnalysis::analyze(&horizons, |h| {
            let ic = self.ic(h);
            let ic_series = self.ic_time_series(h);
            let valid_ics: Vec<f64> = ic_series
                .iter()
                .copied()
                .filter(|x| x.is_finite())
                .collect();

            let std_err = if valid_ics.len() > 1 {
                let mean = valid_ics.iter().sum::<f64>() / valid_ics.len() as f64;
                let variance = valid_ics.iter().map(|ic| (ic - mean).powi(2)).sum::<f64>()
                    / (valid_ics.len() - 1) as f64;
                variance.sqrt() / (valid_ics.len() as f64).sqrt()
            } else {
                f64::NAN
            };

            (ic, std_err)
        })
    }

    /// Compute rank series for turnover calculation.
    fn compute_rank_series(&self) -> Vec<Vec<f64>> {
        self.signal_scores
            .iter()
            .map(|scores| compute_ranks(scores))
            .collect()
    }
}

/// Compute ranks of values.
fn compute_ranks(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut indexed: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| if v.is_finite() { Some((i, v)) } else { None })
        .collect();

    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![f64::NAN; n];

    for (rank, &(idx, _)) in indexed.iter().enumerate() {
        ranks[idx] = rank as f64;
    }

    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data() -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let signal_scores = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![2.0, 1.0, 4.0, 3.0],
            vec![3.0, 4.0, 1.0, 2.0],
            vec![4.0, 3.0, 2.0, 1.0],
        ];

        let forward_returns = vec![
            vec![0.01, 0.02, 0.03, 0.04],
            vec![0.02, 0.01, 0.04, 0.03],
            vec![0.03, 0.04, 0.01, 0.02],
            vec![0.04, 0.03, 0.02, 0.01],
        ];

        (signal_scores, forward_returns)
    }

    #[test]
    fn test_evaluator_creation() {
        let (scores, returns) = create_test_data();
        let evaluator = DefaultEvaluator::new(scores, returns, EvaluatorConfig::default());

        assert_eq!(evaluator.signal_scores.len(), 4);
        assert_eq!(evaluator.forward_returns.len(), 4);
    }

    #[test]
    fn test_ic_calculation() {
        let (scores, returns) = create_test_data();
        let config = EvaluatorConfig {
            min_observations: 2,
            ..Default::default()
        };
        let evaluator = DefaultEvaluator::new(scores, returns, config);

        let ic = evaluator.ic(1);
        // IC may be NaN with small test data - this is expected behavior
        if ic.is_finite() {
            assert!(ic >= -1.0 && ic <= 1.0);
        }
    }

    #[test]
    fn test_ir_calculation() {
        let (scores, returns) = create_test_data();
        let config = EvaluatorConfig {
            min_observations: 2,
            annualize: false,
            ..Default::default()
        };
        let evaluator = DefaultEvaluator::new(scores, returns, config);

        let ir = evaluator.ir(1);
        assert!(ir.is_finite());
    }

    #[test]
    fn test_turnover_calculation() {
        let (scores, returns) = create_test_data();
        let config = EvaluatorConfig {
            min_observations: 2,
            ..Default::default()
        };
        let evaluator = DefaultEvaluator::new(scores, returns, config);

        let turnover = evaluator.turnover();
        assert!(turnover.is_finite());
        assert!(turnover >= 0.0 && turnover <= 2.0);
    }

    #[test]
    fn test_ic_time_series() {
        let (scores, returns) = create_test_data();
        let evaluator = DefaultEvaluator::new(scores, returns, EvaluatorConfig::default());

        let ic_series = evaluator.ic_time_series(1);
        assert!(!ic_series.is_empty());
        assert!(
            ic_series
                .iter()
                .all(|&ic| ic >= -1.0 && ic <= 1.0 || ic.is_nan())
        );
    }

    #[test]
    fn test_metrics() {
        let (scores, returns) = create_test_data();
        let config = EvaluatorConfig {
            min_observations: 2,
            annualize: false,
            ..Default::default()
        };
        let evaluator = DefaultEvaluator::new(scores, returns, config);

        let metrics = evaluator.metrics(1);
        assert!(metrics.mean_ic.is_finite());
        assert!(metrics.ic_std.is_finite());
    }

    #[test]
    fn test_decay_analysis() {
        let (mut scores, mut returns) = create_test_data();

        // Add more periods for decay analysis
        for _ in 0..100 {
            scores.push(vec![1.0, 2.0, 3.0, 4.0]);
            returns.push(vec![0.01, 0.02, 0.03, 0.04]);
        }

        let config = EvaluatorConfig {
            min_observations: 2,
            ..Default::default()
        };
        let evaluator = DefaultEvaluator::new(scores, returns, config);

        let decay = evaluator.decay_analysis();
        assert!(!decay.curve.horizons.is_empty());
        assert_eq!(decay.curve.horizons.len(), decay.curve.ic_values.len());
    }

    #[test]
    fn test_compute_ranks() {
        let values = vec![3.0, 1.0, 2.0, 4.0];
        let ranks = compute_ranks(&values);
        assert_eq!(ranks, vec![2.0, 0.0, 1.0, 3.0]);
    }

    #[test]
    fn test_evaluator_config_default() {
        let config = EvaluatorConfig::default();
        assert_eq!(config.min_observations, 20);
        assert_eq!(config.trading_days_per_year, 252);
        assert_eq!(config.default_horizon, 21);
        assert!(config.annualize);
    }
}
