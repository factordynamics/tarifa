//! Signal decay analysis.
//!
//! Analyzes how signal predictive power decays over different time horizons.
//! Useful for determining optimal signal holding periods and rebalancing frequency.

use serde::{Deserialize, Serialize};

/// Decay curve data points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayCurve {
    /// Time horizons (in days)
    pub horizons: Vec<usize>,
    /// IC values at each horizon
    pub ic_values: Vec<f64>,
    /// Standard errors of IC at each horizon
    pub ic_std_errors: Vec<f64>,
}

impl DecayCurve {
    /// Create a new decay curve.
    pub const fn new(horizons: Vec<usize>, ic_values: Vec<f64>, ic_std_errors: Vec<f64>) -> Self {
        Self {
            horizons,
            ic_values,
            ic_std_errors,
        }
    }

    /// Get IC at a specific horizon (interpolated if needed).
    pub fn ic_at_horizon(&self, horizon: usize) -> Option<f64> {
        // Find exact match
        if let Some(pos) = self.horizons.iter().position(|&h| h == horizon) {
            return Some(self.ic_values[pos]);
        }

        // Linear interpolation
        for i in 0..self.horizons.len() - 1 {
            if self.horizons[i] < horizon && horizon < self.horizons[i + 1] {
                let h1 = self.horizons[i] as f64;
                let h2 = self.horizons[i + 1] as f64;
                let ic1 = self.ic_values[i];
                let ic2 = self.ic_values[i + 1];

                let weight = (horizon as f64 - h1) / (h2 - h1);
                return Some(ic1 + weight * (ic2 - ic1));
            }
        }

        None
    }

    /// Estimate half-life: horizon at which IC drops to 50% of initial value.
    pub fn half_life(&self) -> Option<f64> {
        if self.ic_values.is_empty() {
            return None;
        }

        let initial_ic = self.ic_values[0].abs();
        let half_ic = initial_ic / 2.0;

        // Find where IC crosses half value
        for i in 0..self.ic_values.len() - 1 {
            let ic1 = self.ic_values[i].abs();
            let ic2 = self.ic_values[i + 1].abs();

            if ic1 >= half_ic && ic2 <= half_ic {
                // Interpolate
                let h1 = self.horizons[i] as f64;
                let h2 = self.horizons[i + 1] as f64;
                let weight = (ic1 - half_ic) / (ic1 - ic2);
                return Some(h1 + weight * (h2 - h1));
            }
        }

        None
    }
}

/// Signal decay analysis across multiple time horizons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayAnalysis {
    /// Decay curve data
    pub curve: DecayCurve,
    /// Estimated half-life in days
    pub half_life: Option<f64>,
    /// Maximum IC and its horizon
    pub max_ic: (usize, f64),
    /// Whether signal has monotonic decay
    pub is_monotonic: bool,
}

impl DecayAnalysis {
    /// Analyze signal decay across multiple horizons.
    ///
    /// # Arguments
    ///
    /// * `horizons` - Time horizons to analyze (in days)
    /// * `ic_calculator` - Function that calculates IC for a given horizon
    ///
    /// # Returns
    ///
    /// DecayAnalysis with calculated metrics
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use tarifa_eval::DecayAnalysis;
    ///
    /// let horizons = vec![1, 5, 10, 21, 42, 63];
    /// let analysis = DecayAnalysis::analyze(&horizons, |h| calculate_ic_at_horizon(h));
    /// ```
    pub fn analyze<F>(horizons: &[usize], mut ic_calculator: F) -> Self
    where
        F: FnMut(usize) -> (f64, f64), // Returns (IC, std_error)
    {
        let mut ic_values = Vec::with_capacity(horizons.len());
        let mut ic_std_errors = Vec::with_capacity(horizons.len());

        for &horizon in horizons {
            let (ic, std_err) = ic_calculator(horizon);
            ic_values.push(ic);
            ic_std_errors.push(std_err);
        }

        let curve = DecayCurve::new(horizons.to_vec(), ic_values.clone(), ic_std_errors);
        let half_life = curve.half_life();

        // Find maximum IC
        let max_ic = ic_values
            .iter()
            .enumerate()
            .filter(|(_, ic)| ic.is_finite())
            .max_by(|(_, a), (_, b)| {
                a.abs()
                    .partial_cmp(&b.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(idx, &ic)| (horizons[idx], ic))
            .unwrap_or((0, f64::NAN));

        // Check if monotonic decay (absolute IC values decrease)
        let is_monotonic = ic_values.windows(2).all(|w| w[0].abs() >= w[1].abs());

        Self {
            curve,
            half_life,
            max_ic,
            is_monotonic,
        }
    }

    /// Standard horizons for decay analysis (1, 5, 10, 21, 42, 63 days).
    pub fn standard_horizons() -> Vec<usize> {
        vec![1, 5, 10, 21, 42, 63]
    }

    /// Short-term horizons (1, 2, 3, 5, 10 days).
    pub fn short_term_horizons() -> Vec<usize> {
        vec![1, 2, 3, 5, 10]
    }

    /// Long-term horizons (21, 42, 63, 126, 252 days).
    pub fn long_term_horizons() -> Vec<usize> {
        vec![21, 42, 63, 126, 252]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_curve() {
        let horizons = vec![1, 5, 10, 21];
        let ic_values = vec![0.10, 0.08, 0.05, 0.02];
        let std_errors = vec![0.01, 0.01, 0.01, 0.01];

        let curve = DecayCurve::new(horizons, ic_values, std_errors);

        assert_eq!(curve.ic_at_horizon(1), Some(0.10));
        assert_eq!(curve.ic_at_horizon(21), Some(0.02));
    }

    #[test]
    fn test_decay_curve_interpolation() {
        let horizons = vec![1, 10, 20];
        let ic_values = vec![0.10, 0.05, 0.00];
        let std_errors = vec![0.01, 0.01, 0.01];

        let curve = DecayCurve::new(horizons, ic_values, std_errors);

        let ic_5 = curve.ic_at_horizon(5);
        assert!(ic_5.is_some());
        let ic_5 = ic_5.unwrap();
        assert!(ic_5 > 0.05 && ic_5 < 0.10);
    }

    #[test]
    fn test_half_life() {
        let horizons = vec![1, 5, 10, 21];
        let ic_values = vec![0.10, 0.08, 0.05, 0.02];
        let std_errors = vec![0.01, 0.01, 0.01, 0.01];

        let curve = DecayCurve::new(horizons, ic_values, std_errors);
        let half_life = curve.half_life();

        assert!(half_life.is_some());
        let hl = half_life.unwrap();
        assert!(hl > 0.0 && hl < 21.0);
    }

    #[test]
    fn test_decay_analysis() {
        let horizons = vec![1, 5, 10, 21];
        let analysis = DecayAnalysis::analyze(&horizons, |h| {
            let ic = match h {
                1 => 0.10,
                5 => 0.08,
                10 => 0.05,
                21 => 0.02,
                _ => 0.0,
            };
            (ic, 0.01)
        });

        assert_eq!(analysis.max_ic.0, 1);
        assert!((analysis.max_ic.1 - 0.10).abs() < 1e-10);
        assert!(analysis.is_monotonic);
        assert!(analysis.half_life.is_some());
    }

    #[test]
    fn test_non_monotonic_decay() {
        let horizons = vec![1, 5, 10, 21];
        let analysis = DecayAnalysis::analyze(&horizons, |h| {
            let ic = match h {
                1 => 0.05,
                5 => 0.10, // Peak at 5 days
                10 => 0.07,
                21 => 0.02,
                _ => 0.0,
            };
            (ic, 0.01)
        });

        assert!(!analysis.is_monotonic);
        assert_eq!(analysis.max_ic.0, 5);
    }

    #[test]
    fn test_standard_horizons() {
        let horizons = DecayAnalysis::standard_horizons();
        assert_eq!(horizons, vec![1, 5, 10, 21, 42, 63]);
    }

    #[test]
    fn test_short_term_horizons() {
        let horizons = DecayAnalysis::short_term_horizons();
        assert_eq!(horizons, vec![1, 2, 3, 5, 10]);
    }

    #[test]
    fn test_long_term_horizons() {
        let horizons = DecayAnalysis::long_term_horizons();
        assert_eq!(horizons, vec![21, 42, 63, 126, 252]);
    }
}
