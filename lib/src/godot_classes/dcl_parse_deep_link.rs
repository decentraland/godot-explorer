use godot::prelude::*;

use crate::deep_link;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclParseDeepLink {
    #[var]
    location: Vector2i,

    #[var]
    realm: GString,

    /// Preview URL for hot reloading (e.g., http://192.168.0.55:8000)
    /// When set, skips lobby and enables preview mode with WebSocket hot reload
    #[var]
    preview: GString,

    /// Dynamic scene loading mode (deep link param: dynamic-scene-loading=true)
    /// When true, uses continuous scene loading/unloading without terrain generation.
    /// Works for any realm including Genesis City.
    #[var]
    dynamic_scene_loading: bool,

    #[var]
    params: VarDictionary,

    /// The signin identity ID from deep link `decentraland://open?signin=${identityId}`
    #[var]
    signin_identity_id: GString,

    /// The environment parameter from deep link `decentraland://open?dclenv=zone`
    /// Valid values: "org", "zone", "today"
    #[var]
    dclenv: GString,

    /// True if this is a WalletConnect callback that should be ignored
    #[var]
    is_walletconnect_callback: bool,

    /// Numbered profile slot for identity storage (e.g., "2" uses account_2/guest_profile_2)
    #[var]
    saved_profile: GString,

    /// Enable LiveKit debug panel from deep link (livekit_debug=true)
    #[var]
    livekit_debug: bool,

    /// The URL path component (e.g., "/jump", "/events", "/places", "/mobile")
    #[var]
    path: GString,

    /// Scene logging target: empty=off, "true"=auto, "ws://host:port"=custom target
    #[var]
    scene_logging: GString,

    /// Whether to write JSONL scene log files to disk
    #[var]
    scene_logging_file: bool,
}

#[godot_api]
impl IRefCounted for DclParseDeepLink {
    fn init(_base: Base<RefCounted>) -> Self {
        Self::default_fields()
    }
}

impl DclParseDeepLink {
    fn default_fields() -> Self {
        DclParseDeepLink {
            // Due to Option<Vector2i> isn't supported in godot-rust extension, we workaround it by setting ::MAX as invalid location
            //  Check is_location_defined
            location: Vector2i::MAX,
            realm: GString::new(),
            preview: GString::new(),
            dynamic_scene_loading: false,
            params: VarDictionary::new(),
            signin_identity_id: GString::new(),
            is_walletconnect_callback: false,
            dclenv: GString::new(),
            saved_profile: GString::new(),
            livekit_debug: false,
            path: GString::new(),
            scene_logging: GString::new(),
            scene_logging_file: false,
        }
    }

    fn from_result(r: deep_link::DeepLinkResult) -> Self {
        let mut params = VarDictionary::new();
        for (k, v) in &r.params {
            let _ = params.insert(k.to_variant(), v.to_variant());
        }

        DclParseDeepLink {
            location: match r.location {
                Some((x, y)) => Vector2i { x, y },
                None => Vector2i::MAX,
            },
            realm: GString::from(&r.realm),
            preview: GString::from(&r.preview),
            dynamic_scene_loading: r.dynamic_scene_loading,
            params,
            signin_identity_id: GString::from(&r.signin_identity_id),
            dclenv: GString::from(&r.dclenv),
            is_walletconnect_callback: r.is_walletconnect_callback,
            saved_profile: GString::from(&r.saved_profile),
            livekit_debug: r.livekit_debug,
            path: GString::from(&r.path),
            scene_logging: GString::from(&r.scene_logging),
            scene_logging_file: r.scene_logging_file,
        }
    }
}

#[godot_api]
impl DclParseDeepLink {
    #[func]
    pub fn parse_decentraland_link(url_str: GString) -> Gd<DclParseDeepLink> {
        let url_string = url_str.to_string();
        let obj = match deep_link::parse_deep_link(&url_string) {
            Some(result) => Self::from_result(result),
            None => Self::default_fields(),
        };
        Gd::from_object(obj)
    }

    #[func]
    pub fn is_location_defined(&self) -> bool {
        self.location.x < 1000000
    }

    #[func]
    pub fn is_signin_request(&self) -> bool {
        !self.signin_identity_id.is_empty()
    }
}
