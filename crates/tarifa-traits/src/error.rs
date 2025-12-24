//! Error types for the Tarifa framework.
//!
//! This module defines the error types used throughout the Tarifa ecosystem,
//! providing comprehensive error handling for signal computation, data validation,
//! and other operations.

use thiserror::Error;

/// The main error type for Tarifa operations.
///
/// This enum encompasses all error cases that can occur when working with
/// signals, alpha models, and market data.
#[derive(Debug, Error)]
pub enum TarifaError {
    /// Error during signal computation.
    #[error("Signal computation failed: {0}")]
    SignalComputation(String),

    /// Error due to invalid or malformed data.
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Error when a required column is missing from the data.
    #[error("Missing required column: {0}")]
    MissingColumn(String),

    /// Error from Polars operations.
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// Error when data is insufficient for the requested operation.
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    /// Error when a symbol is not found in the universe.
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// Error when a date is out of range or invalid.
    #[error("Invalid date: {0}")]
    InvalidDate(String),

    /// Error fetching data from external sources.
    #[error("Data fetch error: {0}")]
    DataFetch(String),

    /// Error when a signal is not found.
    #[error("Signal not found: {0}")]
    SignalNotFound(String),

    /// Generic error for other cases.
    #[error("Error: {0}")]
    Other(String),
}

impl From<String> for TarifaError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for TarifaError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

/// A specialized Result type for Tarifa operations.
///
/// This is a convenience type that uses [`TarifaError`] as the error type.
pub type Result<T> = std::result::Result<T, TarifaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TarifaError::SignalComputation("test error".to_string());
        assert_eq!(err.to_string(), "Signal computation failed: test error");

        let err = TarifaError::MissingColumn("close".to_string());
        assert_eq!(err.to_string(), "Missing required column: close");
    }

    #[test]
    fn test_error_from_string() {
        let err: TarifaError = TarifaError::InvalidData("bad data".to_string());
        assert!(matches!(err, TarifaError::InvalidData(_)));
    }

    #[test]
    fn test_result_type() {
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());

        let err_result: Result<i32> = Err(TarifaError::Other("fail".to_string()));
        assert!(err_result.is_err());
    }
}
