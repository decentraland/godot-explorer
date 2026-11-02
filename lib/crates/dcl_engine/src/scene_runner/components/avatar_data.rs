use crate::{
    dcl::crdt::{
        grow_only_set::GenericGrowOnlySetComponentOperation,
        last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
        SceneCrdtStateProtoComponents,
    },
    scene_runner::scene::Scene,
};

pub fn update_avatar_scene_updates(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    for entity_id in scene.avatar_scene_updates.deleted_entities.drain() {
        // Issue #1818: a departing player (disconnect or walk-out) must be communicated
        // to the scene's V8 CRDT so `@dcl/sdk/players` onLeaveScene fires and iterating
        // entities with PlayerIdentityData drops the peer.
        //
        // We must NOT do this with an entity death: the renderer→scene path never
        // carries entity deaths (they're stripped from the dirty set, and #1444 stops
        // them reaching the SDK). Instead we mirror the enter path — which works by
        // PUT-ing components — and set the player components to None. A None value is a
        // dirty component DELETE that propagates to V8 exactly like a PUT does.
        //
        // The entity is left alive (component-less) in the scene CRDT; avatar entity
        // slots are recycled from `AvatarScene::crdt_state`, which is killed separately
        // in `AvatarScene::remove_avatar`, so no scene-side kill is needed here.
        SceneCrdtStateProtoComponents::get_player_identity_data_mut(crdt_state)
            .put(entity_id, None);
        SceneCrdtStateProtoComponents::get_avatar_base_mut(crdt_state).put(entity_id, None);
        SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(crdt_state)
            .put(entity_id, None);
        crdt_state
            .get_internal_player_data_mut()
            .put(entity_id, None);
        crdt_state.get_transform_mut().put(entity_id, None);
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
}
