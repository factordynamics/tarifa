//! Evaluation command implementation.

use crate::{data, signals};
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use tarifa_traits::CE_TO_UNIX_EPOCH_DAYS;
use tarifa_eval::{DefaultEvaluator, EvaluatorConfig};

/// Find price at a specific date for a symbol.
pub(crate) fn find_price_at_date(
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

/// Evaluate a signal's predictive power.
pub(crate) async fn evaluate_signal(
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
