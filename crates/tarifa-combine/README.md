# tarifa-combine

Signal combination strategies for tarifa alpha models.

## Overview

`tarifa-combine` provides methods for combining multiple alpha signals into a composite score. It implements various combination strategies including equal-weighting, IC-weighting, and volatility-scaled approaches.

## Combination Strategies

- **Equal Weight**: Simple average of z-scored signals
- **IC Weight**: Weight signals by their historical Information Coefficient
- **Vol Scale**: IC-weighted combination with volatility targeting

## Usage

```rust
use tarifa_combine::{Combiner, EqualWeightCombiner, SignalScore};
use ndarray::Array1;

let combiner = EqualWeightCombiner::default();
let signals = vec![
    SignalScore {
        name: "momentum".to_string(),
        scores: Array1::from_vec(vec![0.5, -0.2, 1.0]),
    },
    SignalScore {
        name: "value".to_string(),
        scores: Array1::from_vec(vec![-0.3, 0.8, 0.1]),
    },
];

let composite = combiner.combine(&signals)?;
```

## License

MIT OR Apache-2.0
