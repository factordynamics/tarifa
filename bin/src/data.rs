//! Data loading utilities for the Tarifa CLI.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use perth_data::yahoo::YahooQuoteProvider;
use tarifa_traits::{MarketData, TarifaError};

/// Load market data for the given symbols.
///
/// Fetches OHLCV data from Yahoo Finance for the specified symbols
/// and date range. The data is returned as a `MarketData` instance
/// suitable for use with signal computations.
pub(crate) async fn load_market_data(
    symbols: &[String],
    lookback_days: usize,
    end_date: Option<NaiveDate>,
) -> Result<MarketData, TarifaError> {
    let provider = YahooQuoteProvider::new();

    // Determine date range
    let end: DateTime<Utc> = match end_date {
        Some(d) => d
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| TarifaError::InvalidDate("Failed to construct end time".to_string()))?
            .and_utc(),
        None => Utc::now(),
    };

    // Convert trading days to calendar days (approx 1.5x) and add buffer
    // Trading year has ~252 days, calendar year has ~365 days
    let calendar_days = (lookback_days as f64 * 1.5) as i64 + 30;
    let start = end - Duration::days(calendar_days);

    // Fetch data for all symbols
    let df = provider
        .fetch_quotes_batch(symbols, start, end)
        .await
        .map_err(|e| TarifaError::DataFetch(e.to_string()))?;

    Ok(MarketData::new(df))
}

/// Parse a date string in YYYY-MM-DD format.
pub(crate) fn parse_date(date_str: &str) -> Result<NaiveDate, TarifaError> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| TarifaError::InvalidData(format!("Invalid date format: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_date() {
        let date = parse_date("2024-01-15").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);
    }

    #[test]
    fn test_parse_date_invalid() {
        let result = parse_date("invalid");
        assert!(result.is_err());
    }
}
