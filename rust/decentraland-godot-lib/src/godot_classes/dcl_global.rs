use godot::{
    engine::{node::ProcessMode, Engine},
    prelude::*,
};

use crate::{
    auth::dcl_player_identity::DclPlayerIdentity,
    avatars::avatar_scene::AvatarScene,
    comms::communication_manager::CommunicationManager,
    scene_runner::{scene_manager::SceneManager, tokio_runtime::TokioRuntime},
    test_runner::testing_tools::DclTestingTools,
};

use super::{dcl_realm::DclRealm, portables::DclPortableExperienceController};

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
}

#[godot_api]
impl INode for DclGlobal {
    fn init(base: Base<Node>) -> Self {
        #[cfg(target_os = "android")]
        android::init_logger();

        #[cfg(not(target_os = "android"))]
        let _ = tracing_subscriber::fmt::try_init();

        tracing::info!("DclGlobal init invoked");

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

        // var scene_test_index := args.find("--scene-test")
        Self {
            _base: base,
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
        }
    }
}

#[godot_api]
impl DclGlobal {
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
