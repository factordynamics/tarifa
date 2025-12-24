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
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
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
        indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

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

    #[test]
    fn test_backtest_config_default() {
        let config = BacktestConfig::default();
        assert_eq!(config.rebalance_frequency, 21);
        assert_eq!(config.transaction_cost_bps, 10.0);
        assert_eq!(config.initial_capital, 1_000_000.0);
        assert!(config.long_short);
    }

    #[test]
    fn test_calculate_sharpe() {
        let returns = vec![0.01, -0.005, 0.015, 0.002, -0.003];
        let sharpe = BacktestResult::calculate_sharpe(&returns, 252);
        assert!(sharpe.is_finite());
    }

    #[test]
    fn test_calculate_max_drawdown() {
        let cumulative = vec![0.0, 0.1, 0.15, 0.05, 0.08, 0.12];
        let max_dd = BacktestResult::calculate_max_drawdown(&cumulative);
        assert!(max_dd > 0.0);
        assert!(max_dd < 1.0);
    }

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
    }

    #[test]
    fn test_calculate_turnover() {
        let backtest = Backtest::default();
        let old_pos = vec![0.5, 0.3, 0.2];
        let new_pos = vec![0.4, 0.4, 0.2];
        let turnover = backtest.calculate_turnover(&old_pos, &new_pos);
        assert!((turnover - 0.1).abs() < 1e-10);
    }
}
