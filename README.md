# tarifa

[![CI](https://github.com/factordynamics/tarifa/actions/workflows/ci.yml/badge.svg)](https://github.com/factordynamics/tarifa/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

Tarifa is an alpha signal research framework for quantitative equity analysis. It provides a modular system for defining, evaluating, and combining trading signals, with built-in support for backtesting and signal quality metrics. The framework computes cross-sectionally standardized scores (z-scores with mean=0, std=1) and evaluates them against forward returns using Information Coefficient (IC) analysis.

Signal categories include momentum (short/medium/long-term price trends), value (book-to-price, earnings yield, FCF yield), and quality (ROE, ROA, profit margins). Signals can be combined using equal-weighting, IC-weighting, or volatility-scaled approaches.

## Scope

Tarifa is an alpha model, not a risk model. It answers "what might returns be?" rather than "why did returns happen?" The framework generates forward-looking signals and evaluates their predictive power through IC, Information Ratio (IR), and decay analysis. Use these outputs for signal research and strategy development. For risk decomposition and factor attribution, see [perth](https://github.com/factordynamics/perth).

## Quick Start

List available signals:

```bash
just signals                    # List all signals
just signals --verbose          # With descriptions and lookback periods
```

Score stocks using a signal:

```bash
just score short_term_momentum AAPL,MSFT,GOOGL
just score earnings_yield AAPL,MSFT,GOOGL,AMZN,META
```

Evaluate signal quality:

```bash
cargo run --release -- eval short_term_momentum --symbols AAPL,MSFT,GOOGL -H 21
cargo run --release -- research medium_term_momentum --analysis all
```

Run a backtest:

```bash
just backtest momentum_6m 2020-01-01 2024-01-01
```

## Architecture

The framework is organized as a Cargo workspace with specialized crates:

- `tarifa-traits`: Core abstractions (`Signal`, `AlphaModel`, `SignalEvaluator`) and shared types
- `tarifa-signals`: Signal implementations for momentum, value, and quality factors
- `tarifa-combine`: Signal combination strategies (equal-weight, IC-weighted, volatility-scaled)
- `tarifa-eval`: Backtesting engine, IC calculation, decay curves, and signal metrics

Market data is fetched via [perth-data](https://github.com/factordynamics/perth), which sources from Yahoo Finance and SEC EDGAR.

## Signals

| Category | Signal | Description | Lookback |
|----------|--------|-------------|----------|
| Momentum | `short_term_momentum` | 1-month cumulative returns | 21 days |
| Momentum | `medium_term_momentum` | 6-month cumulative returns | 126 days |
| Momentum | `long_term_momentum` | 12-month returns (skip last month) | 252 days |
| Value | `book_to_price` | Book value / market cap | fundamentals |
| Value | `earnings_yield` | Earnings / market cap | fundamentals |
| Value | `fcf_yield` | Free cash flow / market cap | fundamentals |
| Quality | `return_on_equity` | Net income / equity | fundamentals |
| Quality | `return_on_assets` | Net income / assets | fundamentals |
| Quality | `profit_margins` | Gross/operating/net margins | fundamentals |

## Evaluation Metrics

- **Information Coefficient (IC)**: Spearman rank correlation between signal scores and forward returns
- **Information Ratio (IR)**: Mean IC / IC standard deviation, measures signal consistency
- **IC Hit Rate**: Percentage of periods with positive IC
- **Decay Analysis**: IC at multiple horizons, half-life estimation
- **Turnover**: Signal autocorrelation and position turnover rate

## Development

Requires Rust 1.88+ and [just](https://github.com/casey/just). Run `just ci` to ensure all tests and lints pass.

## License

MIT License - see [LICENSE](LICENSE).
