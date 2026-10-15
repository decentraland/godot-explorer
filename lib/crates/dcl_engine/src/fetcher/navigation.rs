//! Executes a navigation intent (#2948). Resolve first, act second: nothing is offloaded until
//! the destination is known to be real, which turns a black screen over a discarded scene into
//! a modal over the one the player is still standing in.

use super::globals;
use std::{collections::HashMap, sync::Mutex};

use dcl_core::{
    gates::{
        destination::{
            resolve_intent, AccessVerdict, DestinationKind, Intent, Resolution, ResolveFailure,
        },
        fetch::ReqwestFetch,
    },
    realm, urls, Core,
};
use godot::classes::Time;
use godot::prelude::*;

use super::{
    loading_session::LoadingSession,
    realm_flow::{self, Landing},
    scene_fetcher::explorer,
    world_access,
};
use crate::{engine::local_task::on_core, godot_classes::dcl_global::DclGlobal};

/// Passwords the shell collected for protected worlds, by normalized realm url.
static WORLD_SECRETS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

pub fn set_world_secret(realm_string: &str, secret: &str) {
    WORLD_SECRETS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(realm::normalize_realm_url(realm_string), secret.to_string());
}

fn world_secret(realm_string: &str) -> Option<String> {
    WORLD_SECRETS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()?
        .get(&realm::normalize_realm_url(realm_string))
        .cloned()
}

/// Where the player asked to go.
#[derive(Clone, Debug)]
pub struct Navigation {
    /// The realm to enter, never empty.
    pub realm: String,
    pub parcel: Option<(i32, i32)>,
    /// An intent is gated: an unreal destination is refused before the current scene is
    /// offloaded. A restoration is not: there is no scene to protect, so an empty parcel lands
    /// on grass.
    pub is_intent: bool,
    /// A coordinate given with no realm is retried against Genesis City before being refused.
    pub fallback_to_genesis: bool,
}

impl Navigation {
    /// The one adapter for untrusted realm strings (deeplinks, chat commands, teleportTo): a
    /// creator can say "go to 5,5" inside their world, but coordinates are Genesis City's
    /// addressing, so a world without that parcel hands the question on.
    pub fn from_input(realm: &str, parcel: Option<(i32, i32)>) -> Self {
        if realm.is_empty() {
            return Self {
                realm: current_realm(),
                parcel,
                is_intent: true,
                fallback_to_genesis: parcel.is_some(),
            };
        }
        Self {
            realm: realm.to_string(),
            parcel,
            is_intent: true,
            fallback_to_genesis: false,
        }
    }

    /// Resuming a session or a cold-start deeplink: not an intent, so the parcel gate is off,
    /// but /about and access still run and an unreachable world is still refused.
    pub fn restore(realm: &str, parcel: Option<(i32, i32)>) -> Self {
        Self {
            is_intent: false,
            ..Self::from_input(realm, parcel)
        }
    }

    /// /reload, scene-crash retry, loading-timeout retry. Not an intent: re-checking a place
    /// we are already inside would refuse to reload a parcel the player walked to.
    pub fn reload_current() -> Self {
        Self {
            is_intent: false,
            ..Self::from_input("", None)
        }
    }
}

/// The realm the player is in, or Genesis City before one is set.
fn current_realm() -> String {
    let realm = globals::realm();
    let current = realm.bind().get_realm_string().to_string();
    if current.is_empty() {
        urls::main_realm()
    } else {
        current
    }
}

enum Verdict {
    Ready(Box<Resolution>),
    NotAllowed,
    NeedsPassword,
    Failed(ResolveFailure),
}

fn failure_name(failure: ResolveFailure) -> &'static str {
    match failure {
        ResolveFailure::InvalidInput => "invalid_input",
        ResolveFailure::WorldNotFound => "world_not_found",
        ResolveFailure::ParcelOutOfBounds => "parcel_out_of_bounds",
        ResolveFailure::ParcelEmpty => "parcel_empty",
        ResolveFailure::FetchFailed => "fetch_failed",
        ResolveFailure::PreviewUnreachable => "preview_unreachable",
        ResolveFailure::Timeout => "timeout",
        ResolveFailure::RateLimited => "rate_limited",
    }
}

fn signed_in_address() -> String {
    Core::try_get()
        .and_then(|core| core.with_session(|s| s.try_address()))
        .map(|address| format!("{address:#x}"))
        .unwrap_or_default()
}

async fn resolve_once(
    realm: &str,
    parcel: Option<(i32, i32)>,
    secret: Option<String>,
) -> Resolution {
    let intent = Intent {
        realm: realm.to_string(),
        parcel: parcel.map(|(x, y)| dcl_core::app::Parcel::new(x, y)),
    };
    let address = signed_in_address();
    on_core(async move {
        resolve_intent(&ReqwestFetch::new(), &address, &intent, secret.as_deref()).await
    })
    .await
    .unwrap_or_else(Resolution::timed_out)
}

/// Retries a bare coordinate against Genesis City when the realm the player is in has no such
/// parcel. Sequential rather than raced: the common case pays nothing.
async fn resolve(navigation: &mut Navigation) -> Verdict {
    let secret = world_secret(&navigation.realm);
    let mut resolution = resolve_once(&navigation.realm, navigation.parcel, secret).await;
    if resolution.resolved.failure == Some(ResolveFailure::ParcelEmpty)
        && navigation.fallback_to_genesis
        && resolution.resolved.kind != DestinationKind::Genesis
    {
        navigation.realm = urls::main_realm();
        navigation.fallback_to_genesis = false;
        let secret = world_secret(&navigation.realm);
        resolution = resolve_once(&navigation.realm, navigation.parcel, secret).await;
    }
    let resolved = &resolution.resolved;
    match (resolved.failure, resolved.access) {
        // A restoration never refuses a parcel: it lands on grass.
        (Some(ResolveFailure::ParcelEmpty | ResolveFailure::ParcelOutOfBounds), _)
            if !navigation.is_intent && resolution.about.is_some() =>
        {
            Verdict::Ready(Box::new(resolution))
        }
        (Some(failure), _) => Verdict::Failed(failure),
        (None, AccessVerdict::Allowed) => Verdict::Ready(Box::new(resolution)),
        (None, AccessVerdict::NotAllowed) => Verdict::NotAllowed,
        (None, AccessVerdict::NeedsPassword) => Verdict::NeedsPassword,
    }
}

/// `when` is the loading-funnel bucket the dashboards index on. True only when the player
/// actually went somewhere.
pub async fn go(mut navigation: Navigation, when: &str) -> bool {
    let mut runner = globals::scene_runner();
    // On the intent, not the loading screen: a navigation refused before the screen goes up
    // still has to produce a funnel row.
    runner
        .bind_mut()
        .loading_begin_episode(when.into(), navigation.realm.as_str().into());
    let started = Time::singleton().get_ticks_msec();
    godot_print!("[NAV] intent {navigation:?} {when}");

    let verdict = resolve(&mut navigation).await;
    let elapsed = Time::singleton().get_ticks_msec() - started;
    match verdict {
        Verdict::Ready(resolution) => {
            log_verdict(
                "ready",
                &navigation,
                elapsed,
                &format!(
                    "assets={} title=\"{}\"",
                    resolution.resolved.asset_count, resolution.resolved.title
                ),
            );
            enter(&navigation, *resolution, when).await
        }
        Verdict::NotAllowed => {
            log_verdict("not_allowed", &navigation, elapsed, "");
            recover_from_refusal(&navigation.realm);
            report_notice("entry_private", &navigation.realm);
            abandon("private_world_access_denied")
        }
        Verdict::NeedsPassword => {
            log_verdict("needs_password", &navigation, elapsed, "");
            // The shell asks for the password and enters again.
            report_notice("entry_password", &navigation.realm);
            abandon("needs_password")
        }
        Verdict::Failed(failure) => {
            log_verdict("failed", &navigation, elapsed, failure_name(failure));
            runner.bind_mut().loading_realm_change_failed(
                navigation.realm.as_str().into(),
                failure_name(failure).into(),
            );
            recover_from_refusal(&navigation.realm);
            report_notice("entry_not_found", &navigation.realm);
            abandon("resolve_failed")
        }
    }
}

fn log_verdict(state: &str, navigation: &Navigation, elapsed_ms: u64, detail: &str) {
    godot_print!(
        "[NAV] {state} {} in {elapsed_ms}ms {detail}",
        navigation.realm
    );
}

fn report_notice(kind: &str, realm: &str) {
    globals::engine()
        .bind()
        .report_notice(kind.into(), realm.into());
}

fn abandon(reason: &str) -> bool {
    globals::scene_runner()
        .bind_mut()
        .loading_end_episode(reason.into());
    false
}

/// Undoes what a refusal would otherwise leave behind (#1725): a refused realm kept as the
/// boot realm reopens the same modal on every launch, and a boot that never committed a realm
/// leaves nothing behind the modal to dismiss back to.
pub fn recover_from_refusal(realm_string: &str) {
    // The shell decides where the player goes after a refusal.
    if globals::engine().bind().is_host_driven() {
        return;
    }
    // Still standing somewhere: the refusal cost nothing and the saved realm is still good.
    if globals::realm().bind().has_realm() {
        return;
    }
    // The main realm IS the fallback, so when it is what failed there is nowhere left to fall
    // back to: an offline boot has to end on the modal, not loop through it.
    if realm::normalize_realm_url(realm_string) == realm::normalize_realm_url(&urls::main_realm()) {
        return;
    }
    clear_boot_realm_if(realm_string);
    // Deferred: the modal is going up on this frame, and this re-enters `go`.
    if explorer().is_some() {
        crate::engine::local_task::spawn_local(async {
            go(
                Navigation::restore(&urls::main_realm(), None),
                "on_explorer_ready",
            )
            .await;
        });
    }
}

/// Compares canonical urls, not world names: a dead preview server is as capable of being the
/// saved boot realm as a world that revoked access.
fn clear_boot_realm_if(realm_string: &str) {
    if realm_string.is_empty() {
        return;
    }
    let engine = globals::engine();
    let stored = engine.bind().setting("last_realm_joined".into());
    let stored = stored
        .try_to::<GString>()
        .map(|s| s.to_string())
        .unwrap_or_default();
    if stored.is_empty()
        || realm::normalize_realm_url(&stored) != realm::normalize_realm_url(realm_string)
    {
        return;
    }
    engine.bind().set_setting(
        "last_realm_joined".into(),
        urls::main_realm().as_str().to_variant(),
        true,
    );
}

async fn enter(navigation: &Navigation, resolution: Resolution, when: &str) -> bool {
    let Some(mut explorer) = explorer() else {
        return abandon("no_explorer");
    };
    // The episode is already open; a second one would report this intent as superseded by its
    // own loading screen.
    if let Some(loading) = loading_of(&explorer) {
        loading
            .bind()
            .begin(navigation.realm.as_str().into(), when.into(), false);
    }
    warm_caches(&resolution);

    let realm_url = realm::normalize_realm_url(&navigation.realm);
    let needs_change =
        !navigation.is_intent || realm_url != globals::realm().bind().get_realm_url().to_string();
    if needs_change {
        let Some(about) = resolution.about else {
            end_loading(&mut explorer, "Failed");
            return false;
        };
        let landing = Landing {
            realm_string: navigation.realm.clone(),
            realm_url,
            about,
            listing: resolution.listing,
            credential: world_secret(&navigation.realm).unwrap_or_default(),
            wants_spawn: navigation.is_intent && navigation.parcel.is_none(),
        };
        let realm = globals::realm();
        if !realm_flow::apply(realm, landing).await {
            end_loading(&mut explorer, "Failed");
            return false;
        }
    }

    // A restoration carries its parcel only so the resolve can describe it: the explorer's
    // `_ready` already put the player there.
    if let (true, Some((x, y))) = (navigation.is_intent, navigation.parcel) {
        explorer.call("teleport_to", &[Vector2i::new(x, y).to_variant()]);
    }
    true
}

fn loading_of(explorer: &Gd<Node>) -> Option<Gd<LoadingSession>> {
    explorer.get("loading").try_to().ok()
}

fn end_loading(explorer: &mut Gd<Node>, status: &str) {
    if let Some(loading) = loading_of(explorer) {
        loading.bind().end(status.into());
    }
}

/// Starts the downloads the load will ask for anyway: after the verdict and never awaited, so
/// a refused destination spends no bytes. Not the thumbnail: the screen asks for the same hash
/// one frame later, so there is no head start here.
fn warm_caches(resolution: &Resolution) {
    let resolved = &resolution.resolved;
    if resolved.kind == DestinationKind::Genesis {
        warm_boot_bundle(&resolved.scene_id);
    }
}

/// main.js and main.crdt arrive together in one `{entity}-boot.zip`, and the scene loader
/// checks disk first, so fetching here turns that check into a hit a second later. A 404 is
/// the ordinary "not optimized" answer. Genesis only, and never in preview.
fn warm_boot_bundle(scene_id: &str) {
    if scene_id.is_empty() {
        return;
    }
    let global = DclGlobal::singleton();
    let preview_link = global
        .get("deep_link_obj")
        .try_to::<Gd<Object>>()
        .ok()
        .map(|link| !link.get("preview").to::<GString>().is_empty())
        .unwrap_or(false);
    let cli = globals::cli();
    let cli = cli.bind();
    if cli.preview_mode || preview_link || global.bind().testing_scene_mode || cli.only_no_optimized
    {
        return;
    }
    let mut provider = globals::content_provider();
    let base = provider.bind().get_optimized_base_url().to_string();
    let boot_zip = format!("{scene_id}-boot.zip");
    provider.bind_mut().fetch_boot_bundle(
        boot_zip.as_str().into(),
        format!("{base}/{boot_zip}").as_str().into(),
    );
}

/// Whether the private-world gate lets the player into `realm_string`. Checked before any
/// navigation UI is shown, so a world the user cannot enter blocks with the modal instantly
/// instead of first flashing a loading transition (#1725). Non-worlds resolve with no network.
/// False means access was denied and the modal is already up.
pub async fn precheck_realm_access(realm_string: &str) -> bool {
    let world = world_access::world_name_from_realm(realm_string);
    if world.is_empty() {
        return true;
    }
    if world_access::is_allowed(&world, &signed_in_address()).await {
        return true;
    }
    let mut realm_node = globals::realm();
    realm_node.emit_signal(
        "realm_access_denied",
        &[realm_string.to_variant(), world.to_variant()],
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_realm_is_an_intent_that_never_falls_back() {
        let nav = Navigation::from_input("arena.dcl.eth", Some((3, 4)));
        assert_eq!(nav.realm, "arena.dcl.eth");
        assert_eq!(nav.parcel, Some((3, 4)));
        assert!(nav.is_intent && !nav.fallback_to_genesis);
    }

    #[test]
    fn a_restoration_keeps_its_parcel_but_is_not_gated() {
        let nav = Navigation::restore("http://localhost:8000", Some((0, 0)));
        assert!(!nav.is_intent);
        assert_eq!(nav.parcel, Some((0, 0)));
    }

    #[test]
    fn world_secrets_are_keyed_by_the_canonical_realm_url() {
        set_world_secret("Arena.dcl.eth", "hunter2");
        assert_eq!(world_secret("arena.dcl.eth").as_deref(), Some("hunter2"));
        assert_eq!(world_secret("other.dcl.eth"), None);
    }

    #[test]
    fn failures_keep_the_funnel_vocabulary() {
        assert_eq!(
            failure_name(ResolveFailure::ParcelOutOfBounds),
            "parcel_out_of_bounds"
        );
        assert_eq!(
            failure_name(ResolveFailure::PreviewUnreachable),
            "preview_unreachable"
        );
    }
}
