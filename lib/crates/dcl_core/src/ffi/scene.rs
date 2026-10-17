//! FFI for the scene domain: the scene the player is in, screenshots, and analytics.

use std::{sync::Arc, time::Duration};

use super::{core, CoreError};
use crate::{
    analytics::{global::analytics, property::Property, service::Analytics},
    scene::{
        capture_screenshot as capture, current_scene as derive_current, scene_tracker,
        screenshot_provider, CurrentScene, ReqwestPlaceSource,
    },
};

fn service() -> Result<&'static Arc<Analytics>, CoreError> {
    analytics().ok_or_else(|| CoreError::InvalidCall {
        detail: "analytics is not installed".to_string(),
    })
}

fn joined<T>(result: Result<T, tokio::task::JoinError>) -> Result<T, CoreError> {
    result.map_err(|e| CoreError::InvalidCall {
        detail: e.to_string(),
    })
}

/// Title, creator, parcel and realm of the scene the player is in. The creator comes from the
/// destinations API and stays empty when the lookup fails.
#[uniffi::export]
pub async fn current_scene() -> Result<CurrentScene, CoreError> {
    let facts = scene_tracker().facts();
    joined(
        core()?
            .handle()
            .spawn(async move { derive_current(&facts, &ReqwestPlaceSource::new()).await })
            .await,
    )
}

/// Captures the viewport and returns the absolute path of the PNG.
#[uniffi::export]
pub async fn capture_screenshot() -> Result<String, CoreError> {
    let provider = screenshot_provider();
    let path = joined(
        core()?
            .handle()
            .spawn(async move { capture(provider.as_deref()).await })
            .await,
    )?
    .map_err(|e| CoreError::InvalidCall {
        detail: e.to_string(),
    })?;
    Ok(path.to_string_lossy().into_owned())
}

/// Queues an analytics event; it ships once the EULA is accepted.
#[uniffi::export]
pub fn track(event: String, props: Vec<Property>) -> Result<(), CoreError> {
    service()?.track(&event, &props);
    Ok(())
}

/// Queues a "Screen Viewed" for `name`.
#[uniffi::export]
pub fn track_screen(name: String, props: Vec<Property>) -> Result<(), CoreError> {
    service()?.track_screen(&name, &props);
    Ok(())
}

/// Opens (or closes) the consent gate; opening ships what waited.
#[uniffi::export]
pub async fn set_analytics_consent(accepted: bool) -> Result<(), CoreError> {
    let service = service()?.clone();
    joined(
        core()?
            .handle()
            .spawn(async move { service.set_eula_accepted(accepted).await })
            .await,
    )
}

/// Ships everything pending now; the shell calls it when the app goes to the background.
#[uniffi::export]
pub async fn flush_analytics() -> Result<(), CoreError> {
    let service = service()?.clone();
    joined(
        core()?
            .handle()
            .spawn(async move { service.flush().await })
            .await,
    )
}

/// Seconds between periodic flushes: short while onboarding, longer in the world.
#[uniffi::export]
pub fn set_analytics_flush_interval(seconds: f64) -> Result<(), CoreError> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(CoreError::InvalidCall {
            detail: "the flush interval must be positive".to_string(),
        });
    }
    service()?.set_flush_interval(Duration::from_secs_f64(seconds));
    Ok(())
}
