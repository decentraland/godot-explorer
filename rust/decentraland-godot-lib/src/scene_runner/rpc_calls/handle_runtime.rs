use godot::builtin::{meta::FromGodot, Dictionary};

use crate::{
    dcl::scene_apis::{
        ContentMapping, GetRealmResponse, GetSceneInformationResponse, RpcResultSender,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::Scene,
};

pub fn get_realm(response: &RpcResultSender<GetRealmResponse>) {
    let realm = DclGlobal::singleton().bind().realm.clone();
    let realm = realm.bind();
    let realm_name = realm.get_realm_name().to_string();
    let base_url = realm.get_realm_url().to_string();
    let network_id = realm.get_network_id();

    let is_preview = DclGlobal::singleton().bind().get_preview_mode();

    let comms_adapter = DclGlobal::singleton()
        .bind()
        .comms
        .bind()
        .get_current_adapter_conn_str()
        .to_string();

    response.send(GetRealmResponse {
        base_url,
        realm_name,
        network_id,
        comms_adapter,
        is_preview,
    })
}

pub fn get_scene_information(
    scene: &Scene,
    response: &RpcResultSender<GetSceneInformationResponse>,
) {
    let base_url = scene.content_mapping.get("base_url").unwrap().to_string();

    let content_dictionary =
        Dictionary::from_variant(&scene.content_mapping.get("content").unwrap());
    let content: Vec<ContentMapping> = content_dictionary
        .iter_shared()
        .map(|(file_name, file_hash)| ContentMapping {
            file: file_name.to_string(),
            hash: file_hash.to_string(),
        })
        .collect();

    response.send(GetSceneInformationResponse {
        urn: scene.definition.entity_id.clone(),
        content,
        metadata_json: scene.definition.metadata.clone(),
        base_url,
    })
}
