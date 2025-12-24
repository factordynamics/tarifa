//! Momentum signals based on historical price returns.
//!
//! This module provides momentum signals at different time horizons:
//! - Short-term: 1-month momentum (21 trading days)
//! - Medium-term: 6-month momentum (126 trading days)
//! - Long-term: 12-month momentum (252 trading days), skipping the most recent month
//!
//! Each signal computes cumulative returns over the lookback period and applies
//! cross-sectional standardization.

mod long_term;
mod medium_term;
mod short_term;

pub use long_term::{LongTermMomentum, LongTermMomentumConfig};
pub use medium_term::{MediumTermMomentum, MediumTermMomentumConfig};
pub use short_term::{ShortTermMomentum, ShortTermMomentumConfig};
