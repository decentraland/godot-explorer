//! The chat commands that move the player or the realm.

use godot::prelude::*;

use super::chat;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;

pub fn clear_realm() {
    globals::realm().bind_mut().async_clear_realm();
}

/// Announces the teleport once it happened, not before: the destination is proved first, so
/// claiming success up front would put "Teleported" above the modal that refused it.
pub fn go_to_parcel(parcel: Vector2i) {
    spawn_local(async move {
        let promise = DclNavigator::go_input("".into(), parcel.to_variant(), "on_teleport".into());
        if await_promise(promise)
            .await
            .try_to::<bool>()
            .unwrap_or(false)
        {
            let mut replacements = VarDictionary::new();
            replacements.set("location", format!("({}, {})", parcel.x, parcel.y));
            chat::say_translated("CHAT_SYSTEM_TELEPORTED", &replacements);
        }
    });
}

pub fn change_realm_with_notice(realm: &str, when: &str) {
    let mut replacements = VarDictionary::new();
    replacements.set("realm", realm);
    chat::say_translated("CHAT_SYSTEM_CHANGING_REALM", &replacements);
    let (realm, when) = (realm.to_string(), when.to_string());
    spawn_local(async move {
        let promise = DclNavigator::go_input(
            realm.as_str().into(),
            Vector2i::MAX.to_variant(),
            when.as_str().into(),
        );
        await_promise(promise).await;
    });
}
