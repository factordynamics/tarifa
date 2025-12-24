#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/tarifa/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! Core trait definitions for the Tarifa quantitative finance framework.
//!
//! This crate provides the foundational abstractions for building quantitative
//! trading strategies, including signal generation, alpha modeling, and evaluation.

/// The version of the tarifa-traits crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Module declarations
pub mod alpha;
pub mod error;
pub mod evaluator;
pub mod signal;
pub mod types;

// Re-exports
pub use alpha::AlphaModel;
pub use error::{Result, TarifaError};
pub use evaluator::SignalEvaluator;
pub use signal::Signal;
pub use types::{Date, MarketData, Symbol};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'));
    }
}
