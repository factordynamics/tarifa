//! Signal creation utilities for the Tarifa CLI.

use tarifa_signals::{
    momentum::{LongTermMomentum, MediumTermMomentum, ShortTermMomentum},
    registry::get_signal_info,
};
use tarifa_traits::{Signal, TarifaError};

/// Create a signal instance by name.
///
/// Returns a boxed Signal trait object for the given signal name.
pub(crate) fn create_signal(name: &str) -> Result<Box<dyn Signal>, TarifaError> {
    match name {
        // Momentum signals
        "short_term_momentum" | "momentum_1m" | "mom_1m" => {
            Ok(Box::new(ShortTermMomentum::default()))
        }
        "medium_term_momentum" | "momentum_6m" | "mom_6m" => {
            Ok(Box::new(MediumTermMomentum::default()))
        }
        "long_term_momentum" | "momentum_12m" | "mom_12m" => {
            Ok(Box::new(LongTermMomentum::default()))
        }
        // Value signals require fundamental data - not yet supported via Yahoo
        "book_to_price" | "earnings_yield" | "fcf_yield" => Err(TarifaError::InvalidData(format!(
            "Signal '{}' requires fundamental data which is not yet available via Yahoo Finance",
            name
        ))),
        // Quality signals require fundamental data - not yet supported via Yahoo
        "return_on_equity" | "roe" | "return_on_assets" | "roa" | "profit_margins" | "margins" => {
            Err(TarifaError::InvalidData(format!(
                "Signal '{}' requires fundamental data which is not yet available via Yahoo Finance",
                name
            )))
        }
        _ => Err(TarifaError::SignalNotFound(format!(
            "Unknown signal: '{}'. Use 'tarifa signals' to list available signals.",
            name
        ))),
    }
}

/// Get the lookback period required for a signal.
pub(crate) fn get_lookback(name: &str) -> usize {
    get_signal_info(name)
        .map(|info| info.typical_lookback)
        .unwrap_or(252) // Default to 1 year
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_momentum_signals() {
        assert!(create_signal("short_term_momentum").is_ok());
        assert!(create_signal("medium_term_momentum").is_ok());
        assert!(create_signal("long_term_momentum").is_ok());

        // Aliases
        assert!(create_signal("momentum_1m").is_ok());
        assert!(create_signal("mom_6m").is_ok());
        assert!(create_signal("mom_12m").is_ok());
    }

    #[test]
    fn test_unknown_signal() {
        let result = create_signal("nonexistent_signal");
        assert!(matches!(result, Err(TarifaError::SignalNotFound(_))));
    }

    #[test]
    fn test_fundamental_signal_error() {
        let result = create_signal("book_to_price");
        assert!(matches!(result, Err(TarifaError::InvalidData(_))));
    }
}
