//! Combine command implementation.

use crate::{data, signals};
use anyhow::Result;
use chrono::Utc;
use ndarray::Array1;
use tarifa_combine::{Combiner, EqualWeightCombiner, ICWeightedCombiner, SignalScore};

/// Combine multiple signals into a composite score.
pub(crate) async fn combine_signals(
    signal_names: &[String],
    method: &str,
    symbols: &[String],
    date: Option<String>,
) -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Signal Combination                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Signals: {}", signal_names.join(", "));
    println!("Method:  {}", method);
    if let Some(ref d) = date {
        println!("Date:    {}", d);
    } else {
        println!("Date:    Latest available");
    }
    println!();

    // Validate we have at least one signal
    if signal_names.is_empty() {
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

    for signal_name in signal_names {
        match signals::create_factor(signal_name) {
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
    let mut signal_scores: Vec<SignalScore> = Vec::new();

    for (i, signal) in signal_objects.iter().enumerate() {
        match signal.score(&market_data, score_date) {
            Ok(scores_df) => {
                // Extract scores from DataFrame
                let score_col = match scores_df.column("score") {
                    Ok(col) => col,
                    Err(e) => {
                        println!(
                            "Error: Signal '{}' missing score column: {}",
                            signal_names[i], e
                        );
                        return Ok(());
                    }
                };

                let scores_vec: Vec<f64> = match score_col.as_materialized_series().f64() {
                    Ok(series) => series.into_iter().flatten().collect(),
                    Err(e) => {
                        println!(
                            "Error: Signal '{}' score column error: {}",
                            signal_names[i], e
                        );
                        return Ok(());
                    }
                };

                if scores_vec.is_empty() {
                    println!("Warning: Signal '{}' produced no scores", signal_names[i]);
                    continue;
                }

                signal_scores.push(SignalScore {
                    name: signal_names[i].clone(),
                    scores: Array1::from_vec(scores_vec),
                });
            }
            Err(e) => {
                println!(
                    "Error computing scores for signal '{}': {}",
                    signal_names[i], e
                );
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
    println!("{}", "-".repeat(24));

    for (sym, score) in symbols_vec.iter().zip(combined_scores.iter()) {
        println!("{:<10} {:>12.4}", sym, score);
    }

    println!();
    println!("Scores are cross-sectionally standardized (z-scores, mean=0, std=1)");
    println!();

    Ok(())
}
