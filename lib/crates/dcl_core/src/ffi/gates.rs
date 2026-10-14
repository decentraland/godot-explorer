//! FFI for the boot gates, the URLs and the destination rules.

use std::sync::{Arc, OnceLock};

use super::{core, CoreError};
use crate::gates::{
    campaign::CampaignResolution,
    destination::ResolvedDestination,
    device::{DeviceFacts, DeviceSupport, SupportModal},
    fetch::ReqwestFetch,
    state::{StateStore, FILE},
    urls::{self, Environment, Urls},
    version::VersionGate,
    Gates,
};

/// The running build: the workspace version every artifact of this release is stamped with.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn gates() -> Result<&'static Gates, CoreError> {
    static GATES: OnceLock<Gates> = OnceLock::new();
    let core = core()?;
    Ok(GATES.get_or_init(|| {
        Gates::new(
            Arc::new(ReqwestFetch::new()),
            StateStore::open(core.data_dir().join(FILE)),
            core.config().platform,
            APP_VERSION,
        )
    }))
}

fn io_error(e: std::io::Error) -> CoreError {
    CoreError::InvalidCall {
        detail: format!("cannot save the gates state: {e}"),
    }
}

/// Runs `future` on the core runtime, so reqwest and timers have a tokio context whatever
/// executor the foreign side polls from.
async fn on_core<T, F>(future: F) -> Result<T, CoreError>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    core()?
        .handle()
        .spawn(future)
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })
}

/// Every URL the shell needs, for the current environment.
#[uniffi::export]
pub fn urls() -> Urls {
    urls::build()
}

#[uniffi::export]
pub fn environment() -> Environment {
    urls::environment()
}

/// The catalog page of a backpack section (`"wearables"` or `"emotes"`).
#[uniffi::export]
pub fn marketplace_browse_url(section: String) -> String {
    urls::marketplace_browse(&section)
}

/// The detail page of a catalog item.
#[uniffi::export]
pub fn marketplace_item_url(contract_address: String, item_id: String) -> String {
    urls::marketplace_item(&contract_address, &item_id)
}

/// Web link that opens the explorer in a world.
#[uniffi::export]
pub fn jump_realm_url(realm: String) -> String {
    urls::jump_realm_link(&realm)
}

/// Web link that opens the explorer at a Genesis City position, `"x,y"`.
#[uniffi::export]
pub fn jump_position_url(position: String) -> String {
    urls::jump_position_link(&position)
}

#[uniffi::export]
pub fn event_url(event_id: String) -> String {
    urls::event_link(&event_id)
}

/// Shareable link to a scene: a position in Genesis City, the world's name elsewhere.
#[uniffi::export]
pub fn scene_link_url(realm: String, x: i32, y: i32) -> String {
    urls::scene_link(&realm, x, y)
}

/// The update gate. Fail-open: any failure to ask lets the user in.
#[uniffi::export]
pub async fn version_gate() -> Result<VersionGate, CoreError> {
    let gates = gates()?;
    on_core(async move { gates.version_gate().await }).await
}

/// "Later" on an optional update: silent for a day.
#[uniffi::export]
pub fn snooze_version_gate() -> Result<(), CoreError> {
    gates()?.snooze_version_gate().map_err(io_error)
}

/// Whether the device is end-of-support or below the minimum spec, and which modal is due.
#[uniffi::export]
pub async fn device_support(facts: DeviceFacts) -> Result<DeviceSupport, CoreError> {
    let gates = gates()?;
    on_core(async move { gates.device_support(&facts).await })
        .await?
        .map_err(io_error)
}

/// The host showed `modal`; the cadence moves on.
#[uniffi::export]
pub fn device_support_modal_shown(modal: SupportModal) -> Result<(), CoreError> {
    gates()?.device_support_modal_shown(modal).map_err(io_error)
}

/// The campaign that should drive this launch, `None` for the default first-run flow.
#[uniffi::export]
pub async fn pending_campaign() -> Result<Option<crate::gates::campaign::Campaign>, CoreError> {
    Ok(campaign_resolution().await?.campaign)
}

/// `pending_campaign` plus the token and the reason it fell back, for the first-run metrics.
#[uniffi::export]
pub async fn campaign_resolution() -> Result<CampaignResolution, CoreError> {
    let gates = gates()?;
    on_core(async move { gates.campaign_resolution().await }).await
}

/// Remembers the token of a link or of the install attribution; the first one wins. `true` when
/// it was stored.
#[uniffi::export]
pub fn capture_campaign_token(token: String) -> Result<bool, CoreError> {
    gates()?.capture_campaign_token(&token).map_err(io_error)
}

/// The launch booted into the campaign's target: one campaign per install.
#[uniffi::export]
pub fn consume_campaign() -> Result<(), CoreError> {
    gates()?.consume_campaign().map_err(io_error)
}

fn signed_in_address() -> Result<String, CoreError> {
    Ok(core()?.with_session(|s| {
        s.try_address()
            .map(|a| format!("{a:#x}"))
            .unwrap_or_default()
    }))
}

/// Answers "is this place real, and may I enter it" for a coordinate (`"x,y"`), a `.dcl.eth`
/// name, a realm URL, optionally followed by `x,y`, or a jump link naming one.
#[uniffi::export]
pub async fn resolve_destination(input: String) -> Result<ResolvedDestination, CoreError> {
    resolve(input, None).await
}

/// The same, for a world that asked for a password.
#[uniffi::export]
pub async fn resolve_destination_with_secret(
    input: String,
    secret: String,
) -> Result<ResolvedDestination, CoreError> {
    resolve(input, Some(secret)).await
}

async fn resolve(input: String, secret: Option<String>) -> Result<ResolvedDestination, CoreError> {
    let gates = gates()?;
    let address = signed_in_address()?;
    on_core(async move {
        gates
            .resolve_destination(&input, secret.as_deref(), &address)
            .await
    })
    .await
}

/// The terms version this build asks players to accept.
#[uniffi::export]
pub fn terms_version() -> u32 {
    crate::gates::TERMS_VERSION
}

/// The terms version the player accepted; 0 until they do.
#[uniffi::export]
pub fn terms_accepted_version() -> Result<u32, CoreError> {
    Ok(gates()?.terms_accepted_version())
}

#[uniffi::export]
pub fn accept_terms(version: u32) -> Result<(), CoreError> {
    gates()?.accept_terms(version).map_err(io_error)
}
