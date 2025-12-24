//! Signal registry for discovering and categorizing available signals.
//!
//! This module provides metadata and discovery functionality for all signals
//! in the tarifa-signals library.

use serde::{Deserialize, Serialize};

/// Signal category classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalCategory {
    /// Price momentum signals
    Momentum,
    /// Valuation signals
    Value,
    /// Profitability and quality signals
    Quality,
    /// Growth signals
    Growth,
    /// Mean reversion signals
    Reversion,
    /// Technical indicators
    Technical,
    /// Fundamental signals
    Fundamental,
    /// Alternative data signals
    Alternative,
}

impl SignalCategory {
    /// Get a human-readable description of the category.
    #[must_use]
    pub const fn description(&self) -> &str {
        match self {
            Self::Momentum => "Price momentum and trend-following signals",
            Self::Value => "Valuation metrics comparing fundamentals to price",
            Self::Quality => "Profitability and operational efficiency metrics",
            Self::Growth => "Revenue and earnings growth signals",
            Self::Reversion => "Mean reversion and contrarian signals",
            Self::Technical => "Technical analysis indicators",
            Self::Fundamental => "Fundamental financial metrics",
            Self::Alternative => "Alternative data signals (sentiment, flow, etc.)",
        }
    }
}

/// Metadata about a signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalInfo {
    /// Unique identifier for the signal
    pub name: &'static str,

    /// Category classification
    pub category: SignalCategory,

    /// Human-readable description
    pub description: &'static str,

    /// Typical lookback period in days
    pub typical_lookback: usize,

    /// Whether the signal requires fundamental data
    pub requires_fundamentals: bool,
}

/// Get information about all available signals.
#[must_use]
pub fn available_signals() -> Vec<SignalInfo> {
    vec![
        // Momentum signals
        SignalInfo {
            name: "short_term_momentum",
            category: SignalCategory::Momentum,
            description: "1-month cumulative returns",
            typical_lookback: 21,
            requires_fundamentals: false,
        },
        SignalInfo {
            name: "medium_term_momentum",
            category: SignalCategory::Momentum,
            description: "6-month cumulative returns",
            typical_lookback: 126,
            requires_fundamentals: false,
        },
        SignalInfo {
            name: "long_term_momentum",
            category: SignalCategory::Momentum,
            description: "12-month cumulative returns (skipping last month)",
            typical_lookback: 252,
            requires_fundamentals: false,
        },
        // Value signals
        SignalInfo {
            name: "book_to_price",
            category: SignalCategory::Value,
            description: "Book value of equity relative to market cap",
            typical_lookback: 0,
            requires_fundamentals: true,
        },
        SignalInfo {
            name: "earnings_yield",
            category: SignalCategory::Value,
            description: "Earnings relative to market cap (inverse P/E)",
            typical_lookback: 0,
            requires_fundamentals: true,
        },
        SignalInfo {
            name: "fcf_yield",
            category: SignalCategory::Value,
            description: "Free cash flow relative to market cap",
            typical_lookback: 0,
            requires_fundamentals: true,
        },
        // Quality signals
        SignalInfo {
            name: "return_on_equity",
            category: SignalCategory::Quality,
            description: "Net income relative to shareholder equity",
            typical_lookback: 0,
            requires_fundamentals: true,
        },
        SignalInfo {
            name: "return_on_assets",
            category: SignalCategory::Quality,
            description: "Net income relative to total assets",
            typical_lookback: 0,
            requires_fundamentals: true,
        },
        SignalInfo {
            name: "profit_margins",
            category: SignalCategory::Quality,
            description: "Gross, operating, or net profit margins",
            typical_lookback: 0,
            requires_fundamentals: true,
        },
    ]
}

/// Get all signals in a specific category.
#[must_use]
pub fn signals_by_category(category: &SignalCategory) -> Vec<SignalInfo> {
    available_signals()
        .into_iter()
        .filter(|info| &info.category == category)
        .collect()
}

/// Get information about a specific signal by name.
#[must_use]
pub fn get_signal_info(name: &str) -> Option<SignalInfo> {
    available_signals()
        .into_iter()
        .find(|info| info.name == name)
}

/// Get all signal categories with signals.
#[must_use]
pub fn available_categories() -> Vec<SignalCategory> {
    let mut categories: Vec<_> = available_signals()
        .into_iter()
        .map(|info| info.category)
        .collect();
    categories.sort_by_key(|c| format!("{c:?}"));
    categories.dedup();
    categories
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_signals() {
        let signals = available_signals();
        assert!(!signals.is_empty());

        // Check that we have signals from each implemented category
        let categories: Vec<_> = signals.iter().map(|s| s.category).collect();
        assert!(categories.contains(&SignalCategory::Momentum));
        assert!(categories.contains(&SignalCategory::Value));
        assert!(categories.contains(&SignalCategory::Quality));
    }

    #[test]
    fn test_signals_by_category() {
        let momentum_signals = signals_by_category(&SignalCategory::Momentum);
        assert_eq!(momentum_signals.len(), 3);

        let value_signals = signals_by_category(&SignalCategory::Value);
        assert_eq!(value_signals.len(), 3);

        let quality_signals = signals_by_category(&SignalCategory::Quality);
        assert_eq!(quality_signals.len(), 3);
    }

    #[test]
    fn test_get_signal_info() {
        let info = get_signal_info("short_term_momentum");
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.name, "short_term_momentum");
        assert_eq!(info.category, SignalCategory::Momentum);
        assert_eq!(info.typical_lookback, 21);

        let missing = get_signal_info("nonexistent_signal");
        assert!(missing.is_none());
    }

    #[test]
    fn test_category_descriptions() {
        assert!(!SignalCategory::Momentum.description().is_empty());
        assert!(!SignalCategory::Value.description().is_empty());
        assert!(!SignalCategory::Quality.description().is_empty());
    }

    #[test]
    fn test_available_categories() {
        let categories = available_categories();
        assert!(!categories.is_empty());
        assert!(categories.contains(&SignalCategory::Momentum));
        assert!(categories.contains(&SignalCategory::Value));
        assert!(categories.contains(&SignalCategory::Quality));

        // Should not contain duplicates
        let unique_count = categories.len();
        let all_count = available_signals()
            .into_iter()
            .map(|s| s.category)
            .collect::<Vec<_>>()
            .len();
        assert!(unique_count <= all_count);
    }

    #[test]
    fn test_fundamentals_flag() {
        let momentum = get_signal_info("short_term_momentum").unwrap();
        assert!(!momentum.requires_fundamentals);

        let value = get_signal_info("book_to_price").unwrap();
        assert!(value.requires_fundamentals);

        let quality = get_signal_info("return_on_equity").unwrap();
        assert!(quality.requires_fundamentals);
    }
}
