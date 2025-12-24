//! Quality signals based on profitability and operational efficiency.
//!
//! This module provides signals that measure the fundamental quality of businesses:
//! - Return on equity (ROE): Net income relative to shareholder equity
//! - Return on assets (ROA): Net income relative to total assets
//! - Profit margins: Gross, operating, and net margins
//!
//! Higher quality companies tend to outperform over the long term.

mod margins;
mod roa;
mod roe;

pub use margins::{MarginType, ProfitMargins, ProfitMarginsConfig};
pub use roa::{ReturnOnAssets, ReturnOnAssetsConfig};
pub use roe::{ReturnOnEquity, ReturnOnEquityConfig};
