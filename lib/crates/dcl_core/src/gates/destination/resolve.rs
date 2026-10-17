//! The resolution itself: classify the input, ask the realm, read the verdicts.
//!
//! The Godot client reused the realm it was standing in to save requests; the core holds no
//! realm, so every resolve costs its own `/about`.

use std::time::Duration;

use serde_json::{json, Value};

use super::{
    input::{parse_input, Intent},
    AccessVerdict, DestinationKind, ResolveFailure, ResolvedDestination,
};
use crate::{
    app::{Destination, Parcel},
    destination::{self as rules, SceneInfo},
    gates::fetch::{Fetch, SignedFetchError},
    realm, urls,
};

/// The whole resolve, not per request: three sequential calls each under their own timeout is
/// how the hang this replaces stayed alive.
pub const DEADLINE: Duration = Duration::from_secs(8);

/// A preview server on the LAN answers fast or is not reachable at all (#2684).
pub const PREVIEW_DEADLINE: Duration = Duration::from_secs(3);

const ACCESS_SHARED_SECRET: &str = "shared-secret";
const ACCESS_ALLOW_LIST: &str = "allow-list";

/// The verdict plus what the resolve already read, so the engine commits the realm without
/// asking the same servers again.
#[derive(Clone, Debug)]
pub struct Resolution {
    pub resolved: ResolvedDestination,
    /// The realm's `/about`, when it was reached.
    pub about: Option<Value>,
    /// A world's `/scenes` listing, when it was read.
    pub listing: Option<Value>,
}

impl Resolution {
    fn of(resolved: ResolvedDestination) -> Self {
        Self {
            resolved,
            about: None,
            listing: None,
        }
    }

    /// A resolve that never produced an answer.
    pub fn timed_out() -> Self {
        Self::of(invalid_with(ResolveFailure::Timeout))
    }

    fn with_about(mut self, about: &Value) -> Self {
        self.about = Some(about.clone());
        self
    }

    fn with_listing(mut self, listing: &Value) -> Self {
        self.listing = Some(listing.clone());
        self
    }
}

/// Where the intent lands.
#[derive(Clone, Debug)]
struct Target {
    kind: DestinationKind,
    realm: String,
    parcel: Option<Parcel>,
    /// The `.dcl.eth` name, empty unless `kind` is `World`.
    world_name: String,
}

/// The bare `<name>.dcl.eth` when the realm entered is a world, empty otherwise. Detection is on
/// the resolved URL: every world resolves to `<worlds-content-server>/<name>`, and the resume
/// path stores the full URL, not the name.
pub fn world_name_from_realm(realm_string: &str) -> String {
    let resolved = realm::resolve_realm_url(realm_string);
    let worlds_base = urls::worlds_content_server();
    if let Some(rest) = resolved.strip_prefix(&worlds_base) {
        return percent_decode(realm::ensure_remove_slash(rest).as_str()).to_lowercase();
    }
    if realm::is_dcl_ens(realm_string) {
        return realm_string.to_lowercase();
    }
    String::new()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn classify(intent: &Intent) -> Option<Target> {
    let realm = intent.realm.as_str();
    if realm.is_empty() {
        // Bare coordinates are Genesis City's addressing scheme.
        intent.parcel?;
        return Some(Target {
            kind: DestinationKind::Genesis,
            realm: urls::main_realm(),
            parcel: intent.parcel,
            world_name: String::new(),
        });
    }
    let (kind, world_name) = if realm::is_local_preview(realm) {
        (DestinationKind::Preview, String::new())
    } else {
        let world = world_name_from_realm(realm);
        if !world.is_empty() {
            (DestinationKind::World, world)
        } else if realm::is_genesis_city(realm) {
            (DestinationKind::Genesis, String::new())
        } else {
            (DestinationKind::CustomRealm, String::new())
        }
    };
    Some(Target {
        kind,
        realm: realm.to_string(),
        parcel: intent.parcel,
        world_name,
    })
}

/// What `enter_world` takes: the default realm is `None`, a world is its name.
fn destination_of(target: &Target) -> Destination {
    let realm = match target.kind {
        DestinationKind::World => Some(target.world_name.clone()),
        DestinationKind::Genesis
            if realm::normalize_realm_url(&target.realm)
                == realm::normalize_realm_url(&urls::main_realm()) =>
        {
            None
        }
        _ => Some(target.realm.clone()),
    };
    Destination {
        realm,
        parcel: target.parcel,
    }
}

fn answer(
    target: &Target,
    access: AccessVerdict,
    failure: Option<ResolveFailure>,
    scene: Option<&SceneInfo>,
) -> ResolvedDestination {
    let empty = SceneInfo {
        id: String::new(),
        title: String::new(),
        creator: String::new(),
        image_url: String::new(),
        asset_count: 0,
    };
    let scene = scene.unwrap_or(&empty);
    ResolvedDestination {
        kind: target.kind,
        access,
        failure,
        ready: failure.is_none() && access == AccessVerdict::Allowed,
        world_name: target.world_name.clone(),
        parcel: target.parcel,
        scene_id: scene.id.clone(),
        title: scene.title.clone(),
        creator: scene.creator.clone(),
        image_url: scene.image_url.clone(),
        asset_count: scene.asset_count as u32,
        destination: destination_of(target),
    }
}

fn ready(target: &Target, scene: Option<&SceneInfo>) -> ResolvedDestination {
    answer(target, AccessVerdict::Allowed, None, scene)
}

fn failed(target: &Target, reason: ResolveFailure) -> ResolvedDestination {
    answer(target, AccessVerdict::Allowed, Some(reason), None)
}

fn refused(target: &Target, access: AccessVerdict) -> ResolvedDestination {
    answer(target, access, None, None)
}

/// A parcel outside the advertised bounds, or inside them with no scene.
pub fn genesis_failure(
    parcel: Parcel,
    min: (i32, i32),
    max: (i32, i32),
    entity_id: &str,
) -> Option<ResolveFailure> {
    if parcel.x < min.0 || parcel.x > max.0 || parcel.y < min.1 || parcel.y > max.1 {
        return Some(ResolveFailure::ParcelOutOfBounds);
    }
    entity_id.is_empty().then_some(ResolveFailure::ParcelEmpty)
}

/// Whether a world lets this address in. `allow-list` and `nft-ownership` fail open on purpose:
/// the first was decided in #1725 and the client cannot resolve ownership for the second.
pub fn access_of(permissions: &Value, address: &str) -> AccessVerdict {
    let Some(access) = permissions
        .pointer("/permissions/access")
        .filter(|a| a.is_object())
    else {
        return AccessVerdict::Allowed;
    };
    match access
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        ACCESS_SHARED_SECRET => AccessVerdict::NeedsPassword,
        ACCESS_ALLOW_LIST if !is_on_allow_list(permissions, access, address) => {
            AccessVerdict::NotAllowed
        }
        _ => AccessVerdict::Allowed,
    }
}

/// Denies only a plain wallet allow-list the address is absent from. Community allow-lists are let
/// through: resolving membership needs the social service, and letting a non-member in is far
/// cheaper than locking out a legitimate one.
fn is_on_allow_list(permissions: &Value, access: &Value, address: &str) -> bool {
    let address = address.to_lowercase();
    if address.is_empty() {
        return true;
    }
    let lower = |value: &Value| value.as_str().map(str::to_lowercase);
    let listed = access
        .get("wallets")
        .and_then(Value::as_array)
        .is_some_and(|wallets| wallets.iter().filter_map(lower).any(|w| w == address));
    let owner = permissions.get("owner").and_then(lower) == Some(address);
    let communities = access
        .get("communities")
        .and_then(Value::as_array)
        .is_some_and(|c| !c.is_empty());
    listed || owner || communities
}

struct Resolver<'a> {
    fetch: &'a dyn Fetch,
    /// The signed-in address; empty when signed out.
    address: &'a str,
    secret: Option<&'a str>,
}

/// Resolves `input` within its deadline.
pub async fn resolve(
    fetch: &dyn Fetch,
    address: &str,
    input: &str,
    secret: Option<&str>,
) -> ResolvedDestination {
    let Some(intent) = parse_input(input) else {
        return invalid();
    };
    resolve_intent(fetch, address, &intent, secret)
        .await
        .resolved
}

fn invalid() -> ResolvedDestination {
    invalid_with(ResolveFailure::InvalidInput)
}

fn invalid_with(failure: ResolveFailure) -> ResolvedDestination {
    let target = Target {
        kind: DestinationKind::CustomRealm,
        realm: String::new(),
        parcel: None,
        world_name: String::new(),
    };
    failed(&target, failure)
}

/// Resolves an already-parsed intent within its deadline, keeping what it read.
pub async fn resolve_intent(
    fetch: &dyn Fetch,
    address: &str,
    intent: &Intent,
    secret: Option<&str>,
) -> Resolution {
    let Some(target) = classify(intent) else {
        return Resolution::of(invalid());
    };
    let budget = if target.kind == DestinationKind::Preview {
        PREVIEW_DEADLINE
    } else {
        DEADLINE
    };
    let resolver = Resolver {
        fetch,
        address,
        secret,
    };
    match tokio::time::timeout(budget, resolver.run(&target)).await {
        Ok(resolution) => resolution,
        Err(_) => Resolution::of(failed(&target, ResolveFailure::Timeout)),
    }
}

impl Resolver<'_> {
    /// Two lookup strategies, not four kinds: a catalyst is asked for the scene at a pointer, a
    /// world and a preview name their scenes themselves.
    async fn run(&self, target: &Target) -> Resolution {
        match target.kind {
            DestinationKind::World => self.world(target).await,
            DestinationKind::Preview => self.preview(target).await,
            DestinationKind::Genesis | DestinationKind::CustomRealm => self.catalyst(target).await,
        }
    }

    async fn about(&self, target: &Target) -> Option<Value> {
        let url = format!("{}about", realm::normalize_realm_url(&target.realm));
        self.fetch
            .get_json(&url)
            .await
            .ok()
            .filter(Value::is_object)
    }

    /// The scene at `parcel`, `None` when the realm holds none. Always asks the network: a cache
    /// that answers "" for both "no scene" and "never asked" is how #2473 happened.
    async fn scene_at(&self, content_base_url: &str, parcel: Parcel) -> Option<SceneInfo> {
        let url = format!("{}/entities/active", content_base_url.trim_end_matches('/'));
        let body = json!({"pointers": [format!("{},{}", parcel.x, parcel.y)]}).to_string();
        let response = self.fetch.post_json(&url, body).await.ok()?;
        let entity = response.as_array()?.first().filter(|e| e.is_object())?;
        Some(rules::scene_info(entity, content_base_url))
    }

    async fn catalyst(&self, target: &Target) -> Resolution {
        let about = match self.about(target).await {
            Some(about) if realm::about_problem(&about).is_none() => about,
            _ => return Resolution::of(failed(target, ResolveFailure::FetchFailed)),
        };
        let Some(parcel) = target.parcel else {
            return Resolution::of(ready(target, None)).with_about(&about);
        };
        let content_url = realm::ensure_ends_with_slash(content_public_url(&about));
        let found = self.scene_at(&content_url, parcel).await;
        if target.kind == DestinationKind::Genesis {
            let bounds = realm::commit_from_about(&about, &[]);
            let id = found.as_ref().map_or("", |s| s.id.as_str());
            if let Some(reason) = genesis_failure(parcel, bounds.min_bounds, bounds.max_bounds, id)
            {
                return Resolution::of(failed(target, reason)).with_about(&about);
            }
        }
        Resolution::of(ready(target, found.as_ref())).with_about(&about)
    }

    async fn world(&self, target: &Target) -> Resolution {
        // No status code survives the rejection, so an /about that does not come back is
        // reported as not found: the dominant cause, and the copy is the same anyway.
        let Some(about) = self.about(target).await else {
            return Resolution::of(failed(target, ResolveFailure::WorldNotFound));
        };
        if realm::about_problem(&about).is_some() {
            return Resolution::of(failed(target, ResolveFailure::FetchFailed));
        }
        let name = &target.world_name;
        let base = realm::dcl_world_url(name);
        let permissions = self
            .fetch
            .get_json(&format!("{base}/permissions"))
            .await
            .unwrap_or_else(|_| json!({}));
        match access_of(&permissions, self.address) {
            AccessVerdict::NotAllowed => {
                return Resolution::of(refused(target, AccessVerdict::NotAllowed))
                    .with_about(&about)
            }
            AccessVerdict::NeedsPassword => {
                let Some(secret) = self.secret.filter(|s| !s.is_empty()) else {
                    return Resolution::of(refused(target, AccessVerdict::NeedsPassword))
                        .with_about(&about);
                };
                if let Some(answer) = self.verify_secret(target, &about, secret).await {
                    return answer;
                }
            }
            AccessVerdict::Allowed => {}
        }

        let listing = self
            .fetch
            .get_json(&format!("{base}/scenes"))
            .await
            .unwrap_or_else(|_| json!({}));
        // /about points content at the Genesis peer, not where a world's files live.
        let worlds_base = urls::worlds_content_server().replace("/world/", "/");
        let mut entity = last_entity(&listing);
        if let Some(parcel) = target.parcel {
            let index = rules::world_parcel_index(&listing);
            match index.get(&format!("{},{}", parcel.x, parcel.y)) {
                Some(at_parcel) => entity = at_parcel.clone(),
                // Or the player walks into empty space inside a world that loaded fine. An
                // unreadable listing lets them through rather than blocking entry.
                None if !index.is_empty() => {
                    return Resolution::of(failed(target, ResolveFailure::ParcelEmpty))
                        .with_about(&about)
                        .with_listing(&listing)
                }
                None => {}
            }
        }
        Resolution::of(ready(
            target,
            Some(&rules::scene_info(&entity, &worlds_base)),
        ))
        .with_about(&about)
        .with_listing(&listing)
    }

    /// The comms handshake is the only place a secret can be checked: `/permissions` strips it
    /// and it is hashed anyway. `None` when it checked out.
    async fn verify_secret(
        &self,
        target: &Target,
        about: &Value,
        secret: &str,
    ) -> Option<Resolution> {
        let url = rules::comms_handshake_url(about);
        // Nowhere to prove the secret is not the same answer as proven: letting it through would
        // enter on any password and fail later, at the real connect.
        if url.is_empty() {
            return Some(
                Resolution::of(failed(target, ResolveFailure::FetchFailed)).with_about(about),
            );
        }
        let metadata =
            rules::handshake_metadata(&realm::normalize_realm_url(&target.realm), secret);
        match self.fetch.signed_post(&url, &metadata).await {
            Ok(()) => None,
            Err(SignedFetchError::NoIdentity) => {
                Some(Resolution::of(failed(target, ResolveFailure::FetchFailed)).with_about(about))
            }
            Err(SignedFetchError::Rejected(message)) if rules::is_rate_limited(&message) => {
                Some(Resolution::of(failed(target, ResolveFailure::RateLimited)).with_about(about))
            }
            Err(SignedFetchError::Rejected(_)) => Some(
                Resolution::of(refused(target, AccessVerdict::NeedsPassword)).with_about(about),
            ),
        }
    }

    async fn preview(&self, target: &Target) -> Resolution {
        let Some(about) = self.about(target).await else {
            return Resolution::of(failed(target, ResolveFailure::PreviewUnreachable));
        };
        // A local preview serves its own content, so the pointer lookup works on it like any
        // catalyst. Its /about lists `localSceneParcels` and leaves `scenesUrn` empty, which is
        // why the urn read below is the fallback and not the other way round.
        if let Some(parcel) = target.parcel {
            if realm::about_problem(&about).is_none() {
                let content_url = realm::ensure_ends_with_slash(content_public_url(&about));
                if let Some(found) = self.scene_at(&content_url, parcel).await {
                    return Resolution::of(ready(target, Some(&found))).with_about(&about);
                }
            }
        }
        let scene = self.urn_scene(&about).await;
        Resolution::of(ready(target, scene.as_ref())).with_about(&about)
    }

    /// For a realm that advertises scene urns instead of serving pointers. One that will not
    /// describe itself still loads: ready without metadata is normal.
    async fn urn_scene(&self, about: &Value) -> Option<SceneInfo> {
        let urn = about
            .pointer("/configurations/scenesUrn")?
            .as_array()?
            .first()?
            .as_str()?;
        let parsed = realm::parse_urn(urn).filter(|p| !p.base_url.is_empty())?;
        let entity = self
            .fetch
            .get_json(&format!("{}{}", parsed.base_url, parsed.entity_id))
            .await
            .ok()
            .filter(Value::is_object)?;
        // A urn's baseUrl already addresses content files.
        Some(rules::scene_info_with_files(&entity, &parsed.base_url))
    }
}

fn content_public_url(about: &Value) -> &str {
    about
        .pointer("/content/publicUrl")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// Where a world join lands: the last entity of the listing.
fn last_entity(listing: &Value) -> Value {
    listing
        .get("scenes")
        .and_then(Value::as_array)
        .and_then(|scenes| scenes.last())
        .and_then(|scene| scene.get("entity"))
        .filter(|entity| entity.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}))
}
