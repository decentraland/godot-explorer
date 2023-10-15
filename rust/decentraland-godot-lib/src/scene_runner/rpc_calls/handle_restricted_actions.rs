use crate::{common::rpc::RpcCall, scene_runner::scene::Scene};

use godot::prelude::{GodotString, Variant};

pub fn change_realm(
    scene: &Scene,
    RpcCall::ChangeRealm {
        to,
        message,
        response,
    }: &RpcCall,
) {
    if let Some(mut realm) = scene
        .godot_dcl_scene
        .root_node
        .get_node("/root/realm".into())
    {
        let message = if let Some(message) = message {
            Variant::from(GodotString::from(message))
        } else {
            Variant::nil()
        };
        let ret = realm.call(
            "scene_request_change_realm".into(),
            &[Variant::from(GodotString::from(to)), message],
        );

        let ret = ret.booleanize();
        if ret {
            response.send(Ok(()));
        } else {
            response.send(Err("User rejected to change realm".to_string()));
        }
    }
}
