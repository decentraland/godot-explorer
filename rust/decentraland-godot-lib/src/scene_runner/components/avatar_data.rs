use crate::{
    dcl::crdt::{
        last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
        SceneCrdtStateProtoComponents,
    },
    scene_runner::scene::Scene,
};

pub fn update_avatar_scene_updates(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    for entity_id in scene.avatar_scene_updates.deleted_entities.drain() {
        crdt_state.entities.kill(entity_id);
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

    // {
    //     let avatar_emote_command_component =
    //         SceneCrdtStateProtoComponents::get_avatar_emote_command_mut(crdt_state);
    //     for (entity_id, value) in
    //         scene.avatar_scene_updates.avatar_emote_command.drain()
    //     {
    //         avatar_emote_command_component.put(entity_id, Some(value));
    //     }
    // }
}
