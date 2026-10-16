//! Handles to the services `Global` holds, each cloned out so the borrow of `Global` ends
//! before the service is used: a service call that reaches back into `Global` (metrics, settings)
//! would otherwise find it still bound.

use godot::classes::Node;
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

/// The explorer scene, when it is the current one.
pub fn explorer() -> Option<Gd<Node>> {
    DclGlobal::singleton()
        .call("get_explorer", &[])
        .try_to::<Gd<Node>>()
        .ok()
}
