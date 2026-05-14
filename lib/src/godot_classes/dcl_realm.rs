use godot::prelude::*;

use crate::urls;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclRealm {
    #[export]
    camera_mode: i32,

    #[var]
    realm_about: VarDictionary,
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
    realm_scene_urns: Array<VarDictionary>,
    #[var]
    realm_global_scene_urns: Array<VarDictionary>,
    #[var]
    realm_city_loader_content_base_url: GString,

    lambda_server_base_url: GString,

    #[var]
    content_base_url: GString,

    #[var]
    realm_min_bounds: Vector2i,

    #[var]
    realm_max_bounds: Vector2i,

    _base: Base<Node>,
}

#[godot_api]
impl DclRealm {
    #[func]
    pub fn get_profile_content_url(&self) -> GString {
        urls::peer_content().to_godot()
    }

    /// Content URL for profile deployment — uses the realm's own catalyst when available,
    /// falling back to the load-balancer URL. Intended to match Unity's behavior of
    /// deploying directly to the realm's specific node.
    #[func]
    pub fn get_profile_deployment_url(&self) -> GString {
        if self.content_base_url.is_empty() {
            urls::peer_content().to_godot()
        } else {
            self.content_base_url.clone()
        }
    }

    #[func]
    pub fn get_lambda_server_base_url(&self) -> GString {
        if self.lambda_server_base_url.is_empty() {
            urls::peer_lambdas().to_godot()
        } else {
            self.lambda_server_base_url.clone()
        }
    }

    #[func]
    pub fn set_lambda_server_base_url(&mut self, new_value: GString) {
        self.lambda_server_base_url = new_value;
    }
}
