//! Routing of a parsed deep link: what to navigate to and which side effects the shell, the
//! engine or the settings must apply. Pure: the caller executes the result.

use std::sync::Arc;
use std::time::Instant;

use crate::app::{Destination, Parcel};
use crate::deep_link::{parse_deep_link, DeepLinkResult};
use crate::realm::is_dcl_ens;

/// How long a parked `?signin=` token stays redeemable: the boot it arrived on, no more.
pub const PENDING_SIGNIN_MAX_AGE_MS: u64 = 5 * 60 * 1000;

const CAMPAIGN_TOKEN_MAX_LENGTH: usize = 64;
const DECENTRALAND_DOMAINS: [&str; 4] = [
    "decentraland.org",
    "decentraland.zone",
    "decentraland.today",
    "dclregenesislabs.xyz",
];

/// Monotonic milliseconds; injectable so the sign-in cap is testable.
pub trait MonotonicClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock(Instant);

impl SystemClock {
    pub fn new() -> Self {
        Self(Instant::now())
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for SystemClock {
    fn now_ms(&self) -> u64 {
        self.0.elapsed().as_millis() as u64
    }
}

/// Whether the link launched the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum StartKind {
    Cold,
    Warm,
}

impl StartKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Warm => "warm",
        }
    }
}

/// The mobile sign-in state the router decides on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AuthSnapshot {
    pub sign_in_cancelled: bool,
    pub pending_mobile_auth: bool,
    pub wallet_connected: bool,
}

/// Process state a routing decision depends on.
#[derive(Clone, Debug, Default)]
pub struct RouteContext {
    /// A real phone: navigation is ignored on desktop and in emulation.
    pub real_mobile: bool,
    /// A build cut from `release*`.
    pub production: bool,
    pub current_dclenv: String,
    /// The environment was chosen by `--dclenv` or an earlier deep link.
    pub dclenv_explicit: bool,
    pub marketplace_awaiting_return: bool,
    pub auth: AuthSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ParamPair {
    pub key: String,
    pub value: String,
}

/// A setting a link carries, for the owner of that setting to apply.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ConfigParam {
    RustLog {
        filter: String,
    },
    PulseServer {
        server: String,
    },
    PulseRealm {
        realm: String,
    },
    PulseEnabled {
        enabled: bool,
    },
    DualChannel {
        enabled: bool,
    },
    LivekitEnabled {
        enabled: bool,
    },
    MultiplayerDebug,
    /// Already validated: https on a Decentraland domain or a local network, non-production.
    OptimizedContentBaseUrl {
        url: String,
    },
    RotateGuest,
    /// Already validated; stored first-write-wins by the owner of the install attribution.
    CampaignToken {
        token: String,
    },
    SkipGltf {
        enabled: bool,
    },
    KillSky {
        enabled: bool,
    },
    TouchFeedback {
        enabled: bool,
    },
    EnableUpgradedDeletion,
    GpBenchmark,
    SafeMarginDebug,
    /// Non-production: label every visible HUD control on the next scene entry.
    ControlsFtueShow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum AnalyticsEvent {
    PushOpened {
        campaign_id: String,
        push_id: String,
        start_kind: StartKind,
    },
    /// Every non-empty link, for the `App Opened` attribution.
    DeepLinkReceived { params: Vec<ParamPair> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Effect {
    Config {
        param: ConfigParam,
    },
    Analytics {
        event: AnalyticsEvent,
    },
    /// Point the preview WebSocket (hot reload) at this server.
    SetPreviewUrl {
        url: String,
    },
    /// The in-app marketplace webview handed control back.
    MarketplaceReturn,
    /// QA harness: the review-prompt coordinator reads its own params.
    CaptureReviewPromptParams {
        params: Vec<ParamPair>,
    },
    StartAvatarImpostorBenchmark,
    /// Sign out and restart in this environment.
    SwitchEnvironment {
        dclenv: String,
    },
    /// Warm resume: this process opened the browser; complete the connect now.
    CompleteMobileSignIn {
        identity_id: String,
    },
    /// Cold start: the token is parked for [`DeepLinkRouter::take_pending_signin_identity_id`].
    SignInParked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Navigation {
    /// `realm: None` is the default realm.
    Teleport {
        realm: Option<String>,
        parcel: Parcel,
    },
    /// A world (`name.dcl.eth`) without a position: enter at its spawn.
    JoinWorld {
        realm: String,
    },
    OpenJumpPanel,
    OpenEvent {
        id: String,
    },
    OpenPlace {
        id: String,
    },
    OpenDiscover,
}

impl Navigation {
    /// The world entry the app state machine performs, if this navigation enters one.
    pub fn destination(&self) -> Option<Destination> {
        match self {
            Self::Teleport { realm, parcel } => Some(Destination {
                realm: realm.clone(),
                parcel: Some(*parcel),
            }),
            Self::JoinWorld { realm } => Some(Destination {
                realm: Some(realm.clone()),
                parcel: None,
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Routed {
    pub navigation: Option<Navigation>,
    pub effects: Vec<Effect>,
}

struct Parked {
    identity_id: String,
    at_ms: u64,
}

pub struct DeepLinkRouter {
    clock: Arc<dyn MonotonicClock>,
    parked: Option<Parked>,
}

impl DeepLinkRouter {
    pub fn new(clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            clock,
            parked: None,
        }
    }

    /// Parse and route one link.
    pub fn handle(&mut self, url: &str, start_kind: StartKind, ctx: &RouteContext) -> Routed {
        match parse_deep_link(url) {
            Some(link) => self.route(&link, start_kind, ctx),
            None => Routed::default(),
        }
    }

    pub fn route(
        &mut self,
        link: &DeepLinkResult,
        start_kind: StartKind,
        ctx: &RouteContext,
    ) -> Routed {
        let mut effects = process_effects(link, start_kind, ctx);

        let swallowed = marketplace_return(link, ctx, &mut effects)
            || benchmark_requested(link, &mut effects)
            || link.is_walletconnect_callback
            || environment_change(link, ctx, &mut effects)
            || self.signin(link, ctx, &mut effects);
        if swallowed || !ctx.real_mobile {
            return Routed {
                navigation: None,
                effects,
            };
        }
        let navigation = navigate(link, &mut effects);
        Routed {
            navigation,
            effects,
        }
    }

    /// Consume a parked sign-in token. Single-shot; `None` when absent or older than
    /// [`PENDING_SIGNIN_MAX_AGE_MS`].
    pub fn take_pending_signin_identity_id(&mut self) -> Option<String> {
        let parked = self.parked.take()?;
        let age = self.clock.now_ms().saturating_sub(parked.at_ms);
        (age <= PENDING_SIGNIN_MAX_AGE_MS).then_some(parked.identity_id)
    }

    fn signin(
        &mut self,
        link: &DeepLinkResult,
        ctx: &RouteContext,
        effects: &mut Vec<Effect>,
    ) -> bool {
        if !link.is_signin_request() {
            return false;
        }
        let id = link.signin_identity_id.clone();
        let auth = ctx.auth;
        if auth.sign_in_cancelled {
            // The user cancelled; a late token must not sign them in.
        } else if auth.pending_mobile_auth {
            effects.push(Effect::CompleteMobileSignIn { identity_id: id });
        } else if auth.wallet_connected {
            // Parking would sign the user back in as the link's owner after a sign-out.
        } else {
            self.parked = Some(Parked {
                identity_id: id,
                at_ms: self.clock.now_ms(),
            });
            effects.push(Effect::SignInParked);
        }
        true
    }
}

/// The last value wins, as in a Godot `Dictionary` built from the query.
fn param<'a>(link: &'a DeepLinkResult, key: &str) -> &'a str {
    link.params
        .iter()
        .rfind(|(k, _)| k == key)
        .map_or("", |(_, v)| v.as_str())
}

fn truthy(value: &str) -> bool {
    matches!(value.to_lowercase().as_str(), "true" | "1" | "yes")
}

fn pairs(link: &DeepLinkResult) -> Vec<ParamPair> {
    link.params
        .iter()
        .map(|(key, value)| ParamPair {
            key: key.clone(),
            value: value.clone(),
        })
        .collect()
}

fn cfg(param: ConfigParam) -> Effect {
    Effect::Config { param }
}

fn process_effects(
    link: &DeepLinkResult,
    start_kind: StartKind,
    ctx: &RouteContext,
) -> Vec<Effect> {
    let mut out = Vec::new();

    let text = |key: &str| Some(param(link, key)).filter(|v| !v.is_empty());
    let flag = |key: &str| text(key).map(truthy);

    if let Some(filter) = text("rust-log") {
        out.push(cfg(ConfigParam::RustLog {
            filter: filter.into(),
        }));
    }
    if let Some(server) = text("pulse-server") {
        out.push(cfg(ConfigParam::PulseServer {
            server: server.into(),
        }));
    }
    if let Some(realm) = text("pulse-realm") {
        out.push(cfg(ConfigParam::PulseRealm {
            realm: realm.into(),
        }));
    }
    if let Some(enabled) = flag("pulse") {
        out.push(cfg(ConfigParam::PulseEnabled { enabled }));
    }
    if let Some(enabled) = flag("dual-channel") {
        out.push(cfg(ConfigParam::DualChannel { enabled }));
    }
    if let Some(enabled) = flag("livekit") {
        out.push(cfg(ConfigParam::LivekitEnabled { enabled }));
    }
    if link.multiplayer_debug {
        out.push(cfg(ConfigParam::MultiplayerDebug));
    }
    if let Some(url) = text("optimized-content-base-url") {
        if !ctx.production && (is_decentraland_https(url) || is_local_network(url)) {
            out.push(cfg(ConfigParam::OptimizedContentBaseUrl {
                url: url.into(),
            }));
        }
    }
    if !ctx.production && param(link, "rotate-guest").eq_ignore_ascii_case("true") {
        out.push(cfg(ConfigParam::RotateGuest));
    }
    if !ctx.production && param(link, "controls-ftue") == "show" {
        out.push(cfg(ConfigParam::ControlsFtueShow));
    }
    let token = param(link, "c").trim();
    if is_valid_campaign_token(token) {
        out.push(cfg(ConfigParam::CampaignToken {
            token: token.into(),
        }));
    }

    let campaign_id = param(link, "push_campaign_id").trim();
    if !campaign_id.is_empty() {
        out.push(Effect::Analytics {
            event: AnalyticsEvent::PushOpened {
                campaign_id: campaign_id.into(),
                push_id: param(link, "push_id").trim().into(),
                start_kind,
            },
        });
    }
    out.push(Effect::Analytics {
        event: AnalyticsEvent::DeepLinkReceived {
            params: pairs(link),
        },
    });

    if let Some(enabled) = flag("skip-gltf") {
        out.push(cfg(ConfigParam::SkipGltf { enabled }));
    }
    if let Some(enabled) = flag("kill-sky") {
        out.push(cfg(ConfigParam::KillSky { enabled }));
    }
    if let Some(enabled) = flag("touch-feedback") {
        out.push(cfg(ConfigParam::TouchFeedback { enabled }));
    }
    if flag("enable-upgraded-deletion") == Some(true) {
        out.push(cfg(ConfigParam::EnableUpgradedDeletion));
    }
    if link.gp_benchmark {
        out.push(cfg(ConfigParam::GpBenchmark));
    }
    if link.safe_margin_debug {
        out.push(cfg(ConfigParam::SafeMarginDebug));
    }
    out.push(Effect::CaptureReviewPromptParams {
        params: pairs(link),
    });
    out
}

/// True when the link was the marketplace handing control back, which nothing else may route.
fn marketplace_return(
    link: &DeepLinkResult,
    ctx: &RouteContext,
    effects: &mut Vec<Effect>,
) -> bool {
    let is_return = !param(link, "urn").is_empty() || !param(link, "iap_enabled").is_empty();
    if is_return || ctx.marketplace_awaiting_return {
        effects.push(Effect::MarketplaceReturn);
    }
    is_return
}

fn benchmark_requested(link: &DeepLinkResult, effects: &mut Vec<Effect>) -> bool {
    let requested = param(link, "benchmark") == "avatar-impostors";
    if requested {
        effects.push(Effect::StartAvatarImpostorBenchmark);
    }
    requested
}

/// A sign-in callback echoes the environment it authenticated against; it never asks to switch.
fn environment_change(
    link: &DeepLinkResult,
    ctx: &RouteContext,
    effects: &mut Vec<Effect>,
) -> bool {
    let wanted = &link.dclenv;
    if wanted.is_empty()
        || *wanted == ctx.current_dclenv
        || link.is_signin_request()
        || ctx.dclenv_explicit
    {
        return false;
    }
    effects.push(Effect::SwitchEnvironment {
        dclenv: wanted.clone(),
    });
    true
}

fn has_target(link: &DeepLinkResult) -> bool {
    link.is_location_defined() || !link.realm.is_empty() || !link.preview.is_empty()
}

fn navigate(link: &DeepLinkResult, effects: &mut Vec<Effect>) -> Option<Navigation> {
    let id = param(link, "id");
    let by_id = |make: fn(String) -> Navigation| (!id.is_empty()).then(|| make(id.to_string()));
    match link.section.as_str() {
        "jump" | "open" => {
            if has_target(link) {
                teleport(link, effects)
            } else {
                link.params.is_empty().then_some(Navigation::OpenJumpPanel)
            }
        }
        "events" => by_id(|id| Navigation::OpenEvent { id }).or_else(|| {
            if has_target(link) {
                teleport(link, effects)
            } else {
                Some(Navigation::OpenDiscover)
            }
        }),
        "places" => by_id(|id| Navigation::OpenPlace { id }).or_else(|| {
            if has_target(link) {
                teleport(link, effects)
            } else {
                Some(Navigation::OpenDiscover)
            }
        }),
        _ => teleport(link, effects),
    }
}

fn teleport(link: &DeepLinkResult, effects: &mut Vec<Effect>) -> Option<Navigation> {
    let realm = if link.preview.is_empty() {
        link.realm.clone()
    } else {
        effects.push(Effect::SetPreviewUrl {
            url: link.preview.clone(),
        });
        link.preview.clone()
    };
    match link.location {
        None if is_dcl_ens(&realm) => Some(Navigation::JoinWorld { realm }),
        None if realm.is_empty() => None,
        None => Some(Navigation::Teleport {
            realm: Some(realm),
            parcel: Parcel::new(0, 0),
        }),
        Some((x, y)) => Some(Navigation::Teleport {
            realm: (!realm.is_empty()).then_some(realm),
            parcel: Parcel::new(x, y),
        }),
    }
}

/// `[a-z0-9]+(-[a-z0-9]+)*`, at most 64 characters.
fn is_valid_campaign_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= CAMPAIGN_TOKEN_MAX_LENGTH
        && !token.starts_with('-')
        && !token.ends_with('-')
        && !token.contains("--")
        && token
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Lower-cased host of a plain `scheme://host[:port][/path]` URL; `None` for anything looser
/// (userinfo, query, fragment).
fn url_host(url: &str) -> Option<String> {
    let lower = url.to_lowercase();
    let rest = ["wss://", "ws://", "https://", "http://"]
        .iter()
        .find_map(|scheme| lower.strip_prefix(scheme))?;
    let (authority, path) = rest.find('/').map_or((rest, ""), |i| rest.split_at(i));
    let path_ok = path
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._~%!$&'()*+,;=:/-".contains(c));
    let (host, port) = if let Some(inner) = authority.strip_prefix('[') {
        let (inner, after) = inner.split_once(']')?;
        let hex = !inner.is_empty() && inner.chars().all(|c| c.is_ascii_hexdigit() || c == ':');
        (hex.then_some(inner)?, after.strip_prefix(':'))
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    let host_ok = host.starts_with('[')
        || (!host.is_empty()
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':'));
    let port_ok =
        port.is_none_or(|p| (1..=5).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()));
    (path_ok && host_ok && port_ok).then(|| host.to_string())
}

pub fn is_decentraland_https(url: &str) -> bool {
    if !url.to_lowercase().starts_with("https://") {
        return false;
    }
    url_host(url).is_some_and(|host| {
        DECENTRALAND_DOMAINS
            .iter()
            .any(|d| host == *d || host.ends_with(&format!(".{d}")))
    })
}

pub fn is_local_network(url: &str) -> bool {
    let Some(host) = url_host(url) else {
        return false;
    };
    if host == "localhost" || host == "::1" || host.ends_with(".local") {
        return true;
    }
    let octets: Vec<Option<u8>> = host.split('.').map(|o| o.parse().ok()).collect();
    match octets.as_slice() {
        [Some(a), Some(b), Some(_), Some(_)] => {
            *a == 127
                || *a == 10
                || (*a == 192 && *b == 168)
                || (*a == 172 && (16..=31).contains(b))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    struct FakeClock(AtomicU64);

    impl MonotonicClock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn router() -> (DeepLinkRouter, Arc<FakeClock>) {
        let clock = Arc::new(FakeClock(AtomicU64::new(1_000)));
        (DeepLinkRouter::new(clock.clone()), clock)
    }

    fn mobile() -> RouteContext {
        RouteContext {
            real_mobile: true,
            current_dclenv: "org".into(),
            ..Default::default()
        }
    }

    fn run(url: &str, ctx: &RouteContext) -> Routed {
        router().0.handle(url, StartKind::Warm, ctx)
    }

    fn tp(realm: Option<&str>, x: i32, y: i32) -> Option<Navigation> {
        Some(Navigation::Teleport {
            realm: realm.map(String::from),
            parcel: Parcel::new(x, y),
        })
    }

    fn cfg_of(routed: &Routed) -> Vec<ConfigParam> {
        routed
            .effects
            .iter()
            .filter_map(|e| match e {
                Effect::Config { param } => Some(param.clone()),
                _ => None,
            })
            .collect()
    }

    // ---- Navigation ---------------------------------------------------------

    #[test]
    fn navigation_table() {
        let cases: Vec<(&str, Option<Navigation>)> = vec![
            ("decentraland://jump", Some(Navigation::OpenJumpPanel)),
            ("decentraland://open", Some(Navigation::OpenJumpPanel)),
            ("decentraland://jump?foo=1", None),
            ("decentraland://jump?location=10,20", tp(None, 10, 20)),
            ("decentraland://open?position=-5,7", tp(None, -5, 7)),
            (
                "decentraland://jump?location=1,2&realm=https://r.example/x",
                tp(Some("https://r.example/x"), 1, 2),
            ),
            (
                "decentraland://jump?realm=https://r.example/x",
                tp(Some("https://r.example/x"), 0, 0),
            ),
            (
                "decentraland://jump?realm=foo.dcl.eth",
                Some(Navigation::JoinWorld {
                    realm: "foo.dcl.eth".into(),
                }),
            ),
            (
                "decentraland://jump?realm=foo.dcl.eth&location=3,4",
                tp(Some("foo.dcl.eth"), 3, 4),
            ),
            (
                "decentraland://events?id=e1",
                Some(Navigation::OpenEvent { id: "e1".into() }),
            ),
            (
                "decentraland://events?id=e1&location=1,1",
                Some(Navigation::OpenEvent { id: "e1".into() }),
            ),
            (
                "decentraland://events?position=2,3&realm=r",
                tp(Some("r"), 2, 3),
            ),
            ("decentraland://events", Some(Navigation::OpenDiscover)),
            (
                "https://decentraland.org/jump/event?id=e2",
                Some(Navigation::OpenEvent { id: "e2".into() }),
            ),
            (
                "decentraland://places?id=p1",
                Some(Navigation::OpenPlace { id: "p1".into() }),
            ),
            ("decentraland://places?position=2,3", tp(None, 2, 3)),
            ("decentraland://places", Some(Navigation::OpenDiscover)),
            (
                "https://decentraland.org/places/place/10,20",
                Some(Navigation::OpenDiscover),
            ),
            ("decentraland://mobile?location=8,9", tp(None, 8, 9)),
            ("decentraland://mobile", None),
            ("decentraland://whatever?realm=r", tp(Some("r"), 0, 0)),
            (
                "https://mobile.dclexplorer.com/open?location=1,2",
                tp(None, 1, 2),
            ),
        ];
        for (url, expected) in cases {
            assert_eq!(run(url, &mobile()).navigation, expected, "{url}");
        }
    }

    #[test]
    fn preview_replaces_realm_and_repoints_the_socket() {
        let routed = run(
            "decentraland://open?preview=http://192.168.1.2:8000&realm=other",
            &mobile(),
        );
        assert_eq!(routed.navigation, tp(Some("http://192.168.1.2:8000"), 0, 0));
        assert!(routed.effects.contains(&Effect::SetPreviewUrl {
            url: "http://192.168.1.2:8000".into()
        }));
    }

    #[test]
    fn no_navigation_off_real_mobile_but_effects_still_apply() {
        let ctx = RouteContext {
            real_mobile: false,
            ..mobile()
        };
        let routed = run("decentraland://open?location=1,2&rust-log=debug", &ctx);
        assert_eq!(routed.navigation, None);
        assert_eq!(
            cfg_of(&routed),
            vec![ConfigParam::RustLog {
                filter: "debug".into()
            }]
        );
    }

    #[test]
    fn empty_and_walletconnect_links_do_nothing() {
        assert_eq!(run("", &mobile()), Routed::default());
        let routed = run("decentraland://walletconnect?x=1", &mobile());
        assert_eq!(routed.navigation, None);
        assert!(cfg_of(&routed).is_empty());
    }

    #[test]
    fn destination_for_the_state_machine() {
        let teleport = Navigation::Teleport {
            realm: None,
            parcel: Parcel::new(1, 2),
        };
        assert_eq!(
            teleport.destination(),
            Some(Destination {
                realm: None,
                parcel: Some(Parcel::new(1, 2))
            })
        );
        let world = Navigation::JoinWorld {
            realm: "a.dcl.eth".into(),
        };
        assert_eq!(
            world.destination(),
            Some(Destination {
                realm: Some("a.dcl.eth".into()),
                parcel: None
            })
        );
        assert_eq!(Navigation::OpenDiscover.destination(), None);
    }

    // ---- Config params ------------------------------------------------------

    #[test]
    fn config_params_table() {
        let cases: Vec<(&str, ConfigParam)> = vec![
            (
                "rust-log=dclgodot::comms=debug",
                ConfigParam::RustLog {
                    filter: "dclgodot::comms=debug".into(),
                },
            ),
            (
                "pulse-server=h:1",
                ConfigParam::PulseServer {
                    server: "h:1".into(),
                },
            ),
            (
                "pulse-realm=r",
                ConfigParam::PulseRealm { realm: "r".into() },
            ),
            ("pulse=TRUE", ConfigParam::PulseEnabled { enabled: true }),
            ("pulse=no", ConfigParam::PulseEnabled { enabled: false }),
            ("dual-channel=1", ConfigParam::DualChannel { enabled: true }),
            (
                "livekit=false",
                ConfigParam::LivekitEnabled { enabled: false },
            ),
            ("multiplayer_debug=true", ConfigParam::MultiplayerDebug),
            ("livekit_debug=1", ConfigParam::MultiplayerDebug),
            ("skip-gltf=yes", ConfigParam::SkipGltf { enabled: true }),
            ("kill-sky=0", ConfigParam::KillSky { enabled: false }),
            (
                "touch-feedback=true",
                ConfigParam::TouchFeedback { enabled: true },
            ),
            (
                "enable-upgraded-deletion=true",
                ConfigParam::EnableUpgradedDeletion,
            ),
            ("gp-benchmark=true", ConfigParam::GpBenchmark),
            ("safemargindebug=1", ConfigParam::SafeMarginDebug),
            ("rotate-guest=true", ConfigParam::RotateGuest),
            ("controls-ftue=show", ConfigParam::ControlsFtueShow),
            (
                "c=summer-24",
                ConfigParam::CampaignToken {
                    token: "summer-24".into(),
                },
            ),
        ];
        for (query, expected) in cases {
            let routed = run(&format!("decentraland://open?{query}"), &mobile());
            assert_eq!(cfg_of(&routed), vec![expected], "{query}");
        }
    }

    #[test]
    fn config_params_that_must_not_apply() {
        for query in [
            "enable-upgraded-deletion=false",
            "enable-upgraded-deletion=",
            "rotate-guest=false",
            "controls-ftue=hide",
            "c=Bad_Token",
            "c=-x",
            "c=a--b",
            "pulse=",
        ] {
            let routed = run(&format!("decentraland://open?{query}"), &mobile());
            assert_eq!(cfg_of(&routed), vec![], "{query}");
        }
        let long = format!("c={}", "a".repeat(65));
        assert_eq!(
            cfg_of(&run(&format!("decentraland://open?{long}"), &mobile())),
            vec![]
        );
    }

    #[test]
    fn rotate_guest_is_ignored_in_production() {
        let ctx = RouteContext {
            production: true,
            ..mobile()
        };
        assert_eq!(
            cfg_of(&run("decentraland://open?rotate-guest=true", &ctx)),
            vec![]
        );
        assert_eq!(
            cfg_of(&run("decentraland://open?controls-ftue=show", &ctx)),
            vec![]
        );
    }

    #[test]
    fn optimized_content_url_table() {
        let cases = [
            ("https://assets.decentraland.zone/v5", false, true),
            ("https://decentraland.org", false, true),
            ("https://evil.example/v5", false, false),
            ("https://decentraland.org.evil.example/v5", false, false),
            ("http://assets.decentraland.zone", false, false),
            ("http://192.168.1.5:8080/x", false, true),
            ("http://localhost:3000", false, true),
            ("http://10.0.0.1", false, true),
            ("http://172.16.0.1", false, true),
            ("http://172.32.0.1", false, false),
            ("http://8.8.8.8", false, false),
            ("https://user@decentraland.org/x", false, false),
            ("https://assets.decentraland.zone/v5", true, false),
            ("http://192.168.1.5:8080/x", true, false),
        ];
        for (url, production, accepted) in cases {
            let ctx = RouteContext {
                production,
                ..mobile()
            };
            let link = format!("decentraland://open?optimized-content-base-url={url}");
            let expected: Vec<ConfigParam> = if accepted {
                vec![ConfigParam::OptimizedContentBaseUrl { url: url.into() }]
            } else {
                vec![]
            };
            assert_eq!(
                cfg_of(&run(&link, &ctx)),
                expected,
                "{url} production={production}"
            );
        }
    }

    #[test]
    fn config_effects_come_before_navigation_and_link_with_target_still_routes() {
        let routed = run("decentraland://jump?location=1,2&pulse=true", &mobile());
        assert_eq!(routed.navigation, tp(None, 1, 2));
        assert_eq!(
            cfg_of(&routed),
            vec![ConfigParam::PulseEnabled { enabled: true }]
        );
    }

    // ---- Analytics ----------------------------------------------------------

    #[test]
    fn push_open_carries_ids_and_start_kind() {
        for kind in [StartKind::Cold, StartKind::Warm] {
            let routed = router().0.handle(
                "decentraland://open?push_campaign_id=%20camp1%20&push_id=p9",
                kind,
                &mobile(),
            );
            assert!(routed.effects.contains(&Effect::Analytics {
                event: AnalyticsEvent::PushOpened {
                    campaign_id: "camp1".into(),
                    push_id: "p9".into(),
                    start_kind: kind
                }
            }));
        }
    }

    #[test]
    fn push_open_without_campaign_is_not_tracked() {
        let routed = run("decentraland://open?push_id=p9", &mobile());
        assert!(!routed.effects.iter().any(|e| matches!(
            e,
            Effect::Analytics {
                event: AnalyticsEvent::PushOpened { .. }
            }
        )));
    }

    #[test]
    fn every_link_notifies_the_session_with_its_params() {
        let routed = run("decentraland://events?id=e1", &mobile());
        assert!(routed.effects.contains(&Effect::Analytics {
            event: AnalyticsEvent::DeepLinkReceived {
                params: vec![ParamPair {
                    key: "id".into(),
                    value: "e1".into()
                }]
            }
        }));
    }

    #[test]
    fn start_kind_names() {
        assert_eq!(StartKind::Cold.as_str(), "cold");
        assert_eq!(StartKind::Warm.as_str(), "warm");
    }

    // ---- Swallowed links ----------------------------------------------------

    #[test]
    fn marketplace_return_is_swallowed() {
        for q in ["urn=urn:x", "iap_enabled=true"] {
            let routed = run(&format!("decentraland://open?{q}&location=1,2"), &mobile());
            assert_eq!(routed.navigation, None, "{q}");
            assert!(routed.effects.contains(&Effect::MarketplaceReturn));
        }
    }

    #[test]
    fn awaited_marketplace_return_notifies_but_keeps_routing() {
        let ctx = RouteContext {
            marketplace_awaiting_return: true,
            ..mobile()
        };
        let routed = run("decentraland://open?location=1,2", &ctx);
        assert_eq!(routed.navigation, tp(None, 1, 2));
        assert!(routed.effects.contains(&Effect::MarketplaceReturn));
        assert!(!run("decentraland://open?location=1,2", &mobile())
            .effects
            .contains(&Effect::MarketplaceReturn));
    }

    #[test]
    fn avatar_impostor_benchmark_is_swallowed() {
        let routed = run(
            "decentraland://open?benchmark=avatar-impostors&location=1,2",
            &mobile(),
        );
        assert_eq!(routed.navigation, None);
        assert!(routed
            .effects
            .contains(&Effect::StartAvatarImpostorBenchmark));
        let other = run("decentraland://open?benchmark=other", &mobile());
        assert!(!other
            .effects
            .contains(&Effect::StartAvatarImpostorBenchmark));
    }

    // ---- Environment --------------------------------------------------------

    #[test]
    fn dclenv_table() {
        // (query, current, explicit, switches)
        let cases = [
            ("dclenv=zone", "org", false, true),
            ("dclenv=org", "org", false, false),
            ("dclenv=zone", "org", true, false),
            ("dclenv=zone&signin=abc", "org", false, false),
        ];
        for (query, current, explicit, switches) in cases {
            let ctx = RouteContext {
                current_dclenv: current.into(),
                dclenv_explicit: explicit,
                ..mobile()
            };
            let routed = run(&format!("decentraland://open?{query}&location=1,2"), &ctx);
            let switched = routed.effects.contains(&Effect::SwitchEnvironment {
                dclenv: "zone".into(),
            });
            assert_eq!(switched, switches, "{query}");
            if switches {
                assert_eq!(routed.navigation, None);
            }
        }
    }

    // ---- Sign-in ------------------------------------------------------------

    fn signin_ctx(auth: AuthSnapshot) -> RouteContext {
        RouteContext { auth, ..mobile() }
    }

    #[test]
    fn signin_with_pending_auth_completes_right_away() {
        let (mut r, _) = router();
        let ctx = signin_ctx(AuthSnapshot {
            pending_mobile_auth: true,
            ..Default::default()
        });
        let routed = r.handle("decentraland://open?signin=id1", StartKind::Warm, &ctx);
        assert_eq!(routed.navigation, None);
        assert!(routed.effects.contains(&Effect::CompleteMobileSignIn {
            identity_id: "id1".into()
        }));
        assert_eq!(r.take_pending_signin_identity_id(), None);
    }

    #[test]
    fn cancelled_signin_is_dropped_even_with_pending_auth() {
        let (mut r, _) = router();
        let ctx = signin_ctx(AuthSnapshot {
            sign_in_cancelled: true,
            pending_mobile_auth: true,
            wallet_connected: false,
        });
        let routed = r.handle("decentraland://open?signin=id1", StartKind::Warm, &ctx);
        assert!(!routed.effects.iter().any(|e| matches!(
            e,
            Effect::CompleteMobileSignIn { .. } | Effect::SignInParked
        )));
        assert_eq!(r.take_pending_signin_identity_id(), None);
    }

    #[test]
    fn signin_with_a_wallet_connected_is_dropped() {
        let (mut r, _) = router();
        let ctx = signin_ctx(AuthSnapshot {
            wallet_connected: true,
            ..Default::default()
        });
        let routed = r.handle("decentraland://open?signin=id1", StartKind::Warm, &ctx);
        assert!(!routed.effects.contains(&Effect::SignInParked));
        assert_eq!(r.take_pending_signin_identity_id(), None);
    }

    #[test]
    fn cold_start_signin_is_parked_and_never_navigates() {
        let (mut r, _) = router();
        let routed = r.handle(
            "decentraland://open?signin=id1&location=1,2",
            StartKind::Cold,
            &signin_ctx(AuthSnapshot::default()),
        );
        assert_eq!(routed.navigation, None);
        assert!(routed.effects.contains(&Effect::SignInParked));
        assert_eq!(r.take_pending_signin_identity_id(), Some("id1".into()));
    }

    #[test]
    fn parked_token_is_single_shot() {
        let (mut r, _) = router();
        r.handle("decentraland://open?signin=id1", StartKind::Cold, &mobile());
        assert_eq!(r.take_pending_signin_identity_id(), Some("id1".into()));
        assert_eq!(r.take_pending_signin_identity_id(), None);
    }

    #[test]
    fn parked_token_age_cap_table() {
        let cases = [
            (0, true),
            (PENDING_SIGNIN_MAX_AGE_MS - 1, true),
            (PENDING_SIGNIN_MAX_AGE_MS, true),
            (PENDING_SIGNIN_MAX_AGE_MS + 1, false),
        ];
        for (age, redeemable) in cases {
            let (mut r, clock) = router();
            r.handle("decentraland://open?signin=id1", StartKind::Cold, &mobile());
            clock.0.fetch_add(age, Ordering::SeqCst);
            let expected = redeemable.then(|| "id1".to_string());
            assert_eq!(r.take_pending_signin_identity_id(), expected, "age {age}");
        }
    }

    #[test]
    fn stale_token_is_cleared_by_the_failed_take() {
        let (mut r, clock) = router();
        r.handle("decentraland://open?signin=id1", StartKind::Cold, &mobile());
        clock
            .0
            .fetch_add(PENDING_SIGNIN_MAX_AGE_MS + 1, Ordering::SeqCst);
        assert_eq!(r.take_pending_signin_identity_id(), None);
        clock.0.store(1_000, Ordering::SeqCst);
        assert_eq!(r.take_pending_signin_identity_id(), None);
    }

    #[test]
    fn a_later_token_replaces_the_parked_one() {
        let (mut r, clock) = router();
        r.handle("decentraland://open?signin=old", StartKind::Cold, &mobile());
        clock
            .0
            .fetch_add(PENDING_SIGNIN_MAX_AGE_MS, Ordering::SeqCst);
        r.handle("decentraland://open?signin=new", StartKind::Cold, &mobile());
        clock.0.fetch_add(10, Ordering::SeqCst);
        assert_eq!(r.take_pending_signin_identity_id(), Some("new".into()));
    }

    #[test]
    fn system_clock_is_monotonic() {
        let clock = SystemClock::new();
        let a = clock.now_ms();
        assert!(clock.now_ms() >= a);
    }
}
