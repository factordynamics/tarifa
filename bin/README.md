# tarifa-cli

Command-line interface for the Tarifa alpha model.

## Installation

```bash
cargo install tarifa-cli
```

## Usage

```bash
# List available signals
tarifa signals
tarifa signals --category momentum
tarifa signals --verbose

# Show signal scores for specific stocks
tarifa score momentum_12m AAPL,MSFT,GOOGL
tarifa score book_to_price AAPL,MSFT --date 2024-12-23

# Evaluate signal quality (IC/IR)
tarifa eval momentum_12m --symbols AAPL,MSFT,GOOGL --horizon 21
tarifa eval book_to_price --symbols AAPL,MSFT --start 2023-01-01 --end 2024-12-23

# Run backtest
tarifa backtest momentum_12m --start 2023-01-01 --end 2024-12-23
tarifa backtest combined_alpha --start 2023-01-01 --end 2024-12-23 --universe sp500

# Combine multiple signals
tarifa combine --signals momentum_12m,book_to_price,roe --method equal AAPL,MSFT
tarifa combine --signals momentum_12m,book_to_price --method ic-weight AAPL,MSFT,GOOGL

# Research signal properties
tarifa research momentum_12m --analysis ic
tarifa research book_to_price --analysis decay --horizon 21
tarifa research momentum_12m --analysis all --start 2023-01-01
```

## Commands

### `signals`

List all available signals in the Tarifa signal library:
- Price-based signals (momentum, mean reversion, technical)
- Fundamental signals (value, quality, growth, earnings)
- Alternative signals (sentiment, flow, events)

### `score`

Compute current signal scores for specific ticker symbols:
- Returns cross-sectionally standardized scores (mean=0, std=1)
- Supports both raw and standardized output
- Can specify historical dates for backtesting

### `eval`

Evaluate signal quality metrics:
- Information Coefficient (IC): correlation between signal and future returns
- IC Information Ratio (IR): consistency of predictive power (mean IC / std IC)
- Forward-looking evaluation over specified horizon

### `backtest`

Run historical backtest of a signal or combined alpha model:
- Performance metrics (return, Sharpe ratio, max drawdown)
- Signal quality metrics (average IC, IR, turnover)
- Works with individual signals or composite models

### `combine`

Combine multiple signals into a composite alpha score:
- Equal-weight: simple average of standardized signals
- IC-weight: weight by historical information coefficient
- ML ensemble: machine learning-based combination (future)

### `research`

In-depth signal research and analysis:
- IC analysis: time-series IC, hit rate, consistency
- Decay curves: predictive power vs. horizon
- Turnover: signal stability (rank autocorrelation)
- Comprehensive reports for signal evaluation

## Signal Categories

### Price-Based
- **Momentum**: 1-month, 6-month, 12-month cumulative returns
- **Mean Reversion**: Distance from moving averages, RSI
- **Technical**: Breakouts, volume patterns, 52-week high proximity

### Fundamental
- **Value**: Book-to-price, earnings yield, FCF yield
- **Quality**: ROE, ROA, gross margin, earnings stability
- **Growth**: Earnings growth, revenue growth, estimate revisions
- **Earnings**: Standardized unexpected earnings (SUE)

### Alternative
- **Sentiment**: News sentiment, social media, analyst revisions
- **Flow**: Institutional ownership changes, short interest
- **Events**: Index additions, spinoffs, buybacks

## Output Formats

Most commands support `--format` flag for output:
- `text` (default): Human-readable formatted output
- `json`: Machine-readable JSON for programmatic use

## Configuration

Tarifa integrates with Perth for market data:
- Uses `perth-data` crate for price and fundamental data
- Supports Yahoo Finance and other data sources
- Configurable caching for performance
