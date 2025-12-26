//! Score command implementation.

use crate::{data, signals};
use anyhow::Result;
use chrono::Utc;

/// Show signal scores for the given symbols.
pub(crate) async fn show_scores(
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
