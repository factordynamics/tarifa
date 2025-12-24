//! Signal quality metrics.
//!
//! This module provides various metrics for evaluating signal quality including:
//! - Information Ratio (IR): mean IC / std IC
//! - Signal Turnover: autocorrelation of signal ranks
//! - Aggregate metrics for comprehensive signal evaluation

use serde::{Deserialize, Serialize};

/// Configuration for metrics calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Minimum number of observations required for metric calculation
    pub min_observations: usize,
    /// Number of periods for turnover calculation
    pub turnover_periods: usize,
    /// Whether to annualize metrics
    pub annualize: bool,
    /// Number of trading days per year for annualization
    pub trading_days_per_year: usize,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            min_observations: 20,
            turnover_periods: 21,
            annualize: true,
            trading_days_per_year: 252,
        }
    }
}

/// Information Ratio: mean IC divided by standard deviation of IC.
///
/// IR measures the consistency of a signal's predictive power.
/// Higher IR indicates more reliable signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformationRatio {
    /// Mean IC
    pub mean_ic: f64,
    /// Standard deviation of IC
    pub std_ic: f64,
    /// Information Ratio
    pub ir: f64,
    /// Number of observations
    pub n_obs: usize,
}

impl InformationRatio {
    /// Calculate Information Ratio from IC time series.
    ///
    /// # Arguments
    ///
    /// * `ic_series` - Time series of IC values
    /// * `config` - Configuration for calculation
    ///
    /// # Returns
    ///
    /// InformationRatio struct with calculated metrics
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tarifa_eval::{InformationRatio, MetricsConfig};
    ///
    /// let ic_series = vec![0.05, 0.03, 0.07, 0.02, 0.06];
    /// let ir = InformationRatio::calculate(&ic_series, &MetricsConfig::default());
    /// ```
    pub fn calculate(ic_series: &[f64], config: &MetricsConfig) -> Self {
        // Filter out NaN values
        let valid_ics: Vec<f64> = ic_series
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();

        let n_obs = valid_ics.len();

        if n_obs < config.min_observations {
            return Self {
                mean_ic: f64::NAN,
                std_ic: f64::NAN,
                ir: f64::NAN,
                n_obs,
            };
        }

        let mean_ic: f64 = valid_ics.iter().sum::<f64>() / n_obs as f64;

        let variance: f64 = valid_ics
            .iter()
            .map(|ic| (ic - mean_ic).powi(2))
            .sum::<f64>()
            / (n_obs - 1) as f64;

        let std_ic = variance.sqrt();

        let ir = if std_ic > 0.0 {
            if config.annualize {
                mean_ic / std_ic * (config.trading_days_per_year as f64).sqrt()
            } else {
                mean_ic / std_ic
            }
        } else {
            f64::NAN
        };

        Self {
            mean_ic,
            std_ic,
            ir,
            n_obs,
        }
    }
}

/// Signal turnover measured by autocorrelation of signal ranks.
///
/// Lower autocorrelation indicates higher turnover (less stable positions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTurnover {
    /// Autocorrelation of signal ranks (lag 1)
    pub autocorr: f64,
    /// Average turnover rate
    pub turnover_rate: f64,
    /// Number of observations
    pub n_obs: usize,
}

impl SignalTurnover {
    /// Calculate signal turnover from rank time series.
    ///
    /// # Arguments
    ///
    /// * `rank_series` - Time series of signal ranks for each asset
    /// * `config` - Configuration for calculation
    ///
    /// # Returns
    ///
    /// SignalTurnover struct with calculated metrics
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tarifa_eval::{SignalTurnover, MetricsConfig};
    ///
    /// let rank_series = vec![
    ///     vec![1.0, 2.0, 3.0],
    ///     vec![1.0, 3.0, 2.0],
    ///     vec![2.0, 1.0, 3.0],
    /// ];
    /// let turnover = SignalTurnover::calculate(&rank_series, &MetricsConfig::default());
    /// ```
    pub fn calculate(rank_series: &[Vec<f64>], config: &MetricsConfig) -> Self {
        if rank_series.len() < 2 {
            return Self {
                autocorr: f64::NAN,
                turnover_rate: f64::NAN,
                n_obs: rank_series.len(),
            };
        }

        let n_obs = rank_series.len();
        let n_assets = rank_series[0].len();

        // Calculate autocorrelation for each asset and average
        let mut total_autocorr = 0.0;
        let mut valid_assets = 0;

        for asset_idx in 0..n_assets {
            let asset_ranks: Vec<f64> = rank_series
                .iter()
                .map(|ranks| ranks[asset_idx])
                .filter(|x| x.is_finite())
                .collect();

            if asset_ranks.len() >= config.min_observations {
                let autocorr = calculate_autocorrelation(&asset_ranks, 1);
                if autocorr.is_finite() {
                    total_autocorr += autocorr;
                    valid_assets += 1;
                }
            }
        }

        let autocorr = if valid_assets > 0 {
            total_autocorr / valid_assets as f64
        } else {
            f64::NAN
        };

        // Turnover rate is approximately 1 - autocorr
        let turnover_rate = 1.0 - autocorr;

        Self {
            autocorr,
            turnover_rate,
            n_obs,
        }
    }
}

/// Aggregate signal metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMetrics {
    /// Information Ratio
    pub ir: InformationRatio,
    /// Signal turnover
    pub turnover: SignalTurnover,
    /// Mean IC
    pub mean_ic: f64,
    /// IC standard deviation
    pub ic_std: f64,
    /// IC hit rate (proportion of positive ICs)
    pub ic_hit_rate: f64,
}

impl SignalMetrics {
    /// Calculate comprehensive signal metrics.
    ///
    /// # Arguments
    ///
    /// * `ic_series` - Time series of IC values
    /// * `rank_series` - Time series of signal ranks
    /// * `config` - Configuration for calculation
    ///
    /// # Returns
    ///
    /// SignalMetrics struct with all calculated metrics
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tarifa_eval::{SignalMetrics, MetricsConfig};
    ///
    /// let metrics = SignalMetrics::calculate(
    ///     &ic_series,
    ///     &rank_series,
    ///     &MetricsConfig::default()
    /// );
    /// ```
    pub fn calculate(ic_series: &[f64], rank_series: &[Vec<f64>], config: &MetricsConfig) -> Self {
        let ir = InformationRatio::calculate(ic_series, config);
        let turnover = SignalTurnover::calculate(rank_series, config);

        // Calculate IC hit rate
        let valid_ics: Vec<f64> = ic_series
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();
        let positive_ics = valid_ics.iter().filter(|&&ic| ic > 0.0).count();
        let ic_hit_rate = if !valid_ics.is_empty() {
            positive_ics as f64 / valid_ics.len() as f64
        } else {
            f64::NAN
        };

        Self {
            mean_ic: ir.mean_ic,
            ic_std: ir.std_ic,
            ir,
            turnover,
            ic_hit_rate,
        }
    }
}

/// Calculate autocorrelation at a given lag.
fn calculate_autocorrelation(series: &[f64], lag: usize) -> f64 {
    if series.len() <= lag {
        return f64::NAN;
    }

    let n = series.len() - lag;
    let mean: f64 = series.iter().sum::<f64>() / series.len() as f64;

    let mut numerator = 0.0;
    let mut denominator = 0.0;

    for i in 0..n {
        let dev1 = series[i] - mean;
        let dev2 = series[i + lag] - mean;
        numerator += dev1 * dev2;
    }

    for val in series {
        denominator += (val - mean).powi(2);
    }

    if denominator == 0.0 {
        f64::NAN
    } else {
        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_information_ratio() {
        let ic_series = vec![0.05, 0.03, 0.07, 0.02, 0.06, 0.04, 0.08, 0.03, 0.05, 0.06];
        let config = MetricsConfig {
            annualize: false,
            min_observations: 2,
            ..Default::default()
        };
        let ir = InformationRatio::calculate(&ic_series, &config);

        assert!(ir.mean_ic >= 0.0);
        assert!(ir.std_ic >= 0.0);
        assert_eq!(ir.n_obs, 10);
    }

    #[test]
    fn test_information_ratio_with_nans() {
        let ic_series = vec![0.05, f64::NAN, 0.07, 0.02, f64::NAN, 0.04];
        let config = MetricsConfig {
            min_observations: 2,
            ..Default::default()
        };
        let ir = InformationRatio::calculate(&ic_series, &config);

        assert_eq!(ir.n_obs, 4);
        // With sufficient data, mean should be finite
        assert!(ir.mean_ic.is_finite() || ir.n_obs < config.min_observations);
    }

    #[test]
    fn test_signal_turnover() {
        let rank_series = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
            vec![2.0, 1.0, 4.0, 3.0],
            vec![2.0, 1.0, 4.0, 3.0],
        ];
        let config = MetricsConfig {
            min_observations: 2,
            ..Default::default()
        };
        let turnover = SignalTurnover::calculate(&rank_series, &config);

        assert!(turnover.autocorr.is_finite());
        assert!(turnover.turnover_rate >= 0.0 && turnover.turnover_rate <= 1.0);
        assert_eq!(turnover.n_obs, 4);
    }

    #[test]
    fn test_signal_metrics() {
        let ic_series = vec![0.05, 0.03, 0.07, 0.02, 0.06];
        let rank_series = vec![
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 3.0],
            vec![2.0, 1.0, 3.0],
            vec![2.0, 1.0, 3.0],
            vec![1.0, 3.0, 2.0],
        ];
        let config = MetricsConfig {
            min_observations: 2,
            annualize: false,
            ..Default::default()
        };

        let metrics = SignalMetrics::calculate(&ic_series, &rank_series, &config);

        assert!(metrics.mean_ic > 0.0);
        assert!(metrics.ic_std > 0.0);
        assert!(metrics.ic_hit_rate > 0.0 && metrics.ic_hit_rate <= 1.0);
        assert!(metrics.ir.ir.is_finite());
    }

    #[test]
    fn test_calculate_autocorrelation() {
        let series = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let autocorr = calculate_autocorrelation(&series, 1);
        assert!(autocorr > 0.0); // Should be positive for increasing series
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert_eq!(config.min_observations, 20);
        assert_eq!(config.turnover_periods, 21);
        assert_eq!(config.trading_days_per_year, 252);
        assert!(config.annualize);
    }
}
