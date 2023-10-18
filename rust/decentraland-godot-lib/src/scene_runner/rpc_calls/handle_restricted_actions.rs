use std::{cell::RefCell, rc::Rc};

use crate::{
    common::rpc::RpcCall, godot_classes::dcl_confirm_dialog::DclConfirmDialog,
    scene_runner::scene::Scene,
};

use godot::prelude::{GodotString, Variant};

pub fn change_realm(
    scene: &Scene,
    RpcCall::ChangeRealm {
        to,
        message,
        response,
    }: &RpcCall,
) {
    if let Some(confirm_dialog) = scene
        .godot_dcl_scene
        .root_node
        .get_node("/root/explorer/UI/ConfirmDialog".into())
    {
        let mut confirm_dialog = confirm_dialog.cast::<DclConfirmDialog>();

        // Show node :)
        confirm_dialog.show();

        let mut confirm_dialog = confirm_dialog.bind_mut();

        let description = format!(
            "The scene wants to move you to a new realm\nTo: `{}`\nScene message: {}",
            to.clone(),
            if let Some(message) = message {
                message
            } else {
                ""
            }
        );

        confirm_dialog.set_texts(
            "Change Realm",
            description.as_str(),
            "Let's go!",
            "No thanks",
        );

        if let Some(realm) = scene
            .godot_dcl_scene
            .root_node
            .get_node("/root/realm".into())
        {
            // clone data that is going to the callback
            let response_ok = response.clone();
            let realm = Rc::new(RefCell::new(realm));
            let to = to.clone();

            confirm_dialog.set_ok_callback(move || {
                realm.borrow_mut().call(
                    "set_realm".into(),
                    &[Variant::from(GodotString::from(to.clone()))],
                );
                response_ok.send(Ok(()));
            });
        }

        let response_reject = response.clone();
        confirm_dialog.set_reject_callback(move || {
            response_reject.send(Err("User rejected to change realm".to_string()));
        });
    } else {
        println!("Error: ConfirmDialog not found")
    }
}
