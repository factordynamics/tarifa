//! Tarifa CLI binary.
//!
//! Provides command-line interface for the Tarifa alpha model.

mod data;
mod signals;

use anyhow::Result;
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use factors::{FactorCategory, FactorRegistry};
use std::process;
use tarifa_eval::{DefaultEvaluator, EvaluatorConfig};

#[derive(Parser)]
#[command(name = "tarifa")]
#[command(about = "Alpha model for equity return prediction", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available signals
    Signals {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Evaluate a signal
    Eval {
        /// Signal name
        signal: String,

        /// Ticker symbol(s)
        #[arg(short, long, value_delimiter = ',')]
        symbols: Vec<String>,

        /// Evaluation horizon in days
        #[arg(short = 'H', long, default_value = "21")]
        horizon: usize,

        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: Option<String>,
    },

    /// Run backtest
    Backtest {
        /// Signal or combiner to test
        signal: String,

        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,

        /// Universe to test on
        #[arg(short, long, default_value = "sp500")]
        universe: String,

        /// Output format (text or json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Show signal scores for symbols
    Score {
        /// Signal name
        signal: String,

        /// Ticker symbols
        #[arg(value_delimiter = ',')]
        symbols: Vec<String>,

        /// Date to compute scores (YYYY-MM-DD, defaults to latest)
        #[arg(short, long)]
        date: Option<String>,

        /// Show raw scores instead of standardized
        #[arg(long)]
        raw: bool,
    },

    /// Combine multiple signals
    Combine {
        /// Signal names to combine
        #[arg(short, long, value_delimiter = ',')]
        signals: Vec<String>,

        /// Combination method (equal, ic-weight, ml)
        #[arg(short, long, default_value = "equal")]
        method: String,

        /// Ticker symbols to score
        symbols: Vec<String>,

        /// Date to compute combined score (YYYY-MM-DD, defaults to latest)
        #[arg(long)]
        date: Option<String>,
    },

    /// Research signals (IC analysis, decay curves, etc.)
    Research {
        /// Signal name to research
        signal: String,

        /// Analysis type (ic, decay, turnover, all)
        #[arg(short, long, default_value = "all")]
        analysis: String,

        /// Evaluation horizon in days
        #[arg(short = 'H', long, default_value = "21")]
        horizon: usize,

        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: Option<String>,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Signals { category, verbose } => {
            list_signals(category, verbose).await?;
        }
        Commands::Eval {
            signal,
            symbols,
            horizon,
            start,
            end,
        } => {
            evaluate_signal(&signal, &symbols, horizon, start, end).await?;
        }
        Commands::Backtest {
            signal,
            start,
            end,
            universe,
            format,
        } => {
            run_backtest(&signal, &start, &end, &universe, &format).await?;
        }
        Commands::Score {
            signal,
            symbols,
            date,
            raw,
        } => {
            show_scores(&signal, &symbols, date, raw).await?;
        }
        Commands::Combine {
            signals,
            method,
            symbols,
            date,
        } => {
            combine_signals(&signals, &method, &symbols, date).await?;
        }
        Commands::Research {
            signal,
            analysis,
            horizon,
            start,
            end,
        } => {
            research_signal(&signal, &analysis, horizon, start, end).await?;
        }
    }

    Ok(())
}

async fn list_signals(category: Option<String>, verbose: bool) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Available Factors                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let registry = FactorRegistry::with_defaults();

    // Group by category
    let categories = [
        (FactorCategory::Momentum, "Momentum"),
        (FactorCategory::Value, "Value"),
        (FactorCategory::Quality, "Quality"),
        (FactorCategory::Growth, "Growth"),
        (FactorCategory::Size, "Size"),
        (FactorCategory::Volatility, "Volatility"),
        (FactorCategory::Liquidity, "Liquidity"),
        (FactorCategory::Sentiment, "Sentiment"),
    ];

    for (cat, cat_name) in categories {
        if let Some(ref filter) = category
            && !cat_name.to_lowercase().contains(&filter.to_lowercase())
        {
            continue;
        }

        let cat_factors = registry.by_category(cat);
        if cat_factors.is_empty() {
            continue;
        }

        println!("{}:", cat_name);
        println!("{}", "-".repeat(60));

        for factor in cat_factors {
            if verbose {
                println!(
                    "  {:25} - {} (lookback: {} days)",
                    factor.name(),
                    factor.description(),
                    factor.lookback()
                );
            } else {
                println!("  {}", factor.name());
            }
        }
        println!();
    }

    if !verbose {
        println!("Use --verbose for detailed factor descriptions.\n");
    }

    // Show aliases
    println!("Factor aliases:");
    println!("  momentum_1m, mom_1m    -> short_term_momentum");
    println!("  momentum_6m, mom_6m    -> medium_term_momentum");
    println!("  momentum_12m, mom_12m  -> long_term_momentum");
    println!();

    Ok(())
}

async fn evaluate_signal(
    signal_name: &str,
    symbols: &[String],
    horizon: usize,
    start: Option<String>,
    end: Option<String>,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Signal Evaluation                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Create the signal
    let signal = match signals::create_factor(signal_name) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        }
    };

    println!("Signal:   {} ({})", signal.name(), signal_name);
    println!("Symbols:  {}", symbols.join(", "));
    println!("Horizon:  {} days", horizon);

    // Parse dates
    let end_date: Option<NaiveDate> = match end {
        Some(ref e) => {
            println!("End:      {}", e);
            Some(data::parse_date(e).map_err(|e| anyhow::anyhow!("{}", e))?)
        }
        None => {
            println!("End:      Latest available");
            None
        }
    };

    if let Some(ref s) = start {
        println!("Start:    {}", s);
    }
    println!();

    // Need more data for evaluation - at least 252 trading days of history
    let eval_lookback = signal.lookback() + 252 + horizon;
    println!(
        "Fetching market data (need {} days for evaluation)...",
        eval_lookback
    );

    let market_data = match data::load_market_data(symbols, eval_lookback, end_date).await {
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

    // Build evaluation data by computing signal scores and forward returns
    println!("Computing signal scores and forward returns...");

    // Get unique dates from data
    let df = market_data.data();
    let date_col = df.column("date").map_err(|e| anyhow::anyhow!("{}", e))?;

    // Try to parse dates - the column might be Date or Datetime type
    let dates: Vec<NaiveDate> = if let Ok(date_series) = date_col.as_materialized_series().date() {
        date_series
            .into_iter()
            .filter_map(|d: Option<i32>| {
                d.map(|days| {
                    // Date is stored as days since epoch
                    NaiveDate::from_num_days_from_ce_opt(days + 719163) // 719163 = days from CE to Unix epoch
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

    // Skip first lookback days and last horizon days
    let eval_dates: Vec<NaiveDate> = unique_dates
        .iter()
        .skip(signal.lookback())
        .take(
            unique_dates
                .len()
                .saturating_sub(signal.lookback() + horizon),
        )
        .copied()
        .collect();

    if eval_dates.len() < 20 {
        println!(
            "Error: Not enough dates for evaluation (need at least 20, got {})",
            eval_dates.len()
        );
        return Ok(());
    }

    // Compute signal scores for each date
    let mut signal_scores: Vec<Vec<f64>> = Vec::new();
    let mut forward_returns: Vec<Vec<f64>> = Vec::new();

    // Get close prices for forward return calculation
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

    // Sample evaluation dates (every 5 days for performance)
    let sample_step = 5;
    let sampled_dates: Vec<NaiveDate> = eval_dates.iter().step_by(sample_step).copied().collect();

    println!(
        "Evaluating {} dates (sampled from {})...",
        sampled_dates.len(),
        eval_dates.len()
    );

    for date in &sampled_dates {
        // Compute signal score
        if let Ok(scores_df) = signal.score(&market_data, *date) {
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

                // Compute forward returns for this date
                // For simplicity, we'll compute average forward return per symbol
                let mut fwd_returns: Vec<f64> = Vec::with_capacity(scores.len());
                for sym in &score_symbols {
                    // Find the price at date and date + horizon
                    let current_price =
                        find_price_at_date(&symbols_vec, &dates, &closes, sym, date);
                    let future_date = *date + chrono::Duration::days(horizon as i64);
                    let future_price =
                        find_price_at_date(&symbols_vec, &dates, &closes, sym, &future_date);

                    if let (Some(cur), Some(fut)) = (current_price, future_price) {
                        fwd_returns.push((fut - cur) / cur);
                    } else {
                        fwd_returns.push(0.0);
                    }
                }
                forward_returns.push(fwd_returns);
            }
        }
    }

    if signal_scores.len() < 10 {
        println!(
            "Error: Not enough signal scores computed (got {})",
            signal_scores.len()
        );
        return Ok(());
    }

    println!("Computed {} periods of signal scores", signal_scores.len());
    println!();

    // Create evaluator and compute metrics
    let config = EvaluatorConfig {
        min_observations: 5,
        annualize: true,
        ..Default::default()
    };

    let evaluator = DefaultEvaluator::new(signal_scores, forward_returns, config);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("EVALUATION METRICS (horizon = {} days)", horizon);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let metrics = evaluator.metrics(1);

    println!("Information Coefficient (IC):");
    println!("  Mean IC:     {:>8.4}", metrics.mean_ic);
    println!("  IC Std Dev:  {:>8.4}", metrics.ic_std);
    println!("  IC Hit Rate: {:>8.2}%", metrics.ic_hit_rate * 100.0);
    println!();

    println!("Information Ratio (IR):");
    println!("  IR:          {:>8.4}", metrics.ir.ir);
    println!();

    println!("Signal Turnover:");
    println!("  Autocorr:    {:>8.4}", metrics.turnover.autocorr);
    println!("  Turnover:    {:>8.4}", metrics.turnover.turnover_rate);
    println!();

    // Decay analysis
    let decay = evaluator.decay_analysis();
    println!("IC Decay (by horizon):");
    for (h, ic) in decay
        .curve
        .horizons
        .iter()
        .zip(decay.curve.ic_values.iter())
    {
        if ic.is_finite() {
            println!("  {:>3} days:   {:>8.4}", h, ic);
        }
    }
    if let Some(half_life) = decay.half_life {
        println!();
        println!("  Half-life:  {:>8.1} days", half_life);
    }
    println!();

    Ok(())
}

/// Find price at a specific date for a symbol
fn find_price_at_date(
    symbols: &[String],
    dates: &[NaiveDate],
    closes: &[f64],
    symbol: &str,
    target_date: &NaiveDate,
) -> Option<f64> {
    for i in 0..symbols.len().min(dates.len()).min(closes.len()) {
        if symbols[i] == symbol && dates[i] == *target_date {
            return Some(closes[i]);
        }
    }
    // Try to find closest date within 5 days
    for days_offset in 1..=5 {
        for offset_dir in &[-1i64, 1i64] {
            let check_date = *target_date + chrono::Duration::days(days_offset * offset_dir);
            for i in 0..symbols.len().min(dates.len()).min(closes.len()) {
                if symbols[i] == symbol && dates[i] == check_date {
                    return Some(closes[i]);
                }
            }
        }
    }
    None
}

async fn run_backtest(
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
                    NaiveDate::from_num_days_from_ce_opt(days + 719163)
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

async fn show_scores(
    signal_name: &str,
    symbols: &[String],
    date: Option<String>,
    _raw: bool,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                      Signal Scores                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Create the signal
    let signal = match signals::create_factor(signal_name) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        }
    };

    println!("Signal: {} ({})", signal.name(), signal_name);

    // Parse date
    let end_date = match date {
        Some(ref d) => {
            println!("Date:   {}", d);
            Some(data::parse_date(d).map_err(|e| anyhow::anyhow!("{}", e))?)
        }
        None => {
            println!("Date:   Latest available");
            None
        }
    };
    println!();

    println!("Fetching market data for {} symbol(s)...", symbols.len());

    // Get lookback period
    let lookback = signals::get_lookback(signal_name);

    // Load market data
    let market_data = match data::load_market_data(symbols, lookback, end_date).await {
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

    // Compute signal scores
    let score_date = end_date.unwrap_or_else(|| Utc::now().date_naive());

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("SCORES (as of {})", score_date);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    match signal.score(&market_data, score_date) {
        Ok(scores_df) => {
            println!("{:<10} {:>12}", "Symbol", "Score");
            println!("{}", "─".repeat(24));

            // Extract scores from DataFrame
            let symbol_col = scores_df
                .column("symbol")
                .map_err(|e| anyhow::anyhow!("Missing symbol column: {}", e))?;
            let score_col = scores_df
                .column("score")
                .map_err(|e| anyhow::anyhow!("Missing score column: {}", e))?;

            let symbols_vec: Vec<&str> = symbol_col
                .as_materialized_series()
                .str()
                .map_err(|e| anyhow::anyhow!("Symbol column error: {}", e))?
                .into_iter()
                .flatten()
                .collect();

            let scores_vec: Vec<f64> = score_col
                .as_materialized_series()
                .f64()
                .map_err(|e| anyhow::anyhow!("Score column error: {}", e))?
                .into_iter()
                .flatten()
                .collect();

            for (sym, score) in symbols_vec.iter().zip(scores_vec.iter()) {
                println!("{:<10} {:>12.4}", sym, score);
            }
            println!();

            println!("Scores are cross-sectionally standardized (z-scores, mean=0, std=1)");
        }
        Err(e) => {
            println!("Error computing scores: {}", e);
            println!();
            println!("This might be due to insufficient data for the lookback period.");
            println!(
                "The {} signal requires {} trading days of data.",
                signal.name(),
                signal.lookback()
            );
        }
    }

    println!();
    Ok(())
}

async fn combine_signals(
    signals: &[String],
    method: &str,
    symbols: &[String],
    date: Option<String>,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Signal Combination                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Signals: {}", signals.join(", "));
    println!("Method:  {}", method);
    if let Some(ref d) = date {
        println!("Date:    {}", d);
    } else {
        println!("Date:    Latest available");
    }
    println!();

    // Validate we have at least one signal
    if signals.is_empty() {
        println!("Error: No signals specified. Use --signals to provide signal names.");
        return Ok(());
    }

    // Parse date
    let end_date = match date {
        Some(ref d) => Some(data::parse_date(d).map_err(|e| anyhow::anyhow!("{}", e))?),
        None => None,
    };

    // Create all signals
    let mut signal_objects = Vec::new();
    let mut max_lookback = 0;

    for signal_name in signals {
        match self::signals::create_factor(signal_name) {
            Ok(signal) => {
                max_lookback = max_lookback.max(signal.lookback());
                signal_objects.push(signal);
            }
            Err(e) => {
                println!("Error creating signal '{}': {}", signal_name, e);
                return Ok(());
            }
        }
    }

    println!("Fetching market data (need {} days)...", max_lookback);

    // Load market data with max lookback
    let market_data = match data::load_market_data(symbols, max_lookback, end_date).await {
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

    // Compute signal scores
    let score_date = end_date.unwrap_or_else(|| Utc::now().date_naive());

    println!("Computing scores for {} signals...", signal_objects.len());

    // Build SignalScore structs for the combiner
    use ndarray::Array1;
    use tarifa_combine::{Combiner, EqualWeightCombiner, ICWeightedCombiner, SignalScore};

    let mut signal_scores: Vec<SignalScore> = Vec::new();

    for (i, signal) in signal_objects.iter().enumerate() {
        match signal.score(&market_data, score_date) {
            Ok(scores_df) => {
                // Extract scores from DataFrame
                let score_col = match scores_df.column("score") {
                    Ok(col) => col,
                    Err(e) => {
                        println!("Error: Signal '{}' missing score column: {}", signals[i], e);
                        return Ok(());
                    }
                };

                let scores_vec: Vec<f64> = match score_col.as_materialized_series().f64() {
                    Ok(series) => series.into_iter().flatten().collect(),
                    Err(e) => {
                        println!("Error: Signal '{}' score column error: {}", signals[i], e);
                        return Ok(());
                    }
                };

                if scores_vec.is_empty() {
                    println!("Warning: Signal '{}' produced no scores", signals[i]);
                    continue;
                }

                signal_scores.push(SignalScore {
                    name: signals[i].clone(),
                    scores: Array1::from_vec(scores_vec),
                });
            }
            Err(e) => {
                println!("Error computing scores for signal '{}': {}", signals[i], e);
                return Ok(());
            }
        }
    }

    if signal_scores.is_empty() {
        println!("Error: No signal scores could be computed");
        return Ok(());
    }

    // Create the appropriate combiner based on method
    let method_lower = method.to_lowercase();
    let combined_scores = if method_lower == "equal" || method_lower == "equal_weight" {
        let combiner = EqualWeightCombiner::default();
        match combiner.combine(&signal_scores) {
            Ok(scores) => scores,
            Err(e) => {
                println!("Error combining signals: {}", e);
                return Ok(());
            }
        }
    } else if method_lower == "ic" || method_lower == "ic_weight" {
        let combiner = ICWeightedCombiner::default();
        match combiner.combine(&signal_scores) {
            Ok(scores) => scores,
            Err(e) => {
                println!("Error combining signals: {}", e);
                return Ok(());
            }
        }
    } else {
        println!(
            "Error: Unknown combination method '{}'. Use 'equal' or 'ic'.",
            method
        );
        return Ok(());
    };

    // Get the first signal's scores to extract symbols in the right order
    let first_scores_df = signal_objects[0]
        .score(&market_data, score_date)
        .map_err(|e| anyhow::anyhow!("Error getting symbols: {}", e))?;

    let symbol_col = first_scores_df
        .column("symbol")
        .map_err(|e| anyhow::anyhow!("Missing symbol column: {}", e))?;

    let symbols_vec: Vec<&str> = symbol_col
        .as_materialized_series()
        .str()
        .map_err(|e| anyhow::anyhow!("Symbol column error: {}", e))?
        .into_iter()
        .flatten()
        .collect();

    // Display combined scores
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("COMBINED SCORES (as of {})", score_date);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("{:<10} {:>12}", "Symbol", "Combined Score");
    println!("{}", "─".repeat(24));

    for (sym, score) in symbols_vec.iter().zip(combined_scores.iter()) {
        println!("{:<10} {:>12.4}", sym, score);
    }

    println!();
    println!("Scores are cross-sectionally standardized (z-scores, mean=0, std=1)");
    println!();

    Ok(())
}

async fn research_signal(
    signal_name: &str,
    analysis: &str,
    horizon: usize,
    start: Option<String>,
    end: Option<String>,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Signal Research                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Parse analysis type
    let analysis_lower = analysis.to_lowercase();
    let show_ic = analysis_lower == "ic" || analysis_lower == "all";
    let show_decay = analysis_lower == "decay" || analysis_lower == "all";
    let show_turnover = analysis_lower == "turnover" || analysis_lower == "all";

    if !show_ic && !show_decay && !show_turnover {
        return Err(anyhow::anyhow!(
            "Unknown analysis type: {}. Use: ic, decay, turnover, or all",
            analysis
        ));
    }

    // Create the signal
    let signal = match signals::create_factor(signal_name) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        }
    };

    // Default universe for research
    let default_universe = vec![
        "AAPL".to_string(),
        "MSFT".to_string(),
        "GOOGL".to_string(),
        "AMZN".to_string(),
        "META".to_string(),
        "NVDA".to_string(),
        "TSLA".to_string(),
        "UNH".to_string(),
        "JPM".to_string(),
        "V".to_string(),
    ];

    println!("Signal:   {} ({})", signal.name(), signal_name);
    println!("Universe: {}", default_universe.join(", "));
    println!("Analysis: {}", analysis);
    println!("Horizon:  {} days", horizon);

    // Parse dates
    let end_date: Option<NaiveDate> = match end {
        Some(ref e) => {
            println!("End:      {}", e);
            Some(data::parse_date(e).map_err(|e| anyhow::anyhow!("{}", e))?)
        }
        None => {
            println!("End:      Latest available");
            None
        }
    };

    if let Some(ref s) = start {
        println!("Start:    {}", s);
    }
    println!();

    // Need more data for research - at least 252 trading days of history
    let eval_lookback = signal.lookback() + 252 + horizon.max(63); // Use max horizon for decay
    println!(
        "Fetching market data (need {} days for research)...",
        eval_lookback
    );

    let market_data = match data::load_market_data(&default_universe, eval_lookback, end_date).await
    {
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

    // Build evaluation data by computing signal scores and forward returns
    println!("Computing signal scores and forward returns...");

    // Get unique dates from data
    let df = market_data.data();
    let date_col = df.column("date").map_err(|e| anyhow::anyhow!("{}", e))?;

    // Try to parse dates - the column might be Date or Datetime type
    let dates: Vec<NaiveDate> = if let Ok(date_series) = date_col.as_materialized_series().date() {
        date_series
            .into_iter()
            .filter_map(|d: Option<i32>| {
                d.map(|days| {
                    // Date is stored as days since epoch
                    NaiveDate::from_num_days_from_ce_opt(days + 719163) // 719163 = days from CE to Unix epoch
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

    // Skip first lookback days and last horizon days (use max horizon for decay analysis)
    let max_horizon = if show_decay { 63 } else { horizon };
    let eval_dates: Vec<NaiveDate> = unique_dates
        .iter()
        .skip(signal.lookback())
        .take(
            unique_dates
                .len()
                .saturating_sub(signal.lookback() + max_horizon),
        )
        .copied()
        .collect();

    // Filter by start date if provided
    let eval_dates: Vec<NaiveDate> = if let Some(ref s) = start {
        let start_date = data::parse_date(s).map_err(|e| anyhow::anyhow!("{}", e))?;
        eval_dates
            .into_iter()
            .filter(|d| d >= &start_date)
            .collect()
    } else {
        eval_dates
    };

    if eval_dates.len() < 20 {
        println!(
            "Error: Not enough dates for research (need at least 20, got {})",
            eval_dates.len()
        );
        return Ok(());
    }

    // Compute signal scores for each date
    let mut signal_scores: Vec<Vec<f64>> = Vec::new();
    let mut forward_returns: Vec<Vec<f64>> = Vec::new();

    // Get close prices for forward return calculation
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

    // Sample evaluation dates (every 3 days for more granular research)
    let sample_step = 3;
    let sampled_dates: Vec<NaiveDate> = eval_dates.iter().step_by(sample_step).copied().collect();

    println!(
        "Evaluating {} dates (sampled from {})...",
        sampled_dates.len(),
        eval_dates.len()
    );

    for date in &sampled_dates {
        // Compute signal score
        if let Ok(scores_df) = signal.score(&market_data, *date) {
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

                // Compute forward returns for this date at the specified horizon
                let mut fwd_returns: Vec<f64> = Vec::with_capacity(scores.len());
                for sym in &score_symbols {
                    let current_price =
                        find_price_at_date(&symbols_vec, &dates, &closes, sym, date);
                    let future_date = *date + chrono::Duration::days(horizon as i64);
                    let future_price =
                        find_price_at_date(&symbols_vec, &dates, &closes, sym, &future_date);

                    if let (Some(cur), Some(fut)) = (current_price, future_price) {
                        fwd_returns.push((fut - cur) / cur);
                    } else {
                        fwd_returns.push(0.0);
                    }
                }
                forward_returns.push(fwd_returns);
            }
        }
    }

    if signal_scores.len() < 10 {
        println!(
            "Error: Not enough signal scores computed (got {})",
            signal_scores.len()
        );
        return Ok(());
    }

    println!("Computed {} periods of signal scores", signal_scores.len());
    println!();

    // Create evaluator and compute metrics
    let config = EvaluatorConfig {
        min_observations: 5,
        annualize: true,
        ..Default::default()
    };

    let evaluator = DefaultEvaluator::new(signal_scores.clone(), forward_returns.clone(), config);

    // Display IC Analysis
    if show_ic {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("INFORMATION COEFFICIENT (IC) ANALYSIS");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let metrics = evaluator.metrics(1);

        println!("Time-series IC:");
        println!("  Mean IC:            {:>8.4}", metrics.mean_ic);
        println!("  Std Dev:            {:>8.4}", metrics.ic_std);
        println!("  Information Ratio:  {:>8.4}", metrics.ir.ir);
        println!(
            "  Hit Rate:           {:>8.2}%",
            metrics.ic_hit_rate * 100.0
        );
        println!();

        // Display rolling IC chart (text-based)
        println!("Rolling IC (last 20 periods):");
        let rolling = evaluator.ic_time_series(1);
        let display_count = rolling.len().min(20);
        let start_idx = rolling.len().saturating_sub(display_count);

        for (i, ic) in rolling[start_idx..].iter().enumerate() {
            if ic.is_finite() {
                let bar_width = 30;
                let normalized = (ic * 10.0).clamp(-3.0, 3.0); // Scale for display
                let bar_len = ((normalized + 3.0) / 6.0 * bar_width as f64) as usize;
                let bar: String = if normalized >= 0.0 {
                    format!(
                        "{:>width$}{}",
                        "",
                        "█".repeat(bar_len.saturating_sub(bar_width / 2)),
                        width = bar_width / 2
                    )
                } else {
                    format!("{}{}", "█".repeat(bar_len), "")
                };
                println!("  Period {:>3}: {:>7.4} │{}│", start_idx + i + 1, ic, bar);
            }
        }
        println!();
    }

    // Display Decay Analysis
    if show_decay {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("DECAY ANALYSIS");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let decay = evaluator.decay_analysis();

        println!("IC by horizon:");
        for (h, ic) in decay
            .curve
            .horizons
            .iter()
            .zip(decay.curve.ic_values.iter())
        {
            if ic.is_finite() {
                println!("  {:>3} days: {:>8.4}", h, ic);
            }
        }
        println!();

        if let Some(half_life) = decay.half_life {
            println!("Signal Half-life: {:>7.1} days", half_life);
            println!("(Time for predictive power to decay by 50%)");
        } else {
            println!("Signal Half-life: N/A (insufficient decay or no decay detected)");
        }
        println!();
    }

    // Display Turnover Analysis
    if show_turnover {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("TURNOVER ANALYSIS");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let metrics = evaluator.metrics(1);

        println!("Signal Turnover:");
        println!("  Autocorrelation:    {:>8.4}", metrics.turnover.autocorr);
        println!(
            "  Turnover Rate:      {:>8.4}",
            metrics.turnover.turnover_rate
        );
        println!();

        println!("Interpretation:");
        if metrics.turnover.autocorr > 0.9 {
            println!("  - Very stable signal (low turnover)");
        } else if metrics.turnover.autocorr > 0.7 {
            println!("  - Moderately stable signal");
        } else {
            println!("  - High turnover signal (rapidly changing)");
        }

        if metrics.turnover.turnover_rate < 0.3 {
            println!("  - Low trading costs expected");
        } else if metrics.turnover.turnover_rate < 0.6 {
            println!("  - Moderate trading costs");
        } else {
            println!("  - High trading costs may impact performance");
        }
        println!();
    }

    Ok(())
}
