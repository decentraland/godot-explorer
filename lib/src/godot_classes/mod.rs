use godot::obj::NewGd;

pub mod animator_controller;
pub mod dcl_audio_source;
pub mod dcl_audio_stream;
pub mod dcl_avatar;
pub mod dcl_avatar_modifier_area_3d;
pub mod dcl_camera_3d;
pub mod dcl_camera_mode_area_3d;
pub mod dcl_cli;
pub mod dcl_config;
pub mod dcl_confirm_dialog;
pub mod dcl_ether;
pub mod dcl_global;
pub mod dcl_global_time;
pub mod dcl_gltf_container;
pub mod dcl_hashing;
pub mod dcl_ios_plugin;
pub mod dcl_android_plugin;
pub mod dcl_node_entity_3d;
pub mod dcl_realm;
#[cfg(feature = "use_resource_tracking")]
pub mod dcl_resource_tracker;
pub mod dcl_scene_node;
pub mod dcl_social_blacklist;
pub mod dcl_tokio_rpc;
pub mod dcl_ui_background;
pub mod dcl_ui_control;
pub mod dcl_ui_dropdown;
pub mod dcl_ui_input;
pub mod dcl_ui_text;
pub mod dcl_video_player;
pub mod font;
pub mod portables;
pub mod promise;
pub mod resource_locker;
pub mod rpc_sender;

pub trait JsonGodotClass
where
    Self: serde::Serialize + serde::de::DeserializeOwned,
{
    fn to_godot_from_json(&self) -> Result<godot::prelude::Variant, String> {
        let json_str = serde_json::to_string(&self).map_err(|e| e.to_string())?;
        let mut json_parser = godot::classes::Json::new_gd();
        if json_parser.parse(json_str.into()) == godot::engine::global::Error::OK {
            Ok(json_parser.get_data())
        } else {
            Err("godot json parse error".to_string())
        }
    }

    fn from_godot_to_json(value: godot::prelude::Variant) -> Result<Self, String> {
        let json_str = godot::engine::Json::stringify(value).to_string();
        json5::from_str(json_str.as_str()).map_err(|e| e.to_string())
    }
}
