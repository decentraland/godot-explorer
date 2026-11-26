use std::collections::HashMap;

use godot::prelude::*;

use super::{avatar_scene::AvatarScene, item::DclItemEntityDefinition};
use crate::{
    comms::profile::UserProfile,
    dcl::{
        components::SceneEntityId,
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents},
    },
    godot_classes::{dcl_global::DclGlobal, promise::Promise},
};

/// Same order as `Wearables.Categories.HIDING_PRIORITY` in wearables.gd, which renders the avatar.
const HIDING_PRIORITY: [&str; 18] = [
    "skin",
    "upper_body",
    "hands_wear",
    "lower_body",
    "feet",
    "helmet",
    "hat",
    "top_head",
    "mask",
    "eyewear",
    "earring",
    "tiara",
    "hair",
    "eyebrows",
    "eyes",
    "mouth",
    "facial_hair",
    "body_shape",
];

/// Whether each wearable of a profile outfit is visible once hides, replaces and force render
/// apply. Of two wearables with the same category the later one is used. A wearable that did
/// not load, or has no representation for the body shape, is not visible.
fn outfit_visibility(body_shape: &str, wearables: &[String], force_render: &[String]) -> Vec<bool> {
    let mut content_provider = DclGlobal::singleton().bind().content_provider.clone();
    let definitions: Vec<Option<Gd<DclItemEntityDefinition>>> = {
        let mut content_provider = content_provider.bind_mut();
        wearables
            .iter()
            .map(|urn| {
                content_provider
                    .get_wearable(GString::from(urn))
                    .filter(|definition| definition.bind()._is_valid_wearable(body_shape, true))
            })
            .collect()
    };
    let category =
        |definition: &Gd<DclItemEntityDefinition>| definition.bind().get_category().to_string();

    let mut used: HashMap<String, usize> = HashMap::new();
    for (index, definition) in definitions.iter().enumerate() {
        if let Some(definition) = definition {
            used.insert(category(definition), index);
        }
    }

    let mut hidden: Vec<String> = Vec::new();
    for priority_category in HIDING_PRIORITY {
        if hidden.iter().any(|category| category == priority_category) {
            continue;
        }
        let Some(definition) = used
            .get(priority_category)
            .and_then(|index| definitions[*index].as_ref())
        else {
            continue;
        };
        for category in definition
            .bind()
            .get_hides_list(body_shape.to_string())
            .as_slice()
        {
            let category = category.to_string();
            if !hidden.contains(&category) {
                hidden.push(category);
            }
        }
    }
    hidden.retain(|category| !force_render.contains(category));

    definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            definition.as_ref().is_some_and(|definition| {
                let category = category(definition);
                used.get(&category) == Some(&index) && !hidden.contains(&category)
            })
        })
        .collect()
}

impl AvatarScene {
    /// Starts resolving `AvatarEquippedData.wearables[].visible` for the outfit of `profile`. It
    /// is computed from the profile, not from the rendered avatar, so a scene look does not change it.
    pub(super) fn request_equipped_visibility(
        &mut self,
        entity: SceneEntityId,
        profile: &UserProfile,
    ) {
        let urns = VarArray::from_iter(
            profile
                .content
                .avatar
                .wearables
                .iter()
                .map(|urn| urn.to_variant()),
        );
        let global = DclGlobal::singleton();
        let content_url = global.bind().get_realm().bind().get_profile_content_url();
        let mut content_provider = global.bind().content_provider.clone();
        let promises = content_provider
            .bind_mut()
            .fetch_wearables(urns, content_url);
        self.pending_equipped_visibility.insert(
            entity,
            promises
                .iter_shared()
                .map(|promise| promise.instance_id())
                .collect(),
        );
    }

    /// Runs every frame: writes the visibility of the outfits whose wearables all loaded or failed.
    pub(super) fn resolve_pending_equipped_visibility(&mut self) {
        if self.pending_equipped_visibility.is_empty() {
            return;
        }
        let resolved: Vec<SceneEntityId> = self
            .pending_equipped_visibility
            .iter()
            .filter(|(_, promises)| {
                promises.iter().all(|id| {
                    Gd::<Promise>::try_from_instance_id(*id)
                        .map_or(true, |promise| promise.bind().is_resolved())
                })
            })
            .map(|(entity, _)| *entity)
            .collect();
        for entity in resolved {
            self.pending_equipped_visibility.remove(&entity);
            self.write_equipped_visibility(entity);
        }
    }

    fn write_equipped_visibility(&mut self, entity: SceneEntityId) {
        let Some(profile) = self.last_updated_profile.get(&entity) else {
            return;
        };
        let Some(mut data) =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data(&self.crdt_state)
                .get(&entity)
                .and_then(|value| value.value.clone())
        else {
            return;
        };
        let avatar = &profile.content.avatar;
        let visibility = outfit_visibility(
            avatar.body_shape.as_deref().unwrap_or_default(),
            &avatar.wearables,
            avatar.force_render.as_deref().unwrap_or_default(),
        );
        let before = data.clone();
        for (wearable, visible) in data.wearables.iter_mut().zip(visibility) {
            wearable.visible = Some(visible);
        }
        if data == before {
            return;
        }

        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        for (_, scene) in scene_runner.bind_mut().get_all_scenes_mut().iter_mut() {
            scene
                .avatar_scene_updates
                .avatar_equipped_data
                .insert(entity, data.clone());
        }
        SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(&mut self.crdt_state)
            .put(entity, Some(data));
    }
}
