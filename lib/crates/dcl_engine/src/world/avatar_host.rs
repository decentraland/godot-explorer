//! The scene of the shell's avatar-only mode. No world is loaded; the engine only renders the
//! preview that `Backpack` places over the shell's screens.

use godot::classes::{INode, Node};
use godot::prelude::*;

use super::host::on_avatar_host_ready;
use crate::engine::local_task::{next_frame, signal_once, spawn_local};
use crate::fetcher::globals;

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct AvatarHost {
    base: Base<Node>,
}

#[godot_api]
impl INode for AvatarHost {
    fn ready(&mut self) {
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_transparent_background(true);
        }
        spawn_local(boot());
    }
}

async fn boot() {
    let mut identity = globals::player_identity();
    if !identity.bind().get_address_str().is_empty() {
        // Back from a world: keep the account and the customized profile.
        on_avatar_host_ready();
        return;
    }
    identity.bind_mut().create_disposable_account();
    // The guest wallet's own default-profile pass is deferred; the random profile must land after it.
    next_frame().await;
    next_frame().await;
    identity.bind_mut().set_random_profile();
    signal_once(&mut identity, "profile_changed").await;
    on_avatar_host_ready();
}
