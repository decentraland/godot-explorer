use crate::{
    dcl::crdt::{
        grow_only_set::GenericGrowOnlySetComponentOperation,
        last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
        SceneCrdtStateProtoComponents,
    },
    scene_runner::scene::Scene,
};

pub fn update_avatar_scene_updates(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    if !scene.avatar_scene_updates.deleted_entities.is_empty() {
        for entity_id in &scene.avatar_scene_updates.deleted_entities {
            let in_transform = scene
                .avatar_scene_updates
                .transform
                .contains_key(entity_id);
            let in_avatar_base = scene
                .avatar_scene_updates
                .avatar_base
                .contains_key(entity_id);
            let in_player_identity = scene
                .avatar_scene_updates
                .player_identity_data
                .contains_key(entity_id);
            let in_avatar_equipped = scene
                .avatar_scene_updates
                .avatar_equipped_data
                .contains_key(entity_id);
            let in_internal = scene
                .avatar_scene_updates
                .internal_player_data
                .contains_key(entity_id);
            if in_transform
                || in_avatar_base
                || in_player_identity
                || in_avatar_equipped
                || in_internal
            {
                tracing::warn!(
                    "avatar_lifecycle: same-tick race scene={:?} entity={:?} concurrent puts: transform={} avatar_base={} player_identity={} avatar_equipped={} internal={}",
                    scene.scene_id,
                    entity_id,
                    in_transform,
                    in_avatar_base,
                    in_player_identity,
                    in_avatar_equipped,
                    in_internal
                );
            }
        }
    }

    {
        let transform_component = crdt_state.get_transform_mut();
        for (entity_id, value) in scene.avatar_scene_updates.transform.drain() {
            transform_component.put(entity_id, value);
        }
    }

    {
        let avatar_base_component = SceneCrdtStateProtoComponents::get_avatar_base_mut(crdt_state);
        for (entity_id, value) in scene.avatar_scene_updates.avatar_base.drain() {
            avatar_base_component.put(entity_id, Some(value));
        }
    }

    {
        let player_identity_data_component =
            SceneCrdtStateProtoComponents::get_player_identity_data_mut(crdt_state);
        for (entity_id, value) in scene.avatar_scene_updates.player_identity_data.drain() {
            player_identity_data_component.put(entity_id, Some(value));
        }
    }

    {
        let avatar_equipped_data_component =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(crdt_state);
        for (entity_id, value) in scene.avatar_scene_updates.avatar_equipped_data.drain() {
            avatar_equipped_data_component.put(entity_id, Some(value));
        }
    }

    {
        let internal_player_data_component = crdt_state.get_internal_player_data_mut();
        for (entity_id, value) in scene.avatar_scene_updates.internal_player_data.drain() {
            internal_player_data_component.put(entity_id, Some(value));
        }
    }

    {
        let avatar_emote_command_component =
            SceneCrdtStateProtoComponents::get_avatar_emote_command_mut(crdt_state);
        for (entity_id, vec_value) in scene.avatar_scene_updates.avatar_emote_command.drain() {
            let mut timestamp: u32 = {
                if let Some(commands) = avatar_emote_command_component.get(&entity_id) {
                    commands.iter().map(|c| c.timestamp).max().unwrap_or(0) + 1
                } else {
                    0
                }
            };

            for mut value in vec_value {
                value.timestamp = timestamp;
                timestamp += 1;
                avatar_emote_command_component.append(entity_id, value);
            }
        }
    }

    // Clear last so a disconnect always wins over a concurrent profile/transform
    // update for the same entity in the same tick — otherwise the puts above
    // would resurrect components on a player that just left.
    for entity_id in scene.avatar_scene_updates.deleted_entities.drain() {
        tracing::info!(
            "avatar_lifecycle: clear_entity_components scene={:?} entity={:?}",
            scene.scene_id,
            entity_id
        );
        crdt_state.clear_entity_components(&entity_id);
    }
}
