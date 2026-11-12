use godot::prelude::*;

use super::{avatar_scene::AvatarScene, dcl_user_profile::DclUserProfile};
use crate::{
    avatars::avatar_type::DclAvatarWireFormat,
    comms::profile::{AvatarColor, AvatarColor3},
    dcl::{
        components::{
            proto_components::{common::Color3, sdk::components::PbAvatarShape},
            SceneEntityId,
        },
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents},
    },
    godot_classes::dcl_global::DclGlobal,
};

fn to_avatar_color(color: &Color3) -> AvatarColor {
    AvatarColor {
        color: AvatarColor3 {
            r: color.r,
            g: color.g,
            b: color.b,
        },
    }
}

/// Item URN of a loaded wearable vs the (possibly token-instance) URN in the profile.
fn same_item(profile_urn: &str, loaded_id: &str) -> bool {
    let (a, b) = (profile_urn.to_lowercase(), loaded_id.to_lowercase());
    a == b || a.starts_with(&format!("{b}:")) || b.starts_with(&format!("{a}:"))
}

impl AvatarScene {
    /// `AvatarShape` written on `engine.PlayerEntity`: overrides the local player's look.
    /// Unset fields keep the profile values; `None` (component removed) restores the profile.
    pub fn set_player_avatar_override(&mut self, shape: Option<&PbAvatarShape>) {
        let player = SceneEntityId::PLAYER;
        let (Some(avatar), Some(profile)) = (
            self.avatar_godot_scene.get(&player),
            self.last_updated_profile.get(&player),
        ) else {
            return;
        };
        let mut avatar = avatar.clone();

        let Some(shape) = shape else {
            avatar.set_meta("avatar_shape_override", &false.to_variant());
            avatar.call(
                "async_update_avatar_from_profile",
                &[DclUserProfile::from_gd(profile.clone()).to_variant()],
            );
            return;
        };

        let mut wire = profile.content.avatar.clone();
        // POC: only wearable URNs; scene GLB paths (`<name>_<category>.glb`) are not loaded yet.
        let urns: Vec<String> = shape
            .wearables
            .iter()
            .filter(|w| w.starts_with("urn:"))
            .cloned()
            .collect();
        if !urns.is_empty() {
            wire.wearables = urns;
            wire.force_render = None;
        }
        if let Some(body_shape) = &shape.body_shape {
            wire.body_shape = Some(body_shape.clone());
        }
        if let Some(c) = &shape.skin_color {
            wire.skin = Some(to_avatar_color(c));
        }
        if let Some(c) = &shape.hair_color {
            wire.hair = Some(to_avatar_color(c));
        }
        if let Some(c) = &shape.eye_color {
            wire.eyes = Some(to_avatar_color(c));
        }
        if let Some(only) = shape.show_only_wearables {
            wire.show_only_wearables = only;
        }

        let name: GString = avatar.call("get_avatar_name", &[]).to();
        avatar.set_meta("avatar_shape_override", &true.to_variant());
        avatar.call_deferred(
            "async_update_avatar",
            &[
                DclAvatarWireFormat::from_gd(wire).to_variant(),
                name.to_variant(),
                VarDictionary::new().to_variant(),
            ],
        );
    }

    pub(super) fn apply_player_equipped_visibility(&mut self, visible_urns: &PackedStringArray) {
        let player = SceneEntityId::PLAYER;
        let Some(mut data) =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data(&self.crdt_state)
                .get(&player)
                .and_then(|v| v.value.clone())
        else {
            return;
        };
        let visible: Vec<String> = visible_urns
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for wearable in data.wearables.iter_mut() {
            wearable.visible = Some(visible.iter().any(|id| same_item(&wearable.urn, id)));
        }

        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let mut scene_runner = scene_runner.bind_mut();
        for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
            scene
                .avatar_scene_updates
                .avatar_equipped_data
                .insert(player, data.clone());
        }
        SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(&mut self.crdt_state)
            .put(player, Some(data));
    }
}
