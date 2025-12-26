//! Research command implementation.

use crate::cmd::eval::find_price_at_date;
use crate::{data, signals};
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use tarifa_traits::CE_TO_UNIX_EPOCH_DAYS;
use tarifa_eval::{DefaultEvaluator, EvaluatorConfig};

/// Research a signal's characteristics (IC analysis, decay curves, turnover).
pub(crate) async fn research_signal(
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
                        "|".repeat(bar_len.saturating_sub(bar_width / 2)),
                        width = bar_width / 2
                    )
                } else {
                    format!("{}{}", "|".repeat(bar_len), "")
                };
                println!("  Period {:>3}: {:>7.4} |{}|", start_idx + i + 1, ic, bar);
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
