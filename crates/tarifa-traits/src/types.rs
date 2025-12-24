//! Common types used throughout the Tarifa framework.
//!
//! This module defines the core data types and structures used for representing
//! market data, symbols, and temporal information.

use polars::prelude::*;

// Re-export date type from chrono
pub use chrono::NaiveDate as Date;

/// A market symbol identifier.
///
/// Symbols are used to identify securities across the Tarifa framework.
/// Typically these are ticker symbols like "AAPL" or "MSFT".
pub type Symbol = String;

/// Container for market data.
///
/// `MarketData` wraps a Polars DataFrame containing prices, volumes,
/// and fundamental data for securities. This provides a zero-copy,
/// efficient representation of market information.
///
/// # Expected Schema
///
/// The DataFrame should typically contain columns such as:
/// - `symbol`: Security identifier
/// - `date`: Trading date
/// - `open`, `high`, `low`, `close`: Price data
/// - `volume`: Trading volume
/// - Additional fundamental or derived columns as needed
///
/// # Example
///
/// ```no_run
/// use tarifa_traits::MarketData;
/// use polars::prelude::*;
///
/// let df = df! {
///     "symbol" => &["AAPL", "MSFT"],
///     "close" => &[150.0, 300.0],
///     "volume" => &[1000000, 2000000],
/// }.unwrap();
///
/// let market_data = MarketData::new(df);
/// ```
#[derive(Debug, Clone)]
pub struct MarketData {
    /// The underlying DataFrame containing market data.
    data: DataFrame,
}

impl MarketData {
    /// Creates a new `MarketData` instance from a DataFrame.
    ///
    /// # Arguments
    ///
    /// * `data` - A Polars DataFrame containing market information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tarifa_traits::MarketData;
    /// use polars::prelude::*;
    ///
    /// let df = DataFrame::default();
    /// let market_data = MarketData::new(df);
    /// ```
    pub const fn new(data: DataFrame) -> Self {
        Self { data }
    }

    /// Returns a reference to the underlying DataFrame.
    ///
    /// This provides zero-copy access to the market data.
    pub const fn data(&self) -> &DataFrame {
        &self.data
    }

    /// Consumes self and returns the underlying DataFrame.
    ///
    /// This is useful when you need to take ownership of the data.
    pub fn into_inner(self) -> DataFrame {
        self.data
    }

    /// Returns the number of rows in the market data.
    pub fn len(&self) -> usize {
        self.data.height()
    }

    /// Returns whether the market data is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the column names in the market data.
    pub fn columns(&self) -> Vec<String> {
        self.data
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Checks if a column exists in the market data.
    ///
    /// # Arguments
    ///
    /// * `name` - The column name to check
    pub fn has_column(&self, name: &str) -> bool {
        self.data
            .get_column_names()
            .iter()
            .any(|s| s.as_str() == name)
    }

    /// Gets a column by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The column name to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(&Column)` if the column exists, `None` otherwise.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.data.column(name).ok()
    }
}

impl From<DataFrame> for MarketData {
    fn from(data: DataFrame) -> Self {
        Self::new(data)
    }
}

impl AsRef<DataFrame> for MarketData {
    fn as_ref(&self) -> &DataFrame {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_data_new() {
        let df = DataFrame::default();
        let market_data = MarketData::new(df);
        assert!(market_data.is_empty());
    }

    #[test]
    fn test_market_data_from_dataframe() {
        let df = df! {
            "symbol" => &["AAPL", "MSFT"],
            "close" => &[150.0, 300.0],
        }
        .unwrap();

        let market_data = MarketData::from(df);
        assert_eq!(market_data.len(), 2);
        assert!(market_data.has_column("symbol"));
        assert!(market_data.has_column("close"));
    }

    #[test]
    fn test_market_data_columns() {
        let df = df! {
            "symbol" => &["AAPL"],
            "close" => &[150.0],
            "volume" => &[1000000],
        }
        .unwrap();

        let market_data = MarketData::new(df);
        let columns = market_data.columns();
        assert_eq!(columns.len(), 3);
        assert!(columns.contains(&"symbol".to_string()));
        assert!(columns.contains(&"close".to_string()));
        assert!(columns.contains(&"volume".to_string()));
    }

    #[test]
    fn test_market_data_has_column() {
        let df = df! {
            "close" => &[150.0],
        }
        .unwrap();

        let market_data = MarketData::new(df);
        assert!(market_data.has_column("close"));
        assert!(!market_data.has_column("open"));
    }

    #[test]
    fn test_market_data_into_inner() {
        let df = df! {
            "close" => &[150.0],
        }
        .unwrap();

        let market_data = MarketData::new(df);
        let inner = market_data.into_inner();
        assert_eq!(inner.height(), 1);
    }

    #[test]
    fn test_symbol_type() {
        let symbol: Symbol = "AAPL".to_string();
        assert_eq!(symbol, "AAPL");
    }

    #[test]
    fn test_date_type() {
        use chrono::{Datelike, NaiveDate};
        let date: Date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert_eq!(date.year(), 2024);
    }
}
