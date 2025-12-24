//! Signal combination strategies for tarifa alpha models.
//!
//! This crate provides methods for combining multiple alpha signals into a composite score.
//! It implements various combination strategies including equal-weighting, IC-weighting,
//! and volatility-scaled approaches.
//!
//! # Examples
//!
//! ```rust,no_run
//! use tarifa_combine::{Combiner, EqualWeightCombiner, SignalScore};
//! use ndarray::Array1;
//!
//! let combiner = EqualWeightCombiner::default();
//! let signals = vec![
//!     SignalScore {
//!         name: "momentum".to_string(),
//!         scores: Array1::from_vec(vec![0.5, -0.2, 1.0]),
//!     },
//!     SignalScore {
//!         name: "value".to_string(),
//!         scores: Array1::from_vec(vec![-0.3, 0.8, 0.1]),
//!     },
//! ];
//!
//! let composite = combiner.combine(&signals).unwrap();
//! ```

mod combiner;
mod equal_weight;
mod ic_weight;
mod vol_scale;

// Re-export main types
pub use combiner::{Combiner, SignalScore};
pub use equal_weight::{EqualWeightCombiner, EqualWeightConfig};
pub use ic_weight::{ICWeightedCombiner, ICWeightedConfig};
pub use vol_scale::{VolScaledCombiner, VolScaledConfig};
