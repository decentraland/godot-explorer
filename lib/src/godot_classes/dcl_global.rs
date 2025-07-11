use std::sync::Arc;

use godot::{
    engine::{node::ProcessMode, Engine},
    prelude::*,
};

use crate::{
    analytics::metrics::Metrics,
    auth::{dcl_player_identity::DclPlayerIdentity, ethereum_provider::EthereumProvider},
    avatars::avatar_scene::AvatarScene,
    comms::communication_manager::CommunicationManager,
    content::content_provider::ContentProvider,
    dcl::common::set_scene_log_enabled,
    http_request::rust_http_queue_requester::RustHttpQueueRequester,
    profile::profile_service::ProfileService,
    scene_runner::{scene_manager::SceneManager, tokio_runtime::TokioRuntime},
    test_runner::testing_tools::DclTestingTools,
    tools::network_inspector::{NetworkInspector, NetworkInspectorSender},
};

use super::{
    dcl_config::DclConfig, dcl_realm::DclRealm, dcl_social_blacklist::DclSocialBlacklist,
    dcl_tokio_rpc::DclTokioRpc, portables::DclPortableExperienceController,
};

#[cfg(target_os = "android")]
mod android {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{self, registry};

    pub fn init_logger() {
        let android_layer = paranoid_android::layer(env!("CARGO_PKG_NAME"))
            .with_span_events(FmtSpan::CLOSE)
            .with_thread_names(true)
            .with_filter(LevelFilter::WARN);

        registry().with(android_layer).init();
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

    pub is_virtual_mobile: bool,

    #[var]
    pub has_javascript_debugger: bool,

    #[var]
    pub network_inspector: Gd<NetworkInspector>,

    #[var]
    pub social_blacklist: Gd<DclSocialBlacklist>,

    #[var(get)]
    pub profile_service: Gd<ProfileService>,
}

#[godot_api]
impl INode for DclGlobal {
    fn init(base: Base<Node>) -> Self {
        #[cfg(feature = "use_deno")]
        crate::dcl::js::init_runtime();

        #[cfg(target_os = "android")]
        android::init_logger();

        #[cfg(not(target_os = "android"))]
        let _ = tracing_subscriber::fmt::try_init();

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
        let mut metrics: Gd<Metrics> = Metrics::new_alloc();

        // For now, keep using base Rust classes - GDScript extensions will be created in global.gd
        let mut realm: Gd<DclRealm> = DclRealm::new_alloc();
        let mut dcl_tokio_rpc: Gd<DclTokioRpc> = DclTokioRpc::new_alloc();
        let mut player_identity: Gd<DclPlayerIdentity> = DclPlayerIdentity::new_alloc();
        let mut testing_tools: Gd<DclTestingTools> = DclTestingTools::new_alloc();
        let mut portable_experience_controller: Gd<DclPortableExperienceController> =
            DclPortableExperienceController::new_alloc();

        tokio_runtime.set_name("tokio_runtime".into());
        scene_runner.set_name("scene_runner".into());
        scene_runner.set_process_mode(ProcessMode::DISABLED);
        comms.set_name("comms".into());
        avatars.set_name("avatar_scene".into());
        realm.set_name("realm".into());
        dcl_tokio_rpc.set_name("dcl_tokio_rpc".into());
        player_identity.set_name("player_identity".into());
        testing_tools.set_name("testing_tool".into());
        content_provider.set_name("content_provider".into());
        portable_experience_controller.set_name("portable_experience_controller".into());
        network_inspector.set_name("network_inspector".into());
        social_blacklist.set_name("social_blacklist".into());
        metrics.set_name("metrics".into());

        let args = godot::engine::Os::singleton().get_cmdline_args();

        let testing_scene_mode = args.find(&"--scene-test".into(), None).is_some();
        let preview_mode = args.find(&"--preview".into(), None).is_some();
        let developer_mode = args.find(&"--dev".into(), None).is_some();

        let fixed_skybox_time =
            testing_scene_mode || args.find(&"--scene-renderer".into(), None).is_some();

        set_scene_log_enabled(preview_mode || testing_scene_mode || developer_mode);

        let is_mobile = godot::engine::Os::singleton().has_feature("mobile".into());

        // For now, use base class - ConfigData will be created in global.gd
        let config = DclConfig::new_gd();

        Self {
            _base: base,
            is_mobile,
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
            profile_service: ProfileService::new_gd(),

            #[cfg(feature = "enable_inspector")]
            has_javascript_debugger: true,
            #[cfg(not(feature = "enable_inspector"))]
            has_javascript_debugger: false,
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
    fn is_virtual_mobile(&self) -> bool {
        self.is_virtual_mobile
    }

    #[func]
    fn _set_is_mobile(&mut self, is_mobile: bool) {
        self.is_mobile = is_mobile;
        self.is_virtual_mobile = is_mobile;
    }

    pub fn has_singleton() -> bool {
        let Some(main_loop) = Engine::singleton().get_main_loop() else {
            return false;
        };
        let Some(root) = main_loop.cast::<SceneTree>().get_root() else {
            return false;
        };
        root.has_node("Global".into())
    }

    pub fn try_singleton() -> Option<Gd<Self>> {
        let res = Engine::singleton()
            .get_main_loop()?
            .cast::<SceneTree>()
            .get_root()?
            .get_node_or_null("Global".into())?
            .try_cast::<Self>();
        if let Ok(res) = res {
            Some(res)
        } else {
            None
        }
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
}
