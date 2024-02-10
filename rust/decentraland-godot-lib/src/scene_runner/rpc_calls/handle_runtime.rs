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
    let content: Vec<ContentMapping> = scene
        .content_mapping
        .content
        .iter()
        .map(|(file, hash)| ContentMapping {
            file: file.clone(),
            hash: hash.clone(),
        })
        .collect();

    response.send(GetSceneInformationResponse {
        urn: scene.scene_entity_definition.id.clone(),
        content,
        metadata_json: "".into(), // TODO scene.definition.metadata.clone(),
        base_url: scene.content_mapping.base_url.clone(),
    })
}
