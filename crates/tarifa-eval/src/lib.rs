//! Backtesting and signal evaluation for tarifa.
//!
//! This crate provides comprehensive tools for evaluating trading signals including:
//! - Information Coefficient (IC) calculations
//! - Signal quality metrics (IR, turnover, etc.)
//! - Signal decay analysis across time horizons
//! - Backtesting framework with transaction costs
//!
//! # Example
//!
//! ```rust,ignore
//! use tarifa_eval::{DefaultEvaluator, EvaluatorConfig, calculate_ic};
//! use tarifa_traits::SignalEvaluator;
//!
//! // Calculate IC between signal scores and returns
//! let ic = calculate_ic(&signal_scores, &forward_returns);
//!
//! // Use the evaluator for comprehensive analysis
//! let evaluator = DefaultEvaluator::new(data, EvaluatorConfig::default());
//! let ic = evaluator.ic(&signal, 21);
//! let ir = evaluator.ir(&signal, 21);
//! ```

pub mod backtest;
pub mod decay;
pub mod evaluator;
pub mod ic;
pub mod metrics;

// Re-export main types
pub use backtest::{Backtest, BacktestConfig, BacktestResult};
pub use decay::{DecayAnalysis, DecayCurve};
pub use evaluator::{DefaultEvaluator, EvaluatorConfig};
pub use ic::{calculate_ic, ic_series};
pub use metrics::{InformationRatio, MetricsConfig, SignalMetrics, SignalTurnover};
