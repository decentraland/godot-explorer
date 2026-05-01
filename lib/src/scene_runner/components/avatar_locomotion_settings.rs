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

/// Updates the locomotion settings for the scene.
/// Returns `true` if the settings were changed, `false` otherwise.
pub fn update_avatar_locomotion_settings(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
) -> bool {
    let dirty_lww_components = &scene.current_dirty.lww_components;

    if let Some(locomotion_dirty) =
        dirty_lww_components.get(&SceneComponentId::AVATAR_LOCOMOTION_SETTINGS)
    {
        // Only process ROOT_ENTITY (id = 0)
        if locomotion_dirty.contains(&SceneEntityId::ROOT) {
            let locomotion_component =
                SceneCrdtStateProtoComponents::get_avatar_locomotion_settings(crdt_state);

            if let Some(value) = locomotion_component.get(&SceneEntityId::ROOT) {
                if let Some(proto) = &value.value {
                    scene.locomotion_settings.bind_mut().set_from_proto(proto);
                } else {
                    scene.locomotion_settings.bind_mut().reset_to_defaults();
                }
            } else {
                scene.locomotion_settings.bind_mut().reset_to_defaults();
            }
            return true;
        }
    }
    false
}
