use godot::{
    engine::{node::ProcessMode, Engine},
    prelude::*,
};

use crate::{
    avatars::avatar_scene::AvatarScene,
    comms::communication_manager::CommunicationManager,
    scene_runner::{scene_manager::SceneManager, tokio_runtime::TokioRuntime},
    test_runner::testing_tools::DclTestingTools,
};

use super::{dcl_realm::DclRealm, portables::DclPortableExperienceController};

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

        Self {
            _base: base,
            scene_runner,
            comms,
            avatars,
            tokio_runtime,
            testing_tools: DclTestingTools::alloc_gd(),
            realm: DclRealm::alloc_gd(),
            portable_experience_controller: DclPortableExperienceController::alloc_gd(),
            preview_mode: false,
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
