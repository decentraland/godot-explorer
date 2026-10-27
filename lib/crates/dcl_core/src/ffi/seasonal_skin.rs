//! FFI for the seasonal Discover reskin (`crate::seasonal_skin`).

use crate::seasonal_skin::{self, SeasonalSkin};

/// The active season, `None` when off or while the remote flags are not loaded.
#[uniffi::export]
pub fn seasonal_skin() -> Option<SeasonalSkin> {
    seasonal_skin::active()
}

/// Spend one try at fetching the season's assets; `false` once the cap is reached.
#[uniffi::export]
pub fn seasonal_skin_take_fetch_attempt() -> bool {
    seasonal_skin::take_fetch_attempt()
}
