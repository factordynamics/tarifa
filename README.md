# tarifa

[![CI](https://github.com/factordynamics/tarifa/actions/workflows/ci.yml/badge.svg)](https://github.com/factordynamics/tarifa/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Alpha signal research framework for quantitative equity analysis.

## Overview

Tarifa provides a modular system for defining, evaluating, and combining trading signals with built-in backtesting support. The framework computes cross-sectionally standardized scores (z-scores) and evaluates them against forward returns using Information Coefficient (IC) analysis.

For factor definitions and implementations, see [factors](https://github.com/factordynamics/factors).

## Scope

Tarifa is an alpha model, not a risk model. It answers "what might returns be?" rather than "why did returns happen?" For risk decomposition and factor attribution, see [perth](https://github.com/factordynamics/perth).

## Examples

Run backtest examples using real market data from [Financial Modeling Prep](https://financialmodelingprep.com/):

```bash
# Set your API key
export FMP_API_KEY=your_api_key_here

# Run all examples
just examples

# Or run individually
cargo run --release -p tarifa-examples --example momentum_backtest
cargo run --release -p tarifa-examples --example value_backtest
cargo run --release -p tarifa-examples --example quality_backtest
cargo run --release -p tarifa-examples --example multifactor_backtest
```

See [examples/CONSIDERATIONS.md](examples/CONSIDERATIONS.md) for known limitations and future improvements.

## Architecture

The framework is organized as a Cargo workspace:

- `tarifa-traits`: Core abstractions (`Signal`, `AlphaModel`, `SignalEvaluator`)
- `tarifa-combine`: Signal combination strategies (equal-weight, IC-weighted, volatility-scaled)
- `tarifa-eval`: Backtesting engine, IC calculation, decay curves
- `tarifa-fmp`: Financial Modeling Prep API client for market data

## Evaluation Metrics

- **Information Coefficient (IC)**: Spearman rank correlation between signal scores and forward returns
- **Information Ratio (IR)**: Mean IC / IC standard deviation
- **Sharpe Ratio**: Risk-adjusted returns (annualized)
- **Maximum Drawdown**: Largest peak-to-trough decline
- **Win Rate**: Percentage of positive return periods

## Development

Requires Rust 1.88+ and [just](https://github.com/casey/just).

```bash
just ci        # Run full CI suite (fmt, clippy, test, udeps)
just examples  # Run all backtest examples
```

## License

MIT License - see [LICENSE](LICENSE).
