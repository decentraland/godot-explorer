//! Applies the remote feature flags to the app. The rules and the fetch live in the core
//! (`DclFeatureFlags`); this node starts the fetch at boot and pushes the flags that have runtime
//! effects: comms, the push registration and async shader compilation.
//!
//! The fetch is fire-and-forget. Fail-open: a timeout, a network error or a malformed payload leaves
//! every flag at its default and the feature enabled. Exceptions: `pulse` and
//! `async-shader-compilation` are fail-closed, active only when the payload explicitly enables
//! them. If a flag lands after comms already connected, the Rust-side runtime toggles reconcile
//! gracefully (archipelago goes down, scene rooms stay).

use godot::classes::{INode, Node};
use godot::prelude::*;

use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_feature_flags::DclFeatureFlags;
use crate::godot_classes::promise::PromiseError;

/// Flag names exactly as the mobile-bff payload serves them.
pub const FLAG_ARCHIPELAGO: &str = "archipielago";
pub const FLAG_PULSE: &str = "pulse";
pub const FLAG_DUAL_CHANNEL: &str = "dual-channel";
/// Collects FCM push tokens and maps them in Segment. Turning it off stops new tokens from being
/// registered; it does not stop delivery to tokens already collected, a server-side decision.
pub const FLAG_PUSH_ENABLED: &str = "push-enabled";

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct FeatureFlags {
    base: Base<Node>,
}

#[godot_api]
impl INode for FeatureFlags {
    fn ready(&mut self) {
        self.run_deferred(|this| {
            // A local opt-in applies right away, without waiting for the fetch.
            super::async_shaders::update();
            this.load_flags();
        });
    }
}

#[godot_api]
impl FeatureFlags {
    #[signal]
    pub fn flags_loaded();

    #[func]
    pub fn is_loaded(&self) -> bool {
        DclFeatureFlags::is_loaded()
    }

    /// `default_value` is returned while the flags are not loaded yet or the flag is absent.
    #[func]
    pub fn is_enabled(
        &self,
        flag_name: GString,
        #[opt(default = true)] default_value: bool,
    ) -> bool {
        DclFeatureFlags::is_enabled(flag_name, default_value)
    }

    /// Numeric flags (sample rates). `default_value` is returned while the flags are not loaded,
    /// when the flag is absent, or when it is not a number.
    #[func]
    pub fn get_number(&self, flag_name: GString, default_value: f64) -> f64 {
        DclFeatureFlags::get_number(flag_name, default_value)
    }

    fn load_flags(&mut self) {
        let me = self.to_gd();
        spawn_local(load(me));
    }
}

async fn load(node: Gd<FeatureFlags>) {
    let result = await_promise(DclFeatureFlags::async_load()).await;
    if !node.is_instance_valid() {
        return;
    }
    match result.try_to::<Gd<PromiseError>>() {
        // Expected on offline or slow cold starts: fail-open is the design, so not error level.
        Ok(error) => godot_warn!(
            "[FeatureFlags] fetch failed (fail-open defaults): {}",
            error.bind().get_error()
        ),
        Err(_) => godot_print!("[FeatureFlags] loaded: {}", DclFeatureFlags::flags()),
    }
    node.bind().apply_flags();
    node.signals().flags_loaded().emit();
}

impl FeatureFlags {
    fn enabled(&self, flag: &str, default: bool) -> bool {
        self.is_enabled(flag.into(), default)
    }

    /// Applies the comms flags. `archipielago=false` skips the archipelago connection while scene
    /// rooms and Pulse keep working. `pulse` is fail-closed; it only decides the default, and an
    /// explicit local choice (deeplink `pulse=true`, `pulse-server=`, `--pulse`) or opt-out
    /// (`--no-pulse`, `pulse=false`) always wins. `dual-channel` is the rollout control for avatar
    /// sync: while true (the default, today's behaviour) movement and emotes keep going over
    /// LiveKit even when Pulse is established; false hands them to Pulse alone, which is only safe
    /// once the deployment's authoritative servers ingest Pulse as scene listeners.
    fn apply_flags(&self) {
        let mut comms = globals::comms();
        {
            let mut comms = comms.bind_mut();
            comms.set_archipelago_enabled(self.enabled(FLAG_ARCHIPELAGO, true));
            comms.set_pulse_flag_enabled(self.enabled(FLAG_PULSE, false));
            comms.set_dual_channel_flag_enabled(self.enabled(FLAG_DUAL_CHANNEL, true));
        }
        // Metrics holds the startup identify until this arrives: the cached FCM token is already
        // there at its ready, so the identify would otherwise always win the race and the switch
        // could only suppress token rotations. Reached on both paths above, so the hold ends even
        // when the fetch fails.
        globals::metrics()
            .bind_mut()
            .set_push_enabled(self.enabled(FLAG_PUSH_ENABLED, true));
        let _ = &mut comms;
        super::async_shaders::on_flags_applied();
    }
}
