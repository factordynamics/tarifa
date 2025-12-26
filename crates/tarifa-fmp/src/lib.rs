//! Financial Modeling Prep (FMP) API client for Tarifa.
//!
//! This crate provides a client for fetching fundamental financial data from
//! the [Financial Modeling Prep](https://financialmodelingprep.com/) API.
//!
//! # Usage
//!
//! ```rust,ignore
//! use tarifa_fmp::FmpClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = FmpClient::from_env()?;
//!
//!     // Fetch income statement
//!     let income = client.income_statement("AAPL", Period::Annual, Some(5)).await?;
//!
//!     // Fetch balance sheet
//!     let balance = client.balance_sheet("AAPL", Period::Annual, Some(5)).await?;
//!
//!     // Fetch key metrics (ROE, ROA, margins, etc.)
//!     let metrics = client.key_metrics("AAPL", Period::Annual, Some(5)).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Environment Variables
//!
//! Set `FMP_API_KEY` in your environment or `.env` file:
//!
//! ```bash
//! FMP_API_KEY=your_api_key_here
//! ```

mod client;
mod error;
mod types;

pub use client::FmpClient;
pub use error::FmpError;
pub use types::*;

/// Result type for FMP operations.
pub type Result<T> = std::result::Result<T, FmpError>;
