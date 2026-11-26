use std::collections::{HashMap, HashSet};

use godot::prelude::*;

use super::{
    avatar_scene::AvatarScene, avatar_type::DclAvatarWireFormat, dcl_user_profile::DclUserProfile,
};
use crate::{
    comms::profile::{AvatarColor, AvatarWireFormat},
    dcl::{components::SceneEntityId, SceneId},
    godot_classes::dcl_global::DclGlobal,
};

/// What an `AvatarShape` written on a player entity changes in that player's look.
/// `None` keeps the profile value.
#[derive(Clone, PartialEq)]
pub struct SceneLook {
    pub body_shape: Option<String>,
    pub skin: Option<AvatarColor>,
    pub hair: Option<AvatarColor>,
    pub eyes: Option<AvatarColor>,
    pub show_only_wearables: Option<bool>,
    /// Replaces the profile wearables when non-empty.
    pub wearables: Vec<String>,
    /// Categories of the scene GLBs in `wearables`: force-rendered, a scene GLB is never hidden.
    pub scene_categories: Vec<String>,
}

impl SceneLook {
    fn apply_to(&self, profile_avatar: &AvatarWireFormat) -> AvatarWireFormat {
        let mut look = profile_avatar.clone();
        if !self.wearables.is_empty() {
            look.wearables = self.wearables.clone();
        }
        if let Some(body_shape) = &self.body_shape {
            look.body_shape = Some(body_shape.clone());
        }
        look.skin = self.skin.or(look.skin);
        look.hair = self.hair.or(look.hair);
        look.eyes = self.eyes.or(look.eyes);
        if let Some(show_only_wearables) = self.show_only_wearables {
            look.show_only_wearables = show_only_wearables;
        }
        if !self.scene_categories.is_empty() {
            look.force_render
                .get_or_insert_with(Vec::new)
                .extend(self.scene_categories.iter().cloned());
        }
        look
    }
}

/// Looks written by scenes on player entities. Only the current scene's looks are rendered;
/// they are local to this client and never broadcast.
#[derive(Default)]
pub struct SceneLooks {
    by_scene: HashMap<SceneId, HashMap<SceneEntityId, SceneLook>>,
    current_scene: Option<SceneId>,
    /// Players to re-render on the next frame.
    dirty: HashSet<SceneEntityId>,
}

/// Drops what a removed scene put on avatars. Its scene wearables stay while a reload of the same
/// scene is alive, since both share them.
pub fn forget_removed_scene(scene_id: SceneId, scene_entity_id: &str, reload_alive: bool) {
    let global = DclGlobal::singleton();
    let mut avatars = global.bind().avatars.clone();
    avatars.bind_mut().forget_scene_looks(scene_id);
    if !reload_alive {
        let mut content_provider = global.bind().content_provider.clone();
        content_provider
            .bind_mut()
            .scene_wearables
            .forget_scene(scene_entity_id);
    }
}

impl AvatarScene {
    /// `look` is `None` when the scene removes the `AvatarShape`.
    pub fn set_scene_look(
        &mut self,
        scene_id: SceneId,
        entity: SceneEntityId,
        look: Option<SceneLook>,
    ) {
        let looks = &mut self.scene_looks;
        let scene_looks = looks.by_scene.entry(scene_id).or_default();
        let changed = match look {
            Some(look) => scene_looks.insert(entity, look.clone()) != Some(look),
            None => scene_looks.remove(&entity).is_some(),
        };
        if changed && looks.current_scene == Some(scene_id) {
            looks.dirty.insert(entity);
        }
    }

    pub fn set_current_scene_for_looks(&mut self, scene_id: SceneId) {
        let looks = &mut self.scene_looks;
        if looks.current_scene == Some(scene_id) {
            return;
        }
        for id in [looks.current_scene, Some(scene_id)].into_iter().flatten() {
            if let Some(scene_looks) = looks.by_scene.get(&id) {
                looks.dirty.extend(scene_looks.keys().copied());
            }
        }
        looks.current_scene = Some(scene_id);
    }

    fn forget_scene_looks(&mut self, scene_id: SceneId) {
        let looks = &mut self.scene_looks;
        if let Some(scene_looks) = looks.by_scene.remove(&scene_id) {
            if looks.current_scene == Some(scene_id) {
                looks.dirty.extend(scene_looks.into_keys());
            }
        }
    }

    pub(super) fn forget_entity_looks(&mut self, entity: SceneEntityId) {
        for scene_looks in self.scene_looks.by_scene.values_mut() {
            scene_looks.remove(&entity);
        }
        self.scene_looks.dirty.remove(&entity);
    }

    /// The look to render for `entity` wearing `profile_avatar`; `None` renders the profile.
    pub(super) fn scene_look_for(
        &self,
        entity: SceneEntityId,
        profile_avatar: &AvatarWireFormat,
    ) -> Option<Gd<DclAvatarWireFormat>> {
        let looks = &self.scene_looks;
        let look = looks.by_scene.get(&looks.current_scene?)?.get(&entity)?;
        Some(DclAvatarWireFormat::from_gd(look.apply_to(profile_avatar)))
    }

    /// Runs every frame: re-renders the players whose look changed.
    pub(super) fn apply_dirty_scene_looks(&mut self) {
        if self.scene_looks.dirty.is_empty() {
            return;
        }
        for entity in std::mem::take(&mut self.scene_looks.dirty) {
            let Some(profile) = self.last_updated_profile.get(&entity) else {
                continue;
            };
            let avatar: Option<Gd<Node>> = if entity == SceneEntityId::PLAYER {
                let scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
                let avatar = scene_runner.bind().get_player_avatar_node();
                avatar.map(|avatar| avatar.upcast())
            } else {
                self.avatar_godot_scene
                    .get(&entity)
                    .map(|avatar| avatar.clone().upcast())
            };
            let Some(mut avatar) = avatar else {
                continue;
            };
            let look = self.scene_look_for(entity, &profile.content.avatar);
            avatar.call(
                "async_update_avatar_from_profile",
                &[
                    DclUserProfile::from_gd(profile.clone()).to_variant(),
                    look.to_variant(),
                ],
            );
        }
    }
}
