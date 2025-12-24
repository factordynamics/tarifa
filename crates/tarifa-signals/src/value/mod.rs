//! Value signals based on fundamental valuation metrics.
//!
//! This module provides traditional value signals that compare fundamental
//! metrics to market prices:
//! - Book-to-price: Book value relative to market capitalization
//! - Earnings yield: Earnings relative to market capitalization
//! - Free cash flow yield: Free cash flow relative to market capitalization
//!
//! All signals apply cross-sectional standardization and optional winsorization
//! to handle outliers.

mod book_to_price;
mod earnings_yield;
mod fcf_yield;

pub use book_to_price::{BookToPrice, BookToPriceConfig};
pub use earnings_yield::{EarningsYield, EarningsYieldConfig};
pub use fcf_yield::{FreeCashFlowYield, FreeCashFlowYieldConfig};
