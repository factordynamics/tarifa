//! Backtest command implementation.

use crate::cmd::eval::find_price_at_date;
use crate::{data, signals};
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use tarifa_traits::CE_TO_UNIX_EPOCH_DAYS;

/// Run a backtest on a signal over a given time period.
pub(crate) async fn run_backtest(
    signal: &str,
    start: &str,
    end: &str,
    universe: &str,
    format: &str,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                       Backtesting                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Signal:   {}", signal);
    println!("Period:   {} to {}", start, end);
    println!("Universe: {}", universe);
    println!("Format:   {}", format);
    println!();

    // Parse dates
    let start_date = data::parse_date(start).map_err(|e| anyhow::anyhow!("{}", e))?;
    let end_date = data::parse_date(end).map_err(|e| anyhow::anyhow!("{}", e))?;

    // Determine symbols from universe
    let symbols: Vec<String> = if universe.to_lowercase() == "sp500" {
        // Default SP500 subset for now
        vec![
            "AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA", "UNH", "JPM", "V",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect()
    } else {
        // Parse comma-separated symbols
        universe.split(',').map(|s| s.trim().to_string()).collect()
    };

    println!("Testing on {} symbols", symbols.len());
    println!();

    // Create the signal
    let signal_instance = match signals::create_factor(signal) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        }
    };

    // Load market data with sufficient history
    // Need data before start date for signal computation
    let total_lookback = signal_instance.lookback() + 252; // Extra year for stability
    println!(
        "Fetching market data (lookback: {} days)...",
        total_lookback
    );

    let market_data = match data::load_market_data(&symbols, total_lookback, Some(end_date)).await {
        Ok(md) => md,
        Err(e) => {
            println!("Error loading data: {}", e);
            return Ok(());
        }
    };

    println!(
        "Loaded {} rows of data for {} columns",
        market_data.len(),
        market_data.columns().len()
    );
    println!();

    // Get unique dates and filter to backtest period
    println!("Computing signal scores and returns...");
    let df = market_data.data();
    let date_col = df.column("date").map_err(|e| anyhow::anyhow!("{}", e))?;

    // Parse dates from the data
    let dates: Vec<NaiveDate> = if let Ok(date_series) = date_col.as_materialized_series().date() {
        date_series
            .into_iter()
            .filter_map(|d: Option<i32>| {
                d.map(|days| {
                    NaiveDate::from_num_days_from_ce_opt(days + CE_TO_UNIX_EPOCH_DAYS)
                        .unwrap_or_else(|| Utc::now().date_naive())
                })
            })
            .collect()
    } else if let Ok(datetime_series) = date_col.as_materialized_series().datetime() {
        datetime_series
            .into_iter()
            .filter_map(|d: Option<i64>| {
                d.map(|ts| {
                    chrono::DateTime::from_timestamp_millis(ts)
                        .map(|dt| dt.date_naive())
                        .unwrap_or_else(|| Utc::now().date_naive())
                })
            })
            .collect()
    } else {
        println!("Error: Could not parse date column");
        return Ok(());
    };

    let mut unique_dates: Vec<NaiveDate> = dates.clone();
    unique_dates.sort();
    unique_dates.dedup();

    // Filter dates to backtest period
    let backtest_dates: Vec<NaiveDate> = unique_dates
        .iter()
        .filter(|&&d| d >= start_date && d <= end_date)
        .skip(signal_instance.lookback()) // Need lookback data
        .copied()
        .collect();

    if backtest_dates.len() < 20 {
        println!(
            "Error: Not enough dates for backtest (need at least 20, got {})",
            backtest_dates.len()
        );
        return Ok(());
    }

    // Extract price data for return calculation
    let close_col = df.column("close").map_err(|e| anyhow::anyhow!("{}", e))?;
    let closes: Vec<f64> = close_col
        .as_materialized_series()
        .f64()
        .map_err(|e| anyhow::anyhow!("Close column error: {}", e))?
        .into_iter()
        .flatten()
        .collect();

    let symbol_col = df.column("symbol").map_err(|e| anyhow::anyhow!("{}", e))?;
    let symbols_vec: Vec<String> = symbol_col
        .as_materialized_series()
        .str()
        .map_err(|e| anyhow::anyhow!("Symbol column error: {}", e))?
        .into_iter()
        .filter_map(|s: Option<&str>| s.map(|x| x.to_string()))
        .collect();

    // Compute signal scores and returns for each date
    let mut signal_scores: Vec<Vec<f64>> = Vec::new();
    let mut returns: Vec<Vec<f64>> = Vec::new();

    for (i, &date) in backtest_dates.iter().enumerate() {
        // Compute signal score for this date
        if let Ok(scores_df) = signal_instance.score(&market_data, date) {
            let score_symbols: Vec<String> = scores_df
                .column("symbol")
                .ok()
                .and_then(|c| c.as_materialized_series().str().ok())
                .map(|s| {
                    s.into_iter()
                        .filter_map(|x| x.map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let scores: Vec<f64> = scores_df
                .column("score")
                .ok()
                .and_then(|c| c.as_materialized_series().f64().ok())
                .map(|s| s.into_iter().flatten().collect())
                .unwrap_or_default();

            if !scores.is_empty() && scores.len() == score_symbols.len() {
                signal_scores.push(scores.clone());

                // Compute 1-day forward returns
                let mut period_returns: Vec<f64> = Vec::with_capacity(scores.len());
                for sym in &score_symbols {
                    let current_price =
                        find_price_at_date(&symbols_vec, &dates, &closes, sym, &date);

                    // Get next date for returns
                    let next_date = if i + 1 < backtest_dates.len() {
                        backtest_dates[i + 1]
                    } else {
                        date + chrono::Duration::days(1)
                    };

                    let future_price =
                        find_price_at_date(&symbols_vec, &dates, &closes, sym, &next_date);

                    if let (Some(cur), Some(fut)) = (current_price, future_price) {
                        period_returns.push((fut - cur) / cur);
                    } else {
                        period_returns.push(0.0);
                    }
                }
                returns.push(period_returns);
            }
        }
    }

    if signal_scores.is_empty() {
        println!("Error: Could not compute signal scores for any dates");
        return Ok(());
    }

    println!("Computed {} periods of signal scores", signal_scores.len());
    println!();

    // Run backtest using tarifa_eval::Backtest
    let backtest_config = tarifa_eval::BacktestConfig {
        start_date,
        end_date,
        rebalance_frequency: 21, // Monthly rebalancing
        transaction_cost_bps: 10.0,
        initial_capital: 1_000_000.0,
        max_position_size: 0.1,
        min_position_size: 0.0,
        n_long: Some(5),
        n_short: Some(5),
        long_short: true,
    };

    let backtest = tarifa_eval::Backtest::new(backtest_config);
    let result = backtest.run(&signal_scores, &returns, &backtest_dates);

    // Display results
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("BACKTEST RESULTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    if format == "json" {
        // JSON output
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))?;
        println!("{}", json);
    } else {
        // Text output
        println!("Performance Metrics:");
        println!(
            "  Total Return:      {:>10.2}%",
            result.total_return * 100.0
        );
        println!(
            "  Annualized Return: {:>10.2}%",
            result.annualized_return * 100.0
        );
        println!(
            "  Annualized Vol:    {:>10.2}%",
            result.annualized_volatility * 100.0
        );
        println!("  Sharpe Ratio:      {:>10.2}", result.sharpe_ratio);
        println!(
            "  Max Drawdown:      {:>10.2}%",
            result.max_drawdown * 100.0
        );
        println!();

        println!("Trading Metrics:");
        println!(
            "  Avg Turnover:      {:>10.2}%",
            result.avg_turnover * 100.0
        );
        println!(
            "  Total Txn Costs:   {:>10.2}%",
            result.total_transaction_costs * 100.0
        );
        println!("  Number of Trades:  {:>10}", result.n_trades);
        println!();

        println!("Signal Quality:");
        if !result.ic_history.is_empty() {
            let avg_ic: f64 = result
                .ic_history
                .iter()
                .filter(|x| x.is_finite())
                .sum::<f64>()
                / result.ic_history.iter().filter(|x| x.is_finite()).count() as f64;
            let ic_std: f64 = {
                let valid_ics: Vec<f64> = result
                    .ic_history
                    .iter()
                    .copied()
                    .filter(|x| x.is_finite())
                    .collect();
                if valid_ics.len() > 1 {
                    let mean = valid_ics.iter().sum::<f64>() / valid_ics.len() as f64;
                    let variance = valid_ics.iter().map(|ic| (ic - mean).powi(2)).sum::<f64>()
                        / (valid_ics.len() - 1) as f64;
                    variance.sqrt()
                } else {
                    0.0
                }
            };
            let ic_ir = if ic_std > 0.0 { avg_ic / ic_std } else { 0.0 };
            println!("  Average IC:        {:>10.4}", avg_ic);
            println!("  IC Std Dev:        {:>10.4}", ic_std);
            println!("  IC Info Ratio:     {:>10.4}", ic_ir);
        } else {
            println!("  Average IC:        N/A");
        }
        println!();
    }

    Ok(())
}
