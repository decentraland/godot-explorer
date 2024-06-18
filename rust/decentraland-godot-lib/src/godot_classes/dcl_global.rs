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
    scene_runner::{scene_manager::SceneManager, tokio_runtime::TokioRuntime},
    test_runner::testing_tools::DclTestingTools,
};

use super::{
    dcl_config::DclConfig, dcl_realm::DclRealm, portables::DclPortableExperienceController,
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
            .with_filter(LevelFilter::DEBUG);

        registry().with(android_layer).init();
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclGlobal {
    #[base]
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
    pub player_identity: Gd<DclPlayerIdentity>,
    #[var]
    pub content_provider: Gd<ContentProvider>,
    #[var]
    pub http_requester: Gd<RustHttpQueueRequester>,

    pub ethereum_provider: Arc<EthereumProvider>,

    #[var]
    pub metrics: Gd<Metrics>,

    pub is_mobile: bool,
}

#[godot_api]
impl INode for DclGlobal {
    fn init(base: Base<Node>) -> Self {
        #[cfg(target_os = "android")]
        android::init_logger();

        #[cfg(not(target_os = "android"))]
        let _ = tracing_subscriber::fmt::try_init();

        tracing::info!(
            "DclGlobal init invoked version={}",
            env!("GODOT_EXPLORER_VERSION")
        );

        log_panics::init();

        let mut avatars: Gd<AvatarScene> = AvatarScene::alloc_gd();
        let mut comms: Gd<CommunicationManager> = CommunicationManager::alloc_gd();
        let mut scene_runner: Gd<SceneManager> = SceneManager::alloc_gd();
        let mut tokio_runtime: Gd<TokioRuntime> = TokioRuntime::alloc_gd();

        tokio_runtime.set_name("tokio_runtime".into());
        scene_runner.set_name("scene_runner".into());
        scene_runner.set_process_mode(ProcessMode::PROCESS_MODE_DISABLED);

        comms.set_name("comms".into());
        avatars.set_name("avatars".into());

        let args = godot::engine::Os::singleton().get_cmdline_args();

        let testing_scene_mode = args.find("--scene-test".into(), None).is_some();
        let preview_mode = args.find("--preview".into(), None).is_some();
        let developer_mode = args.find("--dev".into(), None).is_some();

        set_scene_log_enabled(preview_mode || testing_scene_mode || developer_mode);

        Self {
            _base: base,
            is_mobile: godot::engine::Os::singleton().has_feature("mobile".into()),
            scene_runner,
            comms,
            avatars,
            tokio_runtime,
            testing_tools: DclTestingTools::alloc_gd(),
            realm: DclRealm::alloc_gd(),
            portable_experience_controller: DclPortableExperienceController::alloc_gd(),
            preview_mode,
            testing_scene_mode,
            player_identity: DclPlayerIdentity::alloc_gd(),
            content_provider: ContentProvider::alloc_gd(),
            http_requester: RustHttpQueueRequester::new_gd(),
            config: DclConfig::new_gd(),
            ethereum_provider: Arc::new(EthereumProvider::new()),
            metrics: Metrics::alloc_gd(),
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
    fn _set_is_mobile(&mut self, is_mobile: bool) {
        self.is_mobile = is_mobile;
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
            .get_node("Global".into())?
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
}
