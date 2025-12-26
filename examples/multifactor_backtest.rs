//! Multi-factor strategy backtest combining Momentum, Value, and Quality.
//!
//! This example demonstrates:
//! - Fetching historical prices and fundamental data from FMP API
//! - Computing multi-factor signals (momentum + earnings yield + net margin)
//! - Running a long/short backtest (long top 3, short bottom 3)
//! - Computing performance metrics (returns, Sharpe, drawdown, win rate)

use std::collections::HashMap;
use tarifa_fmp::{FmpClient, Period};

/// Stock universe to backtest.
const UNIVERSE: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "JPM", "V", "WMT",
];

/// Backtest period.
const START_DATE: &str = "2023-01-01";
const END_DATE: &str = "2024-12-01";

/// Momentum lookback in trading days (approx 6 months).
const MOMENTUM_DAYS: usize = 126;

/// Portfolio positions.
const LONG_COUNT: usize = 3;
const SHORT_COUNT: usize = 3;

/// Rebalance frequency in days (monthly).
const REBALANCE_DAYS: usize = 21;

#[derive(Clone)]
struct PriceData {
    dates: Vec<String>,
    closes: Vec<f64>,
}

#[derive(Clone, Default)]
struct FactorScores {
    earnings_yield: f64,
    net_margin: f64,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FMP client
    let client = FmpClient::from_env()
        .map_err(|_| "Failed to initialize FMP client. Set FMP_API_KEY environment variable.")?;

    // Fetch fundamental factor scores for all stocks
    let mut factor_scores = HashMap::new();
    for symbol in UNIVERSE {
        let mut scores = FactorScores::default();

        if let Ok(metrics) = client.key_metrics(symbol, Period::Annual, Some(1)).await {
            if let Some(m) = metrics.first() {
                if m.earnings_yield.is_finite() {
                    scores.earnings_yield = m.earnings_yield;
                }
            }
        }

        if let Ok(ratios) = client.ratios(symbol, Period::Annual, Some(1)).await {
            if let Some(r) = ratios.first() {
                if r.net_profit_margin.is_finite() {
                    scores.net_margin = r.net_profit_margin;
                }
            }
        }

        factor_scores.insert(symbol.to_string(), scores);
    }

    // Fetch historical prices for all stocks
    let mut price_data = HashMap::new();
    for symbol in UNIVERSE {
        match client
            .historical_prices(symbol, Some(START_DATE), Some(END_DATE))
            .await
        {
            Ok(mut prices) => {
                // Sort by date ascending
                prices.sort_by(|a, b| a.date.cmp(&b.date));
                if !prices.is_empty() {
                    let dates: Vec<String> = prices.iter().map(|p| p.date.clone()).collect();
                    let closes: Vec<f64> = prices.iter().map(|p| p.close).collect();
                    price_data.insert(symbol.to_string(), PriceData { dates, closes });
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to fetch {symbol}: {e}");
            }
        }
    }

    if price_data.is_empty() {
        return Err("No price data available. Check API key and network connection.".into());
    }

    // Get common dates across all stocks
    let common_dates = find_common_dates(&price_data);
    if common_dates.len() < MOMENTUM_DAYS + 1 {
        return Err(format!(
            "Insufficient data: need at least {} days, got {}",
            MOMENTUM_DAYS + 1,
            common_dates.len()
        )
        .into());
    }

    // Run backtest
    let returns = backtest(&price_data, &factor_scores, &common_dates);

    // Compute performance metrics
    let stats = compute_stats(&returns, &common_dates);

    // Print results
    print_results(&stats);

    Ok(())
}

/// Find common trading dates across all stocks.
fn find_common_dates(price_data: &HashMap<String, PriceData>) -> Vec<String> {
    let mut date_counts: HashMap<String, usize> = HashMap::new();
    for data in price_data.values() {
        for date in &data.dates {
            *date_counts.entry(date.clone()).or_insert(0) += 1;
        }
    }

    let total_stocks = price_data.len();
    let mut common: Vec<String> = date_counts
        .into_iter()
        .filter(|(_, count)| *count == total_stocks)
        .map(|(date, _)| date)
        .collect();
    common.sort();
    common
}

/// Calculate momentum for a stock at a given date.
fn momentum(data: &PriceData, date: &str, lookback: usize) -> Option<f64> {
    let idx = data.dates.iter().position(|d| d == date)?;
    if idx < lookback {
        return None;
    }
    let current = data.closes[idx];
    let past = data.closes[idx - lookback];
    if past > 0.0 {
        Some(current / past - 1.0)
    } else {
        None
    }
}

/// Calculate z-score of a value within a distribution.
fn z_score(value: f64, values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std = variance.sqrt();
    if std < 1e-10 {
        return 0.0;
    }
    (value - mean) / std
}

/// Run backtest: long top N, short bottom N based on multi-factor composite score.
fn backtest(
    price_data: &HashMap<String, PriceData>,
    factor_scores: &HashMap<String, FactorScores>,
    dates: &[String],
) -> Vec<f64> {
    let mut portfolio_returns = Vec::new();
    let mut last_rebalance = 0;
    let mut positions: Vec<(String, f64)> = Vec::new(); // (symbol, weight)

    for (i, date) in dates.iter().enumerate() {
        if i < MOMENTUM_DAYS {
            continue;
        }

        // Rebalance portfolio
        if i - last_rebalance >= REBALANCE_DAYS || positions.is_empty() {
            // Calculate momentum for all stocks at this date
            let momentums: HashMap<String, f64> = price_data
                .iter()
                .filter_map(|(symbol, data)| {
                    momentum(data, date, MOMENTUM_DAYS).map(|mom| (symbol.clone(), mom))
                })
                .collect();

            if momentums.len() < LONG_COUNT + SHORT_COUNT {
                continue;
            }

            // Collect all factor values for z-score calculation
            let mom_values: Vec<f64> = momentums.values().copied().collect();
            let ey_values: Vec<f64> = factor_scores
                .iter()
                .filter(|(s, _)| momentums.contains_key(*s))
                .map(|(_, f)| f.earnings_yield)
                .collect();
            let margin_values: Vec<f64> = factor_scores
                .iter()
                .filter(|(s, _)| momentums.contains_key(*s))
                .map(|(_, f)| f.net_margin)
                .collect();

            // Calculate composite z-scores for each stock
            let mut composite_scores: Vec<(String, f64)> = momentums
                .iter()
                .filter_map(|(symbol, &mom)| {
                    let scores = factor_scores.get(symbol)?;
                    let mom_z = z_score(mom, &mom_values);
                    let ey_z = z_score(scores.earnings_yield, &ey_values);
                    let margin_z = z_score(scores.net_margin, &margin_values);
                    // Equal weight: 1/3 each factor
                    let composite = (mom_z + ey_z + margin_z) / 3.0;
                    Some((symbol.clone(), composite))
                })
                .collect();

            // Sort by composite score descending
            composite_scores
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Select long (top) and short (bottom) positions
            positions.clear();
            let weight = 1.0 / (LONG_COUNT + SHORT_COUNT) as f64;
            for j in 0..LONG_COUNT {
                positions.push((composite_scores[j].0.clone(), weight));
            }
            for j in 0..SHORT_COUNT {
                let idx = composite_scores.len() - 1 - j;
                positions.push((composite_scores[idx].0.clone(), -weight));
            }

            last_rebalance = i;
        }

        // Calculate daily portfolio return
        let mut daily_return = 0.0;
        for (symbol, weight) in &positions {
            if let Some(data) = price_data.get(symbol) {
                if let Some(idx) = data.dates.iter().position(|d| d == date) {
                    if idx > 0 && data.closes[idx - 1] > 0.0 {
                        let ret = data.closes[idx] / data.closes[idx - 1] - 1.0;
                        daily_return += weight * ret;
                    }
                }
            }
        }
        portfolio_returns.push(daily_return);
    }

    portfolio_returns
}

struct PerformanceStats {
    total_return: f64,
    sharpe_ratio: f64,
    max_drawdown: f64,
    win_rate: f64,
    start_date: String,
    end_date: String,
    universe_size: usize,
}

/// Compute performance statistics.
fn compute_stats(returns: &[f64], dates: &[String]) -> PerformanceStats {
    if returns.is_empty() {
        return PerformanceStats {
            total_return: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            win_rate: 0.0,
            start_date: String::new(),
            end_date: String::new(),
            universe_size: 0,
        };
    }

    // Total return
    let total_return: f64 = returns.iter().map(|r| 1.0 + r).product::<f64>() - 1.0;

    // Sharpe ratio (annualized)
    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let std_return = variance.sqrt();
    let sharpe_ratio = if std_return > 0.0 {
        mean_return / std_return * (252.0_f64).sqrt()
    } else {
        0.0
    };

    // Maximum drawdown
    let mut cumulative = 1.0;
    let mut peak = 1.0;
    let mut max_dd = 0.0;
    for r in returns {
        cumulative *= 1.0 + r;
        if cumulative > peak {
            peak = cumulative;
        }
        let dd = (peak - cumulative) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    // Win rate
    let wins = returns.iter().filter(|r| **r > 0.0).count();
    let win_rate = wins as f64 / returns.len() as f64;

    let start_idx = dates.len() - returns.len();
    PerformanceStats {
        total_return,
        sharpe_ratio,
        max_drawdown: max_dd,
        win_rate,
        start_date: dates[start_idx].clone(),
        end_date: dates[dates.len() - 1].clone(),
        universe_size: UNIVERSE.len(),
    }
}

/// Print performance results.
fn print_results(stats: &PerformanceStats) {
    println!("\nMulti-Factor Strategy (Mom+Val+Qual)");
    println!("════════════════════════════════════");
    println!("Period:     {} to {}", stats.start_date, stats.end_date);
    println!("Universe:   {} stocks", stats.universe_size);
    println!();
    println!("Factors:");
    println!("  Momentum (6M):  33%");
    println!("  Earnings Yield: 33%");
    println!("  Net Margin:     33%");
    println!();
    println!("Performance:");
    println!("  Total Return:    {:+.1}%", stats.total_return * 100.0);
    println!("  Sharpe Ratio:    {:.2}", stats.sharpe_ratio);
    println!("  Max Drawdown:    {:.1}%", stats.max_drawdown * 100.0);
    println!("  Win Rate:        {:.0}%", stats.win_rate * 100.0);
}
