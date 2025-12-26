//! Error types for FMP API client.

use thiserror::Error;

/// Errors that can occur when using the FMP API.
#[derive(Debug, Error)]
pub enum FmpError {
    /// Missing API key.
    #[error("FMP_API_KEY environment variable not set")]
    MissingApiKey,

    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// JSON parsing failed.
    #[error("Failed to parse JSON response: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error.
    #[error("FMP API error: {0}")]
    Api(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded. Free tier allows 250 requests/day.")]
    RateLimitExceeded,

    /// Symbol not found.
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// No data available.
    #[error("No data available for {0}")]
    NoData(String),

    /// Environment variable error.
    #[error("Environment error: {0}")]
    Env(#[from] dotenvy::Error),
}
