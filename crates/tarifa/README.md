# tarifa

Alpha model for equity return prediction.

## What tarifa Is

tarifa predicts **future returns** from current signals. It outputs an expected return vector that feeds into a portfolio optimizer.

```text
Signals (t) → tarifa → E[returns(t+1)]
```text

The core workflow is:
1. Compute multiple alpha signals from market data
2. Evaluate signal quality (IC/IR, turnover, decay)
3. Combine signals into a composite alpha score
4. Scale to expected return units for optimization

## What tarifa Is Not

- **Not a risk model** - [perth](https://github.com/factordynamics/perth) handles covariance estimation and risk decomposition
- **Not an optimizer** - a separate component (cadiz) will take alpha + risk → weights
- **Not an execution system** - order routing and market impact are out of scope

tarifa focuses exclusively on the alpha generation problem: transforming raw market signals into forward-looking return predictions.

## Architecture Overview

tarifa is organized as a multi-crate workspace:

```text
tarifa/
├── tarifa              # Umbrella crate (this crate)
├── tarifa-traits       # Core trait definitions
├── tarifa-signals      # Signal implementations
├── tarifa-combine      # Signal combination strategies
└── tarifa-eval         # Backtesting and IC evaluation
```text

This umbrella crate re-exports all sub-crates for convenience:

```ignore
use tarifa::{Signal, AlphaModel, SignalEvaluator};
use tarifa::signals::Momentum;
use tarifa::combine::ICWeightedCombiner;
use tarifa::eval::InformationCoefficient;
```text

## Core Traits

### Signal

A `Signal` scores assets at a point in time based on historical data:

```ignore
pub trait Signal: Send + Sync {
    /// Unique identifier
    fn name(&self) -> &str;

    /// Compute signal scores for a universe at a point in time
    /// Returns cross-sectionally standardized scores (mean=0, std=1)
    fn score(&self, data: &MarketData, date: Date) -> Result<DataFrame>;

    /// Lookback period required for computation
    fn lookback(&self) -> usize;
}
```text

Signals are cross-sectionally standardized (z-scored) to make them comparable across different signal types.

### AlphaModel

An `AlphaModel` combines multiple signals into expected returns:

```ignore
pub trait AlphaModel: Send + Sync {
    /// Compute expected returns for a universe
    fn expected_returns(&self, universe: &[Symbol], date: Date) -> Result<Array1<f64>>;

    /// Get underlying signal scores (for analysis/debugging)
    fn signal_scores(&self, universe: &[Symbol], date: Date) -> Result<DataFrame>;

    /// List of signals used
    fn signals(&self) -> &[Box<dyn Signal>];
}
```text

### SignalEvaluator

A `SignalEvaluator` measures signal quality:

```ignore
pub trait SignalEvaluator {
    /// Information coefficient: corr(signal_t, returns_{t+horizon})
    fn ic(&self, signal: &dyn Signal, horizon: usize) -> f64;

    /// IC information ratio: mean(IC) / std(IC)
    fn ir(&self, signal: &dyn Signal, horizon: usize) -> f64;

    /// Signal turnover (autocorrelation of ranks)
    fn turnover(&self, signal: &dyn Signal) -> f64;
}
```text

Evaluation metrics:
- **IC > 0.02**: Weak but usable signal
- **IC > 0.05**: Strong signal
- **IC > 0.10**: Likely overfit, investigate further
- **High IR**: Consistent predictive power over time

### Combiner

A `Combiner` blends multiple signals into a composite alpha:

```ignore
pub trait Combiner: Send + Sync {
    /// Combine multiple signals into a composite alpha
    fn combine(&self, signals: &[SignalScore]) -> Result<Array1<f64>>;
}
```text

## Signal Categories

### Price-Based Signals

Momentum and mean-reversion patterns:

- **Momentum**: Short-term (1mo), medium-term (6mo), long-term (12mo) cumulative returns
- **Mean Reversion**: Distance from moving averages, RSI extremes
- **Technical**: Breakouts, volume patterns, 52-week high proximity

### Fundamental Signals

Balance sheet and income statement factors:

- **Value**: Book-to-price, earnings yield, FCF yield
- **Quality**: ROE, ROA, gross margins, earnings stability
- **Growth**: Earnings growth, revenue growth, estimate revisions
- **Earnings**: SUE (standardized unexpected earnings), post-earnings drift

### Alternative Signals

Non-traditional data sources:

- **Sentiment**: News sentiment, social media, analyst revisions
- **Flow**: Institutional ownership changes, short interest
- **Events**: Index additions, spinoffs, buybacks

## Signal Combination Strategies

tarifa provides multiple strategies for combining signals:

- **EqualWeight**: Simple average of z-scored signals
- **ICWeight**: Weight by historical information coefficient
- **VolScale**: IC-weighted with volatility scaling
- **MLEnsemble**: Machine learning meta-model (XGBoost/RandomForest)

## Integration with perth and cadiz

tarifa integrates with the Factor Dynamics ecosystem:

```ignore
use perth::RiskModel;
use tarifa::{AlphaModel, signals::Momentum, combine::ICWeightedCombiner};

// Step 1: Build alpha model
let signals: Vec<Box<dyn Signal>> = vec![
    Box::new(Momentum::new(20)),
    Box::new(Momentum::new(60)),
    // ... more signals
];

let combiner = ICWeightedCombiner::new(lookback_days);
let alpha_model = AlphaModel::new(signals, combiner);

// Step 2: Get expected returns
let expected_returns = alpha_model.expected_returns(&universe, today)?;

// Step 3: Get risk model from perth
let risk_model = RiskModel::fit(&market_data)?;
let covariance = risk_model.total_covariance();

// Step 4: Optimize (future cadiz integration)
// let optimizer = cadiz::MeanVarianceOptimizer::new(constraints);
// let weights = optimizer.optimize(&expected_returns, &covariance)?;
```text

## Usage Example

Here's a complete example of building and evaluating an alpha model:

```ignore
use tarifa::{Signal, AlphaModel, SignalEvaluator, Result};
use tarifa::signals::{Momentum, Value, Quality};
use tarifa::combine::ICWeightedCombiner;
use tarifa::eval::RollingEvaluator;
use tarifa::types::{MarketData, Date};

fn main() -> Result<()> {
    // Load market data
    let market_data = MarketData::load("data/prices.parquet")?;
    let universe = market_data.symbols();

    // Define signals
    let signals: Vec<Box<dyn Signal>> = vec![
        Box::new(Momentum::new(20)),   // 1-month momentum
        Box::new(Momentum::new(60)),   // 3-month momentum
        Box::new(Value::book_to_price()),
        Box::new(Quality::roe()),
    ];

    // Evaluate individual signals
    let evaluator = RollingEvaluator::new(&market_data, lookback_days);
    for signal in &signals {
        let ic = evaluator.ic(signal.as_ref(), horizon = 21);
        let ir = evaluator.ir(signal.as_ref(), horizon = 21);
        let turnover = evaluator.turnover(signal.as_ref());

        println!("{}: IC={:.3}, IR={:.2}, Turnover={:.1}%",
                 signal.name(), ic, ir, turnover * 100.0);
    }

    // Combine signals
    let combiner = ICWeightedCombiner::new(252); // 1-year lookback
    let alpha_model = AlphaModel::new(signals, combiner);

    // Generate expected returns
    let today = Date::today();
    let expected_returns = alpha_model.expected_returns(&universe, today)?;

    println!("Expected returns (top 10):");
    for (symbol, ret) in expected_returns.iter().take(10) {
        println!("{}: {:.2}%", symbol, ret * 100.0);
    }

    Ok(())
}
```text

## Data Flow

The typical tarifa workflow:

```text
┌─────────────────────────────────────────────────────────────┐
│ Market Data (prices, volumes, fundamentals)                 │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Signal Computation (tarifa-signals)                         │
│ - Compute raw signals with proper lags                      │
│ - Cross-sectional standardization                           │
│ - Handle missing data                                       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Signal Evaluation (tarifa-eval)                             │
│ - Calculate IC/IR for each signal                           │
│ - Analyze decay curves                                      │
│ - Measure turnover                                          │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Signal Combination (tarifa-combine)                         │
│ - Blend signals using IC weights or ML                      │
│ - Produce composite alpha score                             │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│ Expected Returns                                            │
│ - Scale composite to expected return units                  │
│ - Output: E[r] vector for optimizer                         │
└─────────────────────────────────────────────────────────────┘
```text

## Evaluation Metrics

### Information Coefficient (IC)

The correlation between signal scores and future returns:

```text
IC_t = corr(signal_t, returns_{t+horizon})
```text

### Information Ratio (IR)

The consistency of the information coefficient:

```text
IR = mean(IC) / std(IC)
```text

A high IC with low IR suggests an unreliable signal that works occasionally but not consistently.

### Turnover

How much the signal changes period-to-period:

```text
Turnover = 1 - rank_correlation(signal_t, signal_{t-1})
```text

High turnover leads to high transaction costs. Target < 20% monthly turnover for practical implementation.

### Decay Analysis

How quickly does signal predictive power decay with forecast horizon? This helps determine optimal rebalancing frequency.

## Development Status

tarifa is in **alpha development**. The API is unstable and subject to change.

Current status:
- Core traits defined in `tarifa-traits`
- Basic signal implementations (momentum, value, quality)
- Evaluation framework with IC/IR calculation
- Simple combiners (equal-weight, IC-weight)

Coming soon:
- ML-based signal combination
- Real-time signal computation
- Integration with perth risk models
- CLI for signal research and backtesting

## Contributing

tarifa is part of the Factor Dynamics ecosystem. For contribution guidelines, see the main repository.

## License

Licensed under the Apache License, Version 2.0. See LICENSE file for details.
