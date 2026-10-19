//! Handles to the services `Global` holds, each cloned out so the borrow of `Global` ends
//! before the service is used: a service call that reaches back into `Global` (metrics, settings)
//! would otherwise find it still bound.

use godot::prelude::*;

use crate::{
    analytics::metrics::Metrics,
    auth::dcl_player_identity::DclPlayerIdentity,
    avatars::avatar_scene::AvatarScene,
    comms::communication_manager::CommunicationManager,
    content::content_provider::ContentProvider,
    engine::DclEngine,
    godot_classes::{dcl_cli::DclCli, dcl_global::DclGlobal, dcl_realm::DclRealm},
    http_request::rust_http_queue_requester::RustHttpQueueRequester,
    scene_runner::scene_manager::SceneManager,
};

macro_rules! service {
    ($name:ident, $field:ident, $ty:ty) => {
        pub fn $name() -> Gd<$ty> {
            DclGlobal::singleton().bind().$field.clone()
        }
    };
}

service!(scene_runner, scene_runner, SceneManager);
service!(avatars_scene, avatars, AvatarScene);
service!(engine, engine, DclEngine);
service!(metrics, metrics, Metrics);
service!(comms, comms, CommunicationManager);
service!(content_provider, content_provider, ContentProvider);
service!(realm, realm, DclRealm);
service!(cli, cli, DclCli);
service!(player_identity, player_identity, DclPlayerIdentity);
service!(http_requester, http_requester, RustHttpQueueRequester);
service!(
    dcl_tokio_rpc,
    dcl_tokio_rpc,
    crate::godot_classes::dcl_tokio_rpc::DclTokioRpc
);
service!(
    scene_inspector_dispatcher,
    scene_inspector_dispatcher,
    crate::tools::scene_inspector::SceneInspectorDispatcher
);
service!(
    social_blacklist,
    social_blacklist,
    crate::godot_classes::dcl_social_blacklist::DclSocialBlacklist
);

/// The scene fetcher; it exists from the first boot step on.
pub fn scene_fetcher() -> Gd<crate::fetcher::scene_fetcher::SceneFetcher> {
    DclGlobal::singleton()
        .bind()
        .scene_fetcher
        .clone()
        .expect("the scene fetcher is created at boot")
}

/// The sky's clock, once it exists.
pub fn skybox_time() -> Option<Gd<crate::sky::skybox_time::SkyboxTime>> {
    DclGlobal::singleton().bind().skybox_time.clone()
}

/// The explorer scene, when it is the current one.
pub fn explorer() -> Option<Gd<crate::world::explorer::Explorer>> {
    DclGlobal::get_explorer()
}
