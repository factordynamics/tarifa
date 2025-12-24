//! Information Coefficient (IC) calculations.
//!
//! IC measures the Spearman rank correlation between signal scores and forward returns.
//! It is a key metric for evaluating signal predictive power.

use ndarray::Array1;

/// Calculate Information Coefficient between signal scores and future returns.
///
/// IC is computed as the Spearman rank correlation coefficient between the signal
/// scores and forward returns. Values range from -1 to 1, where:
/// - Positive values indicate the signal predicts returns in the correct direction
/// - Negative values indicate inverse correlation
/// - Values near zero indicate no predictive power
///
/// # Arguments
///
/// * `signal_scores` - Array of signal scores for assets
/// * `forward_returns` - Array of forward returns for the same assets
///
/// # Returns
///
/// Spearman rank correlation coefficient
///
/// # Example
///
/// ```rust,ignore
/// use ndarray::array;
/// use tarifa_eval::calculate_ic;
///
/// let scores = array![1.5, 0.3, -0.8, 2.1];
/// let returns = array![0.02, 0.01, -0.01, 0.03];
/// let ic = calculate_ic(&scores, &returns);
/// ```
pub fn calculate_ic(signal_scores: &Array1<f64>, forward_returns: &Array1<f64>) -> f64 {
    if signal_scores.len() != forward_returns.len() {
        return f64::NAN;
    }

    let n = signal_scores.len();
    if n < 2 {
        return f64::NAN;
    }

    // Filter out NaN values
    let pairs: Vec<(f64, f64)> = signal_scores
        .iter()
        .zip(forward_returns.iter())
        .filter_map(|(&s, &r)| {
            if s.is_finite() && r.is_finite() {
                Some((s, r))
            } else {
                None
            }
        })
        .collect();

    if pairs.len() < 2 {
        return f64::NAN;
    }

    // Compute ranks for signal scores
    let signal_ranks = compute_ranks(&pairs.iter().map(|(s, _)| *s).collect::<Vec<_>>());

    // Compute ranks for forward returns
    let return_ranks = compute_ranks(&pairs.iter().map(|(_, r)| *r).collect::<Vec<_>>());

    // Calculate Spearman correlation
    spearman_correlation(&signal_ranks, &return_ranks)
}

/// Calculate IC time series over multiple periods.
///
/// This function evaluates the signal's predictive power over time by computing
/// IC for each period in the provided dates.
///
/// # Arguments
///
/// * `signal_scores` - Time series of signal scores (dates x assets)
/// * `forward_returns` - Time series of forward returns (dates x assets)
/// * `dates_len` - Number of time periods to evaluate
///
/// # Returns
///
/// Vector of IC values, one per time period
///
/// # Example
///
/// ```rust,ignore
/// use tarifa_eval::ic_series;
///
/// let ic_ts = ic_series(&signal_scores, &forward_returns, dates.len());
/// ```
pub fn ic_series(
    signal_scores: &[Array1<f64>],
    forward_returns: &[Array1<f64>],
    dates_len: usize,
) -> Vec<f64> {
    let mut ics = Vec::with_capacity(dates_len);

    for i in 0..dates_len
        .min(signal_scores.len())
        .min(forward_returns.len())
    {
        let ic = calculate_ic(&signal_scores[i], &forward_returns[i]);
        ics.push(ic);
    }

    ics
}

/// Compute ranks of values (handling ties with average rank).
fn compute_ranks(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut indexed: Vec<(usize, f64)> = values.iter().enumerate().map(|(i, &v)| (i, v)).collect();

    // Sort by value
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; n];
    let mut i = 0;

    while i < n {
        let mut j = i;
        // Find ties
        while j < n && (indexed[j].1 - indexed[i].1).abs() < f64::EPSILON {
            j += 1;
        }

        // Average rank for ties
        let avg_rank = (i + j - 1) as f64 / 2.0;
        for k in i..j {
            ranks[indexed[k].0] = avg_rank;
        }

        i = j;
    }

    ranks
}

/// Calculate Spearman correlation coefficient.
fn spearman_correlation(ranks_x: &[f64], ranks_y: &[f64]) -> f64 {
    let n = ranks_x.len() as f64;

    if n < 2.0 {
        return f64::NAN;
    }

    // Calculate means
    let mean_x: f64 = ranks_x.iter().sum::<f64>() / n;
    let mean_y: f64 = ranks_y.iter().sum::<f64>() / n;

    // Calculate covariance and standard deviations
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n as usize {
        let dx = ranks_x[i] - mean_x;
        let dy = ranks_y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    if var_x == 0.0 || var_y == 0.0 {
        return f64::NAN;
    }

    cov / (var_x.sqrt() * var_y.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_calculate_ic_perfect_correlation() {
        let scores = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let returns = array![0.01, 0.02, 0.03, 0.04, 0.05];
        let ic = calculate_ic(&scores, &returns);
        assert!((ic - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_ic_negative_correlation() {
        let scores = array![5.0, 4.0, 3.0, 2.0, 1.0];
        let returns = array![0.01, 0.02, 0.03, 0.04, 0.05];
        let ic = calculate_ic(&scores, &returns);
        assert!((ic + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_ic_no_correlation() {
        let scores = array![1.0, 2.0, 3.0, 4.0];
        let returns = array![0.03, 0.01, 0.04, 0.02];
        let ic = calculate_ic(&scores, &returns);
        // IC should be between -1 and 1
        assert!(ic >= -1.0 && ic <= 1.0);
    }

    #[test]
    fn test_calculate_ic_with_nans() {
        let scores = array![1.0, 2.0, f64::NAN, 4.0];
        let returns = array![0.01, 0.02, 0.03, 0.04];
        let ic = calculate_ic(&scores, &returns);
        assert!(ic.is_finite());
    }

    #[test]
    fn test_ic_series() {
        let scores = vec![
            array![1.0, 2.0, 3.0],
            array![2.0, 1.0, 3.0],
            array![3.0, 2.0, 1.0],
        ];
        let returns = vec![
            array![0.01, 0.02, 0.03],
            array![0.02, 0.01, 0.03],
            array![0.03, 0.02, 0.01],
        ];

        let ics = ic_series(&scores, &returns, 3);
        assert_eq!(ics.len(), 3);
        assert!(ics.iter().all(|&ic| ic >= -1.0 && ic <= 1.0));
    }

    #[test]
    fn test_compute_ranks() {
        let values = vec![3.0, 1.0, 2.0, 5.0, 4.0];
        let ranks = compute_ranks(&values);
        assert_eq!(ranks, vec![2.0, 0.0, 1.0, 4.0, 3.0]);
    }

    #[test]
    fn test_compute_ranks_with_ties() {
        let values = vec![1.0, 2.0, 2.0, 3.0];
        let ranks = compute_ranks(&values);
        // Ranks should be [0, 1.5, 1.5, 3]
        assert!((ranks[0] - 0.0).abs() < 1e-10);
        assert!((ranks[1] - 1.5).abs() < 1e-10);
        assert!((ranks[2] - 1.5).abs() < 1e-10);
        assert!((ranks[3] - 3.0).abs() < 1e-10);
    }
}
