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
    realm_url: GodotString,
    #[var]
    realm_string: GodotString,

    // Mirror realm_about.get("configuration")
    #[var]
    realm_name: GodotString,
    #[var]
    network_id: i32,
    #[var]
    realm_scene_urns: Array<Dictionary>,
    #[var]
    realm_global_scene_urns: Array<Dictionary>,
    #[var]
    realm_city_loader_content_base_url: GodotString,

    #[var]
    content_base_url: GodotString,

    #[base]
    _base: Base<Node>,
}

#[godot_api]
impl DclRealm {}
