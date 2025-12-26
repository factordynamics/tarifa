//! Quality strategy backtest using net profit margin.
//!
//! This example demonstrates:
//! - Fetching fundamental data (financial ratios) from FMP API
//! - Computing quality signals based on net profit margin
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

/// Portfolio positions.
const LONG_COUNT: usize = 3;
const SHORT_COUNT: usize = 3;

/// Rebalance frequency in days (quarterly).
const REBALANCE_DAYS: usize = 63;

#[derive(Clone)]
struct PriceData {
    dates: Vec<String>,
    closes: Vec<f64>,
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

    // Fetch quality scores (net profit margin) for all stocks
    let mut quality_scores = HashMap::new();
    for symbol in UNIVERSE {
        if let Ok(ratios) = client.ratios(symbol, Period::Annual, Some(1)).await {
            if let Some(ratio) = ratios.first() {
                let margin = ratio.net_profit_margin;
                // Skip invalid values
                if margin.is_finite() && margin != 0.0 {
                    quality_scores.insert(symbol.to_string(), margin);
                }
            }
        }
    }

    if quality_scores.len() < LONG_COUNT + SHORT_COUNT {
        return Err(format!(
            "Insufficient quality data: need at least {}, got {}",
            LONG_COUNT + SHORT_COUNT,
            quality_scores.len()
        )
        .into());
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
                eprintln!("Warning: Failed to fetch prices for {symbol}: {e}");
            }
        }
    }

    if price_data.is_empty() {
        return Err("No price data available. Check API key and network connection.".into());
    }

    // Get common dates across all stocks
    let common_dates = find_common_dates(&price_data);
    if common_dates.len() < 2 {
        return Err(format!(
            "Insufficient price data: need at least 2 days, got {}",
            common_dates.len()
        )
        .into());
    }

    // Run backtest
    let returns = backtest(&price_data, &quality_scores, &common_dates);

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

/// Run backtest: long top N quality, short bottom N quality, rebalance quarterly.
fn backtest(
    price_data: &HashMap<String, PriceData>,
    quality_scores: &HashMap<String, f64>,
    dates: &[String],
) -> Vec<f64> {
    let mut portfolio_returns = Vec::new();
    let mut last_rebalance = 0;
    let mut positions: Vec<(String, f64)> = Vec::new(); // (symbol, weight)

    for (i, date) in dates.iter().enumerate() {
        // Rebalance portfolio
        if i - last_rebalance >= REBALANCE_DAYS || positions.is_empty() {
            // Rank stocks by quality (higher margin = better quality)
            let mut signals: Vec<(String, f64)> = quality_scores
                .iter()
                .filter(|(symbol, _)| price_data.contains_key(*symbol))
                .map(|(symbol, score)| (symbol.clone(), *score))
                .collect();

            if signals.len() < LONG_COUNT + SHORT_COUNT {
                continue;
            }

            // Sort by quality score descending (highest margin first)
            signals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Select long (top quality) and short (bottom quality) positions
            positions.clear();
            let weight = 1.0 / (LONG_COUNT + SHORT_COUNT) as f64;
            for j in 0..LONG_COUNT {
                positions.push((signals[j].0.clone(), weight));
            }
            for j in 0..SHORT_COUNT {
                let idx = signals.len() - 1 - j;
                positions.push((signals[idx].0.clone(), -weight));
            }

            last_rebalance = i;
        }

        // Calculate daily portfolio return
        if i > 0 {
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

    // Total return (compounded)
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

    let start_idx = if dates.len() > returns.len() {
        dates.len() - returns.len() - 1
    } else {
        0
    };

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
    println!("\nQuality Strategy (Net Margin)");
    println!("═════════════════════════════");
    println!("Period:     {} to {}", stats.start_date, stats.end_date);
    println!("Universe:   {} stocks", stats.universe_size);
    println!();
    println!("Performance:");
    println!("  Total Return:    {:+.1}%", stats.total_return * 100.0);
    println!("  Sharpe Ratio:    {:.2}", stats.sharpe_ratio);
    println!("  Max Drawdown:    {:.1}%", stats.max_drawdown * 100.0);
    println!("  Win Rate:        {:.0}%", stats.win_rate * 100.0);
}
