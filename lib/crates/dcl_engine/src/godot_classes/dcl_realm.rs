use godot::prelude::*;

use crate::{godot_classes::dcl_global::DclGlobal, urls};

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclRealm {
    #[export]
    #[var(pub)]
    camera_mode: i32,

    #[var(pub)]
    realm_about: VarDictionary,
    #[var(pub)]
    pub realm_url: GString,
    #[var(pub)]
    realm_string: GString,

    // Mirror realm_about.get("configuration")
    #[var(pub)]
    realm_name: GString,
    #[var(pub)]
    network_id: i32,
    #[var(pub)]
    realm_scene_urns: Array<VarDictionary>,
    #[var(pub)]
    realm_global_scene_urns: Array<VarDictionary>,
    #[var(pub)]
    realm_city_loader_content_base_url: GString,

    lambda_server_base_url: GString,

    #[var(pub)]
    pub content_base_url: GString,

    #[var(pub)]
    realm_min_bounds: Vector2i,

    #[var(pub)]
    realm_max_bounds: Vector2i,

    /// Shared secret already validated against this world's comms endpoint (#2651).
    /// Empty for every other realm, which is what keeps `secret` off the wire.
    #[var(pub)]
    realm_credential: GString,

    has_realm: bool,

    _base: Base<Node>,
}

#[godot_api]
impl DclRealm {
    #[signal]
    pub fn realm_changed();
    #[signal]
    pub fn realm_changing();
    #[signal]
    pub fn realm_change_failed(new_realm_string: GString, reason: GString);
    /// The target world is private and the user is not on its allow-list (#1725). Kept apart
    /// from `realm_change_failed` so it surfaces the "is private" modal instead of the generic
    /// "World unavailable" toast.
    #[signal]
    pub fn realm_access_denied(new_realm_string: GString, world_name: GString);

    #[func]
    pub fn has_realm(&self) -> bool {
        self.has_realm
    }

    #[func]
    pub fn async_clear_realm(&mut self) {
        self.has_realm = false;
        self.realm_url = GString::new();
        self.realm_string = GString::new();
        self.realm_about = VarDictionary::new();
        self.realm_scene_urns.clear();
        self.realm_global_scene_urns.clear();
        self.realm_city_loader_content_base_url = GString::new();
        self.realm_name = GString::new();
        self.network_id = 0;
        self.content_base_url = GString::new();
        DclGlobal::singleton()
            .bind()
            .scene_runner
            .clone()
            .bind_mut()
            .kill_all_scenes();
    }

    #[func]
    pub fn get_profile_content_url(&self) -> GString {
        // Identity override (Option D testing): pin profile/wearable content to the
        // Profile-group env, decoupled from the connected realm. No-op normally.
        if let Some(url) = urls::identity_content() {
            return url.to_godot();
        }
        urls::peer_content().to_godot()
    }

    /// Content URL for profile deployment — uses the realm's own catalyst when available,
    /// falling back to the load-balancer URL. Intended to match Unity's behavior of
    /// deploying directly to the realm's specific node.
    #[func]
    pub fn get_profile_deployment_url(&self) -> GString {
        // Identity override (Option D testing): deploy the own profile to the
        // Profile-group env's catalyst, not the realm used for scenes.
        if let Some(url) = urls::identity_content() {
            return url.to_godot();
        }
        if self.content_base_url.is_empty() {
            urls::peer_content().to_godot()
        } else {
            self.content_base_url.clone()
        }
    }

    #[func]
    pub fn get_lambda_server_base_url(&self) -> GString {
        // Identity override (Option D testing): pin profile/names/identity lambdas
        // to the Profile-group env, decoupled from the connected realm. This also
        // makes livekit broadcast the same identity catalyst to other peers, so
        // they fetch this player's profile from the right place. No-op normally.
        if let Some(url) = urls::identity_lambdas() {
            return url.to_godot();
        }
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

impl DclRealm {
    /// Stores a validated realm. The urn rows are `{urn, entityId, baseUrl}`.
    pub(crate) fn commit(&mut self, commit: RealmData) {
        self.realm_string = commit.realm_string.as_str().into();
        self.realm_url = commit.realm_url.as_str().into();
        self.realm_about = commit.about;
        self.realm_scene_urns = commit.scene_urns;
        self.realm_global_scene_urns = commit.global_scene_urns;
        self.realm_city_loader_content_base_url =
            commit.city_loader_content_base_url.as_str().into();
        self.lambda_server_base_url = commit.lambda_server_base_url.as_str().into();
        self.realm_name = commit.realm_name.as_str().into();
        self.network_id = commit.network_id as i32;
        self.realm_min_bounds = commit.min_bounds;
        self.realm_max_bounds = commit.max_bounds;
        self.content_base_url = commit.content_base_url.as_str().into();
        self.has_realm = true;
    }

    pub(crate) fn set_credential(&mut self, credential: &str) {
        self.realm_credential = credential.into();
    }
}

pub(crate) struct RealmData {
    pub realm_string: String,
    pub realm_url: String,
    pub about: VarDictionary,
    pub scene_urns: Array<VarDictionary>,
    pub global_scene_urns: Array<VarDictionary>,
    pub city_loader_content_base_url: String,
    pub lambda_server_base_url: String,
    pub realm_name: String,
    pub network_id: i64,
    pub min_bounds: Vector2i,
    pub max_bounds: Vector2i,
    pub content_base_url: String,
}
