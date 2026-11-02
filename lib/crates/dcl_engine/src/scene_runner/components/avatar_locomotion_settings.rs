use crate::{
    dcl::{
        components::{SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

/// Updates the locomotion settings for the scene from the `PBAvatarLocomotionSettings`
/// component on the PLAYER entity (id = 1), which is where scenes address it:
/// `AvatarLocomotionSettings.createOrReplace(engine.PlayerEntity, { ... })`.
///
/// This mirrors the other player-targeted components (`InputModifier`,
/// `PhysicsCombinedForce`) and the reference implementation. The component is
/// ignored on any other entity.
///
/// Returns `true` if the settings were changed, `false` otherwise.
pub fn update_avatar_locomotion_settings(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
) -> bool {
    let dirty_lww_components = &scene.current_dirty.lww_components;

    let is_dirty = dirty_lww_components
        .get(&SceneComponentId::AVATAR_LOCOMOTION_SETTINGS)
        .is_some_and(|dirty| dirty.contains(&SceneEntityId::PLAYER));

    if !is_dirty {
        return false;
    }

    let locomotion_component =
        SceneCrdtStateProtoComponents::get_avatar_locomotion_settings(crdt_state);
    let player_settings = locomotion_component.get(&SceneEntityId::PLAYER);

    // A missing entry or a cleared value means the scene removed the component:
    // fall back to the engine defaults.
    let mut settings = scene.locomotion_settings.bind_mut();
    match player_settings.and_then(|entry| entry.value.as_ref()) {
        Some(proto) => settings.set_from_proto(proto),
        None => settings.reset_to_defaults(),
    }

    true
}
