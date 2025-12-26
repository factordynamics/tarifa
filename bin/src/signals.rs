//! Factor creation utilities for the Tarifa CLI.

use chrono::NaiveDate;
use factors::{
    Factor, FactorRegistry,
    momentum::{LongTermMomentum, MediumTermMomentum, ShortTermMomentum},
};
use polars::error::PolarsError;
use polars::prelude::*;
use tarifa_traits::{MarketData, TarifaError};

/// A wrapper around a Factor that provides a score method compatible with MarketData.
pub(crate) struct FactorWrapper {
    inner: Box<dyn Factor>,
}

impl FactorWrapper {
    /// Create a new FactorWrapper from a boxed Factor.
    pub(crate) fn new(factor: Box<dyn Factor>) -> Self {
        Self { inner: factor }
    }

    /// Get the factor name.
    pub(crate) fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the lookback period.
    pub(crate) fn lookback(&self) -> usize {
        self.inner.lookback()
    }

    /// Compute factor scores for the given market data and date.
    ///
    /// This wraps the Factor::compute method to work with MarketData.
    pub(crate) fn score(
        &self,
        data: &MarketData,
        date: NaiveDate,
    ) -> Result<DataFrame, TarifaError> {
        // Convert MarketData to LazyFrame
        let lazy = data.data().clone().lazy();

        // Compute factor scores using the Factor trait
        let result = self
            .inner
            .compute(&lazy, date)
            .map_err(|e| TarifaError::SignalComputation(e.to_string()))?;

        // The Factor::compute returns DataFrame with columns: symbol, date, <factor_name>
        // We need to rename the factor column to "score" for compatibility
        let factor_name = self.inner.name();
        let result = result
            .lazy()
            .rename([factor_name], ["score"], true)
            .collect()
            .map_err(|e: PolarsError| TarifaError::Polars(e))?;

        Ok(result)
    }
}

impl std::fmt::Debug for FactorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FactorWrapper")
            .field("name", &self.name())
            .field("lookback", &self.lookback())
            .finish()
    }
}

/// Create a factor instance by name.
///
/// Returns a wrapped Factor that provides a score method compatible with MarketData.
pub(crate) fn create_factor(name: &str) -> Result<FactorWrapper, TarifaError> {
    let factor: Box<dyn Factor> = match name {
        // Momentum factors
        "short_term_momentum" | "momentum_1m" | "mom_1m" => Box::new(ShortTermMomentum::default()),
        "medium_term_momentum" | "momentum_6m" | "mom_6m" => {
            Box::new(MediumTermMomentum::default())
        }
        "long_term_momentum" | "momentum_12m" | "mom_12m" => Box::new(LongTermMomentum::default()),
        // Value factors require fundamental data - not yet supported via Yahoo
        "book_to_price" | "earnings_yield" | "fcf_yield" => {
            return Err(TarifaError::InvalidData(format!(
                "Factor '{}' requires fundamental data which is not yet available via Yahoo Finance",
                name
            )));
        }
        // Quality factors require fundamental data - not yet supported via Yahoo
        "return_on_equity" | "roe" | "return_on_assets" | "roa" | "profit_margins" | "margins" => {
            return Err(TarifaError::InvalidData(format!(
                "Factor '{}' requires fundamental data which is not yet available via Yahoo Finance",
                name
            )));
        }
        _ => {
            return Err(TarifaError::SignalNotFound(format!(
                "Unknown factor: '{}'. Use 'tarifa signals' to list available factors.",
                name
            )));
        }
    };

    Ok(FactorWrapper::new(factor))
}

/// Get the lookback period required for a factor.
pub(crate) fn get_lookback(name: &str) -> usize {
    let registry = FactorRegistry::with_defaults();
    registry
        .get(name)
        .map(|factor| factor.lookback())
        .unwrap_or(252) // Default to 1 year
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_momentum_factors() {
        assert!(create_factor("short_term_momentum").is_ok());
        assert!(create_factor("medium_term_momentum").is_ok());
        assert!(create_factor("long_term_momentum").is_ok());

        // Aliases
        assert!(create_factor("momentum_1m").is_ok());
        assert!(create_factor("mom_6m").is_ok());
        assert!(create_factor("mom_12m").is_ok());
    }

    #[test]
    fn test_unknown_factor() {
        let result = create_factor("nonexistent_factor");
        assert!(matches!(result, Err(TarifaError::SignalNotFound(_))));
    }

    #[test]
    fn test_fundamental_factor_error() {
        let result = create_factor("book_to_price");
        assert!(matches!(result, Err(TarifaError::InvalidData(_))));
    }
}
