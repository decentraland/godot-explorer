use url::Url;

use crate::env::DclEnvConfig;

/// Pure-Rust result of parsing a Decentraland deep link.
/// Free of any Godot types so it can be unit-tested without the engine.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeepLinkResult {
    /// Parcel location (None when not specified)
    pub location: Option<(i32, i32)>,
    pub realm: String,
    /// Preview URL for hot reloading
    pub preview: String,
    /// Dynamic scene loading mode
    pub dynamic_scene_loading: bool,
    /// All query-string key/value pairs
    pub params: Vec<(String, String)>,
    /// Signin identity ID
    pub signin_identity_id: String,
    /// Environment override (e.g. "org", "zone", "today", or per-group)
    pub dclenv: String,
    /// WalletConnect callback flag
    pub is_walletconnect_callback: bool,
    /// Numbered profile slot
    pub saved_profile: String,
    /// LiveKit debug flag
    pub livekit_debug: bool,
    /// Routable path (e.g. "/jump", "/events", "/places", "/mobile")
    pub path: String,
    /// Scene logging target: empty=off, "true"=auto (use preview WS + debugger), "ws://host:port"=custom target
    pub scene_logging: String,
    /// Whether to write JSONL scene log files to disk
    pub scene_logging_file: bool,
}

impl DeepLinkResult {
    pub fn is_location_defined(&self) -> bool {
        self.location.is_some()
    }

    pub fn is_signin_request(&self) -> bool {
        !self.signin_identity_id.is_empty()
    }
}

/// Parse a Decentraland deep link URL into a [`DeepLinkResult`].
///
/// Accepted formats:
/// - `decentraland://events?id=X`  (native scheme)
/// - `https://decentraland.org/events?id=X`  (app link)
/// - `https://decentraland.zone/events?id=X` (app link, auto-infers dclenv=zone)
/// - `https://mobile.dclexplorer.com/open?location=X,Y` (legacy mobile)
///
/// Returns `None` only when `url_str` is empty.  Malformed URLs return a
/// default result (matching the previous Godot-only behaviour that logged
/// an error but still returned an object).
pub fn parse_deep_link(url_str: &str) -> Option<DeepLinkResult> {
    if url_str.is_empty() {
        return None;
    }

    let mut result = DeepLinkResult::default();

    let parsed = match Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return Some(result),
    };

    // --- Scheme normalisation ---------------------------------------------------
    let url = match parsed.scheme() {
        "https" | "http" => {
            let host = parsed.host_str()?;
            match host {
                "mobile.dclexplorer.com" | "decentraland.org" | "decentraland.zone" => {
                    // Infer dclenv from domain when not explicitly set
                    if host == "decentraland.zone"
                        && !parsed.query_pairs().any(|(k, _)| k == "dclenv")
                    {
                        result.dclenv = "zone".into();
                    }

                    // Capture path before scheme conversion
                    let path = parsed.path();
                    result.path = path.to_string();

                    let query = parsed
                        .query()
                        .map(|q| format!("?{}", q))
                        .unwrap_or_default();
                    let decentraland_url = format!("decentraland:/{}{}", path, query);
                    match Url::parse(&decentraland_url) {
                        Ok(converted) => converted,
                        Err(_) => return Some(result),
                    }
                }
                _ => return Some(result),
            }
        }
        "decentraland" => parsed,
        _ => return Some(result),
    };

    // --- WalletConnect check ----------------------------------------------------
    if let Some(host) = url.host_str() {
        if host == "walletconnect" {
            result.is_walletconnect_callback = true;
            return Some(result);
        }
    }

    // --- Path from native scheme ------------------------------------------------
    // For `decentraland://events?id=X` the route name is in the host field.
    if result.path.is_empty() {
        if let Some(host) = url.host_str() {
            result.path = format!("/{}", host);
        }
    }

    // --- Query parameters -------------------------------------------------------
    for (key, value) in url.query_pairs() {
        result.params.push((key.to_string(), value.to_string()));

        match key.as_ref() {
            "location" | "position" => {
                let coords: Vec<&str> = value.split(',').collect();
                if coords.len() == 2 {
                    if let (Ok(x), Ok(y)) = (coords[0].parse::<i32>(), coords[1].parse::<i32>()) {
                        result.location = Some((x, y));
                    }
                }
            }
            "realm" => result.realm = value.to_string(),
            "signin" => result.signin_identity_id = value.to_string(),
            "preview" => result.preview = value.to_string(),
            "dynamic-scene-loading" => {
                result.dynamic_scene_loading = value.eq_ignore_ascii_case("true") || value == "1";
            }
            "dclenv" => {
                if DclEnvConfig::parse(&value).is_some() {
                    result.dclenv = value.to_string();
                }
            }
            "saved-profile" => {
                if let Ok(n) = value.parse::<u32>() {
                    result.saved_profile = n.to_string();
                }
            }
            "livekit_debug" => {
                result.livekit_debug = value.eq_ignore_ascii_case("true") || value == "1";
            }
            "scene-logging" => {
                result.scene_logging = if value.eq_ignore_ascii_case("true") || value == "1" {
                    "true".to_string()
                } else {
                    value.to_string()
                };
            }
            "scene-logging-file" => {
                result.scene_logging_file = value.eq_ignore_ascii_case("true") || value == "1";
            }
            _ => {}
        }
    }

    Some(result)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Helpers ------------------------------------------------------------

    fn parse(url: &str) -> DeepLinkResult {
        parse_deep_link(url).expect("should return Some for non-empty input")
    }

    fn get_param<'a>(result: &'a DeepLinkResult, key: &str) -> Option<&'a str> {
        result
            .params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    // ---- Empty / malformed --------------------------------------------------

    #[test]
    fn empty_url_returns_none() {
        assert!(parse_deep_link("").is_none());
    }

    #[test]
    fn malformed_url_returns_default() {
        let r = parse("not a url at all");
        assert_eq!(r, DeepLinkResult::default());
    }

    // ---- Native scheme: equivalence -----------------------------------------

    #[test]
    fn native_events_path() {
        let r = parse("decentraland://events?id=abc123");
        assert_eq!(r.path, "/events");
        assert_eq!(get_param(&r, "id"), Some("abc123"));
    }

    #[test]
    fn native_places_path() {
        let r = parse("decentraland://places?id=place456");
        assert_eq!(r.path, "/places");
        assert_eq!(get_param(&r, "id"), Some("place456"));
    }

    #[test]
    fn native_jump_path() {
        let r = parse("decentraland://jump");
        assert_eq!(r.path, "/jump");
        assert!(r.location.is_none());
        assert!(r.realm.is_empty());
    }

    #[test]
    fn native_jump_with_location() {
        let r = parse("decentraland://jump?location=10,20");
        assert_eq!(r.path, "/jump");
        assert_eq!(r.location, Some((10, 20)));
    }

    #[test]
    fn native_jump_with_location_and_realm() {
        let r = parse("decentraland://jump?location=-50,30&realm=my-realm.dcl.eth");
        assert_eq!(r.path, "/jump");
        assert_eq!(r.location, Some((-50, 30)));
        assert_eq!(r.realm, "my-realm.dcl.eth");
    }

    #[test]
    fn native_jump_with_realm_only() {
        let r = parse("decentraland://jump?realm=some-realm");
        assert_eq!(r.path, "/jump");
        assert!(r.location.is_none());
        assert_eq!(r.realm, "some-realm");
    }

    #[test]
    fn native_open_with_location() {
        let r = parse("decentraland://open?location=10,20");
        assert_eq!(r.path, "/open");
        assert_eq!(r.location, Some((10, 20)));
    }

    #[test]
    fn native_mobile_with_location() {
        let r = parse("decentraland://mobile?location=-5,30");
        assert_eq!(r.path, "/mobile");
        assert_eq!(r.location, Some((-5, 30)));
    }

    // ---- HTTPS app links: decentraland.org ----------------------------------

    #[test]
    fn https_org_events() {
        let r = parse("https://decentraland.org/events?id=evt789");
        assert_eq!(r.path, "/events");
        assert_eq!(get_param(&r, "id"), Some("evt789"));
        assert!(r.dclenv.is_empty(), "org should not set dclenv");
    }

    #[test]
    fn https_org_places() {
        let r = parse("https://decentraland.org/places?id=plc000");
        assert_eq!(r.path, "/places");
        assert_eq!(get_param(&r, "id"), Some("plc000"));
    }

    #[test]
    fn https_org_jump() {
        let r = parse("https://decentraland.org/jump");
        assert_eq!(r.path, "/jump");
    }

    #[test]
    fn https_org_open_location() {
        let r = parse("https://decentraland.org/open?location=100,-50&realm=my-realm");
        assert_eq!(r.path, "/open");
        assert_eq!(r.location, Some((100, -50)));
        assert_eq!(r.realm, "my-realm");
    }

    // ---- HTTPS app links: decentraland.zone ---------------------------------

    #[test]
    fn https_zone_infers_dclenv() {
        let r = parse("https://decentraland.zone/events?id=z1");
        assert_eq!(r.path, "/events");
        assert_eq!(r.dclenv, "zone");
    }

    #[test]
    fn https_zone_explicit_dclenv_overrides() {
        let r = parse("https://decentraland.zone/open?dclenv=org");
        assert_eq!(
            r.dclenv, "org",
            "explicit dclenv should win over zone inference"
        );
    }

    #[test]
    fn https_zone_jump() {
        let r = parse("https://decentraland.zone/jump");
        assert_eq!(r.path, "/jump");
        assert_eq!(r.dclenv, "zone");
    }

    // ---- HTTPS: mobile.dclexplorer.com --------------------------------------

    #[test]
    fn https_mobile_dclexplorer() {
        let r = parse("https://mobile.dclexplorer.com/open?location=5,5&realm=r1");
        assert_eq!(r.path, "/open");
        assert_eq!(r.location, Some((5, 5)));
        assert_eq!(r.realm, "r1");
    }

    // ---- Equivalence across formats -----------------------------------------

    #[test]
    fn native_and_org_events_are_equivalent() {
        let native = parse("decentraland://events?id=abc");
        let https = parse("https://decentraland.org/events?id=abc");

        assert_eq!(native.path, https.path);
        assert_eq!(get_param(&native, "id"), get_param(&https, "id"));
        assert_eq!(native.location, https.location);
        assert_eq!(native.realm, https.realm);
    }

    #[test]
    fn native_and_org_places_are_equivalent() {
        let native = parse("decentraland://places?id=p1");
        let https = parse("https://decentraland.org/places?id=p1");

        assert_eq!(native.path, https.path);
        assert_eq!(get_param(&native, "id"), get_param(&https, "id"));
    }

    #[test]
    fn native_and_org_jump_are_equivalent() {
        let native = parse("decentraland://jump");
        let https = parse("https://decentraland.org/jump");

        assert_eq!(native.path, https.path);
    }

    #[test]
    fn native_and_org_jump_with_params_are_equivalent() {
        let native = parse("decentraland://jump?location=10,20&realm=r1");
        let org = parse("https://decentraland.org/jump?location=10,20&realm=r1");
        let zone = parse("https://decentraland.zone/jump?location=10,20&realm=r1");
        let mobile = parse("https://mobile.dclexplorer.com/jump?location=10,20&realm=r1");

        for (label, r) in [("org", &org), ("zone", &zone), ("mobile", &mobile)] {
            assert_eq!(native.path, r.path, "{label}: path mismatch");
            assert_eq!(native.location, r.location, "{label}: location mismatch");
            assert_eq!(native.realm, r.realm, "{label}: realm mismatch");
        }
    }

    #[test]
    fn native_and_org_open_location_are_equivalent() {
        let native = parse("decentraland://open?location=10,20&realm=r1");
        let https = parse("https://decentraland.org/open?location=10,20&realm=r1");

        assert_eq!(native.path, https.path);
        assert_eq!(native.location, https.location);
        assert_eq!(native.realm, https.realm);
    }

    // ---- WalletConnect ------------------------------------------------------

    #[test]
    fn walletconnect_callback_is_flagged() {
        let r = parse("decentraland://walletconnect");
        assert!(r.is_walletconnect_callback);
    }

    #[test]
    fn walletconnect_with_path_is_flagged() {
        let r = parse("decentraland://walletconnect/some/path");
        assert!(r.is_walletconnect_callback);
    }

    // ---- Special parameters -------------------------------------------------

    #[test]
    fn signin_identity() {
        let r = parse("decentraland://open?signin=id123");
        assert!(r.is_signin_request());
        assert_eq!(r.signin_identity_id, "id123");
    }

    #[test]
    fn preview_url() {
        let r = parse("decentraland://open?preview=http://192.168.1.1:8000");
        assert_eq!(r.preview, "http://192.168.1.1:8000");
    }

    #[test]
    fn dynamic_scene_loading_true() {
        let r = parse("decentraland://open?dynamic-scene-loading=true");
        assert!(r.dynamic_scene_loading);
    }

    #[test]
    fn dynamic_scene_loading_one() {
        let r = parse("decentraland://open?dynamic-scene-loading=1");
        assert!(r.dynamic_scene_loading);
    }

    #[test]
    fn saved_profile() {
        let r = parse("decentraland://open?saved-profile=2");
        assert_eq!(r.saved_profile, "2");
    }

    #[test]
    fn saved_profile_invalid_ignored() {
        let r = parse("decentraland://open?saved-profile=abc");
        assert!(r.saved_profile.is_empty());
    }

    #[test]
    fn livekit_debug() {
        let r = parse("decentraland://open?livekit_debug=true");
        assert!(r.livekit_debug);
    }

    #[test]
    fn rust_log_param() {
        let r = parse("decentraland://open?rust-log=debug");
        assert_eq!(get_param(&r, "rust-log"), Some("debug"));
    }

    // ---- Edge cases ---------------------------------------------------------

    #[test]
    fn location_zero_zero() {
        let r = parse("decentraland://open?location=0,0");
        assert_eq!(r.location, Some((0, 0)));
    }

    #[test]
    fn position_is_alias_for_location() {
        let r = parse("decentraland://open?position=42,-10");
        assert_eq!(r.location, Some((42, -10)));
    }

    #[test]
    fn position_and_location_are_equivalent() {
        let loc = parse("decentraland://jump?location=10,20");
        let pos = parse("decentraland://jump?position=10,20");
        assert_eq!(loc.location, pos.location);
    }

    #[test]
    fn negative_coordinates() {
        let r = parse("decentraland://open?location=-100,-200");
        assert_eq!(r.location, Some((-100, -200)));
    }

    #[test]
    fn unknown_https_host_returns_default() {
        let r = parse("https://evil.com/events?id=hack");
        assert!(r.path.is_empty());
        assert!(r.params.is_empty());
    }

    #[test]
    fn unknown_scheme_returns_default() {
        let r = parse("ftp://decentraland.org/events");
        assert!(r.path.is_empty());
    }

    #[test]
    fn realm_with_custom_value() {
        let r = parse("decentraland://open?location=-140,-87&realm=fractilians.dcl.eth");
        assert_eq!(r.realm, "fractilians.dcl.eth");
        assert_eq!(r.location, Some((-140, -87)));
    }

    #[test]
    fn multiple_params_preserved() {
        let r = parse("decentraland://open?location=1,2&realm=r1&rust-log=debug&livekit_debug=1");
        assert_eq!(r.location, Some((1, 2)));
        assert_eq!(r.realm, "r1");
        assert!(r.livekit_debug);
        assert_eq!(get_param(&r, "rust-log"), Some("debug"));
    }

    // ---- Scene logging parameters -------------------------------------------

    #[test]
    fn scene_logging_true() {
        let r = parse("decentraland://open?scene-logging=true");
        assert_eq!(r.scene_logging, "true");
    }

    #[test]
    fn scene_logging_one() {
        let r = parse("decentraland://open?scene-logging=1");
        assert_eq!(r.scene_logging, "true");
    }

    #[test]
    fn scene_logging_custom_ws() {
        let r = parse("decentraland://open?scene-logging=ws://192.168.1.5:9090");
        assert_eq!(r.scene_logging, "ws://192.168.1.5:9090");
    }

    #[test]
    fn scene_logging_file() {
        let r = parse("decentraland://open?scene-logging=true&scene-logging-file=true");
        assert_eq!(r.scene_logging, "true");
        assert!(r.scene_logging_file);
    }

    #[test]
    fn scene_logging_default_off() {
        let r = parse("decentraland://open?location=0,0");
        assert!(r.scene_logging.is_empty());
        assert!(!r.scene_logging_file);
    }
}
