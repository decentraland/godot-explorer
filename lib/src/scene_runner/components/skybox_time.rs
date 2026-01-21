use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::TransitionMode, SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::{Scene, SceneType},
};

/// Updates the SDK-controlled skybox time based on the SkyboxTime component
/// set on the ROOT entity in the current parcel scene.
///
/// The SkyboxTime component allows scenes to control the time of day for the skybox:
/// - fixed_time: Time of day in seconds since midnight (0-86400)
/// - transition_mode: Direction of transitions (FORWARD or BACKWARD)
///
/// This component is only processed when set on the ROOT entity (entity ID 0).
pub fn update_skybox_time(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    // Determine if we should process the SkyboxTime for this scene:
    // - For the current parcel scene: only when the component is dirty
    // - For global scenes: ignore (skybox is controlled by parcel scenes only)
    // - For other parcel scenes: ignore
    let should_process = match scene.scene_type {
        SceneType::Parcel => {
            if is_current_parcel_scene {
                // Only process when component is dirty on ROOT entity
                dirty_lww_components
                    .get(&SceneComponentId::SKYBOX_TIME)
                    .is_some_and(|dirty| dirty.contains(&SceneEntityId::ROOT))
            } else {
                false
            }
        }
        SceneType::Global(_) => false, // Global scenes don't control skybox
    };

    if !should_process {
        return;
    }

    // Get access to the global singleton to update skybox time state
    let Some(mut global) = DclGlobal::try_singleton() else {
        return;
    };

    let global_bind = global.bind_mut();
    let mut scene_runner = global_bind.scene_runner.clone();
    let mut scene_runner_bind = scene_runner.bind_mut();

    // Get the SkyboxTime component for the ROOT entity
    let skybox_time_component = SceneCrdtStateProtoComponents::get_skybox_time(crdt_state);
    let root_skybox_time = skybox_time_component.get(&SceneEntityId::ROOT);

    // Extract the skybox time values, or reset if component was removed
    match root_skybox_time.and_then(|entry| entry.value.as_ref()) {
        Some(skybox_time) => {
            let fixed_time = skybox_time.fixed_time;
            let transition_forward = skybox_time.transition_mode() != TransitionMode::TmBackward;

            // Log state changes for debugging
            if !scene_runner_bind.get_sdk_skybox_time_active() {
                tracing::debug!(
                    "SkyboxTime SDK control enabled: time={}, forward={}",
                    fixed_time,
                    transition_forward
                );
            } else if fixed_time != scene_runner_bind.get_sdk_skybox_fixed_time() {
                tracing::debug!(
                    "SkyboxTime value changed: time={} (was {}), forward={}",
                    fixed_time,
                    scene_runner_bind.get_sdk_skybox_fixed_time(),
                    transition_forward
                );
            }

            scene_runner_bind.set_sdk_skybox_time_active(true);
            scene_runner_bind.set_sdk_skybox_fixed_time(fixed_time);
            scene_runner_bind.set_sdk_skybox_transition_forward(transition_forward);
        }
        None => {
            // Component was removed, reset skybox control
            if scene_runner_bind.get_sdk_skybox_time_active() {
                tracing::debug!("SkyboxTime SDK control disabled");
                scene_runner_bind.set_sdk_skybox_time_active(false);
                scene_runner_bind.set_sdk_skybox_fixed_time(0);
                scene_runner_bind.set_sdk_skybox_transition_forward(true);
            }
        }
    }
}
