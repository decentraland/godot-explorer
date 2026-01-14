use std::sync::Arc;

use godot::{
    classes::{node::ProcessMode, Engine, Os},
    obj::Singleton,
    prelude::*,
};

use crate::{
    analytics::metrics::Metrics,
    auth::{dcl_player_identity::DclPlayerIdentity, ethereum_provider::EthereumProvider},
    avatars::avatar_scene::AvatarScene,
    comms::communication_manager::CommunicationManager,
    content::content_provider::ContentProvider,
    dcl::common::set_scene_log_enabled,
    godot_classes::dcl_avatar::DclAvatar,
    http_request::rust_http_queue_requester::RustHttpQueueRequester,
    profile::profile_service::ProfileService,
    scene_runner::{scene_manager::SceneManager, tokio_runtime::TokioRuntime},
    test_runner::testing_tools::DclTestingTools,
    tools::network_inspector::{NetworkInspector, NetworkInspectorSender},
};

#[cfg(feature = "use_memory_debugger")]
use crate::tools::memory_debugger::MemoryDebugger;

#[cfg(feature = "use_memory_debugger")]
use crate::tools::benchmark_report::BenchmarkReport;

use super::{
    dcl_cli::DclCli, dcl_config::DclConfig, dcl_realm::DclRealm,
    dcl_social_blacklist::DclSocialBlacklist, dcl_social_service::DclSocialService,
    dcl_tokio_rpc::DclTokioRpc, portables::DclPortableExperienceController,
};

#[cfg(target_os = "android")]
mod android {
    use crate::tools::sentry_logger::SentryTracingLayer;
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{self, registry};

    pub fn init_logger() {
        // Configure logging filters for Android
        // By default, filter everything to WARN level
        // You can customize specific modules here:
        // Examples:
        //   "warn" - only warnings and errors (default)
        //   "debug" - show debug logs from all modules
        //   "dclgodot::scene_runner=debug,warn" - debug for scene_runner, warn for others
        //   "dclgodot::scene_runner=debug,dclgodot::comms=info,warn" - multiple modules

        let filter = EnvFilter::new(
            // TODO: Modify this line to change logging levels
            // "warn"  // Only warnings and errors
            // "libwebrtc=error,debug" // Filter out libwebrtc noise, show debug for everything else
            // "debug"  // Debug, info, warnings and errors (shows all debug logs)
            // "dclgodot::scene_runner=trace,warn"  // Trace for scene_runner, warn for everything else
            // "dclgodot::scene_runner=debug,dclgodot::comms=info,warn"  // Debug for scene_runner, info for comms, warn for everything else
            "info",
        );

        let android_layer = paranoid_android::layer(env!("CARGO_PKG_NAME"))
            .with_span_events(FmtSpan::CLOSE)
            .with_thread_names(true)
            .with_ansi(false) // Disable ANSI color codes for cleaner logcat output
            .with_filter(filter);

        // Add Sentry layer to capture errors and warnings
        let sentry_layer = SentryTracingLayer;

        registry().with(android_layer).with(sentry_layer).init();
    }
}

#[cfg(target_os = "ios")]
mod ios {
    use crate::tools::sentry_logger::SentryTracingLayer;
    use tracing_oslog::OsLogger;
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry;

    pub fn init_logger() {
        // Configure logging filters for iOS
        // By default, filter everything to INFO level
        // You can customize specific modules here:
        // Examples:
        //   "warn" - only warnings and errors
        //   "debug" - show debug logs from all modules
        //   "dclgodot::scene_runner=debug,warn" - debug for scene_runner, warn for others
        //   "dclgodot::scene_runner=debug,dclgodot::comms=info,warn" - multiple modules

        let filter = EnvFilter::new(
            // TODO: Modify this line to change logging levels
            // "warn"  // Only warnings and errors
            // "debug"  // Debug, info, warnings and errors (shows all debug logs)
            // "dclgodot::scene_runner=trace,warn"  // Trace for scene_runner, warn for everything else
            // "dclgodot::scene_runner=debug,dclgodot::comms=info,warn"  // Debug for scene_runner, info for comms, warn for everything else
            "info",
        );

        // Use OSLog for iOS - writes to the system log instead of stderr
        // This avoids crashes when stderr is not available (e.g., running without Xcode)
        let oslog_layer = OsLogger::new(env!("CARGO_PKG_NAME"), "default").with_filter(filter);

        // Add Sentry layer to capture errors and warnings
        let sentry_layer = SentryTracingLayer;

        registry().with(oslog_layer).with(sentry_layer).init();
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod desktop {
    use crate::tools::sentry_logger::SentryTracingLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry;

    pub fn init_logger() {
        let fmt_layer = tracing_subscriber::fmt::layer();
        let sentry_layer = SentryTracingLayer;

        registry()
            .with(fmt_layer)
            .with(sentry_layer)
            .try_init()
            .ok();
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclGlobal {
    _base: Base<Node>,

    #[var]
    pub config: Gd<DclConfig>,
    #[var]
    pub scene_runner: Gd<SceneManager>,
    #[var]
    pub comms: Gd<CommunicationManager>,
    #[var]
    pub avatars: Gd<AvatarScene>,
    #[var]
    pub tokio_runtime: Gd<TokioRuntime>,
    #[var]
    pub realm: Gd<DclRealm>,
    #[var]
    pub portable_experience_controller: Gd<DclPortableExperienceController>,
    #[var]
    pub testing_tools: Gd<DclTestingTools>,
    #[var]
    pub preview_mode: bool,
    #[var]
    pub testing_scene_mode: bool,
    #[var]
    pub fixed_skybox_time: bool,
    #[var]
    pub player_identity: Gd<DclPlayerIdentity>,
    #[var]
    pub content_provider: Gd<ContentProvider>,
    #[var(get)]
    pub http_requester: Gd<RustHttpQueueRequester>,
    #[var]
    pub dcl_tokio_rpc: Gd<DclTokioRpc>,

    pub ethereum_provider: Arc<EthereumProvider>,

    #[var]
    pub metrics: Gd<Metrics>,

    #[var]
    pub renderer_version: GString,

    pub is_mobile: bool,

    pub is_android: bool,

    pub is_ios: bool,

    pub is_virtual_mobile: bool,

    #[var]
    pub has_javascript_debugger: bool,

    #[var]
    pub network_inspector: Gd<NetworkInspector>,

    #[var]
    pub social_blacklist: Gd<DclSocialBlacklist>,

    #[var]
    pub social_service: Gd<DclSocialService>,

    #[cfg(feature = "use_memory_debugger")]
    #[var]
    pub memory_debugger: Gd<MemoryDebugger>,

    #[cfg(feature = "use_memory_debugger")]
    #[var]
    pub benchmark_report: Gd<BenchmarkReport>,

    #[var(get)]
    pub profile_service: Gd<ProfileService>,

    #[var(get)]
    pub cli: Gd<DclCli>,

    pub selected_avatar: Option<Gd<DclAvatar>>,

    // Input modifier state - set by scenes via PBInputModifier component on PLAYER entity
    #[var]
    pub input_modifier_disable_all: bool,
    #[var]
    pub input_modifier_disable_walk: bool,
    #[var]
    pub input_modifier_disable_jog: bool,
    #[var]
    pub input_modifier_disable_run: bool,
    #[var]
    pub input_modifier_disable_jump: bool,
    #[var]
    pub input_modifier_disable_emote: bool,
}

#[godot_api]
impl INode for DclGlobal {
    fn init(base: Base<Node>) -> Self {
        #[cfg(feature = "use_deno")]
        crate::dcl::js::init_runtime();

        #[cfg(target_os = "android")]
        android::init_logger();

        #[cfg(target_os = "ios")]
        ios::init_logger();

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        desktop::init_logger();

        tracing::info!(
            "DclGlobal init invoked version={}",
            env!("GODOT_EXPLORER_VERSION")
        );

        log_panics::init();

        // Initialize Rust classes
        let mut avatars: Gd<AvatarScene> = AvatarScene::new_alloc();
        let mut comms: Gd<CommunicationManager> = CommunicationManager::new_alloc();
        let mut scene_runner: Gd<SceneManager> = SceneManager::new_alloc();
        let mut tokio_runtime: Gd<TokioRuntime> = TokioRuntime::new_alloc();
        let mut content_provider: Gd<ContentProvider> = ContentProvider::new_alloc();
        let mut network_inspector: Gd<NetworkInspector> = NetworkInspector::new_alloc();
        let mut social_blacklist: Gd<DclSocialBlacklist> = DclSocialBlacklist::new_alloc();
        let mut social_service: Gd<DclSocialService> = DclSocialService::new_alloc();

        #[cfg(feature = "use_memory_debugger")]
        let mut memory_debugger: Gd<MemoryDebugger> = MemoryDebugger::new_alloc();

        #[cfg(feature = "use_memory_debugger")]
        let mut benchmark_report: Gd<BenchmarkReport> = BenchmarkReport::new_alloc();

        let mut metrics: Gd<Metrics> = Metrics::new_alloc();
        let mut cli: Gd<DclCli> = DclCli::new_alloc();

        // For now, keep using base Rust classes - GDScript extensions will be created in global.gd
        let mut realm: Gd<DclRealm> = DclRealm::new_alloc();
        let mut dcl_tokio_rpc: Gd<DclTokioRpc> = DclTokioRpc::new_alloc();
        let mut player_identity: Gd<DclPlayerIdentity> = DclPlayerIdentity::new_alloc();
        let mut testing_tools: Gd<DclTestingTools> = DclTestingTools::new_alloc();
        let mut portable_experience_controller: Gd<DclPortableExperienceController> =
            DclPortableExperienceController::new_alloc();

        tokio_runtime.set_name("tokio_runtime");
        scene_runner.set_name("scene_runner");
        scene_runner.set_process_mode(ProcessMode::DISABLED);
        comms.set_name("comms");
        avatars.set_name("avatar_scene");
        realm.set_name("realm");
        dcl_tokio_rpc.set_name("dcl_tokio_rpc");
        player_identity.set_name("player_identity");
        testing_tools.set_name("testing_tool");
        content_provider.set_name("content_provider");
        portable_experience_controller.set_name("portable_experience_controller");
        network_inspector.set_name("network_inspector");
        social_blacklist.set_name("social_blacklist");
        social_service.set_name("social_service");

        #[cfg(feature = "use_memory_debugger")]
        memory_debugger.set_name("memory_debugger");

        #[cfg(feature = "use_memory_debugger")]
        benchmark_report.set_name("benchmark_report");

        metrics.set_name("metrics");
        cli.set_name("cli");

        // Use CLI singleton for parsing
        let (testing_scene_mode, preview_mode, developer_mode, fixed_skybox_time, force_mobile) = {
            let cli_bind = cli.bind();
            (
                cli_bind.scene_test_mode,
                cli_bind.preview_mode,
                cli_bind.developer_mode,
                cli_bind.fixed_skybox_time,
                cli_bind.force_mobile,
            )
        };

        set_scene_log_enabled(preview_mode || testing_scene_mode || developer_mode);

        let is_mobile = Os::singleton().has_feature("mobile") || force_mobile;
        let is_android = std::env::consts::OS == "android";
        let is_ios = std::env::consts::OS == "ios";

        // For now, use base class - ConfigData will be created in global.gd
        let config = DclConfig::new_gd();

        Self {
            _base: base,
            is_mobile,
            is_android,
            is_ios,
            is_virtual_mobile: false,
            scene_runner,
            comms,
            avatars,
            tokio_runtime,
            testing_tools,
            realm,
            portable_experience_controller,
            preview_mode,
            testing_scene_mode,
            fixed_skybox_time,
            dcl_tokio_rpc,
            player_identity,
            content_provider,
            http_requester: RustHttpQueueRequester::new_gd(),
            config,
            ethereum_provider: Arc::new(EthereumProvider::new()),
            metrics,
            renderer_version: env!("GODOT_EXPLORER_VERSION").into(),
            network_inspector,
            social_blacklist,
            social_service,

            #[cfg(feature = "use_memory_debugger")]
            memory_debugger,

            #[cfg(feature = "use_memory_debugger")]
            benchmark_report,

            profile_service: ProfileService::new_gd(),

            #[cfg(feature = "enable_inspector")]
            has_javascript_debugger: true,
            #[cfg(not(feature = "enable_inspector"))]
            has_javascript_debugger: false,
            cli,
            selected_avatar: None,

            // Input modifiers start as false (no modification)
            input_modifier_disable_all: false,
            input_modifier_disable_walk: false,
            input_modifier_disable_jog: false,
            input_modifier_disable_run: false,
            input_modifier_disable_jump: false,
            input_modifier_disable_emote: false,
        }
    }
}

#[godot_api]
impl DclGlobal {
    #[func]
    fn set_scene_log_enabled(&self, enabled: bool) {
        set_scene_log_enabled(enabled);
    }

    #[func]
    fn is_mobile(&self) -> bool {
        self.is_mobile
    }

    #[func]
    fn is_android(&self) -> bool {
        self.is_android
    }

    #[func]
    fn is_ios(&self) -> bool {
        self.is_ios
    }

    #[func]
    fn is_virtual_mobile(&self) -> bool {
        self.is_virtual_mobile
    }

    #[func]
    fn _set_is_mobile(&mut self, is_mobile: bool) {
        self.is_mobile = is_mobile;
        self.is_virtual_mobile = is_mobile;
    }

    #[func]
    fn get_selected_avatar(&self) -> Option<Gd<DclAvatar>> {
        self.selected_avatar.clone()
    }

    #[func]
    pub fn ui_has_focus(&self) -> bool {
        // Check if the explorer UI has focus by calling the GDScript function
        let tree = Engine::singleton().get_main_loop();
        if let Some(tree) = tree {
            if let Ok(tree) = tree.try_cast::<SceneTree>() {
                let root = tree.get_root();
                if let Some(root) = root {
                    // Try to find the explorer node
                    let explorer = root.get_node_or_null("explorer");
                    if let Some(mut explorer) = explorer {
                        // Call ui_has_focus if it exists
                        if explorer.has_method("ui_has_focus") {
                            return explorer.call("ui_has_focus", &[]).to::<bool>();
                        }
                    }
                }
            }
        }
        // Default to true if we can't determine focus state
        true
    }

    #[func]
    pub fn get_version() -> GString {
        env!("GODOT_EXPLORER_VERSION").into()
    }

    #[func]
    pub fn is_production() -> bool {
        env!("GODOT_EXPLORER_VERSION").contains("-prod")
    }

    #[func]
    pub fn is_staging() -> bool {
        env!("GODOT_EXPLORER_VERSION").contains("-staging")
    }

    #[func]
    pub fn is_dev() -> bool {
        env!("GODOT_EXPLORER_VERSION").contains("-dev")
    }

    pub fn has_singleton() -> bool {
        let Some(main_loop) = Engine::singleton().get_main_loop() else {
            return false;
        };
        let Some(root) = main_loop.cast::<SceneTree>().get_root() else {
            return false;
        };
        root.has_node("Global")
    }

    pub fn try_singleton() -> Option<Gd<Self>> {
        let res = Engine::singleton()
            .get_main_loop()?
            .cast::<SceneTree>()
            .get_root()?
            .get_node_or_null("Global")?
            .try_cast::<Self>();
        res.ok()
    }

    pub fn singleton() -> Gd<Self> {
        Self::try_singleton().expect("Failed to get global singleton!")
    }

    pub fn get_network_inspector_sender() -> Option<NetworkInspectorSender> {
        Some(
            Self::try_singleton()?
                .bind()
                .network_inspector
                .bind()
                .get_sender(),
        )
    }

    /// Reset all input modifiers to false (no modification)
    pub fn reset_input_modifiers(&mut self) {
        self.input_modifier_disable_all = false;
        self.input_modifier_disable_walk = false;
        self.input_modifier_disable_jog = false;
        self.input_modifier_disable_run = false;
        self.input_modifier_disable_jump = false;
        self.input_modifier_disable_emote = false;
    }

    /// Check if walk input is disabled (either by disable_all or disable_walk)
    #[func]
    pub fn is_walk_disabled(&self) -> bool {
        self.input_modifier_disable_all || self.input_modifier_disable_walk
    }

    /// Check if jog input is disabled (either by disable_all or disable_jog)
    #[func]
    pub fn is_jog_disabled(&self) -> bool {
        self.input_modifier_disable_all || self.input_modifier_disable_jog
    }

    /// Check if run input is disabled (either by disable_all or disable_run)
    #[func]
    pub fn is_run_disabled(&self) -> bool {
        self.input_modifier_disable_all || self.input_modifier_disable_run
    }

    /// Check if jump input is disabled (either by disable_all or disable_jump)
    #[func]
    pub fn is_jump_disabled(&self) -> bool {
        self.input_modifier_disable_all || self.input_modifier_disable_jump
    }

    /// Check if emote input is disabled (either by disable_all or disable_emote)
    #[func]
    pub fn is_emote_disabled(&self) -> bool {
        self.input_modifier_disable_all || self.input_modifier_disable_emote
    }

    /// Check if all movement input is disabled
    #[func]
    pub fn is_all_input_disabled(&self) -> bool {
        self.input_modifier_disable_all
    }

    /// Drains and returns all pending Rust log entries (errors and warnings) for Sentry.
    /// Returns an Array of Dictionaries with keys: "level" (String), "message" (String), "target" (String)
    #[func]
    pub fn drain_rust_logs() -> VarArray {
        use crate::tools::sentry_logger::drain_sentry_logs;

        let logs = drain_sentry_logs();
        let mut result = VarArray::new();

        for entry in logs {
            let mut dict = VarDictionary::new();
            dict.set("level", entry.level.as_str());
            dict.set("message", entry.message);
            dict.set("target", entry.target);
            result.push(&dict.to_variant());
        }

        result
    }

    /// Emits test messages at various Rust tracing levels to verify Sentry integration.
    #[func]
    pub fn emit_sentry_rust_test_messages() {
        use crate::tools::sentry_logger::emit_sentry_test_messages;
        emit_sentry_test_messages();
    }
}
