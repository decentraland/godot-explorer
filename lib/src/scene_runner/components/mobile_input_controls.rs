use crate::{
    dcl::{
        components::{SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::{Scene, SceneType},
};

/// Updates the global mobile input controls visibility based on the
/// PBMobileInputControls component set on the PLAYER entity in the current parcel scene.
///
/// The component lets scenes hide the native on-screen mobile controls so creators can
/// render their own touch UI (bound via PBUiInputBinding):
/// - hide_joystick: hides the native virtual joystick
/// - hide_gamepad: hides the native action button cluster (joypad)
///
/// Both booleans are false by default (controls visible). The flags are written to the
/// DclGlobal singleton; the GDScript explorer reacts and toggles the actual UI (no-op on
/// desktop). Only processed when set on the PLAYER entity (entity ID 1).
pub fn update_mobile_input_controls(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    // Mirror the InputModifier processing rules:
    // - current parcel scene: process every tick (so entering a scene applies it)
    // - global scenes: process only when the component is dirty on PLAYER
    // - other parcel scenes: ignore
    let should_process = match scene.scene_type {
        SceneType::Parcel => is_current_parcel_scene,
        SceneType::Global(_) => dirty_lww_components
            .get(&SceneComponentId::MOBILE_INPUT_CONTROLS)
            .is_some_and(|dirty| dirty.contains(&SceneEntityId::PLAYER)),
    };

    if !should_process {
        return;
    }

    let component = SceneCrdtStateProtoComponents::get_mobile_input_controls(crdt_state);
    let player_value = component.get(&SceneEntityId::PLAYER);

    let Some(mut global) = DclGlobal::try_singleton() else {
        return;
    };
    let mut global_bind = global.bind_mut();

    match player_value.and_then(|entry| entry.value.as_ref()) {
        Some(controls) => {
            global_bind.mobile_input_hide_joystick = controls.hide_joystick;
            global_bind.mobile_input_hide_gamepad = controls.hide_gamepad;
        }
        None => {
            // Component removed or not set: reset to visible (only for current parcel scene)
            if is_current_parcel_scene {
                global_bind.reset_mobile_input_controls();
            }
        }
    }
}
