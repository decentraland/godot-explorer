//! The navigation entry points GDScript still calls by name. Each returns a `Promise` resolved
//! with the outcome when the caller needs to wait for it.

use super::globals;
use godot::classes::{http_client::Method, Time};
use godot::prelude::*;

use super::{
    navigation::{self, Navigation},
    scene_fetcher::explorer,
};
use crate::{
    engine::local_task::{await_promise, spawn_local},
    godot_classes::{
        dcl_global::DclGlobal,
        promise::{Promise, PromiseError},
    },
};

const EXPLORER_SCENE: &str = "res://src/ui/explorer.tscn";

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct DclNavigator {
    base: Base<Object>,
}

fn parcel_of(value: &Variant) -> Option<(i32, i32)> {
    let parcel = value.try_to::<Vector2i>().ok()?;
    // `Vector2i.MAX` is the "no parcel" sentinel the deeplink object uses.
    (parcel != Vector2i::new(i32::MAX, i32::MAX)).then_some((parcel.x, parcel.y))
}

fn run_bool(task: impl std::future::Future<Output = bool> + 'static) -> Gd<Promise> {
    let promise = Promise::new_alloc();
    let mut handle = promise.clone();
    spawn_local(async move {
        let outcome = task.await;
        if handle.is_instance_valid() {
            handle.bind_mut().resolve_with_data(outcome.to_variant());
        }
    });
    promise
}

fn say_in_chat(translation_key: &str, replacements: VarDictionary) {
    let mut global = DclGlobal::singleton();
    let text = global.tr(translation_key);
    let text = text
        .to_variant()
        .call("format", &[replacements.to_variant()]);
    global.emit_signal(
        "on_chat_message",
        &[
            "system".to_variant(),
            text,
            Time::singleton().get_unix_time_from_system().to_variant(),
        ],
    );
}

/// A cold start from the lobby: there is no scene to protect and no explorer to route through,
/// so the private-world gate stays here (booting straight into a world the user cannot enter
/// is the bug #1725 fixed) and the explorer scene starts on the saved destination.
async fn start_explorer(realm: &str, parcel: (i32, i32)) {
    if !navigation::precheck_realm_access(realm).await {
        return;
    }
    let global = DclGlobal::singleton();
    DclGlobal::set_orientation_landscape();
    let engine = globals::engine();
    let engine = engine.bind();
    engine.set_setting("last_realm_joined".into(), realm.to_variant(), false);
    engine.set_setting("last_parcel_x".into(), parcel.0.to_variant(), false);
    engine.set_setting("last_parcel_y".into(), parcel.1.to_variant(), true);
    if let Some(mut tree) = global.get_tree_or_null() {
        tree.change_scene_to_file(EXPLORER_SCENE);
    }
}

#[godot_api]
impl DclNavigator {
    #[func]
    pub fn set_world_secret(realm: GString, secret: GString) {
        navigation::set_world_secret(&realm.to_string(), &secret.to_string());
    }

    /// Goes to `realm` (empty: the current one) at `parcel` (`Vector2i.MAX`: none), as an
    /// intent. Resolves with whether the player went somewhere.
    #[func]
    pub fn go_input(realm: GString, parcel: Variant, when: GString) -> Gd<Promise> {
        let navigation = Navigation::from_input(&realm.to_string(), parcel_of(&parcel));
        let when = when.to_string();
        run_bool(async move { navigation::go(navigation, &when).await })
    }

    /// Re-enters `realm` without gating the parcel: resuming a session or a cold-start link.
    #[func]
    pub fn go_restore(realm: GString, parcel: Variant, when: GString) -> Gd<Promise> {
        let navigation = Navigation::restore(&realm.to_string(), parcel_of(&parcel));
        let when = when.to_string();
        run_bool(async move { navigation::go(navigation, &when).await })
    }

    /// Makes `realm` the current realm without resolving it or needing an explorer (the offline
    /// scene renderer). Resolves with whether it was committed.
    #[func]
    pub fn set_realm(realm: GString) -> Gd<Promise> {
        let realm = realm.to_string();
        run_bool(async move {
            let node = globals::realm();
            super::realm_flow::set_realm(node, &realm).await
        })
    }

    /// /reload, scene-crash retry, loading-timeout retry.
    #[func]
    pub fn reload_current(when: GString) -> Gd<Promise> {
        let when = when.to_string();
        run_bool(async move { navigation::go(Navigation::reload_current(), &when).await })
    }

    #[func]
    pub fn recover_from_refusal(realm: GString) {
        navigation::recover_from_refusal(&realm.to_string());
    }

    /// Resolves with false when the private-world modal is already up.
    #[func]
    pub fn precheck_realm_access(realm: GString) -> Gd<Promise> {
        let realm = realm.to_string();
        run_bool(async move { navigation::precheck_realm_access(&realm).await })
    }

    #[func]
    pub fn teleport_to(parcel: Vector2i, realm: GString) -> Gd<Promise> {
        let realm = realm.to_string();
        run_bool(async move {
            if explorer().is_none() {
                start_explorer(&realm, (parcel.x, parcel.y)).await;
                return false;
            }
            let navigation = Navigation::from_input(&realm, Some((parcel.x, parcel.y)));
            let went = navigation::go(navigation, "on_teleport").await;
            if went {
                let mut replacements = VarDictionary::new();
                replacements.set("location", format!("({}, {})", parcel.x, parcel.y));
                say_in_chat("CHAT_SYSTEM_TELEPORTED", replacements);
            }
            went
        })
    }

    #[func]
    pub fn join_world(realm: GString) -> Gd<Promise> {
        let realm = realm.to_string();
        run_bool(async move {
            if explorer().is_none() {
                start_explorer(&realm, (0, 0)).await;
                return false;
            }
            let mut replacements = VarDictionary::new();
            replacements.set("world", realm.as_str());
            say_in_chat("CHAT_SYSTEM_CHANGING_WORLD", replacements);
            // No parcel: the world names its own spawn point.
            navigation::go(Navigation::from_input(&realm, None), "on_world").await
        })
    }

    /// Signed fetch (ADR-44): the request metadata travels in `x-identity-metadata`, and a
    /// bodyless request signs `{}` (the credits-server crypto-middleware rejects `null`).
    /// Services that verify something other than the body pass `metadata_override`.
    /// Resolves with the `RequestResponse`, or is rejected.
    #[func]
    pub fn signed_fetch(
        url: GString,
        method: i64,
        body: GString,
        metadata_override: GString,
        extra_headers: VarDictionary,
    ) -> Gd<Promise> {
        let promise = Promise::new_alloc();
        let mut handle = promise.clone();
        let (url, body) = (url.to_string(), body.to_string());
        let mut metadata = metadata_override.to_string();
        if metadata.is_empty() {
            metadata = if body.is_empty() {
                "{}".to_string()
            } else {
                body.clone()
            };
        }
        spawn_local(async move {
            let outcome = signed_fetch(&url, method as i32, &body, &metadata, extra_headers).await;
            if !handle.is_instance_valid() {
                return;
            }
            match outcome {
                Ok(response) => handle.bind_mut().resolve_with_data(response),
                Err(reason) => handle.bind_mut().reject(reason.as_str().into()),
            }
        });
        promise
    }
}

fn method_name(method: i32) -> (&'static str, Method) {
    match method {
        1 => ("POST", Method::POST),
        2 => ("PUT", Method::PUT),
        3 => ("DELETE", Method::DELETE),
        4 => ("HEAD", Method::HEAD),
        5 => ("OPTIONS", Method::OPTIONS),
        6 => ("TRACE", Method::TRACE),
        7 => ("CONNECT", Method::CONNECT),
        8 => ("PATCH", Method::PATCH),
        _ => ("GET", Method::GET),
    }
}

async fn signed_fetch(
    url: &str,
    method: i32,
    body: &str,
    metadata: &str,
    extra_headers: VarDictionary,
) -> Result<Variant, String> {
    let (name, godot_method) = method_name(method);
    let identity = globals::player_identity();
    let headers_promise =
        identity
            .bind()
            .async_get_identity_headers(url.into(), metadata.into(), name.into());
    let headers = await_promise(headers_promise).await;
    if let Ok(error) = headers.try_to::<Gd<PromiseError>>() {
        return Err(error.bind().get_error().to_string());
    }
    let mut headers = headers
        .try_to::<VarDictionary>()
        .map_err(|_| "identity headers are not a dictionary".to_string())?;
    if !body.is_empty() {
        headers.set("Content-Type", "application/json");
    }
    for (key, value) in extra_headers.iter_shared() {
        headers.set(&key, &value);
    }
    let requester = globals::http_requester();
    let response = requester
        .bind()
        .request_json(url.into(), godot_method, body.into(), headers);
    let result = await_promise(response).await;
    match result.try_to::<Gd<PromiseError>>() {
        Ok(error) => Err(error.bind().get_error().to_string()),
        Err(_) => Ok(result),
    }
}
