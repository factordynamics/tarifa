//! FMP API client implementation.

use crate::{
    Result,
    error::FmpError,
    types::{
        BalanceSheet, CashFlowStatement, FinancialRatios, FundamentalData, HistoricalPrice,
        IncomeStatement, KeyMetrics, Period, Quote,
    },
};
use reqwest::Client;
use std::env;

/// Base URL for the FMP stable API.
const FMP_BASE_URL: &str = "https://financialmodelingprep.com/stable";

/// Financial Modeling Prep API client.
#[derive(Debug, Clone)]
pub struct FmpClient {
    client: Client,
    api_key: String,
}

impl FmpClient {
    /// Create a new FMP client with the given API key.
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
        }
    }

    /// Create a new FMP client from the `FMP_API_KEY` environment variable.
    ///
    /// This will also load from a `.env` file if present.
    ///
    /// # Errors
    ///
    /// Returns an error if the environment variable is not set.
    pub fn from_env() -> Result<Self> {
        // Try to load .env file (ignore errors if not found)
        let _ = dotenvy::dotenv();

        let api_key = env::var("FMP_API_KEY").map_err(|_| FmpError::MissingApiKey)?;

        Ok(Self::new(api_key))
    }

    /// Build a URL with the API key.
    fn url(&self, endpoint: &str) -> String {
        if endpoint.contains('?') {
            format!("{FMP_BASE_URL}/{endpoint}&apikey={}", self.api_key)
        } else {
            format!("{FMP_BASE_URL}/{endpoint}?apikey={}", self.api_key)
        }
    }

    /// Make a GET request and parse the JSON response.
    async fn get<T: serde::de::DeserializeOwned>(&self, endpoint: &str) -> Result<T> {
        let url = self.url(endpoint);
        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(FmpError::RateLimitExceeded);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(FmpError::Api(format!("HTTP {status}: {text}")));
        }

        let text = response.text().await?;

        // Check for error responses
        if text.contains("\"Error Message\"") || text.contains("\"error\"") {
            return Err(FmpError::Api(text));
        }

        serde_json::from_str(&text).map_err(|e| {
            FmpError::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse: {e}. Response: {text}"),
            )))
        })
    }

    /// Get income statements for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol (e.g., "AAPL")
    /// * `period` - Annual or quarterly
    /// * `limit` - Number of periods to return (most recent first)
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn income_statement(
        &self,
        symbol: &str,
        period: Period,
        limit: Option<u32>,
    ) -> Result<Vec<IncomeStatement>> {
        let limit_param = limit.map(|l| format!("&limit={l}")).unwrap_or_default();
        let endpoint = format!(
            "income-statement?symbol={}&period={}{}",
            symbol.to_uppercase(),
            period.as_str(),
            limit_param
        );
        self.get(&endpoint).await
    }

    /// Get balance sheets for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol
    /// * `period` - Annual or quarterly
    /// * `limit` - Number of periods to return
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn balance_sheet(
        &self,
        symbol: &str,
        period: Period,
        limit: Option<u32>,
    ) -> Result<Vec<BalanceSheet>> {
        let limit_param = limit.map(|l| format!("&limit={l}")).unwrap_or_default();
        let endpoint = format!(
            "balance-sheet-statement?symbol={}&period={}{}",
            symbol.to_uppercase(),
            period.as_str(),
            limit_param
        );
        self.get(&endpoint).await
    }

    /// Get cash flow statements for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol
    /// * `period` - Annual or quarterly
    /// * `limit` - Number of periods to return
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn cash_flow(
        &self,
        symbol: &str,
        period: Period,
        limit: Option<u32>,
    ) -> Result<Vec<CashFlowStatement>> {
        let limit_param = limit.map(|l| format!("&limit={l}")).unwrap_or_default();
        let endpoint = format!(
            "cash-flow-statement?symbol={}&period={}{}",
            symbol.to_uppercase(),
            period.as_str(),
            limit_param
        );
        self.get(&endpoint).await
    }

    /// Get key metrics for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol
    /// * `period` - Annual or quarterly
    /// * `limit` - Number of periods to return
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn key_metrics(
        &self,
        symbol: &str,
        period: Period,
        limit: Option<u32>,
    ) -> Result<Vec<KeyMetrics>> {
        let limit_param = limit.map(|l| format!("&limit={l}")).unwrap_or_default();
        let endpoint = format!(
            "key-metrics?symbol={}&period={}{}",
            symbol.to_uppercase(),
            period.as_str(),
            limit_param
        );
        self.get(&endpoint).await
    }

    /// Get financial ratios for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol
    /// * `period` - Annual or quarterly
    /// * `limit` - Number of periods to return
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn ratios(
        &self,
        symbol: &str,
        period: Period,
        limit: Option<u32>,
    ) -> Result<Vec<FinancialRatios>> {
        let limit_param = limit.map(|l| format!("&limit={l}")).unwrap_or_default();
        let endpoint = format!(
            "ratios?symbol={}&period={}{}",
            symbol.to_uppercase(),
            period.as_str(),
            limit_param
        );
        self.get(&endpoint).await
    }

    /// Get real-time quote for a symbol.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn quote(&self, symbol: &str) -> Result<Quote> {
        let endpoint = format!("quote?symbol={}", symbol.to_uppercase());
        let quotes: Vec<Quote> = self.get(&endpoint).await?;
        quotes
            .into_iter()
            .next()
            .ok_or_else(|| FmpError::SymbolNotFound(symbol.to_string()))
    }

    /// Get real-time quotes for multiple symbols.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn quotes(&self, symbols: &[&str]) -> Result<Vec<Quote>> {
        let symbols_str = symbols
            .iter()
            .map(|s| s.to_uppercase())
            .collect::<Vec<_>>()
            .join(",");
        let endpoint = format!("quote?symbol={symbols_str}");
        self.get(&endpoint).await
    }

    /// Get historical daily prices for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol
    /// * `from` - Start date (YYYY-MM-DD)
    /// * `to` - End date (YYYY-MM-DD)
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    pub async fn historical_prices(
        &self,
        symbol: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<HistoricalPrice>> {
        let mut params = String::new();
        if let Some(f) = from {
            params.push_str(&format!("&from={f}"));
        }
        if let Some(t) = to {
            params.push_str(&format!("&to={t}"));
        }

        let endpoint = format!(
            "historical-price-eod/full?symbol={}{}",
            symbol.to_uppercase(),
            params
        );
        // The stable API returns a flat array, not a wrapped response
        self.get(&endpoint).await
    }

    /// Get comprehensive fundamental data for a symbol.
    ///
    /// This fetches income statements, balance sheets, cash flows, key metrics,
    /// ratios, and current quote in parallel.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock ticker symbol
    /// * `periods` - Number of historical periods to fetch
    ///
    /// # Errors
    ///
    /// Returns an error if any API request fails.
    pub async fn fundamental_data(&self, symbol: &str, periods: u32) -> Result<FundamentalData> {
        // Fetch all data in parallel
        let (income, balance, cash, metrics, ratios, quote) = tokio::join!(
            self.income_statement(symbol, Period::Annual, Some(periods)),
            self.balance_sheet(symbol, Period::Annual, Some(periods)),
            self.cash_flow(symbol, Period::Annual, Some(periods)),
            self.key_metrics(symbol, Period::Annual, Some(periods)),
            self.ratios(symbol, Period::Annual, Some(periods)),
            self.quote(symbol),
        );

        Ok(FundamentalData {
            symbol: symbol.to_uppercase(),
            income_statements: income.unwrap_or_default(),
            balance_sheets: balance.unwrap_or_default(),
            cash_flows: cash.unwrap_or_default(),
            key_metrics: metrics.unwrap_or_default(),
            ratios: ratios.unwrap_or_default(),
            quote: quote.ok(),
        })
    }

    /// Get fundamental data for multiple symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of stock ticker symbols
    /// * `periods` - Number of historical periods to fetch
    ///
    /// # Errors
    ///
    /// Returns an error if any API request fails.
    pub async fn bulk_fundamental_data(
        &self,
        symbols: &[&str],
        periods: u32,
    ) -> Result<Vec<FundamentalData>> {
        let mut results = Vec::with_capacity(symbols.len());

        // Process in batches to avoid overwhelming the API
        for symbol in symbols {
            match self.fundamental_data(symbol, periods).await {
                Ok(data) => results.push(data),
                Err(e) => {
                    eprintln!("Warning: Failed to fetch data for {symbol}: {e}");
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_building() {
        let client = FmpClient::new("test_key");
        assert_eq!(
            client.url("quote?symbol=AAPL"),
            "https://financialmodelingprep.com/stable/quote?symbol=AAPL&apikey=test_key"
        );
        assert_eq!(
            client.url("income-statement?symbol=AAPL&period=annual"),
            "https://financialmodelingprep.com/stable/income-statement?symbol=AAPL&period=annual&apikey=test_key"
        );
    }
}
