# tarifa-traits

Core trait definitions for the Tarifa quantitative finance framework.

## Overview

This crate provides the foundational traits and types used throughout the Tarifa ecosystem:

- **Factor**: Re-exported from the `factors` crate, trait for implementing individual trading factors that score securities
- **AlphaModel**: Trait for combining factors into expected return predictions
- **FactorEvaluator**: Trait for evaluating factor quality and performance
- **Common Types**: Shared data structures like `MarketData` and error types

## Usage

These traits define the contracts for implementing quantitative trading strategies:

```rust,no_run
use tarifa_traits::{Factor, AlphaModel, MarketData, Result};
use factors::{FactorCategory, DataFrequency};
use polars::prelude::*;
use chrono::NaiveDate;

#[derive(Debug)]
struct MyFactor;

impl Factor for MyFactor {
    fn name(&self) -> &str {
        "my_factor"
    }

    fn description(&self) -> &str {
        "A custom trading factor"
    }

    fn category(&self) -> FactorCategory {
        FactorCategory::Momentum
    }

    fn required_columns(&self) -> &[&str] {
        &["close", "volume"]
    }

    fn lookback(&self) -> usize {
        20 // Days of historical data needed
    }

    fn frequency(&self) -> DataFrequency {
        DataFrequency::Daily
    }

    fn compute_raw(&self, data: &LazyFrame, date: NaiveDate) -> factors::Result<DataFrame> {
        // Compute factor scores
        todo!()
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
