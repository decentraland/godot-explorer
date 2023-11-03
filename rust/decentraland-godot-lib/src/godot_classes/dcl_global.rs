use godot::{
    engine::{node::ProcessMode, Engine},
    prelude::*,
};

use crate::{
    avatars::avatar_scene::AvatarScene,
    comms::communication_manager::CommunicationManager,
    scene_runner::{scene_manager::SceneManager, tokio_runtime::TokioRuntime},
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclGlobal {
    #[base]
    _base: Base<Node>,
    #[var]
    scene_runner: Gd<SceneManager>,
    #[var]
    comms: Gd<CommunicationManager>,
    #[var]
    avatars: Gd<AvatarScene>,
    #[var]
    tokio_runtime: Gd<TokioRuntime>,
}

#[godot_api]
impl NodeVirtual for DclGlobal {
    fn init(base: Base<Node>) -> Self {
        #[cfg(target_os = "android")]
        android::init_logger();

        #[cfg(not(target_os = "android"))]
        let _ = tracing_subscriber::fmt::try_init();

        tracing::info!("DclGlobal init invoked");

        log_panics::init();

        let mut avatars: Gd<AvatarScene> = Gd::new_default();
        let mut comms: Gd<CommunicationManager> = Gd::new_default();
        let mut scene_runner: Gd<SceneManager> = Gd::new_default();
        let mut tokio_runtime: Gd<TokioRuntime> = Gd::new_default();

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
        }
    }
}

#[godot_api]
impl DclGlobal {
    pub fn try_singleton() -> Option<Gd<Self>> {
        Engine::singleton()
            .get_main_loop()?
            .cast::<SceneTree>()
            .get_root()?
            .get_node("Global".into())?
            .try_cast::<Self>()
    }

    pub fn singleton() -> Gd<Self> {
        Self::try_singleton().expect("Failed to get global singleton!")
    }
}
