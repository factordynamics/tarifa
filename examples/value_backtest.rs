//! Value Strategy Backtest
//!
//! Simple backtest of a value strategy using earnings yield and FCF yield.
//! Long top 3 value stocks, short bottom 3 expensive stocks, rebalance quarterly.
//!
//! ## Prerequisites
//!
//! Set your FMP API key in the environment or `.env` file:
//! ```bash
//! FMP_API_KEY=your_api_key_here
//! ```
//!
//! ## Running
//!
//! ```bash
//! cargo run --release --example value_backtest
//! ```

use std::collections::HashMap;
use tarifa::fmp::{FmpClient, FmpError, HistoricalPrice, Period};

/// Universe of stocks to backtest.
const UNIVERSE: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "JPM", "V", "UNH", "JNJ",
];

/// Backtest period.
const START_DATE: &str = "2023-01-01";
const END_DATE: &str = "2024-12-01";

/// Rebalance frequency (trading days).
const REBALANCE_DAYS: usize = 63; // Quarterly

#[derive(Debug, Clone)]
struct StockData {
    symbol: String,
    value_score: f64,
    prices: Vec<HistoricalPrice>,
}

#[derive(Debug)]
struct BacktestResult {
    total_return: f64,
    sharpe_ratio: f64,
    max_drawdown: f64,
    win_rate: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize FMP client
    let client = match FmpClient::from_env() {
        Ok(c) => c,
        Err(FmpError::MissingApiKey) => {
            eprintln!("Error: FMP_API_KEY not set.");
            eprintln!("\nTo run this example:");
            eprintln!("  1. Get a free API key at https://financialmodelingprep.com/");
            eprintln!("  2. Create a .env file with: FMP_API_KEY=your_key_here");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    // =========================================================================
    // 1. Fetch fundamental data and compute value scores
    // =========================================================================
    let mut stock_data: Vec<StockData> = Vec::new();

    for symbol in UNIVERSE {
        // Fetch key metrics for value signals
        let metrics = match client.key_metrics(symbol, Period::Annual, Some(1)).await {
            Ok(m) if !m.is_empty() => m,
            _ => {
                eprintln!("Warning: No metrics for {symbol}, skipping");
                continue;
            }
        };

        let earnings_yield = metrics[0].earnings_yield;
        let fcf_yield = metrics[0].free_cash_flow_yield;
        let value_score = earnings_yield + fcf_yield;

        // Fetch historical prices for backtest
        let prices = match client
            .historical_prices(symbol, Some(START_DATE), Some(END_DATE))
            .await
        {
            Ok(mut p) => {
                p.sort_by(|a, b| a.date.cmp(&b.date)); // Sort chronologically
                p
            }
            _ => {
                eprintln!("Warning: No price data for {symbol}, skipping");
                continue;
            }
        };

        stock_data.push(StockData {
            symbol: symbol.to_string(),
            value_score,
            prices,
        });
    }

    if stock_data.len() < 6 {
        eprintln!("Error: Not enough stocks with data (need at least 6)");
        return Ok(());
    }

    // =========================================================================
    // 2. Run backtest
    // =========================================================================

    // Rank by value score (higher = cheaper/better)
    stock_data.sort_by(|a, b| b.value_score.partial_cmp(&a.value_score).unwrap());

    let long_stocks: Vec<_> = stock_data.iter().take(3).collect();
    let short_stocks: Vec<_> = stock_data.iter().rev().take(3).collect();

    // Find common dates across all stocks
    let common_dates = find_common_dates(&stock_data);

    if common_dates.is_empty() {
        eprintln!("Error: No common trading dates found");
        return Ok(());
    }

    // Build price lookup
    let price_map = build_price_map(&stock_data);

    // Run backtest with rebalancing
    let mut rebalance_dates = Vec::new();

    for (i, date) in common_dates.iter().enumerate() {
        if i % REBALANCE_DAYS == 0 {
            rebalance_dates.push(date);
        }
    }

    // Calculate returns at each rebalance
    let mut returns = Vec::new();
    for window in rebalance_dates.windows(2) {
        let start_date = window[0];
        let end_date = window[1];

        let period_return = calculate_portfolio_return(
            &long_stocks,
            &short_stocks,
            start_date,
            end_date,
            &price_map,
        );

        returns.push(period_return);
    }

    // Calculate performance metrics
    let result = calculate_metrics(&returns);

    // =========================================================================
    // 3. Display results
    // =========================================================================

    println!("\nValue Strategy (Earnings + FCF Yield)");
    println!("═════════════════════════════════════");
    println!("Period:     {} to {}", START_DATE, END_DATE);
    println!("Universe:   {} stocks", UNIVERSE.len());
    println!();
    println!("Performance:");
    println!("  Total Return:    {:+.1}%", result.total_return * 100.0);
    println!("  Sharpe Ratio:    {:.2}", result.sharpe_ratio);
    println!("  Max Drawdown:    {:.1}%", result.max_drawdown * 100.0);
    println!("  Win Rate:        {:.0}%", result.win_rate * 100.0);
    println!();
    println!("Portfolio:");
    println!("  LONG (top 3 value):");
    for stock in &long_stocks {
        println!(
            "    {} (score: {:.2}%)",
            stock.symbol,
            stock.value_score * 100.0
        );
    }
    println!("  SHORT (bottom 3 expensive):");
    for stock in short_stocks.iter().rev() {
        println!(
            "    {} (score: {:.2}%)",
            stock.symbol,
            stock.value_score * 100.0
        );
    }
    println!();

    Ok(())
}

/// Find common trading dates across all stocks.
fn find_common_dates(stocks: &[StockData]) -> Vec<String> {
    if stocks.is_empty() {
        return Vec::new();
    }

    // Get dates from first stock
    let mut common: Vec<String> = stocks[0].prices.iter().map(|p| p.date.clone()).collect();

    // Intersect with all other stocks
    for stock in stocks.iter().skip(1) {
        let dates: std::collections::HashSet<String> =
            stock.prices.iter().map(|p| p.date.clone()).collect();
        common.retain(|d| dates.contains(d));
    }

    common.sort();
    common
}

/// Build a lookup map: symbol -> (date -> price).
fn build_price_map(stocks: &[StockData]) -> HashMap<String, HashMap<String, f64>> {
    let mut map = HashMap::new();
    for stock in stocks {
        let mut price_map = HashMap::new();
        for price in &stock.prices {
            price_map.insert(price.date.clone(), price.close);
        }
        map.insert(stock.symbol.clone(), price_map);
    }
    map
}

/// Calculate portfolio return for a period (long top 3, short bottom 3).
fn calculate_portfolio_return(
    long_stocks: &[&StockData],
    short_stocks: &[&StockData],
    start_date: &str,
    end_date: &str,
    price_map: &HashMap<String, HashMap<String, f64>>,
) -> f64 {
    let mut long_return = 0.0;
    let mut short_return = 0.0;

    // Long positions
    for stock in long_stocks {
        if let (Some(start_price), Some(end_price)) = (
            price_map.get(&stock.symbol).and_then(|m| m.get(start_date)),
            price_map.get(&stock.symbol).and_then(|m| m.get(end_date)),
        ) {
            long_return += (end_price - start_price) / start_price;
        }
    }
    long_return /= long_stocks.len() as f64;

    // Short positions
    for stock in short_stocks {
        if let (Some(start_price), Some(end_price)) = (
            price_map.get(&stock.symbol).and_then(|m| m.get(start_date)),
            price_map.get(&stock.symbol).and_then(|m| m.get(end_date)),
        ) {
            short_return += (start_price - end_price) / start_price; // Inverted for short
        }
    }
    short_return /= short_stocks.len() as f64;

    // Equal weight long/short
    (long_return + short_return) / 2.0
}

/// Calculate performance metrics from returns.
fn calculate_metrics(returns: &[f64]) -> BacktestResult {
    if returns.is_empty() {
        return BacktestResult {
            total_return: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            win_rate: 0.0,
        };
    }

    // Total return (compounded)
    let total_return: f64 = returns.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0;

    // Sharpe ratio (annualized)
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    let std_dev = variance.sqrt();
    let sharpe_ratio = if std_dev > 0.0 {
        (mean / std_dev) * (4.0_f64).sqrt() // Quarterly to annual
    } else {
        0.0
    };

    // Max drawdown
    let mut cumulative = Vec::with_capacity(returns.len());
    let mut cum = 1.0;
    for r in returns {
        cum *= 1.0 + r;
        cumulative.push(cum);
    }

    let mut max_drawdown = 0.0;
    let mut peak = cumulative[0];
    for &value in &cumulative {
        if value > peak {
            peak = value;
        }
        let drawdown = (peak - value) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    // Win rate
    let wins = returns.iter().filter(|&&r| r > 0.0).count();
    let win_rate = wins as f64 / returns.len() as f64;

    BacktestResult {
        total_return,
        sharpe_ratio,
        max_drawdown,
        win_rate,
    }
}
