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
//! use tarifa::{Factor, AlphaModel, Result};
//! use tarifa::factors::momentum::ShortTermMomentum;
//! use tarifa::combine::EqualWeightCombiner;
//! use tarifa::types::{MarketData, Date};
//!
//! # fn main() -> Result<()> {
//! // Load market data
//! let market_data = MarketData::load("data/prices.parquet")?;
//! let universe = market_data.symbols();
//!
//! // Create factors
//! let factors: Vec<Box<dyn Factor>> = vec![
//!     Box::new(ShortTermMomentum::default()),
//! ];
//!
//! // Combine factors into alpha model
//! let combiner = EqualWeightCombiner::new();
//!
//! // Generate expected returns
//! // let expected_returns = alpha_model.expected_returns(&universe, Date::today())?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Crate Organization
//!
//! - [`traits`] - Core trait definitions ([`Factor`], [`AlphaModel`], etc.)
//! - [`factors`] - Factor implementations (momentum, value, quality, etc.) from the factors crate
//! - [`combine`] - Factor combination strategies
//! - [`eval`] - Factor evaluation and backtesting tools
//!
//! ## Architecture
//!
//! tarifa follows a modular architecture:
//!
//! 1. **Factors** compute scores for assets at a point in time
//! 2. **Evaluators** measure factor quality (IC, IR, turnover)
//! 3. **Combiners** blend multiple factors into composite alpha
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
/// - [`Factor`] - Single alpha factor that scores assets
/// - [`AlphaModel`] - Combined model that produces expected returns
/// - [`FactorEvaluator`] - Evaluates factor quality metrics
/// - [`Combiner`] - Combines multiple factors into composite alpha
///
/// # Example
///
/// ```ignore
/// use tarifa::{Factor, AlphaModel};
/// ```
pub mod traits {
    pub use tarifa_traits::*;
}

// Re-export core traits at top level for convenience
pub use tarifa_combine::Combiner;
pub use tarifa_traits::{AlphaModel, Factor, FactorEvaluator};

// Re-export error types
pub use tarifa_traits::{Result, TarifaError};

// Re-export common types
pub use tarifa_combine::SignalScore;
pub use tarifa_traits::types::{Date, MarketData, Symbol};

// ============================================================================
// Factor Implementations
// ============================================================================

/// Factor implementations.
///
/// This module re-exports the [`factors`] crate which contains 68 factor
/// implementations organized by category:
///
/// ## Price-Based Factors
///
/// - **Momentum**: Short, medium, and long-term return momentum
/// - **Volatility**: Historical volatility, beta, idiosyncratic vol
/// - **Liquidity**: Turnover, Amihud illiquidity, bid-ask spread
///
/// ## Fundamental Factors
///
/// - **Value**: Book-to-price, earnings yield, FCF yield
/// - **Quality**: ROE, ROA, margins, Piotroski F-score
/// - **Growth**: Earnings growth, revenue growth, asset growth
/// - **Size**: Market cap, enterprise value
///
/// ## Alternative Factors
///
/// - **Sentiment**: Analyst revisions, earnings surprise
///
/// # Example
///
/// ```ignore
/// use tarifa::factors::momentum::ShortTermMomentum;
/// use tarifa::factors::value::BookToPrice;
/// use tarifa::Factor;
///
/// # fn example() {
/// // Create momentum factors
/// let mom_1m = ShortTermMomentum::default();
///
/// // Create fundamental factors
/// let value = BookToPrice::default();
///
/// // Use factors
/// let factors: Vec<Box<dyn Factor>> = vec![
///     Box::new(mom_1m),
///     Box::new(value),
/// ];
/// # }
/// ```
pub mod factors {
    pub use factors::*;
}

// ============================================================================
// Factor Combination
// ============================================================================

/// Factor combination strategies.
///
/// This module contains implementations of the [`Combiner`] trait that
/// blend multiple factors into a composite alpha score.
///
/// ## Available Combiners
///
/// - **EqualWeightCombiner**: Simple average of z-scored factors
/// - **ICWeightedCombiner**: Weight by historical information coefficient
/// - **VolScaleCombiner**: IC-weighted with volatility scaling
///
/// # Example
///
/// ```ignore
/// use tarifa::combine::{EqualWeightCombiner};
/// use tarifa::{Combiner, Factor};
/// use tarifa::SignalScore;
///
/// # fn example() -> tarifa::Result<()> {
/// // Equal weight combiner (simplest)
/// let equal_weight = EqualWeightCombiner::new();
///
/// // Use combiner with factor scores
/// let factor_scores: Vec<SignalScore> = vec![/* ... */];
/// let composite = equal_weight.combine(&factor_scores)?;
/// # Ok(())
/// # }
/// ```
pub mod combine {
    pub use tarifa_combine::*;
}

// ============================================================================
// Factor Evaluation
// ============================================================================

/// Factor evaluation and backtesting.
///
/// This module contains tools for evaluating factor quality and conducting
/// backtests to measure predictive power.
///
/// ## Key Components
///
/// - **InformationCoefficient**: Calculate IC and IR metrics
/// - **DefaultEvaluator**: Evaluate factor performance with various metrics
/// - **DecayAnalysis**: Analyze factor decay over different horizons
/// - **Backtest**: Full backtesting framework with transaction costs
///
/// ## Evaluation Metrics
///
/// ### Information Coefficient (IC)
///
/// Correlation between factor scores and future returns:
///
/// ```text
/// IC_t = corr(factor_t, returns_{t+horizon})
/// ```
///
/// - IC > 0.02: Weak but usable
/// - IC > 0.05: Strong factor
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
/// High IC with low IR suggests unreliable factor.
///
/// ### Turnover
///
/// How much the factor changes period-to-period:
///
/// ```text
/// Turnover = 1 - rank_correlation(factor_t, factor_{t-1})
/// ```
///
/// Target < 20% monthly turnover for practical implementation.
///
/// # Example
///
/// ```ignore
/// use tarifa::eval::{DefaultEvaluator, EvaluatorConfig};
///
/// # fn example() {
/// // Create evaluator with pre-computed scores and returns
/// let evaluator = DefaultEvaluator::new(
///     signal_scores,
///     forward_returns,
///     EvaluatorConfig::default()
/// );
///
/// // Evaluate factor quality
/// let ic = evaluator.ic(21);
/// let ir = evaluator.ir(21);
/// let turnover = evaluator.turnover();
///
/// println!("IC: {:.3}", ic);
/// println!("IR: {:.2}", ir);
/// println!("Turnover: {:.1}%", turnover * 100.0);
/// # }
/// ```
pub mod eval {
    pub use tarifa_eval::*;
}

// ============================================================================
// Data Providers
// ============================================================================

/// Financial Modeling Prep (FMP) API client.
///
/// This module provides access to fundamental financial data from the FMP API,
/// including income statements, balance sheets, cash flows, and key metrics.
///
/// ## Setup
///
/// 1. Get a free API key at <https://financialmodelingprep.com/>
/// 2. Set the `FMP_API_KEY` environment variable or add to `.env` file
///
/// ## Example
///
/// ```ignore
/// use tarifa::fmp::{FmpClient, Period};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = FmpClient::from_env()?;
///
///     // Fetch comprehensive fundamental data
///     let data = client.fundamental_data("AAPL", 5).await?;
///
///     // Access key metrics
///     if let Some(metrics) = data.latest_metrics() {
///         println!("ROE: {:.2}%", metrics.roe * 100.0);
///         println!("P/E: {:.1}", metrics.pe_ratio);
///     }
///
///     Ok(())
/// }
/// ```
pub mod fmp {
    pub use tarifa_fmp::*;
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
/// - Core traits: [`Factor`], [`AlphaModel`], [`FactorEvaluator`], [`Combiner`]
/// - Common types: [`MarketData`], [`Symbol`], [`Date`], [`SignalScore`]
/// - Error types: [`Result`], [`TarifaError`]
pub mod prelude {
    pub use crate::traits::*;
    pub use crate::{AlphaModel, Combiner, Factor, FactorEvaluator};
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

        fn _accept_factor(_factor: &dyn Factor) {}
        fn _accept_alpha_model(_model: &dyn AlphaModel) {}
        fn _accept_evaluator(_eval: &dyn FactorEvaluator) {}
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
