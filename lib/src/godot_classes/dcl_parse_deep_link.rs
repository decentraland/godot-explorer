use godot::prelude::*;
use url::Url;

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
}

#[godot_api]
impl IRefCounted for DclParseDeepLink {
    fn init(_base: Base<RefCounted>) -> Self {
        DclParseDeepLink {
            // Due to Option<Vector2i> isn't supported in godot-rust extension, we workaround it by seeting ::MAX as invalid location
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
        }
    }
}

#[godot_api]
impl DclParseDeepLink {
    #[func]
    pub fn parse_decentraland_link(url_str: GString) -> Gd<DclParseDeepLink> {
        let mut return_object = DclParseDeepLink {
            location: Vector2i::MAX,
            realm: GString::new(),
            preview: GString::new(),
            dynamic_scene_loading: false,
            params: VarDictionary::new(),
            signin_identity_id: GString::new(),
            dclenv: GString::new(),
            is_walletconnect_callback: false,
            saved_profile: GString::new(),
        };

        if url_str.is_empty() {
            return Gd::from_object(return_object);
        }

        let url_string = url_str.to_string();
        let url = match Url::parse(url_string.as_str()) {
            Ok(url) => url,
            Err(err) => {
                godot_error!("Deep link URL unparsed {} - {url_str}", err.to_string());
                return Gd::from_object(return_object);
            }
        };

        // Handle both decentraland:// scheme and https://mobile.dclexplorer.com URLs
        // For https URLs from mobile.dclexplorer.com, convert to decentraland:// format
        let url = if url.scheme() == "https" || url.scheme() == "http" {
            if let Some(host) = url.host_str() {
                if host == "mobile.dclexplorer.com" {
                    // Convert https://mobile.dclexplorer.com/open?params to decentraland://open?params
                    let path = url.path();
                    let query = url.query().map(|q| format!("?{}", q)).unwrap_or_default();
                    let decentraland_url = format!("decentraland:/{}{}", path, query);
                    match Url::parse(&decentraland_url) {
                        Ok(converted) => converted,
                        Err(_) => {
                            godot_error!(
                                "Failed to convert URL to decentraland scheme - {url_str}"
                            );
                            return Gd::from_object(return_object);
                        }
                    }
                } else {
                    godot_error!("Invalid host: expected 'mobile.dclexplorer.com' - {url_str}");
                    return Gd::from_object(return_object);
                }
            } else {
                godot_error!("No host in URL - {url_str}");
                return Gd::from_object(return_object);
            }
        } else if url.scheme() == "decentraland" {
            url
        } else {
            godot_error!("Invalid scheme: expected 'decentraland' or 'https' - {url_str}");
            return Gd::from_object(return_object);
        };

        // Check for WalletConnect callback (decentraland://walletconnect or decentraland://walletconnect/...)
        // This is a callback from wallet apps and should be ignored
        if let Some(host) = url.host_str() {
            if host == "walletconnect" {
                return_object.is_walletconnect_callback = true;
                return Gd::from_object(return_object);
            }
        }

        // Parse query parameters
        for (key, value) in url.query_pairs() {
            let _ = return_object
                .params
                .insert(key.to_string().to_variant(), value.to_string().to_variant());

            match key.as_ref() {
                "location" | "position" => {
                    // Parse location as "x,y"
                    let coords: Vec<&str> = value.split(',').collect();
                    if coords.len() == 2 {
                        if let (Ok(x), Ok(y)) = (coords[0].parse::<i32>(), coords[1].parse::<i32>())
                        {
                            return_object.location = Vector2i { x, y };
                        }
                    }
                }
                "realm" => {
                    return_object.realm = value.to_string().to_godot();
                }
                "signin" => {
                    // Handle signin identity ID from deep link `decentraland://open?signin=${identityId}`
                    return_object.signin_identity_id = value.to_string().to_godot();
                }
                "preview" => {
                    // Preview URL for hot reloading (e.g., http://192.168.0.55:8000)
                    return_object.preview = value.to_string().to_godot();
                }
                "dynamic-scene-loading" => {
                    // Dynamic scene loading mode - "true" or "1" enables it
                    return_object.dynamic_scene_loading =
                        value.eq_ignore_ascii_case("true") || value == "1";
                }
                "dclenv" => {
                    // Environment parameter - valid values: org, zone, today
                    let env_lower = value.to_lowercase();
                    if ["org", "zone", "today"].contains(&env_lower.as_str()) {
                        return_object.dclenv = GString::from(env_lower.as_str());
                    }
                }
                "saved-profile" => {
                    // Numbered profile slot for identity storage
                    if let Ok(n) = value.parse::<u32>() {
                        return_object.saved_profile = GString::from(&n.to_string());
                    }
                }
                _ => {}
            }
        }

        Gd::from_object(return_object)
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
