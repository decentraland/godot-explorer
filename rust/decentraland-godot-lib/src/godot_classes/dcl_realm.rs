use godot::engine::Node;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclRealm {
    #[export]
    camera_mode: i32,

    #[var]
    realm_about: Dictionary,
    #[var]
    realm_url: GString,
    #[var]
    realm_string: GString,

    // Mirror realm_about.get("configuration")
    #[var]
    realm_name: GString,
    #[var]
    network_id: i32,
    #[var]
    realm_scene_urns: Array<Dictionary>,
    #[var]
    realm_global_scene_urns: Array<Dictionary>,
    #[var]
    realm_city_loader_content_base_url: GString,

    #[var]
    lambda_server_base_url: GString,

    #[var]
    content_base_url: GString,

    #[base]
    _base: Base<Node>,
}

#[godot_api]
impl DclRealm {}
