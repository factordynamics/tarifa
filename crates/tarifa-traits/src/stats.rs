//! Statistical utility functions for signal processing.
//!
//! This module provides common statistical operations used across
//! signal implementations and combiners, such as z-score standardization.

use ndarray::Array1;

/// Minimum threshold for standard deviation to avoid division by zero.
/// Values below this threshold are treated as zero variance.
pub const MIN_STD_THRESHOLD: f64 = 1e-10;

/// Z-score standardization result containing computed statistics.
#[derive(Debug, Clone, Copy)]
pub struct StandardizeResult {
    /// The computed mean of the input values.
    pub mean: f64,
    /// The computed sample standard deviation (N-1 denominator).
    pub std: f64,
    /// Whether the standardization was applied (false if variance was too low).
    pub applied: bool,
}

/// Standardize a slice of f64 values to z-scores (mean=0, std=1).
///
/// Uses sample standard deviation (N-1 denominator) for unbiased estimation.
/// If the standard deviation is below the minimum threshold, returns zeros
/// to avoid division by near-zero values.
///
/// # Arguments
///
/// * `values` - The input values to standardize
///
/// # Returns
///
/// A tuple containing:
/// - The standardized values as a `Vec<f64>`
/// - A `StandardizeResult` with the computed statistics
///
/// # Edge Cases
///
/// - Empty input: Returns empty vector with mean=NaN, std=NaN, applied=false
/// - Single value: Returns [0.0] with std=0.0, applied=false
/// - Constant values: Returns zeros with applied=false
/// - Contains NaN/Inf: NaN values are excluded from mean/std calculation,
///   and NaN values in input produce NaN in output
///
/// # Examples
///
/// ```
/// use tarifa_traits::stats::standardize;
///
/// let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let (standardized, result) = standardize(&values);
///
/// assert!(result.applied);
/// assert!((result.mean - 3.0).abs() < 1e-10);
/// // Standardized values should have mean ~0 and std ~1
/// ```
pub fn standardize(values: &[f64]) -> (Vec<f64>, StandardizeResult) {
    if values.is_empty() {
        return (
            Vec::new(),
            StandardizeResult {
                mean: f64::NAN,
                std: f64::NAN,
                applied: false,
            },
        );
    }

    // Filter out non-finite values for statistics calculation
    let finite_values: Vec<f64> = values.iter().filter(|x| x.is_finite()).copied().collect();

    if finite_values.is_empty() {
        return (
            vec![f64::NAN; values.len()],
            StandardizeResult {
                mean: f64::NAN,
                std: f64::NAN,
                applied: false,
            },
        );
    }

    let n = finite_values.len();
    let mean = finite_values.iter().sum::<f64>() / n as f64;

    // Sample variance with N-1 denominator (Bessel's correction)
    let variance = if n > 1 {
        finite_values
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64
    } else {
        0.0
    };
    let std = variance.sqrt();

    let applied = std > MIN_STD_THRESHOLD;

    let standardized = if applied {
        values.iter().map(|x| (x - mean) / std).collect()
    } else {
        vec![0.0; values.len()]
    };

    (standardized, StandardizeResult { mean, std, applied })
}

/// Standardize a slice of f64 values to z-scores in-place.
///
/// This is a more efficient version of [`standardize`] when you don't need
/// to keep the original values.
///
/// # Arguments
///
/// * `values` - The input values to standardize (modified in-place)
///
/// # Returns
///
/// A `StandardizeResult` with the computed statistics.
///
/// # Examples
///
/// ```
/// use tarifa_traits::stats::standardize_inplace;
///
/// let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let result = standardize_inplace(&mut values);
///
/// assert!(result.applied);
/// // Values are now standardized
/// ```
pub fn standardize_inplace(values: &mut [f64]) -> StandardizeResult {
    if values.is_empty() {
        return StandardizeResult {
            mean: f64::NAN,
            std: f64::NAN,
            applied: false,
        };
    }

    // Filter out non-finite values for statistics calculation
    let finite_values: Vec<f64> = values.iter().filter(|x| x.is_finite()).copied().collect();

    if finite_values.is_empty() {
        for v in values.iter_mut() {
            *v = f64::NAN;
        }
        return StandardizeResult {
            mean: f64::NAN,
            std: f64::NAN,
            applied: false,
        };
    }

    let n = finite_values.len();
    let mean = finite_values.iter().sum::<f64>() / n as f64;

    // Sample variance with N-1 denominator (Bessel's correction)
    let variance = if n > 1 {
        finite_values
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64
    } else {
        0.0
    };
    let std = variance.sqrt();

    let applied = std > MIN_STD_THRESHOLD;

    if applied {
        for v in values.iter_mut() {
            *v = (*v - mean) / std;
        }
    } else {
        for v in values.iter_mut() {
            *v = 0.0;
        }
    }

    StandardizeResult { mean, std, applied }
}

/// Standardize an ndarray Array1 to z-scores (mean=0, std=1).
///
/// Uses sample standard deviation (ddof=1) for unbiased estimation.
/// If the standard deviation is below the minimum threshold, returns zeros.
///
/// This function is designed for use with combiners that work with ndarray.
///
/// # Arguments
///
/// * `scores` - The input array to standardize
///
/// # Returns
///
/// A tuple containing:
/// - The standardized array as `Array1<f64>`
/// - A `StandardizeResult` with the computed statistics
///
/// # Examples
///
/// ```
/// use tarifa_traits::stats::standardize_array;
/// use ndarray::Array1;
///
/// let scores = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
/// let (standardized, result) = standardize_array(&scores);
///
/// assert!(result.applied);
/// ```
pub fn standardize_array(scores: &Array1<f64>) -> (Array1<f64>, StandardizeResult) {
    if scores.is_empty() {
        return (
            Array1::zeros(0),
            StandardizeResult {
                mean: f64::NAN,
                std: f64::NAN,
                applied: false,
            },
        );
    }

    let mean = scores.mean().unwrap_or(0.0);
    let std = scores.std(1.0); // ddof=1 for sample std

    let applied = std > MIN_STD_THRESHOLD;

    let standardized = if applied {
        (scores - mean) / std
    } else {
        Array1::zeros(scores.len())
    };

    (standardized, StandardizeResult { mean, std, applied })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standardize_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (standardized, result) = standardize(&values);

        assert!(result.applied);
        assert!((result.mean - 3.0).abs() < 1e-10);

        // Check mean of standardized values is ~0
        let std_mean: f64 = standardized.iter().sum::<f64>() / standardized.len() as f64;
        assert!(std_mean.abs() < 1e-10);

        // Check std of standardized values is ~1
        let std_variance: f64 = standardized.iter().map(|x| x.powi(2)).sum::<f64>()
            / (standardized.len() - 1) as f64;
        assert!((std_variance.sqrt() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_standardize_empty() {
        let values: Vec<f64> = vec![];
        let (standardized, result) = standardize(&values);

        assert!(standardized.is_empty());
        assert!(!result.applied);
        assert!(result.mean.is_nan());
        assert!(result.std.is_nan());
    }

    #[test]
    fn test_standardize_single_value() {
        let values = vec![42.0];
        let (standardized, result) = standardize(&values);

        assert_eq!(standardized.len(), 1);
        assert!(!result.applied);
        assert!(standardized[0].abs() < 1e-10);
    }

    #[test]
    fn test_standardize_constant_values() {
        let values = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let (standardized, result) = standardize(&values);

        assert!(!result.applied);
        assert!(standardized.iter().all(|&x| x.abs() < 1e-10));
    }

    #[test]
    fn test_standardize_with_nan() {
        let values = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
        let (standardized, result) = standardize(&values);

        assert!(result.applied);
        // Mean should be computed from finite values only
        assert!((result.mean - 3.0).abs() < 1e-10);
        // The NaN should remain NaN in output
        assert!(standardized[2].is_nan());
    }

    #[test]
    fn test_standardize_inplace_basic() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = standardize_inplace(&mut values);

        assert!(result.applied);
        assert!((result.mean - 3.0).abs() < 1e-10);

        // Check mean of standardized values is ~0
        let std_mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        assert!(std_mean.abs() < 1e-10);
    }

    #[test]
    fn test_standardize_array_basic() {
        let scores = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let (standardized, result) = standardize_array(&scores);

        assert!(result.applied);
        assert!((result.mean - 3.0).abs() < 1e-10);
        assert_eq!(standardized.len(), 5);
    }

    #[test]
    fn test_standardize_array_constant() {
        let scores = Array1::from_vec(vec![5.0, 5.0, 5.0, 5.0]);
        let (standardized, result) = standardize_array(&scores);

        assert!(!result.applied);
        assert!(standardized.iter().all(|&x| x.abs() < 1e-10));
    }

    #[test]
    fn test_standardize_negative_values() {
        let values = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let (standardized, result) = standardize(&values);

        assert!(result.applied);
        assert!(result.mean.abs() < 1e-10);

        // Standardized should also have mean 0
        let std_mean: f64 = standardized.iter().sum::<f64>() / standardized.len() as f64;
        assert!(std_mean.abs() < 1e-10);
    }

    #[test]
    fn test_min_std_threshold() {
        // Values with very small variance
        let values = vec![
            1.0,
            1.0 + 1e-12,
            1.0 - 1e-12,
            1.0 + 2e-12,
            1.0 - 2e-12,
        ];
        let (standardized, result) = standardize(&values);

        // Should not apply standardization due to low variance
        assert!(!result.applied);
        assert!(standardized.iter().all(|&x| x.abs() < 1e-10));
    }
}
