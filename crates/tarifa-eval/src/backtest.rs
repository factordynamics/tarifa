//! Backtesting framework.
//!
//! Provides a complete backtesting framework for evaluating trading signals
//! with transaction costs, rebalancing, and performance metrics.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Backtesting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    /// Start date for backtest
    pub start_date: NaiveDate,
    /// End date for backtest
    pub end_date: NaiveDate,
    /// Rebalancing frequency in days
    pub rebalance_frequency: usize,
    /// Transaction cost (basis points)
    pub transaction_cost_bps: f64,
    /// Initial capital
    pub initial_capital: f64,
    /// Maximum position size (fraction of portfolio)
    pub max_position_size: f64,
    /// Minimum position size (fraction of portfolio)
    pub min_position_size: f64,
    /// Number of long positions
    pub n_long: Option<usize>,
    /// Number of short positions
    pub n_short: Option<usize>,
    /// Use long-short portfolio
    pub long_short: bool,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).expect("2020-01-01 is a valid date"),
            end_date: NaiveDate::from_ymd_opt(2024, 12, 31).expect("2024-12-31 is a valid date"),
            rebalance_frequency: 21,
            transaction_cost_bps: 10.0,
            initial_capital: 1_000_000.0,
            max_position_size: 0.1,
            min_position_size: 0.0,
            n_long: Some(50),
            n_short: Some(50),
            long_short: true,
        }
    }
}

/// Backtesting results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    /// Daily returns
    pub returns: Vec<f64>,
    /// Cumulative returns
    pub cumulative_returns: Vec<f64>,
    /// Sharpe ratio (annualized)
    pub sharpe_ratio: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Total return
    pub total_return: f64,
    /// Annualized return
    pub annualized_return: f64,
    /// Annualized volatility
    pub annualized_volatility: f64,
    /// Average turnover
    pub avg_turnover: f64,
    /// IC history
    pub ic_history: Vec<f64>,
    /// Transaction costs (cumulative)
    pub total_transaction_costs: f64,
    /// Number of trades
    pub n_trades: usize,
}

impl BacktestResult {
    /// Calculate Sharpe ratio from returns.
    pub fn calculate_sharpe(returns: &[f64], trading_days_per_year: usize) -> f64 {
        let valid_returns: Vec<f64> = returns.iter().copied().filter(|x| x.is_finite()).collect();

        if valid_returns.len() < 2 {
            return f64::NAN;
        }

        let mean = valid_returns.iter().sum::<f64>() / valid_returns.len() as f64;
        let variance = valid_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (valid_returns.len() - 1) as f64;
        let std = variance.sqrt();

        if std == 0.0 {
            f64::NAN
        } else {
            mean / std * (trading_days_per_year as f64).sqrt()
        }
    }

    /// Calculate maximum drawdown.
    pub fn calculate_max_drawdown(cumulative_returns: &[f64]) -> f64 {
        let mut max_dd = 0.0;
        let mut peak = 0.0;

        for &cum_ret in cumulative_returns {
            if cum_ret > peak {
                peak = cum_ret;
            }
            let dd = (peak - cum_ret) / (1.0 + peak);
            if dd > max_dd {
                max_dd = dd;
            }
        }

        max_dd
    }
}

/// Backtesting engine.
#[derive(Debug, Default)]
pub struct Backtest {
    /// Configuration
    config: BacktestConfig,
}

impl Backtest {
    /// Create a new backtest with configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Backtesting configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tarifa_eval::{Backtest, BacktestConfig};
    ///
    /// let config = BacktestConfig::default();
    /// let backtest = Backtest::new(config);
    /// ```
    pub const fn new(config: BacktestConfig) -> Self {
        Self { config }
    }

    /// Run the backtest.
    ///
    /// # Arguments
    ///
    /// * `signal_scores` - Time series of signal scores for each asset
    /// * `returns` - Time series of asset returns
    /// * `dates` - Trading dates
    ///
    /// # Returns
    ///
    /// BacktestResult with performance metrics
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = backtest.run(&signal_scores, &returns, &dates);
    /// println!("Sharpe Ratio: {:.2}", result.sharpe_ratio);
    /// println!("Total Return: {:.2}%", result.total_return * 100.0);
    /// ```
    pub fn run(
        &self,
        signal_scores: &[Vec<f64>],
        returns: &[Vec<f64>],
        dates: &[NaiveDate],
    ) -> BacktestResult {
        let n_periods = signal_scores.len().min(returns.len()).min(dates.len());

        let mut portfolio_returns = Vec::with_capacity(n_periods);
        let mut cumulative_returns = Vec::with_capacity(n_periods);
        let mut ic_history = Vec::with_capacity(n_periods);
        let mut turnover_history = Vec::with_capacity(n_periods);

        let mut current_positions: Vec<f64> = Vec::new();
        let mut cum_ret = 0.0;
        let mut total_transaction_costs = 0.0;
        let mut n_trades = 0;

        for i in 0..n_periods {
            // Rebalance check
            let should_rebalance = i % self.config.rebalance_frequency == 0;

            if should_rebalance {
                let new_positions = self.construct_portfolio(&signal_scores[i]);

                // Calculate turnover and transaction costs
                if !current_positions.is_empty() {
                    let turnover = self.calculate_turnover(&current_positions, &new_positions);
                    turnover_history.push(turnover);

                    let tc = turnover * self.config.transaction_cost_bps / 10000.0;
                    total_transaction_costs += tc;
                    n_trades += 1;
                }

                current_positions = new_positions;
            }

            // Calculate portfolio return
            let port_ret = if current_positions.is_empty() {
                0.0
            } else {
                self.calculate_portfolio_return(&current_positions, &returns[i])
            };

            portfolio_returns.push(port_ret);
            cum_ret = (1.0 + cum_ret) * (1.0 + port_ret) - 1.0;
            cumulative_returns.push(cum_ret);

            // Calculate IC if we have forward returns
            if i + 1 < n_periods {
                let ic = crate::ic::calculate_ic(
                    &ndarray::Array1::from_vec(signal_scores[i].clone()),
                    &ndarray::Array1::from_vec(returns[i + 1].clone()),
                );
                ic_history.push(ic);
            }
        }

        let total_return = cum_ret;
        let n_years = (dates.len() as f64) / 252.0;
        let annualized_return = if n_years > 0.0 {
            (1.0 + total_return).powf(1.0 / n_years) - 1.0
        } else {
            f64::NAN
        };

        let sharpe_ratio = BacktestResult::calculate_sharpe(&portfolio_returns, 252);
        let max_drawdown = BacktestResult::calculate_max_drawdown(&cumulative_returns);

        let annualized_volatility = if !portfolio_returns.is_empty() {
            let mean = portfolio_returns.iter().sum::<f64>() / portfolio_returns.len() as f64;
            let variance = portfolio_returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / (portfolio_returns.len() - 1) as f64;
            variance.sqrt() * (252.0_f64).sqrt()
        } else {
            f64::NAN
        };

        let avg_turnover = if !turnover_history.is_empty() {
            turnover_history.iter().sum::<f64>() / turnover_history.len() as f64
        } else {
            0.0
        };

        BacktestResult {
            returns: portfolio_returns,
            cumulative_returns,
            sharpe_ratio,
            max_drawdown,
            total_return,
            annualized_return,
            annualized_volatility,
            avg_turnover,
            ic_history,
            total_transaction_costs,
            n_trades,
        }
    }

    /// Construct portfolio from signal scores.
    fn construct_portfolio(&self, scores: &[f64]) -> Vec<f64> {
        let n_assets = scores.len();
        let mut positions = vec![0.0; n_assets];

        // Create (index, score) pairs and filter valid scores
        let mut indexed_scores: Vec<(usize, f64)> = scores
            .iter()
            .enumerate()
            .filter_map(|(i, &s)| if s.is_finite() { Some((i, s)) } else { None })
            .collect();

        if indexed_scores.is_empty() {
            return positions;
        }

        // Sort by score descending
        indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if self.config.long_short {
            // Long-short portfolio
            let n_long = self.config.n_long.unwrap_or(n_assets / 2);
            let n_short = self.config.n_short.unwrap_or(n_assets / 2);

            // Long positions (top n_long)
            let long_weight = if n_long > 0 { 1.0 / n_long as f64 } else { 0.0 };
            for (idx, _score) in indexed_scores.iter().take(n_long.min(indexed_scores.len())) {
                positions[*idx] = long_weight;
            }

            // Short positions (bottom n_short)
            let short_weight = if n_short > 0 {
                -1.0 / n_short as f64
            } else {
                0.0
            };
            let start_idx = indexed_scores.len().saturating_sub(n_short);
            for (idx, _score) in indexed_scores.iter().skip(start_idx) {
                positions[*idx] = short_weight;
            }
        } else {
            // Long-only portfolio
            let n_long = self.config.n_long.unwrap_or(n_assets);
            let weight = if n_long > 0 { 1.0 / n_long as f64 } else { 0.0 };

            for (idx, _score) in indexed_scores.iter().take(n_long.min(indexed_scores.len())) {
                positions[*idx] = weight;
            }
        }

        positions
    }

    /// Calculate portfolio return given positions and asset returns.
    fn calculate_portfolio_return(&self, positions: &[f64], returns: &[f64]) -> f64 {
        positions
            .iter()
            .zip(returns.iter())
            .map(|(&pos, &ret)| {
                if pos.is_finite() && ret.is_finite() {
                    pos * ret
                } else {
                    0.0
                }
            })
            .sum()
    }

    /// Calculate turnover between old and new positions.
    fn calculate_turnover(&self, old_positions: &[f64], new_positions: &[f64]) -> f64 {
        old_positions
            .iter()
            .zip(new_positions.iter())
            .map(|(&old, &new)| (new - old).abs())
            .sum::<f64>()
            / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== BacktestConfig Tests ====================

    #[test]
    fn test_backtest_config_default() {
        let config = BacktestConfig::default();
        assert_eq!(config.rebalance_frequency, 21);
        assert_eq!(config.transaction_cost_bps, 10.0);
        assert_eq!(config.initial_capital, 1_000_000.0);
        assert!(config.long_short);
        assert_eq!(config.n_long, Some(50));
        assert_eq!(config.n_short, Some(50));
        assert_eq!(config.max_position_size, 0.1);
        assert_eq!(config.min_position_size, 0.0);
    }

    #[test]
    fn test_backtest_config_custom() {
        let config = BacktestConfig {
            start_date: NaiveDate::from_ymd_opt(2022, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
            rebalance_frequency: 5,
            transaction_cost_bps: 25.0,
            initial_capital: 500_000.0,
            max_position_size: 0.2,
            min_position_size: 0.01,
            n_long: Some(10),
            n_short: Some(5),
            long_short: false,
        };
        assert_eq!(config.rebalance_frequency, 5);
        assert_eq!(config.transaction_cost_bps, 25.0);
        assert!(!config.long_short);
    }

    // ==================== Sharpe Ratio Tests ====================

    #[test]
    fn test_calculate_sharpe() {
        let returns = vec![0.01, -0.005, 0.015, 0.002, -0.003];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe.is_finite());
    }

    #[test]
    fn test_calculate_sharpe_positive() {
        // Consistently positive returns should have positive Sharpe
        let returns = vec![0.01, 0.02, 0.015, 0.01, 0.012];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe > 0.0);
    }

    #[test]
    fn test_calculate_sharpe_negative() {
        // Consistently negative returns should have negative Sharpe
        let returns = vec![-0.01, -0.02, -0.015, -0.01, -0.012];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe < 0.0);
    }

    #[test]
    fn test_calculate_sharpe_zero_std() {
        // All identical returns should return NaN (zero std dev)
        let returns = vec![0.01, 0.01, 0.01, 0.01];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe.is_nan());
    }

    #[test]
    fn test_calculate_sharpe_empty() {
        let returns: Vec<f64> = vec![];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe.is_nan());
    }

    #[test]
    fn test_calculate_sharpe_single_return() {
        let returns = vec![0.01];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe.is_nan());
    }

    #[test]
    fn test_calculate_sharpe_with_nans() {
        let returns = vec![0.01, f64::NAN, 0.015, f64::NAN, -0.003];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        // Should filter NaNs and still compute valid Sharpe
        assert!(sharpe.is_finite());
    }

    #[test]
    fn test_calculate_sharpe_all_nans() {
        let returns = vec![f64::NAN, f64::NAN, f64::NAN];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe.is_nan());
    }

    #[test]
    fn test_calculate_sharpe_annualization() {
        let returns = vec![0.01, -0.005, 0.015, 0.002, -0.003];
        let sharpe_252 = BacktestResult::calculate_sharpe(&returns, 252);
        let sharpe_12 = BacktestResult::calculate_sharpe(&returns, 12);
        // Annualized Sharpe scales with sqrt of trading days
        let ratio = sharpe_252 / sharpe_12;
        let expected_ratio = (252.0_f64 / 12.0).sqrt();
        assert!((ratio - expected_ratio).abs() < 1e-10);
    }

    // ==================== Max Drawdown Tests ====================

    #[test]
    fn test_calculate_max_drawdown() {
        let cumulative = vec![0.0, 0.1, 0.15, 0.05, 0.08, 0.12];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        assert!(max_dd > 0.0);
        assert!(max_dd < 1.0);
    }

    #[test]
    fn test_calculate_max_drawdown_no_drawdown() {
        // Monotonically increasing: no drawdown
        let cumulative = vec![0.0, 0.05, 0.1, 0.15, 0.2];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        assert!((max_dd - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_max_drawdown_full_loss() {
        // Goes to -1.0 (100% loss)
        let cumulative = vec![0.0, 0.1, -1.0];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        // From peak 0.1 to -1.0: dd = (0.1 - (-1.0)) / (1 + 0.1) = 1.1 / 1.1 = 1.0
        assert!((max_dd - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_max_drawdown_empty() {
        let cumulative: Vec<f64> = vec![];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        assert!((max_dd - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_max_drawdown_single_value() {
        let cumulative = vec![0.1];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        assert!((max_dd - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_max_drawdown_multiple_drawdowns() {
        // Two separate drawdowns, second is larger
        let cumulative = vec![0.0, 0.1, 0.05, 0.15, 0.20, 0.08, 0.18];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        // From peak 0.20 to 0.08: dd = (0.20 - 0.08) / (1 + 0.20) = 0.12 / 1.20 = 0.1
        assert!(max_dd > 0.0);
        assert!(max_dd < 0.15);
    }

    // ==================== Portfolio Construction Tests ====================

    #[test]
    fn test_construct_portfolio_long_short() {
        let config = BacktestConfig {
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let scores = vec![0.5, -0.3, 0.8, -0.6, 0.1];
        let positions = backtest.construct_portfolio(&scores);

        // Check that we have 2 long and 2 short positions
        let n_long = positions.iter().filter(|&&p| p > 0.0).count();
        let n_short = positions.iter().filter(|&&p| p < 0.0).count();
        assert_eq!(n_long, 2);
        assert_eq!(n_short, 2);

        // Verify weights sum to approximately zero (dollar neutral)
        let sum: f64 = positions.iter().sum();
        assert!(sum.abs() < 1e-10);
    }

    #[test]
    fn test_construct_portfolio_long_only() {
        let config = BacktestConfig {
            n_long: Some(3),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let scores = vec![0.5, -0.3, 0.8, -0.6, 0.1];
        let positions = backtest.construct_portfolio(&scores);

        // Check that we have only long positions
        let n_long = positions.iter().filter(|&&p| p > 0.0).count();
        let n_short = positions.iter().filter(|&&p| p < 0.0).count();
        assert_eq!(n_long, 3);
        assert_eq!(n_short, 0);

        // Verify weights sum to 1.0 (fully invested)
        let sum: f64 = positions.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_construct_portfolio_empty_scores() {
        let config = BacktestConfig::default();
        let backtest = Backtest::new(config);

        let scores: Vec<f64> = vec![];
        let positions = backtest.construct_portfolio(&scores);

        assert!(positions.is_empty());
    }

    #[test]
    fn test_construct_portfolio_single_asset() {
        let config = BacktestConfig {
            n_long: Some(1),
            n_short: Some(0),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let scores = vec![0.5];
        let positions = backtest.construct_portfolio(&scores);

        assert_eq!(positions.len(), 1);
        assert!((positions[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_construct_portfolio_all_nan_scores() {
        let config = BacktestConfig::default();
        let backtest = Backtest::new(config);

        let scores = vec![f64::NAN, f64::NAN, f64::NAN];
        let positions = backtest.construct_portfolio(&scores);

        // All positions should be zero when all scores are NaN
        assert!(positions.iter().all(|&p| p == 0.0));
    }

    #[test]
    fn test_construct_portfolio_some_nan_scores() {
        let config = BacktestConfig {
            n_long: Some(2),
            n_short: Some(1),
            long_short: true,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let scores = vec![f64::NAN, 0.5, 0.8, f64::NAN, -0.3];
        let positions = backtest.construct_portfolio(&scores);

        // Should only use valid (finite) scores for portfolio construction
        let n_long = positions.iter().filter(|&&p| p > 0.0).count();
        let n_short = positions.iter().filter(|&&p| p < 0.0).count();
        assert_eq!(n_long, 2);
        assert_eq!(n_short, 1);
    }

    #[test]
    fn test_construct_portfolio_n_long_greater_than_assets() {
        let config = BacktestConfig {
            n_long: Some(10), // More than available assets
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let scores = vec![0.5, 0.3, 0.8];
        let positions = backtest.construct_portfolio(&scores);

        // Should use all available assets
        let n_long = positions.iter().filter(|&&p| p > 0.0).count();
        assert_eq!(n_long, 3);

        // Weights are 1/n_long (1/10), not 1/n_assets, so sum is 3/10 = 0.3
        // This is the actual behavior: weight = 1/n_long regardless of available assets
        let sum: f64 = positions.iter().sum();
        assert!((sum - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_construct_portfolio_weights_normalized() {
        let config = BacktestConfig {
            n_long: Some(5),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let scores = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let positions = backtest.construct_portfolio(&scores);

        // Each position should be 0.2 (1/5)
        for &pos in &positions {
            assert!((pos - 0.2).abs() < 1e-10);
        }
    }

    #[test]
    fn test_construct_portfolio_correct_assets_selected() {
        let config = BacktestConfig {
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // Scores: index 2 (0.8) and 0 (0.5) are top, index 3 (-0.6) and 1 (-0.3) are bottom
        let scores = vec![0.5, -0.3, 0.8, -0.6, 0.1];
        let positions = backtest.construct_portfolio(&scores);

        // Top 2: index 2 and 0 should be long
        assert!(positions[2] > 0.0);
        assert!(positions[0] > 0.0);

        // Bottom 2: index 3 and 1 should be short
        assert!(positions[3] < 0.0);
        assert!(positions[1] < 0.0);
    }

    // ==================== Turnover Tests ====================

    #[test]
    fn test_calculate_turnover() {
        let backtest = Backtest::default();
        let old_pos = vec![0.5, 0.3, 0.2];
        let new_pos = vec![0.4, 0.4, 0.2];
        let turnover = backtest.calculate_turnover(&old_pos, &new_pos);
        assert!((turnover - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_turnover_zero() {
        let backtest = Backtest::default();
        let old_pos = vec![0.5, 0.3, 0.2];
        let new_pos = vec![0.5, 0.3, 0.2];
        let turnover = backtest.calculate_turnover(&old_pos, &new_pos);
        assert!((turnover - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_turnover_full() {
        let backtest = Backtest::default();
        let old_pos = vec![1.0, 0.0, 0.0];
        let new_pos = vec![0.0, 0.0, 1.0];
        let turnover = backtest.calculate_turnover(&old_pos, &new_pos);
        // Total change: |0-1| + |0-0| + |1-0| = 2, divided by 2 = 1.0
        assert!((turnover - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_turnover_long_short() {
        let backtest = Backtest::default();
        let old_pos = vec![0.5, -0.5, 0.0];
        let new_pos = vec![0.0, 0.0, 0.5];
        let turnover = backtest.calculate_turnover(&old_pos, &new_pos);
        // Total change: |0-0.5| + |0-(-0.5)| + |0.5-0| = 0.5 + 0.5 + 0.5 = 1.5, divided by 2 = 0.75
        assert!((turnover - 0.75).abs() < 1e-10);
    }

    // ==================== Portfolio Return Tests ====================

    #[test]
    fn test_calculate_portfolio_return_simple() {
        let backtest = Backtest::default();
        let positions = vec![0.5, 0.3, 0.2];
        let returns = vec![0.10, 0.05, -0.02];
        let port_ret = backtest.calculate_portfolio_return(&positions, &returns);
        // Expected: 0.5*0.10 + 0.3*0.05 + 0.2*(-0.02) = 0.05 + 0.015 - 0.004 = 0.061
        assert!((port_ret - 0.061).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_portfolio_return_long_short() {
        let backtest = Backtest::default();
        let positions = vec![0.5, -0.5];
        let returns = vec![0.10, -0.10];
        let port_ret = backtest.calculate_portfolio_return(&positions, &returns);
        // Expected: 0.5*0.10 + (-0.5)*(-0.10) = 0.05 + 0.05 = 0.10
        assert!((port_ret - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_portfolio_return_with_nan() {
        let backtest = Backtest::default();
        let positions = vec![0.5, 0.3, 0.2];
        let returns = vec![0.10, f64::NAN, -0.02];
        let port_ret = backtest.calculate_portfolio_return(&positions, &returns);
        // NaN returns should contribute 0
        assert!((port_ret - 0.046).abs() < 1e-10); // 0.5*0.10 + 0 + 0.2*(-0.02)
    }

    #[test]
    fn test_calculate_portfolio_return_empty() {
        let backtest = Backtest::default();
        let positions: Vec<f64> = vec![];
        let returns: Vec<f64> = vec![];
        let port_ret = backtest.calculate_portfolio_return(&positions, &returns);
        assert!((port_ret - 0.0).abs() < 1e-10);
    }

    // ==================== Full Backtest Run Tests ====================

    fn create_test_dates(n_periods: usize) -> Vec<NaiveDate> {
        (0..n_periods)
            .map(|i| {
                NaiveDate::from_ymd_opt(2020, 1, 1)
                    .unwrap()
                    .checked_add_signed(chrono::Duration::days(i as i64))
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn test_backtest_run_basic() {
        let config = BacktestConfig {
            rebalance_frequency: 1,    // Daily rebalancing
            transaction_cost_bps: 0.0, // No transaction costs for simplicity
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // 5 days, 4 assets
        let signal_scores = vec![
            vec![0.8, 0.6, -0.5, -0.7],
            vec![0.7, 0.9, -0.4, -0.6],
            vec![0.6, 0.7, -0.3, -0.8],
            vec![0.9, 0.5, -0.6, -0.4],
            vec![0.5, 0.8, -0.7, -0.5],
        ];
        let returns = vec![
            vec![0.01, 0.02, -0.01, -0.02],
            vec![0.015, 0.01, -0.015, -0.01],
            vec![0.02, 0.015, -0.02, -0.015],
            vec![0.01, 0.02, -0.01, -0.02],
            vec![0.015, 0.01, -0.015, -0.01],
        ];
        let dates = create_test_dates(5);

        let result = backtest.run(&signal_scores, &returns, &dates);

        assert_eq!(result.returns.len(), 5);
        assert_eq!(result.cumulative_returns.len(), 5);
        assert!(result.sharpe_ratio.is_finite());
        assert!(result.max_drawdown >= 0.0);
    }

    #[test]
    fn test_backtest_run_with_transaction_costs() {
        let config_with_costs = BacktestConfig {
            rebalance_frequency: 1,
            transaction_cost_bps: 50.0, // 50 bps = 0.5%
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };

        let config_no_costs = BacktestConfig {
            rebalance_frequency: 1,
            transaction_cost_bps: 0.0,
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };

        let backtest_with_costs = Backtest::new(config_with_costs);
        let backtest_no_costs = Backtest::new(config_no_costs);

        // Use 5 periods to ensure we have enough rebalances after the initial position
        let signal_scores = vec![
            vec![0.8, 0.6, -0.5, -0.7],
            vec![0.7, 0.9, -0.4, -0.6], // Rebalance here (different from initial)
            vec![0.6, 0.7, -0.3, -0.8], // Rebalance here
            vec![0.9, 0.5, -0.2, -0.9], // Rebalance here
            vec![0.5, 0.8, -0.6, -0.4], // Rebalance here
        ];
        let returns = vec![
            vec![0.01, 0.02, -0.01, -0.02],
            vec![0.015, 0.01, -0.015, -0.01],
            vec![0.02, 0.015, -0.02, -0.015],
            vec![0.01, 0.02, -0.01, -0.02],
            vec![0.015, 0.01, -0.015, -0.01],
        ];
        let dates = create_test_dates(5);

        let result_with_costs = backtest_with_costs.run(&signal_scores, &returns, &dates);
        let result_no_costs = backtest_no_costs.run(&signal_scores, &returns, &dates);

        // With transaction costs, total costs should be tracked (after initial position)
        // n_trades counts rebalances after the first one (periods 1,2,3,4 = 4 trades)
        assert_eq!(result_with_costs.n_trades, 4);
        assert!(result_with_costs.total_transaction_costs > 0.0);
        assert!((result_no_costs.total_transaction_costs - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_backtest_run_empty_data() {
        let config = BacktestConfig::default();
        let backtest = Backtest::new(config);

        let signal_scores: Vec<Vec<f64>> = vec![];
        let returns: Vec<Vec<f64>> = vec![];
        let dates: Vec<NaiveDate> = vec![];

        let result = backtest.run(&signal_scores, &returns, &dates);

        assert!(result.returns.is_empty());
        assert!(result.cumulative_returns.is_empty());
        assert!(result.sharpe_ratio.is_nan());
        assert!((result.total_return - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_backtest_run_single_period() {
        let config = BacktestConfig {
            rebalance_frequency: 1,
            n_long: Some(2),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        let signal_scores = vec![vec![0.8, 0.6, 0.3]];
        let returns = vec![vec![0.01, 0.02, 0.015]];
        let dates = create_test_dates(1);

        let result = backtest.run(&signal_scores, &returns, &dates);

        assert_eq!(result.returns.len(), 1);
        assert_eq!(result.n_trades, 0); // No rebalance from previous positions
    }

    #[test]
    fn test_backtest_run_rebalance_frequency() {
        let config = BacktestConfig {
            rebalance_frequency: 3, // Rebalance every 3 days
            transaction_cost_bps: 10.0,
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // 10 periods
        let signal_scores: Vec<Vec<f64>> = (0..10)
            .map(|i| vec![0.5 + (i as f64) * 0.01, 0.3, -0.4, -0.6])
            .collect();
        let returns: Vec<Vec<f64>> = (0..10).map(|_| vec![0.01, 0.005, -0.005, -0.01]).collect();
        let dates = create_test_dates(10);

        let result = backtest.run(&signal_scores, &returns, &dates);

        // Should rebalance at periods 0, 3, 6, 9 (4 times)
        // But first rebalance (period 0) doesn't count as a trade since there are no previous positions
        // So trades happen at 3, 6, 9 = 3 trades
        assert_eq!(result.n_trades, 3);
    }

    #[test]
    fn test_backtest_run_annualized_return() {
        let config = BacktestConfig {
            rebalance_frequency: 1,
            transaction_cost_bps: 0.0,
            n_long: Some(2),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // 252 periods (1 year) with consistent 1% daily return
        let n_periods = 252;
        let signal_scores: Vec<Vec<f64>> = (0..n_periods).map(|_| vec![0.8, 0.6]).collect();
        let returns: Vec<Vec<f64>> = (0..n_periods).map(|_| vec![0.01, 0.01]).collect();
        let dates = create_test_dates(n_periods);

        let result = backtest.run(&signal_scores, &returns, &dates);

        // With ~1% daily return, annualized should be very high
        assert!(result.annualized_return > 0.0);
        assert!(result.total_return > 0.0);
    }

    #[test]
    fn test_backtest_run_volatility() {
        let config = BacktestConfig {
            rebalance_frequency: 1,
            transaction_cost_bps: 0.0,
            n_long: Some(2),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // Alternating returns create volatility
        let signal_scores: Vec<Vec<f64>> = (0..20).map(|_| vec![0.8, 0.6]).collect();
        let returns: Vec<Vec<f64>> = (0..20)
            .map(|i| {
                if i % 2 == 0 {
                    vec![0.02, 0.02]
                } else {
                    vec![-0.02, -0.02]
                }
            })
            .collect();
        let dates = create_test_dates(20);

        let result = backtest.run(&signal_scores, &returns, &dates);

        // Should have positive volatility
        assert!(result.annualized_volatility > 0.0);
    }

    #[test]
    fn test_backtest_run_ic_history() {
        let config = BacktestConfig {
            rebalance_frequency: 1,
            transaction_cost_bps: 0.0,
            n_long: Some(2),
            n_short: Some(2),
            long_short: true,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // Perfect correlation between signals and next-period returns
        let signal_scores = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 3.0, 4.0],
        ];
        let returns = vec![
            vec![0.01, 0.02, 0.03, 0.04],
            vec![0.01, 0.02, 0.03, 0.04],
            vec![0.01, 0.02, 0.03, 0.04],
            vec![0.01, 0.02, 0.03, 0.04],
        ];
        let dates = create_test_dates(4);

        let result = backtest.run(&signal_scores, &returns, &dates);

        // IC history should have n_periods - 1 entries
        assert_eq!(result.ic_history.len(), 3);

        // With perfect rank correlation, IC should be 1.0
        for ic in &result.ic_history {
            assert!((*ic - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_backtest_run_avg_turnover() {
        let config = BacktestConfig {
            rebalance_frequency: 1,
            transaction_cost_bps: 0.0,
            n_long: Some(2),
            n_short: Some(0),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // Signals that flip each period
        let signal_scores = vec![
            vec![0.9, 0.8, 0.1, 0.2],
            vec![0.1, 0.2, 0.9, 0.8],
            vec![0.9, 0.8, 0.1, 0.2],
            vec![0.1, 0.2, 0.9, 0.8],
        ];
        let returns: Vec<Vec<f64>> = (0..4).map(|_| vec![0.01, 0.01, 0.01, 0.01]).collect();
        let dates = create_test_dates(4);

        let result = backtest.run(&signal_scores, &returns, &dates);

        // Should have high turnover due to flipping positions
        assert!(result.avg_turnover > 0.0);
    }

    #[test]
    fn test_backtest_run_mismatched_lengths() {
        let config = BacktestConfig {
            rebalance_frequency: 1,
            n_long: Some(2),
            long_short: false,
            ..Default::default()
        };
        let backtest = Backtest::new(config);

        // Different lengths for signals, returns, and dates
        let signal_scores = vec![
            vec![0.8, 0.6],
            vec![0.7, 0.9],
            vec![0.6, 0.7],
            vec![0.9, 0.5],
            vec![0.5, 0.8],
        ];
        let returns = vec![vec![0.01, 0.02], vec![0.015, 0.01], vec![0.02, 0.015]];
        let dates = create_test_dates(4);

        let result = backtest.run(&signal_scores, &returns, &dates);

        // Should use minimum length (3 from returns)
        assert_eq!(result.returns.len(), 3);
    }

    #[test]
    fn test_backtest_new_const() {
        // Verify that Backtest::new is const
        const _BACKTEST: Backtest = Backtest::new(BacktestConfig {
            start_date: match NaiveDate::from_ymd_opt(2020, 1, 1) {
                Some(d) => d,
                None => panic!("Invalid date"),
            },
            end_date: match NaiveDate::from_ymd_opt(2024, 12, 31) {
                Some(d) => d,
                None => panic!("Invalid date"),
            },
            rebalance_frequency: 21,
            transaction_cost_bps: 10.0,
            initial_capital: 1_000_000.0,
            max_position_size: 0.1,
            min_position_size: 0.0,
            n_long: Some(50),
            n_short: Some(50),
            long_short: true,
        });
    }

    #[test]
    fn test_backtest_default() {
        let backtest = Backtest::default();
        // Default should have default config
        let signal_scores = vec![vec![0.8, 0.6, 0.3]];
        let returns = vec![vec![0.01, 0.02, 0.015]];
        let dates = create_test_dates(1);

        let result = backtest.run(&signal_scores, &returns, &dates);
        assert_eq!(result.returns.len(), 1);
    }

    #[test]
    fn test_backtest_result_serialization() {
        let result = BacktestResult {
            returns: vec![0.01, 0.02],
            cumulative_returns: vec![0.01, 0.0302],
            sharpe_ratio: 1.5,
            max_drawdown: 0.05,
            total_return: 0.0302,
            annualized_return: 0.15,
            annualized_volatility: 0.10,
            avg_turnover: 0.25,
            ic_history: vec![0.05, 0.06],
            total_transaction_costs: 0.001,
            n_trades: 5,
        };

        // Verify serialization works
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: BacktestResult = serde_json::from_str(&serialized).unwrap();

        assert!((deserialized.sharpe_ratio - 1.5).abs() < 1e-10);
        assert_eq!(deserialized.n_trades, 5);
    }

    #[test]
    fn test_backtest_config_serialization() {
        let config = BacktestConfig::default();

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: BacktestConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.rebalance_frequency, config.rebalance_frequency);
        assert_eq!(
            deserialized.transaction_cost_bps,
            config.transaction_cost_bps
        );
    }
}
