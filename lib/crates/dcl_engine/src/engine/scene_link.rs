//! Engine side of the core's scene and profile contracts: where the player is, which catalyst
//! serves profiles, and the profile the engine holds. The sources are GDScript nodes with no
//! Rust hook, so the engine reads them a few times a second and reports what changed.

use std::time::{Duration, Instant};

use dcl_core::app::Parcel;
use dcl_core::profile::{self, Endpoints, ProfileDocument};
use dcl_core::scene::{update_scene, SceneFacts};
use godot::prelude::*;

use super::settings_link::on_process_frame;
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::fetcher::scene_fetcher::SceneFetcher;
use crate::godot_classes::dcl_global::DclGlobal;

const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// `SceneFetcher.INVALID_PARCEL`: no position yet.
const NO_PARCEL: Vector2i = Vector2i::new(-1000, -1000);

/// Registers the realm, scene and profile hooks. Called once when the engine starts.
pub(super) fn register() {
    forward_profile_changes();

    let mut last_poll: Option<Instant> = None;
    let mut reported_facts: Option<SceneFacts> = None;
    let mut reported_endpoints: Option<Endpoints> = None;
    on_process_frame("report_scene_facts", move || {
        if last_poll.is_some_and(|at| at.elapsed() < POLL_INTERVAL) {
            return;
        }
        last_poll = Some(Instant::now());
        let Some(global) = DclGlobal::try_singleton() else {
            return;
        };
        let facts = read_facts(&global);
        if reported_facts.as_ref() != Some(&facts) {
            update_scene(facts.clone());
            reported_facts = Some(facts);
        }
        let endpoints = read_endpoints(&global);
        if reported_endpoints.as_ref() != Some(&endpoints) {
            profile::global().set_endpoints(endpoints.clone());
            reported_endpoints = Some(endpoints);
        }
    });
}

fn read_facts(global: &Gd<DclGlobal>) -> SceneFacts {
    let realm = global.bind().realm.clone();
    let (realm_url, realm_name) = {
        let realm = realm.bind();
        (
            realm.get_realm_url().to_string(),
            realm.get_realm_name().to_string(),
        )
    };
    let fetcher = global.bind().scene_fetcher.clone();
    SceneFacts {
        realm_url,
        parcel: fetcher.as_ref().map_or(Parcel::new(0, 0), parcel_of),
        scene_title: fetcher
            .as_ref()
            .and_then(|fetcher| fetcher.bind().current_scene_title()),
        realm_name,
    }
}

fn parcel_of(fetcher: &Gd<SceneFetcher>) -> Parcel {
    let parcel = fetcher.bind().get_current_position();
    if parcel == NO_PARCEL {
        Parcel::new(0, 0)
    } else {
        Parcel::new(parcel.x, parcel.y)
    }
}

/// The catalyst that serves profiles: the realm's lambdas, and the realm's content node for deploys.
fn read_endpoints(global: &Gd<DclGlobal>) -> Endpoints {
    let realm = global.bind().realm.clone();
    let realm = realm.bind();
    Endpoints::new(
        &realm.get_lambda_server_base_url().to_string(),
        &realm.get_profile_deployment_url().to_string(),
    )
}

/// Hands the core every profile the engine sets for the player, so the next deploy builds on it.
fn forward_profile_changes() {
    let Some(global) = DclGlobal::try_singleton() else {
        return;
    };
    let mut identity = global.bind().player_identity.clone();
    let callable = Callable::from_fn("adopt_engine_profile", |args| {
        if let Some(profile) = args
            .first()
            .and_then(|arg| arg.try_to::<Gd<DclUserProfile>>().ok())
        {
            adopt_profile(&profile);
        }
        Variant::nil()
    });
    identity.connect("profile_changed", &callable);
}

fn adopt_profile(profile: &Gd<DclUserProfile>) {
    let content = serde_json::to_value(&profile.bind().inner.content);
    let Ok(serde_json::Value::Object(content)) = content else {
        godot_warn!("scene_link: the engine profile is not a JSON object");
        return;
    };
    if let Err(error) = profile::global().adopt(ProfileDocument::from_content(content)) {
        tracing::debug!("scene_link: profile not adopted: {error}");
    }
}
