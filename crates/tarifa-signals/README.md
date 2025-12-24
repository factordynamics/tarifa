# tarifa-signals

Signal implementations for the tarifa alpha model.

This crate provides concrete implementations of alpha signals across multiple categories:
- **Momentum**: Short-term, medium-term, and long-term price momentum
- **Value**: Book-to-price, earnings yield, free cash flow yield
- **Quality**: Return on equity, return on assets, profit margins

Each signal implements the `Signal` trait from `tarifa-traits` and produces cross-sectionally standardized scores.

## Usage

```rust
use tarifa_signals::momentum::ShortTermMomentum;
use tarifa_signals::registry::{available_signals, signals_by_category};

// Create a momentum signal with default configuration
let signal = ShortTermMomentum::default();

// Discover all available signals
let all_signals = available_signals();

// Get signals by category
let momentum_signals = signals_by_category(&SignalCategory::Momentum);
```

## Signal Registry

The registry module provides metadata about all available signals, including their category, description, and typical lookback periods.
