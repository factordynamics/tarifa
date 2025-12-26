//! Data types for FMP API responses.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Reporting period for financial statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Period {
    /// Annual reports (10-K filings).
    #[default]
    Annual,
    /// Quarterly reports (10-Q filings).
    Quarter,
}

impl Period {
    /// Get the API parameter value.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Annual => "annual",
            Self::Quarter => "quarter",
        }
    }
}

/// Income statement data from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomeStatement {
    /// Filing date.
    pub date: String,
    /// Ticker symbol.
    pub symbol: String,
    /// Reporting period (annual/quarterly).
    pub period: String,
    /// Total revenue.
    #[serde(default)]
    pub revenue: f64,
    /// Cost of revenue (COGS).
    #[serde(default)]
    pub cost_of_revenue: f64,
    /// Gross profit.
    #[serde(default)]
    pub gross_profit: f64,
    /// Operating expenses.
    #[serde(default)]
    pub operating_expenses: f64,
    /// Operating income.
    #[serde(default)]
    pub operating_income: f64,
    /// Net income.
    #[serde(default)]
    pub net_income: f64,
    /// Earnings per share (basic).
    #[serde(default)]
    pub eps: f64,
    /// Earnings per share (diluted).
    #[serde(default)]
    pub eps_diluted: f64,
    /// Weighted average shares outstanding.
    #[serde(default)]
    pub weighted_average_shs_out: f64,
    /// EBITDA.
    #[serde(default)]
    pub ebitda: f64,
}

impl IncomeStatement {
    /// Parse the date string into a NaiveDate.
    #[must_use]
    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }

    /// Calculate gross margin.
    #[must_use]
    pub fn gross_margin(&self) -> f64 {
        if self.revenue > 0.0 {
            self.gross_profit / self.revenue
        } else {
            0.0
        }
    }

    /// Calculate operating margin.
    #[must_use]
    pub fn operating_margin(&self) -> f64 {
        if self.revenue > 0.0 {
            self.operating_income / self.revenue
        } else {
            0.0
        }
    }

    /// Calculate net margin.
    #[must_use]
    pub fn net_margin(&self) -> f64 {
        if self.revenue > 0.0 {
            self.net_income / self.revenue
        } else {
            0.0
        }
    }
}

/// Balance sheet data from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheet {
    /// Filing date.
    pub date: String,
    /// Ticker symbol.
    pub symbol: String,
    /// Reporting period.
    pub period: String,
    /// Total assets.
    #[serde(default)]
    pub total_assets: f64,
    /// Total current assets.
    #[serde(default)]
    pub total_current_assets: f64,
    /// Cash and cash equivalents.
    #[serde(default)]
    pub cash_and_cash_equivalents: f64,
    /// Total liabilities.
    #[serde(default)]
    pub total_liabilities: f64,
    /// Total current liabilities.
    #[serde(default)]
    pub total_current_liabilities: f64,
    /// Total debt.
    #[serde(default)]
    pub total_debt: f64,
    /// Total stockholders' equity.
    #[serde(default)]
    pub total_stockholders_equity: f64,
    /// Total equity (including non-controlling interests).
    #[serde(default)]
    pub total_equity: f64,
    /// Retained earnings.
    #[serde(default)]
    pub retained_earnings: f64,
    /// Common stock.
    #[serde(default)]
    pub common_stock: f64,
    /// Goodwill.
    #[serde(default)]
    pub goodwill: f64,
    /// Intangible assets.
    #[serde(default)]
    pub intangible_assets: f64,
}

impl BalanceSheet {
    /// Parse the date string into a NaiveDate.
    #[must_use]
    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }

    /// Calculate book value (tangible).
    #[must_use]
    pub fn book_value(&self) -> f64 {
        self.total_stockholders_equity - self.goodwill - self.intangible_assets
    }
}

/// Cash flow statement data from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashFlowStatement {
    /// Filing date.
    pub date: String,
    /// Ticker symbol.
    pub symbol: String,
    /// Reporting period.
    pub period: String,
    /// Net income.
    #[serde(default)]
    pub net_income: f64,
    /// Operating cash flow.
    #[serde(default)]
    pub operating_cash_flow: f64,
    /// Capital expenditure.
    #[serde(default)]
    pub capital_expenditure: f64,
    /// Free cash flow.
    #[serde(default)]
    pub free_cash_flow: f64,
    /// Dividends paid.
    #[serde(default)]
    pub dividends_paid: f64,
    /// Stock repurchased.
    #[serde(default)]
    pub common_stock_repurchased: f64,
}

impl CashFlowStatement {
    /// Parse the date string into a NaiveDate.
    #[must_use]
    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }
}

/// Key financial metrics from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyMetrics {
    /// Filing date.
    pub date: String,
    /// Ticker symbol.
    pub symbol: String,
    /// Reporting period.
    pub period: String,
    /// Market capitalization.
    #[serde(default)]
    pub market_cap: f64,
    /// Enterprise value.
    #[serde(default)]
    pub enterprise_value: f64,
    /// Price to earnings ratio.
    #[serde(default)]
    pub pe_ratio: f64,
    /// Price to book ratio.
    #[serde(default)]
    pub pb_ratio: f64,
    /// Price to sales ratio.
    #[serde(default)]
    pub price_to_sales_ratio: f64,
    /// Price to free cash flow ratio.
    #[serde(default)]
    pub pfcf_ratio: f64,
    /// Earnings yield.
    #[serde(default)]
    pub earnings_yield: f64,
    /// Free cash flow yield.
    #[serde(default)]
    pub free_cash_flow_yield: f64,
    /// Dividend yield.
    #[serde(default)]
    pub dividend_yield: f64,
    /// Book value per share.
    #[serde(default)]
    pub book_value_per_share: f64,
    /// Tangible book value per share.
    #[serde(default)]
    pub tangible_book_value_per_share: f64,
    /// Revenue per share.
    #[serde(default)]
    pub revenue_per_share: f64,
    /// Net income per share.
    #[serde(default)]
    pub net_income_per_share: f64,
    /// Return on equity.
    #[serde(default)]
    pub roe: f64,
    /// Return on assets.
    #[serde(default)]
    pub return_on_tangible_assets: f64,
    /// Current ratio.
    #[serde(default)]
    pub current_ratio: f64,
    /// Debt to equity.
    #[serde(default)]
    pub debt_to_equity: f64,
    /// Debt to assets.
    #[serde(default)]
    pub debt_to_assets: f64,
}

impl KeyMetrics {
    /// Parse the date string into a NaiveDate.
    #[must_use]
    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }

    /// Book-to-price ratio (inverse of P/B).
    #[must_use]
    pub fn book_to_price(&self) -> f64 {
        if self.pb_ratio > 0.0 {
            1.0 / self.pb_ratio
        } else {
            0.0
        }
    }
}

/// Financial ratios from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialRatios {
    /// Filing date.
    pub date: String,
    /// Ticker symbol.
    pub symbol: String,
    /// Reporting period.
    pub period: String,
    // Profitability ratios
    /// Gross profit margin.
    #[serde(default)]
    pub gross_profit_margin: f64,
    /// Operating profit margin.
    #[serde(default)]
    pub operating_profit_margin: f64,
    /// Net profit margin.
    #[serde(default)]
    pub net_profit_margin: f64,
    /// Return on assets.
    #[serde(default)]
    pub return_on_assets: f64,
    /// Return on equity.
    #[serde(default)]
    pub return_on_equity: f64,
    /// Return on capital employed.
    #[serde(default)]
    pub return_on_capital_employed: f64,
    // Liquidity ratios
    /// Current ratio.
    #[serde(default)]
    pub current_ratio: f64,
    /// Quick ratio.
    #[serde(default)]
    pub quick_ratio: f64,
    /// Cash ratio.
    #[serde(default)]
    pub cash_ratio: f64,
    // Leverage ratios
    /// Debt ratio.
    #[serde(default)]
    pub debt_ratio: f64,
    /// Debt to equity ratio.
    #[serde(default)]
    pub debt_equity_ratio: f64,
    // Efficiency ratios
    /// Asset turnover.
    #[serde(default)]
    pub asset_turnover: f64,
    /// Inventory turnover.
    #[serde(default)]
    pub inventory_turnover: f64,
    // Valuation ratios
    /// Price to earnings ratio.
    #[serde(default)]
    pub price_earnings_ratio: f64,
    /// Price to book ratio.
    #[serde(default)]
    pub price_to_book_ratio: f64,
    /// Price to sales ratio.
    #[serde(default)]
    pub price_to_sales_ratio: f64,
    /// Price to free cash flow ratio.
    #[serde(default)]
    pub price_to_free_cash_flows_ratio: f64,
    /// Dividend yield.
    #[serde(default)]
    pub dividend_yield: f64,
}

impl FinancialRatios {
    /// Parse the date string into a NaiveDate.
    #[must_use]
    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }
}

/// Real-time quote data from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    /// Ticker symbol.
    pub symbol: String,
    /// Company name.
    pub name: String,
    /// Current price.
    pub price: f64,
    /// Change in price.
    #[serde(default)]
    pub change: f64,
    /// Percent change.
    #[serde(default)]
    pub changes_percentage: f64,
    /// Day high.
    #[serde(default)]
    pub day_high: f64,
    /// Day low.
    #[serde(default)]
    pub day_low: f64,
    /// 52-week high.
    #[serde(default)]
    pub year_high: f64,
    /// 52-week low.
    #[serde(default)]
    pub year_low: f64,
    /// Market cap.
    #[serde(default)]
    pub market_cap: f64,
    /// Volume.
    #[serde(default)]
    pub volume: f64,
    /// Average volume.
    #[serde(default)]
    pub avg_volume: f64,
    /// Open price.
    #[serde(default)]
    pub open: f64,
    /// Previous close.
    #[serde(default)]
    pub previous_close: f64,
    /// EPS.
    #[serde(default)]
    pub eps: f64,
    /// P/E ratio.
    #[serde(default)]
    pub pe: f64,
}

/// Historical price data from FMP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPrice {
    /// Date.
    pub date: String,
    /// Open price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
    /// Adjusted close.
    #[serde(rename = "adjClose", default)]
    pub adj_close: f64,
    /// Volume.
    #[serde(default)]
    pub volume: f64,
}

impl HistoricalPrice {
    /// Parse the date string into a NaiveDate.
    #[must_use]
    pub fn parsed_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }
}

/// Wrapper for historical price response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPriceResponse {
    /// Symbol.
    pub symbol: String,
    /// Historical prices.
    pub historical: Vec<HistoricalPrice>,
}

/// Comprehensive fundamental data for a symbol.
#[derive(Debug, Clone)]
pub struct FundamentalData {
    /// Ticker symbol.
    pub symbol: String,
    /// Income statements (most recent first).
    pub income_statements: Vec<IncomeStatement>,
    /// Balance sheets (most recent first).
    pub balance_sheets: Vec<BalanceSheet>,
    /// Cash flow statements (most recent first).
    pub cash_flows: Vec<CashFlowStatement>,
    /// Key metrics (most recent first).
    pub key_metrics: Vec<KeyMetrics>,
    /// Financial ratios (most recent first).
    pub ratios: Vec<FinancialRatios>,
    /// Current quote.
    pub quote: Option<Quote>,
}

impl FundamentalData {
    /// Get the most recent income statement.
    #[must_use]
    pub fn latest_income(&self) -> Option<&IncomeStatement> {
        self.income_statements.first()
    }

    /// Get the most recent balance sheet.
    #[must_use]
    pub fn latest_balance(&self) -> Option<&BalanceSheet> {
        self.balance_sheets.first()
    }

    /// Get the most recent cash flow statement.
    #[must_use]
    pub fn latest_cash_flow(&self) -> Option<&CashFlowStatement> {
        self.cash_flows.first()
    }

    /// Get the most recent key metrics.
    #[must_use]
    pub fn latest_metrics(&self) -> Option<&KeyMetrics> {
        self.key_metrics.first()
    }

    /// Get the most recent ratios.
    #[must_use]
    pub fn latest_ratios(&self) -> Option<&FinancialRatios> {
        self.ratios.first()
    }

    /// Get current market cap.
    #[must_use]
    pub fn market_cap(&self) -> Option<f64> {
        self.quote.as_ref().map(|q| q.market_cap)
    }

    /// Calculate ROE from latest data.
    #[must_use]
    pub fn roe(&self) -> Option<f64> {
        let income = self.latest_income()?;
        let balance = self.latest_balance()?;
        if balance.total_stockholders_equity > 0.0 {
            Some(income.net_income / balance.total_stockholders_equity)
        } else {
            None
        }
    }

    /// Calculate ROA from latest data.
    #[must_use]
    pub fn roa(&self) -> Option<f64> {
        let income = self.latest_income()?;
        let balance = self.latest_balance()?;
        if balance.total_assets > 0.0 {
            Some(income.net_income / balance.total_assets)
        } else {
            None
        }
    }

    /// Calculate earnings yield from latest data.
    #[must_use]
    pub fn earnings_yield(&self) -> Option<f64> {
        let income = self.latest_income()?;
        let market_cap = self.market_cap()?;
        if market_cap > 0.0 {
            Some(income.net_income / market_cap)
        } else {
            None
        }
    }

    /// Calculate FCF yield from latest data.
    #[must_use]
    pub fn fcf_yield(&self) -> Option<f64> {
        let cf = self.latest_cash_flow()?;
        let market_cap = self.market_cap()?;
        if market_cap > 0.0 {
            Some(cf.free_cash_flow / market_cap)
        } else {
            None
        }
    }

    /// Calculate book-to-price from latest data.
    #[must_use]
    pub fn book_to_price(&self) -> Option<f64> {
        let balance = self.latest_balance()?;
        let market_cap = self.market_cap()?;
        if market_cap > 0.0 {
            Some(balance.total_stockholders_equity / market_cap)
        } else {
            None
        }
    }
}
