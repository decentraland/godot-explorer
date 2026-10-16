//! The avatar methods the remaining GDScript and the scene's signal connections call by name.
//! Rust callers use the typed methods in the sibling modules.

use godot::classes::{Node, Skeleton3D};
use godot::prelude::*;

use super::build::{update_avatar, update_from_profile};
use super::emote_load::{async_freeze_on_emote, async_play_emote_or_stash};
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::engine::local_task::spawn_local;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::promise::Promise;

/// A promise resolved when the future ends (a GDScript caller awaits it).
fn promise_of(future: impl std::future::Future<Output = ()> + 'static) -> Gd<Promise> {
    let promise = Promise::new_alloc();
    let handle = promise.clone();
    spawn_local(async move {
        future.await;
        if handle.is_instance_valid() {
            handle.clone().bind_mut().resolve();
        }
    });
    promise
}

#[godot_api(secondary)]
impl DclAvatar {
    #[func(rename = async_update_avatar_from_profile)]
    fn gd_update_from_profile(&mut self, profile: Gd<DclUserProfile>) -> Gd<Promise> {
        promise_of(update_from_profile(self.to_gd(), profile))
    }

    #[func(rename = async_update_avatar)]
    fn gd_update_avatar(&mut self, wire: Gd<DclAvatarWireFormat>, name: GString) -> Gd<Promise> {
        promise_of(update_avatar(self.to_gd(), wire, name.to_string(), None))
    }

    #[func(rename = async_play_emote)]
    fn gd_play_emote(&mut self, urn: GString, mask: i64, scene_id: i32) -> Gd<Promise> {
        promise_of(async_play_emote_or_stash(
            self.to_gd(),
            urn.to_string(),
            mask,
            scene_id,
        ))
    }

    #[func(rename = async_freeze_on_emote)]
    fn gd_freeze_on_emote(&mut self, urn: GString, normalized_t: f64) -> Gd<Promise> {
        let avatar = self.to_gd();
        let promise = Promise::new_alloc();
        let handle = promise.clone();
        spawn_local(async move {
            let frozen = async_freeze_on_emote(avatar, &urn.to_string(), normalized_t).await;
            if handle.is_instance_valid() {
                handle
                    .clone()
                    .bind_mut()
                    .resolve_with_data(frozen.to_variant());
            }
        });
        promise
    }

    #[func(rename = freeze_on_idle)]
    fn gd_freeze_on_idle(&mut self) {
        self.freeze_on_idle();
    }

    #[func(rename = set_teleport_grace)]
    fn gd_set_teleport_grace(&mut self) {
        self.set_teleport_grace();
    }

    #[func(rename = clean_unused_emotes)]
    fn gd_clean_unused_emotes(&mut self) {
        self.clean_unused_emotes();
    }

    #[func(rename = try_show)]
    fn gd_try_show(&mut self) {
        self.try_show();
    }

    #[func(rename = get_body_shape_skeleton)]
    fn gd_body_shape_skeleton(&self) -> Option<Gd<Skeleton3D>> {
        self.nodes.as_ref().map(|n| n.skeleton.clone())
    }

    #[func(rename = register_anchor_use)]
    fn gd_register_anchor_use(&mut self, anchor_id: i32, attachment: Gd<Node>) {
        self.add_anchor_user(anchor_id, attachment.instance_id());
    }

    #[func(rename = unregister_anchor_use)]
    fn gd_unregister_anchor_use(&mut self, anchor_id: i32, attachment: Gd<Node>) {
        self.remove_anchor_user(anchor_id, attachment.instance_id());
    }

    #[func(rename = get_anchor_point_global_transform)]
    fn gd_anchor_point(&self, anchor_id: i32) -> Transform3D {
        self.get_anchor_point_global_transform(anchor_id)
    }

    #[func(rename = apply_toon_material_to)]
    fn gd_apply_toon_material_to(&self, node: Gd<Node>) {
        self.apply_toon_material_to(node);
    }

    #[func(rename = get_impostor_cache_key)]
    fn gd_impostor_cache_key(&self) -> GString {
        GString::from(&self.get_impostor_cache_key())
    }

    /// Starts downloading an emote's definition (not its glTF); resolved when it is cached.
    #[func]
    fn async_fetch_emote_definition(urn: GString) -> Gd<Promise> {
        promise_of(async move {
            crate::avatars::loaders::fetch_emote_definition(&urn.to_string()).await;
        })
    }

    /// The global toggle for the move/jump/land dust, set from the graphics profile.
    #[func]
    fn apply_particles_enabled(enabled: bool) {
        super::lod::apply_particles_enabled(enabled);
    }

    #[func]
    fn particles_enabled() -> bool {
        super::lod::particles_enabled()
    }

    // Signal and notification targets.

    /// The local player entered or left a scene (wired by the player body).
    #[func]
    fn on_current_scene_changed(&mut self, scene_id: i32) {
        self.on_current_scene_changed_impl(scene_id);
    }

    #[func]
    fn on_skeleton_updated(&mut self) {
        self.refresh_anchor_poses();
        self.recompute_nametag_posed_top();
    }

    #[func]
    fn on_global_chat_message(&mut self, address: GString, message: GString, _timestamp: f64) {
        self.on_chat_message(&address.to_string(), &message.to_string());
    }

    #[func]
    fn on_avatar_screen_entered(&mut self) {
        self.lod.on_screen = true;
        self.apply_screen_freeze();
    }

    #[func]
    fn on_avatar_screen_exited(&mut self) {
        self.lod.on_screen = false;
        self.apply_screen_freeze();
    }

    #[func]
    fn on_timer_hide_mic_timeout(&mut self) {
        self.hide_mic();
    }
}
