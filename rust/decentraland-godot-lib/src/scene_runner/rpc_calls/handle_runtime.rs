use crate::{
    dcl::scene_apis::{GetRealmResponse, RpcResultSender},
    godot_classes::dcl_global::DclGlobal,
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
