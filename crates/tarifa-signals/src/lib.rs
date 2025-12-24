//! Signal implementations for tarifa alpha model.
//!
//! This crate provides concrete signal implementations across multiple categories:
//! - Momentum: Short-term, medium-term, and long-term price momentum
//! - Value: Book-to-price, earnings yield, free cash flow yield
//! - Quality: Return on equity, return on assets, profit margins
//!
//! Each signal produces cross-sectionally standardized scores (mean=0, std=1).
//!
//! # Example
//!
//! ```ignore
//! use tarifa_signals::momentum::ShortTermMomentum;
//! use tarifa_signals::registry::available_signals;
//!
//! // Create a signal with default configuration
//! let signal = ShortTermMomentum::default();
//!
//! // Discover available signals
//! let signals = available_signals();
//! ```

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

pub mod momentum;
pub mod quality;
pub mod registry;
pub mod value;

// Re-export key types
pub use registry::{SignalCategory, SignalInfo};
