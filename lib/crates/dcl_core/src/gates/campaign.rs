//! Ad and referrer campaigns (#2670, #2669). A link or the install attribution carries an opaque
//! token; the mobile-bff serves the map from token to the target the install boots into. The
//! token is not the destination: a deep link carrying a position would skip the avatar creation
//! the first-run flow hangs off.
//!
//! Fail-open throughout: every failure resolves to "no campaign", which the shell renders as the
//! default first-run flow. Nothing is cached on disk, because a campaign is retired by deleting
//! it and a cached map could still resolve a token the server forgot.

use std::time::Duration;

use serde_json::{Map, Value};

use super::{fetch::Fetch, state::GatesState};
use crate::{
    app::{Destination, Parcel},
    realm,
};

pub const TIMEOUT: Duration = Duration::from_secs(5);

/// The Play install referrer survives 90 days and only changes on reinstall; without a bound a
/// reinstall months later would replay a long-dead campaign.
pub const TOKEN_MAX_AGE_SECONDS: i64 = 7 * 24 * 60 * 60;

const TOKEN_MAX_LENGTH: usize = 64;

/// At most four digits per coordinate, as the server's `campaigns_position_format` check.
const POSITION_ABS_MAX: i64 = 9999;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum CampaignKind {
    Genesis,
    World,
}

/// Why a launch fell back to the default first-run flow; shipped on the existing first-run
/// metrics so personalized and default can be compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum CampaignFallback {
    NoToken,
    UnknownToken,
    ExpiredToken,
    AlreadyConsumed,
    ResolverUnavailable,
    /// The campaign resolved but its target cannot be turned into a destination.
    UnusableTarget,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Campaign {
    pub token: String,
    pub kind: CampaignKind,
    /// Where to boot: the default realm and a parcel for Genesis, the world for a world.
    pub destination: Destination,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct CampaignResolution {
    /// The token this launch carries, empty when none.
    pub token: String,
    pub campaign: Option<Campaign>,
    /// `None` exactly when `campaign` is set.
    pub fallback: Option<CampaignFallback>,
}

impl CampaignResolution {
    fn fallback(token: &str, reason: CampaignFallback) -> Self {
        Self {
            token: token.to_string(),
            campaign: None,
            fallback: Some(reason),
        }
    }
}

/// `{"ok": true, "data": {"campaigns": {"<token>": {...}}}}` → the map; empty on any other shape.
pub fn parse_response(response: &Value) -> Map<String, Value> {
    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        return Map::new();
    }
    response
        .pointer("/data/campaigns")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Fetches the map once. `None` when the request failed or timed out, which is different from an
/// empty map: the resolver is unavailable, rather than the token unknown.
pub async fn fetch_map(fetch: &dyn Fetch, url: &str) -> Option<Map<String, Value>> {
    match tokio::time::timeout(TIMEOUT, fetch.get_json(url)).await {
        Ok(Ok(response)) => Some(parse_response(&response)),
        Ok(Err(e)) => {
            tracing::warn!("[Campaigns] fetch failed: {e}");
            None
        }
        Err(_) => {
            tracing::warn!("[Campaigns] timeout");
            None
        }
    }
}

fn coordinate(raw: &str) -> Option<i32> {
    // Strict: an unparseable value must not read as the valid parcel 0,0, which would burn the
    // token on the Genesis spawn.
    let value = raw.trim().parse::<i64>().ok()?;
    (-POSITION_ABS_MAX..=POSITION_ABS_MAX)
        .contains(&value)
        .then_some(value as i32)
}

/// The server's `^[a-z0-9]+(-[a-z0-9]+)*$`, at most 64 characters.
pub fn is_valid_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= TOKEN_MAX_LENGTH
        && !token.starts_with('-')
        && !token.ends_with('-')
        && !token.contains("--")
        && token
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The target of a campaign body as a destination, `None` when unusable.
pub fn target(token: &str, body: &Value) -> Option<Campaign> {
    let target = body.get("target").filter(|t| t.is_object())?;
    let text = |key: &str| target.get(key).and_then(Value::as_str).unwrap_or_default();
    match text("type") {
        "world" => {
            let name = text("name");
            // A name that is not a `.dcl.eth` would be read as a realm URL further down and never
            // reach the intended world.
            realm::is_dcl_ens(name).then(|| Campaign {
                token: token.to_string(),
                kind: CampaignKind::World,
                destination: Destination {
                    realm: Some(name.to_string()),
                    parcel: None,
                },
            })
        }
        "genesis" => {
            let parts: Vec<&str> = text("position").split(',').collect();
            let [x, y] = parts.as_slice() else {
                return None;
            };
            Some(Campaign {
                token: token.to_string(),
                kind: CampaignKind::Genesis,
                destination: Destination {
                    realm: None,
                    parcel: Some(Parcel::new(coordinate(x)?, coordinate(y)?)),
                },
            })
        }
        _ => None,
    }
}

/// The fallback that applies before the map is even needed, in the order they are checked.
pub fn precheck(state: &GatesState, now: i64) -> Result<(), CampaignFallback> {
    if state.campaign_token.is_empty() {
        return Err(CampaignFallback::NoToken);
    }
    if state.campaign_consumed {
        return Err(CampaignFallback::AlreadyConsumed);
    }
    let age = now - state.campaign_token_captured_at;
    if state.campaign_token_captured_at <= 0 || age > TOKEN_MAX_AGE_SECONDS {
        return Err(CampaignFallback::ExpiredToken);
    }
    Ok(())
}

/// Resolves the stored token against the map (`None` when the fetch failed).
pub fn resolve(map: Option<&Map<String, Value>>, state: &GatesState) -> CampaignResolution {
    let token = state.campaign_token.as_str();
    let Some(map) = map else {
        return CampaignResolution::fallback(token, CampaignFallback::ResolverUnavailable);
    };
    let Some(body) = map.get(token).filter(|body| body.is_object()) else {
        return CampaignResolution::fallback(token, CampaignFallback::UnknownToken);
    };
    match target(token, body) {
        Some(campaign) => CampaignResolution {
            token: token.to_string(),
            campaign: Some(campaign),
            fallback: None,
        },
        None => CampaignResolution::fallback(token, CampaignFallback::UnusableTarget),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::test_support::FakeFetch;
    use serde_json::json;

    fn genesis(position: &str) -> Value {
        json!({"target": {"type": "genesis", "position": position}})
    }

    #[test]
    fn tokens_are_lowercase_words_joined_by_single_hyphens() {
        for good in ["a", "summer-26", "a-b-c", "0"] {
            assert!(is_valid_token(good), "{good}");
        }
        for bad in [
            "",
            "-a",
            "a-",
            "a--b",
            "A",
            "a_b",
            "a b",
            "ä",
            &"a".repeat(65),
        ] {
            assert!(!is_valid_token(bad), "{bad}");
        }
        assert!(is_valid_token(&"a".repeat(64)));
    }

    #[test]
    fn the_response_envelope_is_strict_and_fails_open() {
        let ok = json!({"ok": true, "data": {"campaigns": {"summer-26": genesis("-9,-9")}}});
        assert!(parse_response(&ok).contains_key("summer-26"));
        for bad in [
            json!({"ok": false, "data": {"campaigns": {"x": {}}}}),
            json!({"ok": true}),
            json!({"ok": true, "data": {}}),
            json!({"ok": true, "data": []}),
            json!({"ok": true, "data": {"campaigns": []}}),
            json!(null),
            json!("nope"),
        ] {
            assert!(parse_response(&bad).is_empty(), "{bad}");
        }
    }

    #[test]
    fn genesis_targets_become_a_parcel_in_the_default_realm() {
        let campaign = target("t", &genesis(" -9 , 12 ")).unwrap();
        assert_eq!(campaign.kind, CampaignKind::Genesis);
        assert_eq!(
            campaign.destination,
            Destination {
                realm: None,
                parcel: Some(Parcel::new(-9, 12))
            }
        );
        assert!(target("t", &genesis("9999,-9999")).is_some());
    }

    #[test]
    fn junk_positions_never_read_as_the_parcel_zero() {
        for bad in [
            "abc,def",
            "10,def",
            "abc,20",
            "x=10,y=20",
            ",",
            " , ",
            "",
            "1,2,3",
            "5",
            "10000,0",
            "0,-10000",
            "4294967296,0",
            "9223372036854775808,0",
            "-9223372036854775808,0",
            "1.5,2",
        ] {
            assert!(target("t", &genesis(bad)).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn world_targets_need_a_dcl_eth_name() {
        let world = |name: &str| json!({"target": {"type": "world", "name": name}});
        let campaign = target("t", &world("foo.dcl.eth")).unwrap();
        assert_eq!(campaign.kind, CampaignKind::World);
        assert_eq!(
            campaign.destination,
            Destination {
                realm: Some("foo.dcl.eth".into()),
                parcel: None
            }
        );
        for bad in [
            "https://evil.example",
            "foo",
            "foo-bar.dcl.eth",
            ".dcl.eth",
            "",
        ] {
            assert!(target("t", &world(bad)).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn bodies_without_a_usable_target_are_unusable() {
        for bad in [
            json!({}),
            json!({"target": 3}),
            json!({"target": {"type": "mars"}}),
            json!({"target": {}}),
        ] {
            assert!(target("t", &bad).is_none(), "{bad}");
        }
    }

    fn state(token: &str, captured: i64, consumed: bool) -> GatesState {
        GatesState {
            campaign_token: token.into(),
            campaign_token_captured_at: captured,
            campaign_consumed: consumed,
            ..Default::default()
        }
    }

    #[test]
    fn prechecks_run_in_order_before_any_fetch() {
        let now = 10 * 86_400;
        let table = [
            (state("", now, false), Err(CampaignFallback::NoToken)),
            (state("", now, true), Err(CampaignFallback::NoToken)),
            (
                state("t", now, true),
                Err(CampaignFallback::AlreadyConsumed),
            ),
            (state("t", 0, false), Err(CampaignFallback::ExpiredToken)),
            (
                state("t", now - TOKEN_MAX_AGE_SECONDS - 1, false),
                Err(CampaignFallback::ExpiredToken),
            ),
            (state("t", now - TOKEN_MAX_AGE_SECONDS, false), Ok(())),
            (state("t", now - 5, false), Ok(())),
        ];
        for (state, expected) in table {
            assert_eq!(precheck(&state, now), expected, "{state:?}");
        }
    }

    #[test]
    fn resolution_distinguishes_unavailable_unknown_unusable_and_found() {
        let map = parse_response(&json!({"ok": true, "data": {"campaigns": {
            "good": genesis("1,2"),
            "bad": genesis("nope"),
        }}}));
        let fallback = |token: &str, map: Option<&Map<String, Value>>| {
            resolve(map, &state(token, 1, false)).fallback
        };
        assert_eq!(
            fallback("good", None),
            Some(CampaignFallback::ResolverUnavailable)
        );
        assert_eq!(
            fallback("gone", Some(&map)),
            Some(CampaignFallback::UnknownToken)
        );
        assert_eq!(
            fallback("bad", Some(&map)),
            Some(CampaignFallback::UnusableTarget)
        );

        let found = resolve(Some(&map), &state("good", 1, false));
        assert_eq!(found.fallback, None);
        assert_eq!(
            found.campaign.unwrap().destination.parcel,
            Some(Parcel::new(1, 2))
        );
    }

    #[tokio::test]
    async fn a_failed_fetch_is_not_an_empty_map() {
        let ok = FakeFetch::with_get(Ok(json!({"ok": true, "data": {"campaigns": {}}})));
        assert_eq!(fetch_map(&ok, "u").await, Some(Map::new()));
        let offline = FakeFetch::with_get(Err("offline".into()));
        assert_eq!(fetch_map(&offline, "u").await, None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_slow_fetch_is_unavailable() {
        assert_eq!(fetch_map(&FakeFetch::hanging(), "u").await, None);
    }
}
