//! Signal listing command implementation.

use anyhow::Result;
use factors::{FactorCategory, FactorRegistry};

/// List available signals, optionally filtered by category.
pub(crate) async fn list_signals(category: Option<String>, verbose: bool) -> Result<()> {
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
                    factor.name(), factor.description(), factor.lookback()
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
