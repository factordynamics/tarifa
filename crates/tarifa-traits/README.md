# tarifa-traits

Core trait definitions for the Tarifa quantitative finance framework.

## Overview

This crate provides the foundational traits and types used throughout the Tarifa ecosystem:

- **Signal**: Trait for implementing individual trading signals that score securities
- **AlphaModel**: Trait for combining signals into expected return predictions
- **SignalEvaluator**: Trait for evaluating signal quality and performance
- **Common Types**: Shared data structures like `MarketData` and error types

## Usage

These traits define the contracts for implementing quantitative trading strategies:

```rust
use tarifa_traits::{Signal, AlphaModel, MarketData, Result};
use polars::prelude::*;
use chrono::NaiveDate;

struct MySignal;

impl Signal for MySignal {
    fn name(&self) -> &str {
        "my_signal"
    }

    fn score(&self, data: &MarketData, date: NaiveDate) -> Result<DataFrame> {
        // Compute signal scores
        todo!()
    }

    fn lookback(&self) -> usize {
        20 // Days of historical data needed
    }

    fn required_columns(&self) -> &[&str] {
        &["close", "volume"]
    }
}
```

## Features

- Zero-copy data structures using Polars DataFrames
- Thread-safe trait implementations
- Comprehensive error handling
- Type-safe date and symbol handling

## License

See the repository root for license information.
