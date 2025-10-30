use crate::{
    dcl::{
        components::{proto_components::sdk::components::PbRealmInfo, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::Scene,
};

pub fn sync_realm_info(scene: &Scene, crdt_state: &mut SceneCrdtState) {
    // Throttle: run on first frame (tick 0) and every 15 frames after that
    if scene.tick_number != 0 && !scene.tick_number.is_multiple_of(15) {
        return;
    }

    let maybe_current_realm_info = SceneCrdtStateProtoComponents::get_realm_info(crdt_state)
        .get(&SceneEntityId::ROOT)
        .and_then(|realm_info_value| realm_info_value.value.clone());

    let dcl_global = DclGlobal::singleton();
    let realm = dcl_global.bind().realm.clone();
    let realm = realm.bind();
    let base_url = realm.get_realm_url().to_string();
    let realm_name = realm.get_realm_name().to_string();
    let network_id = realm.get_network_id();
    let is_preview = dcl_global.bind().get_preview_mode();

    let comms = dcl_global.bind().comms.clone();
    let comms = comms.bind();
    let comms_adapter = comms.get_current_adapter_conn_str().to_string();

    // Check if the scene room matches this scene
    let scene_room_id = comms.get_current_scene_room_id().to_string();
    let current_scene_id = scene.scene_entity_definition.id.clone();
    let is_scene_room_for_this_scene =
        !scene_room_id.is_empty() && scene_room_id == current_scene_id;

    let is_connected_scene_room =
        is_scene_room_for_this_scene && comms.is_connected_to_scene_room();

    let room = if is_connected_scene_room {
        Some(scene_room_id)
    } else {
        None
    };

    let new_realm_info = PbRealmInfo {
        base_url: base_url.clone(),
        realm_name: realm_name.clone(),
        network_id,
        comms_adapter: comms_adapter.clone(),
        is_preview,
        room,
        is_connected_scene_room: Some(is_connected_scene_room),
    };

    let should_update = match maybe_current_realm_info {
        Some(current) => {
            current.base_url != base_url
                || current.realm_name != realm_name
                || current.network_id != network_id
                || current.comms_adapter != comms_adapter
                || current.is_preview != is_preview
                || current.room != new_realm_info.room
                || current.is_connected_scene_room != Some(is_connected_scene_room)
        }
        None => true,
    };

    if should_update {
        SceneCrdtStateProtoComponents::get_realm_info_mut(crdt_state)
            .put(SceneEntityId::ROOT, Some(new_realm_info));
    }
}
