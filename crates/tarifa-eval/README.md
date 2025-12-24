# tarifa-eval

Backtesting and signal evaluation framework for tarifa.

## Features

- **Information Coefficient (IC)**: Calculate Spearman rank correlation between signal scores and forward returns
- **Signal Metrics**: Information Ratio, turnover, autocorrelation, and other quality metrics
- **Decay Analysis**: Analyze signal predictive power over multiple time horizons
- **Backtesting**: Full backtesting framework with transaction costs and rebalancing
- **Signal Evaluation**: Comprehensive evaluation of signal quality and performance

## Usage

```rust
use tarifa_eval::{DefaultEvaluator, EvaluatorConfig, calculate_ic};
use tarifa_traits::SignalEvaluator;

// Calculate IC between signal scores and returns
let ic = calculate_ic(&signal_scores, &forward_returns);

// Use the evaluator for comprehensive analysis
let evaluator = DefaultEvaluator::new(data, EvaluatorConfig::default());
let ic = evaluator.ic(&signal, 21);
let ir = evaluator.ir(&signal, 21);
let turnover = evaluator.turnover(&signal);
```

## Modules

- `ic`: Information Coefficient calculations
- `metrics`: Signal quality metrics (IR, turnover, etc.)
- `decay`: Signal decay analysis across time horizons
- `backtest`: Backtesting framework with transaction costs
- `evaluator`: SignalEvaluator trait implementation
