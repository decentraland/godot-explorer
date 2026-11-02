//! Deep links reaching the engine: iOS signals, Android intents and `--fake-deeplink`. The link is
//! parsed and stored on `Global`, and the engine-owned settings it carries are applied. When the
//! host drives the app (the shipping case) the core routes the link (navigation, sign-in resume,
//! campaign and push attribution); only the settings the engine owns stay here.

use dcl_core::deep_link_route::{is_decentraland_https, is_local_network};
use godot::classes::RefCounted;
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_ios_plugin::DclIosPlugin;
use crate::godot_classes::dcl_parse_deep_link::DclParseDeepLink;

/// Truthy values of a deeplink flag.
pub fn truthy(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

/// The decision about an `optimized-content-base-url` param.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptimizedUrl {
    Absent,
    /// Production builds never take it.
    IgnoredInProduction,
    /// Only https Decentraland hosts or a LAN host qualify.
    Rejected,
    Accepted,
}

pub fn judge_optimized_url(url: &str, production: bool) -> OptimizedUrl {
    if url.is_empty() {
        return OptimizedUrl::Absent;
    }
    if production {
        return OptimizedUrl::IgnoredInProduction;
    }
    if is_decentraland_https(url) || is_local_network(url) {
        OptimizedUrl::Accepted
    } else {
        OptimizedUrl::Rejected
    }
}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct DeepLinkRouter {
    base: Base<RefCounted>,
}

#[godot_api]
impl DeepLinkRouter {
    /// A link was parsed and nothing in the engine consumed it.
    #[signal]
    pub fn deep_link_received();

    /// Parses and stores a link, applies its engine settings and emits `deep_link_received`.
    /// `start_kind` is "cold" when the link launched the process and "warm" when the app was
    /// already running; only the push attribution reads it.
    #[func]
    pub fn process_deep_link(
        &mut self,
        url: GString,
        #[opt(default = "warm")] start_kind: GString,
    ) {
        let url = url.to_string();
        if url.is_empty() {
            return;
        }
        // Consume the plugin's received URL so the next resume does not read it again.
        if DclIosPlugin::is_available() {
            let _ = DclIosPlugin::get_deeplink_args();
        }
        let obj = DclParseDeepLink::parse_decentraland_link(url.as_str().into());
        {
            let mut global = DclGlobal::singleton();
            let mut global = global.bind_mut();
            global.deep_link_url = url.as_str().into();
            global.deep_link_obj = obj.clone();
        }
        godot_print!(
            "[DEEPLINK] process_deep_link params: {}",
            obj.bind().get_params()
        );
        apply_link_settings(&obj);

        // With the host driving the app the core routes the link, including push attribution.
        if globals::engine().bind().is_host_driven() {
            return;
        }
        track_push_open(&obj, &start_kind.to_string());
        self.run_deferred(|this| this.emit_deep_link_received());
    }

    #[func]
    fn emit_deep_link_received(&mut self) {
        self.signals().deep_link_received().emit();
    }

    /// Whether a URL names a loopback, private-range or `.local` host.
    #[func]
    fn is_local_network(url: GString) -> bool {
        is_local_network(&url.to_string())
    }

    /// Clears the link URL only: the parsed link is still needed by the scene fetcher (preview
    /// mode) and anything else that checks its params.
    #[func]
    pub fn clear_deep_link(&mut self) {
        DclGlobal::singleton().bind_mut().deep_link_url = GString::new();
    }
}

fn param(obj: &Gd<DclParseDeepLink>, key: &str) -> String {
    obj.bind()
        .get_params()
        .get(key)
        .map(|v| v.stringify().to_string())
        .unwrap_or_default()
}

/// Applies what a link configures in the engine: the log filter, the comms transport, the optimized
/// content host, a guest rotation, the glTF and sky switches and the benchmark runners.
pub fn apply_link_settings(obj: &Gd<DclParseDeepLink>) {
    let production = DclGlobal::is_production();
    let rust_log = param(obj, "rust-log");
    if !rust_log.is_empty() {
        godot_print!("[DEEPLINK] Found rust-log param: {rust_log}");
        DclGlobal::set_rust_log_filter(rust_log.as_str().into());
    }
    apply_comms_params(obj);
    apply_optimized_content_base_url(obj, production);
    capture_debug_guest_rotate(obj, production);

    // `skip-gltf` must be set before any scene's glTF component is processed, and the bench
    // runner's own override runs too late: apply it here, which lands before a scene starts.
    let skip_gltf = param(obj, "skip-gltf");
    let kill_sky = param(obj, "kill-sky");
    {
        let mut cli = globals::cli();
        let mut cli = cli.bind_mut();
        if !skip_gltf.is_empty() {
            cli.skip_gltf_load = truthy(&skip_gltf);
            godot_print!("[DEEPLINK] skip-gltf={}", cli.skip_gltf_load);
        }
        if !kill_sky.is_empty() {
            cli.kill_sky = truthy(&kill_sky);
            godot_print!("[DEEPLINK] kill-sky={}", cli.kill_sky);
        }
    }

    if obj.bind().get_gp_benchmark() {
        spawn_gp_benchmark();
    }
    DclGlobal::maybe_run_logging_selftest(&param(obj, "test-logging"));
    super::async_shaders::update();
    // Frame-sync warnings (slow onUpdate, missed frames) follow the scene-stats availability.
    if obj.bind().get_scene_stats() {
        crate::dcl::common::set_scene_perf_warnings_enabled(true);
    }
}

/// The Genesis Plaza profiling benchmark (issue #1862). The CLI path starts the runner at boot,
/// but on mobile the deep link is parsed later: start it here, once, if no runner exists.
fn spawn_gp_benchmark() {
    let mut global = DclGlobal::singleton();
    if global.has_node("GPBenchmarkRunner") {
        return;
    }
    // Flip bench mode before the runner exists: this fires before the deferred dynamic-graphics
    // init and before the first-launch hardware benchmark trigger, so both honor it.
    globals::cli().bind_mut().bench_mode = true;
    godot_print!("[DEEPLINK] bench_mode=true (gp-benchmark deeplink)");
    crate::engine::bootstrap::spawn_gp_benchmark_runner(&mut global);
}

/// The optimized-content host of a link: only non-production builds, and only https Decentraland or
/// a LAN host.
pub fn apply_optimized_content_base_url(obj: &Gd<DclParseDeepLink>, production: bool) {
    let url = param(obj, "optimized-content-base-url");
    match judge_optimized_url(&url, production) {
        OptimizedUrl::Absent => {}
        OptimizedUrl::IgnoredInProduction => {
            godot_print!("[DEEPLINK] optimized-content-base-url ignored on production builds")
        }
        OptimizedUrl::Rejected => godot_print!(
            "[DEEPLINK] optimized-content-base-url rejected (not https Decentraland or LAN)"
        ),
        OptimizedUrl::Accepted => {
            godot_print!("[DEEPLINK] optimized-content-base-url={url}");
            globals::cli().bind_mut().optimized_content_base_url = url.as_str().into();
        }
    }
}

/// QA affordance: `rotate-guest=true` mints a brand-new guest, the only way back to the first-time
/// flow on a device whose native anchor survives reinstall (Android SSAID, iOS Keychain). It also
/// wipes the on-disk guest identity, so the flow is reachable from this launch. Anything not cut
/// from `release*` honours it, TestFlight included; it is recoverable, since the flag lives in
/// `user://` and the native anchor is never touched.
pub fn capture_debug_guest_rotate(obj: &Gd<DclParseDeepLink>, production: bool) {
    if production || !param(obj, "rotate-guest").eq_ignore_ascii_case("true") {
        return;
    }
    globals::engine().bind().set_setting(
        "debug_rotate_guest_anchor".into(),
        true.to_variant(),
        true,
    );
    let removed = crate::app::session::clear_guest_device_storage();
    godot_print!("[DEEPLINK] rotate-guest=true: cleared {removed} guest file(s)");
}

/// The comms deeplink params (pulse-server, pulse-realm, pulse, dual-channel, livekit,
/// multiplayer_debug), shared by live links and `--fake-deeplink`.
pub fn apply_comms_params(obj: &Gd<DclParseDeepLink>) {
    let mut comms = globals::comms();
    // `pulse-server=<host:port>` joins a specific Pulse server (shareable: everyone opening the link
    // lands on the same instance; implies enabling).
    let server = param(obj, "pulse-server");
    if !server.is_empty() {
        godot_print!("[DEEPLINK] pulse-server={server}");
        comms.bind_mut().set_pulse_server(server.as_str().into());
    }
    // `pulse-realm=<realm>` announces this realm instead of the derived one (exact match; implies
    // enabling).
    let realm = param(obj, "pulse-realm");
    if !realm.is_empty() {
        godot_print!("[DEEPLINK] pulse-realm={realm}");
        comms.bind_mut().set_pulse_realm(realm.as_str().into());
    }
    // `pulse=true/false` toggles the transport with the configured endpoint.
    let pulse = param(obj, "pulse");
    if !pulse.is_empty() {
        godot_print!("[DEEPLINK] pulse={pulse}");
        comms.bind_mut().set_pulse_enabled(truthy(&pulse));
    }
    // `dual-channel=true/false`: whether movement and emotes keep going over LiveKit while Pulse is
    // established. Overrides the deployment's flag (default true) for this run.
    let dual = param(obj, "dual-channel");
    if !dual.is_empty() {
        godot_print!("[DEEPLINK] dual-channel={dual}");
        comms
            .bind_mut()
            .set_livekit_movement_dual_channel(truthy(&dual));
    }
    // `livekit=false`: pulse-only mode, no LiveKit rooms at all (no chat, voice or scene messages).
    // A dev switch; `livekit=true` re-enables.
    let livekit = param(obj, "livekit");
    if !livekit.is_empty() {
        godot_print!("[DEEPLINK] livekit={livekit}");
        comms.bind_mut().set_livekit_enabled(truthy(&livekit));
    }
    // `multiplayer_debug=true` (legacy alias livekit_debug): latched on the comms manager, which
    // survives scene changes. Enable-only: absence must not turn the panel off on a later link.
    if obj.bind().get_multiplayer_debug() {
        godot_print!("[DEEPLINK] multiplayer_debug=true");
        comms.bind_mut().set_multiplayer_debug(true);
    }
}

/// `Push Opened` when the link carries push attribution. Here because every link passes through
/// this point on both platforms and in both start kinds; attributing from a routing branch would
/// miss whichever branch consumed the link first.
fn track_push_open(obj: &Gd<DclParseDeepLink>, start_kind: &str) {
    let campaign = param(obj, "push_campaign_id").trim().to_string();
    if campaign.is_empty() {
        return;
    }
    let push_id = param(obj, "push_id").trim().to_string();
    godot_print!("[PUSH] opened campaign={campaign} push_id={push_id} start={start_kind}");
    globals::metrics()
        .bind_mut()
        .track_push_opened(campaign, push_id, start_kind.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_accept_the_usual_truthy_words() {
        for v in ["true", "True", "1", "yes", "YES"] {
            assert!(truthy(v), "{v}");
        }
        for v in ["", "false", "0", "no", "on"] {
            assert!(!truthy(v), "{v}");
        }
    }

    #[test]
    fn the_optimized_host_is_judged_in_order() {
        assert_eq!(judge_optimized_url("", false), OptimizedUrl::Absent);
        assert_eq!(
            judge_optimized_url("https://asset.decentraland.org", true),
            OptimizedUrl::IgnoredInProduction
        );
        assert_eq!(
            judge_optimized_url("https://asset.decentraland.org", false),
            OptimizedUrl::Accepted
        );
        assert_eq!(
            judge_optimized_url("http://192.168.1.5:8000", false),
            OptimizedUrl::Accepted
        );
        assert_eq!(
            judge_optimized_url("https://evil.example.com", false),
            OptimizedUrl::Rejected
        );
    }
}
