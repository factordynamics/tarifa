#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/tarifa/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! # tarifa
//!
//! Alpha model for equity return prediction.
//!
//! tarifa is an umbrella crate that re-exports all tarifa sub-crates for convenience.
//! It provides a unified API for working with alpha signals, signal evaluation, and
//! signal combination strategies.
//!
//! ## Quick Start
//!
//! ```ignore
//! use tarifa::{Signal, AlphaModel, Result};
//! use tarifa::signals::Momentum;
//! use tarifa::combine::EqualWeightCombiner;
//! use tarifa::types::{MarketData, Date};
//!
//! # fn main() -> Result<()> {
//! // Load market data
//! let market_data = MarketData::load("data/prices.parquet")?;
//! let universe = market_data.symbols();
//!
//! // Create signals
//! let signals: Vec<Box<dyn Signal>> = vec![
//!     Box::new(Momentum::new(20)),
//!     Box::new(Momentum::new(60)),
//! ];
//!
//! // Combine signals into alpha model
//! let combiner = EqualWeightCombiner::new();
//! let alpha_model = AlphaModel::new(signals, combiner);
//!
//! // Generate expected returns
//! let expected_returns = alpha_model.expected_returns(&universe, Date::today())?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Crate Organization
//!
//! - [`traits`] - Core trait definitions ([`Signal`], [`AlphaModel`], etc.)
//! - [`signals`] - Signal implementations (momentum, value, quality, etc.)
//! - [`combine`] - Signal combination strategies
//! - [`eval`] - Signal evaluation and backtesting tools
//!
//! ## Architecture
//!
//! tarifa follows a modular architecture:
//!
//! 1. **Signals** compute scores for assets at a point in time
//! 2. **Evaluators** measure signal quality (IC, IR, turnover)
//! 3. **Combiners** blend multiple signals into composite alpha
//! 4. **Alpha Models** output expected returns for portfolio optimization
//!
//! ## Integration
//!
//! tarifa integrates with the Factor Dynamics ecosystem:
//!
//! - **perth**: Risk model providing covariance estimates
//! - **cadiz**: Portfolio optimizer (future integration)
//! - **Data layer**: Shared market data infrastructure

/// Version information for the tarifa crate.
///
/// This constant contains the current version of tarifa as specified in Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// Core Traits
// ============================================================================

/// Core trait definitions for tarifa.
///
/// This module re-exports the foundational traits that define the tarifa API:
///
/// - [`Signal`] - Single alpha signal that scores assets
/// - [`AlphaModel`] - Combined model that produces expected returns
/// - [`SignalEvaluator`] - Evaluates signal quality metrics
/// - [`Combiner`] - Combines multiple signals into composite alpha
///
/// # Example
///
/// ```ignore
/// use tarifa::{Signal, AlphaModel};
/// ```
pub mod traits {
    pub use tarifa_traits::*;
}

// Re-export core traits at top level for convenience
pub use tarifa_combine::Combiner;
pub use tarifa_traits::{AlphaModel, Signal, SignalEvaluator};

// Re-export error types
pub use tarifa_traits::{Result, TarifaError};

// Re-export common types
pub use tarifa_combine::SignalScore;
pub use tarifa_traits::types::{Date, MarketData, Symbol};

// ============================================================================
// Signal Implementations
// ============================================================================

/// Signal implementations.
///
/// This module contains concrete implementations of the [`Signal`] trait,
/// organized by category:
///
/// ## Price-Based Signals
///
/// - **Momentum**: Short, medium, and long-term return momentum
/// - **MeanReversion**: Distance from moving averages, RSI extremes
/// - **Technical**: Breakouts, volume patterns, 52-week highs
///
/// ## Fundamental Signals
///
/// - **Value**: Book-to-price, earnings yield, FCF yield
/// - **Quality**: ROE, ROA, margins, earnings stability
/// - **Growth**: Earnings growth, revenue growth, estimate revisions
/// - **Earnings**: SUE, post-earnings drift
///
/// ## Alternative Signals
///
/// - **Sentiment**: News sentiment, social media, analyst revisions
/// - **Flow**: Institutional ownership, short interest
/// - **Events**: Index additions, spinoffs, buybacks
///
/// # Example
///
/// ```ignore
/// use tarifa::signals::{Momentum, Value, Quality};
/// use tarifa::Signal;
///
/// # fn example() -> tarifa::Result<()> {
/// // Create momentum signals
/// let mom_1m = Momentum::new(20);   // 1-month momentum
/// let mom_3m = Momentum::new(60);   // 3-month momentum
///
/// // Create fundamental signals
/// let value = Value::book_to_price();
/// let quality = Quality::roe();
///
/// // Use signals
/// let signals: Vec<Box<dyn Signal>> = vec![
///     Box::new(mom_1m),
///     Box::new(mom_3m),
///     Box::new(value),
///     Box::new(quality),
/// ];
/// # Ok(())
/// # }
/// ```
pub mod signals {
    pub use tarifa_signals::*;
}

// ============================================================================
// Signal Combination
// ============================================================================

/// Signal combination strategies.
///
/// This module contains implementations of the [`Combiner`] trait that
/// blend multiple signals into a composite alpha score.
///
/// ## Available Combiners
///
/// - **EqualWeightCombiner**: Simple average of z-scored signals
/// - **ICWeightedCombiner**: Weight by historical information coefficient
/// - **VolScaleCombiner**: IC-weighted with volatility scaling
/// - **MLEnsembleCombiner**: Machine learning meta-model (XGBoost/RandomForest)
///
/// # Example
///
/// ```ignore
/// use tarifa::combine::{EqualWeightCombiner, ICWeightedCombiner};
/// use tarifa::{Combiner, Signal};
/// use tarifa::types::SignalScore;
///
/// # fn example() -> tarifa::Result<()> {
/// // Equal weight combiner (simplest)
/// let equal_weight = EqualWeightCombiner::new();
///
/// // IC-weighted combiner (uses historical IC to weight signals)
/// let ic_weighted = ICWeightedCombiner::new(252); // 1-year lookback
///
/// // Use combiner with signal scores
/// let signal_scores: Vec<SignalScore> = vec![/* ... */];
/// let composite = ic_weighted.combine(&signal_scores)?;
/// # Ok(())
/// # }
/// ```
pub mod combine {
    pub use tarifa_combine::*;
}

// ============================================================================
// Signal Evaluation
// ============================================================================

/// Signal evaluation and backtesting.
///
/// This module contains tools for evaluating signal quality and conducting
/// backtests to measure predictive power.
///
/// ## Key Components
///
/// - **InformationCoefficient**: Calculate IC and IR metrics
/// - **RollingEvaluator**: Time-series evaluation with rolling windows
/// - **DecayAnalysis**: Analyze signal decay over different horizons
/// - **TurnoverAnalysis**: Measure signal stability and transaction costs
///
/// ## Evaluation Metrics
///
/// ### Information Coefficient (IC)
///
/// Correlation between signal scores and future returns:
///
/// ```text
/// IC_t = corr(signal_t, returns_{t+horizon})
/// ```
///
/// - IC > 0.02: Weak but usable
/// - IC > 0.05: Strong signal
/// - IC > 0.10: Likely overfit
///
/// ### Information Ratio (IR)
///
/// Consistency of the information coefficient:
///
/// ```text
/// IR = mean(IC) / std(IC)
/// ```
///
/// High IC with low IR suggests unreliable signal.
///
/// ### Turnover
///
/// How much the signal changes period-to-period:
///
/// ```text
/// Turnover = 1 - rank_correlation(signal_t, signal_{t-1})
/// ```
///
/// Target < 20% monthly turnover for practical implementation.
///
/// # Example
///
/// ```ignore
/// use tarifa::eval::{RollingEvaluator, InformationCoefficient};
/// use tarifa::{Signal, SignalEvaluator};
/// use tarifa::signals::Momentum;
/// use tarifa::types::MarketData;
///
/// # fn example() -> tarifa::Result<()> {
/// let market_data = MarketData::load("data/prices.parquet")?;
/// let signal = Momentum::new(20);
///
/// // Create evaluator
/// let evaluator = RollingEvaluator::new(&market_data, lookback_days = 252);
///
/// // Evaluate signal quality
/// let ic = evaluator.ic(&signal, horizon = 21);
/// let ir = evaluator.ir(&signal, horizon = 21);
/// let turnover = evaluator.turnover(&signal);
///
/// println!("Signal: {}", signal.name());
/// println!("IC: {:.3}", ic);
/// println!("IR: {:.2}", ir);
/// println!("Turnover: {:.1}%", turnover * 100.0);
/// # Ok(())
/// # }
/// ```
pub mod eval {
    pub use tarifa_eval::*;
}

// ============================================================================
// Prelude
// ============================================================================

/// Prelude module for convenient imports.
///
/// This module re-exports the most commonly used types and traits for
/// working with tarifa. Import it with:
///
/// ```ignore
/// use tarifa::prelude::*;
/// ```
///
/// This brings into scope:
/// - Core traits: [`Signal`], [`AlphaModel`], [`SignalEvaluator`], [`Combiner`]
/// - Common types: [`MarketData`], [`Symbol`], [`Date`], [`SignalScore`]
/// - Error types: [`Result`], [`TarifaError`]
pub mod prelude {
    pub use crate::traits::*;
    pub use crate::{AlphaModel, Combiner, Signal, SignalEvaluator};
    pub use crate::{Result, TarifaError};
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        // Version should be in semver format (x.y.z)
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }

    #[test]
    fn test_re_exports() {
        // This test verifies that all re-exports compile correctly
        // by using them in type annotations

        fn _accept_signal(_signal: &dyn Signal) {}
        fn _accept_alpha_model(_model: &dyn AlphaModel) {}
        fn _accept_evaluator(_eval: &dyn SignalEvaluator) {}
        fn _accept_combiner(_combiner: &dyn Combiner) {}

        // If this compiles, re-exports are working
    }

    #[test]
    fn test_error_types() {
        // Verify Result type works
        let _result: Result<()> = Ok(());

        // Verify error conversion works
        let _error: TarifaError = TarifaError::InvalidData("test".to_string());
    }
}
