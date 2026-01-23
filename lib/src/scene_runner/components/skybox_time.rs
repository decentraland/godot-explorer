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

/// Updates the global SDK skybox time state based on the SkyboxTime component
/// set on the ROOT entity in the current parcel scene.
///
/// The SkyboxTime component allows scenes to control the time of day displayed
/// in the skybox:
/// - fixed_time: Time in seconds since 00:00 (0-86400)
/// - transition_mode: TM_FORWARD (default) or TM_BACKWARD for transition direction
///
/// This component is only processed when set on the ROOT entity (entity ID 0).
pub fn update_skybox_time(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    let is_dirty = dirty_lww_components
        .get(&SceneComponentId::SKYBOX_TIME)
        .is_some_and(|dirty| dirty.contains(&SceneEntityId::ROOT));

    // Only process for current parcel scene or dirty global scenes
    let should_process = match scene.scene_type {
        SceneType::Parcel => is_current_parcel_scene,
        SceneType::Global(_) => is_dirty,
    };

    if !should_process {
        return;
    }

    let Some(mut global) = DclGlobal::try_singleton() else {
        return;
    };
    let mut global_bind = global.bind_mut();

    // For parcel scenes: skip if already active and not dirty (optimization)
    if matches!(scene.scene_type, SceneType::Parcel)
        && global_bind.sdk_skybox_time_active
        && !is_dirty
    {
        return;
    }

    // Get the SkyboxTime component for the ROOT entity
    let skybox_time_component = SceneCrdtStateProtoComponents::get_skybox_time(crdt_state);
    let root_skybox_time = skybox_time_component.get(&SceneEntityId::ROOT);

    match root_skybox_time.and_then(|entry| entry.value.as_ref()) {
        Some(skybox_time) => {
            let fixed_time = skybox_time.fixed_time;
            let transition_forward = skybox_time.transition_mode() != TransitionMode::TmBackward;

            if !global_bind.sdk_skybox_time_active {
                tracing::debug!(
                    "SkyboxTime SDK control changed: active=true, time={}, forward={}",
                    fixed_time,
                    transition_forward
                );
            } else if fixed_time != global_bind.sdk_skybox_fixed_time {
                tracing::debug!(
                    "SkyboxTime value changed: time={} (was {}), forward={}",
                    fixed_time,
                    global_bind.sdk_skybox_fixed_time,
                    transition_forward
                );
            }

            global_bind.sdk_skybox_time_active = true;
            global_bind.sdk_skybox_fixed_time = fixed_time;
            global_bind.sdk_skybox_transition_forward = transition_forward;
        }
        None => {
            if global_bind.sdk_skybox_time_active {
                tracing::debug!("SkyboxTime SDK control changed: active=false");
            }
            global_bind.reset_skybox_time();
        }
    }
}
