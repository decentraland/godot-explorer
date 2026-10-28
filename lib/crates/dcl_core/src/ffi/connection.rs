//! FFI for the connection quality: the shell polls on the interval the last report asked for and
//! shows what it says.

use std::time::Duration;

use http::Method;

use super::{core, CoreError};
use crate::{
    analytics::session::monotonic_ms,
    app::State,
    connection::{self, Report, Sample, REQUEST_TIMEOUT_SECS},
    profile::{HttpRequest, ReqwestTransport, Transport},
    urls,
};

/// Where reachability is asked: the realm of the world the player is in, else the catalyst.
fn health_url(state: &State) -> String {
    match state {
        State::InWorld { realm, .. } if realm.starts_with("http") => {
            format!("{}/about", realm.trim_end_matches('/'))
        }
        _ => format!("{}/about", urls::peer_base().trim_end_matches('/')),
    }
}

/// One HEAD to the health url, scored. `in_world` follows the state the core is in.
#[uniffi::export]
pub async fn probe_connection() -> Result<Report, CoreError> {
    let core = core()?;
    let state = core.with_app(|app| app.state().clone());
    let in_world = matches!(state, State::InWorld { .. });
    let url = health_url(&state);
    let started = std::time::Instant::now();
    let request = HttpRequest {
        method: Method::HEAD,
        url,
        headers: Vec::new(),
        body: Vec::new(),
    };
    let sample = core
        .handle()
        .spawn(async move {
            match tokio::time::timeout(
                Duration::from_secs_f64(REQUEST_TIMEOUT_SECS),
                ReqwestTransport::new().send(request),
            )
            .await
            {
                Err(_) => Sample::TimedOut,
                // Any HTTP status is a reply: the network is there.
                Ok(Ok(_)) => Sample::Ok {
                    latency_secs: started.elapsed().as_secs_f64(),
                },
                Ok(Err(_)) => Sample::Failed,
            }
        })
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?;
    Ok(connection::global().observe(sample, monotonic_ms() as f64 / 1000.0, in_world))
}

/// The player pressed retry on the connection-lost dialog: it stays away for a growing window.
#[uniffi::export]
pub fn connection_retry() {
    connection::global().retry(monotonic_ms() as f64 / 1000.0);
}

/// A world change settles: the score starts clean.
#[uniffi::export]
pub fn connection_reset() {
    connection::global().reset();
}

/// Whether the connection is good enough that nothing is in an error state.
#[uniffi::export]
pub fn connection_healthy() -> bool {
    connection::global().is_good()
}
